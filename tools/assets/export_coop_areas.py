"""Export the co-op map divisions (Game.coopLayouts) from the UI scenes bundle.

Usage:
    tools/.venv/Scripts/python.exe tools/assets/export_coop_areas.py <game install> <out json>

The game's co-op areas are the same for every map: seven CoopAreaDivisionType objects
(CoOpHorizontal, ...), each with CoopAreaLayout children (layout: AreaLayoutType PLAYERS_2 /
PLAYERS_4 / ...) whose CoopArea children (player: P1..P4) carry Area polygons
(Game.GetCoopAreaLayoutsForDivision @0x181365820 picks the division's layouts; MapLoader builds
CoopAreaModels from them). Points are written as export_map.py writes map areas: Unity world
(x, z + y / tan 60), the frame the clone's map polygons use.
Proprietary data: keep the output in local_extracted/.
"""
import glob
import json
import math
import os
import sys

import UnityPy

# CoopDivision (MIN = -1): FREE_FOR_ALL 0, HORIZONTAL 1, VERTICAL 2, DIAGONAL_LR 3, DIAGONAL_RL 4,
# DEFAULT 5, RADIOACTIVE 6, STAIRS 7.
DIVISIONS = {0: "FreeForAll", 1: "Horizontal", 2: "Vertical", 3: "DiagonalLR", 4: "DiagonalRL", 5: "Default", 6: "RadioActive", 7: "Stairs"}


def qrot(q, v):
    x, y, z, w = q["x"], q["y"], q["z"], q["w"]
    vx, vy, vz = v
    tx, ty, tz = 2 * (y * vz - z * vy), 2 * (z * vx - x * vz), 2 * (x * vy - y * vx)
    return (vx + w * tx + (y * tz - z * ty), vy + w * ty + (z * tx - x * tz), vz + w * tz + (x * ty - y * tx))


def main():
    game, out = sys.argv[1:3]
    path = glob.glob(os.path.join(game, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full", "ui_scenes_all_*.bundle"))[0]
    env = UnityPy.load(path)
    objs = {(o.assets_file.name, o.path_id): o for o in env.objects}
    cache = {}

    def tt(f, pid):
        k = (f, pid)
        if k not in cache:
            cache[k] = objs[k].read_typetree()
        return cache[k]

    def comps(f, go):
        for c in go["m_Component"]:
            o = objs.get((f, c["component"]["m_PathID"]))
            if o is not None:
                yield o

    def transform(f, go):
        return next(o for o in comps(f, go) if o.type.name in ("Transform", "RectTransform"))

    def world(f, tr_pid, local):
        p = local
        tid = tr_pid
        while tid:
            tr = tt(f, tid)
            s, q, t = tr["m_LocalScale"], tr["m_LocalRotation"], tr["m_LocalPosition"]
            p = (p[0] * s["x"], p[1] * s["y"], p[2] * s["z"])
            p = qrot(q, p)
            p = (p[0] + t["x"], p[1] + t["y"], p[2] + t["z"])
            tid = tr["m_Father"]["m_PathID"]
        return p

    tilt = 1.0 / math.tan(math.radians(60.0))
    result = {}
    for o in env.objects:
        if o.type.name != "MonoBehaviour":
            continue
        t = o.read_typetree()
        keys = [k for k in t if not k.startswith("m_")]
        if keys != ["divisionType"]:
            continue
        f = o.assets_file.name
        go = tt(f, t["m_GameObject"]["m_PathID"])
        name = DIVISIONS.get(t["divisionType"], str(t["divisionType"]))
        layouts = []
        root_tr = transform(f, go)

        def walk(tr_pid, found):
            tr = tt(f, tr_pid)
            g = tt(f, tr["m_GameObject"]["m_PathID"])
            info = {"name": g["m_Name"], "active": bool(g.get("m_IsActive", 1))}
            for c in comps(f, g):
                if c.type.name != "MonoBehaviour":
                    continue
                ct = c.read_typetree()
                ck = [k for k in ct if not k.startswith("m_")]
                if ck == ["layout"]:
                    info["layout"] = ct["layout"]
                elif ck == ["player"]:
                    info["player"] = ct["player"]
                elif "polygons" in ct:
                    rings = []
                    for poly in ct["polygons"]:
                        pts = [world(f, tr_pid, (q["x"], q["y"], q["z"])) for q in poly["points"]]
                        rings.append([[round(p[0], 4), round(p[2] + p[1] * tilt, 4)] for p in pts])
                    info["polygons"] = rings
                    info["area_type"] = ct.get("type")
            found.append(info)
            for ch in tr["m_Children"]:
                walk(ch["m_PathID"], found)

        nodes = []
        for ch in tt(f, root_tr.path_id)["m_Children"]:
            sub = []
            walk(ch["m_PathID"], sub)
            lay = next((n for n in sub if "layout" in n), None)
            if lay is None:
                continue
            players = [{"player": n["player"], "name": n["name"], "polygons": n.get("polygons", [])} for n in sub if "player" in n]
            # Polygons may sit on the CoopArea's children rather than on it.
            if any(not p["polygons"] for p in players):
                cur = None
                players = []
                for n in sub:
                    if "player" in n:
                        cur = {"player": n["player"], "name": n["name"], "polygons": list(n.get("polygons", []))}
                        players.append(cur)
                    elif "polygons" in n and cur is not None:
                        cur["polygons"] += n["polygons"]
            layouts.append({"name": lay["name"], "layout": lay["layout"], "active": lay["active"], "players": players})
        result[name] = {"division": t["divisionType"], "object": go["m_Name"], "layouts": layouts}
    os.makedirs(os.path.dirname(out), exist_ok=True)
    json.dump(result, open(out, "w"), indent=1)
    for k, v in result.items():
        print(k, [(l["name"], l["layout"], [(p["name"], len(p["polygons"])) for p in l["players"]]) for l in v["layouts"]])


if __name__ == "__main__":
    main()
