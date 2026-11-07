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

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sprite_resolver import SpriteImages
import export_checkpoint
import static_model_batch


GAME = sys.argv[1]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
ROOT = "local_extracted/models/projectiles"
_sprite_images = SpriteImages(AA)
plan = {}
for f in sorted(glob.glob("local_extracted/display_plans/*.json")):
    plan.update({g: p for g, p in json.load(open(f))["projectiles"].items() if p["prefab"]})
index = json.load(open("local_extracted/all_container_paths.json"))
os.makedirs(ROOT, exist_ok=True)
envs = {}


def load_bundle(bundle):
    """The bundle's scene: from its decompressed files in local_extracted/_bundle_cache when
    there (asset_references_assets_all is ~900 MB packed: decompressing it again runs out of
    memory), else the bundle itself."""
    cached = sorted(glob.glob(os.path.join("local_extracted/_bundle_cache", glob.escape(bundle) + "-*")))
    cached = [d for d in cached if os.path.isdir(d)]
    if cached:
        return UnityPy.load(*[os.path.join(cached[0], f) for f in sorted(os.listdir(cached[0]))])
    return UnityPy.load(os.path.join(AA, bundle))


def full_rect_image(sp):
    """The sprite's image padded back to its full m_Rect: atlas packing trims transparent margins
    (m_RD.textureRect at textureRectOffset inside m_Rect, y up), and the renderer draws the image
    over the whole rect (w, h, pivot) (UnpoppedArmyBloon: 120 x 170 art in a 256 x 256 rect)."""
    from PIL import Image
    src = _sprite_images.source(sp) if not sp.m_RD.texture.m_PathID else sp
    img = _sprite_images.image(sp).convert("RGBA")
    w, h = round(src.m_Rect.width), round(src.m_Rect.height)
    if img.size == (w, h):
        return img
    off = src.m_RD.textureRectOffset
    out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    out.paste(img, (round(off.x), round(h - off.y - img.size[1])))
    return out


def sprite_renderer(root):
    """First SpriteRenderer under a prefab root (depth-first), with its GameObject's scale."""
    af = root.assets_file
    g = root.read_typetree()
    stack = [next(c["component"]["m_PathID"] for c in g["m_Component"]
                  if af.objects[c["component"]["m_PathID"]].type.name == "Transform")]
    found = None
    while stack:
        t = af.objects[stack.pop()].read_typetree()
        go = af.objects[t["m_GameObject"]["m_PathID"]].read_typetree()
        for c in go["m_Component"]:
            o = af.objects.get(c["component"]["m_PathID"])
            # A rigged model with a sprite on it (Bloon Trap's blinker light) is a mesh.
            if o is not None and o.type.name == "SkinnedMeshRenderer":
                return None, None, None
            if found is None and o is not None and o.type.name == "SpriteRenderer":
                sr = o.read_typetree()
                if sr["m_Sprite"]["m_FileID"] == 0 and sr["m_Sprite"]["m_PathID"] in af.objects:
                    found = sr, t["m_LocalScale"], af
        stack.extend(ch["m_PathID"] for ch in t["m_Children"])
    return found or (None, None, None)


