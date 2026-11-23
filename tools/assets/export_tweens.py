"""Projectile tweens (CustomRotationAnimator / CustomScaleAnimator) for exported mesh projectiles.

    python tools/assets/export_tweens.py [--game <install>]

The game animates many projectile prefabs in code (Exploding Pineapple: it turns once and shrinks
to 0.59 as it falls; potions, bombs, mines, bananas, rockets...). CustomAnimationComponent.Update
(@0x1804e2830): progress += deltaTime / duration x speedMultiplier from 0 when enabled; while it's
at most 1 (or the animation loops) UpdateAnimation(progress) (looping: its fraction), else
UpdateAnimation(1). CustomRotationAnimator.UpdateAnimation (@0x1804e3600): the target's
localRotation = AngleAxis(curve(progress) x rotationsPerDuration x 360 + angleOffset, axis).
CustomScaleAnimator.UpdateAnimation (@0x1804e3a60): its localScale = the starting scale x
curve(progress).

For every models/projectiles/<name> folder whose prefab has them, writes tweens.json:
{"targets": [{"parts": [meta.json part indices at or under the target], "depth": n,
  "p": 4x4 (row-major) of the target's parent chain and its own position, in the export's model
  space (export_model.py: the prefab root's own transform left out), "base": [rotation x, y, z, w,
  scale x, y, z] (its own local rotation and scale),
  "rot": {"keys": [[time, value, in, out], ...], "duration", "speed", "loop", "turns", "offset",
          "axis": [x, y, z]} or null, "scale": {... no turns / offset / axis} or null}, ...]}
"""
import glob
import json
import os
import sys
from collections import defaultdict
from pathlib import Path

import numpy as np
import UnityPy

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "local_extracted"
AA = Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)")) / "Steam/steamapps/common/BloonsTD6/BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64"


def pid(o):
    """An object's path id (UnityPy keeps it on its reader)."""
    r = getattr(o, "object_reader", None)
    return r.path_id if r is not None else o.path_id


