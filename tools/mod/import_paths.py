"""Replace a map's exported path points with the game's own (a SimRecorder <map>.map dump).

    python -I tools/mod/import_paths.py research/dumps/recordings/Tutorial.map tutorial

The game builds each path's points from the scene's line nodes and curves (BezierLineNodes /
CurvedLineNodes: Bezier pieces, fitted cubic splines); the dump is that result, so the clone's
distances along the track (spawn, leak, segments, positions) match it exactly. Each dumped path
replaces the exported path whose first point is nearest its own (the game's y is the clone's -y).
Updates local_extracted/maps/<key>/map.json, and game.json's map when it is that map.
"""
import json
import math
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))


def load_offsets(path):
    for line in open(path, encoding="utf-8"):
        f = line.split()
        if f and f[0] == "offsets":
            return [int(x) for x in f[1:]]
    return []


def load_areas(path):
    """Map.areas from the dump: name -> (outline, holes), y flipped to our frame."""
    areas, cur = {}, None
    pt = lambda t: [float(t.split(",")[0]), -float(t.split(",")[1])]
    for line in open(path, encoding="utf-8"):
        f = line.split()
        if not f:
            continue
        if f[0] == "area":
            cur = " ".join(f[7:]).removeprefix("AreaModel_")
            areas[cur] = ([], [])
        elif f[0] == "poly" and cur is not None:
            areas[cur] = ([pt(t) for t in f[1:]], [])
        elif f[0] == "hole" and cur is not None:
            areas[cur][1].append([pt(t) for t in f[1:]])
    return areas


def patch_areas(m, areas):
    # The game's own outline points (exact), for the areas it has under the same name.
    for a in m.get("areas", []):
        got = areas.get(a.get("name"))
        if got:
            a["points"], a["holes"] = got


def load_dump(path):
    paths, cur = [], None
    for line in open(path, encoding="utf-8"):
        f = line.split()
        if not f:
            continue
        if f[0] == "path":
            cur = {"id": f[1], "points": []}
            paths.append(cur)
        elif f[0] == "p" and cur is not None:
            cur["points"].append([float(f[1]), -float(f[2])])
            cur.setdefault("dists", []).append(float(f[4]))
            cur.setdefault("speeds", []).append(float(f[5]))
            # PointInfo.disablePathTargeting (newer dumps): segments centred there can't be targeted.
            if len(f) > 9:
                cur.setdefault("no_target", []).append(f[9] == "True")
        elif f[0] == "q" and cur is not None:
            # Path.pointInfos (every CacheDistance): x, y in our frame.
            cur.setdefault("quick", []).extend((float(f[2]), -float(f[3])))
            # QuickPointInfo.rotation (degrees, game frame): a bloon's rotation on the path.
            cur.setdefault("quick_rot", []).append(float(f[5]))
    return paths


def patch(m, dumped):
    used = set()
    for dp in dumped:
        first = dp["points"][0]
        best = min((i for i in range(len(m["paths"])) if i not in used),
                   key=lambda i: math.dist(m["paths"][i]["points"][0][:2], first), default=None)
        if best is None:
            continue
        used.add(best)
        m["paths"][best]["points"] = dp["points"]
        m["paths"][best]["dists"] = dp["dists"]
        m["paths"][best]["speeds"] = dp["speeds"]
        if dp.get("quick"):
            m["paths"][best]["quick"] = dp["quick"]
        if dp.get("quick_rot"):
            m["paths"][best]["quick_rot"] = dp["quick_rot"]
        if dp.get("no_target"):
            m["paths"][best]["no_target"] = dp["no_target"]
    if m["paths"]:
        m["path"] = m["paths"][0]["points"]


def main():
    dump, key = sys.argv[1], sys.argv[2]
    dumped = load_dump(dump)
    mp = os.path.join(ROOT, "local_extracted", "maps", key, "map.json")
    if os.path.exists(mp):
        m = json.load(open(mp, encoding="utf-8"))
        patch(m, dumped)
        patch_areas(m, load_areas(dump))
        json.dump(m, open(mp, "w", encoding="utf-8"))
        print("patched", mp)
    gp = os.path.join(ROOT, "local_extracted", "game", "game.json")
    g = json.load(open(gp, encoding="utf-8"))
    offs = load_offsets(dump)
    if offs:
        # CollisionGrid.orderedNeighborOffsets (same for every standard-size map).
        g["grid_offsets"] = offs
    if g["map"].get("key") == key or offs:
        if g["map"].get("key") == key:
            patch(g["map"], dumped)
            patch_areas(g["map"], load_areas(dump))
        json.dump(g, open(gp, "w", encoding="utf-8"), indent=1)
        print("patched", gp)


if __name__ == "__main__":
    main()
