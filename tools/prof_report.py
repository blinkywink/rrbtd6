"""Summarise a samply profile recorded with --unstable-presymbolicate: self and inclusive
sample shares per function. Usage: python -I tools/prof_report.py PROFILE.json.gz [N]"""
import bisect, collections, gzip, json, sys

path = sys.argv[1]
top = int(sys.argv[2]) if len(sys.argv) > 2 else 30
prof = json.load(gzip.open(path))
syms = json.load(open(path.replace(".json.gz", ".json.syms.json")))
strs = syms["string_table"]
tables = {}
for lib in syms["data"]:
    rows = sorted(lib["symbol_table"], key=lambda r: r["rva"])
    tables[lib["debug_name"]] = ([r["rva"] for r in rows], rows)
libs = prof["libs"]
sys.path.insert(0, __import__("os").path.dirname(__file__))
namers = {}


def pdb_name(lib, addr):
    """Our own binaries: dbghelp reads the PDB that samply can't."""
    path = lib.get("path") or ""
    if not path.lower().endswith(".exe") or "rrbtd6" not in path.lower():
        return None
    if path not in namers:
        try:
            from pdb_names import Namer
            namers[path] = Namer(path)
        except Exception:
            namers[path] = None
    return namers[path].name(addr) if namers[path] else None


def name(lib_index, addr):
    lib = libs[lib_index] if lib_index is not None else None
    if lib:
        n = pdb_name(lib, addr)
        if n:
            return n
    t = tables.get(lib.get("debugName")) if lib else None
    if not t:
        return f"{lib.get('name') if lib else '?'}+{addr:#x}"
    rvas, rows = t
    k = bisect.bisect_right(rvas, addr) - 1
    if k >= 0 and addr < rows[k]["rva"] + max(rows[k]["size"], 1):
        return strs[rows[k]["symbol"]]
    return f"{lib.get('name')}+{addr:#x}"


for th in prof["threads"]:
    samples = th["samples"]["stack"]
    if len(samples) < 200:
        continue
    st, ft, fn, rt = th["stackTable"], th["frameTable"], th["funcTable"], th["resourceTable"]
    fname = []
    for f in range(ft["length"]):
        func = ft["func"][f]
        res = fn["resource"][func]
        lib = rt["lib"][res] if res is not None and res >= 0 else None
        fname.append(name(lib, ft["address"][f]))
    selfc, incl = collections.Counter(), collections.Counter()
    for s in samples:
        if s is None:
            continue
        selfc[fname[st["frame"][s]]] += 1
        seen = set()
        while s is not None:
            n = fname[st["frame"][s]]
            if n not in seen:
                incl[n] += 1
                seen.add(n)
            s = st["prefix"][s]
    n = len(samples)
    print(f"thread {th.get('name')}: {n} samples")
    print("-- self")
    for k, c in selfc.most_common(top):
        print(f"{100 * c / n:5.1f}%  {k[:140]}")
    print("-- inclusive")
    for k, c in incl.most_common(top):
        print(f"{100 * c / n:5.1f}%  {k[:140]}")
