"""GameData.bloonOverlayData (BloonOverlayData.overlayTypes): each mutator overlayType's
BloonOverlayScriptable, its displayLayer and its prefab per BloonOverlayClass. Written into
game.json's "overlays" ({type: {"layer": n, "assets": {class: prefab stem}}}); build_game_data.py
keeps it. The names don't always follow "<class><type>" (AlchemistGold -> RedRubberToGold).

    tools/.venv/Scripts/python.exe -I tools/assets/export_overlays.py

Proprietary data: local use only.
"""
import glob
import json
import os
from pathlib import Path

import UnityPy

GAME = r"C:\Program Files (x86)\Steam\steamapps\common\BloonsTD6"
L = Path("local_extracted")
# Il2Cpp.BloonOverlayClass in order (Invalid is -1); same list as build_game_data.py.
CLASSES = ['Red', 'Blue', 'Green', 'Yellow', 'Pink', 'White', 'RedRegrow', 'BlueRegrow', 'GreenRegrow', 'YellowRegrow',
           'PinkRegrow', 'WhiteRegrow', 'Moab', 'Bfb', 'Zomg', 'Ddt', 'Bad', 'Bloonarius', 'Lych', 'MiniLych', 'Vortex',
           'Dreadbloon', 'Blastapopoulos', 'Wild']


def main():
    bundle = glob.glob(os.path.join(GAME, r"BloonsTD6_Data\StreamingAssets\aa\*\Full\game_data_assets_all_*.bundle"))[0]
    prefabs = json.loads((L / "catalog_prefabs.json").read_text(encoding="utf-8"))
    env = UnityPy.load(bundle)
    by_id, table = {}, None
    for o in env.objects:
        if o.type.name != "MonoBehaviour":
            continue
        try:
            t = o.read_typetree()
        except Exception:
            continue
        if "overlayTypes" in t:
            table = t["overlayTypes"]
        elif "assets" in t and "displayLayer" in t:
            by_id[o.path_id] = t
    out = {}
    for key, ref in zip(table["keys"], table["values"]):
        s = by_id.get(ref["m_PathID"])
        if s is None:
            continue
        assets = {}
        for c, v in zip(s["assets"]["keys"], s["assets"]["values"]):
            p = prefabs.get(v.get("guidRef") or "")
            if 0 <= c < len(CLASSES) and p:
                assets[CLASSES[c]] = p.rsplit("/", 1)[-1].removesuffix(".prefab")
        out[key] = {"layer": s["displayLayer"], "assets": assets}
    gp = L / "game/game.json"
    game = json.loads(gp.read_text(encoding="utf-8"))
    game["overlays"] = out
    gp.write_text(json.dumps(game, indent=1), encoding="utf-8")
    print(len(out), "overlay types;", sum(len(v["assets"]) for v in out.values()), "prefabs")


if __name__ == "__main__":
    main()
