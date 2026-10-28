"""Plan every projectile display GUID in game.json that no display plan has yet, from the game's
own catalog (catalog_guids.json, else catalog_prefabs.json: GUID -> prefab file name), so
export_projectiles.py / export_particles.py export them too (sub-towers, paragons, abilities,
Phoenix, Sun Temple attacks: the shots build_display_plan.py's walk doesn't reach).

    tools/.venv/Scripts/python.exe -I tools/assets/plan_catalog_displays.py

Writes local_extracted/display_plans/_catalog.json {"towers": {}, "projectiles": {guid: {name,
prefab, mapping}}}. Run after build_display_plan.py and build_game_data.py.
"""
import glob
import json
import os
import re

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
L = os.path.join(ROOT, "local_extracted")
GUID = re.compile(r"[0-9a-f]{32}")
OUT = os.path.join(L, "display_plans", "_catalog.json")


def main():
    g = json.load(open(os.path.join(L, "game", "game.json"), encoding="utf-8"))
    planned = set()
    for f in glob.glob(os.path.join(L, "display_plans", "*.json")):
        if os.path.abspath(f) != OUT:
            planned |= set(json.load(open(f))["projectiles"])
    guids = json.load(open(os.path.join(L, "catalog_guids.json")))
    prefabs = json.load(open(os.path.join(L, "catalog_prefabs.json")))
    paths = json.load(open(os.path.join(L, "all_container_paths.json")))
    by_base = {}
    for p in paths:
        by_base.setdefault(os.path.basename(p), []).append(p)

    wanted = {}  # guid -> owner tower (for picking among same-named prefabs)

    def walk(o, tower):
        if isinstance(o, dict):
            for k, v in o.items():
                # A projectile's display and pierce sprites (not tower / sub-tower models).
                # (And LineEffectModel beams: line_fx.beam.)
                if ("pierce" in o and k in ("display", "pierce_sprites")) or ("endless" in o and k == "beam") or ("life" in o and k == "displays"):
                    for x in ([v] if isinstance(v, str) else v if isinstance(v, list) else []):
                        if isinstance(x, str) and GUID.fullmatch(x):
                            wanted.setdefault(x, tower)
                walk(v, tower)
        elif isinstance(o, list):
            for x in o:
                walk(x, tower)

    for tower, tt in g["tower_tiers"].items():
        walk(tt, tower)
    walk(g.get("heroes"), "Heroes")
    out, missing = {}, []
    for gd, tower in sorted(wanted.items()):
        if gd in planned:
            continue
        name = guids.get(gd) or prefabs.get(gd)
        cand = sorted(by_base.get(name, []), key=lambda c: (f"/{tower}/" not in c, "/Generated/" in c, c)) if name else []
        if not cand:
            missing.append((gd, tower, name))
            continue
        prefab = cand[0]
        out[gd] = {"name": os.path.splitext(os.path.basename(prefab))[0], "prefab": prefab, "mapping": "catalog"}
    json.dump({"towers": {}, "projectiles": out}, open(OUT, "w"), indent=1)
    print(len(out), "displays planned from the catalog;", len(missing), "without a prefab:")
    for m in missing:
        print("  ", *m)


if __name__ == "__main__":
    main()
