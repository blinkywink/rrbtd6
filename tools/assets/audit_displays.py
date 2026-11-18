"""Which display every tower's projectiles show in the game, and what the clone draws instead.

For each tower tier in local_extracted/game/game.json, walks every projectile (sub-projectiles,
exhausts, contacts, emits...) and resolves its display GUID the way the game does (catalog_prefabs:
GUID -> prefab) and the way the renderer does (models/projectiles/index.json: GUID -> folder,
then mod_assets::projectile loads models/projectiles/<folder>). Reports:

  MISSING   GUID has no index entry: the renderer falls back to its stand-in dart (or nothing)
  NOFILES   the indexed folder has nothing the loader can read
  RENAMED   the indexed folder's name isn't the game prefab's name (a wrong asset)
  FXONLY    no model, but a particle effect plays for it (effects/proj_index.json): fine

  python -I tools/assets/audit_displays.py [--all]   (default: one line per distinct problem)
"""
import json
import os
import sys
from collections import defaultdict

L = "local_extracted"
game = json.load(open(f"{L}/game/game.json", encoding="utf-8"))
prefabs = json.load(open(f"{L}/catalog_prefabs.json", encoding="utf-8"))
pindex = json.load(open(f"{L}/models/projectiles/index.json", encoding="utf-8"))
fxindex = {}
if os.path.exists(f"{L}/effects/proj_index.json"):
    fxindex = json.load(open(f"{L}/effects/proj_index.json", encoding="utf-8"))


def prefab_name(guid):
    p = prefabs.get(guid)
    return os.path.splitext(os.path.basename(p))[0].strip() if p else None


def loadable(folder):
    d = f"{L}/models/projectiles/{folder}"
    if not os.path.isdir(d):
        return False
    return any(os.path.exists(os.path.join(d, f)) for f in ("meta.json", "layers.json", "sprite.png", "mesh.json", "model.json")) or bool(os.listdir(d))


def projectiles(o, path):
    """Every dict that looks like a projectile spec (has a 'display' key next to 'pierce'), with its path."""
    if isinstance(o, dict):
        if "display" in o and "pierce" in o:
            yield path, o
        for k, v in o.items():
            yield from projectiles(v, path + "." + k)
    elif isinstance(o, list):
        for i, v in enumerate(o):
            yield from projectiles(v, f"{path}[{i}]")


problems = defaultdict(list)  # (kind, guid, detail) -> [tower tier path]
total = 0
for tower, tt in game["tower_tiers"].items():
    tiers = dict(tt.get("tiers") or {})
    if tt.get("paragon"):
        tiers["paragon"] = tt["paragon"].get("tier", tt["paragon"])
    for tier, spec in tiers.items():
        for path, pr in projectiles(spec, ""):
            g = pr.get("display")
            if not g or g == "#empty":
                continue
            total += 1
            want = prefab_name(g)
            have = pindex.get(g)
            if have is None:
                kind = "FXONLY" if g in fxindex else "MISSING"
                problems[(kind, g, f"game prefab {want}")].append(f"{tower} {tier}{path}")
            elif not loadable(have):
                problems[("NOFILES", g, f"folder {have}, game prefab {want}")].append(f"{tower} {tier}{path}")
            elif want and have.strip().lower() != want.lower():
                problems[("RENAMED", g, f"clone {have} vs game {want}")].append(f"{tower} {tier}{path}")

print(total, "projectile displays checked")
order = {"RENAMED": 0, "MISSING": 1, "NOFILES": 2, "FXONLY": 3}
show_all = "--all" in sys.argv
counts = defaultdict(int)
for (kind, g, detail), where in sorted(problems.items(), key=lambda kv: (order[kv[0][0]], kv[0][2])):
    counts[kind] += 1
    if kind == "FXONLY" and not show_all:
        continue
    towers = sorted({w.split()[0] for w in where})
    print(f"{kind:8} {detail} ({g[:8]}) used by {len(where)}: {', '.join(towers[:6])}{' ...' if len(towers) > 6 else ''}  e.g. {where[0]}")
print(dict(counts))
