"""Export original UI controller references and decoded Animator curves and endpoints.

Usage: python export_ui_runtime.py GAME OUT SceneName ...
Output is extracted proprietary data and must remain under local_extracted/.
Values and polynomial coefficients come from Unity streamed/dense/constant curves.
"""
import json
import math
import struct
import sys
import zlib
from pathlib import Path

GAME, OUT, *SCENES = sys.argv[1:]
import glob
import os
import UnityPy
from UnityPy.helpers import TypeTreeHelper
TypeTreeHelper.read_typetree_boost = False
base = Path(GAME)/"BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full"
patterns = os.environ.get('RRBTD6_UI_BUNDLES', '*_monoscripts_*.bundle;ui_scenes_all_*.bundle;asset_references_assets_all_*.bundle').split(';')
env = UnityPy.load(*(glob.glob(str(base/p))[0] for p in patterns))
files = {sf.name.lower():sf for f in env.files.values() for sf in [f,*getattr(f,"files",{}).values()] if hasattr(sf,"objects") and getattr(sf,"name",None)}

def resolve(owner, pointer):
    pid, fid = pointer["m_PathID"], pointer["m_FileID"]
    if not pid: return None
    if not fid: return owner.assets_file.objects.get(pid)
    file = files.get(owner.assets_file.externals[fid-1].path.rsplit("/",1)[-1].lower())
    return file.objects.get(pid) if file else None

def script_name(owner, pointer):
    obj = resolve(owner,pointer)
    return obj.read().m_ClassName if obj else None


def values(clip, time):
    data = clip["m_MuscleClip"]["m_Clip"]["data"]
    streamed = data["m_StreamedClip"]
    out = [0.0] * streamed["curveCount"]
    raw = struct.pack("<%dI" % len(streamed["data"]), *streamed["data"])
    pos = 0
    while pos < len(raw):
        t, count = struct.unpack_from("<fI", raw, pos)
        pos += 8
        for _ in range(count):
            index, a, b, c, d = struct.unpack_from("<I4f", raw, pos)
            pos += 20
            if t <= time and index < len(out):
                dt = time - t
                out[index] = ((a * dt + b) * dt + c) * dt + d
    dense = data["m_DenseClip"]
    count = dense["m_CurveCount"]
    if count:
        f = max(0.0, min(dense["m_FrameCount"] - 1, (time - dense["m_BeginTime"]) * dense["m_SampleRate"]))
        i = int(f)
        j = min(i + 1, dense["m_FrameCount"] - 1)
        samples = dense["m_SampleArray"]
        out += [samples[i * count + k] * (1 - (f-i)) + samples[j * count + k] * (f-i) for k in range(count)]
    return out + data["m_ConstantClip"]["data"]


def curves(clip):
    """Keep original cubic coefficients; no resampling or guessed easing."""
    data = clip["m_MuscleClip"]["m_Clip"]["data"]
    streamed = data["m_StreamedClip"]
    out = [[] for _ in range(streamed["curveCount"])]
    raw = struct.pack("<%dI" % len(streamed["data"]), *streamed["data"])
    pos = 0
    while pos < len(raw):
        t, count = struct.unpack_from("<fI", raw, pos)
        pos += 8
        for _ in range(count):
            index, a, b, c, d = struct.unpack_from("<I4f", raw, pos)
            pos += 20
            if index < len(out) and math.isfinite(t):
                out[index].append([t, a, b, c, d])
    dense = data["m_DenseClip"]
    count, frames = dense["m_CurveCount"], dense["m_FrameCount"]
    for channel in range(count):
        keys = []
        for frame in range(frames):
            value = dense["m_SampleArray"][frame * count + channel]
            slope = ((dense["m_SampleArray"][(frame+1)*count+channel]-value) * dense["m_SampleRate"]) if frame+1 < frames else 0.0
            keys.append([dense["m_BeginTime"] + frame / dense["m_SampleRate"], 0.0, 0.0, slope, value])
        out.append(keys)
    for value in data["m_ConstantClip"]["data"]:
        out.append([[0.0, 0.0, 0.0, 0.0, value]])
    return out


def walk(go, path=""):
    t = go.read_typetree()
    path = path + "/" + t["m_Name"] if path else t["m_Name"]
    yield go, path
    for c in t["m_Component"]:
        component = resolve(go, c["component"])
        if component and component.type.name in ("Transform", "RectTransform"):
            for child in component.read_typetree()["m_Children"]:
                tr = resolve(component, child)
                if tr:
                    obj = resolve(tr, tr.read_typetree()["m_GameObject"])
                    if obj:
                        yield from walk(obj, path)


def target(owner, pointer, paths):
    obj = resolve(owner, pointer)
    if obj is None:
        return None
    data = obj.read_typetree()
    if obj.type.name == "GameObject":
        return paths.get(obj.path_id, data.get("m_Name"))
    if "m_GameObject" in data:
        return paths.get(data["m_GameObject"]["m_PathID"])
    return data.get("m_Name")


