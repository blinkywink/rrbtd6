"""Print a tower tier's whole model tree (every behavior, emission, filter, projectile and
field), to check a tower against the game model field by field.

Usage: python -I tools/assets/dump_tower.py <TowerId e.g. DartMonkey-004> [--out dir]
With --all <BaseId> it writes one file per tier of that tower into --out (default
research/dumps/towers/<BaseId>/).
"""
import json
import os
import sys

objs = {}
for line in open("local_extracted/decoded/named/objects.jsonl", encoding="utf-8"):
    r = json.loads(line)
    objs[r["i"]] = r
values = {int(k): v for k, v in json.load(open("local_extracted/decoded/values.json")).items()}

# Fields that point outside the tower or only carry bookkeeping.
SKIP = {"childDependants", "mods", "upgrades", "appliedUpgrades", "paragonUpgrade", "targetTypes",
        "footprint", "areaTypes", "runtimeTypeIndex", "_name", "isTower"}


def show(v, depth, seen, out, key):
    pad = "  " * depth
    if isinstance(v, dict) and "$ref" in v:
        node(v["$ref"], depth, seen, out, key)
    elif isinstance(v, dict) and isinstance(v.get("items"), list):
        if not v["items"]:
            out.append(f"{pad}{key}: []")
            return
        out.append(f"{pad}{key}:")
        for n, x in enumerate(v["items"]):
            if isinstance(x, int):
                node(x, depth + 1, seen, out, f"[{n}]")
            else:
                show(x, depth + 1, seen, out, f"[{n}]")
    else:
        out.append(f"{pad}{key}: {json.dumps(v)}")


def node(i, depth, seen, out, key):
    pad = "  " * depth
    if i in values:
        show(values[i], depth, seen, out, key)
        return
    r = objs.get(i)
    if r is None:
        out.append(f"{pad}{key}: <{i}?>")
        return
    if r["type"] == "TowerModel" and depth > 0 and not r["f"].get("isSubTower"):
        out.append(f"{pad}{key}: TowerModel {r["f"].get("_name")} (not expanded)")
        return
    if i in seen:
        out.append(f"{pad}{key}: {r['type']} #{i} (see above)")
        return
    seen.add(i)
    out.append(f"{pad}{key}: {r['type']} #{i}")
    for k, v in r["f"].items():
        if k not in SKIP:
            show(v, depth + 1, seen, out, k)


def tower(name):
    for r in objs.values():
        if r["type"] == "TowerModel" and r["f"].get("_name") == name:
            return r["i"]
    raise SystemExit(f"no TowerModel {name}")


if "--all" in sys.argv:
    base = sys.argv[sys.argv.index("--all") + 1]
    out_dir = sys.argv[sys.argv.index("--out") + 1] if "--out" in sys.argv else f"research/dumps/towers/{base}"
    os.makedirs(out_dir, exist_ok=True)
    names = sorted(r["f"]["_name"] for r in objs.values()
                   if r["type"] == "TowerModel" and r["f"].get("baseId") == base and not r["f"].get("isSubTower"))
    for n in names:
        out = []
        node(tower(n), 0, set(), out, n)
        open(os.path.join(out_dir, n + ".txt"), "w", encoding="utf-8").write("\n".join(out) + "\n")
    print(len(names), "tiers ->", out_dir)
else:
    out = []
    node(tower(sys.argv[1]), 0, set(), out, sys.argv[1])
    print("\n".join(out))
