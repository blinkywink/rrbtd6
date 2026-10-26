"""Export one map (track, placement areas, terrain image) from its scene bundle.

Usage:
    tools/.venv/Scripts/python.exe tools/assets/export_map.py <game install> <bundle key, e.g. tutorial> <out_dir>

Writes <out_dir>/<key>/map.json and terrain.png. Coordinates are Unity world
(x, z) on the ground plane. Proprietary data: keep in local_extracted/.

APPROXIMATION (H-PATH-2): `Curve` nodes are sampled here as Bezier curves
through their child control points; the game's own path sampling has not
been recovered yet.
"""
import glob
import json
import math
import os
import sys

import UnityPy
from UnityPy.helpers import TypeTreeHelper

TypeTreeHelper.read_typetree_boost = False


def qrot(q, v):
    x, y, z, w = q["x"], q["y"], q["z"], q["w"]
    vx, vy, vz = v
    # t = 2 * cross(q.xyz, v); v' = v + w*t + cross(q.xyz, t)
    tx, ty, tz = 2 * (y * vz - z * vy), 2 * (z * vx - x * vz), 2 * (x * vy - y * vx)
    return (vx + w * tx + (y * tz - z * ty), vy + w * ty + (z * tx - x * tz), vz + w * tz + (x * ty - y * tx))


def bezier(pts, n):
    out = []
    for k in range(n + 1):
        t = k / n
        p = list(pts)
        while len(p) > 1:
            p = [((1 - t) * a[0] + t * b[0], (1 - t) * a[1] + t * b[1]) for a, b in zip(p, p[1:])]
        out.append(p[0])
    return out


