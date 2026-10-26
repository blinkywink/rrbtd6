"""Add TMP vertex colour gradients to already dumped UI JSON (local only).

dump_ui_prefab.py now writes "gradient"/"colorMode" itself; this backfills existing dumps
without re-running every export. Components are matched by MonoBehaviour path id and m_text.
Usage: python patch_tmp_gradients.py GAME ROOT [BUNDLE_OR_CAB ...]
Default sources: ui_scenes_all_*.bundle. Pass a pre-extracted asset_references CAB too.
"""
import glob
import json
import os
import sys
from pathlib import Path
import UnityPy
from UnityPy.helpers import TypeTreeHelper
TypeTreeHelper.read_typetree_boost = False

game, root, *sources = sys.argv[1:]
aa = os.path.join(game, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
monoscripts = glob.glob(os.path.join(aa, "*_monoscripts_*.bundle"))[0]
sources = sources or ["ui_scenes_all_*.bundle"]


def color(c):
    return [round(c["r"], 4), round(c["g"], 4), round(c["b"], 4), round(c["a"], 4)]


gradients = {}
for source in sources:
    path = source if os.path.isabs(source) else glob.glob(os.path.join(aa, source))[0]
    env = UnityPy.load(monoscripts, path)
    for obj in env.objects:
        if obj.type.name != "MonoBehaviour":
            continue
        try:
            t = obj.read_typetree()
        except Exception:
            continue
        if "m_text" in t and "m_fontAsset" in t and t.get("m_enableVertexGradient"):
            g = t["m_fontColorGradient"]
            gradients[(obj.path_id, t["m_text"])] = {
                "gradient": [color(g[k]) for k in ("topLeft", "topRight", "bottomLeft", "bottomRight")],
                "colorMode": t.get("m_colorMode")}
    print(Path(path).name, len(gradients), "gradient texts so far")


def walk(n):
    count = 0
    for c in n.get("comps", []):
        if c.get("type") == "TextMeshProUGUI":
            g = gradients.get((c.get("id"), c.get("m_text", "")))
            if g and c.get("gradient") != g["gradient"]:
                c.update(g)
                count += 1
    for ch in n.get("children", []):
        count += walk(ch)
    return count


total = 0
for file in sorted(Path(root, "ui/prefabs").glob("*.json")):
    data = json.loads(file.read_text(encoding="utf-8"))
    changed = sum(walk(n) for n in (data if isinstance(data, list) else [data]))
    if changed:
        file.write_text(json.dumps(data, indent=1), encoding="utf-8")
        print(file.name, changed)
        total += changed
print("patched", total)
