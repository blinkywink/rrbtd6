"""Map Addressables GUIDs to asset file names from catalog.bin via their locations (read-only).

INFERRED layout: a key string [u32 len=32][32 hex guid] is preceded 24 bytes (28 for Sprite records)
earlier by a u32 with flag 0x40000000 whose low bits are the offset of a
location record; that record's first u32 points at the length-prefixed internal
id ("Pop01.wav", "Moab3D.prefab"). Checked against all named SoundModels
(assetId -> name): 1058/1058. Covers keys catalog_guids.py misses (audio clips,
bloon displays and their damage states).

Usage: python -I catalog_audio.py <catalog.bin> <out.json> [extensions, default wav ogg mp3]
"""
import json
import re
import struct
import sys

d = open(sys.argv[1], "rb").read()
exts = "|".join(re.escape(e) for e in (sys.argv[3:] or ["wav", "ogg", "mp3"]))
name_re = re.compile(rb"[\x20-\x7e]+\.(" + exts.encode() + rb")")


def u(o):
    return struct.unpack_from("<I", d, o)[0] if 0 <= o <= len(d) - 4 else 0


def name_at(loc):
    s = u(loc)
    if not 4 <= s < len(d):
        return None
    n = u(s - 4)
    if not 0 < n < 300:
        return None
    nm = d[s:s + n]
    return nm.decode() if name_re.fullmatch(nm) else None


out = {}
for m in re.finditer(rb"\x20\x00\x00\x00([0-9a-f]{32})", d):
    p = m.start()
    for k in range(24, 32):
        v = u(p - k)
        if v & 0xC0000000 == 0x40000000:
            nm = name_at(v & 0x3FFFFFFF)
            if nm:
                out[m.group(1).decode()] = nm
                break
json.dump(out, open(sys.argv[2], "w"), indent=0)
print(len(out), "guid ->", exts)
