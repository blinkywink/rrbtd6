"""Buff icons under the names towers ask for, from the game's BuffIconSprites table: a BuffIndicatorModel's
iconName is a buff id, and the table maps it to a sprite that can have another name
(BuffIconSkywardenRange -> BuffIconSkywarden1xx). Each one missing becomes a copy in
<root>/sprites/UiBuffIcons, where the renderer looks names up.

    python -I export_buff_icons.py <game dir> <root (local_extracted)>
"""
import glob
import os
import re
import shutil
import sys
from pathlib import Path

import UnityPy

GAME, ROOT = sys.argv[1], Path(sys.argv[2])
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")


def main():
    env = UnityPy.load(glob.glob(os.path.join(AA, "game_data_assets_all_*.bundle"))[0])
    table = None
    for o in env.objects:
        if o.type.name == "MonoBehaviour":
            t = o.read_typetree()
            if "buffIconSprites" in t:
                table = t["buffIconSprites"]
                break
    if table is None:
        raise SystemExit("no BuffIconSprites in game_data_assets_all")
    out = ROOT / "sprites/UiBuffIcons"
    added, missing = 0, []
    for entry in table:
        target = out / f"{entry['buffId']}.png"
        if target.exists():
            continue
        match = re.fullmatch(r"(\w+)\[(\w+)\]", entry["icon"].get("guidRef", ""))
        source = ROOT / "sprites" / match[1] / f"{match[2]}.png" if match else None
        if source is not None and source.exists():
            shutil.copyfile(source, target)
            added += 1
        else:
            missing.append(entry["buffId"])
    print(len(table), "buff icons;", added, "added; no sprite:", missing, flush=True)


main()
