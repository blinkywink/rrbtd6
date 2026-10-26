"""Export round sets into a compact TSV for btd6-sim (engine/src/data.rs).

Input:  local_extracted/data/round_sets.json, from the Data.Gameplay.RoundSet
        MonoBehaviours (written by tools/assets/export_rounds.py)
Output: local_extracted/engine_data/round_sets.tsv  (proprietary: git-ignored)

The values stay in the MonoBehaviour's units (seconds, start/duration). The
conversion to BloonGroupModel frames is done by the engine with the
transcribed formula (RoundData.GetRoundDef lambda @0x1813358C0):
    startFrame = (scale * start) * 60f;  endFrame = (scale * (duration + start)) * 60f

    python -I tools/data_export/export_rounds.py
"""
import json
import os
import struct

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
SRC = os.path.join(ROOT, "local_extracted", "data", "round_sets.json")
OUT = os.path.join(ROOT, "local_extracted", "engine_data", "round_sets.tsv")


def f32hex(v):
    return "0x%08X" % struct.unpack("<I", struct.pack("<f", v))[0]


def main():
    sets = json.load(open(SRC, encoding="utf-8"))
    lines = 0
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("# Round sets (Data.Gameplay.RoundSet). Proprietary game data: local use only.\n")
        f.write("set\tscale\tround\tgroup\tbloon\tcount\tstart\tduration\n")
        for name in sorted(sets):
            s = sets[name]
            for ri, r in enumerate(s["rounds"]):
                for gi, g in enumerate(r.get("bloonGroups") or []):
                    f.write("\t".join([name, f32hex(s["scale"]), str(ri), str(gi), g["bloon"], str(g["count"]),
                                       f32hex(g["start"]), f32hex(g["duration"])]) + "\n")
                    lines += 1
    print(len(sets), "sets,", lines, "groups ->", OUT)


if __name__ == "__main__":
    main()
