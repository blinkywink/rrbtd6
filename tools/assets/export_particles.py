"""Export Unity particle-system prefabs (Shuriken) for the renderer's particle player.

Usage: tools/.venv/Scripts/python.exe tools/assets/export_particles.py <game install> <out dir> <prefab path>...
       ... <out dir> scene:<map key>    every ambient particle effect placed in that map's scene:
       one effect per top-level particle object (map_<key>_<n>), and <out dir>/map_<key>.json
       listing them with their world positions.

Writes <out dir>/<prefab name>/effect.json and the particle textures (tex/<name>.png). Each
GameObject with a ParticleSystem becomes one "system": its transform relative to the prefab
root, and the modules the renderer simulates (src/particles.rs), in compact form:

  curve    {"m": minMaxState, "s": scalar, "lo": minScalar, "c": maxCurve keys, "c0": minCurve keys}
           keys = [[time, value, inSlope, outSlope], ...]; states: 0 const, 1 curve,
           2 random between two curves, 3 random between two constants.
  gradient {"m": minMaxState, "c": maxColor, "c0": minColor, "g": maxGradient, "g0": minGradient}
           gradients = {"c": [[t, r, g, b], ...], "a": [[t, a], ...], "fixed": bool}

Modules not simulated (noise, trails, collision, sub-emitters, lights...) are listed under
"unsupported" so stand-ins are visible. Proprietary data: local use only.
"""
import gc
import glob
import json
import os
import sys

import UnityPy

GAME, OUT, *PREFABS = sys.argv[1:]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
# Some bundles ship only in the Half folder (asset_library_assets_all: map editor props, Adora's
# YellowSolarSpark); the game loads those from there.
AA_HALF = os.path.join(os.path.dirname(AA), "Half")


def bpath(b):
    """The installed file of bundle `b`: Full, else Half."""
    f = os.path.join(AA, b)
    return f if os.path.exists(f) or not os.path.exists(os.path.join(AA_HALF, b)) else os.path.join(AA_HALF, b)
ROOT = os.path.dirname(os.path.abspath(__file__))
INDEX = json.load(open(os.path.join(ROOT, "..", "..", "local_extracted", "all_container_paths.json")))
_CAB_PATH = os.path.join(ROOT, "..", "..", "local_extracted", "cab_index.json")
CABS = json.load(open(_CAB_PATH)) if os.path.exists(_CAB_PATH) else {}

SIMULATED = {"InitialModule", "CustomDataModule", "EmissionModule", "ShapeModule", "SizeModule", "ColorModule", "RotationModule",
             "VelocityModule", "ForceModule", "ClampVelocityModule", "UVModule", "SubModule"}


def keys(c):
    return [[k["time"], k["value"], k["inSlope"], k["outSlope"]] for k in c.get("m_Curve", [])]


def curve(v):
    return {"m": v["minMaxState"], "s": v["scalar"], "lo": v.get("minScalar", 0.0), "c": keys(v["maxCurve"]),
            "c0": keys(v["minCurve"])}


def grad(g):
    nc, na = g.get("m_NumColorKeys", 2), g.get("m_NumAlphaKeys", 2)
    cols = [[g[f"ctime{i}"] / 65535, g[f"key{i}"]["r"], g[f"key{i}"]["g"], g[f"key{i}"]["b"]] for i in range(nc)]
    alph = [[g[f"atime{i}"] / 65535, g[f"key{i}"]["a"]] for i in range(na)]
    return {"c": cols, "a": alph, "fixed": g.get("m_Mode", 0) == 1}


def color(v):
    rgba = lambda c: [c["r"], c["g"], c["b"], c["a"]]
    return {"m": v["minMaxState"], "c": rgba(v["maxColor"]), "c0": rgba(v["minColor"]),
            "g": grad(v["maxGradient"]), "g0": grad(v["minGradient"])}


def vec(v):
    return [v["x"], v["y"], v["z"]]


def finite(o):
    """JSON-safe copy: infinities become +-3e38 and NaN 0 (strict JSON readers reject them)."""
    if isinstance(o, float):
        return 0.0 if o != o else (3e38 if o == float("inf") else (-3e38 if o == float("-inf") else o))
    if isinstance(o, dict):
        return {k: finite(v) for k, v in o.items()}
    if isinstance(o, (list, tuple)):
        return [finite(v) for v in o]
    return o


def quat(q):
    return [q["x"], q["y"], q["z"], q["w"]]


