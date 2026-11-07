"""The bloon overlay prefabs (glue, ice, bleed... on a bloon: "<overlay class><overlayType>", e.g.
WhiteRegrowGlueBasic, MoabIce) as a display plan, for export_projectiles.py to export like projectile
displays into models/projectiles/ (indexed by display GUID in its index.json).

    tools/.venv/Scripts/python.exe -I plan_overlays.py

Reads game.json's bloon overlay classes and effect overlay types; writes
local_extracted/display_plans/Overlays.json. Proprietary art: local use only.
"""
import json
import os
from pathlib import Path

L = Path("local_extracted")


def main():
    game = json.loads((L / "game/game.json").read_text(encoding="utf-8"))
    prefabs = json.loads((L / "catalog_prefabs.json").read_text(encoding="utf-8"))
    paths = json.loads((L / "all_container_paths.json").read_text(encoding="utf-8"))
    classes = {b.get("overlayClass") for b in game["bloons"].values() if isinstance(b, dict)} - {"", None}

    overlays = set()

    def walk(v):
        if isinstance(v, dict):
            if isinstance(v.get("overlay"), str) and v["overlay"]:
                overlays.add(v["overlay"])
            for x in v.values():
                walk(x)
        elif isinstance(v, list):
            for x in v:
                walk(x)

    walk(game)
    # GameData.bloonOverlayData (export_overlays.py): each type's prefab per class; a type not in
    # it: "<class><type>" (MiniLych's prefabs are named LychMini...).
    table = game.get("overlays") or {}
    names = {n for o in overlays if o in table for c, n in table[o]["assets"].items() if c in classes}
    names |= {(c.replace("MiniLych", "LychMini") + o) for c in classes for o in overlays if o not in table}
    by_name = {}
    for guid, n in prefabs.items():
        stem = n.rsplit("/", 1)[-1].removesuffix(".prefab")
        if stem in names:
            by_name.setdefault(stem, guid)
    plan = {}
    for stem, guid in sorted(by_name.items()):
        cand = sorted(p for p in paths if p.endswith(f"/{stem}.prefab"))
        if cand:
            plan[guid] = {"name": stem, "prefab": cand[0], "mapping": "bloon-overlay"}
    out = L / "display_plans/Overlays.json"
    out.write_text(json.dumps({"towers": {}, "projectiles": plan}, indent=1), encoding="utf-8")
    print(len(classes), "overlay classes,", len(overlays), "overlay types,", len(plan), "overlay prefabs planned", flush=True)


if __name__ == "__main__":
    main()
