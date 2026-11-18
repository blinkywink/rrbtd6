"""Every prefab the game's tower models reference vs what the clone's data carries.

For each TowerModel (base towers' tiers and paragons) in the decoded game model, collects every
GUID reachable from it that names a prefab (catalog_prefabs.json), with the (model type, field)
that holds it. A GUID is carried when the clone's tier in local_extracted/game/game.json (or the
tier's display plan) holds that GUID or the prefab's name anywhere. Not carried = the game shows
something there that the clone never draws. Grouped by (model type, field), so whole behaviours
the data builder ignores stand out.

  python -I tools/assets/audit_dropped_displays.py [--tower BombShooter] [--examples N]
"""
import json
import os
import re
import sys
from collections import defaultdict

L = "local_extracted"
args = sys.argv[1:]
only = args[args.index("--tower") + 1] if "--tower" in args else None
nex = int(args[args.index("--examples") + 1]) if "--examples" in args else 3

prefabs = json.load(open(f"{L}/catalog_prefabs.json", encoding="utf-8"))
pname = lambda g: os.path.splitext(os.path.basename(prefabs[g]))[0].strip()
game = json.load(open(f"{L}/game/game.json", encoding="utf-8"))

values = {int(k): v for k, v in json.load(open(f"{L}/decoded/values.json")).items()}
objs = {}
for line in open(f"{L}/decoded/named/objects.jsonl", encoding="utf-8"):
    r = json.loads(line)
    objs[r["i"]] = r

# Fields that are menus / UI, not the battlefield.
UI = {"icon", "icon3D", "portrait", "instaIcon", "iconSquare", "iconPath", "emoteSpriteSmall",
      "emoteSpriteLarge", "upgradeIcon", "cardIcon", "buttonIcon"}


HOLDERS = {"EffectModel", "DisplayModel", "AssetPathModel"}
SKIP_TYPES = {"UpgradeModel", "UpgradePathModel"}
SKIP_FIELDS = {"childDependants", "upgrades", "appliedUpgrades", "paragonUpgrade"}
# Effects the clone plays for every tower without per-tier data (placing, selling, upgrading).
GENERIC = {"SellEffect", "PlacementEffect", "UpgradeEffect", "PlacementEffectWater", "SellEffectWater"}


def guids(o, owner, field, seen, out):
    """(guid, owner type, field) for every prefab GUID under object o."""
    if isinstance(o, dict):
        if "$ref" in o:
            r = objs.get(o["$ref"])
            if r is None and o["$ref"] in values and o["$ref"] not in seen:
                seen.add(o["$ref"])
                c = values[o["$ref"]]
                for it in (c.get("items") or c.get("values") or []) if isinstance(c, dict) else []:
                    guids({"$ref": it} if isinstance(it, int) and (it in objs or it in values) else it, owner, field, seen, out)
                for k, v in (c.items() if isinstance(c, dict) else []):
                    if k not in ("items", "values", "_container", "len"):
                        guids(v, owner, field, seen, out)
                return
            if r is not None and r["i"] not in seen:
                seen.add(r["i"])
                # Other towers (upgrade links, powers, transforms) aren't this tier's; a sub-tower
                # (pets, sentries) is.
                if r["type"] in SKIP_TYPES or (r["type"] == "TowerModel" and len(seen) > 1 and not r["f"].get("isSubTower")):
                    return
                # A shared holder (EffectModel, DisplayModel...) is named by its parent behaviour.
                label = f"{owner}>{r['type']}" if r["type"] in HOLDERS else r["type"]
                for k, v in r["f"].items():
                    if k in UI or k in SKIP_FIELDS:
                        continue
                    guids(v, label, k, seen, out)
            return
        if "guidRef" in o:
            guids(o["guidRef"], owner, field, seen, out)
            return
        for k, v in o.items():
            guids(v, owner, field if k in ("items", "_items") else k, seen, out)
    elif isinstance(o, list):
        for v in o:
            guids(v, owner, field, seen, out)
    elif isinstance(o, str) and o in prefabs:
        out.append((o, owner, field))


def strings(o, out):
    if isinstance(o, dict):
        for v in o.values():
            strings(v, out)
    elif isinstance(o, list):
        for v in o:
            strings(v, out)
    elif isinstance(o, str):
        out.add(o)
        out.add(o.strip().lower())
        if o.endswith(".prefab"):
            out.add(os.path.splitext(os.path.basename(o))[0].strip().lower())


def tier_key(tiers):
    return "".join(str(t) for t in tiers)


missing = defaultdict(list)  # (owner, field) -> [(tower tier, prefab)]
carried = defaultdict(int)
checked = 0
for r in objs.values():
    if r["type"] != "TowerModel" or not r["f"].get("isTower") or r["f"].get("isSubTower"):
        continue
    f = r["f"]
    base = f.get("baseId")
    if base not in game["tower_tiers"] or (only and base != only):
        continue
    tt = game["tower_tiers"][base]
    nm = f.get("_name") or ""
    if f.get("isParagon"):
        spec, key = tt.get("paragon"), "paragon"
        spec = spec.get("tier", spec) if isinstance(spec, dict) else spec
    else:
        key = nm.split("-", 1)[1] if "-" in nm else "000"
        if not re.fullmatch(r"[0-5]{3}", key):
            continue
        spec = (tt.get("tiers") or {}).get(key)
    if spec is None:
        continue
    have = set()
    strings(spec, have)
    plan = f"{L}/display_plans/{base}.json"
    if os.path.exists(plan):
        p = json.load(open(plan, encoding="utf-8")).get("towers", {}).get(key)
        if p:
            strings(p, have)
            have.add(os.path.splitext(os.path.basename(p["prefab"]))[0].strip().lower())
    out = []
    guids({"$ref": r["i"]}, "TowerModel", "", set(), out)
    # The tier's own body (its display, also as its DisplayModel) is the display plan's model.
    have.add(f.get("display"))
    for g, owner, field in {(g, o, fl) for g, o, fl in out}:
        checked += 1
        name = pname(g)
        if g in have or name.lower() in have or name in have or name in GENERIC:
            carried[(owner, field)] += 1
        else:
            missing[(owner, field)].append((f"{base} {key}", name))

print(checked, "tier/prefab references;", sum(len(v) for v in missing.values()), "not carried into the clone's data")
for (owner, field), v in sorted(missing.items(), key=lambda kv: -len(kv[1])):
    names = sorted({n for _, n in v})
    towers = sorted({t.split()[0] for t, _ in v})
    print(f"{len(v):5}  {owner}.{field}  (carried elsewhere: {carried.get((owner, field), 0)})  towers: {', '.join(towers[:8])}{' ...' if len(towers) > 8 else ''}")
    print("       e.g. " + "; ".join(f"{t}: {n}" for t, n in v[:nex]) + f"   [{len(names)} distinct: {', '.join(names[:12])}]")
