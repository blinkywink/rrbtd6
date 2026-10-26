"""Export an animated prefab: mesh + every Animator clip baked to vertex frames.

Decodes Mecanim generic clips (streamed Hermite curves, dense samples,
constants) bound by CRC32 of the transform path relative to the Animator,
poses the whole hierarchy per frame (bones, plus meshes parented to bones),
skins (linear blend), and writes per-frame positions and normals.

Usage:
  tools/.venv/Scripts/python.exe tools/assets/export_animated.py <game install> <out_dir> <bundle-glob> <prefab container path> [fps]

Output (<out_dir>/): meta.json {parts, anims: [{name, frames, fps, loop, events, offset}]},
verts.bin (rest pose: f32 x y z u v), idx.bin (u32), tex/*.png,
anim.bin (per anim, per frame, per vertex: f32 px py pz nx ny nz). Unity prefab space.
"""
import glob
import json
import os
import struct
import sys
import zlib

import numpy as np
import UnityPy
from UnityPy.helpers import TypeTreeHelper
from UnityPy.helpers.MeshHelper import MeshHandler

TypeTreeHelper.read_typetree_boost = False
GAME, OUT, BUNDLE, PREFAB = sys.argv[1:5]
FPS = float(sys.argv[5]) if len(sys.argv) > 5 else 60.0
BAKED_OUTLINE = 0.97
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")

sys.path.insert(0, os.path.dirname(__file__))
sys.argv = [sys.argv[0], GAME, OUT, "prefab", BUNDLE]  # reuse helpers from export_model
import export_model as em  # noqa: E402


def quat_mat(q):
    x, y, z, w = q
    n = x * x + y * y + z * z + w * w
    if n > 0:
        s = 2.0 / n
    else:
        s = 0.0
    return np.array([[1 - s * (y * y + z * z), s * (x * y - z * w), s * (x * z + y * w)],
                     [s * (x * y + z * w), 1 - s * (x * x + z * z), s * (y * z - x * w)],
                     [s * (x * z - y * w), s * (y * z + x * w), 1 - s * (x * x + y * y)]])


def euler_quat(e):
    # Unity euler (degrees), applied Z, X, Y.
    x, y, z = [np.radians(v) / 2 for v in e]
    cx, sx, cy, sy, cz, sz = np.cos(x), np.sin(x), np.cos(y), np.sin(y), np.cos(z), np.sin(z)
    return (sx * cy * cz + cx * sy * sz, cx * sy * cz - sx * cy * sz, cx * cy * sz - sx * sy * cz, cx * cy * cz + sx * sy * sz)


def trs_mat(p, q, s):
    m = np.eye(4)
    m[:3, :3] = quat_mat(q) @ np.diag(s)
    m[:3, 3] = p
    return m


