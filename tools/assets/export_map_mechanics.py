"""Everything a map scene does beyond its geometry, into local_extracted/maps/<key>/mechanics.json:
removeables (obstacles bought off the map), MapEvents with their triggers and actions, clickable
items, splitter rules, area buffs, and the scripts that animate props.

    python -I export_map_mechanics.py <game dir> <out dir (maps)> <map key> [...]

Generic on purpose: every game script on the map, with its serialized fields as the game has them,
so wiring a mechanic up needs no new export. A reference becomes
    {"go": id}             a GameObject in "nodes",
    {"comp": id}           a script in "components",
    {"type": "AnimationClip", "name": ...}   any other asset of the scene, by name, or
    {"file": cab, "id": path id, "name": ...}   an asset of another bundle (name when loaded).
Positions are Unity world space, as export_map.py's. Proprietary data: local use only.
"""
import glob
import json
import os
import sys

import UnityPy

GAME, OUT, *KEYS = sys.argv[1:]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
# Serialized by Unity for every MonoBehaviour; nothing of the game's.
BOILERPLATE = {"m_ObjectHideFlags", "m_CorrespondingSourceObject", "m_PrefabInstance", "m_PrefabAsset",
               "m_EditorHideFlags", "m_GeneratorAsset", "m_Script", "m_EditorClassIdentifier", "m_Name",
               "m_GameObject", "m_Enabled"}
# Unity's own UI and text, and per-point data map.json already has.
SKIP_PREFIX = ("UnityEngine.", "TMPro.")
SKIP = {"Assets.Scripts.Unity.Map.PathPointInfo", "Assets.Scripts.Unity.Map.CurvedLineNodes",
        "Assets.Scripts.Unity.Map.PolyLineNodes", "Assets.Scripts.Unity.Map.BezierLineNodes"}
# Named assets worth a name in the file (a part's texture or mesh is the model export's).
NAMED = {"AnimationClip", "AudioClip", "Sprite", "Texture2D", "Material", "Mesh", "AnimatorController",
         "AnimatorOverrideController"}


def monoscripts():
    names = {}
    for f in glob.glob(os.path.join(AA, "*monoscripts*.bundle")):
        for o in UnityPy.load(f).objects:
            if o.type.name == "MonoScript":
                s = o.read()
                names[o.path_id] = (s.m_Namespace + "." if s.m_Namespace else "") + s.m_ClassName
    return names


def is_pptr(v):
    return isinstance(v, dict) and set(v) == {"m_FileID", "m_PathID"}


def qmul(a, b):
    return (a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
            a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
            a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
            a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2])


def qrot(q, v):
    u = q[:3]
    t = (2 * (u[1] * v[2] - u[2] * v[1]), 2 * (u[2] * v[0] - u[0] * v[2]), 2 * (u[0] * v[1] - u[1] * v[0]))
    return (v[0] + q[3] * t[0] + u[1] * t[2] - u[2] * t[1],
            v[1] + q[3] * t[1] + u[2] * t[0] - u[0] * t[2],
            v[2] + q[3] * t[2] + u[0] * t[1] - u[1] * t[0])


def short(c):
    return c["type"].rsplit(".", 1)[-1]


def ids(field):
    return [r["comp"] for r in field or [] if r and "comp" in r]


def comp_id(field):
    return (field or {}).get("comp")


def under(go, root, nodes):
    while go in nodes:
        if go == root:
            return True
        go = nodes[go]["parent"]
    return False


