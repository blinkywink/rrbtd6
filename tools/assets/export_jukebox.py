"""Export the game's jukebox (AudioJukeBox.musicTrackData, referenced by Assets/Data/GameData.asset)
to local_extracted/ui/runtime/jukebox.json: every music track in the game's order with its loc key,
whether it's free (the rest are Trophy Store items) and whether the jukebox lists it.

Usage: python -I tools/assets/export_jukebox.py <game install>
"""
import glob
import json
import os
import sys

import UnityPy
from UnityPy.helpers import TypeTreeHelper

TypeTreeHelper.read_typetree_boost = False
GAME = sys.argv[1]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
bundle = json.load(open("local_extracted/all_container_paths.json"))["Assets/Data/GameData.asset"].rsplit("_", 1)[0]
env = UnityPy.load(*(glob.glob(os.path.join(AA, bundle + "_*.bundle")) + glob.glob(os.path.join(AA, "*_monoscripts_*.bundle"))))

tracks = []
for o in env.objects:
    if o.type.name != "MonoBehaviour":
        continue
    try:
        t = o.read_typetree()
    except Exception:
        continue
    if "musicTrackData" not in t:
        continue
    for ref in t["musicTrackData"]:
        m = o.assets_file.objects[ref["m_PathID"]].read_typetree()
        tracks.append({"id": m["id"], "loc": m["locKey"], "free": bool(m["freeTrack"]),
                       "jukebox": not m["unavailableInJukeBox"]})
    break
os.makedirs("local_extracted/ui/runtime", exist_ok=True)
json.dump(tracks, open("local_extracted/ui/runtime/jukebox.json", "w"), indent=1)
print(len(tracks), "tracks,", sum(t["free"] and t["jukebox"] for t in tracks), "free in the jukebox")
