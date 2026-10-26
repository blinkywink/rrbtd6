"""List the model types (behaviours, emissions, filters...) the game's towers use that
build_game_data.py never mentions, i.e. that our export, and so the sim, ignores.

Usage: python -I tools/assets/audit_behaviors.py [--heroes] [--tower Name]
Prints each unhandled type with how many tower tiers use it and which towers.
"""
import collections
import glob
import json
import re
import sys

objs = {}
for line in open("local_extracted/decoded/named/objects.jsonl", encoding="utf-8"):
    r = json.loads(line)
    objs[r["i"]] = r
values = {int(k): v for k, v in json.load(open("local_extracted/decoded/values.json")).items()}
src = open("tools/assets/build_game_data.py", encoding="utf-8").read()
src += "".join(open(f, encoding="utf-8").read() for f in glob.glob("renderer/src/**/*.rs", recursive=True))
handled = set(re.findall(r"[A-Z][A-Za-z0-9_]*Model\b", src)) | set(re.findall(r'"([A-Z][A-Za-z0-9_]+)"', src))


def is_handled(t):
    """Named in the exporter or the sim, by full name or by its core (CreateEffectOnExpire...)."""
    core = t.split("_")[-1]
    core = core[:-5] if core.endswith("Model") else core
    return t in handled or core in src
# Types that only carry display / bookkeeping data, never gameplay.
IGNORE = {"TowerModel", "UpgradePathModel", "AttackModel", "WeaponModel", "ProjectileModel", "SoundModel",
          "EffectModel", "AssetPathModel", "CircleFootprintModel", "RectangleFootprintModel", "FootprintModel",
          "TargetTypeModel", "ApplyModModel", "TowerSetModel", "DisplayModel", "AreaTypeModel"}
heroes = "--heroes" in sys.argv
only = sys.argv[sys.argv.index("--tower") + 1] if "--tower" in sys.argv else None
HEROES = {"Quincy", "Gwendolin", "StrikerJones", "ObynGreenfoot", "CaptainChurchill", "Benjamin", "Ezili", "PatFusty",
          "Adora", "AdmiralBrickell", "Etienne", "Sauda", "Psi", "Geraldo", "Corvus", "Rosalia", "Silas", "Gwendolin",
          "Beast", "DanDMonke"}
# Powers / hero summons / Legends units: not shop towers.
POWERS = {"BananaFarmer", "BananaFarmerPro", "MonkeyBoostPro", "PortableLake", "PortableLakePro", "Pontoon", "TechBot",
          "TechBotPrime", "SuperMonkeyBeacon", "EnergisingTotem", "SpookyCreature", "SheRa", "Skeletor", "SwordOfPower",
          "BattleCat", "CaveMonkey", "Sheriff"}


def refs(v):
    """Object references in a field: {"$ref": i}, or the ids of an {"items": [...]} list
    (plain lists hold values, not references)."""
    if isinstance(v, dict):
        if "$ref" in v:
            yield v["$ref"]
        elif "items" in v and isinstance(v["items"], list):
            for x in v["items"]:
                if isinstance(x, int):
                    yield x
                else:
                    yield from refs(x)
        else:
            for x in v.values():
                yield from refs(x)
    elif isinstance(v, list):
        for x in v:
            if not isinstance(x, int):
                yield from refs(x)


# Fields that point outside the tower (other towers' upgrades, global mods, cosmetics).
SKIP_FIELDS = {"childDependants", "mods", "upgrades", "appliedUpgrades", "paragonUpgrade", "targetTypes",
               "footprint", "tiers", "areaTypes"}


def walk(i, seen, out, root):
    if i in seen:
        return
    seen.add(i)
    if i in values:
        for c in refs(values[i]):
            walk(c, seen, out, root)
        return
    r = objs.get(i)
    if r is None:
        return
    if r["type"] == "TowerModel" and i != root and not r["f"].get("isSubTower"):
        return
    out.add(r["type"])
    for k, v in r["f"].items():
        if k in SKIP_FIELDS:
            continue
        for c in refs(v):
            walk(c, seen, out, root)


use = collections.defaultdict(set)
count = collections.Counter()
for r in objs.values():
    if r["type"] != "TowerModel" or r["f"].get("isSubTower"):
        continue
    base = r["f"].get("baseId") or ""
    if not base or (base in HEROES) != heroes or (only and base != only):
        continue
    if (r["f"].get("towerSet", 0) == 0 and not heroes) or base in POWERS:
        continue
    types = set()
    walk(r["i"], set(), types, r["i"])
    for t in types:
        if not is_handled(t) and t not in IGNORE:
            use[t].add(base)
            count[t] += 1
if "--by-tower" in sys.argv:
    per = collections.defaultdict(list)
    for t, bases in use.items():
        for b in bases:
            per[b].append(t)
    for b in sorted(per):
        print(f"{b}: {', '.join(sorted(per[b]))}")
else:
    for t, n in count.most_common():
        print(f"{n:4d} {t}: {', '.join(sorted(use[t]))}")
