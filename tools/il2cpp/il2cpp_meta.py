"""IL2CPP global-metadata.dat (v24-v39) decoder + method address linker.

Layout logic ported from REA (morluto/rea, MIT) src/il2cpp/Il2cppMetadataFile.ts
and Il2cppBinaryRegistration.ts. Reads files only; never executes the target.

Usage:
    python -I il2cpp_meta.py <global-metadata.dat> <GameAssembly.dll> <out.jsonl> [image ... | *]

Writes one JSON object per method of the selected images (default
Assembly-CSharp.dll): {"i", "type", "name", "token", "rva", "va"}.
"""
import json
import struct
import sys

SECTIONS = ["stringLiterals", "stringLiteralData", "strings", "events", "properties", "methods",
            "parameterDefaultValues", "fieldDefaultValues", "fieldAndParameterDefaultValueData",
            "fieldMarshaledSizes", "parameters", "fields", "genericParameters",
            "genericParameterConstraints", "genericContainers", "nestedTypes", "interfaces",
            "vtableMethods", "interfaceOffsets", "typeDefinitions", "images", "assemblies",
            "fieldRefs", "referencedAssemblies", "attributeData", "attributeDataRanges",
            "unresolvedIndirectCallParameterTypes", "unresolvedIndirectCallParameterRanges",
            "windowsRuntimeTypeNames", "windowsRuntimeStrings", "exportedTypeDefinitions"]


def width(n):
    return 1 if n <= 0xFF else 2 if n <= 0xFFFF else 4


def rd_index(b, off, w):
    if w == 1:
        v = b[off]
        return -1 if v == 0xFF else v
    if w == 2:
        v = struct.unpack_from("<H", b, off)[0]
        return -1 if v == 0xFFFF else v
    return struct.unpack_from("<i", b, off)[0]


class Metadata:
    def __init__(self, path):
        b = self.b = open(path, "rb").read()
        magic, ver = struct.unpack_from("<Ii", b, 0)
        assert magic == 0xFAB11BAF
        assert ver == 39, f"only v39 handled here (got {ver})"
        self.sec = {}
        for i, n in enumerate(SECTIONS):
            self.sec[n] = struct.unpack_from("<iii", b, 8 + 12 * i)
        cnt = lambda n: self.sec[n][2]
        rec = lambda n: self.sec[n][1] // self.sec[n][2] if self.sec[n][2] else None
        self.wTD = width(cnt("typeDefinitions"))
        self.wGC = width(cnt("genericContainers"))
        self.wP = width(cnt("parameters"))
        io = rec("interfaceOffsets")
        self.wT = io - 4 if io else (rec("typeDefinitions") - 68 - self.wGC) // 3
        self.tdsize = 68 + 3 * self.wT + self.wGC
        self.msize = 16 + 4 + self.wTD + self.wT + self.wP + self.wGC
        assert rec("typeDefinitions") == self.tdsize, (rec("typeDefinitions"), self.tdsize)
        assert rec("methods") == self.msize, (rec("methods"), self.msize)
        self.isize = 32 + 2 * self.wTD
        so = self.sec["strings"][0]
        self._s = so

    def string(self, idx):
        p = self._s + idx
        return self.b[p:self.b.index(b"\0", p)].decode("utf-8", "replace")

    def images(self):
        off, _, n = self.sec["images"]
        for i in range(n):
            at = off + i * self.isize
            yield (self.string(struct.unpack_from("<i", self.b, at)[0]),
                   rd_index(self.b, at + 8, self.wTD),
                   struct.unpack_from("<I", self.b, at + 8 + self.wTD)[0])

    def types(self):
        off, _, n = self.sec["typeDefinitions"]
        out = []
        for i in range(n):
            at = off + i * self.tdsize
            starts = at + 12 + 3 * self.wT + self.wGC
            c = starts + 32
            out.append(dict(
                name=self.string(struct.unpack_from("<i", self.b, at)[0]),
                ns=self.string(struct.unpack_from("<i", self.b, at + 4)[0]),
                field_start=struct.unpack_from("<i", self.b, starts)[0],
                method_start=struct.unpack_from("<i", self.b, starts + 4)[0],
                nested_start=struct.unpack_from("<i", self.b, starts + 16)[0],
                method_count=struct.unpack_from("<H", self.b, c)[0],
                field_count=struct.unpack_from("<H", self.b, c + 4)[0],
                nested_count=struct.unpack_from("<H", self.b, c + 8)[0],
                parent=-1))
        noff = self.sec["nestedTypes"][0]
        for i, t in enumerate(out):
            for k in range(t["nested_count"]):
                out[struct.unpack_from("<i", self.b, noff + 4 * (t["nested_start"] + k))[0]]["parent"] = i
        return out

    def methods(self):
        off, _, n = self.sec["methods"]
        tok_off = 4 + self.wTD + self.wT + 4 + self.wP + self.wGC
        for i in range(n):
            at = off + i * self.msize
            yield (self.string(struct.unpack_from("<i", self.b, at)[0]),
                   rd_index(self.b, at + 4, self.wTD),
                   struct.unpack_from("<I", self.b, at + tok_off)[0])


