"""Minimal, dependency-free ECMA-335 metadata reader.

Reads a .NET PE file (e.g. MelonLoader's Il2CppInterop-generated
Il2CppAssemblies/Assembly-CSharp.dll) and dumps types, fields, properties and
methods with decoded signatures. Read-only; never executes the target.

Usage:
    python -I dotnet_meta.py <assembly.dll> <out.txt> [--json out.jsonl]
"""
import json
import struct
import sys

# ---------------------------------------------------------------- PE layer


class PE:
    def __init__(self, data: bytes):
        self.d = data
        pe_off = struct.unpack_from("<I", data, 0x3C)[0]
        assert data[pe_off:pe_off + 4] == b"PE\0\0", "not a PE file"
        nsec = struct.unpack_from("<H", data, pe_off + 6)[0]
        opt_size = struct.unpack_from("<H", data, pe_off + 20)[0]
        opt = pe_off + 24
        magic = struct.unpack_from("<H", data, opt)[0]
        dd = opt + (96 if magic == 0x10B else 112)
        self.cli_rva, _ = struct.unpack_from("<II", data, dd + 14 * 8)
        self.sections = []
        s = opt + opt_size
        for i in range(nsec):
            vsize, va, rsize, rptr = struct.unpack_from("<IIII", data, s + i * 40 + 8)
            self.sections.append((va, max(vsize, rsize), rptr))

    def off(self, rva):
        for va, size, rptr in self.sections:
            if va <= rva < va + size:
                return rva - va + rptr
        raise ValueError(hex(rva))


# ---------------------------------------------------------------- tables

T = dict(Module=0x00, TypeRef=0x01, TypeDef=0x02, FieldPtr=0x03, Field=0x04,
         MethodPtr=0x05, MethodDef=0x06, ParamPtr=0x07, Param=0x08,
         InterfaceImpl=0x09, MemberRef=0x0A, Constant=0x0B,
         CustomAttribute=0x0C, FieldMarshal=0x0D, DeclSecurity=0x0E,
         ClassLayout=0x0F, FieldLayout=0x10, StandAloneSig=0x11,
         EventMap=0x12, EventPtr=0x13, Event=0x14, PropertyMap=0x15,
         PropertyPtr=0x16, Property=0x17, MethodSemantics=0x18,
         MethodImpl=0x19, ModuleRef=0x1A, TypeSpec=0x1B, ImplMap=0x1C,
         FieldRVA=0x1D, EncLog=0x1E, EncMap=0x1F, Assembly=0x20,
         AssemblyProcessor=0x21, AssemblyOS=0x22, AssemblyRef=0x23,
         AssemblyRefProcessor=0x24, AssemblyRefOS=0x25, File=0x26,
         ExportedType=0x27, ManifestResource=0x28, NestedClass=0x29,
         GenericParam=0x2A, MethodSpec=0x2B, GenericParamConstraint=0x2C)

CODED = {
    "TypeDefOrRef": (2, ["TypeDef", "TypeRef", "TypeSpec"]),
    "HasConstant": (2, ["Field", "Param", "Property"]),
    "HasCustomAttribute": (5, ["MethodDef", "Field", "TypeRef", "TypeDef", "Param",
                               "InterfaceImpl", "MemberRef", "Module", "DeclSecurity",
                               "Property", "Event", "StandAloneSig", "ModuleRef",
                               "TypeSpec", "Assembly", "AssemblyRef", "File",
                               "ExportedType", "ManifestResource", "GenericParam",
                               "GenericParamConstraint", "MethodSpec"]),
    "HasFieldMarshal": (1, ["Field", "Param"]),
    "HasDeclSecurity": (2, ["TypeDef", "MethodDef", "Assembly"]),
    "MemberRefParent": (3, ["TypeDef", "TypeRef", "ModuleRef", "MethodDef", "TypeSpec"]),
    "HasSemantics": (1, ["Event", "Property"]),
    "MethodDefOrRef": (1, ["MethodDef", "MemberRef"]),
    "MemberForwarded": (1, ["Field", "MethodDef"]),
    "Implementation": (2, ["File", "AssemblyRef", "ExportedType"]),
    "CustomAttributeType": (3, [None, None, "MethodDef", "MemberRef", None]),
    "ResolutionScope": (2, ["Module", "ModuleRef", "AssemblyRef", "TypeRef"]),
    "TypeOrMethodDef": (1, ["TypeDef", "MethodDef"]),
}