def main():
    game, key, out = sys.argv[1:4]
    aa = os.path.join(game, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
    env = UnityPy.load(glob.glob(os.path.join(aa, f"map_{key}_*.bundle"))[0])
    objs = {o.path_id: o for o in env.objects}
    cache = {}

    def tt(pid):
        if pid not in cache:
            cache[pid] = objs[pid].read_typetree()
        return cache[pid]

    def comp(go, typename):
        for c in go["m_Component"]:
            o = objs.get(c["component"]["m_PathID"])
            if o is not None and o.type.name == typename:
                return c["component"]["m_PathID"]
        return None

    def world(tr_id, local=(0.0, 0.0, 0.0)):
        """World position of `local` expressed in transform tr_id's space."""
        p = local
        while tr_id:
            tr = tt(tr_id)
            s = tr["m_LocalScale"]
            p = (p[0] * s["x"], p[1] * s["y"], p[2] * s["z"])
            p = qrot(tr["m_LocalRotation"], p)
            lp = tr["m_LocalPosition"]
            p = (p[0] + lp["x"], p[1] + lp["y"], p[2] + lp["z"])
            tr_id = tr["m_Father"]["m_PathID"]
        return p

    path_pts, areas, terrain = [], [], None
    raw_path = []  # ordered child entries, for the engine's exact PathModel build
    # One entry per entrySplitter (a map's separate tracks), with its GameObject's active flag.
    paths = []
    for o in env.objects:
        if o.type.name != "MonoBehaviour":
            continue
        t = tt(o.path_id)
        if "entrySplitter" in t:
            go = tt(t["m_GameObject"]["m_PathID"])
            root = tt(comp(go, "Transform"))
            path_pts = []
            for ch in root["m_Children"]:
                cid = ch["m_PathID"]
                c = tt(cid)
                g = tt(c["m_GameObject"]["m_PathID"])
                mbs = []
                for comp_ref in g["m_Component"]:
                    co = objs.get(comp_ref["component"]["m_PathID"])
                    if co is not None and co.type.name == "MonoBehaviour":
                        mt = tt(co.path_id)
                        script = co.assets_file.objects.get(mt["m_Script"]["m_PathID"]) if mt["m_Script"]["m_FileID"] == 0 else None
                        mbs.append({k: v for k, v in mt.items() if not k.startswith("m_")})
                raw_path.append({
                    "name": g["m_Name"], "active": bool(g.get("m_IsActive", 1)),
                    "kind": "Curve" if c["m_Children"] else "LineNode",
                    "world": [list(world(k["m_PathID"])) for k in c["m_Children"]] if c["m_Children"] else [list(world(cid))],
                    "child_names": [tt(tt(k["m_PathID"])["m_GameObject"]["m_PathID"])["m_Name"] for k in c["m_Children"]],
                    "monobehaviours": mbs,
                })
                if c["m_Children"]:
                    ctrl = [world(k["m_PathID"]) for k in c["m_Children"]]
                    seg = bezier([(p[0], p[2]) for p in ctrl], 16)
                    path_pts.extend(seg if not path_pts else seg[1:] if path_pts[-1] == seg[0] else seg)
                else:
                    p = world(cid)
                    path_pts.append((p[0], p[2]))
                    g  # node name unused
            # Path component: its splitters (Splitter.paths lists, resolved below). The exit
            # splitter's paths are where bloons go on (Splitter.Rout, round-robin); a path with no
            # entry splitter is a spawn path.
            paths.append({"name": go["m_Name"], "active": bool(go.get("m_IsActive", 1)) and bool(t.get("isActive", 1)),
                          "points": path_pts, "id": o.path_id,
                          "entry_splitter": t["entrySplitter"]["m_PathID"], "exit_splitter": t["exitSplitter"]["m_PathID"]})
        elif "polygons" in t:
            go_tr = comp(tt(t["m_GameObject"]["m_PathID"]), "Transform")
            # One area per Area component: AreaModel.IsPointInside is "inside polygons[0] and in
            # none of the others" (holes; Area.ToModel copies polygons[1..] into the holes array).
            rings = []
            for poly in t["polygons"]:
                pts = [world(go_tr, (q["x"], q["y"], q["z"])) for q in poly["points"]]
                # Vector3.ToSimulationSpace(tilt 60): sim.y = -z - y/tan(60). In our Unity-ground
                # (x, z) frame that is z' = z + y/tan(60), so raised areas (tree blockers sit at
                # y=100) land where the game puts them.
                tilt = 1.0 / math.tan(math.radians(60.0))
                # Area.ToModel: height = transform.position.y. Blocker circles are authored in
                # sim space (x, sim.y) and used untransformed (GatherMapAreasAndBlockers), so
                # here z' = -sim.y.
                rings.append([(p[0], p[2] + p[1] * tilt) for p in pts])
            if rings:
                areas.append({"type": t["type"], "isBlocker": bool(t["isBlocker"]),
                              "name": tt(t["m_GameObject"]["m_PathID"])["m_Name"],
                              "height": world(go_tr)[1],
                              "renderHeightOffset": t.get("renderHeightOffset", 0.0),
                              "isInputBlocking": bool(t.get("isInputBlocking", 0)),
                              "isDisabled": bool(t.get("isDisabled", 0)),
                              "blockers": [(c["position"]["x"], -c["position"]["y"], c["radius"])
                                           for c in (t.get("blockerCircles") or [])] if t["isBlocker"] else [],
                              "points": rings[0], "holes": rings[1:]})

    # Terrain: the largest GameObject whose renderer uses the *TerrainTexture material (props
    # such as Workshop's chutes share it).
    # Its mesh usually lives in another bundle (resolved through the CAB index).
    # Some maps (Sanctuary, Infernal, ...) have no "*Terrain*" texture in their bundle: then
    # there's no 2D terrain image (the renderer uses the 3D scene and the game's world bounds).
    tex_obj = next((o for o in env.objects if o.type.name == "Texture2D" and "Terrain" in o.read().m_Name), None)
    tex_name = tex_obj.read().m_Name if tex_obj else None
    cab_index = json.load(open(os.path.join(out, "..", "cab_index.json")))
    candidates = []
    for o in env.objects if tex_obj else []:
        if o.type.name != "MeshRenderer":
            continue
        af = o.assets_file
        r = o.read_typetree()
        mats = []
        for mref in r["m_Materials"]:
            if mref["m_FileID"] == 0:
                mo = af.objects.get(mref["m_PathID"])
            else:
                mo = None
                ext = af.externals[mref["m_FileID"] - 1].path.split("/")[-1]
                for f2 in env.files.values():
                    for sub in getattr(f2, "files", {}).values():
                        if getattr(sub, "name", "") == ext:
                            mo = sub.objects.get(mref["m_PathID"])
            if mo is not None and mo.type.name == "Material":
                mats.append(mo.read_typetree()["m_Name"])
        if tex_name not in mats:
            continue
        go = af.objects[r["m_GameObject"]["m_PathID"]].read_typetree()
        mf = next(af.objects[c["component"]["m_PathID"]] for c in go["m_Component"]
                  if af.objects[c["component"]["m_PathID"]].type.name == "MeshFilter")
        ref = mf.read_typetree()["m_Mesh"]
        cab = af.externals[ref["m_FileID"] - 1].path.split("/")[-1] if ref["m_FileID"] else None
        if cab is not None and cab not in cab_index:
            # A built-in mesh (Castle Revenge's scrolling terrain plane): no fixed terrain
            # rectangle; the renderer uses the 3D scene and the world bounds.
            break
        else:
            env2 = UnityPy.load(os.path.join(aa, cab_index[cab])) if cab else env
            # The mesh in that CAB file (path ids repeat across a bundle's files).
            files2 = [sub for f2 in env2.files.values() for nm, sub in getattr(f2, "files", {}).items()
                      if hasattr(sub, "objects") and (cab is None or nm == cab)]
            if cab is None:
                mesh_obj = af.objects[ref["m_PathID"]]
            else:
                mesh_obj = next((sub.objects[ref["m_PathID"]] for sub in files2 if ref["m_PathID"] in sub.objects),
                                None) or next(o2 for o2 in env2.objects if o2.path_id == ref["m_PathID"])
            mesh = mesh_obj.read_typetree()
            from UnityPy.helpers.MeshHelper import MeshHandler
            mh = MeshHandler(mesh_obj.read())
            mh.process()
            us = [u[0] for u in mh.m_UV0]
            vs = [u[1] for u in mh.m_UV0]
            uv_rect = (min(us), min(vs), max(us), max(vs))
            c, e = mesh["m_LocalAABB"]["m_Center"], mesh["m_LocalAABB"]["m_Extent"]
        tr = comp(go, "Transform")
        a = world(tr, (c["x"] - e["x"], c["y"], c["z"] - e["z"]))
        b = world(tr, (c["x"] + e["x"], c["y"], c["z"] + e["z"]))
        candidates.append({"min": (min(a[0], b[0]), min(a[2], b[2])), "max": (max(a[0], b[0]), max(a[2], b[2])),
                           "mesh": mesh["m_Name"], "object": go["m_Name"], "uv_rect": uv_rect})
    area = lambda t: (t["max"][0] - t["min"][0]) * (t["max"][1] - t["min"][1])
    terrain = max(candidates, key=area) if candidates else None
    # Smaller than the play area (300 x 232): a prop that shares the texture, not the ground.
    if terrain and area(terrain) < 300 * 232:
        print("terrain candidate too small, none used:", terrain["object"])
        terrain = None
    d = os.path.join(out, key)
    os.makedirs(d, exist_ok=True)
    tex = tex_obj.read() if tex_obj else None
    if tex is not None:
        img = tex.image
        if terrain and "uv_rect" in terrain:
            # Crop to the part of the texture the terrain mesh shows (v is bottom-up).
            u0, v0, u1, v1 = terrain["uv_rect"]
            W, H = img.size
            img = img.crop((round(u0 * W), round((1 - v1) * H), round(u1 * W), round((1 - v0) * H)))
        img.convert("RGB").save(os.path.join(d, "terrain.png"))
    # Order the paths as GetComponentsInChildren would: depth-first over the transform
    # hierarchy (children in m_Children order), roots in object order.
    order, stack = {}, []
    for o in env.objects:
        if o.type.name == "Transform":
            tr = tt(o.path_id)
            if not tr.get("m_Father", {}).get("m_PathID"):
                stack.append(o.path_id)
    roots = list(stack)
    def visit(tid):
        order[tid] = len(order)
        for ch in tt(tid).get("m_Children", []):
            if ch["m_PathID"] in objs:
                visit(ch["m_PathID"])
    for r in roots:
        visit(r)
    for q in paths:
        go = tt(tt(q["id"])["m_GameObject"]["m_PathID"])
        q["_order"] = order.get(comp(go, "Transform"), 1 << 30)
    paths.sort(key=lambda q: q.pop("_order"))

    def splitter_paths(sid):
        if not sid or sid not in objs:
            return []
        return [r["m_PathID"] for r in tt(sid).get("paths", []) if r.get("m_PathID")]
    for q in paths:
        q["entry"] = splitter_paths(q["entry_splitter"])
        q["exit"] = splitter_paths(q["exit_splitter"])
    # "path" stays the first active track (single-track readers); "paths" has them all.
    path_pts = next((q["points"] for q in paths if q["active"]), paths[0]["points"] if paths else [])
    length = sum(math.dist(a, b) for a, b in zip(path_pts, path_pts[1:]))
    json.dump({"key": key, "raw_path": raw_path, "path": path_pts, "paths": paths, "path_length": length, "areas": areas, "terrain": terrain,
               "terrain_size": [tex.m_Width, tex.m_Height] if tex else [0, 0]}, open(os.path.join(d, "map.json"), "w"), indent=0)
    print(f"{key}: {len(paths)} paths ({sum(q['active'] for q in paths)} active), first {len(path_pts)} pts, length {length:.1f}; {len(areas)} areas; terrain {terrain}")


if __name__ == "__main__":
    main()