def system(ps, out_tex, env):
    t = ps.read_typetree()
    s = {"duration": t["lengthInSec"], "looping": bool(t["looping"]), "prewarm": bool(t["prewarm"]),
         "delay": curve(t["startDelay"]), "speed": t.get("simulationSpeed", 1.0),
         "space": t.get("moveWithTransform", 0), "scaling": t.get("scalingMode", 1)}
    im = t["InitialModule"]
    s["init"] = {"lifetime": curve(im["startLifetime"]), "speed": curve(im["startSpeed"]),
                 "color": color(im["startColor"]), "size": curve(im["startSize"]),
                 "size3d": bool(im.get("size3D")), "size_y": curve(im["startSizeY"]), "size_z": curve(im["startSizeZ"]),
                 "rotation": curve(im["startRotation"]), "rotation3d": bool(im.get("rotation3D")),
                 "rotation_x": curve(im["startRotationX"]), "rotation_y": curve(im["startRotationY"]),
                 "flip": im.get("randomizeRotationDirection", 0.0),
                 "gravity": curve(im["gravityModifier"]) if isinstance(im.get("gravityModifier"), dict) else
                 {"m": 0, "s": im.get("gravityModifier", 0.0), "lo": 0, "c": [], "c0": []},
                 "max": im.get("maxNumParticles", 1000)}
    em = t["EmissionModule"]
    if em["enabled"]:
        s["emission"] = {"rate": curve(em["rateOverTime"]), "rate_dist": curve(em["rateOverDistance"]),
                         "bursts": [{"time": b["time"], "count": curve(b["countCurve"]) if "countCurve" in b else
                                     {"m": 3, "s": b["maxCount"], "lo": b["minCount"], "c": [], "c0": []},
                                     "cycles": b.get("cycleCount", 1), "interval": b.get("repeatInterval", 0.01),
                                     "probability": b.get("probability", 1.0)} for b in em.get("m_Bursts", [])]}
    sh = t["ShapeModule"]
    if sh["enabled"]:
        s["shape"] = {"type": sh["type"], "angle": sh.get("angle", 0.0), "length": sh.get("length", 5.0),
                      "radius": sh["radius"]["value"] if isinstance(sh.get("radius"), dict) else sh.get("radius", 1.0),
                      "thickness": sh.get("radiusThickness", 1.0),
                      "arc": sh["arc"]["value"] if isinstance(sh.get("arc"), dict) else 360.0,
                      "box": vec(sh.get("m_Scale", {"x": 1, "y": 1, "z": 1})),
                      "pos": vec(sh.get("m_Position", {"x": 0, "y": 0, "z": 0})),
                      "rot": vec(sh.get("m_Rotation", {"x": 0, "y": 0, "z": 0})),
                      "random_dir": sh.get("randomDirectionAmount", 0.0),
                      "spherize": sh.get("sphericalDirectionAmount", 0.0)}
        if sh["type"] == 17:
            s["shape"]["donut"] = sh["donutRadius"]
    for name, key in (("SizeModule", "size"), ("RotationModule", "rotation")):
        m = t[name]
        if m["enabled"]:
            s[key] = {"x": curve(m.get("x", m.get("curve"))), "y": curve(m["y"]) if "y" in m else None,
                      "z": curve(m["z"] if "z" in m else m["curve"]), "separate": bool(m.get("separateAxes"))}
    if t["ColorModule"]["enabled"]:
        s["color"] = color(t["ColorModule"]["gradient"])
    for name, key in (("VelocityModule", "velocity"), ("ForceModule", "force")):
        m = t[name]
        if m["enabled"]:
            s[key] = {"x": curve(m["x"]), "y": curve(m["y"]), "z": curve(m["z"]), "world": bool(m.get("inWorldSpace")),
                      "orbital": [curve(m[k]) for k in ("orbitalX", "orbitalY", "orbitalZ")] if "orbitalX" in m else None,
                      "radial": curve(m["radial"]) if "radial" in m else None,
                      "speed_mod": curve(m["speedModifier"]) if "speedModifier" in m else None,
                      "offset": [curve(m[k]) for k in ("orbitalOffsetX", "orbitalOffsetY", "orbitalOffsetZ")] if "orbitalOffsetX" in m else None}
    cv = t["ClampVelocityModule"]
    if cv["enabled"]:
        s["limit"] = {"speed": curve(cv["magnitude"]), "dampen": cv.get("dampen", 1.0), "drag": curve(cv["drag"]) if "drag" in cv else None}
    uv = t["UVModule"]
    if uv["enabled"]:
        s["sheet"] = {"x": uv["tilesX"], "y": uv["tilesY"], "frame": curve(uv["frameOverTime"]),
                      "start": curve(uv["startFrame"]), "cycles": uv.get("cycles", 1.0), "row_mode": uv.get("rowMode", 0),
                      "anim": uv.get("animationType", 0), "row": uv.get("rowIndex", 0)}
    # CustomDataModule: per-particle data for the shader (Custom1 / Custom2 vertex streams), as
    # [mode (0 off, 1 vector, 2 colour), [x, y, z, w] curves over the particle's life].
    cd = t.get("CustomDataModule", {})
    if cd.get("enabled"):
        s["custom"] = [[cd.get(f"mode{i}", 0), [curve(cd[f"vector{i}_{k}"]) for k in range(4)]] for i in (0, 1)]
    # SubModule: sub-emitters (the ParticleSystem's path id, resolved to a node by export_root),
    # type 0 Birth / 1 Collision / 2 Death / 3 Trigger / 4 Manual, the inherited properties
    # (1 colour, 2 size, 4 rotation, 8 lifetime, 16 duration) and the emit probability.
    sm = t.get("SubModule", {})
    if sm.get("enabled"):
        s["sub"] = [{"emitter": e["emitter"]["m_PathID"], "type": e.get("type", 0), "props": e.get("properties", 0),
                     "prob": e.get("emitProbability", 1.0)} for e in sm.get("subEmitters", []) if e.get("emitter", {}).get("m_PathID")]
    s["unsupported"] = sorted(k for k, v in t.items() if k.endswith("Module") and isinstance(v, dict)
                              and v.get("enabled") and k not in SIMULATED)
    return s


# Particle materials drawn with the game's own shader translated to GLSL (dxbc_glsl.py, fx.rs):
# shader-graph effects with no plain texture to stand in for them (the Wizard's fire: Fireball,
# Wall of Fire, Dragon's Breath, the Phoenix's fireballs).
TRANSLATED = {"Shader Graphs/Molotov"}
SHADERS = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "local_extracted", "shaders")


def _pptr(af, pp):
    """The object a PPtr names, in `af` or in a loaded external file."""
    if not pp or not pp.get("m_PathID"):
        return None
    if pp["m_FileID"] == 0:
        return af.objects.get(pp["m_PathID"])
    ext = af.externals[pp["m_FileID"] - 1].path.lower().rsplit("/", 1)[-1]
    for f in af.environment.files.values():
        for k, sf in getattr(f, "files", {}).items():
            if k.lower() == ext and hasattr(sf, "objects"):
                o = sf.objects.get(pp["m_PathID"])
                if o is not None:
                    return o
    return None


def uvzw_variant(file):
    """<shader>_uvzw.json: the vertex shader with TEXCOORD0.zw taken from normal.xy (where
    particles.rs puts the custom vertex streams) and its own normal input zeroed."""
    var = file[:-len(".json")] + "_uvzw.json"
    src, dst = os.path.join(SHADERS, file), os.path.join(SHADERS, var)
    if os.path.exists(src) and not os.path.exists(dst):
        r = json.load(open(src))
        vs = r["vs"]
        vs = vs.replace("vec4(texcoord.x, 1.0 - texcoord.y, 0.0, 0.0)", "vec4(texcoord.x, 1.0 - texcoord.y, normal.x, normal.y)")
        vs = vs.replace("vec4(normal.x, normal.y, -normal.z, 0.0)", "vec4(0.0, 0.0, 1.0, 0.0)")
        r["vs"] = vs
        json.dump(r, open(dst, "w"), indent=1)
    return var


