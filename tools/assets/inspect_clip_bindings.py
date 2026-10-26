"""List an animated prefab's non-transform clip bindings (GameObject active, renderer enabled, ...)
with their values over each clip.

Usage:
  tools/.venv/Scripts/python.exe tools/assets/inspect_clip_bindings.py <game install> <bundle-glob> <prefab container path>
"""
import glob
import os
import sys
import zlib

import UnityPy
from UnityPy.helpers import TypeTreeHelper

TypeTreeHelper.read_typetree_boost = False
GAME, BUNDLE, PREFAB = sys.argv[1:4]
sys.argv = [sys.argv[0], GAME, "local_extracted/models/_inspect", BUNDLE, PREFAB]
sys.path.insert(0, os.path.dirname(__file__))
from export_animated import AA, Clip  # noqa: E402

env = UnityPy.load(glob.glob(os.path.join(AA, BUNDLE + "*.bundle"))[0])
ab = next(o for o in env.objects if o.type.name == "AssetBundle").read_typetree()
pid = next(v["asset"]["m_PathID"] for k, v in ab["m_Container"] if k == PREFAB)
root = next(o for o in env.objects if o.path_id == pid)
af = root.assets_file


def comp(go, name):
    for c in go["m_Component"]:
        o = af.objects.get(c["component"]["m_PathID"])
        if o is not None and o.type.name == name:
            return o
    return None


paths = {}
animator = None
stack = [(comp(root.read_typetree(), "Transform").path_id, "")]
while stack:
    tid, path = stack.pop()
    t = af.objects[tid].read_typetree()
    go = af.objects[t["m_GameObject"]["m_PathID"]].read_typetree()
    if animator is None and comp(go, "Animator") is not None:
        animator, path = comp(go, "Animator").read_typetree(), ""
    if animator is not None:
        paths[zlib.crc32(path.encode())] = (path or "<root>") + f" (active={go.get('m_IsActive', 1)})"
    for ch in t["m_Children"]:
        name = af.objects[af.objects[ch["m_PathID"]].read_typetree()["m_GameObject"]["m_PathID"]].read_typetree()["m_Name"]
        stack.append((ch["m_PathID"], f"{path}/{name}" if path else name))

ctrl = af.objects[animator["m_Controller"]["m_PathID"]].read_typetree()
for c in ctrl["m_AnimationClips"]:
    clip = Clip(af.objects[c["m_PathID"]].read_typetree())
    print(f"== {clip.name} ({clip.stop - clip.start:.2f}s) events={clip.events}")
    ci = 0
    for b in clip.bindings:
        n = {1: 3, 2: 4, 3: 3, 4: 3}.get(b["attribute"], 1) if b["typeID"] == 4 else 1
        if b["typeID"] != 4:
            samples = []
            steps = 8
            for k in range(steps + 1):
                t = clip.start + (clip.stop - clip.start) * k / steps
                samples.append(round(clip.values(t)[ci], 2))
            print(f"  type {b['typeID']} attr {b['attribute']} path {paths.get(b['path'], b['path'])}: {samples}")
        ci += n
