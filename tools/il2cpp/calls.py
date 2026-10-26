"""Build an address map (native VA -> IL2CPP method) and list the direct
calls (E8 rel32) and vtable calls (FF 50/90 [reg+disp]) inside a method.

    python -I calls.py <GameAssembly.dll> <global-metadata.dat> <Type> <Method> [--bytes]

Function bounds come from .pdata RUNTIME_FUNCTION entries, so the scan never
leaves the function. A byte-level E8 scan can hit false positives inside
other instructions; a hit is only reported when its target is a known method
start or a .pdata function start.
"""
import bisect
import json
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from codegen import PE64, find_module, method_pointers  # noqa: E402
from metadata_v39 import Metadata  # noqa: E402


def pdata(pe):
    for name, va, vs, rp, rs in pe.sections:
        if name == ".pdata":
            n = vs // 12
            return [struct.unpack_from("<III", pe.d, rp + 12 * i) for i in range(n)]
    raise RuntimeError("no .pdata")


def build_map(pe, md, cache):
    if os.path.exists(cache):
        return {int(k): v for k, v in json.load(open(cache)).items()}
    by_image = {}
    for row in md.iter_methods():
        by_image.setdefault(row[0], []).append(row)
    amap = {}
    for img in md.images:
        iname = md.string(img["nameIndex"])
        rows = by_image.get(iname, [])
        if not rows:
            continue
        mods = find_module(pe, iname, len(rows))
        if not mods:
            print("no codegen module for", iname, file=sys.stderr)
            continue
        ptrs = method_pointers(pe, mods[0])
        for _, tn, mn, tok, *_ in rows:
            rid = tok & 0xFFFFFF
            va = ptrs[rid - 1]
            if va:
                amap.setdefault(va - pe.base, f"{iname}!{tn}::{mn}")
    json.dump(amap, open(cache, "w"))
    return amap


def main():
    pe = PE64(sys.argv[1])
    md = Metadata(sys.argv[2])
    tname, mname = sys.argv[3], sys.argv[4]
    cache = os.path.join(os.path.dirname(__file__), "..", "..", "research", "dumps", "addr_map.json")
    amap = build_map(pe, md, cache)
    funcs = pdata(pe)
    starts = [f[0] for f in funcs]
    targets = [rva for rva, n in amap.items() if n.endswith(f"!{tname}::{mname}") or n.endswith(f".{tname}::{mname}")]
    print(len(amap), "mapped methods;", len(targets), "matches")
    for rva in targets:
        k = bisect.bisect_right(starts, rva) - 1
        b, e, _ = funcs[k]
        print(f"== {amap[rva]} rva={rva:#x} pdata=[{b:#x},{e:#x}) size={e - b}")
        off = pe.rva_to_off(b)
        code = pe.d[off:off + (e - b)]
        for i in range(len(code) - 4):
            if code[i] == 0xE8:
                rel = struct.unpack_from("<i", code, i + 1)[0]
                t = b + i + 5 + rel
                name = amap.get(t)
                if name or (bisect.bisect_left(starts, t) < len(starts) and starts[bisect.bisect_left(starts, t)] == t):
                    print(f"  +{i:#06x} call {t:#x} {name or '(fn)'}")
            elif code[i] == 0xFF and code[i + 1] in (0x50, 0x51, 0x52, 0x53, 0x56, 0x57):
                print(f"  +{i:#06x} vcall [r+{code[i + 2]:#x}]")
            elif code[i] == 0xFF and code[i + 1] in (0x90, 0x91, 0x92, 0x93, 0x96, 0x97):
                print(f"  +{i:#06x} vcall [r+{struct.unpack_from('<i', code, i + 2)[0]:#x}]")
        if "--bytes" in sys.argv:
            print(code.hex())


if __name__ == "__main__":
    main()
