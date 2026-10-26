"""Locate IL2CPP Il2CppCodeGenModule structs in GameAssembly.dll (x64 PE).

Il2CppCodeGenModule begins with { const char* moduleName; size_t
methodPointerCount; Il2CppMethodPointer* methodPointers; ... }. We find the
module-name string, then pointers to it, and check the count against the
metadata. Read-only; never loads or executes the DLL.

    python -I codegen.py <GameAssembly.dll> <module.dll> [expected_count]
"""
import struct
import sys


class PE64:
    def __init__(self, path):
        d = self.d = open(path, "rb").read()
        pe = struct.unpack_from("<I", d, 0x3C)[0]
        nsec = struct.unpack_from("<H", d, pe + 6)[0]
        optsz = struct.unpack_from("<H", d, pe + 20)[0]
        opt = pe + 24
        assert struct.unpack_from("<H", d, opt)[0] == 0x20B, "not PE32+"
        self.base = struct.unpack_from("<Q", d, opt + 24)[0]
        self.sections = []
        s = opt + optsz
        for i in range(nsec):
            name = d[s + i * 40:s + i * 40 + 8].rstrip(b"\0").decode()
            vsize, va, rsize, rptr = struct.unpack_from("<IIII", d, s + i * 40 + 8)
            self.sections.append((name, va, vsize, rptr, rsize))

    def rva_to_off(self, rva):
        for _, va, vs, rp, rs in self.sections:
            if va <= rva < va + max(vs, rs):
                return rva - va + rp
        return None

    def off_to_rva(self, off):
        for _, va, vs, rp, rs in self.sections:
            if rp <= off < rp + rs:
                return off - rp + va
        return None

    def q(self, rva):
        o = self.rva_to_off(rva)
        return struct.unpack_from("<Q", self.d, o)[0]


def find_module(pe, module, expected=None):
    needle = module.encode() + b"\0"
    hits = []
    start = 0
    while (i := pe.d.find(needle, start)) != -1:
        start = i + 1
        if i > 0 and pe.d[i - 1] != 0:
            continue
        rva = pe.off_to_rva(i)
        if rva is None:
            continue
        va = pe.base + rva
        pat = struct.pack("<Q", va)
        s2 = 0
        while (j := pe.d.find(pat, s2)) != -1:
            s2 = j + 1
            if j % 8:
                continue
            count, ptrs = struct.unpack_from("<QQ", pe.d, j + 8)
            if expected is not None and count != expected:
                continue
            if 0 < count < 1_000_000:
                hits.append({"struct_rva": pe.off_to_rva(j), "count": count, "methodPointers_va": ptrs})
    return hits


def method_pointers(pe, mod):
    rva = mod["methodPointers_va"] - pe.base
    off = pe.rva_to_off(rva)
    return list(struct.unpack_from(f"<{mod['count']}Q", pe.d, off))


def main():
    pe = PE64(sys.argv[1])
    exp = int(sys.argv[3]) if len(sys.argv) > 3 else None
    print("image base", hex(pe.base), [s[0] for s in pe.sections])
    for h in find_module(pe, sys.argv[2], exp):
        print({k: hex(v) if isinstance(v, int) and v > 1_000_000 else v for k, v in h.items()})


if __name__ == "__main__":
    main()