def translated(mo, m, name, out_tex):
    """{shader_file, props} for a material whose shader is in TRANSLATED (its GLSL written to
    local_extracted/shaders, its textures to out_tex), else {}."""
    if name not in TRANSLATED:
        return {}
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    import dxbc_glsl
    file = name.replace("/", "_").replace(" ", "_") + ".json"
    path = os.path.join(SHADERS, file)
    if not os.path.exists(path):
        so = _pptr(mo.assets_file, m["m_Shader"])
        if so is None:
            return {}
        try:
            r = dxbc_glsl.translate(so)
        except Exception as e:
            print("shader translation failed", name, type(e).__name__, e, flush=True)
            return {}
        os.makedirs(SHADERS, exist_ok=True)
        json.dump(r, open(path, "w"), indent=1)
    props = m["m_SavedProperties"]
    texs = {}
    for k, v in props.get("m_TexEnvs", []):
        to = _pptr(mo.assets_file, v["m_Texture"])
        if to is None:
            continue
        tx = to.read()
        fn = f"{tx.m_Name}.png"
        fp = os.path.join(out_tex, fn)
        if not os.path.exists(fp):
            os.makedirs(out_tex, exist_ok=True)
            tx.image.save(fp)
        ts = to.read_typetree().get("m_TextureSettings", {})
        texs[k] = {"file": "tex/" + fn, "wrap": [ts.get("m_WrapU", 0), ts.get("m_WrapV", 0)],
                   "scale": [v["m_Scale"]["x"], v["m_Scale"]["y"]], "offset": [v["m_Offset"]["x"], v["m_Offset"]["y"]]}
    return {"shader_file": file, "shader_file_uvzw": uvzw_variant(file), "queue": m.get("m_CustomRenderQueue", -1),
            "props": {"floats": dict(props.get("m_Floats", [])),
                      "colors": {k: [c["r"], c["g"], c["b"], c["a"]] for k, c in props.get("m_Colors", [])},
                      "textures": texs}}


# ParticleSystemVertexStream values that land in TEXCOORD0.zw after UV, by component.
STREAM_COMPONENTS = {21: ["age"], 31: ["c1x"], 32: ["c1x", "c1y"], 33: ["c1x", "c1y", "c1z"], 34: ["c1x", "c1y", "c1z", "c1w"],
                     35: ["c2x"], 36: ["c2x", "c2y"], 37: ["c2x", "c2y", "c2z"], 38: ["c2x", "c2y", "c2z", "c2w"]}
# Streams that go to their own semantic (Position, Normal, Tangent, Color), not TEXCOORD.
OWN_SEMANTIC = {0, 1, 2, 3}


def uv_extra(streams):
    """The two components after UV in TEXCOORD0 ("c1x", "age"...; "0" for an unsupported one)."""
    if 4 not in streams:
        return []
    out = []
    for st in streams[streams.index(4) + 1:]:
        if st in OWN_SEMANTIC:
            continue
        out += STREAM_COMPONENTS.get(st, ["0"])
        if len(out) >= 2:
            break
    return out[:2]


def _flat(a, n):
    if not a:
        return []
    if isinstance(a[0], (list, tuple)):
        return [list(x)[:n] for x in a]
    return [list(x) for x in zip(*[iter(a)] * n)]



def renderer(r, out_tex):
    t = r.read_typetree()
    out = {"mode": t.get("m_RenderMode", 0), "length": t.get("m_LengthScale", 2.0), "vel_scale": t.get("m_VelocityScale", 0.0),
           "max_size": t.get("m_MaxParticleSize", 0.5), "min_size": t.get("m_MinParticleSize", 0.0),
           "pivot": vec(t.get("m_Pivot", {"x": 0, "y": 0, "z": 0})), "fudge": t.get("m_SortingFudge", 0.0),
           "align": t.get("m_RenderAlignment", 0), "enabled": bool(t.get("m_Enabled", 1)),
           # Renderer.sortingOrder: transparent renderers draw in this order (then by distance).
           "order": t.get("m_SortingOrder", 0)}
    # Custom vertex streams (ParticleSystemVertexStream): what follows UV in TEXCOORD0.zw.
    if t.get("m_UseCustomVertexStreams"):
        out["uv_extra"] = uv_extra(t.get("m_VertexStreams", []))
    # Mesh particles (mode 4): the mesh's name (the effects here use Unity's Quad).
    meshes = [m for m in t.get("m_Meshes", [t.get("m_Mesh")]) if m and m.get("m_PathID")]
    if meshes:
        try:
            mo = r.assets_file.objects.get(meshes[0]["m_PathID"]) if meshes[0]["m_FileID"] == 0 else None
            if mo is None:
                # In another bundle (load_bundle loads it): look it up as materials are below.
                ext = r.assets_file.externals[meshes[0]["m_FileID"] - 1].path.lower().rsplit("/", 1)[-1]
                for f in r.assets_file.environment.files.values():
                    for k, sf in getattr(f, "files", {}).items():
                        if k.lower() == ext and hasattr(sf, "objects"):
                            mo = sf.objects.get(meshes[0]["m_PathID"]) or mo
            if mo is not None:
                mesh = mo.read()
                out["mesh"] = mesh.m_Name
                # Its geometry (pmesh_<name>.json beside effect.json: x y z u v, indices): each
                # particle draws this mesh, not a quad (Wall of Fire's flames: a curved sheet).
                if out["mode"] == 4:
                    from UnityPy.helpers.MeshHelper import MeshHandler
                    h = MeshHandler(mesh)
                    h.process()
                    v = _flat(h.m_Vertices, 3)
                    uv = _flat(h.m_UV0, 2) or [[0.0, 0.0]] * len(v)
                    subs = h.get_triangles()
                    tris = []
                    for tri in (subs[0] if subs else []):
                        tris.extend(tri if isinstance(tri, (list, tuple)) else [tri])
                    fn = f"pmesh_{mesh.m_Name}.json"
                    out_dir = os.path.dirname(out_tex)
                    os.makedirs(out_dir, exist_ok=True)
                    json.dump({"v": [a + b for a, b in zip(v, uv)], "i": [int(i) for i in tris]}, open(os.path.join(out_dir, fn), "w"))
                    out["mesh_file"] = fn
            else:
                # Unity's built-in meshes ("unity default resources", not in any bundle).
                ext = r.assets_file.externals[meshes[0]["m_FileID"] - 1].path.replace("\\", "/").rsplit("/", 1)[-1].lower()
                builtin = {10202: "Cube", 10206: "Cylinder", 10207: "Sphere", 10208: "Capsule", 10209: "Plane", 10210: "Quad"}
                out["mesh"] = builtin.get(meshes[0]["m_PathID"], "external") if ext == "unity default resources" else "external"
        except Exception as e:
            print("  particle mesh not exported:", type(e).__name__, e, flush=True)
            out["mesh"] = "?"
    mats = [m for m in t.get("m_Materials", []) if m["m_PathID"]]
    if not mats:
        return out
    mo = r.assets_file.objects.get(mats[0]["m_PathID"]) if mats[0]["m_FileID"] == 0 else None
    if mo is None:
        ext = r.assets_file.externals[mats[0]["m_FileID"] - 1].path.lower().rsplit("/", 1)[-1]
        for name, f in r.assets_file.environment.files.items():
            # (A bundle's serialized files, or a loose one: unity default resources.)
            subs = getattr(f, "files", None) if not hasattr(f, "objects") else {os.path.basename(str(name)): f}
            for k, sf in (subs or {}).items():
                if str(k).lower() == ext and hasattr(sf, "objects"):
                    mo = sf.objects.get(mats[0]["m_PathID"])
    if mo is None:
        return out
    m = mo.read_typetree()
    props = m["m_SavedProperties"]
    floats = dict(props.get("m_Floats", []))
    cols = dict(props.get("m_Colors", []))
    out["material"] = m["m_Name"]
    try:
        sh = m["m_Shader"]
        so = mo.assets_file.objects.get(sh["m_PathID"]) if sh["m_FileID"] == 0 else None
        if so is None:
            so = _pptr(mo.assets_file, sh)
        out["shader"] = so.read().m_ParsedForm.m_Name if so else None
    except Exception:
        out["shader"] = None
    out.update(translated(mo, m, out.get("shader"), out_tex))
    # Blend: the material's _SrcBlend/_DstBlend (UnityEngine.Rendering.BlendMode: 1 One, 5 SrcAlpha,
    # 10 OneMinusSrcAlpha), else the legacy shader name.
    src, dst = floats.get("_SrcBlend"), floats.get("_DstBlend")
    if src is not None and dst is not None:
        out["blend"] = "add" if dst == 1 else ("premul" if src == 1 and dst == 10 else "alpha")
    else:
        n = (out.get("shader") or "").lower()
        # (Legacy "Particles/Alpha Blended Premultiply": Blend One OneMinusSrcAlpha.)
        out["blend"] = "add" if "add" in n else ("premul" if "premultipl" in n else "alpha")
    # URP Particles/Unlit: _BaseColor (materials keep a stale legacy _TintColor). Legacy particle
    # shaders: 2 x _TintColor.
    urp = (out.get("shader") or "").startswith("Universal Render Pipeline") or "_BaseColor" in cols
    tint = cols.get("_BaseColor") if urp else (cols.get("_TintColor") or cols.get("_Color"))
    out["tint_x2"] = not urp and "_TintColor" in cols
    # URP Particles/Unlit colour mode (0 multiply, 1 additive, ...) with the particle colour.
    if urp:
        out["color_mode"] = int(floats.get("_ColorMode", 0))
    if tint:
        out["tint"] = [tint["r"], tint["g"], tint["b"], tint["a"]]
    for k, v in props.get("m_TexEnvs", []):
        if k in ("_MainTex", "_BaseMap", "_Texture") and v["m_Texture"]["m_PathID"]:
            tex_pp = v["m_Texture"]
            to = mo.assets_file.objects.get(tex_pp["m_PathID"]) if tex_pp["m_FileID"] == 0 else None
            if to is None:
                ext = mo.assets_file.externals[tex_pp["m_FileID"] - 1].path.lower().rsplit("/", 1)[-1]
                for f in mo.assets_file.environment.files.values():
                    for kk, sf in getattr(f, "files", {}).items():
                        if kk.lower() == ext and hasattr(sf, "objects"):
                            to = sf.objects.get(tex_pp["m_PathID"])
            if to is not None:
                tx = to.read()
                name = f"{tx.m_Name}.png"
                p = os.path.join(out_tex, name)
                if not os.path.exists(p):
                    os.makedirs(out_tex, exist_ok=True)
                    tx.image.save(p)
                out["texture"] = "tex/" + name
                out["uv_scale"] = [v["m_Scale"]["x"], v["m_Scale"]["y"]]
                out["uv_offset"] = [v["m_Offset"]["x"], v["m_Offset"]["y"]]
            break
    return out


