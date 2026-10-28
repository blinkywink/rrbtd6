"""Index the game's asset bundles by their internal CAB file names, so exporters can load the
bundles a prefab's materials / textures / shaders live in (PPtr m_FileID -> externals
"archive:/CAB-.../CAB-..." -> bundle file).

    tools/.venv/Scripts/python.exe -I tools/assets/cab_index.py <game install>

Writes local_extracted/cab_index.json {cab name (lower case): bundle file name}. Reads only each
bundle's directory (UnityPy loads the file table lazily), so it is quick.
"""
import glob
import json
import os
import sys

import UnityPy

GAME = sys.argv[1]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
OUT = os.path.join(os.path.dirname(__file__), "..", "..", "local_extracted", "cab_index.json")

index = {}
for f in sorted(glob.glob(os.path.join(AA, "*.bundle"))):
    try:
        env = UnityPy.load(f)
    except Exception as e:  # a broken bundle must not stop the index
        print("skip", os.path.basename(f), type(e).__name__, flush=True)
        continue
    for bundle in env.files.values():
        for name in getattr(bundle, "files", {}) or {}:
            if name.lower().startswith("cab-") and not name.lower().endswith((".ress", ".resource", ".ress")):
                index[name.lower()] = os.path.basename(f)
    del env
json.dump(index, open(OUT, "w"), indent=0)
print(len(index), "cab files indexed")
