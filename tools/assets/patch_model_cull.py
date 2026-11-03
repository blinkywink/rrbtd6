"""Add each model part's shader Cull (export_model.shader_cull) to models exported before the
exporters recorded it, matching parts to the game's materials by name.

    python tools/assets/patch_model_cull.py <game dir> <local_extracted>
"""
import glob
import json
import os
import sys

GAME, LE = sys.argv[1:3]
sys.argv = [sys.argv[0], GAME, os.path.join(LE, "models"), "prefab", ""]  # export_model's argv
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import export_model as em  # noqa: E402
import UnityPy  # noqa: E402

metas = [f for f in glob.glob(os.path.join(LE, "models", "**", "meta.json"), recursive=True)]
wanted = set()
for f in metas:
    try:
        wanted |= {p.get("material") for p in json.load(open(f, encoding="utf-8")).get("parts", [])}
    except (OSError, ValueError):
        pass

# Material name -> the distinct culls of the game's materials of that name.
culls = {}
for bundle in sorted(glob.glob(os.path.join(em.AA, "*.bundle"))):
    name = os.path.basename(bundle)
    env = em._envs.get(name) or UnityPy.load(bundle)
    em._envs[name] = env
    for o in env.objects:
        if o.type.name != "Material":
            continue
        mt = o.read_typetree()
        if mt["m_Name"] not in wanted:
            continue
        c = em.shader_cull(o, mt).get("cull")
        if c is not None:
            culls.setdefault(mt["m_Name"], set()).add(json.dumps(c))
    print(name, len(culls), flush=True)
    em._envs.clear()  # one bundle (and what it references) loaded at a time

patched = ambiguous = missing = 0
for f in metas:
    m = json.load(open(f, encoding="utf-8"))
    changed = False
    for p in m.get("parts", []):
        c = culls.get(p.get("material"))
        if not c:
            missing += 1
        elif len(c) > 1:
            ambiguous += 1
            print("ambiguous", f, p.get("material"), sorted(c))
        elif p.get("cull") != json.loads(next(iter(c))):
            p["cull"] = json.loads(next(iter(c)))
            changed = True
    if changed:
        json.dump(m, open(f, "w", encoding="utf-8"))
        patched += 1
print(f"patched {patched} of {len(metas)} models; parts without a game material {missing}, ambiguous {ambiguous}")
