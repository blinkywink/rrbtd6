"""Per-tower asset export (what build_local_assets.sh does in its tower loops), for adding towers.

Usage: tools/.venv/Scripts/python.exe tools/assets/export_tower_assets.py <game install> <TowerId>...

For each tower: the display plan, every tier model, the base model (models/<TowerId>), the
projectile displays, the upgrade icons and the portraits. Needs all_container_paths.json and the
catalog JSONs from build_local_assets.sh. Proprietary art: local use only.
"""
import json
import unicodedata
import os
import subprocess
import sys
from pathlib import Path

GAME, TOWERS = sys.argv[1], sys.argv[2:]
PY = sys.executable
# Upgrade icon folders that aren't named after the tower id.
# Portrait folders that don't hold "<key>-<tower id>.png" files.
PORTRAIT_DIRS = {"WizardMonkey": "Wizard"}
ICON_DIRS = {"Druid": "DruidMonkey", "DartlingGunner": "DartlingMonkey", "Skywarden": "SkyWarden"}


def run(*args, span=(.65, .25)):
    lines = []
    with subprocess.Popen([PY, *args], stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                          text=True, encoding="utf-8", errors="replace", bufsize=1) as child:
        for line in child.stdout:
            if line.startswith("@@RRBTD6_PROGRESS "):
                event = json.loads(line[len("@@RRBTD6_PROGRESS "):])
                event["done"] = span[0] + span[1] * event["done"] / max(1, event["total"])
                event["total"] = 1
                print("@@RRBTD6_PROGRESS " + json.dumps(event), flush=True)
            else:
                lines.append(line.rstrip())
                if len(lines) > 25: lines.pop(0)
        code = child.wait()
    print(args[0], "ok" if code == 0 else "FAILED", (lines or [""])[-1][:160], flush=True)
    if code:
        print("\n".join(lines), flush=True)
        raise subprocess.CalledProcessError(code, args)


run("tools/assets/build_display_plan.py", GAME, *TOWERS)
run("tools/assets/export_tower_tiers.py", GAME, *TOWERS, span=(0., .65))
paths = json.loads(Path("local_extracted/all_container_paths.json").read_text(encoding="utf-8"))
art_jobs = []
model_jobs = {}
model_indices = []
for t in TOWERS:
    plan = json.loads(Path(f"local_extracted/display_plans/{t}.json").read_text(encoding="utf-8"))
    base = plan["towers"].get("000")
    if base and not os.path.exists(f"local_extracted/models/{t}/meta.json"):
        out = f"local_extracted/models/{t}"
        model_jobs.setdefault(out, [out, base["bundle"].rsplit("_", 1)[0], base["prefab"]])
    icon_dir = ICON_DIRS.get(t, t)
    icons = [p for p in paths if p.startswith(f"Assets/ResizedImages/UI/UpgradeIcons/{icon_dir}/")]
    if icons:
        art_jobs.append({"out": "local_extracted/ui/upgrades", "paths": icons, "prefix_numeric": t, "ascii_names": True})
    for f in os.listdir("local_extracted/ui/upgrades") if os.path.isdir("local_extracted/ui/upgrades") else []:
        if len(f) > 3 and f[:3].isdigit() and f[3:] == "UpgradeIcon.png":
            os.replace(f"local_extracted/ui/upgrades/{f}", f"local_extracted/ui/upgrades/{t}{f}")
        # Upgrade names are ASCII ("Pouakai") where the icon file isn't ("Pouākai").
        a = unicodedata.normalize("NFKD", f).encode("ascii", "ignore").decode()
        if a != f and os.path.exists(f"local_extracted/ui/upgrades/{f}"):
            os.replace(f"local_extracted/ui/upgrades/{f}", f"local_extracted/ui/upgrades/{a}")
    ps = [p for p in paths if p.startswith("Assets/ResizedImages/MonkeyPortraits/")]
    dirs = {os.path.dirname(p) for p in ps if p.lower().endswith(f"-{t}.png".lower())}
    if t in PORTRAIT_DIRS:
        dirs.add(f"Assets/ResizedImages/MonkeyPortraits/{PORTRAIT_DIRS[t]}")
    portraits = [p for p in ps if os.path.dirname(p) in dirs]
    if portraits:
        art_jobs.append({"out": f"local_extracted/ui/portraits/{t}", "paths": portraits, "portrait_tower": t})
        # The HUD looks portraits up by tower id ("500-Skywarden", not "500-SkyWarden").
        pd = f"local_extracted/ui/portraits/{t}"
        for f in os.listdir(pd) if os.path.isdir(pd) else []:
            k, _, rest = f.partition("-")
            if k.isdigit() and rest.endswith(".png") and rest != f"{t}.png":
                os.replace(f"{pd}/{f}", f"{pd}/{k}-{t}.png")
    print(t, len(icons), "upgrade icons,", len(portraits), "portraits", flush=True)


