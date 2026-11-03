"""Export 3D renderers (static + skinned, posed by their bones) to a compact format.

Two modes:
  prefab <bundle-glob> <container path>   e.g. asset_references_assets_all "Assets/Monkeys/DartMonkey/Graphics/DartMonkey.prefab"
  scene  <bundle-glob>                    every active renderer in a scene bundle (e.g. map_tutorial)

Usage:
  tools/.venv/Scripts/python.exe tools/assets/export_model.py <game install> <out_dir> prefab|scene <bundle-glob> [container path]

Output (<out_dir>/): meta.json {parts: [{texture, vcount, icount, voff, ioff, name}]},
verts.bin (f32 x y z u v per vertex, Unity world/prefab space), idx.bin (u32), tex/*.png.
Proprietary art: local use only.
"""
import glob
import json
import os
import tempfile
import sys

# The launcher uses isolated Python; resolve this script's shader helpers explicitly.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from export_cache import TextureCopies

_png_copies = TextureCopies()

import numpy as np
import UnityPy
from UnityPy.helpers.MeshHelper import MeshHandler


GAME, OUT, MODE, BUNDLE = sys.argv[1:5]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
_shader_names = {}
def shader_name(mat_obj, mt):
    """Follow external shader refs through the CAB index, including main-menu materials."""
    sh = mt["m_Shader"]
    key = (mat_obj.assets_file.name, sh["m_FileID"], sh["m_PathID"])
    if key not in _shader_names:
        try:
            obj = resolve(mat_obj.assets_file, sh)
            _shader_names[key] = obj.read().m_ParsedForm.m_Name if obj else None
        except Exception:
            _shader_names[key] = None
    return _shader_names[key]


_shader_culls = {}
def shader_cull(mat_obj, mt):
    """{"cull": ShaderLab Cull of the shader's first SubShader (0 Off, 1 Front, 2 Back), its
    passes' in order when they differ; a `Cull [_Prop]` takes the material's float} or {}."""
    sh = mt["m_Shader"]
    key = (mat_obj.assets_file.name, sh["m_FileID"], sh["m_PathID"])
    if key not in _shader_culls:
        try:
            obj = resolve(mat_obj.assets_file, sh)
            subs = obj.read_typetree()["m_ParsedForm"]["m_SubShaders"] if obj else []
            _shader_culls[key] = [p["m_State"]["culling"] for p in subs[0]["m_Passes"]] if subs else None
        except Exception:
            _shader_culls[key] = None
    states = _shader_culls[key]
    if not states:
        return {}
    floats = dict(mt["m_SavedProperties"].get("m_Floats", []))
    culls = [int(floats.get(c["name"], c["val"]) if c.get("name") else c["val"]) for c in states]
    return {"cull": culls[0] if len(set(culls)) == 1 else culls}


def _find_cab_index(start):
    d = os.path.abspath(start)
    while True:
        cand = os.path.join(d, "cab_index.json")
        if os.path.exists(cand):
            return cand
        parent = os.path.dirname(d)
        if parent == d:
            raise FileNotFoundError("cab_index.json not found above " + start)
        d = parent


CAB_INDEX = json.load(open(_find_cab_index(OUT)))
_envs = {}


def bundle_file(name):
    """Bundle `name` as installed: Full, else Half (asset_library_assets_all ships only there)."""
    f = os.path.join(AA, name)
    half = os.path.join(os.path.dirname(AA), "Half", name)
    return f if os.path.exists(f) or not os.path.exists(half) else half


def env_for_bundle(name):
    if name not in _envs:
        _envs[name] = UnityPy.load(bundle_file(name))
    return _envs[name]


