"""The animation of map props, after export_map_mechanics.py:

    maps/map_clips.json     every clip and controller the maps refer to, by "<cab lower case>:<path id>"
    maps/<key>/binds.json   {animator GameObject id: {path hash: GameObject id}}

    python -I export_map_clips.py <game dir> <out dir (maps)> <map key> [...]

A clip is {name, len, loop, fps, tracks: [{path, attr, samples}]}: path is Unity's path hash, relative
to the GameObject with the Animator, attr 1 position, 2 rotation, 3 scale, 4 euler; a track that never
changes has one sample. A clip that can't be decoded is {name, error}. A controller is {clips, default,
params, states: [{name, clip, speed, tag, transitions: [{to, exit_time, conditions: [{param, mode, value}]}]}]}:
what a trigger label (PlayAnimationAction.triggerLabel, PlayAnimationOnClick.trigger) does.
Proprietary data: local use only.
"""
import glob
import json
import os
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import UnityPy
from mecanim_clip import Clip

GAME, OUT, *KEYS = sys.argv[1:]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
FPS = 30
# Fields that hold a clip (everything else in a mechanics.json reference is audio, effects, ...).
CLIP_FIELDS = {"animationClip", "clip", "animClip"}
# Bundles with props' clips; others (asset_references, 900 MB) hold what no map animates with.
SMALL = 64 << 20


def refs(value, field=None):
    """(field, reference) for every external reference in a script's fields."""
    if isinstance(value, dict):
        if "file" in value and "id" in value:
            yield field, value
        else:
            for k, v in value.items():
                yield from refs(v, k)
    elif isinstance(value, list):
        for v in value:
            yield from refs(v, field)


def index(env):
    return {(o.assets_file.name.lower(), o.path_id): o for o in env.objects}


CABS = json.load(open(os.path.join(OUT, "..", "cab_index.json"), encoding="utf-8"))
loaded = {}
failed = {}


def external(cab):
    """The objects of the bundle holding `cab`, or {} when it isn't one we read."""
    bundle = CABS.get(cab)
    if bundle is None:
        return {}
    if bundle not in loaded:
        path = os.path.join(AA, bundle)
        loaded[bundle] = index(UnityPy.load(path)) if os.path.getsize(path) < SMALL else {}
    return loaded[bundle]


def clip_json(typetree):
    clip = Clip(typetree)
    length = clip.stop - clip.start
    frames = max(1, int(round(length * FPS)))
    tracks = {}
    for f in range(frames + 1):
        for path, attrs in clip.pose(clip.start + f / FPS).items():
            for attr, value in attrs.items():
                tracks.setdefault((path, attr), []).append([round(float(x), 4) for x in value])
    return {"name": clip.name, "len": round(float(length), 4), "loop": clip.loop, "fps": FPS,
            "tracks": [{"path": path, "attr": attr, "samples": s if any(x != s[0] for x in s) else s[:1]}
                       for (path, attr), s in tracks.items()]}


def controller_json(typetree, clip_keys):
    names = {h: n for h, n in typetree["m_TOS"]}
    machine = typetree["m_Controller"]["m_StateMachineArray"]
    out = {"clips": clip_keys, "default": 0, "states": [],
           "params": [{"name": names.get(v["m_ID"], v["m_ID"]), "type": v["m_Type"]}
                      for v in typetree["m_Controller"]["m_Values"]["data"]["m_ValueArray"]]}
    if not machine:
        return out
    sm = machine[0]["data"]
    out["default"] = sm.get("m_DefaultState", 0)
    def transition(t):
        return {"to": t["m_DestinationState"], "exit_time": t["m_ExitTime"] if t["m_HasExitTime"] else None,
                "duration": t["m_TransitionDuration"],
                "conditions": [{"param": names.get(c["data"]["m_EventID"], c["data"]["m_EventID"]),
                                "mode": c["data"]["m_ConditionMode"], "value": c["data"]["m_EventThreshold"]}
                               for c in t["m_ConditionConstantArray"]]}
    for s in sm["m_StateConstantArray"]:
        st = s["data"]
        clip = None
        if st.get("m_BlendTreeConstantArray"):
            nodes = st["m_BlendTreeConstantArray"][0]["data"]["m_NodeArray"]
            clip = nodes[0]["data"]["m_ClipID"] if nodes else None
        out["states"].append({"name": names.get(st["m_NameID"], st["m_NameID"]), "clip": clip, "speed": st["m_Speed"],
                              "tag": names.get(st.get("m_TagID"), ""), "loop": bool(st.get("m_Loop", False)),
                              "transitions": [transition(t["data"]) for t in st["m_TransitionConstantArray"]]})
    out["any_state"] = [transition(t["data"]) for t in sm.get("m_AnyStateTransitionConstantArray", [])]
    return out


