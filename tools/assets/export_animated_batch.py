"""Run export_animated.py over many prefabs, loading each bundle once.

Usage: python -I tools/assets/export_animated_batch.py <game install> <jobs.json>
jobs.json: [[out_dir, bundle_glob, prefab container path], ...]. A job whose out_dir already
has meta.json is skipped. Each output is compacted (compact_anims.py). Run in chunks (a fresh
process every few dozen prefabs): UnityPy keeps what it reads.
"""
import json
import os
import sys
import traceback

GAME, JOBS = sys.argv[1], sys.argv[2]
jobs = [j for j in json.load(open(JOBS)) if not os.path.exists(os.path.join(j[0], "meta.json"))]
if not jobs:
    sys.exit(0)
here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
sys.argv = [sys.argv[0], GAME, jobs[0][0], jobs[0][1], jobs[0][2]]
import export_animated as ea  # noqa: E402
import compact_anims as ca  # noqa: E402

for out, bundle, prefab in jobs:
    ea.OUT, ea.BUNDLE, ea.PREFAB = out, bundle, prefab
    ea.em.OUT = out
    ea.em.tex_cache.clear()
    try:
        ea.main()
        ca.convert(out)
        print("ok", out, flush=True)
    except Exception:
        print("FAIL", out, traceback.format_exc().strip().splitlines()[-1][:200], flush=True)
