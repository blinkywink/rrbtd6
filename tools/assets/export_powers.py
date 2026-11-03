"""Every Power's rules, into local_extracted/game/powers.json, by Power id.

    python -I export_powers.py

ui/runtime/powers.json has what the Powers screen shows; this has what a Power does: its flags, its
behaviours by the game's own type and field names, and the tower it places, if any (`tower`: base id,
display prefab, and whether a model is on disk). docs/dev/NOT_BUILT.md has the plan.
Also local_extracted/display_plans/Powers.json: the displays of the projectiles Powers put down
(Cash Drop's crate, the track items, Road Spikes' piles), for export_projectiles.py.
Proprietary data: local use only.
"""
import json
from pathlib import Path

ROOT = Path("local_extracted")
OUT = ROOT / "game/powers.json"
SKIP = {"runtimeTypeIndex", "isTower", "childDependants", "tower"}


def main():
    objs = {}
    with open(ROOT / "decoded/named/objects.jsonl", encoding="utf-8") as stream:
        for line in stream:
            record = json.loads(line)
            objs[record["i"]] = record
    values = {int(k): v for k, v in json.load(open(ROOT / "decoded/values.json")).items()}
    prefabs = json.loads((ROOT / "catalog_prefabs.json").read_text(encoding="utf-8"))
    audio = json.loads((ROOT / "catalog_audio.json").read_text(encoding="utf-8"))

    def deref(value):
        if isinstance(value, dict) and "$ref" in value:
            i = value["$ref"]
            return objs[i] if i in objs else values.get(i)
        return value

    def items(value):
        value = deref(value)
        if isinstance(value, list):
            return value
        if isinstance(value, dict) and "items" in value:
            return [objs.get(i) if i in objs else values.get(i) for i in value["items"] if i]
        return []

    def plain(value, depth=0):
        """A model as nested JSON; asset GUIDs become their names."""
        if isinstance(value, str):
            if value in prefabs:
                return prefabs[value].removesuffix(".prefab")
            if value in audio:
                return audio[value].rsplit(".", 1)[0]
            return value
        value = deref(value)
        if isinstance(value, list):
            return [plain(v, depth + 1) for v in value]
        if not isinstance(value, dict) or depth > 24:
            return value if not isinstance(value, dict) else None
        if "type" in value and "f" in value:
            fields = {k: plain(v, depth + 1) for k, v in value["f"].items() if k not in SKIP}
            return {"type": value["type"], **{k: v for k, v in fields.items() if v not in (None, "", [])}}
        if "items" in value:
            return [plain(v, depth + 1) for v in items(value)]
        return {k: plain(v, depth + 1) for k, v in value.items()}

    game = next(o for o in objs.values() if o["type"] == "GameModel")
    powers = {}
    for power in items(game["f"].get("powers")):
        f = power["f"]
        tower = deref(f.get("tower"))
        placed = None
        if isinstance(tower, dict) and "f" in tower:
            base = tower["f"].get("baseId")
            model = prefabs.get(tower["f"].get("display"), "").removesuffix(".prefab")
            placed = {"base": base, "model": model,
                      "exported": any((ROOT / "models" / kind / model / "meta.json").exists() for kind in ("subtowers", "air", "."))}
        # The id the profile and Powers screen use (export_frontend_data.py): a Pro Power's powerId.
        name = f.get("powerId") if power["type"] == "PowerProModel" else (f.get("name") or f.get("_name"))
        powers[name] = {"cost": f.get("cost", 0), "quantity": f.get("quantity", 1),
                        "between_rounds": bool(f.get("canBeActivatedBetweenRounds")), "hidden": bool(f.get("isHidden")),
                        "screen_centre": bool(f.get("alwaysPlaceAtScreenCentre")), "tower": placed,
                        "mode": f.get("availableInMode") or None,
                        "behaviors": [b for b in (plain(x) for x in items(f.get("behaviors"))) if b]}
    OUT.write_text(json.dumps(powers, separators=(",", ":")), encoding="utf-8")
    containers = json.loads((ROOT / "all_container_paths.json").read_text(encoding="utf-8"))
    plan = {}
    by_name = {}
    for k in containers:
        if k.endswith(".prefab"):
            by_name.setdefault(k.rsplit("/", 1)[1].removesuffix(".prefab"), k)
    for power in items(game["f"].get("powers")):
        for b in items(power["f"].get("behaviors")):
            proj = deref(b.get("f", {}).get("projectileModel"))
            if not isinstance(proj, dict) or "f" not in proj:
                continue
            # Its display, its DisplayModels' and SetSpriteFromPierceModel's sprites (as build_game_data).
            guids = [proj["f"].get("display")]
            for x in items(proj["f"].get("behaviors")):
                if x and x["type"] == "DisplayModel":
                    guids.append(x["f"].get("display"))
                if x and x["type"] == "SetSpriteFromPierceModel":
                    guids += [s for s in (items(x["f"].get("sprites")) or []) if isinstance(s, str)]
            for guid in guids:
                name = prefabs.get(guid or "", "").removesuffix(".prefab")
                if name and name in by_name:
                    plan[guid] = {"name": name, "prefab": by_name[name], "mapping": "power"}
    (ROOT / "display_plans/Powers.json").write_text(json.dumps({"towers": {}, "projectiles": plan}, indent=1), encoding="utf-8")
    towers = [p for p in powers.values() if p["tower"]]
    print(len(powers), "powers;", len(towers), "place a tower,", sum(not t["tower"]["exported"] for t in towers),
          "of those without a model;", OUT.stat().st_size >> 10, "KB;", len(plan), "projectile displays planned", flush=True)


if __name__ == "__main__":
    main()