def sprite_layers(root, out):
    """Every active SpriteRenderer of the prefab as a layer (layer_<i>.png + layers.json): its
    offset and angle in the sprite plane relative to the root (x across the flight, y along it),
    size from the whole parent chain,
    tint, and the prefab's animators: CustomRotationSimple (spin, degrees / s about z) and
    CustomScaleAnimator (scale curve over `duration`, looping). Sun Temple / Plasma blasts are a
    stack of these (core, spinning sprite, pulsing orange glow)."""
    import math
    af = root.assets_file
    g = root.read_typetree()
    root_tr = next(c["component"]["m_PathID"] for c in g["m_Component"]
                   if af.objects[c["component"]["m_PathID"]].type.name == "Transform")

    def qmul(a, b):
        ax, ay, az, aw = a
        bx, by, bz, bw = b
        return (aw * bx + ax * bw + ay * bz - az * by, aw * by - ax * bz + ay * bw + az * bx,
                aw * bz + ax * by - ay * bx + az * bw, aw * bw - ax * bx - ay * by - az * bz)

    def qrot(q, v):
        x, y, z, w = qmul(qmul(q, (v[0], v[1], v[2], 0.0)), (-q[0], -q[1], -q[2], q[3]))
        return (x, y, z)

    layers = []
    # (transform id, parent frame: position, rotation, scale x / y / z, all relative to the root)
    # Each entry also carries the animators of its ancestors: a container's scale / colour
    # animator acts on what is under it (UnpoppedArmyBloon's GlowAnimContainer fades its glow).
    stack = [(root_tr, None, True, [])]
    while stack:
        tid, parent, is_root, inherited = stack.pop()
        t = af.objects[tid].read_typetree()
        go = af.objects[t["m_GameObject"]["m_PathID"]].read_typetree()
        if not go.get("m_IsActive", 1):
            continue
        lp, lq, ls = t["m_LocalPosition"], t["m_LocalRotation"], t["m_LocalScale"]
        if is_root:
            frame = ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0, 1.0), (ls["x"], ls["y"], ls["z"]))
        else:
            (ppos, pq, psc) = parent
            off = qrot(pq, (lp["x"] * psc[0], lp["y"] * psc[1], lp["z"] * psc[2]))
            frame = ((ppos[0] + off[0], ppos[1] + off[1], ppos[2] + off[2]),
                     qmul(pq, (lq["x"], lq["y"], lq["z"], lq["w"])),
                     (psc[0] * ls["x"], psc[1] * ls["y"], psc[2] * ls["z"]))
        # The sprite plane: the ground (x, z) for a sprite turned to lie flat (its normal up or
        # down; the usual projectile, whose +y then points along the flight), else the root's x / y.
        # A tilted node's own height (GlaiveLordGlaiveOrbit's 4 up) isn't an offset in the plane.
        (wpos, wq, wsc) = frame
        normal, axis_x = qrot(wq, (0.0, 0.0, 1.0)), qrot(wq, (1.0, 0.0, 0.0))
        k = 2 if abs(normal[1]) > abs(normal[2]) else 1
        offset = (wpos[0], wpos[k])
        angle = math.atan2(axis_x[k], axis_x[0])
        # A frame that mirrors the sprite in that plane (UnpoppedArmyBloon's OffsetContainer, turned
        # 180 degrees about x): its y flips.
        axis_y = qrot(wq, (0.0, 1.0, 0.0))
        mirrored = axis_x[0] * axis_y[k] - axis_x[k] * axis_y[0] < 0.0
        comps = [af.objects.get(c["component"]["m_PathID"]) for c in go["m_Component"]]
        sr = next((o for o in comps if o is not None and o.type.name == "SpriteRenderer"), None)
        spin = 0.0
        anims = list(inherited)
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
                    # (An animator without a loop flag is the looping CustomScaleAnimator kind.)
                    anim = {"duration": mb["duration"], "keys": keys, "loop": bool(mb.get("loop", 1))}
                    if "color1" in mb and "color2" in mb:
                        anim["colors"] = [[mb[c].get(k, 1.0) for k in ("r", "g", "b", "a")] for c in ("color1", "color2")]
                    anims.append(anim)
        scale_anim = next((a for a in reversed(anims) if "colors" not in a), None)
        color_anim = next((a for a in reversed(anims) if "colors" in a), None)
        if sr is not None:
            r = sr.read_typetree()
            sp_pp = r["m_Sprite"]
            if r.get("m_Enabled", 1) and sp_pp["m_PathID"] and sp_pp["m_FileID"] == 0 and sp_pp["m_PathID"] in af.objects:
                col = r.get("m_Color") or {}
                color = [col.get(k, 1.0) for k in ("r", "g", "b", "a")]
                if color[3] > 0.001:
                    sp = af.objects[sp_pp["m_PathID"]].read()
                    i = len(layers)
                    full_rect_image(sp).save(os.path.join(out, f"layer_{i}.png"))
                    ppu = sp.m_PixelsToUnits
                    layers.append({"png": f"layer_{i}.png", "w": sp.m_Rect.width / ppu * wsc[0],
                                   "h": sp.m_Rect.height / ppu * wsc[1], "pivot": [sp.m_Pivot.x, sp.m_Pivot.y],
                                   "offset": [offset[0], offset[1]], "angle": math.degrees(angle), "color": color,
                                   "order": r.get("m_SortingOrder", 0), "spin": spin, "scale_anim": scale_anim, "color_anim": color_anim,
                                   "flip": [bool(r.get("m_FlipX", 0)), bool(r.get("m_FlipY", 0)) != mirrored]})
        for ch in reversed(t["m_Children"]):
            stack.append((ch["m_PathID"], frame, False, anims))
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


def export_mesh(out, bundle, prefab, env=None):
    if env is not None:
        try:
            return static_model_batch.export(GAME, out, bundle, prefab, env)
        except Exception as error:
            return "FAIL " + str(error)[-200:]
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
    out = os.path.join(ROOT, name)
    signature = json.dumps([1, index.get(prefab), prefab])
    existing = os.path.isdir(out) and bool(os.listdir(out))
    on_disk = export_checkpoint.complete(out, signature)
    if ONLY and name not in ONLY:
        if existing:
            done[name] = "kept"
        continue
    if not ONLY and on_disk:
        done[name] = "kept"
        continue
    export_checkpoint.begin(out)
    if prefab not in index:
        print(name, "MISSING prefab", prefab)
        continue
    bundle = index[prefab]
    if bundle not in envs:
        envs.clear()
        gc.collect()
        try:
            envs[bundle] = load_bundle(bundle)
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
        img = full_rect_image(sp)
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
        # Reuse the scene already loaded to inspect the projectile. This avoids
        # another interpreter, decompression and catalogue parse for each mesh.
        done[name] = export_mesh(out, bundle, prefab, envs[bundle])
        if done[name].startswith("FAIL"):
            # The subprocess runs out of memory while this process holds every loaded bundle.
            static_model_batch.release()
            envs.clear()
            root = sr = af = None
            gc.collect()
            done[name] = export_mesh(out, bundle, prefab)
        if done[name].startswith(("mesh 0 parts", "FAIL")) and bundle not in envs:
            try:
                envs[bundle] = load_bundle(bundle)
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
    if done[name].startswith("FAIL"):
        raise RuntimeError(name + ": " + done[name])
    export_checkpoint.commit(out, signature)
    print(name, done[name], flush=True)
json.dump({g: p["name"] for g, p in plan.items() if p["name"] in done}, open(os.path.join(ROOT, "index.json"), "w"), indent=1)
