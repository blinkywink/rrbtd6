"""Batch-export every tier's display prefab (animated) per local_extracted/display_plans/<TowerId>.json.

Usage: tools/.venv/Scripts/python.exe tools/assets/export_tower_tiers.py <game install> <TowerId>...
Writes local_extracted/models/<TowerId>/tiers/<prefab-stem>/ (shared by tiers using the same prefab;
already exported ones are kept) and local_extracted/models/<TowerId>/tiers/index.json {tier: dir}.
"""
import json, os, subprocess, sys

game = sys.argv[1]
for tower in sys.argv[2:]:
    with open(f"local_extracted/display_plans/{tower}.json", encoding="utf-8") as plan_file:
        plan = json.load(plan_file)
    root = f"local_extracted/models/{tower}/tiers"
    os.makedirs(root, exist_ok=True)
    index, done, attack_index = {}, {}, {}
    # (tier, prefab, bundle) of every model to export: the tier's display, then its attack
    # displays (attack.json: tier -> [{dir, offset, scale}], drawn on the tower at its aim).
    jobs = []
    for tier, t in sorted(plan["towers"].items()):
        stem = os.path.splitext(os.path.basename(t["prefab"]))[0].replace(" ", "_")
        index[tier] = stem
        jobs.append((tier, t))
        for a in t.get("attack_displays", []):
            astem = os.path.splitext(os.path.basename(a["prefab"]))[0].strip().replace(" ", "_")
            attack_index.setdefault(tier, []).append({"dir": astem, "offset": a["offset"], "scale": a["scale"], "attack": a.get("attack", 0)})
            jobs.append((tier, a))
    for tier, t in jobs:
        stem = os.path.splitext(os.path.basename(t["prefab"]))[0].strip().replace(" ", "_")
        if stem in done:
            continue
        out = os.path.join(root, stem)
        meta = os.path.join(out, "meta.json")
        if os.path.exists(os.path.join(out, "outline.bin")) and os.path.exists(meta) and "shadow" in json.load(open(meta)):
            done[stem] = 0
            continue
        bundle_glob = t["bundle"].rsplit("_", 1)[0]
        r = subprocess.run([sys.executable, "tools/assets/export_animated.py", game, out, bundle_glob, t["prefab"]],
                           capture_output=True, text=True, encoding="utf-8", errors="replace")
        done[stem] = r.returncode
        print(tower, tier, stem, "ok" if r.returncode == 0 else "FAIL " + (r.stderr.strip().splitlines() or ["?"])[-1][:200],
              flush=True)
        if r.returncode:
            raise SystemExit(r.returncode)
    json.dump(index, open(os.path.join(root, "index.json"), "w"), indent=1)
    json.dump(attack_index, open(os.path.join(root, "attack.json"), "w"), indent=1)
    print(tower, "exported", sum(1 for v in done.values() if v == 0), "of", len(done))