def qmul(a, b):
    ax, ay, az, aw = a
    bx, by, bz, bw = b
    return [aw * bx + ax * bw + ay * bz - az * by, aw * by - ax * bz + ay * bw + az * bx,
            aw * bz + ax * by - ay * bx + az * bw, aw * bw - ax * bx - ay * by - az * bz]


def qrot(q, v):
    x, y, z, w = q
    tx, ty, tz = 2 * (y * v[2] - z * v[1]), 2 * (z * v[0] - x * v[2]), 2 * (x * v[1] - y * v[0])
    return [v[0] + w * tx + (y * tz - z * ty), v[1] + w * ty + (z * tx - x * tz), v[2] + w * tz + (x * ty - y * tx)]


# Unity's built-in meshes (Cube, Plane, Quad... in "unity default resources"): the install's own,
# else RRBTD6_DEFAULT_RESOURCES, else a Steam install's (this Epic install ships none; its
# bundles are Unity 6000.3.23, the Steam file 6000.3.19: the same built-in meshes).
DEFAULT_RESOURCES = next((f for f in (os.path.join(GAME, "BloonsTD6_Data", "Resources", "unity default resources"),
                                      os.environ.get("RRBTD6_DEFAULT_RESOURCES", ""),
                                      os.path.join(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)"), "Steam", "steamapps",
                                                   "common", "BloonsTD6", "BloonsTD6_Data", "Resources", "unity default resources"))
                          if f and os.path.exists(f)), None)


_DEFAULT_ENV = {}


def builtin_object(path_id):
    """An object of "unity default resources" (a built-in mesh), from its own env: loaded into a
    bundle's env it replaces that env's container table."""
    if not DEFAULT_RESOURCES:
        return None
    if "f" not in _DEFAULT_ENV:
        _DEFAULT_ENV["f"] = next(iter(UnityPy.load(DEFAULT_RESOURCES).files.values()))
    o = _DEFAULT_ENV["f"].objects.get(path_id)
    return o.read() if o is not None else None


def load_bundle(path):
    """UnityPy env of bundle `path` with the monoscripts and the bundles its externals live in
    (materials, textures and shaders shared across bundles: unitybuiltinassets, ...), else their
    PPtrs read as missing (an effect with no material, texture or shader). cab_index.py maps the
    CAB names to bundle files."""
    # Loaded once, its dependencies added to the same env: a separate probe load kept a second
    # copy of the bundle alive (asset_references_assets_all, ~900 MB, then ran out of memory).
    name = os.path.basename(path)
    mono = glob.glob(os.path.join(AA, "*_monoscripts_*.bundle"))
    env = UnityPy.load(path, *mono)
    deps = set()
    for f in list(env.files.values()):
        for af in getattr(f, "files", {}).values():
            for ext in getattr(af, "externals", []) or []:
                cab = os.path.basename(ext.path).lower()
                if cab in CABS and CABS[cab] != name and bpath(CABS[cab]) not in mono:
                    deps.add(CABS[cab])
    for d in sorted(deps):
        env.load_file(bpath(d))
    return env


