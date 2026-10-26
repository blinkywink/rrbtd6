"""Export a TextMeshPro SDF font asset (atlas + glyph table).

Usage:
    tools/.venv/Scripts/python.exe tools/assets/export_font.py <game install> <bundle glob> <asset name> <family> <out_dir>
    e.g. ... localization_assets_all Btd6FontTitleSDF "Bloons TD" local_extracted/fonts/title

Writes <out_dir>/atlas.png (SDF in alpha) and <out_dir>/font.json:
{point_size, line_height, ascent, padding, atlas: [w, h],
 glyphs: {codepoint: [x, y_top, w, h, bearing_x, bearing_y, advance]}} (atlas pixels / font units).
Proprietary art: local use only.
"""
import glob
import json
import os
import sys

import UnityPy
from UnityPy.helpers import TypeTreeHelper

TypeTreeHelper.read_typetree_boost = False

GAME, BUNDLE, NAME, FAMILY, OUT = sys.argv[1:6]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
env = UnityPy.load(glob.glob(os.path.join(AA, BUNDLE + "*.bundle"))[0])
for o in env.objects:
    if o.type.name != "MonoBehaviour":
        continue
    try:
        t = o.read_typetree()
    except Exception:
        continue
    if t.get("m_Name") == NAME and t.get("m_FaceInfo", {}).get("m_FamilyName") == FAMILY:
        break
else:
    sys.exit(f"{NAME} ({FAMILY}) not found")

os.makedirs(OUT, exist_ok=True)
tex_ref = t["m_AtlasTextures"][0]
tex = o.assets_file.objects[tex_ref["m_PathID"]].read()
img = tex.image
aw, ah = t["m_AtlasWidth"], t["m_AtlasHeight"]
img.convert("RGBA").save(os.path.join(OUT, "atlas.png"))
glyph_by_index = {g["m_Index"]: g for g in t["m_GlyphTable"]}
glyphs = {}
for c in t["m_CharacterTable"]:
    g = glyph_by_index.get(c["m_GlyphIndex"])
    if g is None:
        continue
    r, m = g["m_GlyphRect"], g["m_Metrics"]
    glyphs[c["m_Unicode"]] = [r["m_X"], ah - r["m_Y"] - r["m_Height"], r["m_Width"], r["m_Height"],
                              m["m_HorizontalBearingX"], m["m_HorizontalBearingY"], m["m_HorizontalAdvance"]]
fi = t["m_FaceInfo"]
json.dump({"family": FAMILY, "point_size": fi["m_PointSize"], "line_height": fi["m_LineHeight"],
           "ascent": fi["m_AscentLine"], "descent": fi["m_DescentLine"], "padding": t["m_AtlasPadding"], "atlas": [aw, ah], "glyphs": glyphs},
          open(os.path.join(OUT, "font.json"), "w"))
print(NAME, FAMILY, len(glyphs), "glyphs, atlas", aw, ah, "padding", t["m_AtlasPadding"], "mode", img.mode)
