"""Audit the exported tower visuals against what the game can show.

    python -I tools/assets/audit_visuals.py [local_extracted dir]

For every tower model folder under <dir>/models (the ones game.json's towers use): each valid upgrade
combination (two paths at most, one of them at most 2; heroes by level folder), whether
tiers/index.json names a display for it, whether that folder exists with a mesh (verts.bin, idx.bin,
meta.json) and textures, and which animation clips it has (meta.json "anims"). Prints a summary table
(Markdown) of the gaps per tower.
"""
import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))


def combos():
    out = []
    for a in range(6):
        for b in range(6):
            for c in range(6):
                t = sorted([a, b, c])
                if t[0] != 0 or t[1] > 2:
                    continue
                out.append(f"{a}{b}{c}")
    return out


def check_dir(d):
    """(ok, problems, clips) for a model folder."""
    probs = []
    meta = None
    for f in ("meta.json", "verts.bin", "idx.bin"):
        if not os.path.isfile(os.path.join(d, f)):
            probs.append(f"no {f}")
    if os.path.isfile(os.path.join(d, "meta.json")):
        try:
            meta = json.load(open(os.path.join(d, "meta.json"), encoding="utf-8"))
        except Exception as e:  # noqa: BLE001
            probs.append(f"bad meta ({e})")
    clips = []
    if meta:
        clips = [a.get("name") for a in meta.get("anims", [])]
        for p in meta.get("parts", []):
            tex = p.get("texture")
            if tex and not os.path.isfile(os.path.normpath(os.path.join(d, tex))):
                probs.append(f"missing texture for {p.get('name')}")
        if not meta.get("parts"):
            probs.append("no parts")
    return (not probs, probs, clips)


def main():
    base = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "local_extracted")
    game = json.load(open(os.path.join(base, "game", "game.json"), encoding="utf-8"))
    towers = game.get("towers") or {}
    names = list(towers.keys()) if isinstance(towers, dict) else [t.get("id") for t in towers]
    rows = []
    for name in names:
        t = towers[name] if isinstance(towers, dict) else next(x for x in towers if x.get("id") == name)
        model = (t or {}).get("model") or name
        mdir = os.path.join(base, "models", model)
        if not os.path.isdir(mdir):
            rows.append((name, "no model folder", "", ""))
            continue
        ok, probs, clips = check_dir(mdir)
        idx_path = os.path.join(mdir, "tiers", "index.json")
        idx = json.load(open(idx_path, encoding="utf-8")) if os.path.isfile(idx_path) else {}
        # Heroes are indexed by level, not by a three-digit path key.
        is_hero = bool(idx) and not any(len(k) == 3 and k.isdigit() for k in idx)
        missing_keys, broken, noclip, noidle, noattack = [], [], [], [], []
        keys = sorted(idx) if is_hero else combos()
        for k in keys:
            if k not in idx:
                missing_keys.append(k)
                continue
            d = os.path.join(mdir, "tiers", idx[k])
            if not os.path.isdir(d):
                broken.append(f"{k}->{idx[k]} (no folder)")
                continue
            okd, pd, cl = check_dir(d)
            if not okd:
                broken.append(f"{k}->{idx[k]}: " + ", ".join(pd))
            if not cl:
                noclip.append(k)
            elif not any(c and c.endswith("Idle") for c in cl):
                noidle.append(k)
            if cl and not any(c and (c.endswith("Attack") or c.startswith("Attack")) for c in cl):
                noattack.append(k)
        notes = []
        if probs:
            notes.append("base: " + ", ".join(probs))
        if not clips:
            notes.append("base has no clips")
        rows.append((name, f"{len(keys) - len(missing_keys)}/{len(keys)} tiers indexed" + (f"; missing {', '.join(missing_keys[:12])}{'...' if len(missing_keys) > 12 else ''}" if missing_keys else ""),
                     "; ".join(broken[:6]) + ("..." if len(broken) > 6 else ""),
                     "; ".join(x for x in [("no clips: " + ", ".join(noclip[:10])) if noclip else "",
                                           ("no Idle clip: " + ", ".join(noidle[:10]) + ("..." if len(noidle) > 10 else "")) if noidle else "",
                                           ("no Attack clip: " + ", ".join(noattack[:10]) + ("..." if len(noattack) > 10 else "")) if noattack else "",
                                           "; ".join(notes)] if x)))
    print("| Tower | Tier displays | Broken folders | Animation / other |")
    print("|---|---|---|---|")
    for r in rows:
        print("| " + " | ".join(x.replace("|", "/") for x in r) + " |")


if __name__ == "__main__":
    main()
