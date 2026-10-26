"""Export each bloon display prefab's sprite(s) with their in-world size.

Reads Assets/Bloons/Graphics/Prefabs/*.prefab from asset_references. For every
SpriteRenderer in the prefab: the sprite image (resolved by object, not by
name, since names repeat across atlases), its world size (sprite rect /
pixelsPerUnit, times the renderer transform scale), local offset and sorting order.

Usage: tools/.venv/Scripts/python.exe tools/assets/export_bloon_displays.py <game install> <out_dir>
Writes <out_dir>/<Prefab>.png (main sprite) and <out_dir>/displays.json.
"""
import glob
import json
import os
import sys

import UnityPy
from UnityPy.helpers import TypeTreeHelper

TypeTreeHelper.read_typetree_boost = False


def main():
    game, out = sys.argv[1:3]
    aa = os.path.join(game, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
    env = UnityPy.load(glob.glob(os.path.join(aa, "asset_references_assets_all_*.bundle"))[0])
    # Sprites are read in their own environment (atlas lookups only work there), by path id.
    sprites_env = UnityPy.load(glob.glob(os.path.join(aa, "sprite_atlases_assets_all_*.bundle"))[0])
    sprite_by_id = {o.path_id: o for o in sprites_env.objects if o.type.name == "Sprite"}
    # Late-bound atlas sprites: resolve through SpriteAtlas.m_RenderDataMap by render-data key.
    atlas_data, atlas_img = {}, {}
    for o in sprites_env.objects:
        if o.type.name == "SpriteAtlas":
            a = o.read()
            for k, v in a.m_RenderDataMap:
                atlas_data[(str(k[0]), k[1])] = v

    def sprite_image(sp):
        try:
            return sp.image
        except Exception:
            pass
        d = atlas_data.get((str(sp.m_RenderDataKey[0]), sp.m_RenderDataKey[1]))
        if d is None:
            raise KeyError("no atlas render data")
        tid = d.texture.m_PathID
        if tid not in atlas_img:
            atlas_img[tid] = d.texture.read().image
        img = atlas_img[tid]
        r = d.textureRect
        H = img.height
        crop = img.crop((round(r.x), round(H - r.y - r.height), round(r.x + r.width), round(H - r.y)))
        rot = (d.settingsRaw >> 2) & 0xF  # SpritePackingRotation
        if rot == 1:
            crop = crop.transpose(0)  # FlipHorizontal
        elif rot == 2:
            crop = crop.transpose(1)  # FlipVertical
        elif rot == 3:
            crop = crop.rotate(180)
        elif rot == 4:
            crop = crop.rotate(90, expand=True)
        return crop
    cab = json.load(open(os.path.join(os.path.dirname(out.rstrip("/\\")), "cab_index.json")))
    ab = next(o for o in env.objects if o.type.name == "AssetBundle").read_typetree()
    os.makedirs(out, exist_ok=True)
    result = {}
    for path, v in ab["m_Container"]:
        if not (path.startswith("Assets/Bloons/Graphics/Prefabs/") and path.endswith(".prefab")):
            continue
        name = os.path.splitext(os.path.basename(path))[0]
        root = next(o for o in env.objects if o.path_id == v["asset"]["m_PathID"])
        af = root.assets_file
        layers = []
        stack = [(root.read_typetree(), (0.0, 0.0, 0.0), 1.0)]
        while stack:
            go, off, scale = stack.pop()
            comps = [af.objects[c["component"]["m_PathID"]] for c in go["m_Component"]]
            tr = next(c for c in comps if c.type.name == "Transform").read_typetree()
            lp, ls = tr["m_LocalPosition"], tr["m_LocalScale"]
            o2 = (off[0] + lp["x"] * scale, off[1] + lp["y"] * scale, off[2] + lp["z"] * scale)
            s2 = scale * ls["x"]
            for c in comps:
                if c.type.name != "SpriteRenderer":
                    continue
                r = c.read_typetree()
                if not r.get("m_Enabled", 1) or not go.get("m_IsActive", 1):
                    continue
                ref = r["m_Sprite"]
                if ref["m_FileID"] == 0:
                    sp_obj = af.objects.get(ref["m_PathID"])
                else:
                    ext = af.externals[ref["m_FileID"] - 1].path.split("/")[-1]
                    sp_obj = sprite_by_id.get(ref["m_PathID"])
                    if sp_obj is None and ext in cab:
                        print("sprite in other bundle", name, cab[ext])
                if sp_obj is None:
                    continue
                sp = sp_obj.read()
                try:
                    img = sprite_image(sp)
                except Exception as e:
                    print("sprite image fail", name, sp.m_Name, e)
                    continue
                # SpriteRenderer.m_Size is the drawn size of the (tight) sprite in local units.
                sz = r.get("m_Size")
                if sz and sz["x"] > 0:
                    w, h = sz["x"] * s2, sz["y"] * s2
                else:
                    w, h = sp.m_Rect.width / sp.m_PixelsToUnits * s2, sp.m_Rect.height / sp.m_PixelsToUnits * s2
                fname = f"{name}_{len(layers)}.png"
                img.save(os.path.join(out, fname))
                layers.append({"png": fname, "w": w, "h": h, "offset": o2, "order": r.get("m_SortingOrder", 0),
                               "pivot": [sp.m_Pivot.x, sp.m_Pivot.y], "sprite": sp.m_Name,
                               "color": [r["m_Color"][k] for k in "rgba"]})
            for ch in tr["m_Children"]:
                ct = af.objects[ch["m_PathID"]].read_typetree()
                stack.append((af.objects[ct["m_GameObject"]["m_PathID"]].read_typetree(), o2, s2))
        if layers:
            layers.sort(key=lambda l: l["order"])
            result[name] = layers
    json.dump(result, open(os.path.join(out, "displays.json"), "w"), indent=1)
    print(len(result), "bloon displays")
    for k in ("Red", "Blue", "Pink", "Ceramic", "Lead", "RedCamo", "RedRegrow", "Moab"):
        print(k, json.dumps(result.get(k)))


if __name__ == "__main__":
    main()
