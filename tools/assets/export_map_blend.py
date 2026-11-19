"""Each map scene's transparent materials, for the renderer to draw blended instead of opaque.

    tools/.venv/Scripts/python.exe -I tools/assets/export_map_blend.py <game install> [map key ...]

Reads every Material in map_<key>_*.bundle and writes local_extracted/models/map_<key>/blend.json,
{material name: "alpha" | "add"}, for materials Unity draws transparent: URP _Surface 1, a
transparent keyword, or render queue >= 2500 (_DstBlend One: additive, else alpha). Opaque
materials are left out. (Midnight Mansion's LightningStrike window flash, URP Unlit transparent at
29% alpha, drew as a solid cyan block.) Proprietary data: local use only.
"""
import glob
import json
import os
import sys

import UnityPy

GAME = sys.argv[1]
AA = os.path.join(GAME, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
L = "local_extracted"


def blend_of(t):
    sp = t["m_SavedProperties"]
    floats = dict(sp.get("m_Floats", []))
    kw = " ".join(t.get("m_ValidKeywords") or []) + " " + str(t.get("m_ShaderKeywords") or "")
    queue = t.get("m_CustomRenderQueue", -1)
    transparent = floats.get("_Surface", 0.0) == 1.0 or "TRANSPARENT" in kw.upper() or queue >= 2500
    if not transparent:
        return None
    return "add" if floats.get("_DstBlend") == 1.0 else "alpha"


def main():
    keys = sys.argv[2:] or sorted(d[4:] for d in os.listdir(os.path.join(L, "models")) if d.startswith("map_"))
    for key in keys:
        out_dir = os.path.join(L, "models", f"map_{key}")
        bundles = glob.glob(os.path.join(AA, f"map_{key}_*.bundle"))
        if not bundles or not os.path.isdir(out_dir):
            continue
        try:
            env = UnityPy.load(*bundles)
        except Exception as e:
            print(key, "load failed:", type(e).__name__, e, flush=True)
            continue
        found = {}
        for o in env.objects:
            if o.type.name != "Material":
                continue
            try:
                t = o.read_typetree()
            except Exception:
                continue
            b = blend_of(t)
            if b:
                found[t["m_Name"]] = b
        json.dump(found, open(os.path.join(out_dir, "blend.json"), "w"), indent=1, sort_keys=True)
        print(key, len(found), "transparent materials", flush=True)
        del env


if __name__ == "__main__":
    main()
