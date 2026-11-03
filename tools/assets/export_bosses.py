"""Every boss tier's stats and behaviours, into local_extracted/game/bosses.json, and their models into
local_extracted/models/bosses/<prefab>.

    python -I export_bosses.py <game install> [--data-only]

A boss is mostly data: HealthPercentTriggerModel and its siblings name actions by id (spawn bloons,
fireballs, speed changes...), so the file keeps each behaviour whole, by the game's own type and field
names, for the runtime to read. docs/dev/BOSSES.md explains them. Proprietary data: local use only.
"""
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

FAMILIES = ("Bloonarius", "Lych", "Vortex", "Dreadbloon", "Phayze", "Blastapopoulos")
ROOT = Path("local_extracted")
OUT = ROOT / "game/bosses.json"
SKIP = {"runtimeTypeIndex", "isTower", "childDependants"}


def load():
    objs = {}
    with open(ROOT / "decoded/named/objects.jsonl", encoding="utf-8") as stream:
        for line in stream:
            record = json.loads(line)
            objs[record["i"]] = record
    values = {int(k): v for k, v in json.load(open(ROOT / "decoded/values.json")).items()}
    return objs, values


def main():
    objs, values = load()
    prefabs = json.loads((ROOT / "catalog_prefabs.json").read_text(encoding="utf-8"))
    audio = json.loads((ROOT / "catalog_audio.json").read_text(encoding="utf-8"))

    def plain(value):
        """A model as nested JSON; asset GUIDs become their names."""
        if isinstance(value, str):
            if value in prefabs:
                return prefabs[value].removesuffix(".prefab")
            if value in audio:
                return audio[value].rsplit(".", 1)[0]
            return value
        if isinstance(value, list):
            return [plain(v) for v in value]
        if not isinstance(value, dict):
            return value
        if "$ref" in value:
            i = value["$ref"]
            value = objs[i] if i in objs else values.get(i)
            if not isinstance(value, dict):
                return plain(value)
        if "type" in value and "f" in value:
            fields = {k: plain(v) for k, v in value["f"].items() if k not in SKIP}
            return {"type": value["type"], **{k: v for k, v in fields.items() if v not in (None, "", [])}}
        if "items" in value:
            return [plain(objs[i] if i in objs else values.get(i)) for i in value["items"] if i]
        return {k: plain(v) for k, v in value.items()}

    bosses = {}
    for record in objs.values():
        if record["type"] != "BloonModel":
            continue
        f = record["f"]
        name = f.get("id", "")
        if not name.startswith(FAMILIES):
            continue
        family = next(x for x in FAMILIES if name.startswith(x))
        rest = name[len(family):]
        elite = rest.startswith("Elite")
        bosses[name] = {"family": family, "elite": elite, "tier": int(rest.removeprefix("Elite") or 0),
                        "model": prefabs.get(f.get("display"), "").removesuffix(".prefab"),
                        "health": f.get("maxHealth"), "speed": f.get("speed"), "radius": f.get("radius"),
                        "behaviors": [b for b in plain(f.get("behaviors")) or []
                                      if b and b.get("type") not in ("ShowDamageTextModel", "DisplayModel")]}
    OUT.write_text(json.dumps(bosses, separators=(",", ":")), encoding="utf-8")
    print(len(bosses), "boss tiers", OUT.stat().st_size >> 10, "KB", flush=True)
    if "--data-only" in sys.argv:
        return

    from model_batch import run
    paths = json.loads((ROOT / "all_container_paths.json").read_text(encoding="utf-8"))
    only = next((a.removeprefix("--only=") for a in sys.argv if a.startswith("--only=")), None)
    jobs = []
    for boss in sorted(bosses.values(), key=lambda b: b["model"]):
        if only and only not in boss["model"]:
            continue
        prefab = f"Assets/BossBloons/{boss['family']}/Prefabs/{boss['model']}.prefab"
        if prefab not in paths:
            print("no prefab", prefab, flush=True)
            continue
        out = ROOT / "models/bosses" / boss["model"]
        if not (out / "meta.json").exists():
            jobs.append([str(out), "asset_references_assets_all", prefab])
    if jobs:
        run(sys.argv[1], jobs)
    # Most of a boss model's size; tiers in a family share theirs, which the dedupe step links.
    import zstandard
    for attach in (ROOT / "models/bosses").glob("*/attach.bin"):
        attach.with_name("attach.bin.zst").write_bytes(zstandard.ZstdCompressor(level=9).compress(attach.read_bytes()))
        attach.unlink()
    total = sum(f.stat().st_size for f in (ROOT / "models/bosses").rglob("*") if f.is_file())
    print(len(jobs), "models exported;", total >> 20, "MB in models/bosses", flush=True)


if __name__ == "__main__":
    main()