def main():
    table = {}

    def key(file_name, pid):
        return f"{file_name.lower().removesuffix('.sharedassets')}:{pid}"

    def add(obj, file_name):
        """Exports a clip or controller object; returns its key."""
        k = key(file_name, obj.path_id)
        if k in table:
            return k
        kind = obj.type.name
        t = {}
        try:
            t = obj.read_typetree()
            if kind == "AnimationClip":
                table[k] = clip_json(t)
            elif kind == "AnimatorController":
                clip_keys = []
                for c in t["m_AnimationClips"]:
                    clip_keys.append(follow(obj, c))
                table[k] = controller_json(t, clip_keys)
            elif kind == "AnimatorOverrideController":
                base = follow(obj, t["m_Controller"])
                table[k] = {"base": base, "overrides": [[follow(obj, o["m_OriginalClip"]), follow(obj, o["m_OverrideClip"])]
                                                        for o in t["m_Clips"]]}
            else:
                table[k] = {"error": "unexpected " + kind}
        except Exception as e:
            failed[kind] = failed.get(kind, 0) + 1
            table[k] = {"name": t.get("m_Name", ""), "error": f"{type(e).__name__}: {e}"}
        return k

    def follow(obj, pptr):
        """The key of what a PPtr inside `obj` points at, exporting it."""
        pid, fid = pptr["m_PathID"], pptr["m_FileID"]
        if not pid:
            return None
        af = obj.assets_file
        if not fid:
            return add(af.objects[pid], af.name)
        name = af.externals[fid - 1].name.rsplit("/", 1)[-1]
        target = None
        low = name.lower()
        if low.endswith(".sharedassets"):
            target = scene.get((low, pid))
        else:
            target = external(low).get((low, pid))
            if target is None:
                target = scene.get((low, pid))
        return add(target, name) if target is not None else None

    for map_key in KEYS:
        path = os.path.join(OUT, map_key, "mechanics.json")
        if not os.path.exists(path):
            continue
        mech = json.load(open(path, encoding="utf-8"))
        bundle = glob.glob(os.path.join(AA, f"map_{map_key}_scenes_all_*.bundle"))[0]
        env = UnityPy.load(bundle)
        scene = index(env)
        wanted = [r for a in mech["animators"] for r in refs(a["controller"], "controller")]
        for c in mech["components"]:
            wanted += [(f, r) for f, r in refs(c["fields"]) if f in CLIP_FIELDS]
        used = set()
        for _, r in wanted:
            low = str(r["file"]).lower()
            obj = scene.get((low, r["id"]))
            if obj is None and not low.endswith(".sharedassets"):
                obj = external(low).get((low, r["id"]))
            if obj is None:
                failed["missing"] = failed.get("missing", 0) + 1
                continue
            used.add(add(obj, low))
        # Each animator's paths, from its own GameObject down.
        hashes = set()
        pending = list(used)
        while pending:
            entry = table.get(pending.pop())
            if not entry:
                continue
            hashes.update(t["path"] for t in entry.get("tracks", []))
            pending += [c for c in entry.get("clips", []) if c]
            pending += [c for c in [entry.get("base")] if c]
            pending += [c for pair in entry.get("overrides", []) for c in pair if c]
        objs = {o.path_id: o for o in env.objects if not o.assets_file.name.endswith(".sharedAssets")}
        transforms = {}
        for pid, o in objs.items():
            if o.type.name in ("Transform", "RectTransform"):
                t = o.read_typetree()
                transforms[t["m_GameObject"]["m_PathID"]] = t
        names = {}

        def name_of(go):
            if go not in names:
                names[go] = objs[go].read_typetree()["m_Name"]
            return names[go]

        binds = {}
        roots = {a["go"] for a in mech["animators"]}
        for root in roots:
            found = {}
            stack = [(root, "")]
            while stack:
                go, rel = stack.pop()
                h = zlib.crc32(rel.encode("utf-8"))
                if h in hashes:
                    found[str(h)] = go
                t = transforms.get(go)
                for child in t["m_Children"] if t else []:
                    c = objs.get(child["m_PathID"])
                    if c is None:
                        continue
                    cgo = c.read_typetree()["m_GameObject"]["m_PathID"]
                    stack.append((cgo, (rel + "/" if rel else "") + name_of(cgo)))
            if found:
                binds[str(root)] = found
        with open(os.path.join(OUT, map_key, "binds.json"), "w", encoding="utf-8") as stream:
            json.dump(binds, stream, separators=(",", ":"))
        del env, scene, objs, transforms
        print(map_key, len(used), "clips or controllers;", len(binds), "animators bound;", len(table), "total", flush=True)
    tmp = os.path.join(OUT, "map_clips.json.tmp")
    with open(tmp, "w", encoding="utf-8") as stream:
        json.dump(table, stream, separators=(",", ":"))
    os.replace(tmp, os.path.join(OUT, "map_clips.json"))
    print("clips", sum("tracks" in v for v in table.values()), "controllers", sum("states" in v or "base" in v for v in table.values()),
          "failed", failed, flush=True)


main()
