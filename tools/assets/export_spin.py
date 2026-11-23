"""Spinning display parts (RotateDisplayModel: helicopter rotors) for exported models.

    python tools/assets/export_spin.py [--game <install>] [air ...]

The game turns these in code, not by animation: RotateDisplayModel.Update (@0x1804ee020) adds
deltaTime x rotationsPerSecond x 360 to currentRotation (from startingRotation) and sets the
object's localEulerAngles to (0, currentRotation - its grandparent's world yaw, 0): a turn about the
vertical. For every exported model folder whose prefab has one, writes <folder>/spin.json:
{"parts": [{"index": part index in meta.json, "rps": ..., "start": ...}, ...]}: every renderer at
or under a spinning object, matched to meta.json's parts in hierarchy order (the exporter's).
"""
import glob
import json
import os
import sys
from collections import defaultdict
from pathlib import Path

import UnityPy

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "local_extracted"
AA = Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)")) / "Steam/steamapps/common/BloonsTD6/BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full"


def walk(go, spin, out):
    """Renderers in hierarchy order with the spin (rps, start) they're under, if any."""
    comps = [c.component if hasattr(c, "component") else c for c in go.m_Component]
    renderer = False
    for ptr in comps:
        t = ptr.type.name
        if t in ("MeshRenderer", "SkinnedMeshRenderer"):
            renderer = True
        elif t == "MonoBehaviour":
            try:
                mb = ptr.read()
                if mb.m_Script.read().m_ClassName == "RotateDisplayModel":
                    tt = ptr.read_typetree()
                    spin = (float(tt["rotationsPerSecond"]), float(tt["startingRotation"]))
            except Exception:
                pass
    if renderer:
        out.append((go.m_Name, spin))
    for ch in go.m_Transform.read().m_Children:
        walk(ch.read().m_GameObject.read(), spin, out)


def main():
    global AA
    args = sys.argv[1:]
    if args[:1] == ["--game"]:
        AA = Path(args[1]) / "BloonsTD6_Data/StreamingAssets/aa/StandaloneWindows64/Full"
        args = args[2:]
    dirs = [OUT / "models" / d for d in (args or ["air"])]
    folders = [f for d in dirs for f in sorted(d.iterdir()) if (f / "meta.json").exists()]
    paths = json.loads((OUT / "all_container_paths.json").read_text(encoding="utf-8"))
    by_bundle = defaultdict(list)
    for f in folders:
        cand = sorted((p for p in paths if p.endswith("/" + f.name + ".prefab")), key=lambda p: ("HeliPilot" not in p, p))
        if cand:
            by_bundle[paths[cand[0]]].append((f, cand[0]))
    mono = [str(p) for p in AA.glob("*monoscripts*.bundle")]
    written = 0
    for bundle, items in by_bundle.items():
        env = UnityPy.load(str(AA / bundle), *mono)
        cont = {k.lower(): v for k, v in env.container.items()}
        for f, path in items:
            obj = cont.get(path.lower())
            if obj is None:
                continue
            found = []
            walk(obj.read(), None, found)
            if not any(s for _, s in found):
                continue
            parts = [p["name"] for p in json.loads((f / "meta.json").read_text(encoding="utf-8"))["parts"]]
            # Exported parts in hierarchy order (some renderers may be left out): match in sequence.
            j, out = 0, []
            for i, name in enumerate(parts):
                while j < len(found) and found[j][0] != name:
                    j += 1
                if j == len(found):
                    out = None
                    break
                if found[j][1]:
                    out.append({"index": i, "rps": found[j][1][0], "start": found[j][1][1]})
                j += 1
            if out is None:
                print(f.name, "parts don't follow the prefab's renderers; skipped", parts, [n for n, _ in found])
                continue
            if out:
                (f / "spin.json").write_text(json.dumps({"parts": out}), encoding="utf-8")
                written += 1
                print(f.name, [(parts[p["index"]], p["rps"]) for p in out])
    print(f"spin.json written for {written} models")


if __name__ == "__main__":
    main()
