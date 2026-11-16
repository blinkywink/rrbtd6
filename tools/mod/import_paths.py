"""Replace a map's exported path points with the game's own (a SimRecorder <map>.map dump).

    python -I tools/mod/import_paths.py research/dumps/recordings/Tutorial.map tutorial

The game builds each path's points from the scene's line nodes and curves (BezierLineNodes /
CurvedLineNodes: Bezier pieces, fitted cubic splines); the dump is that result, so the clone's
distances along the track (spawn, leak, segments, positions) match it exactly. Each dumped path
replaces the exported path of its name (else the one whose first point is nearest its own) (the game's y is the clone's -y).
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
    """Map.areas from the dump, in order: (name, outline, holes), y flipped to our frame. Names can
    repeat (Cornfield's thirteen CornfieldBlocking1), so this is a list, not a dict."""
    areas, cur = [], None
    pt = lambda t: [float(t.split(",")[0]), -float(t.split(",")[1])]
    for line in open(path, encoding="utf-8"):
        f = line.split()
        if not f:
            continue
        if f[0] == "area":
            cur = [" ".join(f[7:]).removeprefix("AreaModel_"), [], []]
            areas.append(cur)
        elif f[0] == "poly" and cur is not None:
            cur[1] = [pt(t) for t in f[1:]]
        elif f[0] == "hole" and cur is not None:
            cur[2].append([pt(t) for t in f[1:]])
    return areas


def patch_areas(m, areas):
    # The game's own outline points (exact) and Map.areas' order (the dump lists them in the
    # simulation's order: ProjectileBlockerCollisionRebound takes the first blocker area crossed).
    # Each map area takes the unused dumped area of its name whose first point is nearest its own
    # (the asset export's outline), so repeated names keep their own outlines.
    used, match = set(), {}
    for i, a in enumerate(m.get("areas", [])):
        mine = (a.get("points") or [[0.0, 0.0]])[0]
        # (Names compared with whitespace collapsed: the dump's are space-split; MushroomGrotto's
        # "RocksTopLeft  (1)".)
        nm = " ".join((a.get("name") or "").split())
        cands = [k for k, d in enumerate(areas) if d[0] == nm and k not in used and d[1]]
        if not cands:
            continue
        k = min(cands, key=lambda k: math.dist(areas[k][1][0][:2], mine[:2]))
        used.add(k)
        match[i] = k
        a["points"], a["holes"] = areas[k][1], areas[k][2]
    if len(match) == len(m.get("areas", [])):
        order = sorted(range(len(m["areas"])), key=lambda i: match[i])
        m["areas"] = [m["areas"][i] for i in order]


def load_dump(path):
    paths, cur = [], None
    for line in open(path, encoding="utf-8"):
        f = line.split()
        if not f:
            continue
        if f[0] == "path":
            # (The name can hold spaces: Quarry's "PathStandard 1"; the flags start at "active=".)
            k = next((j for j, t in enumerate(f) if t.startswith("active=")), 2)
            cur = {"id": " ".join(f[1:k]), "points": []}
            # Path.isHidden: PathManager.GetPathSegmentsInRange (@0x180f6ba10) leaves out a path
            # only when it is hidden and not active.
            cur["hidden"] = "hidden=True" in f[k:]
            # Path.spawnDistance / leakDistance (-1: none), as the game has them.
            for t in f[k:]:
                if t.startswith(("spawn=", "leak=", "untilLeak=")):
                    cur[t.split("=")[0]] = float(t.split("=")[1])
            paths.append(cur)
        elif f[0] == "p" and cur is not None:
            cur["points"].append([float(f[1]), -float(f[2])])
            cur.setdefault("dists", []).append(float(f[4]))
            cur.setdefault("speeds", []).append(float(f[5]))
            # PointInfo.bloonsInvulnerable / moabsInvulnerable: Bloon.nonTargetable on that stretch.
            cur.setdefault("invuln", []).append(f[6] == "True")
            cur.setdefault("moab_invuln", []).append(f[7] == "True")
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
        # The path of the same name when there is exactly one; else the nearest first point
        # (BloonariusPrime's PathStandard and PathBoss start at the same point).
        named = [i for i in range(len(m["paths"])) if i not in used and m["paths"][i].get("name") == dp["id"]]
        best = named[0] if len(named) == 1 else min((i for i in range(len(m["paths"])) if i not in used),
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
        for k in ("invuln", "moab_invuln"):
            if any(dp.get(k, [])):
                m["paths"][best][k] = dp[k]
            else:
                m["paths"][best].pop(k, None)
        if dp.get("hidden"):
            m["paths"][best]["hidden"] = True
        else:
            m["paths"][best].pop("hidden", None)
        for k, mk in (("spawn", "spawn"), ("leak", "leak"), ("untilLeak", "until_leak")):
            if k in dp:
                m["paths"][best][mk] = dp[k]
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