# column kinds: u1/u2/u4, s (string heap), g (guid), b (blob), name of table
# (simple index) or coded index name.
SCHEMA = {
    "Module": [("Generation", "u2"), ("Name", "s"), ("Mvid", "g"), ("EncId", "g"), ("EncBaseId", "g")],
    "TypeRef": [("Scope", "ResolutionScope"), ("Name", "s"), ("Namespace", "s")],
    "TypeDef": [("Flags", "u4"), ("Name", "s"), ("Namespace", "s"), ("Extends", "TypeDefOrRef"),
                ("FieldList", "Field"), ("MethodList", "MethodDef")],
    "FieldPtr": [("Field", "Field")],
    "Field": [("Flags", "u2"), ("Name", "s"), ("Signature", "b")],
    "MethodPtr": [("Method", "MethodDef")],
    "MethodDef": [("RVA", "u4"), ("ImplFlags", "u2"), ("Flags", "u2"), ("Name", "s"),
                  ("Signature", "b"), ("ParamList", "Param")],
    "ParamPtr": [("Param", "Param")],
    "Param": [("Flags", "u2"), ("Sequence", "u2"), ("Name", "s")],
    "InterfaceImpl": [("Class", "TypeDef"), ("Interface", "TypeDefOrRef")],
    "MemberRef": [("Class", "MemberRefParent"), ("Name", "s"), ("Signature", "b")],
    "Constant": [("Type", "u2"), ("Parent", "HasConstant"), ("Value", "b")],
    "CustomAttribute": [("Parent", "HasCustomAttribute"), ("Type", "CustomAttributeType"), ("Value", "b")],
    "FieldMarshal": [("Parent", "HasFieldMarshal"), ("NativeType", "b")],
    "DeclSecurity": [("Action", "u2"), ("Parent", "HasDeclSecurity"), ("PermissionSet", "b")],
    "ClassLayout": [("PackingSize", "u2"), ("ClassSize", "u4"), ("Parent", "TypeDef")],
    "FieldLayout": [("Offset", "u4"), ("Field", "Field")],
    "StandAloneSig": [("Signature", "b")],
    "EventMap": [("Parent", "TypeDef"), ("EventList", "Event")],
    "EventPtr": [("Event", "Event")],
    "Event": [("Flags", "u2"), ("Name", "s"), ("EventType", "TypeDefOrRef")],
    "PropertyMap": [("Parent", "TypeDef"), ("PropertyList", "Property")],
    "PropertyPtr": [("Property", "Property")],
    "Property": [("Flags", "u2"), ("Name", "s"), ("Type", "b")],
    "MethodSemantics": [("Semantics", "u2"), ("Method", "MethodDef"), ("Association", "HasSemantics")],
    "MethodImpl": [("Class", "TypeDef"), ("Body", "MethodDefOrRef"), ("Decl", "MethodDefOrRef")],
    "ModuleRef": [("Name", "s")],
    "TypeSpec": [("Signature", "b")],
    "ImplMap": [("Flags", "u2"), ("Member", "MemberForwarded"), ("ImportName", "s"), ("ImportScope", "ModuleRef")],
    "FieldRVA": [("RVA", "u4"), ("Field", "Field")],
    "EncLog": [("Token", "u4"), ("FuncCode", "u4")],
    "EncMap": [("Token", "u4")],
    "Assembly": [("HashAlgId", "u4"), ("Major", "u2"), ("Minor", "u2"), ("Build", "u2"), ("Rev", "u2"),
                 ("Flags", "u4"), ("PublicKey", "b"), ("Name", "s"), ("Culture", "s")],
    "AssemblyProcessor": [("Processor", "u4")],
    "AssemblyOS": [("PlatformId", "u4"), ("Major", "u4"), ("Minor", "u4")],
    "AssemblyRef": [("Major", "u2"), ("Minor", "u2"), ("Build", "u2"), ("Rev", "u2"), ("Flags", "u4"),
                    ("PublicKeyOrToken", "b"), ("Name", "s"), ("Culture", "s"), ("HashValue", "b")],
    "AssemblyRefProcessor": [("Processor", "u4"), ("AssemblyRef", "AssemblyRef")],
    "AssemblyRefOS": [("PlatformId", "u4"), ("Major", "u4"), ("Minor", "u4"), ("AssemblyRef", "AssemblyRef")],
    "File": [("Flags", "u4"), ("Name", "s"), ("HashValue", "b")],
    "ExportedType": [("Flags", "u4"), ("TypeDefId", "u4"), ("Name", "s"), ("Namespace", "s"),
                     ("Implementation", "Implementation")],
    "ManifestResource": [("Offset", "u4"), ("Flags", "u4"), ("Name", "s"), ("Implementation", "Implementation")],
    "NestedClass": [("Nested", "TypeDef"), ("Enclosing", "TypeDef")],
    "GenericParam": [("Number", "u2"), ("Flags", "u2"), ("Owner", "TypeOrMethodDef"), ("Name", "s")],
    "MethodSpec": [("Method", "MethodDefOrRef"), ("Instantiation", "b")],
    "GenericParamConstraint": [("Owner", "GenericParam"), ("Constraint", "TypeDefOrRef")],
}
TNAME = {v: k for k, v in T.items()}