PROPERTY_NAMES = ["m_AnchoredPosition.x", "m_AnchoredPosition.y", "m_SizeDelta.x", "m_SizeDelta.y",
                  "m_Pivot.x", "m_Pivot.y", "m_AnchorMin.x", "m_AnchorMin.y", "m_AnchorMax.x", "m_AnchorMax.y",
                  "m_Alpha", "m_IsActive", "m_Enabled", "m_Color.r", "m_Color.g", "m_Color.b", "m_Color.a"]
PROPERTIES = {zlib.crc32(p.encode()): p for p in PROPERTY_NAMES}
Path(OUT).mkdir(parents=True, exist_ok=True)
for scene in SCENES:
    if scene.startswith("prefab:"):
        source=scene[7:]
        source,_,alias=source.partition("=")
        root=env.container[source]
        root=root.deref() if hasattr(root,"deref") else root
        roots=[root]
        scene=alias or Path(source).stem
    else:
        roots=None
    scene_path = "Assets/UI - New/Scenes/" + scene + ".unity"
    cab = next((v for o in env.objects if o.type.name == "AssetBundle"
                for k,v in o.read_typetree().get("m_SceneHashes", []) if k == scene_path), None)
    if cab is None and roots is None:
        matches = [v for o in env.objects if o.type.name == "AssetBundle"
                   for k,v in o.read_typetree().get("m_SceneHashes", [])
                   if k.rsplit("/",1)[-1].lower() == (scene+".unity").lower()]
        if len(matches) != 1:
            raise ValueError(f"Scene {scene}: expected one scene match, got {len(matches)}")
        cab = matches[0]
    if roots is None:
        sf = files[cab.lower()]
        roots = []
        for o in sf.objects.values():
            if o.type.name in ("Transform", "RectTransform"):
                t = o.read_typetree()
                if not t["m_Father"]["m_PathID"]:
                    roots.append(resolve(o, t["m_GameObject"]))
    nodes = [item for root in roots for item in walk(root)]
    paths = {o.path_id:p for o,p in nodes}
    result = {"scene": scene, "scripts": [], "animators": []}
    for go,path in nodes:
        for ref in go.read_typetree()["m_Component"]:
            co = resolve(go, ref["component"])
            if not co:
                continue
            t = co.read_typetree()
            if co.type.name == "MonoBehaviour":
                name = script_name(co,t["m_Script"])
                if name in ("HeroUpgradeDetails", "HeroButton", "HeroUpgradeButton", "MonkeySelectMenu", "MonkeyButton", "UpgradeScreen"):
                    refs = {k:target(co,v,paths) for k,v in t.items()
                            if isinstance(v,dict) and "m_FileID" in v and "m_PathID" in v}
                    arrays = {k:[target(co,p,paths) for p in v] for k,v in t.items()
                              if isinstance(v,list) and v and isinstance(v[0],dict) and "m_PathID" in v[0]}
                    # Plain Color fields (e.g. UpgradeScreen.backgroundColour for CommonBackgroundScreen).
                    colors = {k:[v["r"],v["g"],v["b"],v["a"]] for k,v in t.items()
                              if isinstance(v,dict) and set(v) == {"r","g","b","a"}}
                    result["scripts"].append({"path":path,"type":name,"refs":refs,"arrays":arrays,"colors":colors})
            if co.type.name != "Animator":
                continue
            controller = resolve(co,t["m_Controller"])
            if not controller:
                continue
            ctrl = controller.read_typetree()
            prefix = path + "/"
            hashes = {0:path, **{zlib.crc32(p[len(prefix):].encode()):p for _,p in nodes if p.startswith(prefix)}}
            clips = []
            for pointer in ctrl.get("m_AnimationClips",[]):
                obj = resolve(controller,pointer)
                if not obj:
                    continue
                clip = obj.read_typetree()
                if "m_MuscleClip" not in clip:
                    continue
                muscle = clip["m_MuscleClip"]
                samples = [values(clip,muscle[k]) for k in ("m_StartTime","m_StopTime")]
                channels = curves(clip)
                bindings, index = [], 0
                for b in clip["m_ClipBindingConstant"]["genericBindings"]:
                    n = {1:3,2:4,3:3,4:3}.get(b["attribute"],1) if b["typeID"] == 4 else 1
                    bindings.append({"path":hashes.get(b["path"]), "path_hash":b["path"],
                                     "type":b["typeID"],"attribute":b["attribute"],
                                     "property":PROPERTIES.get(b["attribute"]),
                                     "start":samples[0][index:index+n],"end":samples[1][index:index+n],
                                     "curves":channels[index:index+n]})
                    index += n
                clips.append({"name":clip["m_Name"], "start":muscle["m_StartTime"],"stop":muscle["m_StopTime"],"loop":bool(muscle["m_LoopTime"]),
                              "bindings":bindings})
            result["animators"].append({"path":path,"controller":ctrl.get("m_Name"),"clips":clips})
    destination = Path(OUT)/(scene+".json")
    destination.write_text(json.dumps(result,indent=1,allow_nan=False),encoding="utf8")
    print(scene,len(result["scripts"]),"controllers;",len(result["animators"]),"animators")