class PE:
    def __init__(self, path):
        d = self.d = open(path, "rb").read()
        pe = struct.unpack_from("<I", d, 0x3C)[0]
        nsec = struct.unpack_from("<H", d, pe + 6)[0]
        optsz = struct.unpack_from("<H", d, pe + 20)[0]
        opt = pe + 24
        self.base = struct.unpack_from("<Q", d, opt + 24)[0]
        self.secs = []
        s = opt + optsz
        for i in range(nsec):
            name = d[s + 40 * i:s + 40 * i + 8].rstrip(b"\0").decode()
            vsz, va, rsz, rptr = struct.unpack_from("<IIII", d, s + 40 * i + 8)
            ch = struct.unpack_from("<I", d, s + 40 * i + 36)[0]
            self.secs.append((name, va, vsz, rptr, rsz, ch))

    def va_to_off(self, va):
        rva = va - self.base
        for _, sva, vsz, rptr, rsz, _ in self.secs:
            if sva <= rva < sva + min(vsz, rsz):
                return rva - sva + rptr
        return None

    def is_exec(self, va):
        rva = va - self.base
        return any(sva <= rva < sva + vsz and ch & 0x20000000 for _, sva, vsz, _, _, ch in self.secs)


def find_module(pe, name, count):
    """Pointer-array VA of the Il2CppCodeGenModule {name*, count, pointers*} for `name`."""
    d = pe.d
    target = name.encode() + b"\0"
    name_vas = set()
    p = d.find(target)
    while p != -1:
        if p == 0 or d[p - 1] == 0:
            for _, sva, vsz, rptr, rsz, _ in pe.secs:
                if rptr <= p < rptr + rsz:
                    name_vas.add(pe.base + sva + p - rptr)
        p = d.find(target, p + 1)
    if not hasattr(pe, "_qword_index"):
        # One pass over non-executable sections: qword value -> file offsets.
        idx = {}
        for nm, sva, vsz, rptr, rsz, ch in pe.secs:
            if ch & 0x20000000:
                continue
            for k, (v,) in enumerate(struct.iter_unpack("<Q", d[rptr:rptr + (rsz // 8) * 8])):
                if v > pe.base:
                    idx.setdefault(v, []).append(rptr + 8 * k)
        pe._qword_index = idx
    hits = []
    for nva in name_vas:
        for at in pe._qword_index.get(nva, ()):
            if struct.unpack_from("<I", d, at + 8)[0] == count:
                arr = struct.unpack_from("<Q", d, at + 16)[0]
                if pe.va_to_off(arr) is not None:
                    hits.append(arr)
    assert len(hits) == 1, f"{name}: {len(hits)} module candidates"
    return hits[0]


def main():
    mpath, bpath, out = sys.argv[1:4]
    wanted = set(sys.argv[4:]) or {"Assembly-CSharp.dll"}
    md = Metadata(mpath)
    types = md.types()
    methods = list(md.methods())

    def full(ti):
        t = types[ti]
        if t["parent"] >= 0:
            return full(t["parent"]) + "/" + t["name"]
        return (t["ns"] + "." if t["ns"] else "") + t["name"]

    pe = PE(bpath)
    n = 0
    with open(out, "w", encoding="utf-8") as f:
        for iname, tstart, tcount in md.images():
            if iname not in wanted and "*" not in wanted:
                continue
            mcount = sum(types[t]["method_count"] for t in range(tstart, tstart + tcount))
            if mcount == 0:
                continue
            try:
                arr = find_module(pe, iname, mcount)
            except AssertionError as e:
                print("skip", e)
                continue
            aoff = pe.va_to_off(arr)
            linked = 0
            for ti in range(tstart, tstart + tcount):
                t = types[ti]
                for mi in range(t["method_start"], t["method_start"] + t["method_count"]):
                    name, decl, tok = methods[mi]
                    rid = tok & 0xFFFFFF
                    va = struct.unpack_from("<Q", pe.d, aoff + 8 * (rid - 1))[0] if 1 <= rid <= mcount else 0
                    ok = va and pe.is_exec(va)
                    linked += bool(ok)
                    f.write(json.dumps({"img": iname, "i": mi, "type": full(ti), "name": name, "token": tok,
                                        "rva": hex(va - pe.base) if ok else None,
                                        "va": hex(va) if ok else None}) + "\n")
                    n += 1
            print(f"{iname}: {mcount} methods, {linked} with code, pointer array {arr:#x}")
    print("wrote", n)


if __name__ == "__main__":
    main()