def resolve(af, pptr):
    """Object reader for a PPtr inside assets file `af` (follows external refs)."""
    fid, pid = pptr["m_FileID"], pptr["m_PathID"]
    if pid == 0:
        return None
    if fid == 0:
        return af.objects.get(pid)
    ext = af.externals[fid - 1].path.split("/")[-1]
    # Scene .sharedAssets are in the bundle already being exported. Prefer it
    # over the global index: game updates can leave an older bundle with the
    # same CAB name but different object IDs on disk.
    for loaded in list(_envs.values()):
        siblings = [sub for f in loaded.files.values() for sub in getattr(f, "files", {}).values()]
        if not any(sub is af for sub in siblings):
            continue
        for sub in siblings:
            if getattr(sub, "name", "") == ext and hasattr(sub, "objects"):
                return sub.objects.get(pid)
    bundle = CAB_INDEX.get(ext.lower())  # cab_index.py keys are lower case
    if bundle is None:
        return None
    env = env_for_bundle(bundle)
    for f in env.files.values():
        for sub in getattr(f, "files", {}).values():
            if getattr(sub, "name", "") == ext and hasattr(sub, "objects"):
                return sub.objects.get(pid)
    return None


def trs(tr):
    p, q, s = tr["m_LocalPosition"], tr["m_LocalRotation"], tr["m_LocalScale"]
    x, y, z, w = q["x"], q["y"], q["z"], q["w"]
    r = np.array([[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
                  [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
                  [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]])
    m = np.eye(4)
    m[:3, :3] = r @ np.diag([s["x"], s["y"], s["z"]])
    m[:3, 3] = [p["x"], p["y"], p["z"]]
    return m


# Prefab exports: the root's own position and rotation are dropped (its scale kept). The game
# places an instantiated display on its entity: UnpoppedArmyMoab's root is saved at x 64.9 and
# turned 90 degrees about x (which drew the Prince's undead MOAB off its projectile, nose down);
# Moab3D's root is plain.
PREFAB_ROOT_AT_ORIGIN = False


def world_matrix(af, tr_pid, stop=None):
    m = np.eye(4)
    while tr_pid and tr_pid != stop:
        t = af.objects[tr_pid].read_typetree()
        tm = trs(t)
        if PREFAB_ROOT_AT_ORIGIN and not t["m_Father"]["m_PathID"]:
            sc = t["m_LocalScale"]
            tm = np.diag([sc["x"], sc["y"], sc["z"], 1.0])
        m = tm @ m
        tr_pid = t["m_Father"]["m_PathID"]
    return m


def mat4(m):
    return np.array([[getattr(m, f"e{r}{c}") for c in range(4)] for r in range(4)])


tex_cache = {}


def finite(o):
    """JSON-safe copy: infinities become +-3e38 and NaN 0 (strict JSON readers reject them)."""
    if isinstance(o, float):
        return 0.0 if o != o else (3e38 if o == float("inf") else (-3e38 if o == float("-inf") else o))
    if isinstance(o, dict):
        return {k: finite(v) for k, v in o.items()}
    if isinstance(o, (list, tuple)):
        return [finite(v) for v in o]
    return o

# Shaders drawn by translating the game's own bytecode (dxbc_glsl.py): their look comes from the
# shader (animated water, waterfalls, god rays), not from a texture. The GLSL goes to
# local_extracted/shaders/<name>.json, the material's properties into the part.
TRANSLATED = {
    "Shader Graphs/WaterRipples", "Shader Graphs/WaterPanning", "Shader Graphs/Waterfall",
    "Shader Graphs/WaterfallSulfur", "Shader Graphs/WaterSplash", "NinjaKiwi/BTD6/Pool",
    "NinjaKiwi/BTD6/UnlitWithDiffuseTransparent", "NinjaKiwi/BTD6/UnlitWithDiffuseScrolling",
    "Shader Graphs/ScrollingTransparentCutOut", "NinjaKiwi/BTD6/GodRay", "Shader Graphs/Cloud1",
    "Shader Graphs/ParticleGlow", "Shader Graphs/ShoreFoam",
    # Projectile meshes with no texture: the Wizard Lord Phoenix's lava ball, the Plasma Accelerator beam.
    "Shader Graphs/FireballCore", "Shader Graphs/BeamLightMesh",
}
# local_extracted/shaders, wherever under local_extracted the model goes (maps: models/map_*;
# projectiles: models/projectiles/*).
_le = os.path.abspath(OUT)
while os.path.basename(_le) != "local_extracted" and os.path.dirname(_le) != _le:
    _le = os.path.dirname(_le)
SHADER_DIR = os.path.join(_le, "shaders") if os.path.basename(_le) == "local_extracted" else os.path.join(os.path.dirname(os.path.abspath(OUT)), "..", "shaders")
shader_done = {}


def shader_file(name):
    return name.replace("/", "_").replace(" ", "_") + ".json"


def export_shader(mat_obj, mt, name):
    """Translate the material's shader once per name; False if it can't be."""
    if name not in shader_done:
        ok = False
        try:
            import dxbc_glsl
            so = resolve(mat_obj.assets_file, mt["m_Shader"])
            if so is None and mt["m_Shader"]["m_FileID"] == 0:
                so = mat_obj.assets_file.objects.get(mt["m_Shader"]["m_PathID"])
            if so is not None:
                r = dxbc_glsl.translate(so)
                os.makedirs(SHADER_DIR, exist_ok=True)
                # Multiple map/model workers may translate the same shared shader.
                # Publish complete JSON atomically, never a partially written file.
                fd, pending = tempfile.mkstemp(prefix="shader-", suffix=".pending", dir=SHADER_DIR)
                try:
                    with os.fdopen(fd, "w", encoding="utf-8") as stream:
                        json.dump(finite(r), stream, indent=1)
                    os.replace(pending, os.path.join(SHADER_DIR, shader_file(name)))
                finally:
                    if os.path.exists(pending):
                        os.unlink(pending)
                ok = True
        except Exception as e:
            print("shader translation failed", name, type(e).__name__, e)
        shader_done[name] = ok
    return shader_done[name]


def export_tex_prop(mat_obj, env_entry):
    """One texture property's image (tex/<name>.png) and its wrap mode (0 repeat, 1 clamp)."""
    t = resolve(mat_obj.assets_file, env_entry["m_Texture"])
    if t is None:
        return None
    k = (t.assets_file.name, t.path_id)
    if k not in tex_cache:
        texture = _png_copies.source(t)
        try:
            tex_cache[k] = _png_copies.write(t, OUT, texture)
        except Exception as e:
            print("texture fail", t.path_id, e)
            tex_cache[k] = None
    if tex_cache[k] is None:
        return None
    ts = t.read_typetree().get("m_TextureSettings", {})
    return {"file": tex_cache[k], "wrap": [ts.get("m_WrapU", 0), ts.get("m_WrapV", 0)],
            "scale": [env_entry["m_Scale"]["x"], env_entry["m_Scale"]["y"]],
            "offset": [env_entry["m_Offset"]["x"], env_entry["m_Offset"]["y"]]}


def export_texture(mat_obj):
    mat = mat_obj.read_typetree()
    envs = dict((e[0], e[1]) for e in mat["m_SavedProperties"]["m_TexEnvs"])
    for key in ("_MainTex", "_BaseMap", "_MainTexture", "_Albedo", "_Diffuse"):
        if key in envs and envs[key]["m_Texture"]["m_PathID"]:
            t = resolve(mat_obj.assets_file, envs[key]["m_Texture"])
            if t is None:
                return None
            k = (t.assets_file.name, t.path_id)
            if k not in tex_cache:
                texture = _png_copies.source(t)
                try:
                    tex_cache[k] = _png_copies.write(t, OUT, texture)
                except Exception as e:
                    print("texture fail", t.path_id, e)
                    tex_cache[k] = None
            return tex_cache[k]
    return None


parts, verts, idxs = [], [], []
# Normals (f32 x y z, Unity world space) of translated-shader parts, in normals.bin; a part's
# "noff" is its first vertex there (view-dependent shaders: fresnel).
vnormals = []
# Vertex colours (RGBA bytes) of translated-shader parts, in colors.bin; a part's "coff" is its
# first vertex there.
vcolors = []


# Scene objects of modes this game doesn't have (the home island's Race, Contested Territory and
# Odyssey props): not exported.
SKIP = ("MainMenuWorld/Seasonal/",)


def add_renderer(r_obj, root_tr=None):
    af = r_obj.assets_file
    r = r_obj.read_typetree()
    if not r.get("m_Enabled", 1):
        return
    go = af.objects[r["m_GameObject"]["m_PathID"]].read_typetree()
    if not go.get("m_IsActive", 1):
        return
    # Text and UI meshes placed in a scene have a RectTransform.
    tr_pid = next(c["component"]["m_PathID"] for c in go["m_Component"]
                  if af.objects[c["component"]["m_PathID"]].type.name in ("Transform", "RectTransform"))
    hierarchy = []
    ancestor = tr_pid
    while ancestor:
        transform = af.objects[ancestor].read_typetree()
        hierarchy.append(af.objects[transform["m_GameObject"]["m_PathID"]].read_typetree()["m_Name"])
        ancestor = transform["m_Father"]["m_PathID"]
    hierarchy = "/".join(reversed(hierarchy))
    if hierarchy.startswith(SKIP):
        return
    # Inactive ancestors (e.g. seasonal decoration groups) hide the whole subtree.
    t = af.objects[tr_pid].read_typetree()["m_Father"]["m_PathID"]
    while t and t != root_tr:
        tt = af.objects[t].read_typetree()
        if not af.objects[tt["m_GameObject"]["m_PathID"]].read_typetree().get("m_IsActive", 1):
            return
        t = tt["m_Father"]["m_PathID"]
    skinned = r_obj.type.name == "SkinnedMeshRenderer"
    if skinned:
        mesh_obj = resolve(af, r["m_Mesh"])
    else:
        mf = next((af.objects[c["component"]["m_PathID"]] for c in go["m_Component"]
                   if af.objects[c["component"]["m_PathID"]].type.name == "MeshFilter"), None)
        mesh_obj = resolve(af, mf.read_typetree()["m_Mesh"]) if mf else None
    if mesh_obj is None:
        return
    mesh = mesh_obj.read()
    h = MeshHandler(mesh)
    h.process()
    v = np.array(h.m_Vertices, dtype=np.float64).reshape(-1, 3)
    if len(v) == 0:
        return
    uv = np.array(h.m_UV0 if h.m_UV0 else [(0, 0)] * len(v), dtype=np.float64).reshape(-1, 2)
    vh = np.c_[v, np.ones(len(v))]
    bones = r.get("m_Bones") or []
    if skinned and bones and h.m_BoneWeights:
        bind = [mat4(b) for b in mesh.m_BindPose]
        bone_m = []
        for b in bones:
            if b["m_PathID"] and b["m_PathID"] in af.objects:
                bone_m.append(world_matrix(af, b["m_PathID"], root_tr))
            else:
                bone_m.append(np.eye(4))
        skin = [bone_m[i] @ bind[i] if i < len(bind) else bone_m[i] for i in range(len(bone_m))]
        w = np.array(h.m_BoneWeights, dtype=np.float64).reshape(len(v), -1)
        bi = np.array(h.m_BoneIndices, dtype=np.int64).reshape(len(v), -1)
        out = np.zeros((len(v), 4))
        for k in range(w.shape[1]):
            ms = np.stack([skin[j] if j < len(skin) else np.eye(4) for j in bi[:, k]])
            out += w[:, k:k + 1] * np.einsum("nij,nj->ni", ms, vh)
        world = out[:, :3]
    else:
        m = world_matrix(af, tr_pid, root_tr)
        world = (vh @ m.T)[:, :3]
    tris = h.get_triangles()
    for sub, mref in zip(tris, r["m_Materials"]):
        mat_obj = resolve(af, mref)
        if mat_obj is None or mat_obj.type.name != "Material":
            continue
        tex = export_texture(mat_obj)
        ind = np.array(sub, dtype=np.uint32).reshape(-1)
        if len(ind) == 0:
            continue
        used = np.unique(ind)
        remap = np.zeros(len(v), dtype=np.uint32)
        remap[used] = np.arange(len(used), dtype=np.uint32)
        voff = sum(len(x) for x in verts)
        ioff = sum(len(x) for x in idxs)
        verts.append(np.c_[world[used], uv[used]].astype(np.float32))
        idxs.append(remap[ind])
        mt = mat_obj.read_typetree()
        cols = dict(mt["m_SavedProperties"].get("m_Colors", []))
        grad = [cols[k] for k in ("_ColorBot", "_ColorMid", "_ColorTop") if k in cols]
        base = cols.get("_Color") or cols.get("_BaseColor") or {"r": 1, "g": 1, "b": 1, "a": 1}
        tint = [[c["r"], c["g"], c["b"], c["a"]] for c in grad] if len(grad) == 3 else [[base["r"], base["g"], base["b"], base["a"]]]
        extra = {}
        if len(grad) == 3:
            # Gradient_3Color ramps along UV v with the kink at _Middle (see render3d::Grad).
            extra["middle"] = dict(mt["m_SavedProperties"].get("m_Floats", [])).get("_Middle", 0.5)
            extra["shader"] = shader_name(mat_obj, mt)
        sh = shader_name(mat_obj, mt)
        if sh is None:
            so = resolve(mat_obj.assets_file, mt["m_Shader"])
            try:
                sh = so.read().m_ParsedForm.m_Name if so else None
            except Exception:
                sh = None
        if sh is not None:
            extra["shader"] = sh
        extra.update(shader_cull(mat_obj, mt))
        if sh in TRANSLATED and export_shader(mat_obj, mt, sh):
            sp = mt["m_SavedProperties"]
            texs = {}
            for k, e in sp.get("m_TexEnvs", []):
                if e["m_Texture"]["m_PathID"]:
                    x = export_tex_prop(mat_obj, e)
                    if x:
                        texs[k] = x
            extra["shader"] = sh
            extra["shader_file"] = shader_file(sh)
            extra["props"] = {"floats": dict(sp.get("m_Floats", [])),
                              "colors": {k: [c["r"], c["g"], c["b"], c["a"]] for k, c in sp.get("m_Colors", [])},
                              "textures": texs}
            extra["queue"] = mt.get("m_CustomRenderQueue", -1)
            if h.m_Colors:
                cols_v = np.clip(np.array(h.m_Colors, dtype=np.float64).reshape(-1, 4)[used] * 255.0 + 0.5, 0, 255)
                extra["coff"] = sum(len(x) for x in vcolors)
                vcolors.append(cols_v.astype(np.uint8))
            if h.m_Normals:
                # Some meshes store 4-component normals (Quarry's waterfall), as export_animated.py.
                nrm = np.array(h.m_Normals, dtype=np.float64)
                nrm = nrm.reshape(len(v), -1)[:, :3] if nrm.size % len(v) == 0 else nrm.reshape(-1, 3)[: len(v)]
                if not skinned:
                    nrm = nrm @ world_matrix(af, tr_pid, root_tr)[:3, :3].T
                nrm /= np.maximum(np.linalg.norm(nrm, axis=1, keepdims=True), 1e-9)
                extra["noff"] = sum(len(x) for x in vnormals)
                vnormals.append(nrm[used].astype(np.float32))
        parts.append({"name": go["m_Name"], "path": hierarchy, "material": mt["m_Name"], "texture": tex, "tint": tint, **extra,
                      "vcount": int(len(used)), "icount": int(len(ind)), "voff": voff, "ioff": ioff,
                      "skinned": skinned})


def main(env=None):
    bundle = (glob.glob(os.path.join(AA, BUNDLE + "*.bundle")) or glob.glob(os.path.join(os.path.dirname(AA), "Half", BUNDLE + "*.bundle")))[0]
    if env is None:
        env = UnityPy.load(bundle)
    _envs[os.path.basename(bundle)] = env
    os.makedirs(OUT, exist_ok=True)
    global PREFAB_ROOT_AT_ORIGIN
    PREFAB_ROOT_AT_ORIGIN = MODE == "prefab"
    if MODE == "prefab":
        path = sys.argv[5]
        ab = next(o for o in env.objects if o.type.name == "AssetBundle").read_typetree()
        pid = next((v["asset"]["m_PathID"] for k, v in ab["m_Container"] if k == path), None)
        if pid is not None:
            root = next(o for o in env.objects if o.path_id == pid)
        else:
            # No container entry (asset_library_assets_all has none): the root GameObject named
            # like the prefab.
            stem = os.path.splitext(os.path.basename(path))[0]
            def is_root(o):
                if o.type.name != "GameObject":
                    return False
                g = o.read_typetree()
                if g["m_Name"] != stem:
                    return False
                tr = next(o.assets_file.objects[c["component"]["m_PathID"]] for c in g["m_Component"]
                          if o.assets_file.objects[c["component"]["m_PathID"]].type.name == "Transform")
                return not tr.read_typetree()["m_Father"]["m_PathID"]
            root = next(o for o in env.objects if is_root(o))
        af = root.assets_file
        g = root.read_typetree()
        root_tr = next(c["component"]["m_PathID"] for c in g["m_Component"]
                       if af.objects[c["component"]["m_PathID"]].type.name == "Transform")
        stack = [root_tr]
        while stack:
            t = af.objects[stack.pop()].read_typetree()
            go = af.objects[t["m_GameObject"]["m_PathID"]].read_typetree()
            for c in go["m_Component"]:
                o = af.objects.get(c["component"]["m_PathID"])
                if o is not None and o.type.name in ("MeshRenderer", "SkinnedMeshRenderer"):
                    add_renderer(o, None)
            stack.extend(ch["m_PathID"] for ch in t["m_Children"])
    else:
        # Optional 5th argument "cab:<CAB-name>": only that scene file of a multi-scene bundle.
        only = sys.argv[5][4:].lower() if len(sys.argv) > 5 and sys.argv[5].startswith("cab:") else None
        for o in env.objects:
            if only and o.assets_file.name.lower() != only:
                continue
            if o.type.name in ("MeshRenderer", "SkinnedMeshRenderer"):
                # A scene's .sharedAssets holds NestedPrefab sources (helicopter rotors), not placed
                # anywhere: they would all sit at the world origin.
                if o.assets_file.name.endswith(".sharedAssets"):
                    continue
                try:
                    add_renderer(o)
                except Exception as e:
                    print("skip renderer", o.path_id, type(e).__name__, e)
    with open(os.path.join(OUT, "verts.bin"), "wb") as f:
        for x in verts:
            f.write(x.tobytes())
    with open(os.path.join(OUT, "idx.bin"), "wb") as f:
        for x in idxs:
            f.write(x.astype(np.uint32).tobytes())
    if vcolors:
        with open(os.path.join(OUT, "colors.bin"), "wb") as f:
            for x in vcolors:
                f.write(x.tobytes())
    elif os.path.exists(os.path.join(OUT, "colors.bin")):
        os.remove(os.path.join(OUT, "colors.bin"))
    if vnormals:
        with open(os.path.join(OUT, "normals.bin"), "wb") as f:
            for x in vnormals:
                f.write(x.tobytes())
    elif os.path.exists(os.path.join(OUT, "normals.bin")):
        os.remove(os.path.join(OUT, "normals.bin"))
    allv = np.concatenate(verts) if verts else np.zeros((0, 5))
    bbox = (allv[:, :3].min(0).tolist(), allv[:, :3].max(0).tolist()) if len(allv) else None
    json.dump(finite({"parts": parts, "bbox": bbox}), open(os.path.join(OUT, "meta.json"), "w"), indent=1)
    print(f"{len(parts)} parts, {len(allv)} verts, {sum(len(x) for x in idxs) // 3} tris, bbox {bbox}")
    for p in parts[:40]:
        print("  ", p["name"], p["material"], p["texture"], p["vcount"], "skinned" if p["skinned"] else "")


if __name__ == "__main__":
    main()