class Clip:
    def __init__(self, t):
        self.name = t["m_Name"]
        mc = t["m_MuscleClip"]
        c = mc["m_Clip"]["data"]
        self.start, self.stop = mc["m_StartTime"], mc["m_StopTime"]
        self.loop = bool(mc["m_LoopTime"])
        self.events = [(e["time"], e["functionName"]) for e in t.get("m_Events", [])]
        sc = c["m_StreamedClip"]
        self.n_streamed = sc["curveCount"]
        raw = struct.pack(f"<{len(sc['data'])}I", *sc["data"])
        self.keys = [[] for _ in range(self.n_streamed)]
        p = 0
        while p < len(raw):
            time, n = struct.unpack_from("<fI", raw, p)
            p += 8
            for _ in range(n):
                idx, a, b, cc, d = struct.unpack_from("<I4f", raw, p)
                p += 20
                if idx < self.n_streamed:
                    self.keys[idx].append((time, a, b, cc, d))
        dc = c["m_DenseClip"]
        self.n_dense = dc["m_CurveCount"]
        self.dense = np.array(dc["m_SampleArray"], dtype=np.float64).reshape(-1, self.n_dense) if self.n_dense else None
        self.dense_rate, self.dense_begin = dc["m_SampleRate"], dc["m_BeginTime"]
        self.const = c["m_ConstantClip"]["data"]
        self.bindings = t["m_ClipBindingConstant"]["genericBindings"]

    def values(self, t):
        out = []
        for keys in self.keys:
            v = 0.0
            for k in keys:
                if k[0] <= t:
                    dt = t - k[0]
                    v = ((k[1] * dt + k[2]) * dt + k[3]) * dt + k[4]
                else:
                    break
            out.append(v)
        if self.n_dense:
            f = (t - self.dense_begin) * self.dense_rate
            i0 = int(np.clip(np.floor(f), 0, len(self.dense) - 1))
            i1 = min(i0 + 1, len(self.dense) - 1)
            u = float(np.clip(f - i0, 0, 1))
            out.extend((self.dense[i0] * (1 - u) + self.dense[i1] * u).tolist())
        out.extend(self.const)
        return out

    def pose(self, t):
        """{path_hash: {attr: value tuple}}"""
        vals = self.values(t)
        res, ci = {}, 0
        for b in self.bindings:
            n = {1: 3, 2: 4, 3: 3, 4: 3}.get(b["attribute"], 1) if b["typeID"] == 4 else 1
            if b["typeID"] == 4:
                res.setdefault(b["path"], {})[b["attribute"]] = tuple(vals[ci:ci + n])
            ci += n
        return res


def single_bone_indices(mesh_obj, n):
    """BlendIndices (channel 13) of a mesh with one bone per vertex and no BlendWeight
    channel, from the raw VertexData (streams packed in channel order, each 16-byte aligned)."""
    vd = mesh_obj.read_typetree()["m_VertexData"]
    ch = vd["m_Channels"]
    if len(ch) <= 13 or ch[13]["dimension"] != 1 or (len(ch) > 12 and ch[12]["dimension"]):
        return None
    sizes = {0: 4, 1: 2, 2: 1, 3: 1, 4: 2, 5: 2, 6: 1, 7: 1, 8: 2, 9: 2, 10: 4, 11: 4}
    stride = {}
    for c in ch:
        if c["dimension"]:
            stride[c["stream"]] = max(stride.get(c["stream"], 0), c["offset"] + sizes[c["format"]] * c["dimension"])
    start, off = {}, 0
    for s in sorted(stride):
        start[s] = off
        off = (off + stride[s] * n + 15) // 16 * 16
    c = ch[13]
    dt = {6: np.uint8, 7: np.int8, 8: np.uint16, 9: np.int16, 10: np.uint32, 11: np.int32}.get(c["format"])
    if dt is None:
        return None
    raw = np.frombuffer(bytes(vd["m_DataSize"]), dtype=np.uint8)
    s = c["stream"]
    rows = raw[start[s]:start[s] + stride[s] * n].reshape(n, stride[s])
    size = sizes[c["format"]]
    return rows[:, c["offset"]:c["offset"] + size].copy().view(dt).reshape(n).astype(np.int64)


class StaticClip:
    """One-frame bind pose for prefabs without an animation controller."""

    def __init__(self, name):
        self.name, self.start, self.stop, self.loop, self.events, self.bindings = name, 0.0, 0.0, True, [], []

    def pose(self, t):
        return {}


