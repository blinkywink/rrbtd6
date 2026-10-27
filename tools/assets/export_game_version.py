"""Export the installed BTD6's display version (local only).

Game.GetLocalizedDisplayVersion shows Application.version followed by a build suffix that is a
string literal in the code (".12224" in 57.4), through the "Copyright" text. The version comes
from globalgamemanagers (PlayerSettings.bundleVersion, the first length-prefixed version after
the product name); the suffix is the literal that method loads, resolved from the IL2CPP
metadata usage it references.

Usage: python -I export_game_version.py GAME METHODS_JSONL OUT_JSON
"""
import bisect
import json
import re
import struct
import sys
from pathlib import Path

import capstone

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "il2cpp"))
from codegen import PE64  # noqa: E402

game, methods, out = map(Path, sys.argv[1:4])
data = game / "BloonsTD6_Data"

ggm = (data / "globalgamemanagers").read_bytes()
product = ggm.find(b"BloonsTD6")
# PlayerSettings stores a run of length-prefixed version strings after the product name (two
# platform defaults of "1.0", then bundleVersion); the game's is the highest of that run.
found = [m for m in re.finditer(rb"([\x03-\x10])\x00\x00\x00(\d+\.\d+(?:\.\d+)*)", ggm[max(product, 0):])
         if len(m.group(2)) == m.group(1)[0]]
run = []
for m in found:
    if run and m.start() - run[-1].end() > 4:
        break
    run.append(m)
version = max((m.group(2).decode() for m in run), key=lambda v: tuple(map(int, v.split("."))), default=None)
if version is None:
    sys.exit("bundleVersion not found in globalgamemanagers")

pe = PE64(str(game / "GameAssembly.dll"))
meta = (data / "il2cpp_data/Metadata/global-metadata.dat").read_bytes()
lit_off, lit_size, _ = struct.unpack_from("<3i", meta, 8)
str_base, str_len, _ = struct.unpack_from("<3i", meta, 20)
starts = struct.unpack_from(f"<{lit_size // 4}I", meta, lit_off)
bounds = sorted(set(starts) | {str_len})


def literal(slot_va):
    """A string-literal metadata usage slot (still encoded in the file) -> its text."""
    enc = struct.unpack_from("<Q", pe.d, pe.rva_to_off(slot_va - pe.base))[0]
    if enc >> 29 != 5 or not enc & 1:
        return None
    s = starts[(enc & 0x1FFFFFFF) >> 1]
    return meta[str_base + s:str_base + bounds[bisect.bisect_right(bounds, s)]].decode("utf-8", "replace")


va = next(int(r["va"], 16) for r in map(json.loads, methods.open(encoding="utf-8"))
          if r["type"] == "Assets.Scripts.Unity.Game" and r["name"] == "GetLocalizedDisplayVersion" and r["va"])
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
md.detail = True
off = pe.rva_to_off(va - pe.base)
build = ""
for ins in md.disasm(pe.d[off:off + 0x200], va):
    if ins.mnemonic == "ret":
        break
    for op in ins.operands:
        if op.type == capstone.x86.X86_OP_MEM and op.mem.base == capstone.x86.X86_REG_RIP:
            text = literal(ins.address + ins.size + op.mem.disp)
            if text is not None and re.fullmatch(r"\.\d+", text):
                build = text
    if build:
        break

out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps({"version": version, "build": build, "display": version + build}), encoding="utf-8")
print("BTD6 version", version + build)
