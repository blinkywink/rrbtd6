"""Map props' idle animation: what each scene Animator plays from the start, for the renderer
(renderer/src/map_motion.rs) to move the map's own model parts.

    tools/.venv/Scripts/python.exe -I tools/assets/export_map_motion.py <game install> [map key ...]

Writes local_extracted/models/map_<key>/motion.json (none when nothing moves):
    [{rig: {nodes, parent, start, stop, looped, speed}, parts, random_start, speed_range}]
the same format as the home world's vegetation.json (export_home_vegetation.py, home_world.rs):
nodes are the animated GameObject's whole subtree (path, parent index, local TRS, rest world matrix,
channels: the clip's own cubic curves per transform attribute, 1 position 2 rotation 3 scale 4
euler); parts are the skinned meshes whose bones are all in it (rest vertices, weights, bone
indices, bind poses), checked against the exported map model's rest pose. A rigid part moves with
its own node.

Which clip: a PlayableAnimatorSimple (dontPlayOnInit 0) plays its animationClip at playbackSpeed
(randomStartTime, randomizeSpeed); otherwise an enabled Animator plays its controller's layer 0
default state (an AnimatorOverrideController's override of it) at the state's speed. Only the
transform curves of the clip; GameObject toggles and material curves are left. Covered Garden's
pond fish and lily pads, every map's swaying trees and bushes. Proprietary data: local use only.
"""
import glob
import json
import os
import sys
import zlib

import numpy as np
import UnityPy
from UnityPy.helpers.MeshHelper import MeshHandler

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mecanim_clip import Clip

GAME = sys.argv[1]
KEYS = sys.argv[2:]
L = "local_extracted"
sys.argv = [sys.argv[0], GAME, os.path.join(L, "models", "_motion_unused"), "scene", "map_"]
import export_model as em  # noqa: E402  (reads sys.argv at import)


def channels_of(clip_tree):
    """{(path hash, attribute): [curve keys [t, a, b, c, d] per component]}, the clip's own
    curves: streamed Hermite segments, dense samples as linear segments, constants."""
    c = Clip(clip_tree)
    curves = [list(map(list, k)) for k in c.keys[:c.n_streamed]]
    if c.n_dense:
        rate, begin = c.dense_rate, c.dense_begin
        for ch in range(c.n_dense):
            col = c.dense[:, ch]
            keys = []
            for f in range(len(col)):
                slope = float((col[f + 1] - col[f]) * rate) if f + 1 < len(col) else 0.0
                keys.append([begin + f / rate, 0.0, 0.0, slope, float(col[f])])
            curves.append(keys)
    curves += [[[0.0, 0.0, 0.0, 0.0, float(v)]] for v in c.const]
    out = {}
    for path, attr, ci, n in c.pose_layout:
        out[(path, attr)] = curves[ci:ci + n]
    return c, out


def moves(chs):
    return any(len(k) > 1 or (k and any(abs(x) > 1e-9 for x in k[0][1:4])) for k in chs)


def differs(node, attr, chs):
    """A constant channel that still poses the node away from its scene rest value."""
    v = [k[0][4] for k in chs if k]
    if len(v) != len(chs):
        return False
    if attr == 1:
        return any(abs(a - b) > 1e-4 for a, b in zip(v, node["position"]))
    if attr == 3:
        return any(abs(a - b) > 1e-4 for a, b in zip(v, node["scale"]))
    if attr == 2:
        return abs(sum(a * b for a, b in zip(v, node["rotation"]))) < 0.99999
    return False


def main():
    keys = KEYS or sorted(d[4:] for d in os.listdir(os.path.join(L, "models")) if d.startswith("map_"))
    for key in keys:
        out_dir = os.path.join(L, "models", f"map_{key}")
        bundles = glob.glob(os.path.join(em.AA, f"map_{key}_scenes_all_*.bundle"))
        if not bundles or not os.path.exists(os.path.join(out_dir, "meta.json")):
            continue
        env = UnityPy.load(bundles[0])
        em._envs.clear()
        em._envs[os.path.basename(bundles[0])] = env
        try:
            export(key, env, out_dir)
        except Exception as e:
            print(key, "failed:", type(e).__name__, e, flush=True)
        em._envs.clear()
        del env


