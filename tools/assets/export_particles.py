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

SIMULATED = {"InitialModule", "EmissionModule", "ShapeModule", "SizeModule", "ColorModule", "RotationModule",
             "VelocityModule", "ForceModule", "ClampVelocityModule", "UVModule"}


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
                      "speed_mod": curve(m["speedModifier"]) if "speedModifier" in m else None}
    cv = t["ClampVelocityModule"]
    if cv["enabled"]:
        s["limit"] = {"speed": curve(cv["magnitude"]), "dampen": cv.get("dampen", 1.0), "drag": curve(cv["drag"]) if "drag" in cv else None}
    uv = t["UVModule"]
    if uv["enabled"]:
        s["sheet"] = {"x": uv["tilesX"], "y": uv["tilesY"], "frame": curve(uv["frameOverTime"]),
                      "start": curve(uv["startFrame"]), "cycles": uv.get("cycles", 1.0), "row_mode": uv.get("rowMode", 0),
                      "anim": uv.get("animationType", 0), "row": uv.get("rowIndex", 0)}
    s["unsupported"] = sorted(k for k, v in t.items() if k.endswith("Module") and isinstance(v, dict)
                              and v.get("enabled") and k not in SIMULATED)
    return s


def renderer(r, out_tex):
    t = r.read_typetree()
    out = {"mode": t.get("m_RenderMode", 0), "length": t.get("m_LengthScale", 2.0), "vel_scale": t.get("m_VelocityScale", 0.0),
           "max_size": t.get("m_MaxParticleSize", 0.5), "min_size": t.get("m_MinParticleSize", 0.0),
           "pivot": vec(t.get("m_Pivot", {"x": 0, "y": 0, "z": 0})), "fudge": t.get("m_SortingFudge", 0.0),
           "align": t.get("m_RenderAlignment", 0), "enabled": bool(t.get("m_Enabled", 1))}
    # Mesh particles (mode 4): the mesh's name (the effects here use Unity's Quad).
    meshes = [m for m in t.get("m_Meshes", [t.get("m_Mesh")]) if m and m.get("m_PathID")]
    if meshes:
        try:
            mo = r.assets_file.objects.get(meshes[0]["m_PathID"]) if meshes[0]["m_FileID"] == 0 else None
            out["mesh"] = mo.read().m_Name if mo else "external"
        except Exception:
            out["mesh"] = "?"
    mats = [m for m in t.get("m_Materials", []) if m["m_PathID"]]
    if not mats:
        return out
    mo = r.assets_file.objects.get(mats[0]["m_PathID"]) if mats[0]["m_FileID"] == 0 else None
    if mo is None:
        ext = r.assets_file.externals[mats[0]["m_FileID"] - 1].path.lower().rsplit("/", 1)[-1]
        for f in r.assets_file.environment.files.values():
            for k, sf in getattr(f, "files", {}).items():
                if k.lower() == ext and hasattr(sf, "objects"):
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
        out["shader"] = so.read().m_ParsedForm.m_Name if so else None
    except Exception:
        out["shader"] = None
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


def load_bundle(path):
    """UnityPy env of bundle `path` with the monoscripts and the bundles its externals live in
    (materials, textures and shaders shared across bundles: unitybuiltinassets, ...), else their
    PPtrs read as missing (an effect with no material, texture or shader). cab_index.py maps the
    CAB names to bundle files."""
    name = os.path.basename(path)
    files = [path] + glob.glob(os.path.join(AA, "*_monoscripts_*.bundle"))
    probe = UnityPy.load(path)
    deps = set()
    for f in probe.files.values():
        for af in getattr(f, "files", {}).values():
            for ext in getattr(af, "externals", []) or []:
                cab = os.path.basename(ext.path).lower()
                if cab in CABS and CABS[cab] != name:
                    deps.add(CABS[cab])
    del probe
    return UnityPy.load(*files + [bpath(d) for d in sorted(deps)])


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
        export_root(env, go, name, {"pos": [0, 0, 0], "rot": r, "scale": s})
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
        n = export_root(env, root, name, None, p["prefab"])
        if n:
            out[stem] = name
        else:
            import shutil
            shutil.rmtree(os.path.join(OUT, name), ignore_errors=True)
    json.dump(out, open(os.path.join(OUT, f"tier_{tower}.json"), "w"), indent=1)
    print(tower, "->", len(out), "tier prefabs with particle effects")


def export_projectiles():
    """Particle systems inside projectile prefabs (Wizard bolts, fireballs, trails): one effect
    per prefab with any, proj_<name>, and <OUT>/proj_index.json {projectile name: effect}.
    Bundle by bundle so only one big bundle is loaded at a time."""
    plan = {}
    for f in sorted(glob.glob(os.path.join(ROOT, "..", "..", "local_extracted", "display_plans", "*.json"))):
        for g, p in json.load(open(f))["projectiles"].items():
            if p.get("prefab") and p.get("name"):
                plan[p["name"]] = p["prefab"]
    out = {}
    for name, prefab in sorted(plan.items(), key=lambda kv: INDEX.get(kv[1], "")):
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
        if export_root(env, root, eff, None, prefab):
            out[name] = eff
        else:
            import shutil
            shutil.rmtree(os.path.join(OUT, eff), ignore_errors=True)
    json.dump(out, open(os.path.join(OUT, "proj_index.json"), "w"), indent=1)
    print(len(out), "projectiles with particle effects")


def export(prefab):
    if prefab == "projectiles":
        export_projectiles()
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


def export_root(env, root, name, root_tr=None, prefab=None):
    """Writes <OUT>/<name>/effect.json for the particle systems under GameObject `root`;
    `root_tr` overrides the root node's transform (scene objects: their world rotation / scale)."""
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
        ps = next((c for c in cs if c.object_reader.type.name == "ParticleSystem"), None)
        pr = next((c for c in cs if c.object_reader.type.name == "ParticleSystemRenderer"), None)
        if parent_path is None and root_tr is not None:
            node.update(root_tr)
        idx = len(systems)
        systems.append(node)
        if ps is not None:
            node["system"] = system(ps.object_reader, os.path.join(out_dir, "tex"), env)
            node["renderer"] = renderer(pr.object_reader, os.path.join(out_dir, "tex")) if pr else None
        for ch in tr.m_Children:
            walk(ch.read().m_GameObject.read(), idx)

    walk(root, None)
    os.makedirs(out_dir, exist_ok=True)
    json.dump(finite({"prefab": prefab or name, "nodes": systems}), open(os.path.join(out_dir, "effect.json"), "w"), indent=1)
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
