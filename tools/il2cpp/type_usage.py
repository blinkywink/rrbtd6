"""Name the class behind an il2cpp TypeInfo slot (the `lRam...` globals a decompile compares an
object's klass against, e.g. inside a GetEnumerator / OfType loop).

    python -I tools/il2cpp/type_usage.py 0x184f87f88 [...]

literal.py shows such a slot as "kind 1 index N": N indexes Il2CppMetadataRegistration.types in
GameAssembly.dll (found as field_offsets.py finds the registration: types sits 4 slots before
fieldOffsetsCount). A class or value type names its type definition; a generic instance is
followed through its Il2CppGenericClass to the generic type and its arguments.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from codegen import game_dir  # noqa: E402
from il2cpp_meta import Metadata, PE  # noqa: E402

CLASS, VALUETYPE, GENERICINST = 0x12, 0x11, 0x15


def main():
    game = game_dir()
    md = Metadata(os.path.join(game, r"BloonsTD6_Data\il2cpp_data\Metadata\global-metadata.dat"))
    types = md.types()
    pe = PE(os.path.join(game, "GameAssembly.dll"))
    d, n = pe.d, len(types)
    reg = []
    for _, _, _, rptr, rsz, ch in pe.secs:
        if ch & 0x20000000:
            continue
        q = struct.unpack_from(f"<{rsz // 8}Q", d, rptr)
        for k in range(4, len(q) - 3):
            if q[k] == n and q[k + 2] == n and pe.va_to_off(q[k + 1]) is not None:
                reg.append(q[k - 3])
    assert len(reg) == 1, f"{len(reg)} metadata registration candidates"
    table = pe.va_to_off(reg[0])

    def full(ti):
        t = types[ti]
        if t["parent"] >= 0:
            return full(t["parent"]) + "/" + t["name"]
        return (t["ns"] + "." if t["ns"] else "") + t["name"]

    def name(ptr):
        data, bits = struct.unpack_from("<QI", d, pe.va_to_off(ptr))
        kind = (bits >> 16) & 0xFF
        if kind in (CLASS, VALUETYPE):
            return full(data & 0xFFFFFFFF)
        if kind == GENERICINST:
            generic, class_inst, _ = struct.unpack_from("<QQQ", d, pe.va_to_off(data))
            argc, argv = struct.unpack_from("<QQ", d, pe.va_to_off(class_inst))
            args = [name(struct.unpack_from("<Q", d, pe.va_to_off(argv) + 8 * j)[0]) for j in range(argc)]
            return name(generic) + "<" + ", ".join(args) + ">"
        return f"(type kind {kind:#x})"

    for a in sys.argv[1:]:
        va = int(a, 16)
        v, = struct.unpack_from("<Q", d, pe.va_to_off(va))
        if not v & 1:
            print(f"{a}: not an encoded token ({v:#x})")
            continue
        token = (v >> 1) & 0xFFFFFFFF
        kind, index = token >> 28, token & 0x0FFFFFFF
        if kind != 1:
            print(f"{a}: usage kind {kind}, not a TypeInfo")
            continue
        ptr, = struct.unpack_from("<Q", d, table + 8 * index)
        print(f"{a}: {name(ptr)}")


if __name__ == "__main__":
    main()
