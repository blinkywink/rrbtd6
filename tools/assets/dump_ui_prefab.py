"""Dump Unity UI prefabs (RectTransform tree + Image / TextMeshPro / layout components) to JSON.

Usage:
    tools/.venv/Scripts/python.exe tools/assets/dump_ui_prefab.py <game install> <out_dir> <container path>...

Writes <out_dir>/<prefab name>.json: a node tree
{name, active, rect: {anchor_min, anchor_max, pos, size, pivot, scale, rot_z}, comps: [...], children: [...]}.
Sprites are referenced by name (see export_sprites.py), fonts by asset name. Proprietary data: local use only.
"""
import glob
import json
import math
import os
import sys

import UnityPy
from UnityPy.helpers import TypeTreeHelper

TypeTreeHelper.read_typetree_boost = False

GAME, OUT, *PATHS = sys.argv[1:]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
# A focused research export can omit large texture bundles. The full export remains the default.
patterns = os.environ.get("RRBTD6_UI_BUNDLES", "").split(";") if os.environ.get("RRBTD6_UI_BUNDLES") else (
    "*_monoscripts_*.bundle", "sprite_atlases_assets_all_*.bundle", "localization_assets_all_*.bundle",
    "asset_references_assets_all_*.bundle", "ui_scenes_all_*.bundle")
env = UnityPy.load(*[glob.glob(os.path.join(AA, p))[0] for p in patterns])


# Global object index: (file name, path id) -> object reader.
files = {}
for f in env.files.values():
    # A bundle holds serialized files; a loose (pre-extracted) CAB is one itself.
    for sf in [f, *getattr(f, "files", {}).values()]:
        if hasattr(sf, "objects") and getattr(sf, "name", None):
            files[sf.name.lower()] = sf
by_file = {name: sf.objects for name, sf in files.items()}


def resolve(owner, pptr):
    fid, pid = pptr["m_FileID"], pptr["m_PathID"]
    if pid == 0:
        return None
    if fid == 0:
        return owner.assets_file.objects.get(pid)
    ext = owner.assets_file.externals[fid - 1].path.lower()
    name = ext.rsplit("/", 1)[-1]
    objs = by_file.get(name)
    return objs.get(pid) if objs else None


script_names = {}


def script_name(owner, pptr):
    o = resolve(owner, pptr)
    if o is None:
        return None
    key = (o.assets_file.name, o.path_id)
    if key not in script_names:
        s = o.read()
        script_names[key] = s.m_ClassName
    return script_names[key]


def asset_name(owner, pptr):
    o = resolve(owner, pptr)
    if o is None:
        return None if pptr["m_PathID"] == 0 else f"?{pptr['m_PathID']}"
    try:
        return o.read_typetree().get("m_Name")
    except Exception:
        return o.read().m_Name


MAT_KEYS = ("_OutlineColor", "_OutlineWidth", "_OutlineSoftness", "_FaceDilate", "_FaceColor", "_UnderlayColor",
            "_UnderlayOffsetX", "_UnderlayOffsetY", "_UnderlayDilate", "_UnderlaySoftness", "_GradientScale",
            "_ScaleRatioA", "_ScaleRatioB", "_ScaleRatioC", "_WeightNormal", "_WeightBold", "_TextureWidth", "_TextureHeight")
mat_cache = {}


def material_props(owner, pptr):
    o = resolve(owner, pptr)
    if o is None:
        return None
    key = (o.assets_file.name, o.path_id)
    if key not in mat_cache:
        t = o.read_typetree()
        sp = t.get("m_SavedProperties", {})
        props = {"name": t.get("m_Name")}
        for k, v in sp.get("m_Floats", []):
            if k in MAT_KEYS:
                props[k] = v
        for k, v in sp.get("m_Colors", []):
            if k in MAT_KEYS:
                props[k] = color(v)
        kw = t.get("m_ValidKeywords") or t.get("m_ShaderKeywords") or []
        props["keywords"] = kw if isinstance(kw, list) else str(kw).split()
        mat_cache[key] = props
    return mat_cache[key]


def color(c):
    return [round(c["r"], 4), round(c["g"], 4), round(c["b"], 4), round(c["a"], 4)]