class Metadata:
    def __init__(self, path):
        data = open(path, "rb").read()
        pe = PE(data)
        cli = pe.off(pe.cli_rva)
        md_rva, md_size = struct.unpack_from("<II", data, cli + 8)
        root = pe.off(md_rva)
        assert data[root:root + 4] == b"BSJB", "no metadata root"
        vlen = struct.unpack_from("<I", data, root + 12)[0]
        p = root + 16 + vlen
        nstreams = struct.unpack_from("<H", data, p + 2)[0]
        p += 4
        streams = {}
        for _ in range(nstreams):
            o, sz = struct.unpack_from("<II", data, p)
            p += 8
            e = data.index(b"\0", p)
            name = data[p:e].decode()
            p = (e + 4) & ~3
            streams[name] = (root + o, sz)
        self.d = data
        self.strings = streams["#Strings"][0]
        self.blob = streams["#Blob"][0]
        tbl, _ = streams.get("#~") or streams["#-"]
        heap_sizes = data[tbl + 6]
        self.str_w = 4 if heap_sizes & 1 else 2
        self.guid_w = 4 if heap_sizes & 2 else 2
        self.blob_w = 4 if heap_sizes & 4 else 2
        valid = struct.unpack_from("<Q", data, tbl + 8)[0]
        p = tbl + 24
        self.rows = {}
        for i in range(64):
            if valid >> i & 1:
                self.rows[TNAME.get(i, i)] = struct.unpack_from("<I", data, p)[0]
                p += 4
        if heap_sizes & 0x40:
            p += 4
        self.tables = {}
        for i in range(0x2D):
            name = TNAME[i]
            n = self.rows.get(name, 0)
            cols = [(c, self._width(k)) for c, k in SCHEMA[name]]
            rw = sum(w for _, w in cols)
            self.tables[name] = (p, n, rw, cols)
            p += n * rw
        self._cache = {}

    def _width(self, kind):
        if kind in ("u1",):
            return 1
        if kind == "u2":
            return 2
        if kind == "u4":
            return 4
        if kind == "s":
            return self.str_w
        if kind == "g":
            return self.guid_w
        if kind == "b":
            return self.blob_w
        if kind in CODED:
            bits, tabs = CODED[kind]
            mx = max(self.rows.get(t, 0) for t in tabs if t)
            return 2 if mx < (1 << (16 - bits)) else 4
        return 2 if self.rows.get(kind, 0) < 65536 else 4

    def row(self, table, idx):
        """1-based row index -> dict of raw column values."""
        start, n, rw, cols = self.tables[table]
        assert 1 <= idx <= n, (table, idx, n)
        p = start + (idx - 1) * rw
        out = {}
        for c, w in cols:
            out[c] = struct.unpack_from("<H" if w == 2 else ("<I" if w == 4 else "<B"), self.d, p)[0]
            p += w
        return out

    def count(self, table):
        return self.tables[table][1]

    def str(self, off):
        p = self.strings + off
        return self.d[p:self.d.index(b"\0", p)].decode("utf-8", "replace")

    def blob_bytes(self, off):
        p = self.blob + off
        n, p = decompress(self.d, p)
        return self.d[p:p + n]

    @staticmethod
    def decode_coded(kind, v):
        bits, tabs = CODED[kind]
        return tabs[v & ((1 << bits) - 1)], v >> bits


