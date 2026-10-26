"""Disassemble one IL2CPP method (by Type::Method) with dumpbin and annotate it.

Annotations: direct call targets resolved to method names, BinaryReader
vtable offsets resolved to method names (table below, verified by
cross-checking reader methods), and field offsets resolved to field names
when --fields Type is given (offsets come from the instance layout of the
Il2CppInterop proxy, see notes in research/04-native-analysis.md).

    python -I disasm.py <Type> <Method> [--raw]
"""
import bisect
import json
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(__file__))
from codegen import PE64  # noqa: E402
from calls import pdata  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
GAME = r"C:\Program Files\Epic Games\BloonsTD6"
DUMPBIN = r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\dumpbin.exe"

# BinaryReader vtable offsets (Il2CppClass vtable entries are 16 bytes:
# methodPtr, MethodInfo*). Verified: 0x228 in Read_a_Int32_Array,
# 0x1C8/0x298 in Read_a_String_Array. Others filled in as confirmed.
BR_VT = {0x1C8: "ReadBoolean", 0x228: "ReadInt32", 0x298: "ReadString"}

_amap = None


def amap():
    global _amap
    if _amap is None:
        raw = json.load(open(os.path.join(ROOT, "research", "dumps", "addr_map.json")))
        _amap = {int(k) + 0x180000000: v.split("!", 1)[1] for k, v in raw.items()}
    return _amap


def find(type_name, method):
    for va, n in amap().items():
        if n == f"{type_name}::{method}" or n.endswith(f".{type_name}::{method}"):
            return va
    raise KeyError(f"{type_name}::{method}")


def disasm_va(va):
    pe = PE64(os.path.join(GAME, "GameAssembly.dll"))
    f = pdata(pe)
    st = [x[0] for x in f]
    rva = va - pe.base
    k = bisect.bisect_right(st, rva) - 1
    if st[k] == rva:
        b, e = f[k][0] + pe.base, f[k][1] + pe.base
    else:
        # Leaf functions have no .pdata entry: run to the next known
        # function start (pdata entry or mapped method), whichever is first.
        nxt = st[k + 1]
        later = [a - pe.base for a in amap() if a > va]
        if later:
            nxt = min(nxt, min(later))
        b, e = va, nxt + pe.base
    out = subprocess.run([DUMPBIN, "/nologo", "/disasm:nobytes", f"/range:{b:#x},{e:#x}",
                          os.path.join(GAME, "GameAssembly.dll")], capture_output=True, text=True).stdout
    return [l.strip() for l in out.splitlines() if re.match(r"\s+[0-9A-F]{16}:", l)]


def annotate(lines):
    names = amap()
    res = []
    for l in lines:
        m = re.search(r"call\s+([0-9A-F]{16})", l)
        if m and int(m.group(1), 16) in names:
            l += "   ; " + names[int(m.group(1), 16)]
        m = re.search(r"mov\s+rax,qword ptr \[rdx\+0*([0-9A-F]+)h\]", l)
        if m and int(m.group(1), 16) in BR_VT:
            l += "   ; BinaryReader." + BR_VT[int(m.group(1), 16)]
        res.append(l)
    return res


def disasm_to_next(va):
    """Whole method including split .pdata chunks: from va to the next
    mapped method start."""
    later = sorted(a for a in amap() if a > va)
    end = later[0] if later else va + 0x1000
    out = subprocess.run([DUMPBIN, "/nologo", "/disasm:nobytes", f"/range:{va:#x},{end:#x}",
                          os.path.join(GAME, "GameAssembly.dll")], capture_output=True, text=True).stdout
    return [l.strip() for l in out.splitlines() if re.match(r"\s+[0-9A-F]{16}:", l)]


NOISE = re.compile(r"int\s+3$|call\s+0000000180431150|E4h\],0$|call\s+0000000180430E70|lea\s+rcx,\[0+18[45]")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if "--va" in sys.argv:
        va = int(args[0], 16)
    else:
        va = find(args[0], args[1])
    lines = disasm_to_next(va) if "--to-next" in sys.argv else disasm_va(va)
    for l in annotate(lines):
        if "--clean" in sys.argv:
            if NOISE.search(l):
                continue
            l = l.split(": ", 1)[1] if ": " in l else l
        print(l)


if __name__ == "__main__":
    main()