def map_logic(by, components, nodes, parts):
    """map.json's "logic": removeables and map events (renderer/src/map_logic.rs). A trigger or action
    the runtime doesn't have is left out, so an event with only such triggers never fires."""
    areas = [c for c in components if short(c) == "Area"]
    removeables = []
    for c in components:
        if short(c) != "Removeable":
            continue
        f = c["fields"]
        display = (f.get("removeableObject") or {}).get("go")
        hide = [(h["fields"].get("obj") or {}).get("go") for h in components
                if short(h) == "HideObjectOnRemoveableSold" and h["go"] == display]
        roots = [g for g in hide if g] or ([display] if display else [])
        # No display of its own: what its sold event animates away. STAND-IN until prop clips play:
        # it just goes.
        if not roots:
            sold = [e for e in components if short(e) == "MapEvent" and any(
                comp_id(by[t]["fields"].get("removeable")) == c["id"] for t in ids(e["fields"].get("triggers")) if t in by)]
            for e in sold:
                for a in ids(e["fields"].get("unityActions")):
                    target = by[a]["fields"].get("anim") or by[a]["fields"].get("animator") if a in by else None
                    if target and target.get("go"):
                        roots.append(target["go"])
        removeables.append({"id": c["id"], "name": f.get("menuName") or nodes[c["go"]]["name"], "cost": f.get("removealCost", 0),
                            "active": bool(f.get("isActive")), "destroy_area": bool(f.get("destroyArea")),
                            "areas": [a["id"] for a in areas if under(a["go"], c["go"], nodes)],
                            "hide": [i for i, g in enumerate(parts) if g and any(under(g, r, nodes) for r in roots)]})

    def trigger(t):
        k, f = short(t), t["fields"]
        if k == "AfterRoundTrigger":
            return {"kind": "AfterRound", "round": f["round"], "nth": f["everyNthRound"], "primes": bool(f["usePrimeNumbers"])}
        if k == "StartRoundTrigger":
            return {"kind": "StartRound", "round": f["round"], "nth": f["everyNthRound"], "primes": bool(f["usePrimeNumbers"]),
                    "from_start": bool(f["countFromStartRound"])}
        if k == "SimTimeElapsedTrigger":
            return {"kind": "SimTime", "frames": round(f["time"] * 60), "round_time": bool(f["useRoundTime"])}
        if k == "LivesLowerThanTrigger":
            return {"kind": "LivesLower", "lives": f["lives"], "percent": f["percent"] if f["usePercent"] else None}
        if k in ("StartMapTrigger", "OnMapCreatedTrigger"):
            return {"kind": "StartMap"}
        if k == "PreBloonSpawnTrigger":
            return {"kind": "PreBloonSpawn", "tag": f["bloonTag"]}
        if k == "RemoveableSoldTrigger" and comp_id(f.get("removeable")):
            return {"kind": "RemoveableSold", "id": comp_id(f["removeable"])}
        return None

    def actions(refs, depth=0):
        out = []
        for i in refs:
            a = by.get(i)
            if a is None:
                continue
            k, f = short(a), a["fields"]
            if k == "PathActiveAction" and comp_id(f.get("path")):
                out.append({"kind": "Path", "id": comp_id(f["path"]), "active": bool(f["setActiveToo"])})
            elif k in ("DestroyAreaAction", "CreateAreaAction") and comp_id(f.get("area")):
                out.append({"kind": "Area", "ids": [comp_id(f["area"])], "create": k.startswith("Create")})
            elif k in ("DestroyAreasAction", "CreateAreasAction"):
                out.append({"kind": "Area", "ids": ids(f.get("areas")), "create": k.startswith("Create")})
            elif k == "RemovableActiveAction" and comp_id(f.get("removeable")):
                out.append({"kind": "Removeable", "id": comp_id(f["removeable"]), "active": bool(f["setActiveToo"])})
            elif k == "SellRemoveableAction" and comp_id(f.get("removeable")):
                out.append({"kind": "Sell", "id": comp_id(f["removeable"])})
            elif k == "MapEventActiveAction" and comp_id(f.get("mapEvent")):
                out.append({"kind": "Event", "id": comp_id(f["mapEvent"]), "active": bool(f["setToActive"])})
            elif k == "TriggerMapActionsAction" and depth < 8 and comp_id(f.get("mapActions")) in by:
                out += actions(ids(by[comp_id(f["mapActions"])]["fields"].get("actions")), depth + 1)
        return out

    events = []
    for c in components:
        if short(c) != "MapEvent":
            continue
        f = c["fields"]
        triggers = [t for t in (trigger(by[i]) for i in ids(f.get("triggers")) if i in by) if t]
        acts = actions(ids(f.get("actions")))
        if triggers and acts:
            events.append({"id": c["id"], "once": bool(f["oneTimeEvent"]), "all": bool(f["allTriggersRequired"]),
                           "active": not f["inactiveByDefault"], "triggers": triggers, "actions": acts})
    return {"removeables": removeables, "events": events} if removeables or events else None