def find_prefab(env, prefab):
    """The container entry `prefab`, else the one with its file name (a game update moved the
    prefab within its bundle, as Adora's YellowSolarSpark), else None."""
    if prefab in env.container:
        return env.container[prefab]
    base = "/" + os.path.basename(prefab).lower()
    hit = next((v for k, v in env.container.items() if k.lower().endswith(base)), None)
    if hit is not None:
        return hit
    # Bundles without a container table (asset_library_assets_all): a root GameObject of that name.
    stem = os.path.splitext(os.path.basename(prefab))[0]
    for o in env.objects:
        if o.type.name == "GameObject" and o.assets_file is not None:
            go = o.read()
            if go.m_Name == stem:
                tr = next((c.read() for c in (x if hasattr(x, "read") else x.component for x in go.m_Components)
                           if c.type.name in ("Transform", "RectTransform")), None)
                if tr is not None and not (tr.m_Father and tr.m_Father.path_id):
                    return o
    return None


def export_scene(key):
    """Ambient effects of map <key>: each particle object without a particle-system ancestor."""
    bundle = glob.glob(os.path.join(AA, f"map_{key}_scenes_all_*.bundle"))[0]
    env = load_bundle(bundle)
    # Only the scene's own objects: the dependency bundles load_bundle adds hold other prefabs
    # (tower effects, ...) that aren't placed in the map.
    probe = UnityPy.load(bundle)
    own = {n.lower() for f in probe.files.values() for n in (getattr(f, "files", {}) or {})}
    del probe
    gos = [o for o in env.objects if o.type.name == "GameObject" and o.assets_file.name.lower() in own]
    placed = []

    def comps(go):
        return [(c.read() if hasattr(c, "read") else c.component.read()) for c in go.m_Components]

    def transform(go):
        return next(c for c in comps(go) if c.object_reader.type.name in ("Transform", "RectTransform"))

    def world(tr):
        """World position / rotation / scale and whether every ancestor is active."""
        p, r, s, active = [0, 0, 0], [0, 0, 0, 1], [1, 1, 1], True
        chain = []
        t = tr
        while t is not None:
            chain.append(t)
            f = t.m_Father
            t = f.read() if f and f.path_id else None
        for t in reversed(chain):
            lp = [t.m_LocalPosition.x, t.m_LocalPosition.y, t.m_LocalPosition.z]
            lr = [t.m_LocalRotation.x, t.m_LocalRotation.y, t.m_LocalRotation.z, t.m_LocalRotation.w]
            ls = [t.m_LocalScale.x, t.m_LocalScale.y, t.m_LocalScale.z]
            sp = qrot(r, [lp[i] * s[i] for i in range(3)])
            p = [p[i] + sp[i] for i in range(3)]
            r = qmul(r, lr)
            s = [s[i] * ls[i] for i in range(3)]
            active = active and bool(t.m_GameObject.read().m_IsActive)
        return p, r, s, active

    def has_ps(go):
        return any(c.object_reader.type.name == "ParticleSystem" for c in comps(go))

    n = 0
    for o in gos:
        go = o.read()
        try:
            if not has_ps(go):
                continue
            tr = transform(go)
            f = tr.m_Father
            anc, top = f.read() if f and f.path_id else None, True
            while anc is not None:
                if has_ps(anc.m_GameObject.read()):
                    top = False
                    break
                f = anc.m_Father
                anc = f.read() if f and f.path_id else None
            if not top:
                continue
            p, r, s, active = world(tr)
            if not active:
                continue
        except Exception as e:
            print("skip", getattr(go, "m_Name", "?"), type(e).__name__, e)
            continue
        name = f"map_{key}_{n}"
        n += 1
        export_root(env, go, name, {"pos": [0, 0, 0], "rot": r, "scale": s}, parts=False)
        placed.append({"effect": name, "object": go.m_Name, "pos": p})
    json.dump(placed, open(os.path.join(OUT, f"map_{key}.json"), "w"), indent=1)
    print(key, "->", len(placed), "ambient effects")


ENVS = {}


def export_tiers(tower):
    """Particle systems inside a tower's tier prefabs (Ring of Fire's flames...): one effect per
    prefab with any, tier_<Tower>_<stem>, and <OUT>/tier_<Tower>.json {model dir: effect}."""
    plan = json.load(open(os.path.join(ROOT, "..", "..", "local_extracted", "display_plans", f"{tower}.json")))
    out, seen, jobs = {}, set(), []
    for t in plan["towers"].values():
        for p in [t] + t.get("attack_displays", []):
            stem = os.path.splitext(os.path.basename(p["prefab"]))[0].strip().replace(" ", "_")
            if stem in seen or p["prefab"] not in INDEX:
                continue
            seen.add(stem)
            jobs.append((INDEX[p["prefab"]], stem, p))
    # The Paragon's displays (game.json tower_tiers.<Tower>.paragon: base, tops by degree, bases),
    # found by prefab name (Apex Plasma Master's SunBeamCore orbs on its weapon joint).
    gp = os.path.join(ROOT, "..", "..", "local_extracted", "game", "game.json")
    par = (json.load(open(gp, encoding="utf-8")).get("tower_tiers", {}).get(tower) or {}).get("paragon") if os.path.exists(gp) else None
    for name in ([par.get("display"), par.get("top")] + list(par.get("tops") or []) + list(par.get("bases") or [])) if par else []:
        hits = [k for k in INDEX if name and k.endswith("/" + name + ".prefab")]
        stem = (name or "").replace(" ", "_")
        if len(hits) == 1 and stem not in seen:
            seen.add(stem)
            jobs.append((INDEX[hits[0]], stem, {"prefab": hits[0]}))
    # Bundle by bundle, one loaded at a time: each env carries its dependency bundles, and
    # keeping several (each with its own copy of the ~900 MB asset_references) ran out of memory.
    for b, stem, p in sorted(jobs, key=lambda j: j[0]):
        if b not in ENVS:
            # Drop every reference to the previous env first, or both are alive while this loads.
            env = root = obj = None
            ENVS.clear()
            gc.collect()
            ENVS[b] = load_bundle(bpath(b))
        env = ENVS[b]
        root = env.container[p["prefab"]].read()
        name = f"tier_{tower}_{stem}"
        n = export_root(env, root, name, None, p["prefab"], parts=False)
        if n:
            out[stem] = name
        else:
            import shutil
            shutil.rmtree(os.path.join(OUT, name), ignore_errors=True)
    json.dump(out, open(os.path.join(OUT, f"tier_{tower}.json"), "w"), indent=1)
    print(tower, "->", len(out), "tier prefabs with particle effects")


