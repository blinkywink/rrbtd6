"""Export the projectile displays named in local_extracted/display_plans/*.json.

Usage:
    tools/.venv/Scripts/python.exe tools/assets/export_projectiles.py <game install> [name ...]

With names, only those are re-exported; without, only the ones not on disk yet are exported.

Writes local_extracted/models/projectiles/<name>/:
  - mesh prefabs (Spike-o-pult, Juggernaut balls): via export_model.py (meta.json, verts.bin, ...)
  - sprite prefabs (darts, arrows, tacks): sprite.png + sprite.json {w, h, pivot}. The SpriteRenderer
    is rotated 90 degrees about x, so the sprite lies flat with its +y pointing along the flight path.
  - particle-only prefabs (mesh export finds nothing): a STAND-IN sprite.png from the particle texture.
and local_extracted/models/projectiles/index.json {display guid: name}.
Proprietary art: local use only.
"""
import gc
import glob
import json
import os
import subprocess
import sys

import UnityPy
from UnityPy.helpers import TypeTreeHelper

TypeTreeHelper.read_typetree_boost = False

GAME = sys.argv[1]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
ROOT = "local_extracted/models/projectiles"
plan = {}
for f in sorted(glob.glob("local_extracted/display_plans/*.json")):
    plan.update({g: p for g, p in json.load(open(f))["projectiles"].items() if p["prefab"]})
index = json.load(open("local_extracted/all_container_paths.json"))
os.makedirs(ROOT, exist_ok=True)
envs = {}


def sprite_renderer(root):
    """First SpriteRenderer under a prefab root (depth-first), with its GameObject's scale."""
    af = root.assets_file
    g = root.read_typetree()
    stack = [next(c["component"]["m_PathID"] for c in g["m_Component"]
                  if af.objects[c["component"]["m_PathID"]].type.name == "Transform")]
    while stack:
        t = af.objects[stack.pop()].read_typetree()
        go = af.objects[t["m_GameObject"]["m_PathID"]].read_typetree()
        for c in go["m_Component"]:
            o = af.objects.get(c["component"]["m_PathID"])
            if o is not None and o.type.name == "SpriteRenderer":
                sr = o.read_typetree()
                if sr["m_Sprite"]["m_FileID"] == 0 and sr["m_Sprite"]["m_PathID"] in af.objects:
                    return sr, t["m_LocalScale"], af
        stack.extend(ch["m_PathID"] for ch in t["m_Children"])
    return None, None, None


def sprite_layers(root, out):
    """Every active SpriteRenderer of the prefab as a layer (layer_<i>.png + layers.json): its
    offset and angle in the sprite plane relative to the root, size from the whole parent chain,
    tint, and the prefab's animators: CustomRotationSimple (spin, degrees / s about z) and
    CustomScaleAnimator (scale curve over `duration`, looping). Sun Temple / Plasma blasts are a
    stack of these (core, spinning sprite, pulsing orange glow)."""
    import math
    af = root.assets_file
    g = root.read_typetree()
    root_tr = next(c["component"]["m_PathID"] for c in g["m_Component"]
                   if af.objects[c["component"]["m_PathID"]].type.name == "Transform")
    layers = []
    # (transform id, parent 2D frame: offset x / y, angle (rad), scale x / y)
    stack = [(root_tr, (0.0, 0.0, 0.0, 1.0, 1.0), True)]
    while stack:
        tid, (px, py, pa, psx, psy), is_root = stack.pop()
        t = af.objects[tid].read_typetree()
        go = af.objects[t["m_GameObject"]["m_PathID"]].read_typetree()
        if not go.get("m_IsActive", 1):
            continue
        lp, lq, ls = t["m_LocalPosition"], t["m_LocalRotation"], t["m_LocalScale"]
        if is_root:
            frame = (0.0, 0.0, 0.0, ls["x"], ls["y"])
        else:
            # Rotation about the sprite plane's normal (z): the angle of the local quaternion.
            ang = 2.0 * math.atan2(lq["z"], lq["w"])
            ox, oy = lp["x"] * psx, lp["y"] * psy
            ca, sa = math.cos(pa), math.sin(pa)
            frame = (px + ox * ca - oy * sa, py + ox * sa + oy * ca, pa + ang, psx * ls["x"], psy * ls["y"])
        comps = [af.objects.get(c["component"]["m_PathID"]) for c in go["m_Component"]]
        sr = next((o for o in comps if o is not None and o.type.name == "SpriteRenderer"), None)
        spin, scale_anim = 0.0, None
        for o in comps:
            if o is None or o.type.name != "MonoBehaviour":
                continue
            try:
                mb = o.read_typetree()
            except Exception:
                continue
            if "rotationAxis" in mb and "speed" in mb and mb.get("isEnabled", 1):
                spin = mb["speed"] * (mb["rotationAxis"].get("z", 0.0) or 1.0)
            if "curve" in mb and "duration" in mb and isinstance(mb["curve"], dict):
                keys = [[k["time"], k["value"]] for k in mb["curve"].get("m_Curve", [])]
                if keys:
                    scale_anim = {"duration": mb["duration"], "keys": keys}
        if sr is not None:
            r = sr.read_typetree()
            sp_pp = r["m_Sprite"]
            if r.get("m_Enabled", 1) and sp_pp["m_PathID"] and sp_pp["m_FileID"] == 0 and sp_pp["m_PathID"] in af.objects:
                col = r.get("m_Color") or {}
                color = [col.get(k, 1.0) for k in ("r", "g", "b", "a")]
                if color[3] > 0.001:
                    sp = af.objects[sp_pp["m_PathID"]].read()
                    i = len(layers)
                    sp.image.convert("RGBA").save(os.path.join(out, f"layer_{i}.png"))
                    ppu = sp.m_PixelsToUnits
                    layers.append({"png": f"layer_{i}.png", "w": sp.m_Rect.width / ppu * frame[3],
                                   "h": sp.m_Rect.height / ppu * frame[4], "pivot": [sp.m_Pivot.x, sp.m_Pivot.y],
                                   "offset": [frame[0], frame[1]], "angle": math.degrees(frame[2]), "color": color,
                                   "order": r.get("m_SortingOrder", 0), "spin": spin, "scale_anim": scale_anim,
                                   "flip": [bool(r.get("m_FlipX", 0)), bool(r.get("m_FlipY", 0))]})
        for ch in reversed(t["m_Children"]):
            stack.append((ch["m_PathID"], frame, False))
    if layers:
        json.dump({"layers": layers}, open(os.path.join(out, "layers.json"), "w"), indent=1)
    return len(layers)


