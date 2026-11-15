"""Map coverage scenarios: a Dart Monkey 2-0-5 on a land spot 16-22 from the track.

    python -I tools/mod/gen_map_scenarios.py <dump dir> <MapName> [<MapName> ...]

Reads the game's own map dump (<dump dir>/<MapName>.map: SimRecorder's path points and areas) and
writes tools/mod/scenarios/map_<key>_dart_205_r40.json. A spot is placeable when the first area (Map.areas
order) holding it and eight points round its footprint (radius 6) is land (type 2), off the border; of those within 16-22 of a path point (else 12-30, else 30-40: Spillway's wide channel), the one
nearest the path's middle third (so bloons pass it for a while) is taken.

Areas the map's logic (local_extracted/maps/<key>/map.json) destroys or creates with AfterRound events that
have fired by the start round (CheckIfTriggered at the start, triggerOnceIfRoundMissed catch-up) are applied
first (Erosion: round 40 starts with the round-18 and round-36 land gone).
"""
import json
import math
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
RADIUS = 6.0


def load(path):
    areas, paths, cur = [], [], None
    for line in open(path, encoding="utf-8"):
        f = line.split()
        if not f:
            continue
        if f[0] == "path":
            paths.append([])
        elif f[0] == "p" and paths:
            paths[-1].append((float(f[1]), float(f[2]), float(f[4])))
        elif f[0] == "area":
            cur = {"type": int(f[4]), "active": f[6] == "True", "polys": [], "holes": []}
            areas.append(cur)
        elif f[0] == "poly" and cur is not None:
            cur["polys"].append([tuple(map(float, t.split(","))) for t in f[1:]])
        elif f[0] == "hole" and cur is not None:
            cur["holes"].append([tuple(map(float, t.split(","))) for t in f[1:]])
    return areas, paths


def apply_logic(areas, key, start_round):
    """Marks the dump's areas (same order as map.json's, BaseArea extra at the end) inactive / active as
    AfterRound events fired by `start_round` leave them (STAND-IN: AfterRound triggers only)."""
    mj = os.path.join(ROOT, "local_extracted", "maps", key, "map.json")
    if not os.path.exists(mj):
        return
    d = json.load(open(mj, encoding="utf-8"))
    ids = [a.get("id") for a in d["areas"]]
    r = start_round - 1
    for ev in (d.get("logic") or {}).get("events", []):
        fired = any(t["kind"] == "AfterRound" and (r == t["round"] or (t.get("missed") and r > t["round"])
                    or (t["nth"] > 0 and r > t["round"] and (r - t["round"]) % t["nth"] == 0)) for t in ev["triggers"])
        if not (fired and ev.get("active", True)):
            continue
        for act in ev["actions"]:
            if act["kind"] == "Area":
                for i in act["ids"]:
                    if i in ids and ids.index(i) < len(areas):
                        areas[ids.index(i)]["active"] = act["create"]


def inside(p, poly):
    c, n = False, len(poly)
    for i in range(n):
        a, b = poly[i], poly[(i + 1) % n]
        if (a[1] > p[1]) != (b[1] > p[1]) and p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]:
            c = not c
    return c


def edge_dist(p, poly):
    best = 1e9
    for i in range(len(poly)):
        (ax, ay), (bx, by) = poly[i], poly[(i + 1) % len(poly)]
        dx, dy = bx - ax, by - ay
        t = 0.0 if dx == dy == 0 else max(0.0, min(1.0, ((p[0] - ax) * dx + (p[1] - ay) * dy) / (dx * dx + dy * dy)))
        best = min(best, math.hypot(p[0] - ax - dx * t, p[1] - ay - dy * t))
    return best


def in_area(p, a):
    return any(inside(p, poly) for poly in a["polys"]) and not any(inside(p, h) for h in a["holes"])


def area_at(p, areas):
    """The first active area in Map.areas order that holds p (BaseArea, last, is the fallback)."""
    return next((a for a in areas if a["active"] and in_area(p, a)), None)


def placeable(p, areas):
    if not (-150 + RADIUS <= p[0] <= 150 - RADIUS and -116 + RADIUS <= p[1] <= 116 - RADIUS):
        return False
    # The centre and eight points round the footprint all on land (type 2) by the first area.
    for k in range(9):
        q = p if k == 8 else (p[0] + RADIUS * math.cos(k * math.pi / 4), p[1] + RADIUS * math.sin(k * math.pi / 4))
        a = area_at(q, areas)
        if a is None or a["type"] != 2:
            return False
    return True


def pick(areas, paths):
    return pick_in(areas, paths, 16.0, 22.0) or pick_in(areas, paths, 12.0, 30.0) or pick_in(areas, paths, 30.0, 40.0)


def pick_in(areas, paths, lo, hi):
    pts = [q for path in paths for q in path]
    total = max(q[2] for q in pts)
    best = None
    for gx in range(-144, 145, 2):
        for gy in range(-110, 111, 2):
            p = (float(gx), float(gy))
            near = min(pts, key=lambda q: (q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2)
            d = math.hypot(near[0] - p[0], near[1] - p[1])
            if not lo <= d <= hi or not placeable(p, areas):
                continue
            score = abs(near[2] / total - 0.5)
            if best is None or score < best[0]:
                best = (score, p)
    return best and best[1]


def main():
    dump_dir, names = sys.argv[1], sys.argv[2:]
    for name in names:
        areas, paths = load(os.path.join(dump_dir, name + ".map"))
        apply_logic(areas, name.lower(), 40)
        p = pick(areas, paths)
        if p is None:
            print(name, "no spot")
            continue
        key = name.lower()
        sc = {"name": f"map_{key}_dart_205_r40", "map": name, "difficulty": "Hard", "mode": "Clicks", "round": 40,
              "frames": 3600, "seed": 777123, "towers": [{"id": "DartMonkey-205", "x": p[0], "y": p[1]}], "quit": True,
              "cash": 10000000}
        out = os.path.join(ROOT, "tools", "mod", "scenarios", f"map_{key}_dart_205_r40.json")
        json.dump(sc, open(out, "w", encoding="utf-8"))
        print(name, p, out)


if __name__ == "__main__":
    main()