KEEP = {
    "NK_TextMeshProInputField": ["m_CharacterLimit", "m_ContentType", "m_CharacterValidation", "m_LineType", "m_Text"],
    "ScrollRect": ["m_Content", "m_Viewport", "m_Horizontal", "m_Vertical", "m_MovementType", "m_Elasticity",
                   "m_Inertia", "m_DecelerationRate", "m_ScrollSensitivity"],
    "EventSystem": ["m_PixelDragThreshold"],
    "Image": ["m_Type", "m_PreserveAspect", "m_FillCenter", "m_FillMethod", "m_FillAmount", "m_PixelsPerUnitMultiplier", "m_RaycastTarget"],
    "RawImage": ["m_UVRect"],
    "TextMeshProUGUI": ["m_text", "m_fontSize", "m_fontSizeMin", "m_fontSizeMax", "m_enableAutoSizing", "m_fontStyle",
                        "m_HorizontalAlignment", "m_VerticalAlignment", "m_textAlignment", "m_characterSpacing",
                        "m_lineSpacing", "m_enableWordWrapping", "m_TextWrappingMode", "m_overflowMode", "m_margin"],
    "HorizontalLayoutGroup": ["m_Padding", "m_ChildAlignment", "m_Spacing", "m_ChildForceExpandWidth", "m_ChildForceExpandHeight",
                              "m_ChildControlWidth", "m_ChildControlHeight", "m_ChildScaleWidth", "m_ChildScaleHeight", "m_ReverseArrangement"],
    "VerticalLayoutGroup": ["m_Padding", "m_ChildAlignment", "m_Spacing", "m_ChildForceExpandWidth", "m_ChildForceExpandHeight",
                            "m_ChildControlWidth", "m_ChildControlHeight", "m_ChildScaleWidth", "m_ChildScaleHeight", "m_ReverseArrangement"],
    "GridLayoutGroup": ["m_Padding", "m_ChildAlignment", "m_CellSize", "m_Spacing", "m_StartCorner", "m_StartAxis", "m_Constraint", "m_ConstraintCount"],
    "LayoutElement": ["m_IgnoreLayout", "m_MinWidth", "m_MinHeight", "m_PreferredWidth", "m_PreferredHeight", "m_FlexibleWidth", "m_FlexibleHeight"],
    "ContentSizeFitter": ["m_HorizontalFit", "m_VerticalFit"],
    "AspectRatioFitter": ["m_AspectMode", "m_AspectRatio"],
    "CanvasScaler": ["m_UiScaleMode", "m_ReferenceResolution", "m_ScreenMatchMode", "m_MatchWidthOrHeight", "m_ReferencePixelsPerUnit"],
    "CanvasGroup": ["m_Alpha"],
    "Mask": ["m_ShowMaskGraphic"],
}


def component(owner, t, cls):
    out = {"id": owner.path_id, "type": cls, "enabled": bool(t.get("m_Enabled", 1))}
    if "m_text" in t and "m_fontAsset" in t:
        # NK_TextMeshProUGUI and other TMP subclasses.
        out["script"], cls = cls, "TextMeshProUGUI"
        out["type"] = cls
    # ScrollRect subclasses (ScrollRectWithDragCancel) keep the ScrollRect fields.
    for k in KEEP.get(cls, KEEP["ScrollRect"] if "m_ScrollSensitivity" in t else []):
        if k in t:
            out[k] = t[k]
    if "m_Color" in t:
        out["color"] = color(t["m_Color"])
    if "m_fontColor" in t:
        out["color"] = color(t["m_fontColor"])
    if t.get("m_enableVertexGradient"):
        # TMP_Text vertex colour gradient: per-glyph corners, multiplied by the font colour.
        g = t.get("m_fontColorGradient") or {}
        out["gradient"] = [color(g[k]) for k in ("topLeft", "topRight", "bottomLeft", "bottomRight") if k in g]
        out["colorMode"] = t.get("m_colorMode")
    if "m_Sprite" in t:
        out["sprite"] = asset_name(owner, t["m_Sprite"])
    if "m_Texture" in t and cls == "RawImage":
        out["texture"] = asset_name(owner, t["m_Texture"])
    if "m_fontAsset" in t:
        out["font"] = asset_name(owner, t["m_fontAsset"])
    if "m_sharedMaterial" in t:
        out["material"] = asset_name(owner, t["m_sharedMaterial"])
        out["mat"] = material_props(owner, t["m_sharedMaterial"])
    if "m_Material" in t and cls != "TextMeshProUGUI":
        m = asset_name(owner, t["m_Material"])
        if m:
            out["material"] = m
    if 'm_Colors' in t:
        for key in ['m_Colors','m_Transition','m_Interactable']:
            if key in t:out[key]=t[key]
        target=resolve(owner,t.get('m_TargetGraphic',{'m_FileID':0,'m_PathID':0}))
        if target:
            out['target_go']=target.read_typetree().get('m_GameObject',{}).get('m_PathID')
    if cls == "Toggle":
        # Toggle.graphic is shown only while isOn (Toggle.PlayEffect).
        out["m_IsOn"] = t.get("m_IsOn")
        g = t.get("graphic") or {}
        if g.get("m_PathID"):
            out["graphic"] = "->" + str(g["m_PathID"])
    if cls not in KEEP and cls not in ("Button", "Toggle"):
        # Game scripts: keep scalar fields and object references by name (enough to see roles).
        for k, v in t.items():
            if k.startswith("m_") and k not in ("m_Name",):
                continue
            if isinstance(v, (int, float, str, bool)):
                out[k] = v
            elif isinstance(v, dict) and set(v) == {"m_FileID", "m_PathID"} and v["m_PathID"]:
                out[k] = "->" + str(v["m_PathID"])
            elif isinstance(v, dict) and set(v) == {"r", "g", "b", "a"}:
                out[k] = [v["r"], v["g"], v["b"], v["a"]]
            elif isinstance(v, dict) and set(v) in ({"x", "y"}, {"x", "y", "z"}, {"x", "y", "z", "w"}):
                out[k] = [v[a] for a in "xyzw" if a in v]
            elif isinstance(v, dict) and "m_Curve" in v:
                # AnimationCurve: [time, value, inSlope, outSlope] keys (Hermite), wrap modes.
                out[k] = {"keys": [[kf["time"], kf["value"], kf["inSlope"], kf["outSlope"]] for kf in v["m_Curve"]],
                          "pre": v.get("m_PreInfinity"), "post": v.get("m_PostInfinity")}
    return out


