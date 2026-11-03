"""Models the game's tower data uses that no other step exports, so they're on disk before the mechanics
that show them are built (docs/dev/NOT_BUILT.md): Magus Perfectus' phoenix and graveyard markers, the
Paragons' degree add-ons, Beast Handler's upgraded pets, Rosalia's weapons...

    tools/.venv/Scripts/python.exe -I export_pending_models.py <game install>

Reads the decoded model (local_extracted/decoded) for the fields below. A TowerModel's display goes to
models/subtowers/<name> (where sub-towers load from), anything else to models/extra/<name>, and
models/extra/index.json maps each display GUID to its folder. Then the effects (EffectModel.assetId,
AssetPathModel.assetPath, text pop-ups) and the prefabs above with no mesh go to effects/<name>, as the
particles step writes them. Proprietary art: local use only.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

GAME = sys.argv[1]
L = Path("local_extracted")
# (model type, field) whose prefab is a model shown in game.
FIELDS = {
    ("TowerModel", "display"),
    ("ParagonAssetSwapModel", "newDisplay"),
    ("ParagonAssetSwapModel", "displayToSwap"),
    ("ParagonAssetSwapPerDegreeModel", "newDisplay"),
    ("BeastHandlerPetDisplayStepModel", "display"),
    ("BeastHandlerPetDisplayStepModel", "projectileDisplay"),
    ("AirUnitModel", "display"),
    ("MagusPerfectusGraveyardStateManagerModel", "graveyardDisplay1"),
    ("MagusPerfectusGraveyardStateManagerModel", "graveyardDisplay2"),
    ("MakeshiftAreaTowerDisplayModel", "towerDisplay"),
    ("FreezeNearbyWaterModel", "freezeAsset"),
    ("MonkeyFanClubModel", "towerOriginDisplay"),
    ("MoabTakedownModel", "displayAtEjectId"),
    ("TargetSelectedPointModel", "display"),
    ("SpiritOfTheForestModel", None),
    ("RosaliaMovementModel", None),
    ("JungleVineEffectModel", None),
    ("CreateRopeEffectModel", None),
}
# (model type, field) whose prefab is a particle effect or text pop-up.
EFFECT_FIELDS = {
    ("EffectModel", "assetId"),
    ("AssetPathModel", "assetPath"),
    ("CreateTextEffectModel", "assetId"),
    ("ShowTextOnHitModel", "assetId"),
    ("IncreaseWorthTextEffectModel", "assetId"),
    ("PerRoundCashBonusTowerModel", "assetId"),
    ("BankModel", "fullBankAssetId"),
    ("OverclockModel", "buffDisplayPath"),
    ("MagusPerfectusGraveyardStateManagerModel", "textDisplayPrefab"),
}


def key(name):
    return name.strip().lower().replace(" ", "_")


def main():
    prefabs = json.loads((L / "catalog_prefabs.json").read_text(encoding="utf-8"))
    paths = json.loads((L / "all_container_paths.json").read_text(encoding="utf-8"))
    have = set()
    for top in ("models", "effects", "bloon_displays"):
        for parent, _, _ in os.walk(L / top):
            have.add(key(os.path.basename(parent)))

    wanted = {}  # name -> (kind, guids)
    effects = set()  # effect prefab names
    with open(L / "decoded/named/objects.jsonl", encoding="utf-8") as stream:
        for line in stream:
            record = json.loads(line)
            typ = record["type"]
            model = any(t == typ for t, _ in FIELDS)
            if not model and not any(t == typ for t, _ in EFFECT_FIELDS):
                continue
            for field, value in record["f"].items():
                effect = (typ, field) in EFFECT_FIELDS
                if not effect and (typ, field) not in FIELDS and (typ, None) not in FIELDS:
                    continue
                guid = value.get("guidRef") if isinstance(value, dict) else value
                if not isinstance(guid, str) or guid not in prefabs:
                    continue
                name = os.path.splitext(prefabs[guid].rsplit("/", 1)[-1])[0]
                if key(name) in have or key(name) in ("empty", "emptyprefab", "#emptyasset"):
                    continue
                if effect:
                    effects.add(name)
                    continue
                kind = "subtowers" if typ == "TowerModel" else "extra"
                wanted.setdefault(name, (kind, set()))[1].add(guid)

    jobs, missing = [], []
    for name, (kind, _) in sorted(wanted.items()):
        cand = sorted(p for p in paths if p.endswith(f"/{name}.prefab"))
        if cand:
            # Windows folders can't end in a space ("210-NinjaMonkey .prefab").
            out = str(L / "models" / kind / name.strip())
            jobs.append([out, paths[cand[0]].rsplit("_", 1)[0], cand[0]])
        else:
            missing.append(name)
    print(len(wanted), "pending models;", len(jobs), "to export,", len(missing), "without a prefab path", flush=True)
    if jobs:
        manifest = L / ".pending_model_jobs.json"
        manifest.write_text(json.dumps(jobs), encoding="utf-8")
        try:
            subprocess.run([sys.executable, "tools/assets/model_batch.py", GAME, str(manifest), "--keep-floats"], check=False)
        finally:
            manifest.unlink(missing_ok=True)

    # GUID -> folder, for what exported (a prefab with no mesh writes no meta.json).
    index = {}
    for name, (kind, guids) in wanted.items():
        if (L / "models" / kind / name.strip() / "meta.json").exists():
            for g in guids:
                index[g] = f"{kind}/{name.strip()}"
    extra = L / "models/extra"
    extra.mkdir(parents=True, exist_ok=True)
    old = json.loads((extra / "index.json").read_text(encoding="utf-8")) if (extra / "index.json").exists() else {}
    old.update(index)
    (extra / "index.json").write_text(json.dumps(old, indent=1, sort_keys=True), encoding="utf-8")
    meshless = {n for n, (k, _) in wanted.items() if not (L / "models" / k / n.strip() / "meta.json").exists()}
    print(len(index), "display GUIDs indexed;", len(meshless), "had no mesh", flush=True)

    # Effects, a few dozen prefabs a process (each keeps its bundles loaded).
    todo = []
    for name in sorted(effects | meshless):
        cand = sorted(p for p in paths if p.endswith(f"/{name}.prefab"))
        if cand and not (L / "effects" / name / "effect.json").exists():
            todo.append(cand[0])
    print(len(todo), "pending effects to export", flush=True)
    for i in range(0, len(todo), 40):
        subprocess.run([sys.executable, "tools/assets/export_particles.py", GAME, str(L / "effects"), *todo[i:i + 40]], check=False)
    done = sum((L / "effects" / p.rsplit("/", 1)[-1].removesuffix(".prefab") / "effect.json").exists() for p in todo)
    print(done, "of", len(todo), "effects exported (a prefab without particles writes none)", flush=True)


if __name__ == "__main__":
    main()