def particle_sprite(root):
    """Stand-in sprite for particle-only prefabs (Wizard bolts, fireballs): the texture of the first
    ParticleSystemRenderer whose material lives in the same file, plus that system's startSize.
    Flipbook atlases (4x4 for ArcaneShot) are cropped to their first cell; grayscale flame masks
    (_MainTexture with _Color0/_CentreFlame) are tinted from dark flame colour to centre colour."""
    from PIL import Image
    af = root.assets_file
    g = root.read_typetree()
    stack = [next(c["component"]["m_PathID"] for c in g["m_Component"]
                  if af.objects[c["component"]["m_PathID"]].type.name == "Transform")]
    while stack:
        t = af.objects[stack.pop()].read_typetree()
        go = af.objects[t["m_GameObject"]["m_PathID"]].read_typetree()
        objs = [af.objects.get(c["component"]["m_PathID"]) for c in go["m_Component"]]
        ps = next((o for o in objs if o is not None and o.type.name == "ParticleSystem"), None)
        psr = next((o for o in objs if o is not None and o.type.name == "ParticleSystemRenderer"), None)
        stack.extend(ch["m_PathID"] for ch in t["m_Children"])
        if ps is None or psr is None:
            continue
        for m in psr.read_typetree()["m_Materials"]:
            if m["m_FileID"] != 0 or m["m_PathID"] not in af.objects:
                continue
            mt = af.objects[m["m_PathID"]].read_typetree()
            texs = {k: v for k, v in mt["m_SavedProperties"]["m_TexEnvs"] if v["m_Texture"]["m_PathID"] and v["m_Texture"]["m_FileID"] == 0}
            cols = {k: v for k, v in mt["m_SavedProperties"]["m_Colors"]}
            key = next((k for k in ("_BaseMap", "_MainTex", "_MainTexture") if k in texs), None)
            if key is None:
                continue
            img = af.objects[texs[key]["m_Texture"]["m_PathID"]].read().image.convert("RGBA")
            p = ps.read_typetree()
            sheet = p.get("UVModule", {})
            tx, ty = (sheet.get("tilesX", 1), sheet.get("tilesY", 1)) if sheet.get("enabled") else (1, 1)
            if tx > 1 or ty > 1:
                cw, ch = img.width // tx, img.height // ty
                img = img.crop((0, 0, cw, ch))
            if "_Color0" in cols and "_CentreFlame" in cols:
                lo, hi = cols["_Color0"], cols["_CentreFlame"]
                px = img.load()
                for y in range(img.height):
                    for x in range(img.width):
                        v = px[x, y][0] / 255
                        px[x, y] = tuple(int(255 * (lo[c] + (hi[c] - lo[c]) * v * v)) for c in "rgb") + (int(255 * min(1, v * 1.6)),)
            size = p["InitialModule"]["startSize"]["scalar"] * 0.6
            return img, size
    return None, None