def node(go_obj):
    go = go_obj.read_typetree()
    n = {"name": go["m_Name"], "active": bool(go["m_IsActive"]), "id": go_obj.path_id, "comps": [], "children": []}
    tr_obj = None
    for c in go["m_Component"]:
        co = resolve(go_obj, c["component"])
        if co is None:
            continue
        tn = co.type.name
        if tn in ("RectTransform", "Transform"):
            tr_obj = co
            continue
        if tn == "MonoBehaviour":
            t = co.read_typetree()
            cls = script_name(co, t["m_Script"]) or "?"
            n["comps"].append(component(co, t, cls))
        elif tn in ("Canvas", "CanvasRenderer"):
            if tn == "Canvas":
                n["comps"].append({"type": "Canvas"})
        elif tn == "SpriteRenderer":
            # World-space 2D sprite (title screen background): sprite metrics for drawing it.
            t = co.read_typetree()
            comp = {"type": "SpriteRenderer", "id": co.path_id, "enabled": bool(t.get("m_Enabled", 1)),
                    "sprite": asset_name(co, t["m_Sprite"]), "color": color(t["m_Color"]),
                    "order": t.get("m_SortingOrder", 0), "layer": t.get("m_SortingLayerID", 0),
                    "flip": [bool(t.get("m_FlipX", 0)), bool(t.get("m_FlipY", 0))],
                    "drawMode": t.get("m_DrawMode", 0), "size": [t["m_Size"]["x"], t["m_Size"]["y"]] if "m_Size" in t else None}
            sp = resolve(co, t["m_Sprite"])
            if sp is not None:
                st = sp.read_typetree()
                comp["ppu"] = st["m_PixelsToUnits"]
                comp["pivot"] = [st["m_Pivot"]["x"], st["m_Pivot"]["y"]]
                comp["rect"] = [st["m_Rect"]["width"], st["m_Rect"]["height"]]
            n["comps"].append(comp)
        elif tn == "Camera":
            t = co.read_typetree()
            n["comps"].append({"type": "Camera", "id": co.path_id, "orthographic": bool(t.get("orthographic", 0)),
                               "orthoSize": t.get("orthographic size"), "fov": t.get("field of view"),
                               "near": t.get("near clip plane"), "far": t.get("far clip plane"),
                               "background": color(t["m_BackGroundColor"]) if "m_BackGroundColor" in t else None})
        else:
            n["comps"].append({"type": tn, "id": co.path_id})
    if tr_obj is not None:
        n["transform_id"] = tr_obj.path_id
        tr = tr_obj.read_typetree()
        q = tr["m_LocalRotation"]
        rot_z = math.degrees(2 * math.atan2(q["z"], q["w"]))
        s = tr["m_LocalScale"]
        if tr_obj.type.name == "RectTransform":
            v2 = lambda d: [round(d["x"], 3), round(d["y"], 3)]
            n["rect"] = {"anchor_min": v2(tr["m_AnchorMin"]), "anchor_max": v2(tr["m_AnchorMax"]), "pos": v2(tr["m_AnchoredPosition"]),
                         "size": v2(tr["m_SizeDelta"]), "pivot": v2(tr["m_Pivot"]), "scale": [round(s["x"], 3), round(s["y"], 3)],
                         "rot_z": round(rot_z, 2)}
        else:
            # Plain Transform (world-space objects): full local position, rotation and scale.
            p3 = tr["m_LocalPosition"]
            n["transform"] = {"pos": [p3["x"], p3["y"], p3["z"]], "rot": [q["x"], q["y"], q["z"], q["w"]],
                              "scale": [s["x"], s["y"], s["z"]]}
        for ch in tr["m_Children"]:
            cto = resolve(tr_obj, ch)
            if cto is None:
                continue
            ct = cto.read_typetree()
            cgo = resolve(cto, ct["m_GameObject"])
            if cgo is not None:
                n["children"].append(node(cgo))
    return n


