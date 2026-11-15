"""Repair maps whose repeated area names all got one outline from the old import_paths.py.

    python -I tools/mod/repair_dup_areas.py <fresh export dir> <dump dir> <MapName> [...]

<fresh export dir>/<key>/map.json is a fresh tools/assets/export_map.py export. Each local area
(local_extracted/maps/<key>/map.json) takes back the outline of the fresh area with its name and
its `blockers` samples (left alone by import_paths), keeping its own id; then
import_paths.py's patch_areas puts the game's dumped outlines and order on (<dump dir>/<Map>.map).
"""
import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
sys.path.insert(0, os.path.dirname(__file__))
import import_paths  # noqa: E402


def same(a, b):
    ba, bb = a.get("blockers") or [], b.get("blockers") or []
    return len(ba) == len(bb) and all(abs(x - y) < 1e-3 for p, q in zip(ba, bb) for x, y in zip(p, q))


def main():
    fresh_dir, dump_dir, names = sys.argv[1], sys.argv[2], sys.argv[3:]
    for name in names:
        key = name.lower()
        mp = os.path.join(ROOT, "local_extracted", "maps", key, "map.json")
        m = json.load(open(mp, encoding="utf-8"))
        fresh = json.load(open(os.path.join(fresh_dir, key, "map.json"), encoding="utf-8"))["areas"]
        used, missing = set(), 0
        for a in m["areas"]:
            k = next((i for i, f in enumerate(fresh) if i not in used and f.get("name") == a.get("name") and same(a, f)), None)
            if k is None:
                missing += 1
                continue
            used.add(k)
            a["points"], a["holes"] = fresh[k]["points"], fresh[k].get("holes", [])
        import_paths.patch_areas(m, import_paths.load_areas(os.path.join(dump_dir, name + ".map")))
        json.dump(m, open(mp, "w", encoding="utf-8"))
        print(name, "areas", len(m["areas"]), "unmatched", missing)


if __name__ == "__main__":
    main()
