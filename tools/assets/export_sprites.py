"""Export sprites from the sprite_atlases bundle as PNGs.

Usage: tools/.venv/Scripts/python.exe tools/assets/export_sprites.py <game install> <out_dir> [Atlas ...]
Writes <out_dir>/<Atlas>/<SpriteName>.png, plus <out_dir>/<Atlas>/_borders.json
{name: [left, bottom, right, top, pixelsPerUnit]} (Sprite.m_Border in pixels, m_PixelsToUnits).
Proprietary art: local use only.
"""
import glob
import json
import os
import sys

import UnityPy
from PIL import Image


def full_rect(s):
    """The sprite image padded back to its full m_Rect.

    Atlas packing trims transparent margins (m_RD.textureRect inside m_Rect at
    textureRectOffset, y up); uGUI still lays the image out over the full rect.
    """
    img = s.image
    w, h = round(s.m_Rect.width), round(s.m_Rect.height)
    if img.size == (w, h):
        return img
    off = s.m_RD.textureRectOffset
    out = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    out.paste(img, (round(off.x), round(h - off.y - img.size[1])))
    return out


def main():
    game, out = sys.argv[1:3]
    want = set(sys.argv[3:])
    aa = os.path.join(game, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
    env = UnityPy.load(glob.glob(os.path.join(aa, "sprite_atlases_assets_all_*.bundle"))[0])
    # By sprite object, not name: atlases share names (Bloons/Zebra and UiEmoteIcons/Zebra are
    # different sprites) and the name of the last atlas used to win for both.
    atlas_of = {}
    for o in env.objects:
        if o.type.name == "SpriteAtlas":
            t = o.read_typetree()
            for pp in t.get("m_PackedSprites", []):
                if pp["m_FileID"] == 0:
                    atlas_of[pp["m_PathID"]] = t["m_Name"]
    n = 0
    borders = {}
    for o in env.objects:
        if o.type.name != "Sprite":
            continue
        s = o.read()
        atlas = atlas_of.get(o.path_id, "_unpacked")
        if want and atlas not in want:
            continue
        d = os.path.join(out, atlas)
        os.makedirs(d, exist_ok=True)
        try:
            full_rect(s).save(os.path.join(d, s.m_Name + ".png"))
            n += 1
        except Exception as e:
            print("skip", s.m_Name, e)
        b = s.m_Border
        borders.setdefault(atlas, {})[s.m_Name] = [b.x, b.y, b.z, b.w, s.m_PixelsToUnits]
    for atlas, bs in borders.items():
        json.dump(bs, open(os.path.join(out, atlas, "_borders.json"), "w"), indent=0)
    print(n, "sprites")


if __name__ == "__main__":
    main()