def find_pid(pid):
    for objs in by_file.values():
        if pid in objs:
            return objs[pid]
    return None


os.makedirs(OUT, exist_ok=True)
containers = dict(env.container.items())
for p in PATHS:
    if p.startswith("scenefile:"):
        # All root objects of a scene by its path: "scenefile:Assets/UI - New/Scenes/X.unity=<out>".
        spec, _, alias = p[10:].partition("=")
        cab = next((v for o in env.objects if o.type.name == "AssetBundle"
                    for k, v in o.read_typetree().get("m_SceneHashes", []) if k == spec), None)
        sf = files.get((cab or "").lower())
        if sf is None:
            print(p, "not found")
            continue
        roots = []
        for o in sf.objects.values():
            if o.type.name in ("RectTransform", "Transform"):
                t = o.read_typetree()
                if t["m_Father"]["m_PathID"] == 0:
                    roots.append(resolve(o, t["m_GameObject"]))
        tree = {"name": alias, "active": True, "id": 0, "comps": [], "children": [node(r) for r in roots]}
        json.dump(tree, open(os.path.join(OUT, alias + ".json"), "w"), indent=1)
        print(p, sf.name, len(roots), "roots")
        continue
    if p.startswith("scene:"):
        # All root objects of the scene file containing a GameObject named <name>: "scene:<name>=<out>".
        spec, _, alias = p[6:].partition("=")
        for sf in files.values():
            gos = [o for o in sf.objects.values() if o.type.name == "GameObject"]
            if not any(o.read_typetree()["m_Name"] == spec for o in gos):
                continue
            roots = []
            for o in sf.objects.values():
                if o.type.name in ("RectTransform", "Transform"):
                    t = o.read_typetree()
                    if t["m_Father"]["m_PathID"] == 0:
                        roots.append(resolve(o, t["m_GameObject"]))
            tree = {"name": alias or spec, "active": True, "id": 0, "comps": [], "children": [node(r) for r in roots]}
            json.dump(tree, open(os.path.join(OUT, (alias or spec) + ".json"), "w"), indent=1)
            print(p, sf.name, len(roots), "roots")
            break
        continue
    if p.startswith("field:"):
        # Object referenced by a script field of a named GameObject: "field:<GameObject>/<Script>.<field>=<out>".
        spec, _, alias = p[6:].partition("=")
        go_name, _, rest = spec.partition("/")
        cls, _, field = rest.partition(".")
        root_obj = None
        for sf in files.values():
            for o in sf.objects.values():
                if o.type.name != "GameObject":
                    continue
                go = o.read_typetree()
                if go["m_Name"] != go_name:
                    continue
                for c in go["m_Component"]:
                    co = resolve(o, c["component"])
                    if co is None or co.type.name != "MonoBehaviour":
                        continue
                    t = co.read_typetree()
                    if field in t and script_name(co, t["m_Script"]) == cls:
                        root_obj = resolve(co, t[field])
                        break
                if root_obj is not None:
                    break
            if root_obj is not None:
                break
        if root_obj is None:
            print("not found", p)
            continue
        if root_obj.type.name != "GameObject":
            root_obj = resolve(root_obj, root_obj.read_typetree()["m_GameObject"])
        tree = node(root_obj)
        name = alias or tree["name"]
    elif p.startswith("pid:"):
        # Object referenced by id only (e.g. ShopMenu.towerButtonPrefab): "pid:<id>[=<name>]".
        spec, _, alias = p[4:].partition("=")
        root_obj = find_pid(int(spec))
        if root_obj is None:
            print("not found", p)
            continue
        if root_obj.type.name != "GameObject":
            root_obj = resolve(root_obj, root_obj.read_typetree()["m_GameObject"])
        tree = node(root_obj)
        name = alias or tree["name"]
    else:
        root = containers[p]
        root_obj = root.deref() if hasattr(root, "deref") else root
        tree = node(root_obj)
        name = os.path.splitext(os.path.basename(p))[0]
    json.dump(tree, open(os.path.join(OUT, name + ".json"), "w"), indent=1)

    def count(n):
        return 1 + sum(count(c) for c in n["children"])
    print(p, "->", name + ".json", count(tree), "nodes")
