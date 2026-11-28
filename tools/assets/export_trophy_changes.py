"""What each Trophy Store item changes in game, from the game's own data.

Usage: python -I export_trophy_changes.py GAME EXTRACTED

Every TrophyStoreItem (Assets/Data/TrophyStore/*.asset) lists itemTypes: an itemType
(Assets.Scripts.Data.TrophyStore.TrophyItemType) and an itemTarget, an AssetChangeItem in the same
file: TowerAssetChange (prefab / animation / audio swaps for some towers), Pet, Prop, BloonAssetChange
(prefab swaps for one bloon), BloonDecal (overlay layers), BloonPopFX, PowerAssetChange, and the
music / avatar / banner / emote items the menus already read. Writes
<EXTRACTED>/game/trophy_changes.json: {item id: [{"type": <TrophyItemType name>, "change": {...}}]}
with each change's fields as the game stores them (GUID references as {"guidRef": ...}; managed
references such as a pet's behaviours resolved inline). Proprietary data: local use only.
"""
import glob
import json
import sys
from pathlib import Path

import UnityPy

# Assets.Scripts.Data.TrophyStore.TrophyItemType, in declaration order (Invalid is not stored).
ITEM_TYPES = ["TowerAssetChange", "TowerPet", "NamedMonkeys", "CoopEmote", "BloonAssetChange", "BloonDecal",
              "BloonPopEffect", "Invalid", "MusicTrack", "Avatar", "PowerAssetChange", "Prop", "Banner"]
SKIP = {"m_GameObject", "m_Enabled", "m_Script"}


def resolve_references(tree):
    """Managed references ([SerializeReference]: {"rid": n} pointing into references.RefIds) inline,
    each as {"class": <type name>, **its data}."""
    refs = {r["rid"]: r for r in (tree.get("references") or {}).get("RefIds", [])}
    if not refs:
        return {k: v for k, v in tree.items() if k != "references"}

    def walk(value):
        if isinstance(value, dict):
            if set(value) == {"rid"} and value["rid"] in refs:
                r = refs[value["rid"]]
                return {"class": r["type"]["class"], **walk(r.get("data") or {})}
            return {k: walk(v) for k, v in value.items() if k != "references"}
        if isinstance(value, list):
            return [walk(v) for v in value]
        return value
    return walk(tree)


def export_bloon_skins(game, root, items, names):
    """The MOAB-class skins' models (BloonAssetChange swap targets) to models/bloons/<name>, as
    export_bloon_models.py exports the originals (existing exports are kept)."""
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from model_batch import run
    paths = json.loads((root / "all_container_paths.json").read_text(encoding="utf-8"))
    by_name = {}
    for path, bundle in paths.items():
        if path.endswith(".prefab"):
            by_name.setdefault(Path(path).stem, []).append((path, bundle))
    jobs = {}
    for changes in items.values():
        for c in changes:
            if c["type"] != "BloonAssetChange" or not c["change"]:
                continue
            for swap in c["change"].get("prefabSwaps", []):
                name = names.get(swap["swapToo"]["guidRef"])
                found = sorted(by_name.get(name or "", []))
                if not found:
                    print("bloon skin prefab not found:", name, flush=True)
                    continue
                path, bundle = found[0]
                out = str(root / "models/bloons" / name)
                jobs[out] = [out, bundle.rsplit("_", 1)[0], path]
    print(len(jobs), "bloon skin models", flush=True)
    run(str(game), list(jobs.values()))


def export_pop_effects(game, root, items, names):
    """The pop effects' prefabs (BloonPopFX.popFX: particle systems, not the default Pop flipbook) to
    effects/<name>, as the other effects are exported."""
    import subprocess
    paths = json.loads((root / "all_container_paths.json").read_text(encoding="utf-8"))
    wanted = set()
    for changes in items.values():
        for c in changes:
            if c["type"] == "BloonPopEffect" and c["change"]:
                name = names.get(c["change"]["popFX"]["guidRef"])
                found = sorted(p for p in paths if name and p.endswith(f"/{name}.prefab"))
                if found:
                    wanted.add(found[0])
                else:
                    print("pop effect prefab not found:", name, flush=True)
    todo = sorted(p for p in wanted if not (root / "effects" / Path(p).stem / "effect.json").exists())
    print(len(wanted), "pop effects,", len(todo), "to export", flush=True)
    if todo:
        subprocess.run([sys.executable, "-X", "utf8", "-I", str(Path(__file__).with_name("export_particles.py")),
                        str(game), str(root / "effects"), *todo], check=True)


def main():
    game, root = Path(sys.argv[1]), Path(sys.argv[2])
    base = game / "BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full"
    env = UnityPy.load(glob.glob(str(base / "game_data_assets_all_*.bundle"))[0])
    out, seen = {}, set()
    for path, pointer in sorted(env.container.items()):
        if not path.startswith("Assets/Data/TrophyStore/") or not path.endswith(".asset") or path in seen:
            continue
        seen.add(path)
        obj = pointer.deref() if hasattr(pointer, "deref") else pointer
        item = obj.read_typetree()
        if not item.get("id") or "itemTypes" not in item:
            continue
        changes = []
        for entry in item["itemTypes"]:
            kind = entry["itemType"]
            target = entry.get("itemTarget") or {}
            change = None
            if target.get("m_PathID"):
                if target.get("m_FileID"):
                    raise ValueError(f"{item['id']}: change in another file")
                change = obj.assets_file.objects[target["m_PathID"]].read_typetree()
                change = resolve_references({k: v for k, v in change.items() if k not in SKIP})
            changes.append({"type": ITEM_TYPES[kind] if 0 <= kind < len(ITEM_TYPES) else str(kind), "change": change})
        out[item["id"]] = changes
    # Every referenced prefab's name (catalog_prefabs.json: GUID -> file name), so the game resolves
    # swaps without the whole catalog.
    catalog = json.loads((root / "catalog_prefabs.json").read_text(encoding="utf-8"))
    names = {}

    def collect(value):
        if isinstance(value, dict):
            g = value.get("guidRef")
            if isinstance(g, str) and g in catalog:
                names[g] = Path(catalog[g]).stem
            for v in value.values():
                collect(v)
        elif isinstance(value, list):
            for v in value:
                collect(v)
    collect(out)
    dest = root / "game/trophy_changes.json"
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text(json.dumps({"items": out, "names": names}, indent=1, sort_keys=True), encoding="utf-8")
    export_bloon_skins(game, root, out, names)
    export_pop_effects(game, root, out, names)
    kinds = {}
    for changes in out.values():
        for c in changes:
            kinds[c["type"]] = kinds.get(c["type"], 0) + 1
    print(len(out), "trophy items' changes:", ", ".join(f"{k} {v}" for k, v in sorted(kinds.items())), flush=True)


if __name__ == "__main__":
    main()
