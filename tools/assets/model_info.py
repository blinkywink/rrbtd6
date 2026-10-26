"""Show a model type's distinct parameter sets and which tower tiers use it.

Usage: python -I tools/assets/model_info.py <Type> [<Type>...]
"""
import collections
import json
import sys

objs = {}
for line in open("local_extracted/decoded/named/objects.jsonl", encoding="utf-8"):
    r = json.loads(line)
    objs[r["i"]] = r
values = {int(k): v for k, v in json.load(open("local_extracted/decoded/values.json")).items()}
want = set(sys.argv[1:])
SKIP = {"childDependants", "mods", "upgrades", "appliedUpgrades", "paragonUpgrade", "targetTypes", "footprint", "tiers", "areaTypes"}


def refs(v):
    if isinstance(v, dict):
        if "$ref" in v:
            yield v["$ref"]
        elif "items" in v and isinstance(v["items"], list):
            for x in v["items"]:
                if isinstance(x, int):
                    yield x
        else:
            for x in v.values():
                yield from refs(x)


def walk(i, seen, root, path, hits):
    if i in seen:
        return
    seen.add(i)
    if i in values:
        for c in refs(values[i]):
            walk(c, seen, root, path, hits)
        return
    r = objs.get(i)
    if r is None or (r["type"] == "TowerModel" and i != root and not r["f"].get("isSubTower")):
        return
    if r["type"] in want:
        hits.append((r, path))
    for k, v in r["f"].items():
        if k not in SKIP:
            for c in refs(v):
                walk(c, seen, root, path + [k], hits)


params = collections.defaultdict(lambda: collections.defaultdict(set))
for r in objs.values():
    if r["type"] != "TowerModel" or r["f"].get("isSubTower"):
        continue
    hits = []
    walk(r["i"], set(), r["i"], [], hits)
    for h, path in hits:
        f = {k: (v if not isinstance(v, dict) else "ref") for k, v in h["f"].items() if k not in ("runtimeTypeIndex", "isTower", "childDependants")}
        params[h["type"]][json.dumps(f, sort_keys=True)].add(r["f"]["_name"] + "  @" + ".".join(path[-3:]))
for t, ps in params.items():
    print("==", t)
    for p, towers in ps.items():
        ts = sorted(towers)
        print("  ", p[:400])
        print("     ", len(ts), "tiers:", ", ".join(ts[:6]), "..." if len(ts) > 6 else "")