def write_spawner(d, components, nodes, scene_order, parts, first_spawner):
    """map.json's "spawner": the forward splitter of the map's PathSpawner, when it isn't a plain Splitter
    (renderer/src/map_routes.rs)."""
    path = os.path.join(d, "map.json")
    if not os.path.exists(path):
        return
    by = {c["id"]: c for c in components}

    def active(go):
        while go in nodes:
            if not nodes[go]["active"]:
                return False
            go = nodes[go]["parent"]
        return True

    first = by.get(comp_id(by[first_spawner]["fields"].get("forwardSplitter"))) if first_spawner in by else None
    spawner = None
    if first and short(first) != "Splitter":
        f = first["fields"]
        spawner = {"kind": short(first), "paths": [r["comp"] for r in f["paths"] if r and "comp" in r],
                   "rounds_per_path": f.get("roundsPerPath", 0), "round_offset": f.get("currentRoundOffset", 0),
                   "does_not_loop": bool(f.get("doesNotLoop", 0)), "moab_only": list(f.get("moabOnlyPath") or []),
                   "tags": list(f.get("tagOnlyPath") or []), "groups": list(f.get("groupPaths") or []),
                   # BloonTagSplitter's (path, tag) pairs, as tags in `paths` order.
                   "path_tags": [next((q.get("tag") or "" for q in f.get("pathTagPairs") or []
                                       if comp_id(q.get("path")) == r["comp"]), "") for r in f["paths"] if r and "comp" in r],
                   # RuleSplitter's rule per path.
                   "rules": [short(by[i]) if i in by else "" for i in ids(f.get("pathsRules"))]}
    with open(path, encoding="utf-8") as stream:
        data = json.load(stream)
    # Each area's id: map.json has the scene's Area scripts with polygons, in the scene's order.
    areas = [c for c in scene_order if c["type"].endswith(".Area") and c["fields"].get("polygons")]
    logic = None
    if len(areas) == len(data["areas"]) and all(nodes[c["go"]]["name"] == a["name"] for c, a in zip(areas, data["areas"])):
        for c, a in zip(areas, data["areas"]):
            a["id"] = c["id"]
        logic = map_logic(by, components, nodes, parts)
    else:
        print("areas don't line up with map.json:", os.path.basename(d), len(areas), len(data["areas"]), flush=True)
    if data.get("spawner") != spawner or data.get("logic") != logic or "id" not in (data["areas"] or [{}])[0]:
        data["spawner"] = spawner
        data["logic"] = logic
        tmp = path + ".tmp"
        with open(tmp, "w", encoding="utf-8") as stream:
            json.dump(data, stream)
        os.replace(tmp, path)


