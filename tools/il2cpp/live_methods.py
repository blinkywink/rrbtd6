"""Every IL2CPP method of the installed game with its native VA, read live from
GameAssembly.dll + global-metadata.dat (methods-all.jsonl can be from an older build).

    python -I tools/il2cpp/live_methods.py [out.tsv]

Writes va<TAB>image<TAB>type<TAB>method<TAB>params<TAB>slot (default
tools/external/ghidra_tmp/methods_live.tsv). Methods without a body (abstract, generic
definitions) are left out.
"""
import collections
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from codegen import PE64, find_module, method_pointers  # noqa: E402
from metadata_v39 import Metadata  # noqa: E402

GAME = r"C:\Program Files\Epic Games\BloonsTD6"
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "tools", "external", "ghidra_tmp", "methods_live.tsv")
    pe = PE64(os.path.join(GAME, "GameAssembly.dll"))
    md = Metadata(os.path.join(GAME, r"BloonsTD6_Data\il2cpp_data\Metadata\global-metadata.dat"))
    rows = list(md.iter_methods())
    count = collections.Counter(r[0] for r in rows)
    ptrs = {}
    os.makedirs(os.path.dirname(out), exist_ok=True)
    n = 0
    with open(out, "w", encoding="utf-8") as f:
        for iname, tn, mn, tok, flags, slot, npar in rows:
            if iname not in ptrs:
                try:
                    ptrs[iname] = method_pointers(pe, find_module(pe, iname, count[iname])[0])
                except Exception:
                    ptrs[iname] = None
            p = ptrs[iname]
            i = (tok & 0xFFFFFF) - 1
            if not p or i >= len(p) or not p[i]:
                continue
            f.write(f"{p[i]:x}\t{iname}\t{tn}\t{mn}\t{npar}\t{slot}\n")
            n += 1
    print(n, "methods ->", out)


if __name__ == "__main__":
    main()
