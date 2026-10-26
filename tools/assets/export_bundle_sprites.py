"""Export named Sprites that live outside the sprite atlases (embedded in other bundles).

Usage:
    tools/.venv/Scripts/python.exe tools/assets/export_bundle_sprites.py <game install> <out_dir> <bundle glob> <SpriteName>...

Writes <out_dir>/<SpriteName>.png and merges [left, bottom, right, top, pixelsPerUnit] into
<out_dir>/_borders.json (same layout as export_sprites.py). Proprietary art: local use only.
"""
import glob
import json
import os
import sys

import UnityPy
from export_sprites import full_rect

GAME, OUT, PATTERN, *NAMES = sys.argv[1:]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
want = set(NAMES)
os.makedirs(OUT, exist_ok=True)
bpath = os.path.join(OUT, "_borders.json")
borders = json.load(open(bpath)) if os.path.exists(bpath) else {}
for bundle in sorted(glob.glob(os.path.join(AA, PATTERN))):
    if not want:
        break
    env = UnityPy.load(bundle)
    for o in env.objects:
        if o.type.name != "Sprite":
            continue
        s = o.read()
        if s.m_Name not in want:
            continue
        full_rect(s).save(os.path.join(OUT, s.m_Name + ".png"))
        b = s.m_Border
        borders[s.m_Name] = [b.x, b.y, b.z, b.w, s.m_PixelsToUnits]
        want.discard(s.m_Name)
        print(os.path.basename(bundle), s.m_Name, s.image.size)
json.dump(borders, open(bpath, "w"), indent=0)
if want:
    print("not found:", " ".join(sorted(want)))
