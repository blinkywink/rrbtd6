"""Export GameData's map list (MapSet: id, difficulty tier, music) for the map select screen.

Usage: tools/.venv/Scripts/python.exe tools/assets/export_mapset.py <game install> <out.json>
Writes [{"id", "difficulty" (0 beginner .. 3 expert), "music"}, ...] in the game's order,
leaving out debug and browser-only maps. Proprietary data: local use only.
"""
import glob
import json
import os
import sys

import UnityPy

GAME, OUT = sys.argv[1:3]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
idx = json.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "local_extracted", "all_container_paths.json")))
path = "Assets/Data/GameData.asset"
env = UnityPy.load(os.path.join(AA, idx[path]), *glob.glob(os.path.join(AA, "*_monoscripts_*.bundle")))
obj = env.container[path]
obj = obj.deref() if hasattr(obj, "deref") else obj
gd = obj.read_typetree()
ms = obj.assets_file.objects[gd["mapSet"]["m_PathID"]].read_typetree()
maps = [{"id": m["id"], "difficulty": m["difficulty"], "music": m.get("mapMusic")}
        for m in ms["Maps"]["items"] if not m.get("isDebug") and not m.get("isBrowserOnly")]
# Map select image (MapImages atlas). Most are MapSelect<Id>Button / <Id>MapButton; a few names
# differ from the id (the mapSprite reference isn't resolvable offline).
ALIAS = {"Tutorial": "MonkeyMeadow", "SkullTweak": "SkullPeak", "QuietStreet": "QuietSteet", "Cornfield": "Cornfields"}
imgs = {f[:-4].lower(): f[:-4] for f in os.listdir(os.path.join(os.path.dirname(OUT), "..", "sprites", "MapImages")) if f.endswith(".png")}
for m in maps:
    n = ALIAS.get(m["id"], m["id"])
    m["sprite"] = next((imgs[k.lower()] for k in (f"MapSelect{n}Button", f"MapSelect{n}MapButton", f"MapSelect{n}") if k.lower() in imgs), None)
json.dump(maps, open(OUT, "w"), indent=1)
print("without image:", [m["id"] for m in maps if not m["sprite"]])
print(len(maps), "maps")
