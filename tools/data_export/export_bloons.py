"""Export BloonModel data from the decoded sim_model into a compact TSV that
btd6-sim loads (engine/src/data.rs). Floats are written as raw f32 hex bits
so nothing is lost in conversion.

Input:  local_extracted/decoded/named/{objects.jsonl,containers.json}
        (written by the sim_model decoder, tools/sim_model/)
Output: local_extracted/engine_data/bloons.tsv  (proprietary data: git-ignored)

    python -I tools/data_export/export_bloons.py
"""
import json
import os
import struct

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SRC = os.path.join(ROOT, "local_extracted", "decoded", "named")
OUT = os.path.join(ROOT, "local_extracted", "engine_data")

COLUMNS = ["id", "speed", "radius", "maxHealth", "leakDamage", "leakDamageSet", "leakDamageMultiplier",
           "isMoab", "isBoss", "rotateToFollowPath", "isImmuneToSlow", "children"]


def f32hex(v):
    return "0x%08X" % struct.unpack("<I", struct.pack("<f", v))[0]


def main():
    objs = {}
    with open(os.path.join(SRC, "objects.jsonl"), encoding="utf-8") as f:
        for line in f:
            o = json.loads(line)
            objs[o["i"]] = o
    containers = json.load(open(os.path.join(SRC, "containers.json"), encoding="utf-8"))

    def items(ref):
        if not ref:
            return []
        c = containers.get(str(ref["$ref"]))
        if c is None:
            return []
        return c if isinstance(c, list) else c.get("items", [])

    rows = []
    for o in objs.values():
        if o["type"] != "BloonModel":
            continue
        f = o["f"]
        children = []
        for bi in items(f.get("behaviors")):
            b = objs.get(bi) if isinstance(bi, int) else None
            if b and b["type"] == "SpawnChildrenModel":
                children = [c for c in items(b["f"].get("children")) if isinstance(c, str)]
                break
        rows.append([
            f["id"], f32hex(f["speed"]), f32hex(f["radius"]), str(f["maxHealth"]),
            f32hex(f["leakDamage"]), f32hex(f["leakDamageSet"]), f32hex(f["leakDamageMultiplier"]),
            str(int(f["isMoab"])), str(int(f["isBoss"])), str(int(f["rotateToFollowPath"])),
            str(int(f["isImmuneToSlow"])), ",".join(children),
        ])
    rows.sort(key=lambda r: r[0])
    os.makedirs(OUT, exist_ok=True)
    out = os.path.join(OUT, "bloons.tsv")
    with open(out, "w", encoding="utf-8", newline="\n") as f:
        f.write("# BloonModel export from sim_model. Proprietary game data: local use only.\n")
        f.write("\t".join(COLUMNS) + "\n")
        for r in rows:
            f.write("\t".join(r) + "\n")
    print(len(rows), "bloon models ->", out)


if __name__ == "__main__":
    main()
