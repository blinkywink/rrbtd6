"""List every method of a type with native VA, parameter count and token,
so overloads can be told apart (addr_map.json keeps one name per address).

    python -I methods_of.py <TypeName> [<image.dll>]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from codegen import PE64, find_module, method_pointers  # noqa: E402
from metadata_v39 import Metadata  # noqa: E402

GAME = r"C:\Program Files\Epic Games\BloonsTD6"


def methods_of(type_name, image=None):
    pe = PE64(os.path.join(GAME, "GameAssembly.dll"))
    md = Metadata(os.path.join(GAME, r"BloonsTD6_Data\il2cpp_data\Metadata\global-metadata.dat"))
    rows = [r for r in md.iter_methods() if (r[1] == type_name or r[1].endswith("." + type_name))
            and (image is None or r[0] == image)]
    per_image = {}
    out = []
    for iname, tn, mn, tok, flags, slot, npar in rows:
        if iname not in per_image:
            count = sum(1 for r in md.iter_methods() if r[0] == iname)
            per_image[iname] = method_pointers(pe, find_module(pe, iname, count)[0])
        va = per_image[iname][(tok & 0xFFFFFF) - 1]
        out.append((tn, mn, npar, va, tok, slot))
    return out


if __name__ == "__main__":
    for tn, mn, npar, va, tok, slot in methods_of(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else None):
        print(f"{tn}::{mn}/{npar}  va={va:#x}  token={tok:#x} slot={slot}")
