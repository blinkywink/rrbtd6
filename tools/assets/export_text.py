"""Export TextAssets (e.g. the English localization table) by container path.

Usage:
    tools/.venv/Scripts/python.exe tools/assets/export_text.py <game install> <out_dir> <bundle glob> <container path>...

Writes <out_dir>/<file name> for each path. Proprietary data: local use only.
"""
import glob
import os
import sys

import UnityPy

GAME, OUT, BUNDLES, *PATHS = sys.argv[1:]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
env = UnityPy.load(*glob.glob(os.path.join(AA, BUNDLES)))
containers = dict(env.container.items())
os.makedirs(OUT, exist_ok=True)
for p in PATHS:
    obj = containers.get(p)
    if obj is None:
        print("not found", p)
        continue
    data = obj.read()
    script = data.m_Script
    raw = script.encode("utf-8", "surrogateescape") if isinstance(script, str) else bytes(script)
    out = os.path.join(OUT, os.path.basename(p))
    with open(out, "wb") as f:
        f.write(raw)
    print(p, "->", out, len(raw), "bytes")
