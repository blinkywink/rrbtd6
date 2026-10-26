"""Export a sprite-flipbook effect prefab (e.g. Assets/Bloons/Graphics/Effects/Pop.prefab).

Finds the flipbook MonoBehaviour (fields fps, frames[]) and lifespan, exports each frame
(late-bound atlas sprites resolved through SpriteAtlas render data) with its world size.

Usage: tools/.venv/Scripts/python.exe tools/assets/export_effect_sprites.py <game install> <prefab path> <out_dir>
"""
import glob
import json
import os
import sys

import UnityPy
from UnityPy.helpers import TypeTreeHelper

TypeTreeHelper.read_typetree_boost = False


def main():
    game, prefab, out = sys.argv[1:4]
    aa = os.path.join(game, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
    env = UnityPy.load(glob.glob(os.path.join(aa, "asset_references_assets_all_*.bundle"))[0])
    senv = UnityPy.load(glob.glob(os.path.join(aa, "sprite_atlases_assets_all_*.bundle"))[0])
    atlas_data = {}
    for o in senv.objects:
        if o.type.name == "SpriteAtlas":
            for k, v in o.read().m_RenderDataMap:
                atlas_data[(str(k[0]), k[1])] = v
    ab = next(o for o in env.objects if o.type.name == "AssetBundle").read_typetree()
    pid = next(v["asset"]["m_PathID"] for k, v in ab["m_Container"] if k == prefab)
    root = next(o for o in env.objects if o.path_id == pid)
    af = root.assets_file
    go = root.read_typetree()
    comps = [af.objects.get(c["component"]["m_PathID"]) for c in go["m_Component"]]
    tr = next(c for c in comps if c and c.type.name == "Transform").read_typetree()
    info = {"scale": tr["m_LocalScale"]["x"], "offset": [tr["m_LocalPosition"][k] for k in "xyz"], "frames": []}
    frames = []
    for c in comps:
        if c is not None and c.type.name == "MonoBehaviour":
            t = c.read_typetree()
            if "lifespan" in t:
                info["lifespan"] = t["lifespan"]
            if "frames" in t and "fps" in t:
                info["fps"] = t["fps"]
                frames = t["frames"]
    os.makedirs(out, exist_ok=True)
    cache = {}
    for i, f in enumerate(frames):
        sp = af.objects[f["m_PathID"]].read()
        try:
            img = sp.image
        except Exception:
            d = atlas_data[(str(sp.m_RenderDataKey[0]), sp.m_RenderDataKey[1])]
            tid = d.texture.m_PathID
            if tid not in cache:
                cache[tid] = d.texture.read().image
            im = cache[tid]
            r = d.textureRect
            img = im.crop((round(r.x), round(im.height - r.y - r.height), round(r.x + r.width), round(im.height - r.y)))
        name = f"{i}.png"
        img.save(os.path.join(out, name))
        # Tight (cropped) pixel size at the sprite's pixels-per-unit.
        info["frames"].append({"png": name, "w": img.width / sp.m_PixelsToUnits,
                               "h": img.height / sp.m_PixelsToUnits})
    json.dump(info, open(os.path.join(out, "effect.json"), "w"), indent=1)
    print(json.dumps(info))


if __name__ == "__main__":
    main()