def main():
    bundle = glob.glob(os.path.join(AA, BUNDLE + "*.bundle"))[0]
    # Batch runs (export_animated_batch.py) load each bundle once.
    env = em._envs.get(os.path.basename(bundle)) or UnityPy.load(bundle)
    em._envs[os.path.basename(bundle)] = env
    os.makedirs(OUT, exist_ok=True)
    ab = next(o for o in env.objects if o.type.name == "AssetBundle").read_typetree()
    parent_world = np.eye(4)
    if PREFAB.startswith("scene:"):
        cab, wanted = PREFAB[6:].split("|", 1)
        af = next(o.assets_file for o in env.objects if o.assets_file.name.lower() == cab.lower())
        def scene_path(tid):
            names = []
            while tid:
                t = af.objects[tid].read_typetree()
                names.append(af.objects[t["m_GameObject"]["m_PathID"]].read_typetree()["m_Name"])
                tid = t["m_Father"]["m_PathID"]
            return "/".join(reversed(names))
        tr = next(o for o in af.objects.values() if o.type.name == "Transform" and scene_path(o.path_id) == wanted)
        tt = tr.read_typetree()
        root = af.objects[tt["m_GameObject"]["m_PathID"]]
        parent_world = em.world_matrix(af, tt["m_Father"]["m_PathID"])
    else:
        pid = next(v["asset"]["m_PathID"] for k, v in ab["m_Container"] if k == PREFAB)
        root = next(o for o in env.objects if o.path_id == pid)
    af = root.assets_file

    def comp(go, name):
        for c in go["m_Component"]:
            o = af.objects.get(c["component"]["m_PathID"])
            if o is not None and o.type.name == name:
                return o
        return None

    # Transform tree.
    T = {}
    order = []
    animator = None
    # MonkeyAnimationController's state lists, which name clips the Animator's own list may
    # lack: placementConfigs (played when placed: Fall, the drop in), attackIdleConfigs,
    # IdleVariants (Look, Scratch... played now and then, by variantWeight) and the
    # animationStates flagged isUpgradeAnimation. [(clip object, role, weight)]
    role_clips = []
    stack = [(comp(root.read_typetree(), "Transform").path_id, None)]
    while stack:
        tid, parent = stack.pop()
        t = af.objects[tid].read_typetree()
        go = af.objects[t["m_GameObject"]["m_PathID"]].read_typetree()
        lp, lq, ls = t["m_LocalPosition"], t["m_LocalRotation"], t["m_LocalScale"]
        T[tid] = {"name": go["m_Name"], "parent": parent, "go": go, "active": go.get("m_IsActive", 1),
                  "p": (lp["x"], lp["y"], lp["z"]), "q": (lq["x"], lq["y"], lq["z"], lq["w"]),
                  "s": (ls["x"], ls["y"], ls["z"])}
        order.append(tid)
        an = comp(go, "Animator")
        if an is not None and animator is None:
            a = an.read_typetree()
            # Some prefabs carry a controller-less Animator (e.g. Blade Maelstrom); use the next one.
            # Without a controller, the clips can come from the animation-states MonoBehaviour next
            # to it (animationStates, attackIdleConfigs, IdleVariants; Sniper Monkey 000).
            ctrl_obj = em.resolve(af, a["m_Controller"])
            if ctrl_obj is not None:
                ctrl = ctrl_obj.read_typetree()
                if "m_AnimationClips" in ctrl:
                    clips_of = [em.resolve(ctrl_obj.assets_file, c) for c in ctrl["m_AnimationClips"]]
                else:
                    # AnimatorOverrideController (Mermonkey tentacles): the base controller's
                    # clips, each replaced by its override where one is set.
                    base = em.resolve(ctrl_obj.assets_file, ctrl["m_Controller"])
                    bt = base.read_typetree() if base is not None else {}
                    over = {}
                    for pair in ctrl.get("m_Clips", []):
                        o = em.resolve(ctrl_obj.assets_file, pair["m_OverrideClip"])
                        orig = em.resolve(ctrl_obj.assets_file, pair["m_OriginalClip"])
                        if o is not None and orig is not None:
                            over[orig.path_id] = o
                    clips_of = []
                    for c in bt.get("m_AnimationClips", []):
                        orig = em.resolve(base.assets_file, c)
                        clips_of.append(over.get(orig.path_id, orig) if orig is not None else None)
                animator = (tid, [c for c in clips_of if c is not None])
            else:
                for c in go["m_Component"]:
                    mb = af.objects.get(c["component"]["m_PathID"])
                    if mb is None or mb.type.name != "MonoBehaviour":
                        continue
                    t_mb = mb.read_typetree()
                    states = [s for k in ("animationStates", "attackIdleConfigs", "IdleVariants") for s in t_mb.get(k, [])]
                    objs = [em.resolve(af, s["animationClip"]) for s in states]
                    if any(objs):
                        animator = (tid, [o for o in objs if o is not None])
                        break
        if not role_clips:
            for c in go["m_Component"]:
                mb = af.objects.get(c["component"]["m_PathID"])
                if mb is None or mb.type.name != "MonoBehaviour":
                    continue
                t_mb = mb.read_typetree()
                if "placementConfigs" not in t_mb:
                    continue
                lists = [("place", t_mb.get("placementConfigs", [])), ("attack_idle", t_mb.get("attackIdleConfigs", [])),
                         ("idle_variant", t_mb.get("IdleVariants", [])),
                         ("upgrade", [x for x in t_mb.get("animationStates", []) if x.get("isUpgradeAnimation")])]
                for role, states in lists:
                    for st in states:
                        o = em.resolve(af, st["animationClip"])
                        if o is not None:
                            role_clips.append((o, role, st.get("variantWeight")))
                break
        if os.environ.get("RRBTD6_ANIM_INTERACT") and tid==order[0]:
            for c in go["m_Component"]:
                source=af.objects.get(c["component"]["m_PathID"])
                if source is not None and source.type.name=="MonoBehaviour":
                    fields=source.read_typetree()
                    if "triggerAnimationLength" in fields and fields.get("animationClip",{}).get("m_PathID"):
                        clip=em.resolve(af,fields["animationClip"])
                        if clip is not None:animator=(tid,[clip])
        for ch in reversed(t["m_Children"]):
            stack.append((ch["m_PathID"], tid))

    # Path hashes relative to the Animator's transform.
    a_tid = animator[0] if animator else order[0]

    def rel_path(tid):
        names = []
        while tid is not None and tid != a_tid:
            names.append(T[tid]["name"])
            tid = T[tid]["parent"]
        return "/".join(reversed(names)) if tid == a_tid else None

    by_hash = {}
    for tid in order:
        p = rel_path(tid)
        if p is not None:
            by_hash[zlib.crc32(p.encode("utf-8"))] = tid

    if animator:
        clips = [Clip(o.read_typetree()) for o in animator[1]]
        by_id = {a.path_id: c for c, a in zip(clips, animator[1])}
        for o, role, weight in role_clips:
            c = by_id.get(o.path_id)
            if c is None:
                c = by_id[o.path_id] = Clip(o.read_typetree())
                clips.append(c)
            if getattr(c, "role", None) is None:
                c.role, c.weight = role, weight
    else:
        clips = [StaticClip(T[order[0]]["name"] + "Idle")]
    if os.environ.get("RRBTD6_ANIM_CLIP"):
        clips = [c for c in clips if c.name == os.environ["RRBTD6_ANIM_CLIP"]]
        if not clips:
            raise ValueError("Requested source animation clip not found")
    bound = sum(1 for c in clips for b in c.bindings if b["typeID"] == 4 and b["path"] in by_hash)
    total = sum(1 for c in clips for b in c.bindings if b["typeID"] == 4)
    print(f"{len(T)} transforms, {len(clips)} clips, {bound}/{total} transform bindings resolved")
    # RRBTD6_ANIM_DEBUG: which transforms each clip animates (and which bindings miss).
    if os.environ.get("RRBTD6_ANIM_DEBUG"):
        for c in clips:
            names = sorted({T[by_hash[b["path"]]]["name"] if b["path"] in by_hash else f"?{b['path']}"
                            for b in c.bindings if b["typeID"] == 4})
            print(f"  {c.name}: {names}")
            # How far each transform's keyed values travel over the clip.
            span = {}
            for k in range(21):
                for h, attrs in c.pose(c.start + (c.stop - c.start) * k / 20).items():
                    for at, v in attrs.items():
                        lo, hi = span.get((h, at), (v, v))
                        span[(h, at)] = (tuple(map(min, lo, v)), tuple(map(max, hi, v)))
            for (h, at), (lo, hi) in sorted(span.items(), key=lambda x: str(x[0])):
                d = max(b - a for a, b in zip(lo, hi))
                nm = T[by_hash[h]]["name"] if h in by_hash else f"?{h}"
                if d > 1e-3 and ("Temple" in nm or "?" in nm or "Scale" in nm):
                    print(f"    {nm} attr{at} moves {d:.3f}  {[round(x, 2) for x in lo]} .. {[round(x, 2) for x in hi]}")

    def world_mats(pose):
        W = {}
        for tid in order:  # parents precede children
            n = T[tid]
            p, q, s = n["p"], n["q"], n["s"]
            if pose is not None:
                h = pose.get(next((k for k, v in by_hash.items() if v == tid), None), {})
                p = h.get(1, p)
                q = h.get(2, q)
                if 4 in h:
                    q = euler_quat(h[4])
                s = h.get(3, s)
            m = trs_mat(p, q, s)
            W[tid] = parent_world @ m if n["parent"] is None else W[n["parent"]] @ m
        return W

    hash_of = {v: k for k, v in by_hash.items()}

    def world_mats_fast(pose):
        W = {}
        for tid in order:
            n = T[tid]
            p, q, s = n["p"], n["q"], n["s"]
            if pose is not None and tid in hash_of:
                h = pose.get(hash_of[tid], {})
                p = h.get(1, p)
                q = h.get(2, q)
                if 4 in h:
                    q = euler_quat(h[4])
                s = h.get(3, s)
            m = trs_mat(p, q, s)
            W[tid] = parent_world @ m if n["parent"] is None else W[n["parent"]] @ m
        return W

    # Renderers (active subtree), mesh data, parts.
    renderers = []
    for tid in order:
        n = T[tid]
        t = tid
        active = True
        while t is not None:
            if not T[t]["active"]:
                active = False
                break
            t = T[t]["parent"]
        # RRBTD6_ANIM_ONLY=<name>: just that renderer, active or not (the Sun Temple head the
        # TrueSunGodBase prefab keeps switched off but still animates).
        only = os.environ.get("RRBTD6_ANIM_ONLY")
        if only:
            active = n["name"] == only
        if not active:
            continue
        for kind in ("SkinnedMeshRenderer", "MeshRenderer"):
            r_obj = comp(n["go"], kind)
            if r_obj is None:
                continue
            r = r_obj.read_typetree()
            if not r.get("m_Enabled", 1):
                continue
            if kind == "SkinnedMeshRenderer":
                mesh_obj = em.resolve(af, r["m_Mesh"])
            else:
                mf = comp(n["go"], "MeshFilter")
                mesh_obj = em.resolve(af, mf.read_typetree()["m_Mesh"]) if mf else None
            if mesh_obj is None:
                continue
            mesh = mesh_obj.read()
            h = MeshHandler(mesh)
            h.process()
            v = np.array(h.m_Vertices, dtype=np.float64).reshape(-1, 3)
            # Some meshes store 4-component normals (BadFortifiedDamageState3D1).
            nrm = np.array(h.m_Normals, dtype=np.float64).reshape(len(v), -1)[:, :3] if h.m_Normals else np.zeros_like(v)
            uv = np.array(h.m_UV0, dtype=np.float64).reshape(-1, 2) if h.m_UV0 else np.zeros((len(v), 2))
            col = np.array(h.m_Colors, dtype=np.float64).reshape(len(v), 4) if h.m_Colors else None
            skin = None
            bones = r.get("m_Bones") or []
            if kind == "SkinnedMeshRenderer" and bones and h.m_BoneWeights:
                bind = [em.mat4(b) for b in mesh.m_BindPose]
                w = np.array(h.m_BoneWeights, dtype=np.float64).reshape(len(v), -1)
                bi = np.array(h.m_BoneIndices, dtype=np.int64).reshape(len(v), -1)
                skin = ([b["m_PathID"] for b in bones], bind, w, bi)
            elif kind == "SkinnedMeshRenderer" and bones:
                # One bone per vertex and no weights channel (Sun Temple base: its top and door
                # joints): each vertex follows its BlendIndices bone fully. UnityPy misreads this
                # 1-wide stream, so it's read from the raw vertex data.
                bi = single_bone_indices(mesh_obj, len(v))
                if bi is not None:
                    skin = ([b["m_PathID"] for b in bones], [em.mat4(b) for b in mesh.m_BindPose],
                            np.ones((len(v), 1)), bi.reshape(-1, 1))
            subs = []
            for sub, mref in zip(h.get_triangles(), r["m_Materials"]):
                mat_obj = em.resolve(af, mref)
                if mat_obj is None or mat_obj.type.name != "Material":
                    continue
                mt = mat_obj.read_typetree()
                props = mt["m_SavedProperties"]
                floats = dict(props.get("m_Floats", []))
                cols = dict(props.get("m_Colors", []))
                oc = cols.get("_OutlineColor", {"r": 0, "g": 0, "b": 0, "a": 0})
                # Baked "Flat_" meshes (BAKED_MESHES keyword, no _Outline property) carry the outline
                # per vertex: rgb = the source material's _OutlineColor, a = 1 if it had an outline.
                baked = "_Outline" not in floats and col is not None
                subs.append((np.array(sub, dtype=np.uint32).reshape(-1), em.export_texture(mat_obj), mt["m_Name"],
                             floats.get("_Outline", 0.0), [oc["r"], oc["g"], oc["b"], oc["a"]], baked))
            renderers.append({"tid": tid, "name": n["name"], "v": v, "n": nrm, "uv": uv, "skin": skin, "subs": subs,
                              "col": col})

    def pose_vertices(W, r):
        vh = np.c_[r["v"], np.ones(len(r["v"]))]
        nh = np.c_[r["n"], np.zeros(len(r["n"]))]
        if r["skin"] is not None:
            bones, bind, w, bi = r["skin"]
            mats = np.stack([(W[b] if b in W else np.eye(4)) @ (bind[i] if i < len(bind) else np.eye(4))
                             for i, b in enumerate(bones)])
            pos = np.zeros((len(vh), 4))
            nor = np.zeros((len(vh), 4))
            for k in range(w.shape[1]):
                ms = mats[bi[:, k]]
                pos += w[:, k:k + 1] * np.einsum("nij,nj->ni", ms, vh)
                nor += w[:, k:k + 1] * np.einsum("nij,nj->ni", ms, nh)
        else:
            m = W[r["tid"]]
            pos, nor = vh @ m.T, nh @ m.T
        nn = nor[:, :3]
        nn = nn / np.maximum(np.linalg.norm(nn, axis=1, keepdims=True), 1e-9)
        return pos[:, :3], nn

    # Parts: one per (renderer, submesh), vertices remapped to the used set.
    parts, verts, idxs, remaps, vout = [], [], [], [], []
    W0 = world_mats_fast(None)
    voff = ioff = 0
    for r in renderers:
        pos, _ = pose_vertices(W0, r)
        for ind, tex, mname, outline, ocol, baked in r["subs"]:
            used = np.unique(ind)
            rm = np.zeros(len(r["v"]), dtype=np.uint32)
            rm[used] = np.arange(len(used), dtype=np.uint32)
            verts.append(np.c_[pos[used], r["uv"][used]].astype(np.float32))
            # Per-vertex outline [r, g, b, width]. INFERRED: a baked a = 1 means the width of the
            # monkey body materials it replaced (MonkeyDiffuse _Outline 0.97).
            if baked:
                c = r["col"][used]
                vout.append(np.c_[c[:, :3], c[:, 3] * BAKED_OUTLINE].astype(np.float32))
            else:
                vout.append(np.tile(np.array([ocol[0], ocol[1], ocol[2], outline], dtype=np.float32), (len(used), 1)))
            idxs.append(rm[ind])
            remaps.append((r, used))
            parts.append({"name": r["name"], "material": mname, "texture": tex, "tint": [[1, 1, 1, 1]],
                          "outline": outline, "outline_color": ocol,
                          "vcount": int(len(used)), "icount": int(len(ind)), "voff": voff, "ioff": ioff,
                          "skinned": r["skin"] is not None})
            voff += len(used)
            ioff += len(ind)

    # The ground shadow: a SpriteRenderer named *RadialShadow* lying flat (MonkeyRadialShadow).
    shadow = None
    for tid in order:
        if "Shadow" not in T[tid]["name"] or not T[tid]["active"]:
            continue
        sr = comp(T[tid]["go"], "SpriteRenderer")
        if sr is None:
            continue
        srt = sr.read_typetree()
        sp_obj = em.resolve(af, srt["m_Sprite"])
        if sp_obj is None or not srt.get("m_Enabled", 1):
            continue
        sp = sp_obj.read()
        sp.image.save(os.path.join(OUT, "shadow.png"))
        m = W0[tid]
        scale = float(np.linalg.norm(m[:3, 0]))
        c = srt.get("m_Color", {"a": 1.0})
        shadow = {"size": sp.m_Rect.width / sp.m_PixelsToUnits * scale, "pos": [float(m[0, 3]), float(m[2, 3])],
                  "alpha": c.get("a", 1.0)}
        break

    anims = []
    with open(os.path.join(OUT, "anim.bin"), "wb") as f:
        off = 0
        for c in clips:
            dur = max(c.stop - c.start, 1.0 / FPS)
            nframes = max(1, int(round(dur * FPS)) + (0 if c.loop else 1))
            for k in range(nframes):
                t = c.start + k / FPS
                W = world_mats_fast(c.pose(t))
                cache = {}
                for r, used in remaps:
                    key = id(r)
                    if key not in cache:
                        cache[key] = pose_vertices(W, r)
                    pos, nor = cache[key]
                    f.write(np.c_[pos[used], nor[used]].astype(np.float32).tobytes())
            anims.append({"name": c.name, "frames": nframes, "fps": FPS, "loop": c.loop,
                          "duration": dur, "events": c.events, "offset": off})
            if getattr(c, "role", None):
                anims[-1]["role"] = c.role
                if getattr(c, "weight", None) is not None:
                    anims[-1]["weight"] = c.weight
            off += nframes
            print(f"  clip {c.name}: {nframes} frames, loop={c.loop}, events={c.events}")

    with open(os.path.join(OUT, "verts.bin"), "wb") as f:
        for x in verts:
            f.write(x.tobytes())
    with open(os.path.join(OUT, "idx.bin"), "wb") as f:
        for x in idxs:
            f.write(x.astype(np.uint32).tobytes())
    with open(os.path.join(OUT, "outline.bin"), "wb") as f:
        for x in vout:
            f.write(x.tobytes())
    json.dump({"parts": parts, "anims": anims, "vertex_count": voff, "shadow": shadow},
              open(os.path.join(OUT, "meta.json"), "w"), indent=1)
    print(f"{len(parts)} parts, {voff} verts; anim.bin {sum(a['frames'] for a in anims)} frames")


if __name__ == "__main__":
    main()