def trs(t):
    p, q, s = t.m_LocalPosition, t.m_LocalRotation, t.m_LocalScale
    x, y, z, w = q.x, q.y, q.z, q.w
    r = np.array([[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
                  [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
                  [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]])
    m = np.eye(4)
    m[:3, :3] = r @ np.diag([s.x, s.y, s.z])
    m[:3, 3] = [p.x, p.y, p.z]
    return m


def chain(t, root):
    """The transform's matrix up to, not including, the prefab root (export_model.world_matrix)."""
    m = np.eye(4)
    while t is not None and pid(t) != pid(root):
        m = trs(t) @ m
        t = t.m_Father.read() if t.m_Father and t.m_Father.path_id else None
    return m


def anim(tt, kind):
    keys = [[k["time"], k["value"], k["inSlope"], k["outSlope"]] for k in tt["curve"]["m_Curve"]]
    a = {"keys": keys, "duration": float(tt.get("duration", 1.0)), "speed": float(tt.get("speedMultiplier", 1.0)),
         "loop": bool(tt.get("loop", 0)), "playing": bool(tt.get("isPlaying", 1))}
    if kind == "rot":
        ax = int(tt.get("axis", 1))
        a.update(turns=float(tt.get("rotationsPerDuration", 1.0)), offset=float(tt.get("angleOffset", 0.0)),
                 axis=[[1.0, 0, 0], [0, 1.0, 0], [0, 0, 1.0]][ax if 0 <= ax < 3 else 1],
                 local=bool(tt.get("useLocalCoordinateSustem", 1)))
    return a


def main():
    global AA
    args = sys.argv[1:]
    if args[:1] == ["--game"]:
        AA = Path(args[1]) / "BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64"
        args = args[2:]
    folders = [f for f in sorted((OUT / "models" / "projectiles").iterdir()) if (f / "meta.json").exists()]
    paths = json.loads((OUT / "all_container_paths.json").read_text(encoding="utf-8"))
    by_bundle = defaultdict(list)
    for f in folders:
        cand = sorted(p for p in paths if p.endswith("/" + f.name + ".prefab"))
        if cand:
            by_bundle[paths[cand[0]]].append((f, cand[0]))
    mono = [str(p) for p in (AA / "Full").glob("*monoscripts*.bundle")]
    written = 0
    for bundle, items in by_bundle.items():
        path = AA / "Full" / bundle
        if not path.exists():
            path = AA / "Half" / bundle
        if not path.exists():
            continue
        try:
            env = UnityPy.load(str(path), *mono)
        except Exception as e:
            print("skip bundle", bundle, type(e).__name__)
            continue
        cont = {k.lower(): v for k, v in env.container.items()}
        for f, cpath in items:
            obj = cont.get(cpath.lower())
            if obj is None:
                continue
            root = obj.read()
            root_tr = root.m_Transform.read()
            # Renderers in hierarchy order with their ancestor objects; tweens by target object.
            renderers, tweens = [], defaultdict(dict)

            def walk(go, ancestors):
                here = ancestors + [pid(go)]
                for c in go.m_Component:
                    ptr = c.component if hasattr(c, "component") else c
                    t = ptr.type.name
                    if t in ("MeshRenderer", "SkinnedMeshRenderer"):
                        renderers.append((go.m_Name, here))
                    elif t == "MonoBehaviour":
                        try:
                            name = ptr.read().m_Script.read().m_ClassName
                        except Exception:
                            continue
                        if name in ("CustomRotationAnimator", "CustomScaleAnimator"):
                            tt = ptr.read_typetree()
                            target = tt.get("target", {}).get("m_PathID") or pid(go)
                            tweens[target]["rot" if name == "CustomRotationAnimator" else "scale"] = anim(tt, "rot" if name == "CustomRotationAnimator" else "scale")
                for ch in go.m_Transform.read().m_Children:
                    walk(ch.read().m_GameObject.read(), here)

            walk(root, [])
            if not tweens:
                continue
            parts = [p["name"] for p in json.loads((f / "meta.json").read_text(encoding="utf-8"))["parts"]]
            j, index = 0, []
            for name in parts:
                while j < len(renderers) and renderers[j][0] != name:
                    j += 1
                if j == len(renderers):
                    index = None
                    break
                index.append(j)
                j += 1
            if index is None:
                print(f.name, "parts don't follow the prefab's renderers; skipped")
                continue
            targets = []
            for go_id, an in tweens.items():
                if not any(a.get("playing", True) for a in an.values()):
                    continue
                go = env.assets[0].objects[go_id].read() if go_id in env.assets[0].objects else None
                if go is None:
                    for af in env.assets:
                        if go_id in af.objects:
                            go = af.objects[go_id].read()
                            break
                if go is None:
                    continue
                tr = go.m_Transform.read()
                if pid(tr) == pid(root_tr):
                    continue
                parent = tr.m_Father.read() if tr.m_Father and tr.m_Father.path_id else None
                p = (chain(parent, root_tr) if parent else np.eye(4)) @ np.array([[1, 0, 0, tr.m_LocalPosition.x], [0, 1, 0, tr.m_LocalPosition.y], [0, 0, 1, tr.m_LocalPosition.z], [0, 0, 0, 1.0]])
                q, s = tr.m_LocalRotation, tr.m_LocalScale
                under = [i for i, r in zip(range(len(parts)), index) if go_id in renderers[r][1]]
                if not under:
                    continue
                # (Its depth in the hierarchy: the root is 0.)
                depth = next(renderers[r][1].index(go_id) for r in index if go_id in renderers[r][1])
                targets.append({"parts": under, "depth": depth, "p": p.flatten().tolist(),
                                "base": [q.x, q.y, q.z, q.w, s.x, s.y, s.z], "rot": an.get("rot"), "scale": an.get("scale")})
            if targets:
                # Deepest first (a parent's tween moves its children's already-tweened vertices).
                targets.sort(key=lambda t: -t["depth"])
                (f / "tweens.json").write_text(json.dumps({"targets": targets}), encoding="utf-8")
                written += 1
                print(f.name, [(len(t["parts"]), "rot" if t["rot"] else "", "scale" if t["scale"] else "") for t in targets])
    print(f"tweens.json written for {written} projectile models")


if __name__ == "__main__":
    main()
