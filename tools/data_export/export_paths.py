"""Export raw map path entries (from tools/assets/export_map.py) to a TSV
that btd6-sim's mapbuild::parse_raw_path reads.

One line per child of the Unity `Path` object, in child order:
    L  x y z              LineNode (plain transform)
    C  x y z  x y z  x y z  x y z   Curve (BezierLineNodes control points)
Coordinates are Unity world space, as f32 hex bits.

    python -I tools/data_export/export_paths.py [map_key ...]   (default: all)
"""
import json
import os
import struct
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
MAPS = os.path.join(ROOT, "local_extracted", "maps")
OUT = os.path.join(ROOT, "local_extracted", "engine_data", "paths")


def h(v):
    return "0x%08X" % struct.unpack("<I", struct.pack("<f", v))[0]


def export(key):
    m = json.load(open(os.path.join(MAPS, key, "map.json"), encoding="utf-8"))
    raw = m["raw_path"]
    os.makedirs(OUT, exist_ok=True)
    p = os.path.join(OUT, key + ".tsv")
    with open(p, "w", encoding="utf-8", newline="\n") as f:
        f.write(f"# raw path for {key}. Unity world coords, f32 bits. Proprietary: local use only.\n")
        for e in raw:
            kind = {"LineNode": "L", "Curve": "C"}[e["kind"]]
            pts = e["world"]
            if (kind == "L") != (len(pts) == 1):
                raise SystemExit(f"{key}: unexpected point count in {e['name']}")
            f.write("\t".join([kind] + [h(c) for pt in pts for c in pt]) + "\n")
    print(key, len(raw), "entries ->", p)


def main():
    keys = sys.argv[1:] or [d for d in os.listdir(MAPS) if os.path.exists(os.path.join(MAPS, d, "map.json"))]
    for k in keys:
        export(k)


if __name__ == "__main__":
    main()
