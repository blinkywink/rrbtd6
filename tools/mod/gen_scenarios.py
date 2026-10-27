"""Write SimRecorder scenarios for a batch of towers into tools/mod/scenarios/.

    python -I tools/mod/gen_scenarios.py [--list out.txt]

Per tower: the base tower (and First/Last/Close/Strong where it has targeting), every
single-path tier, a sample of crosspaths, targeting on two high tiers, and mid-round upgrades.
Plus the remaining Dart / Boomerang gaps. Existing scenario files are left as they are.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "scenarios")
SPOT = (-112.815414, -30.9)
SPOT2 = [(-96.815414, -30.9), (-112.815414, -46.9), (-101.50171, -42.213707)]


def scenario(name, towers, rnd, **extra):
    s = {"name": name, "map": "Tutorial", "difficulty": "Hard", "mode": "Clicks", "frames": 3600,
         "quit": True, "round": rnd, "towers": towers}
    s.update(extra)
    return s


def tower(tid, tiers="000", pos=SPOT, **extra):
    t = {"id": tid if tiers == "000" else f"{tid}-{tiers}", "x": pos[0], "y": pos[1]}
    t.update(extra)
    return t


def round_for(tiers):
    top = max(int(c) for c in tiers)
    return 20 if top <= 2 else 30 if top == 3 else 40


def tower_set(prefix, tid, targeting, high):
    out = []
    out.append(scenario(f"{prefix}_000", [tower(tid)], 10))
    if targeting:
        for mode in ("Last", "Close", "Strong"):
            out.append(scenario(f"{prefix}_000_{mode.lower()}", [tower(tid, target=mode)], 10))
    for path in range(3):
        for tier in range(1, 6):
            t = ["0", "0", "0"]
            t[path] = str(tier)
            tiers = "".join(t)
            extra = {"ability": 120} if tier >= 4 else {}
            out.append(scenario(f"{prefix}_{tiers}", [tower(tid, tiers)], round_for(tiers), **extra))
    for tiers in ("520", "502", "250", "052", "205", "025", "320", "032"):
        out.append(scenario(f"{prefix}_{tiers}", [tower(tid, tiers)], 40, ability=120))
    if targeting:
        for tiers in high:
            for mode in ("Last", "Close", "Strong"):
                out.append(scenario(f"{prefix}_{tiers}_{mode.lower()}", [tower(tid, tiers, target=mode)], 40, ability=120))
    ups = [{"frame": 200, "path": 0}, {"frame": 400, "path": 1}, {"frame": 600, "path": 0},
           {"frame": 900, "path": 0}, {"frame": 1300, "path": 0}, {"frame": 1800, "path": 0}]
    out.append(scenario(f"{prefix}_upgrades_r30", [tower(tid, upgrades=ups)], 30, cash=10000000))
    return out


def gaps():
    out = []
    for tid, prefix, highs in (("DartMonkey", "dart", ("205", "420")), ("BoomerangMonkey", "boomerang", ("502", "025"))):
        for tiers in highs:
            for mode in ("Last", "Close", "Strong"):
                out.append(scenario(f"{prefix}_{tiers}_{mode.lower()}", [tower(tid, tiers, target=mode)], 40))
    for tiers in ("020", "040", "420", "052"):
        out.append(scenario(f"boomerang_{tiers}_left", [tower("BoomerangMonkey", tiers, flip=True)], 40, **({"ability": 120} if "4" in tiers else {})))
    for tiers in ("012", "102", "120", "210", "310", "410"):
        out.append(scenario(f"dart_{tiers}", [tower("DartMonkey", tiers)], round_for(tiers)))
    out.append(scenario("mix_dart_boomerang_r40", [
        tower("DartMonkey", "302"), tower("BoomerangMonkey", "203", SPOT2[0], target="Strong"),
        tower("DartMonkey", "023", SPOT2[1], target="Last"), tower("BoomerangMonkey", "320", SPOT2[2], flip=True)], 40))
    out.append(scenario("mix_dart_boomerang_r63", [
        tower("DartMonkey", "420"), tower("BoomerangMonkey", "502", SPOT2[0], target="Close"),
        tower("DartMonkey", "025", SPOT2[1]), tower("BoomerangMonkey", "052", SPOT2[2])], 63))
    return out


TOWERS = [
    ("bomb", "BombShooter", True, ("502", "420")),
    ("tack", "TackShooter", False, ()),
    ("ice", "IceMonkey", False, ()),
    ("glue", "GlueGunner", True, ("205", "420")),
    ("desperado", "Desperado", True, ("502", "420")),
    ("sniper", "SniperMonkey", True, ("205", "420")),
]


def main():
    made = []
    sets = [("", gaps())] + [(p, tower_set(p, t, tg, hi)) for p, t, tg, hi in TOWERS]
    for group, scs in sets:
        for s in scs:
            path = os.path.join(OUT, s["name"] + ".json")
            if os.path.exists(path):
                continue
            with open(path, "w") as f:
                json.dump(s, f)
            made.append((group, path))
    if "--list" in sys.argv:
        with open(sys.argv[sys.argv.index("--list") + 1], "w") as f:
            for group, path in made:
                f.write(f"{group}\t{path}\n")
    print(len(made), "scenarios written")


if __name__ == "__main__":
    main()