def export_projectiles(after=None):
    """Particle systems inside projectile prefabs (Wizard bolts, fireballs, trails): one effect
    per prefab with any, proj_<name>, and <OUT>/proj_index.json {projectile name: effect}.
    Bundle by bundle so only one big bundle is loaded at a time. `after` (projectiles>PREFAB):
    resume past that prefab in a fresh process (a long run can run out of memory late), keeping
    the earlier index entries."""
    plan = {}
    for f in sorted(glob.glob(os.path.join(ROOT, "..", "..", "local_extracted", "display_plans", "*.json"))):
        for g, p in json.load(open(f))["projectiles"].items():
            if p.get("prefab") and p.get("name"):
                plan[p["name"]] = p["prefab"]
    out = {}
    order = sorted(plan.items(), key=lambda kv: INDEX.get(kv[1], ""))
    if after:
        k = [p for _, p in order].index(after) + 1
        # The earlier ones as the interrupted run left them on disk.
        out = {n: f"proj_{n}" for n, _ in order[:k] if os.path.exists(os.path.join(OUT, f"proj_{n}", "effect.json"))}
        order = order[k:]
    for name, prefab in order:
        if prefab not in INDEX:
            continue
        b = INDEX[prefab]
        if b not in ENVS:
            # Drop every reference to the previous env first, or both are alive while this loads.
            env = root = obj = None
            ENVS.clear()
            gc.collect()
            ENVS[b] = load_bundle(bpath(b))
        env = ENVS[b]
        obj = find_prefab(env, prefab)
        if obj is None:
            print(prefab, "-> not in", b, flush=True)
            continue
        root = obj.read()
        eff = f"proj_{name}"
        if export_root(env, root, eff, None, prefab, parts=False):
            out[name] = eff
        else:
            import shutil
            shutil.rmtree(os.path.join(OUT, eff), ignore_errors=True)
    json.dump(out, open(os.path.join(OUT, "proj_index.json"), "w"), indent=1)
    print(len(out), "projectiles with particle effects")


def export(prefab):
    if prefab == "projectiles" or prefab.startswith("projectiles>"):
        export_projectiles(prefab.partition(">")[2] or None)
        return
    if prefab.startswith("scene:"):
        export_scene(prefab[6:])
        return
    if prefab.startswith("tiers:"):
        export_tiers(prefab[6:])
        return
    bundle = INDEX[prefab]
    # One bundle loaded at a time, kept for the next prefab from it (the prefabs are sorted by
    # bundle below; asset_references_assets_all is ~900 MB and takes seconds to load).
    if bundle not in _ENVS:
        _ENVS.clear()
        _ENVS[bundle] = load_bundle(bpath(bundle))
    env = _ENVS[bundle]
    obj = find_prefab(env, prefab)
    if obj is None:
        print(prefab, "-> not in", bundle, flush=True)
        return
    root = obj.read()
    name = os.path.splitext(os.path.basename(prefab))[0]
    export_root(env, root, name, None, prefab)


# NinjaKiwi's CustomAnimationComponent subclasses (decompiled: Update @0x521830 advances elapsed
# by deltaTime / duration x speedMultiplier and evaluates the curve at fmodf(elapsed, 1) when
# looping, else at elapsed until it passes 1, then at 1): CustomColorAnimator (@0x521a60: the
# target's SpriteRenderer colour = lerp(color1, color2, clamp01(curve))), CustomScaleAnimator
# (@0x522a60: localScale = RootScale (Vector3.one, .ctor @0x522b40) x curve),
# CustomRotationAnimator (@0x522600: localRotation = AngleAxis(curve x rotationsPerDuration x 360
# + angleOffset, right / up / forward for axis 0 / 1 / 2)).
CUSTOM_ANIMS = {"CustomColorAnimator": "color", "CustomScaleAnimator": "scale", "CustomRotationAnimator": "rotation"}


def anim_curve(c):
    return [[k["time"], k["value"], k["inSlope"], k["outSlope"]] for k in c.get("m_Curve", [])]


_ATLAS = {}


def atlas_sprite_image(sp, st):
    """A late-bound sprite (its texture in a SpriteAtlas, m_RD.texture empty): the atlas bundle's
    own Sprite with the same m_RenderDataKey (sprite_atlases_assets_all; its rect can be at
    another resolution than this copy's), padded back to its full m_Rect as export_sprites.py
    does. The world size stays this sprite's (rect / pixelsToUnits)."""
    from PIL import Image
    if not _ATLAS:
        senv = UnityPy.load(glob.glob(os.path.join(AA, "sprite_atlases_assets_all_*.bundle"))[0])
        _ATLAS["env"] = senv
        for o in senv.objects:
            if o.type.name == "Sprite":
                a = o.read()
                _ATLAS[(str(a.m_RenderDataKey[0]), a.m_RenderDataKey[1])] = a
    a = _ATLAS[(str(sp.m_RenderDataKey[0]), sp.m_RenderDataKey[1])]
    img = a.image
    w, h = round(a.m_Rect.width), round(a.m_Rect.height)
    if img.size == (w, h):
        return img
    off = a.m_RD.textureRectOffset
    full = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    full.paste(img, (round(off.x), round(h - off.y - img.size[1])))
    return full


def sprite_node(sr, out_tex):
    """A SpriteRenderer: its sprite's image, world size (rect / pixelsToUnits for a simple sprite,
    m_Size for sliced / tiled), pivot, colour, sorting order and material blend."""
    t = sr.object_reader.read_typetree()
    if not sr.m_Sprite or not sr.m_Sprite.path_id:
        return None
    try:
        sp = sr.m_Sprite.read()
    except Exception:
        return None
    st = sp.object_reader.read_typetree()
    fn = f"sprite_{sp.m_Name}.png"
    fp = os.path.join(out_tex, fn)
    if not os.path.exists(fp):
        os.makedirs(out_tex, exist_ok=True)
        try:
            img = sp.image
        except ValueError:
            img = atlas_sprite_image(sp, st)
        img.save(fp)
    r, ppu = st["m_Rect"], st["m_PixelsToUnits"] or 100.0
    if t.get("m_DrawMode", 0) == 0:
        size = [r["width"] / ppu, r["height"] / ppu]
    else:
        size = [t["m_Size"]["x"], t["m_Size"]["y"]]
    mat = renderer(sr.object_reader, out_tex)
    c = t.get("m_Color", {"r": 1, "g": 1, "b": 1, "a": 1})
    return {"texture": "tex/" + fn, "size": size, "pivot": [st["m_Pivot"]["x"], st["m_Pivot"]["y"]],
            "color": [c["r"], c["g"], c["b"], c["a"]], "flip": [bool(t.get("m_FlipX")), bool(t.get("m_FlipY"))],
            "order": t.get("m_SortingOrder", 0), "enabled": bool(t.get("m_Enabled", 1)),
            "material": mat.get("material"), "shader": mat.get("shader"), "blend": mat.get("blend", "alpha")}



