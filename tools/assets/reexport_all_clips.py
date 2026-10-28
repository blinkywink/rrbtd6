"""Re-export every tower model with all of its clips (export_animated.py + compact_anims.py now
keep the placement, idle-variant, attack-idle and upgrade clips the game's
MonkeyAnimationController plays, not just idle and attack).

Usage: python -I tools/assets/reexport_all_clips.py <game install> [chunk size] [target list file]
Covers models/<Tower>/tiers/* (each display plan's tier and attack displays) and
models/{subtowers,air,fans}/*. New exports go to local_extracted/models_new/ and replace the old
dir (moved to local_extracted/models_old/) one by one; reruns skip what's done. Afterwards
export_tower_assets.py's GRAFTS (Sun Temple clips) are re-applied to their tier models.
"""
import glob
import json
import os
import shutil
import subprocess
import sys

GAME = sys.argv[1]
CHUNK = int(sys.argv[2]) if len(sys.argv) > 2 else 40
PY = sys.executable
paths = json.load(open("local_extracted/all_container_paths.json"))
NEW, OLD = "local_extracted/models_new", "local_extracted/models_old"

jobs = {}  # target dir -> (bundle glob, prefab)
for plan_p in sorted(glob.glob("local_extracted/display_plans/*.json")):
    t = os.path.splitext(os.path.basename(plan_p))[0]
    for entry in json.load(open(plan_p)).get("towers", {}).values():
        for e in [entry] + entry.get("attack_displays", []):
            stem = os.path.splitext(os.path.basename(e["prefab"]))[0].strip().replace(" ", "_")
            target = f"local_extracted/models/{t}/tiers/{stem}"
            if os.path.isdir(target):
                jobs.setdefault(target, (e["bundle"].rsplit("_", 1)[0], e["prefab"]))
for kind in ("subtowers", "air", "fans"):
    for d in sorted(glob.glob(f"local_extracted/models/{kind}/*/")):
        n = os.path.basename(d.rstrip("/\\"))
        cand = sorted((p for p in paths if p.endswith(f"/{n}.prefab")), key=lambda p: ("/Monkeys/" not in p, "/Generated/" in p, p))
        if cand:
            jobs.setdefault(d.rstrip("/\\").replace("\\", "/"), (paths[cand[0]].rsplit("_", 1)[0], cand[0]))

# Every other exported model: tier dirs no display plan names (Paragon degree displays,
# "...Legs" bases) and each tower's base model (models/<Tower>, its plan's 000 display).
def by_name(n, t):
    cand = sorted((p for p in paths if p.endswith(f"/{n}.prefab")), key=lambda p: (f"/{t}/" not in p, "/Generated/" in p, p))
    return (paths[cand[0]].rsplit("_", 1)[0], cand[0]) if cand else None
for plan_p in sorted(glob.glob("local_extracted/display_plans/*.json")):
    t = os.path.splitext(os.path.basename(plan_p))[0]
    base = json.load(open(plan_p)).get("towers", {}).get("000")
    if base and os.path.exists(f"local_extracted/models/{t}/meta.json"):
        jobs.setdefault(f"local_extracted/models/{t}", (base["bundle"].rsplit("_", 1)[0], base["prefab"]))
    for d in sorted(glob.glob(f"local_extracted/models/{t}/tiers/*/")):
        target = d.rstrip("/\\").replace("\\", "/")
        if target not in jobs and (j := by_name(os.path.basename(target), t)):
            jobs[target] = j
# Optional: only the target dirs listed in this file (one per line).
if len(sys.argv) > 3:
    only = {l.strip().replace("\\", "/") for l in open(sys.argv[3]) if l.strip()}
    jobs = {t: j for t, j in jobs.items() if t in only}
    for t in only:
        if os.path.exists(os.path.join(t, ".allclips")):
            os.remove(os.path.join(t, ".allclips"))


def staged(target):
    return target.replace("local_extracted/models/", NEW + "/", 1)

todo = [(t, b, p) for t, (b, p) in sorted(jobs.items(), key=lambda kv: (kv[1][0], kv[0]))
        if not os.path.exists(os.path.join(t, ".allclips"))]
print(len(jobs), "models,", len(todo), "to do", flush=True)
for i in range(0, len(todo), CHUNK):
    chunk = todo[i:i + CHUNK]
    os.makedirs(NEW, exist_ok=True)
    jp = os.path.join(NEW, "jobs.json")
    json.dump([[staged(t), b, p] for t, b, p in chunk], open(jp, "w"))
    r = subprocess.run([PY, "-I", "tools/assets/export_animated_batch.py", GAME, jp], capture_output=True, text=True)
    for line in r.stdout.splitlines():
        if line.startswith(("ok", "FAIL")):
            print(line, flush=True)
    if r.returncode:
        print("chunk failed:", (r.stderr.strip().splitlines() or ["?"])[-1][:300], flush=True)
    for t, _, _ in chunk:
        s = staged(t)
        if not os.path.exists(os.path.join(s, "meta.json")) or not any(os.path.exists(os.path.join(s, name)) for name in ("anim_q.bin", "anim_q.bin.z", "anim_q.bin.dz")):
            continue
        old = t.replace("local_extracted/models/", OLD + "/", 1)
        os.makedirs(os.path.dirname(old), exist_ok=True)
        shutil.rmtree(old, ignore_errors=True)
        if os.path.exists(t):
            shutil.move(t, old)
        shutil.move(s, t)
        open(os.path.join(t, ".allclips"), "w").close()
    print(f"{min(i + CHUNK, len(todo))}/{len(todo)}", flush=True)

# Sun Temple / True Sun God clips grafted from the original prefabs (export_tower_assets.py).
src = open("tools/assets/export_tower_assets.py", encoding="utf-8").read()
grafts = eval(src[src.index("GRAFTS = [") + len("GRAFTS = "):src.index("]\nfor t, target, source, only, clips, flags in GRAFTS")] + "]")
for t, target, source, only, clips, flags in grafts:
    s = f"local_extracted/models/_originals/{only or source}"
    d = f"local_extracted/models/{t}/tiers/{target}"
    if os.path.exists(f"{s}/meta.json") and os.path.isdir(d):
        subprocess.run([PY, "-I", "tools/assets/graft_clips.py", *flags, d, s, *clips])
print("done", flush=True)