def decompress(d, p):
    b = d[p]
    if b & 0x80 == 0:
        return b, p + 1
    if b & 0xC0 == 0x80:
        return ((b & 0x3F) << 8) | d[p + 1], p + 2
    return ((b & 0x1F) << 24) | (d[p + 1] << 16) | (d[p + 2] << 8) | d[p + 3], p + 4


# ---------------------------------------------------------------- signatures

PRIM = {0x01: "void", 0x02: "bool", 0x03: "char", 0x04: "sbyte", 0x05: "byte", 0x06: "short",
        0x07: "ushort", 0x08: "int", 0x09: "uint", 0x0A: "long", 0x0B: "ulong", 0x0C: "float",
        0x0D: "double", 0x0E: "string", 0x16: "TypedReference", 0x18: "IntPtr", 0x19: "UIntPtr",
        0x1C: "object"}


class Dumper:
    def __init__(self, md: Metadata):
        self.md = md
        self.nested_parent = {}
        for i in range(1, md.count("NestedClass") + 1):
            r = md.row("NestedClass", i)
            self.nested_parent[r["Nested"]] = r["Enclosing"]

    def typedef_name(self, idx):
        r = self.md.row("TypeDef", idx)
        name = self.md.str(r["Name"])
        if idx in self.nested_parent:
            return self.typedef_name(self.nested_parent[idx]) + "/" + name
        ns = self.md.str(r["Namespace"])
        return f"{ns}.{name}" if ns else name

    def typeref_name(self, idx):
        r = self.md.row("TypeRef", idx)
        name = self.md.str(r["Name"])
        tab, sidx = Metadata.decode_coded("ResolutionScope", r["Scope"])
        if tab == "TypeRef" and sidx:
            return self.typeref_name(sidx) + "/" + name
        ns = self.md.str(r["Namespace"])
        return f"{ns}.{name}" if ns else name

    def tdor_name(self, coded):
        tab, idx = Metadata.decode_coded("TypeDefOrRef", coded)
        if idx == 0:
            return None
        if tab == "TypeDef":
            return self.typedef_name(idx)
        if tab == "TypeRef":
            return self.typeref_name(idx)
        return self.sig_type(self.md.blob_bytes(self.md.row("TypeSpec", idx)["Signature"]), 0)[0]

    def sig_type(self, b, p):
        et = b[p]
        p += 1
        if et in PRIM:
            return PRIM[et], p
        if et in (0x11, 0x12):
            v, p = decompress(b, p)
            return self.tdor_name(v), p
        if et in (0x0F, 0x10):
            t, p = self.sig_type(b, p)
            return t + ("*" if et == 0x0F else "&"), p
        if et == 0x1D:
            t, p = self.sig_type(b, p)
            return t + "[]", p
        if et in (0x13, 0x1E):
            n, p = decompress(b, p)
            return ("!" if et == 0x13 else "!!") + str(n), p
        if et == 0x15:
            t, p = self.sig_type(b, p)
            n, p = decompress(b, p)
            args = []
            for _ in range(n):
                a, p = self.sig_type(b, p)
                args.append(a)
            return f"{t}<{', '.join(args)}>", p
        if et == 0x14:
            t, p = self.sig_type(b, p)
            rank, p = decompress(b, p)
            nsz, p = decompress(b, p)
            for _ in range(nsz):
                _, p = decompress(b, p)
            nlb, p = decompress(b, p)
            for _ in range(nlb):
                _, p = decompress(b, p)
            return t + "[" + "," * (rank - 1) + "]", p
        if et in (0x1F, 0x20):
            _, p = decompress(b, p)
            return self.sig_type(b, p)
        if et == 0x45:
            return self.sig_type(b, p)
        if et == 0x1B:
            return "fnptr", len(b)
        return f"?{et:#x}", len(b)

    def method_sig(self, b):
        p = 0
        cc = b[p]
        p += 1
        if cc & 0x10:
            _, p = decompress(b, p)
        n, p = decompress(b, p)
        ret, p = self.sig_type(b, p)
        params = []
        for _ in range(n):
            if p >= len(b):
                break
            t, p = self.sig_type(b, p)
            params.append(t)
        return ret, params, bool(cc & 0x20)

    def field_type(self, b):
        return self.sig_type(b, 1)[0]

    def prop_type(self, b):
        p = 1
        _, p = decompress(b, p)
        return self.sig_type(b, p)[0]

    def dump(self):
        md = self.md
        ntypes = md.count("TypeDef")
        nfields, nmethods, nparams = md.count("Field"), md.count("MethodDef"), md.count("Param")
        prop_ranges = {}
        npm = md.count("PropertyMap")
        for i in range(1, npm + 1):
            r = md.row("PropertyMap", i)
            end = md.row("PropertyMap", i + 1)["PropertyList"] if i < npm else md.count("Property") + 1
            prop_ranges[r["Parent"]] = (r["PropertyList"], end)
        for ti in range(1, ntypes + 1):
            r = md.row("TypeDef", ti)
            nxt = md.row("TypeDef", ti + 1) if ti < ntypes else None
            f_end = nxt["FieldList"] if nxt else nfields + 1
            m_end = nxt["MethodList"] if nxt else nmethods + 1
            t = {"name": self.typedef_name(ti), "flags": r["Flags"],
                 "extends": self.tdor_name(r["Extends"]), "fields": [], "props": [], "methods": []}
            for fi in range(r["FieldList"], min(f_end, nfields + 1)):
                f = md.row("Field", fi)
                t["fields"].append({"name": md.str(f["Name"]), "flags": f["Flags"],
                                    "type": self.field_type(md.blob_bytes(f["Signature"]))})
            if ti in prop_ranges:
                a, b = prop_ranges[ti]
                for pi in range(a, b):
                    pr = md.row("Property", pi)
                    t["props"].append({"name": md.str(pr["Name"]),
                                       "type": self.prop_type(md.blob_bytes(pr["Type"]))})
            for mi in range(r["MethodList"], min(m_end, nmethods + 1)):
                m = md.row("MethodDef", mi)
                m_nxt = md.row("MethodDef", mi + 1)["ParamList"] if mi < nmethods else nparams + 1
                pnames = {}
                for pi in range(m["ParamList"], min(m_nxt, nparams + 1)):
                    pr = md.row("Param", pi)
                    pnames[pr["Sequence"]] = md.str(pr["Name"])
                ret, params, has_this = self.method_sig(md.blob_bytes(m["Signature"]))
                t["methods"].append({"name": md.str(m["Name"]), "flags": m["Flags"], "ret": ret,
                                     "static": not has_this,
                                     "params": [[pt, pnames.get(i + 1, f"a{i}")] for i, pt in enumerate(params)]})
            yield t


def main():
    src, out = sys.argv[1], sys.argv[2]
    js = sys.argv[sys.argv.index("--json") + 1] if "--json" in sys.argv else None
    md = Metadata(src)
    dm = Dumper(md)
    jf = open(js, "w", encoding="utf-8") if js else None
    n = 0
    with open(out, "w", encoding="utf-8") as f:
        for t in dm.dump():
            n += 1
            f.write(f"type {t['name']} : {t['extends']}\n")
            for x in t["fields"]:
                f.write(f"  field {'static ' if x['flags'] & 0x10 else ''}{x['type']} {x['name']}\n")
            for x in t["props"]:
                f.write(f"  prop {x['type']} {x['name']}\n")
            for x in t["methods"]:
                ps = ", ".join(f"{a} {b}" for a, b in x["params"])
                f.write(f"  method {'static ' if x['static'] else ''}{x['ret']} {x['name']}({ps})\n")
            if jf:
                jf.write(json.dumps(t) + "\n")
    print(f"{n} types", {k: v for k, v in md.rows.items() if k in ("TypeDef", "Field", "MethodDef", "Property")})


if __name__ == "__main__":
    main()
