"""Find direct callers (E8 rel32 / E9 rel32) of a native address.

    python -I xrefs.py <target_va_hex> [...]

Scans executable sections; each hit is attributed to its containing .pdata
function and, when known, the IL2CPP method name.
"""
import bisect
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from codegen import PE64  # noqa: E402
from calls import pdata  # noqa: E402
import disasm  # noqa: E402


def main():
    pe = PE64(os.path.join(disasm.GAME, "GameAssembly.dll"))
    funcs = pdata(pe)
    starts = [f[0] for f in funcs]
    names = disasm.amap()
    targets = {int(a, 16) for a in sys.argv[1:]}
    for name, va, vs, rp, rs in pe.sections:
        if name not in (".text", "il2cpp"):
            continue
        d = pe.d[rp:rp + rs]
        for op in (b"\xE8", b"\xE9"):
            i = d.find(op)
            while i != -1:
                if i + 5 <= len(d):
                    rel = struct.unpack_from("<i", d, i + 1)[0]
                    src = pe.base + va + i
                    t = src + 5 + rel
                    if t in targets:
                        k = bisect.bisect_right(starts, src - pe.base) - 1
                        fs = funcs[k][0] + pe.base
                        print(f"{op.hex()} from {src:#x} in fn {fs:#x} {names.get(fs, '?')} -> {t:#x}")
                i = d.find(op, i + 1)


if __name__ == "__main__":
    main()
