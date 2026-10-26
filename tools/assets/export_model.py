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
import struct
import sys

import numpy as np
import UnityPy
from UnityPy.helpers import TypeTreeHelper
from UnityPy.helpers.MeshHelper import MeshHandler

TypeTreeHelper.read_typetree_boost = False

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


def env_for_bundle(name):
    if name not in _envs:
        _envs[name] = UnityPy.load(os.path.join(AA, name))
    return _envs[name]


def resolve(af, pptr):
    """Object reader for a PPtr inside assets file `af` (follows external refs)."""
    fid, pid = pptr["m_FileID"], pptr["m_PathID"]
    if pid == 0:
        return None
    if fid == 0:
        return af.objects.get(pid)
    ext = af.externals[fid - 1].path.split("/")[-1]
    bundle = CAB_INDEX.get(ext)
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


def world_matrix(af, tr_pid, stop=None):
    m = np.eye(4)
    while tr_pid and tr_pid != stop:
        t = af.objects[tr_pid].read_typetree()
        m = trs(t) @ m
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
}
SHADER_DIR = os.path.join(os.path.dirname(os.path.abspath(OUT)), "..", "shaders")
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
                json.dump(finite(r), open(os.path.join(SHADER_DIR, shader_file(name)), "w"), indent=1)
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
        tex = t.read()
        name = f"{tex.m_Name}_{abs(t.path_id) % 100000}.png"
        os.makedirs(os.path.join(OUT, "tex"), exist_ok=True)
        try:
            tex.image.save(os.path.join(OUT, "tex", name))
            tex_cache[k] = "tex/" + name
        except Exception as e:
            print("texture fail", tex.m_Name, e)
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
                tex = t.read()
                name = f"{tex.m_Name}_{abs(t.path_id) % 100000}.png"
                os.makedirs(os.path.join(OUT, "tex"), exist_ok=True)
                try:
                    tex.image.save(os.path.join(OUT, "tex", name))
                    tex_cache[k] = "tex/" + name
                except Exception as e:
                    print("texture fail", tex.m_Name, e)
                    tex_cache[k] = None
            return tex_cache[k]
    return None


parts, verts, idxs = [], [], []
# Vertex colours (RGBA bytes) of translated-shader parts, in colors.bin; a part's "coff" is its
# first vertex there.
vcolors = []


def add_renderer(r_obj, root_tr=None):
    af = r_obj.assets_file
    r = r_obj.read_typetree()
    if not r.get("m_Enabled", 1):
        return
    go = af.objects[r["m_GameObject"]["m_PathID"]].read_typetree()
    if not go.get("m_IsActive", 1):
        return
    tr_pid = next(c["component"]["m_PathID"] for c in go["m_Component"]
                  if af.objects[c["component"]["m_PathID"]].type.name == "Transform")
    hierarchy = []
    ancestor = tr_pid
    while ancestor:
        transform = af.objects[ancestor].read_typetree()
        hierarchy.append(af.objects[transform["m_GameObject"]["m_PathID"]].read_typetree()["m_Name"])
        ancestor = transform["m_Father"]["m_PathID"]
    hierarchy = "/".join(reversed(hierarchy))
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
        parts.append({"name": go["m_Name"], "path": hierarchy, "material": mt["m_Name"], "texture": tex, "tint": tint, **extra,
                      "vcount": int(len(used)), "icount": int(len(ind)), "voff": voff, "ioff": ioff,
                      "skinned": skinned})


def main():
    bundle = glob.glob(os.path.join(AA, BUNDLE + "*.bundle"))[0]
    env = UnityPy.load(bundle)
    _envs[os.path.basename(bundle)] = env
    os.makedirs(OUT, exist_ok=True)
    if MODE == "prefab":
        path = sys.argv[5]
        ab = next(o for o in env.objects if o.type.name == "AssetBundle").read_typetree()
        pid = next(v["asset"]["m_PathID"] for k, v in ab["m_Container"] if k == path)
        root = next(o for o in env.objects if o.path_id == pid)
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
    allv = np.concatenate(verts) if verts else np.zeros((0, 5))
    bbox = (allv[:, :3].min(0).tolist(), allv[:, :3].max(0).tolist()) if len(allv) else None
    json.dump(finite({"parts": parts, "bbox": bbox}), open(os.path.join(OUT, "meta.json"), "w"), indent=1)
    print(f"{len(parts)} parts, {len(allv)} verts, {sum(len(x) for x in idxs) // 3} tris, bbox {bbox}")
    for p in parts[:40]:
        print("  ", p["name"], p["material"], p["texture"], p["vcount"], "skinned" if p["skinned"] else "")


if __name__ == "__main__":
    main()