if art_jobs:
    manifest = "local_extracted/.extended_art_jobs.json"
    try:
        with open(manifest, "w", encoding="utf-8") as stream:
            json.dump(art_jobs, stream)
        run("tools/assets/export_textures.py", GAME, "--jobs", manifest, span=(.65, .20))
    finally:
        if os.path.exists(manifest):
            os.unlink(manifest)


def sub_prefabs(v, out):
    """Stand and turret prefab names of every create_tower and pet spec in game.json data `v`."""
    if isinstance(v, dict):
        for s in (v.get("create_tower") or []) + (v.get("pets") or []):
            out.update(n for n in (s.get("base"), s.get("model")) if n)
        for x in v.values():
            sub_prefabs(x, out)
    elif isinstance(v, list):
        for x in v:
            sub_prefabs(x, out)


# Paragon display (game.json tower_tiers[t].paragon.display) -> models/<t>/tiers/<prefab>, listed
# as "paragon" in that tiers/index.json.
def export_paragon(t, tt):
    d = f"local_extracted/models/{t}/tiers"
    idx_path = f"{d}/index.json"
    idx = json.loads(Path(idx_path).read_text(encoding="utf-8")) if os.path.exists(idx_path) else {}
    # The tower display ("paragon") and the attack display on top of it ("paragon_top"), plus
    # each display degree's version: "paragon_top<N>" from displayDegreePaths, "paragon<N>"
    # where a "...Lvl<N>" prefab of the base exists.
    pg = tt.get("paragon") or {}
    wanted = [("paragon", pg.get("display")), ("paragon_top", pg.get("top"))]
    wanted += [(f"paragon_top{i + 1}", n) for i, n in enumerate(pg.get("tops") or [])]
    wanted += [(f"paragon{i + 1}", n) for i, n in enumerate(pg.get("bases") or [])]
    base = pg.get("display") or ""
    if "Lvl1" in base and not pg.get("bases"):
        wanted += [(f"paragon{i}", base.replace("Lvl1", f"Lvl{i}")) for i in range(1, (pg.get("levels") or 1) + 1)]
    for key, n in wanted:
        if not n:
            continue
        if not any(p.endswith(f"/{n}.prefab") for p in paths) and not os.path.exists(f"{d}/{n}/meta.json"):
            continue
        if not os.path.exists(f"{d}/{n}/meta.json"):
            cand = sorted((p for p in paths if p.endswith(f"/{n}.prefab")), key=lambda p: (f"/{t}/" not in p, p))
            if not cand:
                print(t, "paragon prefab not found:", n, flush=True)
                continue
            out = f"{d}/{n}"
            model_jobs.setdefault(out, [out, paths[cand[0]].rsplit("_", 1)[0], cand[0]])
        idx[key] = n
    model_indices.append((idx_path, idx))


# Sub-tower displays (Engineer sentries) -> models/subtowers/, and air units (Heli Pilot) ->
# models/air/, from game.json (build_game_data.py).
game = json.loads(Path("local_extracted/game/game.json").read_text(encoding="utf-8"))
for t in TOWERS:
    tt = game.get("tower_tiers", {}).get(t) or {}
    export_paragon(t, tt)
    names = set()
    sub_prefabs(tt, names)
    # Air units: the tower's own (Heli, Ace) and its pets' (Beast Handler birds).
    air = {(tr.get("air") or {}).get("display") for tr in (tt.get("tiers") or {}).values()}
    air |= {((p.get("tier") or {}).get("air") or {}).get("display")
            for tr in (tt.get("tiers") or {}).values() for p in tr.get("pets") or []}
    # ... and its spawned sub-towers' (Aircraft Carrier planes).
    def sub_air(v, out):
        if isinstance(v, dict):
            for c in v.get("create_tower") or []:
                out.add(((c.get("tier") or {}).get("air") or {}).get("display"))
            # Comanche Defense's mini Comanches.
            if isinstance(v.get("comanche"), dict):
                out.add(((v["comanche"].get("tower", {}).get("tier") or {}).get("air") or {}).get("display"))
            for x in v.values():
                sub_air(x, out)
        elif isinstance(v, list):
            for x in v:
                sub_air(x, out)
    sub_air(tt, air)
    air -= {None}
    # Monkey Fan Club fans (the ability's towerDisplay) -> models/fans/.
    fans = {((a.get(k) or {}).get("display")) for tr in (tt.get("tiers") or {}).values()
            for a in tr.get("abilities") or [] for k in ("fan_club", "switch", "morph")} - {None}
    for kind, ns in (("subtowers", names), ("air", air), ("fans", fans)):
        for n in sorted(ns):
            if os.path.exists(f"local_extracted/models/{kind}/{n}/meta.json"):
                continue
            cand = sorted((p for p in paths if p.endswith(f"/{n}.prefab")),
                          key=lambda p: (f"/Monkeys/{t}/" not in p, p))
            if cand:
                out = f"local_extracted/models/{kind}/{n}"
                model_jobs.setdefault(out, [out, paths[cand[0]].rsplit("_", 1)[0], cand[0]])
            else:
                print(t, kind, "prefab not found:", n, flush=True)