def export(key, env, out_dir):
    objs = [o for o in env.objects if not o.assets_file.name.endswith(".sharedAssets")]
    if not objs:
        return
    tree = {}

    def tt(o):
        if o.path_id not in tree:
            tree[o.path_id] = o.read_typetree()
        return tree[o.path_id]

    af = objs[0].assets_file
    transforms = {o.path_id: o for o in objs if o.type.name in ("Transform", "RectTransform")}
    go_tr = {}
    for pid, o in transforms.items():
        go_tr[tt(o)["m_GameObject"]["m_PathID"]] = pid
    gos = {o.path_id: o for o in objs if o.type.name == "GameObject"}

    def name(tid):
        return tt(gos[tt(transforms[tid])["m_GameObject"]["m_PathID"]])["m_Name"]

    def info(tid):
        names, active = [], True
        while tid:
            t = tt(transforms[tid])
            g = tt(gos[t["m_GameObject"]["m_PathID"]])
            names.append(g["m_Name"])
            active &= bool(g["m_IsActive"])
            tid = t["m_Father"]["m_PathID"]
        return "/".join(reversed(names)), active

    # What plays: (GameObject, clip object, speed, random start, speed range).
    plays = {}
    for o in objs:
        if o.type.name != "MonoBehaviour":
            continue
        try:
            s = tt(o)
        except Exception:
            continue
        if "dontPlayOnInit" not in s or "animationClip" not in s or s["dontPlayOnInit"] or not s.get("m_Enabled", 1):
            continue
        clip = em.resolve(o.assets_file, s["animationClip"])
        if clip is None:
            continue
        rng = [s["randomSpeedRange"][k] for k in ("min", "max")] if s.get("randomizeSpeed") else None
        plays[s["m_GameObject"]["m_PathID"]] = (clip, s.get("playbackSpeed", 1.0), bool(s.get("randomStartTime")), rng)
    for o in objs:
        if o.type.name != "Animator":
            continue
        a = tt(o)
        g = a["m_GameObject"]["m_PathID"]
        if g in plays or not a.get("m_Enabled", 1):
            continue
        ctl = em.resolve(o.assets_file, a["m_Controller"])
        if ctl is None:
            continue
        over = {}
        ct = ctl.read_typetree()
        if ctl.type.name == "AnimatorOverrideController":
            over = {(x["m_OriginalClip"]["m_FileID"], x["m_OriginalClip"]["m_PathID"]): x["m_OverrideClip"] for x in ct["m_Clips"] if x["m_OverrideClip"]["m_PathID"]}
            owner = ctl
            ctl = em.resolve(ctl.assets_file, ct["m_Controller"])
            if ctl is None:
                continue
            ct = ctl.read_typetree()
        else:
            owner = None
        sms = ct["m_Controller"]["m_StateMachineArray"]
        if not sms:
            continue
        sm = sms[0]["data"]
        states = sm["m_StateConstantArray"]
        d = sm.get("m_DefaultState", 0)
        if d >= len(states):
            continue
        st = states[d]["data"]
        bt = st.get("m_BlendTreeConstantArray") or []
        nodes = bt[0]["data"]["m_NodeArray"] if bt else []
        if len(nodes) != 1:
            continue  # no motion, or a blend tree
        ci = nodes[0]["data"]["m_ClipID"]
        if ci >= len(ct["m_AnimationClips"]):
            continue
        pp = ct["m_AnimationClips"][ci]
        clip = None
        if owner is not None:
            # The override's original clip PPtr is relative to the override controller's file.
            for (fid, pid), ov in over.items():
                orig = em.resolve(owner.assets_file, {"m_FileID": fid, "m_PathID": pid})
                base = em.resolve(ctl.assets_file, pp)
                if orig is not None and base is not None and orig.path_id == base.path_id:
                    clip = em.resolve(owner.assets_file, ov)
        clip = clip or em.resolve(ctl.assets_file, pp)
        if clip is not None:
            plays[g] = (clip, st.get("m_Speed", 1.0), False, None)

    smrs = [o for o in objs if o.type.name == "SkinnedMeshRenderer"]
    meta = json.load(open(os.path.join(out_dir, "meta.json")))
    static = np.fromfile(os.path.join(out_dir, "verts.bin"), dtype="<f4").reshape(-1, 5)
    by_path = {}
    for p in meta["parts"]:
        by_path.setdefault(p.get("path"), []).append(p)
    out, skipped, worst = [], 0, 0.0
    for g, (clip_obj, speed, rand, rng) in plays.items():
        tid = go_tr.get(g)
        if tid is None:
            continue
        path, active = info(tid)
        if not active:
            continue
        clip, chans = channels_of(clip_obj.read_typetree())
        nodes, ids, hashes = [], {}, {}
        stack = [(tid, None)]
        while stack:
            t_id, parent = stack.pop()
            t = tt(transforms[t_id])
            p, _ = info(t_id)
            index = len(nodes)
            ids[t_id] = index
            hashes[zlib.crc32(p[len(path):].lstrip("/").encode())] = index
            nodes.append({"path": p, "parent": parent, "position": [t["m_LocalPosition"][a] for a in "xyz"],
                          "rotation": [t["m_LocalRotation"][a] for a in "xyzw"], "scale": [t["m_LocalScale"][a] for a in "xyz"],
                          "rest": em.world_matrix(af, t_id).T.flatten().tolist(), "channels": []})
            stack.extend((c["m_PathID"], index) for c in reversed(t["m_Children"]) if c["m_PathID"] in transforms)
        animated = False
        # Every transform channel the clip binds in the subtree (Unity writes constant ones too).
        for (h, attr), cs in chans.items():
            if h in hashes:
                node = nodes[hashes[h]]
                node["channels"].append({"attribute": attr, "curves": cs})
                animated |= moves(cs) or differs(node, attr, cs)
        if not animated:
            continue
        parts = []
        for r_obj in smrs:
            r = tt(r_obj)
            bones = [b["m_PathID"] for b in r["m_Bones"]]
            if not bones or not all(b in ids for b in bones) or not r.get("m_Enabled", 1):
                continue
            rtid = go_tr.get(r["m_GameObject"]["m_PathID"])
            rpath, ractive = info(rtid) if rtid else (None, False)
            if not ractive:
                continue
            mesh = em.resolve(r_obj.assets_file, r["m_Mesh"])
            if mesh is None:
                continue
            mesh = mesh.read()
            h = MeshHandler(mesh)
            h.process()
            vertices = np.asarray(h.m_Vertices, dtype=float).reshape(-1, 3)
            if not h.m_BoneWeights:
                continue
            weights = np.asarray(h.m_BoneWeights, dtype=float).reshape(len(vertices), -1)
            indices = np.asarray(h.m_BoneIndices, dtype=int).reshape(len(vertices), -1)
            binds = [em.mat4(b).T.flatten().tolist() for b in mesh.m_BindPose]
            bone_ix = [ids[b] for b in bones]
            mine = by_path.get(rpath, [])
            slot = 0
            for sub, mref in zip(h.get_triangles(), r["m_Materials"]):
                mat = em.resolve(r_obj.assets_file, mref)
                if mat is None or mat.type.name != "Material":
                    continue
                ind = np.asarray(sub, dtype=np.uint32).reshape(-1)
                if len(ind) == 0:
                    continue
                used = np.unique(ind)
                part = {"path": rpath, "vertices": vertices[used].tolist(), "weights": weights[used].tolist(),
                        "indices": indices[used].tolist(), "bones": bone_ix, "binds": binds}
                chunks = (len(ind) + 4997) // 4998
                # Check against the exported rest pose (the same part in meta.json).
                native = mine[slot] if slot < len(mine) else None
                slot += 1
                if native is None or native["vcount"] != len(used):
                    if os.environ.get("RRBTD6_MOTION_DEBUG"):
                        print("  skip", rpath, "vcount", native and native["vcount"], len(used), len(mine))
                    skipped += 1
                    continue
                expected = static[native["voff"]:native["voff"] + native["vcount"], :3]
                pos = np.c_[vertices[used], np.ones(len(used))]
                actual = np.zeros((len(used), 4))
                for k in range(weights.shape[1]):
                    w = weights[used, k]
                    bi = indices[used, k]
                    for b in np.unique(bi):
                        sel = bi == b
                        rest = np.asarray(nodes[bone_ix[b]]["rest"]).reshape(4, 4).T
                        bind = np.asarray(binds[b]).reshape(4, 4).T
                        actual[sel] += w[sel, None] * (pos[sel] @ (rest @ bind).T)
                err = float(np.max(np.abs(expected - actual[:, :3]))) if len(used) else 0.0
                if err > 0.01:
                    if os.environ.get("RRBTD6_MOTION_DEBUG"):
                        print("  skip", rpath, "rest error", err)
                    skipped += 1
                    continue
                worst = max(worst, err)
                parts.extend([part] * chunks)
        t = tt(transforms[tid])
        out.append({"rig": {"nodes": nodes, "parent": em.world_matrix(af, t["m_Father"]["m_PathID"]).T.flatten().tolist(),
                            "start": clip.start, "stop": clip.stop, "looped": clip.loop, "speed": speed},
                    "parts": parts, "random_start": rand, "speed_range": rng})
    target = os.path.join(out_dir, "motion.json")
    # motion_paths.json: every scene path whose parts can move, for Model::load to keep them out
    # of the static map batches.
    listing = os.path.join(out_dir, "motion_paths.json")
    if out:
        json.dump(out, open(target, "w"), allow_nan=False, separators=(",", ":"))
        json.dump(sorted({n["path"] for o in out for n in o["rig"]["nodes"]} | {p["path"] for o in out for p in o["parts"]}),
                  open(listing, "w"), separators=(",", ":"))
    else:
        for f in (target, listing):
            if os.path.exists(f):
                os.remove(f)
    print(key, len(out), "animated objects,", sum(len(o["parts"]) for o in out), "skinned parts,",
          skipped, "skipped; rest error", round(worst, 6), flush=True)


if __name__ == "__main__":
    main()
