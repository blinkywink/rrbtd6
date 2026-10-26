"""Export the 3D bloon displays game.json names (MOAB-class bodies and damage states).

Runs export_animated.py once per model into local_extracted/models/bloons/<model>; existing
exports are kept.

Usage: python -I export_bloon_models.py <game install>
"""
import json
import os
import subprocess
import sys

GAME = sys.argv[1]
OUT = "local_extracted/models/bloons"
DIR = "Assets/Bloons/Graphics/PrefabsMoabs3D"

g = json.load(open("local_extracted/game/game.json"))
models = sorted({m for b in g["bloons"].values() for m in [b.get("model")] + [d["model"] for d in b.get("damage_models", [])] if m})
paths = set(json.load(open("local_extracted/all_container_paths.json")))
for m in models:
    out = os.path.join(OUT, m)
    if os.path.exists(os.path.join(out, "meta.json")):
        continue
    prefab = f"{DIR}/{m}.prefab"
    if prefab not in paths:
        print("no prefab", prefab)
        continue
    r = subprocess.run([sys.executable, os.path.join(os.path.dirname(__file__), "export_animated.py"), GAME, out,
                        "asset_references_assets_all", prefab], capture_output=True, text=True)
    print(m, "ok" if r.returncode == 0 else "FAIL " + r.stderr.strip().splitlines()[-1], flush=True)