def export(key, scripts):
    bundle = glob.glob(os.path.join(AA, f"map_{key}_scenes_all_*.bundle"))[0]
    env = UnityPy.load(bundle)
    # A map bundle is the scene plus its .sharedAssets, whose path ids overlap the scene's; a
    # reference with file id 0 always means the scene.
    objs = {o.path_id: o for o in env.objects if not o.assets_file.name.endswith(".sharedAssets")}
    externals = None
    cache = {}

    def tt(pid):
        if pid not in cache:
            cache[pid] = objs[pid].read_typetree()
        return cache[pid]

    # Every GameObject's transform, and every script of the map.
    transform_of = {}
    behaviours = []
    for pid, o in objs.items():
        if o.type.name in ("Transform", "RectTransform"):
            transform_of[tt(pid)["m_GameObject"]["m_PathID"]] = pid
        elif o.type.name == "MonoBehaviour":
            behaviours.append(pid)

    def world(tid):
        """(position, rotation, lossy scale) of a transform."""
        chain = []
        while tid and tid in objs:
            chain.append(tt(tid))
            tid = chain[-1]["m_Father"]["m_PathID"]
        pos, rot, scale = (0.0, 0.0, 0.0), (0.0, 0.0, 0.0, 1.0), (1.0, 1.0, 1.0)
        for t in reversed(chain):
            lp, lr, ls = t["m_LocalPosition"], t["m_LocalRotation"], t["m_LocalScale"]
            p = qrot(rot, (lp["x"] * scale[0], lp["y"] * scale[1], lp["z"] * scale[2]))
            pos = (pos[0] + p[0], pos[1] + p[1], pos[2] + p[2])
            rot = qmul(rot, (lr["x"], lr["y"], lr["z"], lr["w"]))
            scale = (scale[0] * ls["x"], scale[1] * ls["y"], scale[2] * ls["z"])
        return pos, rot, scale

    nodes = {}

    def node(go_id):
        """Adds a GameObject (and its parents, for the hierarchy and active state)."""
        if go_id in nodes or go_id not in objs:
            return
        go = tt(go_id)
        tid = transform_of.get(go_id)
        father = tt(tid)["m_Father"]["m_PathID"] if tid else 0
        parent = tt(father)["m_GameObject"]["m_PathID"] if father in objs else 0
        n = nodes[go_id] = {"name": go["m_Name"], "active": bool(go["m_IsActive"]), "parent": parent}
        if tid:
            pos, rot, scale = world(tid)
            n.update(pos=[round(x, 4) for x in pos], rot=[round(x, 5) for x in rot],
                     scale=[round(x, 4) for x in scale])
        kinds = sorted({objs[c["component"]["m_PathID"]].type.name for c in go["m_Component"]
                        if c["component"]["m_PathID"] in objs} - {"Transform", "RectTransform", "MonoBehaviour"})
        if kinds:
            n["unity"] = kinds
        if parent:
            node(parent)

    def asset_name(o):
        try:
            return o.peek_name()
        except Exception:
            return tt(o.path_id).get("m_Name", "") if o.path_id in objs else ""

    def convert(v):
        nonlocal externals
        if is_pptr(v):
            pid, fid = v["m_PathID"], v["m_FileID"]
            if not pid:
                return None
            if fid:
                if externals is None:
                    externals = next(iter(objs.values())).assets_file.externals
                return {"file": externals[fid - 1].name.rsplit("/", 1)[-1] if fid <= len(externals) else fid, "id": pid}
            o = objs.get(pid)
            if o is None:
                return {"missing": pid}
            t = o.type.name
            if t == "GameObject":
                node(pid)
                return {"go": pid}
            if t in ("Transform", "RectTransform"):
                go = tt(pid)["m_GameObject"]["m_PathID"]
                node(go)
                return {"go": go}
            if t == "MonoBehaviour":
                return {"comp": pid}
            go = tt(pid).get("m_GameObject") if t not in NAMED else None
            if go and go.get("m_PathID"):
                node(go["m_PathID"])
                return {"type": t, "go": go["m_PathID"]}
            return {"type": t, "name": asset_name(o)}
        if isinstance(v, dict):
            return {k: convert(x) for k, x in v.items()}
        if isinstance(v, list):
            # Raw byte arrays (a Unity event's cached data) as they are.
            return [convert(x) for x in v]
        return v

    components, unknown = [], 0
    for pid in behaviours:
        t = tt(pid)
        name = scripts.get(t["m_Script"]["m_PathID"])
        if name is None:
            unknown += 1
            continue
        if name in SKIP or name.startswith(SKIP_PREFIX):
            continue
        go = t["m_GameObject"]["m_PathID"]
        node(go)
        components.append({"id": pid, "type": name, "go": go, "enabled": bool(t.get("m_Enabled", 1)),
                           "fields": {k: convert(v) for k, v in t.items() if k not in BOILERPLATE}})
    scene_order = list(components)
    components.sort(key=lambda c: (c["type"], c["id"]))
    def active_chain(go):
        while go in objs:
            if not tt(go)["m_IsActive"]:
                return False
            tid = transform_of.get(go)
            father = tt(tid)["m_Father"]["m_PathID"] if tid else 0
            go = tt(father)["m_GameObject"]["m_PathID"] if father in objs else 0
        return True

    # Which scene object each model part of models/map_<key> is: export_model.py writes the parts in
    # this order, a renderer's submeshes in a row; a renderer it dropped simply has no parts.
    parts = []
    meta = os.path.join(OUT, "..", "models", f"map_{key}", "meta.json")
    if os.path.exists(meta):
        names = [q["name"] for q in json.load(open(meta, encoding="utf-8"))["parts"]]
        i = 0
        for o in env.objects:
            if o.type.name in ("MeshRenderer", "SkinnedMeshRenderer"):
                # A NestedPrefab's renderers are in .sharedAssets: parts of no scene object (null).
                shared = o.path_id not in objs or objs[o.path_id] is not o
                renderer = o.read_typetree()
                ref = renderer["m_GameObject"]["m_PathID"]
                go = o.assets_file.objects[ref].read_typetree() if shared else tt(ref)
                # export_model.py's add_renderer drops these; inactive copies share names (FallingPalmTrees).
                if not shared and (not renderer.get("m_Enabled", 1) or not active_chain(ref)):
                    continue
                if i < len(names) and names[i] == go["m_Name"] and not shared:
                    node(ref)
                # One part per submesh; the next renderer may have the same name.
                for _ in renderer["m_Materials"]:
                    if i < len(names) and names[i] == go["m_Name"]:
                        parts.append(None if shared else ref)
                        i += 1
        if i != len(names):
            print(key, "parts matched", i, "of", len(names), flush=True)
            parts = []
    # Animators, for the props that play clips.
    animators = []
    for pid, o in objs.items():
        if o.type.name == "Animator":
            t = tt(pid)
            go = t["m_GameObject"]["m_PathID"]
            node(go)
            animators.append({"go": go, "enabled": bool(t.get("m_Enabled", 1)), "controller": convert(t["m_Controller"])})
    d = os.path.join(OUT, key)
    os.makedirs(d, exist_ok=True)
    tmp = os.path.join(d, "mechanics.json.tmp")
    with open(tmp, "w", encoding="utf-8") as stream:
        json.dump({"key": key, "components": components, "animators": animators, "parts": parts,
                   "nodes": {str(k): v for k, v in nodes.items()}},
                  stream, separators=(",", ":"))
    os.replace(tmp, os.path.join(d, "mechanics.json"))
    # MapLoader.GatherMapBloonPathSpawners: currentMap.GetComponentInChildren<PathSpawner>(), the first
    # active one depth first, a GameObject before its children.
    spawners = {c["go"]: c["id"] for c in components if c["type"].endswith(".PathSpawner") and c["enabled"]}
    map_go = next((c["go"] for c in components if c["type"].endswith(".Unity.Map.Map")), None)
    first_spawner, stack = None, [map_go] if map_go else []
    while stack and first_spawner is None:
        go = stack.pop()
        if go not in objs or not tt(go)["m_IsActive"]:
            continue
        first_spawner = spawners.get(go)
        children = tt(transform_of[go])["m_Children"] if go in transform_of else []
        stack += [tt(ch["m_PathID"])["m_GameObject"]["m_PathID"] for ch in reversed(children) if ch["m_PathID"] in objs]
    write_spawner(d, components, nodes, scene_order, parts, first_spawner)
    print(key, len(components), "scripts", len(nodes), "nodes", unknown, "unknown", flush=True)


def main():
    scripts = monoscripts()
    for key in KEYS:
        export(key, scripts)


main()