# Geraldo's shop items: towers and merges defined outside every tower's own data.
def spec_prefabs(v, out):
    if isinstance(v, dict):
        out.update(v[k] for k in ("base", "model") if isinstance(v.get(k), str) and v[k])
        for x in v.values():
            spec_prefabs(x, out)
    elif isinstance(v, list):
        for x in v:
            spec_prefabs(x, out)


def spec_air(v, out):
    if isinstance(v, dict):
        if isinstance(v.get("air"), dict) and v["air"].get("display"):
            out.add(v["air"]["display"])
        for x in v.values():
            spec_air(x, out)
    elif isinstance(v, list):
        for x in v:
            spec_air(x, out)


shop_items = set()
spec_prefabs(game.get("geraldo") or [], shop_items)
# The Genie Bottle's floating genie.
shop_air = set()
spec_air(game.get("geraldo") or [], shop_air)
for n in sorted(shop_air):
    out = f"local_extracted/models/air/{n}"
    if not os.path.exists(f"{out}/meta.json"):
        cand = sorted((p for p in paths if p.endswith(f"/{n}.prefab")), key=lambda p: ("/Monkeys/Geraldo/" not in p, p))
        if cand:
            model_jobs.setdefault(out, [out, paths[cand[0]].rsplit("_", 1)[0], cand[0]])
        else:
            print("Geraldo air unit prefab not found:", n, flush=True)
for n in sorted(shop_items):
    out = f"local_extracted/models/subtowers/{n}"
    if not os.path.exists(f"{out}/meta.json"):
        cand = sorted(p for p in paths if p.endswith(f"/{n}.prefab"))
        if cand:
            model_jobs.setdefault(out, [out, paths[cand[0]].rsplit("_", 1)[0], cand[0]])
        else:
            print("Geraldo item prefab not found:", n, flush=True)
if model_jobs:
    manifest = "local_extracted/.extended_model_jobs.json"
    try:
        with open(manifest, "w", encoding="utf-8") as stream:
            json.dump(list(model_jobs.values()), stream)
        run("tools/assets/model_batch.py", GAME, manifest, "--keep-floats", span=(.85, .14))
    finally:
        if os.path.exists(manifest):
            os.unlink(manifest)
for path, index in model_indices:
    Path(path).write_text(json.dumps(index, indent=1), encoding="utf-8")

run("tools/assets/export_projectiles.py", GAME)

# One-off Animator clips the "Generated" tier bakes lack, taken from the original prefabs
# (export_animated.py) and grafted on (graft_clips.py): the Sun Temple opening when it
# sacrifices (SunTempleUpgrade -> TempleBaseUpgrade: top lifts, doors slide apart, the head rises
# out) and True Sun God appearing (SunTempleUpgradeTier5 -> TrueSunGodUpgrade).
# (tower, target tier model, source prefab, only this renderer (or None), clips, graft flags)
GRAFTS = [
    ("SuperMonkey", "SunTempleBase", "TrueSunGodBase", None, ["TempleBaseUpgrade"], []),
    ("SuperMonkey", "SunTempleBaseRed", "TrueSunGodBaseRed", None, ["TempleBaseUpgrade"], []),
    # The head is switched off in TrueSunGodBase but animated; the tier's bake of it is turned,
    # scaled and reordered (--match).
    ("SuperMonkey", "SunTempleMonkey", "TrueSunGodBase", "SunTempleHead", ["TempleBaseUpgrade"], ["--match"]),
    ("SuperMonkey", "TrueSunGodBase", "TrueSunGodBase", None, ["TrueSunGodUpgrade"], []),
    ("SuperMonkey", "TrueSunGodBaseRed", "TrueSunGodBaseRed", None, ["TrueSunGodUpgrade"], []),
    ("SuperMonkey", "TrueSunGodMonkey", "TrueSunGodMonkey", None, ["TrueSunGodUpgrade"], []),
]
# The sources in one batch: one load of their large bundle.
sources = {}
for t, target, source, only, clips, flags in GRAFTS:
    src = f"local_extracted/models/_originals/{only or source}"
    prefab = f"Assets/Monkeys/{t}/Graphics/{source}.prefab"
    if t in TOWERS and not os.path.exists(f"{src}/meta.json"):
        sources[src] = [src, paths[prefab].rsplit("_", 1)[0], prefab, {"RRBTD6_ANIM_ONLY": only} if only else {}]
if sources:
    manifest = "local_extracted/.graft_source_jobs.json"
    try:
        with open(manifest, "w", encoding="utf-8") as stream:
            json.dump(list(sources.values()), stream)
        run("tools/assets/model_batch.py", GAME, manifest, "--keep-floats")
    finally:
        if os.path.exists(manifest):
            os.unlink(manifest)
for t, target, source, only, clips, flags in GRAFTS:
    if t not in TOWERS:
        continue
    src = f"local_extracted/models/_originals/{only or source}"
    run("tools/assets/graft_clips.py", *flags, f"local_extracted/models/{t}/tiers/{target}", src, *clips)
