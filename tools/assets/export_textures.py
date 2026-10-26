"""Export individual textures/sprites by Addressables container path.

Usage:
    tools/.venv/Scripts/python.exe tools/assets/export_textures.py <game install> <out_dir> <container path>...

Bundles are looked up in local_extracted/all_container_paths.json. Each asset is
written to <out_dir>/<file stem>.png. Proprietary art: local use only.
"""
import json
import os
import sys
from collections import defaultdict

import UnityPy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from export_sprites import full_rect  # noqa: E402

# Asset names can hold non-ASCII letters ("Pouākai"); print them whatever the console's code page.
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
GAME, OUT, *PATHS = sys.argv[1:]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
index = json.load(open("local_extracted/all_container_paths.json"))
by_bundle = defaultdict(list)
for p in PATHS:
    if p not in index:
        sys.exit(f"not in container index: {p}")
    by_bundle[index[p]].append(p)
os.makedirs(OUT, exist_ok=True)
for bundle, paths in by_bundle.items():
    env = UnityPy.load(os.path.join(AA, bundle))
    containers = dict(env.container.items())
    for p in paths:
        obj = containers[p].read()
        name = os.path.splitext(os.path.basename(p))[0]
        # Sprites keep their transparent margins: uGUI lays them out over the full m_Rect.
        image = full_rect(obj) if hasattr(obj, "m_Rect") else obj.image
        image.save(os.path.join(OUT, name + ".png"))
        print(p, "->", name + ".png", image.size)
