"""Re-export the models that have one-bone-per-vertex skinned parts.

export_animated.py used to export SkinnedMeshRenderers without a BlendWeight channel (each vertex
follows one bone fully: 2,385 tower parts: hats, guns, sticks, the Sun Temple's doors...) as
static meshes frozen in their bind pose. This re-exports every tower / hero tier model (display
plans), sub-tower, air unit and fan model into a scratch folder, and swaps a model in only when
the new export skins parts the old one didn't (then compacts it like compact_anims.py). Models
that change nothing are left alone, so later post-steps on them (graft_clips.py) survive; run
export_tower_assets.py's grafts again for the towers listed at the end.

Usage: tools/.venv/Scripts/python.exe tools/assets/reexport_skinned.py <game install> [TowerId...]
       ... <game install> --retry <log>   only the models that FAILed in an earlier run's output,
       one at a time (big hero models run out of memory with four exports at once).
"""
import concurrent.futures
import json
import os
import shutil
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(__file__))
from compact_anims import convert  # noqa: E402

GAME = sys.argv[1]
RETRY = sys.argv[3] if len(sys.argv) > 3 and sys.argv[2] == "--retry" else None
ONLY = set() if RETRY else set(sys.argv[2:])
MODELS = "local_extracted/models"
SCRATCH = f"{MODELS}/_reexport"
paths = json.load(open("local_extracted/all_container_paths.json"))


def jobs():
    """(tower, target dir, bundle glob, prefab) for every exported model with a known source."""
    out, seen = [], set()
    for f in sorted(os.listdir("local_extracted/display_plans")):
        tower = f[:-5]
        if ONLY and tower not in ONLY:
            continue
        plan = json.load(open(f"local_extracted/display_plans/{f}"))
        for t in plan["towers"].values():
            for d in [t] + t.get("attack_displays", []):
                stem = os.path.splitext(os.path.basename(d["prefab"]))[0].strip().replace(" ", "_")
                target = f"{MODELS}/{tower}/tiers/{stem}"
                if target in seen or not os.path.exists(f"{target}/meta.json"):
                    continue
                seen.add(target)
                out.append((tower, target, d["bundle"].rsplit("_", 1)[0], d["prefab"]))
    # Sub-towers / air units / fans: export_tower_assets.py finds their prefab by name.
    for kind in ("subtowers", "air", "fans"):
        for n in sorted(os.listdir(f"{MODELS}/{kind}")) if os.path.isdir(f"{MODELS}/{kind}") else []:
            cand = sorted(p for p in paths if p.endswith(f"/{n}.prefab"))
            if cand and os.path.exists(f"{MODELS}/{kind}/{n}/meta.json") and not ONLY:
                out.append((kind, f"{MODELS}/{kind}/{n}", paths[cand[0]].rsplit("_", 1)[0], cand[0]))
    return out


def skinned(meta):
    return sum(1 for p in meta["parts"] if p.get("skinned"))


def one(job):
    try:
        return one_inner(job)
    except Exception as e:  # keep going: report it with the model
        return job[0], job[1], f"FAIL {type(e).__name__}: {e}"[:200]


def one_inner(job):
    tower, target, bundle, prefab = job
    tmp = f"{SCRATCH}/{target[len(MODELS) + 1:]}"
    shutil.rmtree(tmp, ignore_errors=True)
    r = subprocess.run([sys.executable, "tools/assets/export_animated.py", GAME, tmp, bundle, prefab],
                       capture_output=True, text=True)
    if r.returncode != 0 or not os.path.exists(f"{tmp}/meta.json"):
        shutil.rmtree(tmp, ignore_errors=True)
        return tower, target, "FAIL " + ((r.stderr.strip().splitlines() or ["?"])[-1])[:160]
    old, new = json.load(open(f"{target}/meta.json")), json.load(open(f"{tmp}/meta.json"))
    if skinned(new) <= skinned(old):
        shutil.rmtree(tmp, ignore_errors=True)
        return tower, target, "same"
    convert(tmp)
    trash = target + ".old"
    shutil.rmtree(trash, ignore_errors=True)
    # Windows may hold a file briefly (indexer, antivirus, a running game): retry, and put the
    # old model back if the new one can't be moved in.
    for attempt in range(20):
        try:
            if os.path.exists(target):
                os.replace(target, trash)
            os.replace(tmp, target)
            break
        except PermissionError:
            time.sleep(1.5)
    else:
        if not os.path.exists(target) and os.path.exists(trash):
            os.replace(trash, target)
        return tower, target, "FAIL locked"
    shutil.rmtree(trash, ignore_errors=True)
    return tower, target, f"updated ({skinned(old)} -> {skinned(new)} skinned parts)"


def main():
    js = jobs()
    workers = 4
    if RETRY:
        failed = {line.split()[0] for line in open(RETRY, encoding="utf-8") if " FAIL" in line}
        js = [j for j in js if j[1] in failed]
        workers = 1
    print(len(js), "models", flush=True)
    changed = set()
    with concurrent.futures.ThreadPoolExecutor(workers) as ex:
        for tower, target, res in ex.map(one, js):
            print(target, res, flush=True)
            if res.startswith("updated"):
                changed.add(tower)
    shutil.rmtree(SCRATCH, ignore_errors=True)
    print("towers changed:", " ".join(sorted(changed)))


if __name__ == "__main__":
    main()
