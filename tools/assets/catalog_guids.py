"""Map Addressables GUIDs to asset file names from catalog.bin (read-only).

The binary catalog stores records as [u32 len=32][32 hex guid][u32 len][name...]
for keyed assets; this extracts every guid that is directly followed by a
length-prefixed asset name.

Usage: python -I catalog_guids.py <catalog.bin> <out.json>
"""
import json
import re
import struct
import sys

d = open(sys.argv[1], "rb").read()
out = {}
for m in re.finditer(rb"\x20\x00\x00\x00([0-9a-f]{32})", d):
    p = m.end()
    if p + 4 > len(d):
        continue
    n = struct.unpack_from("<I", d, p)[0]
    if 0 < n < 200:
        name = d[p + 4:p + 4 + n]
        if re.fullmatch(rb"[\x20-\x7e]+\.(prefab|png|asset|mat|anim|controller|fbx|bytes)", name):
            out[m.group(1).decode()] = name.decode()
json.dump(out, open(sys.argv[2], "w"), indent=0)
print(len(out), "guid -> name")
