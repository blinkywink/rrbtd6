"""Map coverage scenarios: a Dart Monkey 2-0-5 on a land spot 16-22 from the track.

    python -I tools/mod/gen_map_scenarios.py <dump dir> <MapName> [<MapName> ...]

Reads the game's own map dump (<dump dir>/<MapName>.map: SimRecorder's path points and areas) and
writes tools/mod/scenarios/map_<key>_dart_205_r40.json. A spot is placeable when it lies in a land
area (type 2) and at least the tower radius (6) away from every track / water / unplaceable
area (types 0, 1, 3) and from the map border; of those within 16-22 of a path point, the one
nearest the path's middle third (so bloons pass it for a while) is taken.
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


def placeable(p, areas):
    if not (-150 + RADIUS <= p[0] <= 150 - RADIUS and -116 + RADIUS <= p[1] <= 116 - RADIUS):
        return False
    if not any(a["active"] and a["type"] == 2 and in_area(p, a) for a in areas):
        return False
    for a in areas:
        if not a["active"] or a["type"] not in (0, 1, 3):
            continue
        if in_area(p, a) or any(edge_dist(p, poly) < RADIUS for poly in a["polys"]):
            return False
    return True


def pick(areas, paths):
    pts = [q for path in paths for q in path]
    total = max(q[2] for q in pts)
    best = None
    for gx in range(-144, 145, 2):
        for gy in range(-110, 111, 2):
            p = (float(gx), float(gy))
            near = min(pts, key=lambda q: (q[0] - p[0]) ** 2 + (q[1] - p[1]) ** 2)
            d = math.hypot(near[0] - p[0], near[1] - p[1])
            if not 16.0 <= d <= 22.0 or not placeable(p, areas):
                continue
            score = abs(near[2] / total - 0.5)
            if best is None or score < best[0]:
                best = (score, p)
    return best and best[1]


def main():
    dump_dir, names = sys.argv[1], sys.argv[2:]
    for name in names:
        areas, paths = load(os.path.join(dump_dir, name + ".map"))
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