def export_mesh(out, bundle, prefab):
    r = subprocess.run([sys.executable, "tools/assets/export_model.py", GAME, out, "prefab",
                        bundle.rsplit("_", 1)[0], prefab], capture_output=True, text=True)
    return "mesh " + (r.stdout.strip().splitlines() or ["?"])[0] if r.returncode == 0 else "FAIL " + r.stderr[-200:]


ONLY = set(sys.argv[2:])
done = {}
# Bundle by bundle, one loaded at a time: holding them all runs out of memory.
for guid, p in sorted(plan.items(), key=lambda kv: (index.get(kv[1]["prefab"], ""), kv[0])):
    name, prefab = p["name"], p["prefab"]
    if name in done:
        continue
    on_disk = os.path.isdir(os.path.join(ROOT, name)) and os.listdir(os.path.join(ROOT, name))
    if (ONLY and name not in ONLY) or (not ONLY and on_disk):
        if on_disk:
            done[name] = "kept"
        continue
    if prefab not in index:
        print(name, "MISSING prefab", prefab)
        continue
    bundle = index[prefab]
    if bundle not in envs:
        envs.clear()
        gc.collect()
        try:
            envs[bundle] = UnityPy.load(os.path.join(AA, bundle))
        except MemoryError:
            # asset_references_assets_all is ~900 MB packed; it needs several GB free.
            print(name, "FAIL out of memory loading", bundle, flush=True)
            continue
    root = dict(envs[bundle].container.items())[prefab].deref()
    sr, scale, af = sprite_renderer(root)
    out = os.path.join(ROOT, name)
    os.makedirs(out, exist_ok=True)
    if sr is not None:
        sp = af.objects[sr["m_Sprite"]["m_PathID"]].read()
        img = sp.image.convert("RGBA")
        # SpriteRenderer.m_Color tints white masks (acid puddles).
        col = sr.get("m_Color") or {}
        tint = [col.get(k, 1.0) for k in ("r", "g", "b", "a")]
        if any(abs(v - 1.0) > 1e-3 for v in tint):
            bands = [b.point(lambda x, m=m: int(x * m)) for b, m in zip(img.split(), tint)]
            from PIL import Image
            img = Image.merge("RGBA", bands)
        img.save(os.path.join(out, "sprite.png"))
        ppu = sp.m_PixelsToUnits
        w, h = sp.m_Rect.width / ppu * scale["x"], sp.m_Rect.height / ppu * scale["y"]
        json.dump({"w": w, "h": h, "pivot": [sp.m_Pivot.x, sp.m_Pivot.y]}, open(os.path.join(out, "sprite.json"), "w"))
        n = sprite_layers(root, out)
        done[name] = f"sprite {w:.2f} x {h:.2f}, {n} layers"
    else:
        # The mesh subprocess loads the bundle itself: free ours first so only one copy of a big
        # bundle (asset_references_assets_all, ~13 GB unpacked) is in memory at a time.
        if os.path.getsize(os.path.join(AA, bundle)) > 200_000_000:
            envs.clear()
            root = sr = af = None
            gc.collect()
        done[name] = export_mesh(out, bundle, prefab)
        if done[name].startswith("FAIL"):
            # The subprocess runs out of memory while this process holds every loaded bundle.
            envs.clear()
            gc.collect()
            done[name] = export_mesh(out, bundle, prefab)
        if done[name].startswith(("mesh 0 parts", "FAIL")) and bundle not in envs:
            try:
                envs[bundle] = UnityPy.load(os.path.join(AA, bundle))
                root = dict(envs[bundle].container.items())[prefab].deref()
            except MemoryError:
                envs.clear()
                gc.collect()
                print(name, done[name], "(no reload for the particle stand-in: out of memory)", flush=True)
                continue
        if done[name].startswith(("mesh 0 parts", "FAIL")):
            img, size = particle_sprite(root)
            if img is not None:
                img.save(os.path.join(out, "sprite.png"))
                json.dump({"w": size, "h": size, "pivot": [0.5, 0.5]}, open(os.path.join(out, "sprite.json"), "w"))
                done[name] = f"particle sprite {size:.2f}"
    print(name, done[name], flush=True)
json.dump({g: p["name"] for g, p in plan.items() if p["name"] in done}, open(os.path.join(ROOT, "index.json"), "w"), indent=1)
