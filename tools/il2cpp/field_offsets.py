"""Field names + instance offsets for IL2CPP types (static, read-only).

Locates Il2CppMetadataRegistration in GameAssembly.dll as the place where
fieldOffsetsCount and typeDefinitionsSizesCount (8-byte slots) both equal the
metadata type-definition count (same rule as REA), then reads
fieldOffsets[typeIndex] -> int32[field_count].

Usage:
    python -I field_offsets.py <global-metadata.dat> <GameAssembly.dll> <out.json> [TypeFullName ...]

Without type names, writes all types. Output: {type: [[name, offset], ...]}.
"""
import json
import struct
import sys

sys.path.insert(0, __file__.rsplit("\\", 1)[0].rsplit("/", 1)[0])
from il2cpp_meta import Metadata, PE  # noqa: E402


def find_registration(pe, n_types):
    d = pe.d
    hits = []
    for nm, sva, vsz, rptr, rsz, ch in pe.secs:
        if ch & 0x20000000:
            continue
        q = struct.unpack_from(f"<{rsz // 8}Q", d, rptr)
        for k in range(len(q) - 3):
            if q[k] == n_types and q[k + 2] == n_types:
                fo, ts = q[k + 1], q[k + 3]
                if pe.va_to_off(fo) is not None and pe.va_to_off(ts) is not None:
                    hits.append(fo)
    assert len(hits) == 1, f"{len(hits)} metadata registration candidates"
    return hits[0]


def main():
    mpath, bpath, out = sys.argv[1:4]
    want = set(sys.argv[4:])
    md = Metadata(mpath)
    types = md.types()
    pe = PE(bpath)
    fo_arr = find_registration(pe, len(types))
    foff = pe.va_to_off(fo_arr)
    fsec = md.sec["fields"][0]
    frec = 8 + md.wT

    def full(ti):
        t = types[ti]
        if t["parent"] >= 0:
            return full(t["parent"]) + "/" + t["name"]
        return (t["ns"] + "." if t["ns"] else "") + t["name"]

    result = {}
    for ti, t in enumerate(types):
        name = full(ti)
        if want and name not in want and name.split(".")[-1] not in want:
            continue
        ptr = struct.unpack_from("<Q", pe.d, foff + 8 * ti)[0]
        po = pe.va_to_off(ptr) if ptr else None
        fields = []
        for k in range(t["field_count"]):
            at = fsec + (t["field_start"] + k) * frec
            fname = md.string(struct.unpack_from("<i", md.b, at)[0])
            off = struct.unpack_from("<i", pe.d, po + 4 * k)[0] if po is not None else None
            fields.append([fname, off])
        result[name] = fields
    json.dump(result, open(out, "w"), indent=0)
    print(f"registration fieldOffsets @ {fo_arr:#x}; wrote {len(result)} types")


if __name__ == "__main__":
    main()
