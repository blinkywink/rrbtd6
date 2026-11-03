"""Resolve il2cpp string-literal slots (the `uRam...` globals a decompile passes to GetText and
friends) to their text: a slot holds its encoded metadata token until first use,
((token << 1) | 1), the token's usage type (5: string literal) in bits 28-31, its index below.

    python -I tools/il2cpp/literal.py 0x184fadd18 [...]
"""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from codegen import PE64, game_dir  # noqa: E402
from metadata_v39 import Metadata  # noqa: E402


def literal(md, index):
    """Metadata v39: 4-byte entries, each a literal's start in stringLiteralData (it ends where
    the next one starts)."""
    off, size, count = md.sec["stringLiteral"]
    start, = struct.unpack_from("<I", md.d, off + index * 4)
    end = struct.unpack_from("<I", md.d, off + (index + 1) * 4)[0] if index + 1 < count else md.sec["stringLiteralData"][1]
    base = md.sec["stringLiteralData"][0]
    return md.d[base + start:base + end].decode("utf-8", "replace")


def main():
    game = game_dir()
    dll = os.path.join(game, "GameAssembly.dll")
    pe = PE64(dll)
    raw = open(dll, "rb").read()
    md = Metadata(os.path.join(game, r"BloonsTD6_Data\il2cpp_data\Metadata\global-metadata.dat"))
    for a in sys.argv[1:]:
        va = int(a, 16)
        v, = struct.unpack_from("<Q", raw, pe.rva_to_off(va - 0x180000000))
        if not v & 1:
            print(f"{a}: not an encoded token ({v:#x})")
            continue
        token = (v >> 1) & 0xFFFFFFFF
        kind, index = token >> 28, token & 0x0FFFFFFF
        print(f"{a}: kind {kind} index {index}" + (f" {literal(md, index)!r}" if kind == 5 else ""))


if __name__ == "__main__":
    main()
