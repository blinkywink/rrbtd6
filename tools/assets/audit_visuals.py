"""List every tower / hero visual the clone can't draw, so missing projectiles and effects show up
without playing every upgrade.

    tools/.venv/Scripts/python.exe -I tools/assets/audit_visuals.py [Tower ...] > audit.txt

Three checks:
  1. Projectile displays in game.json (every projectile, sub-projectile and orbit): the display
     GUID must be planned (display_plans), exported (models/projectiles/index.json + its folder),
     and a particle-only prefab must have its particle effect (effects/proj_index.json), else the
     renderer draws a dart or a disc instead.
  2. Effect names in game.json (place_fx, expire_fx, fx, ...): effects/<name> must exist.
  3. Model behaviours that make visuals (*Effect*, *Display*, Line*, *Ray*) in the game's models
     that tools/assets/build_game_data.py never mentions: visuals the clone doesn't model at all.
Proprietary data: local use only.
"""
import glob
import json
import os
import re
import sys
from collections import defaultdict

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
L = os.path.join(ROOT, "local_extracted")


def load(p, default=None):
    try:
        return json.load(open(p, encoding="utf-8"))
    except (OSError, ValueError):
        return default


def main():
    only = set(sys.argv[1:])
    g = load(os.path.join(L, "game", "game.json"))
    plans = {}
    for f in sorted(glob.glob(os.path.join(L, "display_plans", "*.json"))):
        plans.update(load(f, {}).get("projectiles", {}))
    index = load(os.path.join(L, "models", "projectiles", "index.json"), {})
    proj_fx = load(os.path.join(L, "effects", "proj_index.json"), {})
    effects = set(os.listdir(os.path.join(L, "effects")))

    def proj_status(guid):
        if not guid:
            return None
        if guid not in index:
            p = plans.get(guid)
            if p is None:
                return "unmapped (no prefab for this display GUID in display_plans)"
            if not p.get("prefab"):
                return "no prefab path in display_plans"
            return f"not exported ({p['prefab']})"
        name = index[guid]
        d = os.path.join(L, "models", "projectiles", name)
        if not os.path.isdir(d) or not os.listdir(d):
            return f"empty export folder {name}"
        has = set(os.listdir(d))
        if "layers.json" in has or "meta.json" in has:
            return None
        if "sprite.json" in has:
            meta = load(os.path.join(d, "sprite.json"), {})
            if meta.get("standin") or meta.get("particle"):
                return None if name in proj_fx else f"particle-only {name}: no particle effect exported"
            return None
        return f"unloadable export folder {name} ({sorted(has)[:4]})"

    # 1 + 2: walk game.json
    problems = defaultdict(lambda: defaultdict(set))  # tower -> problem -> tiers
    fx_keys = re.compile(r"(^|_)fx$")

    def walk(o, tower, tier, in_proj):
        if isinstance(o, dict):
            is_proj = "pierce" in o and "display" in o
            if is_proj:
                s = proj_status(o.get("display"))
                if s:
                    problems[tower][f"projectile {o.get('display')}: {s}"].add(tier)
                for k in ("pierce_sprites",):
                    for gd in o.get(k) or []:
                        s = proj_status(gd)
                        if s:
                            problems[tower][f"pierce sprite {gd}: {s}"].add(tier)
            for k, v in o.items():
                if fx_keys.search(k) and v:
                    names = [v] if isinstance(v, str) else [x for x in v if isinstance(x, str)] if isinstance(v, list) else []
                    for n in names:
                        if n and n not in effects:
                            problems[tower][f"effect {n} ({k}): not in effects/"].add(tier)
                if k == "display" and not is_proj and isinstance(v, str) and len(v) == 32 and re.fullmatch(r"[0-9a-f]{32}", v):
                    s = proj_status(v)
                    if s:
                        problems[tower][f"display {v} ({'orbit/other'}): {s}"].add(tier)
                walk(v, tower, tier, in_proj or is_proj)
        elif isinstance(o, list):
            for x in o:
                walk(x, tower, tier, in_proj)

    for tower, tt in g["tower_tiers"].items():
        if only and tower not in only:
            continue
        for k, tier in tt.get("tiers", {}).items():
            walk(tier, tower, k, False)
        for name, tier in (tt.get("heroes") or tt.get("hero") or {}).items() if isinstance(tt.get("hero"), dict) else []:
            walk(tier, tower, name, False)
        if isinstance(tt.get("paragon"), dict):
            walk(tt["paragon"], tower, "paragon", False)

    # 3: model behaviours the builder never mentions
    builder = open(os.path.join(ROOT, "tools", "assets", "build_game_data.py"), encoding="utf-8").read()
    objs = {}
    for line in open(os.path.join(L, "decoded", "named", "objects.jsonl"), encoding="utf-8"):
        r = json.loads(line)
        objs[r["i"]] = r
    vals = {int(k): v for k, v in load(os.path.join(L, "decoded", "values.json"), {}).items()}
    visual = re.compile(r"Effect|Display|^Line|Ray|Lightning|Beam|Trail|Flame|Fire")

    def deref(v):
        return (objs.get(v["$ref"]) or vals.get(v["$ref"])) if isinstance(v, dict) and "$ref" in v else v

    def kids(o):
        for v in o.get("f", {}).values():
            c = deref(v)
            if isinstance(c, dict) and "items" in c:
                for i in c["items"]:
                    if isinstance(i, int) and i in objs:
                        yield objs[i]
            elif isinstance(c, dict) and "type" in c and "f" in c:
                yield c

    unmodeled = defaultdict(lambda: defaultdict(set))
    for o in objs.values():
        if o.get("type") != "TowerModel":
            continue
        nm = deref(o["f"].get("_name"))
        if not isinstance(nm, str) or not nm:
            continue
        base, _, tier = nm.partition("-")
        if only and base not in only:
            continue
        seen, stack = set(), [o]
        while stack:
            x = stack.pop()
            if x["i"] in seen:
                continue
            seen.add(x["i"])
            t = x.get("type", "")
            if t.endswith("Model") and visual.search(t) and t not in builder:
                unmodeled[base][t].add(tier or "000")
            if t != "TowerModel" or x is o:
                stack.extend(kids(x))

    for tower in sorted(set(problems) | set(unmodeled)):
        print(f"== {tower}")
        for p, tiers in sorted(problems[tower].items()):
            print(f"  {p}  [{', '.join(sorted(tiers)[:12])}{' ...' if len(tiers) > 12 else ''}]")
        for t, tiers in sorted(unmodeled[tower].items()):
            print(f"  unmodelled {t}  [{', '.join(sorted(tiers)[:12])}{' ...' if len(tiers) > 12 else ''}]")


if __name__ == "__main__":
    main()