def mesh_node(mf, mr, out_dir):
    """A MeshFilter + MeshRenderer: the mesh (local vertices x y z u v, triangle indices) in
    mesh_<name>.json beside effect.json, and its first material (texture, blend, tint, shader)."""
    from UnityPy.helpers.MeshHelper import MeshHandler
    if not mf.m_Mesh or not mf.m_Mesh.path_id:
        return None
    try:
        mesh = mf.m_Mesh.read()
    except FileNotFoundError:
        mesh = builtin_object(mf.m_Mesh.path_id)
        if mesh is None:
            raise
    h = MeshHandler(mesh)
    h.process()
    v = _flat(h.m_Vertices, 3)
    uv = _flat(h.m_UV0, 2) or [[0.0, 0.0]] * len(v)
    subs = h.get_triangles()
    tris = []
    for tri in (subs[0] if subs else []):
        tris.extend(tri if isinstance(tri, (list, tuple)) else [tri])
    fn = f"mesh_{mesh.m_Name}.json"
    json.dump({"v": [a + b for a, b in zip(v, uv)], "i": [int(i) for i in tris]}, open(os.path.join(out_dir, fn), "w"))
    mat = renderer(mr.object_reader, os.path.join(out_dir, "tex"))
    t = mr.object_reader.read_typetree()
    return {"file": fn, "enabled": bool(t.get("m_Enabled", 1)), "order": t.get("m_SortingOrder", 0),
            "material": mat.get("material"), "shader": mat.get("shader"),
            "texture": mat.get("texture"), "tint": mat.get("tint"), "blend": mat.get("blend", "alpha")}


def text_node(t, tr):
    """A world-space TextMeshPro: its text (a placeholder the game may replace), font size (world
    units: fontSize / 10), face colour, four-corner vertex gradient (top left, top right, bottom
    left, bottom right) when enabled, alignment (TMP flags) and rect size."""
    col = lambda c: [c["r"], c["g"], c["b"], c["a"]]
    g = t.get("m_fontColorGradient") or {}
    rt = tr.object_reader.read_typetree() if tr.object_reader.type.name == "RectTransform" else {}
    size = rt.get("m_SizeDelta", {"x": 0.0, "y": 0.0})
    return {"text": t.get("m_text", ""), "size": t.get("m_fontSize", 36.0), "color": col(t["m_fontColor"]),
            "gradient": [col(g[k]) for k in ("topLeft", "topRight", "bottomLeft", "bottomRight")] if t.get("m_enableVertexGradient") and g else None,
            "h_align": t.get("m_HorizontalAlignment", 1), "v_align": t.get("m_VerticalAlignment", 256),
            "rect": [size["x"], size["y"]], "pivot": [rt.get("m_Pivot", {}).get("x", 0.5), rt.get("m_Pivot", {}).get("y", 0.5)],
            "font": "Btd6FontTitleSDF"}


def anim_node(t):
    """A CustomAnimationComponent's serialized fields (target: a GameObject path id, resolved to
    a node index by export_root)."""
    out = {"target": (t.get("target") or {}).get("m_PathID", 0), "duration": t.get("duration", 1.0),
           "curve": anim_curve(t.get("curve", {})), "playing": bool(t.get("isPlaying", 1)), "loop": bool(t.get("loop")),
           "disable_on_end": bool(t.get("disableOnEnd")), "stop_on_end": bool(t.get("stopPlayingOnEnd")),
           "speed": t.get("speedMultiplier", 1.0), "enabled": bool(t.get("m_Enabled", 1))}
    if "color1" in t:
        out["c1"] = [t["color1"][k] for k in "rgba"]
        out["c2"] = [t["color2"][k] for k in "rgba"]
    if "rotationsPerDuration" in t:
        rng = t.get("randomSpeedMultiplierRange") or {}
        out.update({"rotations": t["rotationsPerDuration"], "axis": t.get("axis", 0), "local": bool(t.get("useLocalCoordinateSustem", 1)),
                    "random_angle": bool(t.get("randomizeAngle")), "random_speed": bool(t.get("randomizeSpeed")),
                    "speed_range": [rng.get("min", 1.0), rng.get("max", 1.0)]})
    return out


def animator_clip(an):
    """The Animator's default state's clip sampled every 1/60 s: {len, loop, speed, tracks [{path
    hash, attr (1 position, 2 rotation, 3 scale, 4 euler), samples}]}; export_root resolves the
    paths to nodes."""
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from mecanim_clip import Clip
    if not an.m_Controller or not an.m_Controller.path_id:
        return None
    ctl = an.m_Controller.read()
    # An AnimatorOverrideController (Mermonkey's tentacles) has no states of its own: its base
    # controller's, with clips swapped.
    overrides = {}
    if ctl.object_reader.type.name == "AnimatorOverrideController":
        overrides = {o.m_OriginalClip.path_id: o.m_OverrideClip for o in ctl.m_Clips if o.m_OverrideClip.path_id}
        ctl = ctl.m_Controller.read()
    ct = ctl.object_reader.read_typetree()
    machines = ct["m_Controller"]["m_StateMachineArray"]
    # A controller with no states plays nothing (Aurora).
    if not machines or not machines[0]["data"]["m_StateConstantArray"]:
        return None
    sm = machines[0]["data"]
    st = sm["m_StateConstantArray"][sm.get("m_DefaultState", 0)]["data"]
    clips = ctl.m_AnimationClips
    ci = 0
    if st.get("m_BlendTreeConstantArray"):
        ci = st["m_BlendTreeConstantArray"][0]["data"]["m_NodeArray"][0]["data"]["m_ClipID"]
    if ci >= len(clips):
        return None
    clip = Clip(overrides.get(clips[ci].path_id, clips[ci]).read().object_reader.read_typetree())
    n = max(1, int(round((clip.stop - clip.start) * 60)))
    tracks = {}
    for f in range(n + 1):
        for ph, attrs in clip.pose(clip.start + f / 60).items():
            for a, val in attrs.items():
                tracks.setdefault((ph, a), []).append([float(x) for x in val])
    return {"len": clip.stop - clip.start, "loop": clip.loop, "speed": st.get("m_Speed", 1.0),
            "tracks": [{"path": ph, "attr": a, "samples": smp} for (ph, a), smp in tracks.items()]}


