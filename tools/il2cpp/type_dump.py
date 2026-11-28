"""Every type's base class and field types, straight from the game's own files.

    python -I type_dump.py <global-metadata.dat> <GameAssembly.dll> <out.jsonl> [image ...]

Writes one JSON object per type of the selected images (default Assembly-CSharp.dll), in the shape
name_records.py reads: {"name", "flags", "extends", "props": [{"name", "type"}]}. Names are plain
C# full names (nested types as Outer/Inner, like field_offsets.py), primitives by their C# keyword,
arrays as T[], generic instances as Name`N<Args>.

This replaces reading MelonLoader's generated interop assembly, so setup no longer needs
MelonLoader or a first start of BTD6. Field and base types are Il2CppType records in
Il2CppMetadataRegistration.types in GameAssembly.dll, found as field_offsets.py finds the
registration (types sits 4 slots before fieldOffsetsCount); type_usage.py decodes the same records.
"""
import json
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from il2cpp_meta import Metadata, PE, rd_index  # noqa: E402

# Il2CppTypeEnum.
PRIMS = {0x01: "void", 0x02: "bool", 0x03: "char", 0x04: "sbyte", 0x05: "byte", 0x06: "short",
         0x07: "ushort", 0x08: "int", 0x09: "uint", 0x0A: "long", 0x0B: "ulong", 0x0C: "float",
         0x0D: "double", 0x0E: "string", 0x18: "IntPtr", 0x19: "UIntPtr", 0x1C: "object"}
PTR, VALUETYPE, CLASS, VAR, ARRAY, GENERICINST, SZARRAY, MVAR = 0x0F, 0x11, 0x12, 0x13, 0x14, 0x15, 0x1D, 0x1E


def registration(pe, n_types):
    """Il2CppMetadataRegistration.types (a pointer array) and its length."""
    d, hits = pe.d, []
    for _, _, _, rptr, rsz, ch in pe.secs:
        if ch & 0x20000000:
            continue
        q = struct.unpack_from(f"<{rsz // 8}Q", d, rptr)
        for k in range(4, len(q) - 3):
            if q[k] == n_types and q[k + 2] == n_types and pe.va_to_off(q[k + 1]) is not None:
                hits.append((q[k - 4], q[k - 3]))
    if len(hits) != 1:
        raise SystemExit(f"Couldn't find the game's type table ({len(hits)} candidates). "
                         "This BTD6 build may use a newer Unity version than setup understands.")
    count, table = hits[0]
    return count, pe.va_to_off(table)


def main():
    meta_path, dll_path, out_path = sys.argv[1:4]
    wanted = set(sys.argv[4:]) or {"Assembly-CSharp.dll"}
    md = Metadata(meta_path)
    types = md.types()
    pe = PE(dll_path)
    d = pe.d
    n_types, table = registration(pe, len(types))

    # Typedef fields types() doesn't return: byval / declaring / parent type indices and flags.
    off, _, _ = md.sec["typeDefinitions"]
    for i, t in enumerate(types):
        at = off + i * md.tdsize
        t["base"] = rd_index(md.b, at + 8 + 2 * md.wT, md.wT)
        t["flags"] = struct.unpack_from("<I", md.b, at + 8 + 3 * md.wT + md.wGC)[0]
        # bitfield: bit 0 value type, bit 1 enum. An enum's parent slot holds its underlying type
        # (int), so enums are named by this bit instead.
        bitfield = struct.unpack_from("<I", md.b, at + md.tdsize - 8)[0]
        t["enum"] = bool(bitfield & 2)
        t["struct"] = bool(bitfield & 1)

    def full(ti):
        t = types[ti]
        if t["parent"] >= 0:
            return full(t["parent"]) + "/" + t["name"]
        return (t["ns"] + "." if t["ns"] else "") + t["name"]

    def type_ptr(index):
        if not 0 <= index < n_types:
            return None
        return struct.unpack_from("<Q", d, table + 8 * index)[0]

    def name(ptr, depth=0):
        if not ptr or depth > 12:
            return "?"
        at = pe.va_to_off(ptr)
        if at is None:
            return "?"
        data, bits = struct.unpack_from("<QI", d, at)
        kind = (bits >> 16) & 0xFF
        if kind in PRIMS:
            return PRIMS[kind]
        if kind in (CLASS, VALUETYPE):
            ti = data & 0xFFFFFFFF
            return full(ti) if ti < len(types) else "?"
        if kind == SZARRAY:
            return name(data, depth + 1) + "[]"
        if kind == ARRAY:
            elem = pe.va_to_off(data)
            return (name(struct.unpack_from("<Q", d, elem)[0], depth + 1) if elem is not None else "?") + "[,]"
        if kind == PTR:
            return name(data, depth + 1) + "*"
        if kind == GENERICINST:
            gc = pe.va_to_off(data)
            if gc is None:
                return "?"
            generic, class_inst = struct.unpack_from("<QQ", d, gc)
            args = []
            ci = pe.va_to_off(class_inst) if class_inst else None
            if ci is not None:
                argc, argv = struct.unpack_from("<QQ", d, ci)
                av = pe.va_to_off(argv)
                if av is not None and argc < 64:
                    args = [name(struct.unpack_from("<Q", d, av + 8 * j)[0], depth + 1) for j in range(argc)]
            return name(generic, depth + 1) + "<" + ", ".join(args) + ">"
        if kind in (VAR, MVAR):
            return "T"
        return "?"

    fsec = md.sec["fields"][0]
    frec = 8 + md.wT
    n = 0
    with open(out_path, "w", encoding="utf-8") as out:
        for image, start, count in md.images():
            if image not in wanted:
                continue
            for ti in range(start, start + count):
                t = types[ti]
                fields = []
                # Like the interop assembly this replaces, which has properties for class fields only:
                # struct fields stay unlisted, so name_records keeps their values as they are.
                for k in range(0 if t["struct"] else t["field_count"]):
                    at = fsec + (t["field_start"] + k) * frec
                    fname = md.string(struct.unpack_from("<i", md.b, at)[0])
                    fields.append({"name": fname, "type": name(type_ptr(rd_index(md.b, at + 4, md.wT)))})
                base = "System.Enum" if t["enum"] else name(type_ptr(t["base"])) if t["base"] >= 0 else None
                out.write(json.dumps({"name": full(ti), "flags": t["flags"], "extends": base, "props": fields}) + "\n")
                n += 1
    print(f"wrote {n} types")


if __name__ == "__main__":
    main()
