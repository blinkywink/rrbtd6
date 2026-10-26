"""Export round sets and income sets from the game_data bundle to JSON.

Usage: tools/.venv/Scripts/python.exe tools/assets/export_rounds.py <game install> <out_dir>
Output is proprietary game data; keep it in local_extracted/.
"""
import glob
import json
import os
import sys

import UnityPy
from UnityPy.helpers import TypeTreeHelper

TypeTreeHelper.read_typetree_boost = False  # native parser segfaults on this build


def main():
    game, out = sys.argv[1:3]
    aa = os.path.join(game, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
    env = UnityPy.load(glob.glob(os.path.join(aa, "game_data_assets_all_*.bundle"))[0])
    os.makedirs(out, exist_ok=True)
    rounds, income, modes = {}, {}, {}
    for o in env.objects:
        if o.type.name != "MonoBehaviour":
            continue
        t = o.read_typetree()
        if "rounds" in t:
            rounds[t["m_Name"]] = {k: v for k, v in t.items() if k not in ("m_GameObject", "m_Script", "m_Enabled")}
        elif "mutatorMods" in t:
            refs = {r["rid"]: {"class": r["type"]["class"], **r["data"]} for r in t["references"]["RefIds"]}
            modes[t["m_Name"]] = [refs[m["rid"]] for m in t["mutatorMods"]]
        elif "thresholds" in t:
            income[t["m_Name"]] = {k: v for k, v in t.items() if k not in ("m_GameObject", "m_Script", "m_Enabled")}
    json.dump(rounds, open(os.path.join(out, "round_sets.json"), "w"), indent=0)
    json.dump(income, open(os.path.join(out, "income_sets.json"), "w"), indent=0)
    json.dump(modes, open(os.path.join(out, "modes.json"), "w"), indent=1)
    print(len(rounds), "round sets;", len(income), "income sets;", sorted(modes))


if __name__ == "__main__":
    main()