def export_root(env, root, name, root_tr=None, prefab=None, parts=True):
    """Writes <OUT>/<name>/effect.json for the particle systems under GameObject `root`;
    `root_tr` overrides the root node's transform (scene objects: their world rotation / scale).
    `parts`: also its sprites and meshes, for a standalone effect prefab; a projectile's, tower
    tier's or map's own models are drawn by their renderers (here they would draw twice), so
    those keep only their particle systems and animators."""
    out_dir = os.path.join(OUT, name)
    systems = []

    def comps(go):
        return [(c.read() if hasattr(c, "read") else c.component.read()) for c in go.m_Components]

    def walk(go, parent_path):
        cs = comps(go)
        tr = next(c for c in cs if c.object_reader.type.name in ("Transform", "RectTransform"))
        node = {"name": go.m_Name, "active": bool(go.m_IsActive), "pos": vec(tr.m_LocalPosition.__dict__) if hasattr(tr.m_LocalPosition, "__dict__") else
                [tr.m_LocalPosition.x, tr.m_LocalPosition.y, tr.m_LocalPosition.z],
                "rot": [tr.m_LocalRotation.x, tr.m_LocalRotation.y, tr.m_LocalRotation.z, tr.m_LocalRotation.w],
                "scale": [tr.m_LocalScale.x, tr.m_LocalScale.y, tr.m_LocalScale.z], "parent": parent_path}
        if tr.object_reader.type.name == "RectTransform" and tr.m_Father and tr.m_Father.path_id:
            # Under a plain Transform (no parent rect) with point anchors, Unity places a
            # RectTransform at its anchoredPosition (the serialized localPosition can be stale).
            rt = tr.object_reader.read_typetree()
            if tr.m_Father.read().object_reader.type.name == "Transform" and rt["m_AnchorMin"] == rt["m_AnchorMax"]:
                node["pos"][0], node["pos"][1] = rt["m_AnchoredPosition"]["x"], rt["m_AnchoredPosition"]["y"]
        ps = next((c for c in cs if c.object_reader.type.name == "ParticleSystem"), None)
        pr = next((c for c in cs if c.object_reader.type.name == "ParticleSystemRenderer"), None)
        if parent_path is None and root_tr is not None:
            node.update(root_tr)
        idx = len(systems)
        systems.append(node)
        by_go[go.object_reader.path_id] = idx
        if ps is not None:
            by_ps[ps.object_reader.path_id] = idx
            node["system"] = system(ps.object_reader, os.path.join(out_dir, "tex"), env)
            node["renderer"] = renderer(pr.object_reader, os.path.join(out_dir, "tex")) if pr else None
        # Sprites, meshes and their animation (NK animators, an Animator's clip): drawn by
        # particles.rs beside the particle systems.
        sr = next((c for c in cs if c.object_reader.type.name == "SpriteRenderer"), None)
        mf = next((c for c in cs if c.object_reader.type.name == "MeshFilter"), None)
        mr = next((c for c in cs if c.object_reader.type.name == "MeshRenderer"), None)
        try:
            if sr is not None and parts:
                sn = sprite_node(sr, os.path.join(out_dir, "tex"))
                if sn:
                    node["sprite"] = sn
            if mf is not None and mr is not None and parts:
                os.makedirs(out_dir, exist_ok=True)
                mn = mesh_node(mf, mr, out_dir)
                if mn:
                    node["mesh_part"] = mn
            for c in cs:
                tn = c.object_reader.type.name
                if tn == "MonoBehaviour":
                    sc = c.m_Script.read().m_ClassName if c.m_Script and c.m_Script.path_id else ""
                    if parts and sc in ("TextMeshPro", "NK_TextMeshPro"):
                        node["text"] = text_node(c.object_reader.read_typetree(), tr)
                    if sc in CUSTOM_ANIMS:
                        a = anim_node(c.object_reader.read_typetree())
                        a["kind"] = CUSTOM_ANIMS[sc]
                        node.setdefault("anims", []).append(a)
                elif tn == "Animator":
                    # A clip that can't be read mustn't lose the node's meshes and sprites.
                    try:
                        cl = animator_clip(c)
                    except Exception as e:
                        print("  node", go.m_Name, "clip not exported:", type(e).__name__, e, flush=True)
                        cl = None
                    if cl:
                        node["clip"] = cl
        except Exception as e:
            import traceback
            at = traceback.extract_tb(e.__traceback__)
            where = next((f"{f.name}:{f.lineno}" for f in reversed(at) if f.filename.endswith("export_particles.py")), "?")
            print("  node", go.m_Name, "parts not exported:", type(e).__name__, e, "at", where, flush=True)
        for ch in tr.m_Children:
            walk(ch.read().m_GameObject.read(), idx)

    by_go = {}
    by_ps = {}
    walk(root, None)
    # Sub-emitters: ParticleSystem path ids -> node indices (outside this prefab: dropped).
    for n in systems:
        sy = n.get("system")
        if sy and sy.get("sub"):
            sy["sub"] = [dict(e, emitter=by_ps[e["emitter"]]) for e in sy["sub"] if e["emitter"] in by_ps]
    # NK animator targets: GameObject path ids -> node indices (the component's own node if unset).
    for i, n in enumerate(systems):
        for a in n.get("anims", []):
            a["target"] = by_go.get(a["target"], i)
    # Animator tracks: path hashes (CRC32 of the path below the Animator's node) -> node indices.
    import zlib
    for i, n in enumerate(systems):
        if "clip" not in n:
            continue
        paths = {zlib.crc32(b""): i}

        def below(j, path):
            for k, m in enumerate(systems):
                if m["parent"] == j:
                    pth = (path + "/" if path else "") + m["name"]
                    paths[zlib.crc32(pth.encode())] = k
                    below(k, pth)

        below(i, "")
        n["clip"]["tracks"] = [dict(t, node=paths[t["path"]]) for t in n["clip"]["tracks"] if t["path"] in paths]
    os.makedirs(out_dir, exist_ok=True)
    # UnityDisplayNode.Initialize (Steam build @0x181514830): a display is a sprite (faces the
    # camera: RotationCache's Euler(60, 0, -angle) for its rotation) when its root has an enabled
    # SpriteRenderer (or a certain tag, not exported), else 3D (AngleAxis(angle, up)).
    rsr = next((c for c in [(x.read() if hasattr(x, "read") else x.component.read()) for x in root.m_Components]
                if c.object_reader.type.name == "SpriteRenderer"), None)
    sprite_root = bool(rsr is not None and rsr.object_reader.read_typetree().get("m_Enabled", 1))
    json.dump(finite({"prefab": prefab or name, "sprite_display": sprite_root, "nodes": systems}), open(os.path.join(out_dir, "effect.json"), "w"), indent=1)
    unsup = sorted({u for n in systems for u in n.get("system", {}).get("unsupported", [])})
    print(prefab or name, "->", len([n for n in systems if "system" in n]), "systems", "unsupported:", unsup)
    return len([n for n in systems if "system" in n])


_ENVS = {}
# "@file": one prefab path per line.
PREFABS = [l.strip() for p in PREFABS for l in (open(p[1:]) if p.startswith("@") else [p]) if l.strip()]
for p in sorted(PREFABS, key=lambda p: (INDEX.get(p, ""), p)):
    try:
        export(p)
    except Exception as e:  # one broken prefab must not stop the rest
        print(p, "-> FAIL", type(e).__name__, e, flush=True)
