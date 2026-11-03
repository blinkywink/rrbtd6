"""Export the 3D bloon displays game.json names (MOAB-class bodies and damage states).

Batches export_animated.py (model_batch.py: one load of the large bundle per group) into
local_extracted/models/bloons/<model>; existing exports are kept.

Usage: python -I export_bloon_models.py <game install> [out dir]
"""
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from model_batch import run

GAME = sys.argv[1]
OUT = sys.argv[2] if len(sys.argv) > 2 else "local_extracted/models/bloons"
DIR = "Assets/Bloons/Graphics/PrefabsMoabs3D"

g = json.load(open("local_extracted/game/game.json"))
models = sorted({m for b in g["bloons"].values() for m in [b.get("model")] + [d["model"] for d in b.get("damage_models", [])] if m})
paths = set(json.load(open("local_extracted/all_container_paths.json")))
jobs = []
for m in models:
    prefab = f"{DIR}/{m}.prefab"
    if prefab not in paths:
        print("no prefab", prefab)
        continue
    jobs.append([os.path.join(OUT, m), "asset_references_assets_all", prefab])
run(GAME, jobs)
