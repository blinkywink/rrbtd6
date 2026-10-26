"""Attach field names to decoded sim_model records and resolve references.

Inputs (all local, git-ignored):
  local_extracted/decoded/records.jsonl, values.json   (decode.py)
  research/dumps/field_offsets.json                     (il2cpp/field_offsets.py)
  research/dumps/Assembly-CSharp.jsonl                  (dotnet_meta: property types)

Writes local_extracted/decoded/named/objects.jsonl: {"i", "type", "f": {name: value}}
where object references are {"$ref": i} and arrays/lists/dicts are resolved
to their element refs.

Usage:
    python -I name_records.py <decoded_dir> <field_offsets.json> <Assembly-CSharp.jsonl>
"""
import json
import os
import sys

PRIMS = {"int", "float", "bool", "string", "double", "long", "short", "byte", "uint", "ushort"}


def main():
    ddir, fo_path, asm_path = sys.argv[1:4]
    offsets = json.load(open(fo_path))
    by_short = {}
    for full in offsets:
        by_short.setdefault(full.replace("/", ".").split(".")[-1], []).append(full)
        by_short.setdefault(full.replace(".", "_").replace("/", "_"), []).append(full)

    # Property types from the interop dump; enums -> int.
    enums, ptypes = set(), {}
    for line in open(asm_path, encoding="utf-8"):
        t = json.loads(line)
        n = t["name"].replace("Il2Cpp", "", 1) if t["name"].startswith("Il2Cpp") else t["name"]
        if t["extends"] == "System.Enum":
            enums.add(t["name"])
        ptypes[n] = {p["name"]: p["type"] for p in t["props"]}

    def resolve_type(short):
        c = by_short.get(short, [])
        models = [x for x in c if ".Models." in x or ".Data." in x] or c
        return models[0] if models else None

    def is_ref(full, fname):
        t = ptypes.get(full, {}).get(fname)
        if t is None:
            return None
        base = t.split("<")[0]
        if base in enums or base.split(".")[-1] in PRIMS:
            return False
        if base.startswith(("Il2CppSystem.Nullable",)):
            return False
        return True

    values = json.load(open(os.path.join(ddir, "values.json")))
    out_dir = os.path.join(ddir, "named")
    os.makedirs(out_dir, exist_ok=True)
    unresolved = set()
    with open(os.path.join(ddir, "records.jsonl"), encoding="utf-8") as f, \
            open(os.path.join(out_dir, "objects.jsonl"), "w", encoding="utf-8") as out:
        for line in f:
            rec = json.loads(line)
            named = {}
            for seg in rec["fields"]:
                full = resolve_type(seg["from"])
                if full is None:
                    unresolved.add(seg["from"])
                    continue
                flist = [(n, o) for n, o in offsets[full] if o is not None and o >= 16]
                by_off = {o: n for n, o in flist}
                used = set()
                vals = seg["values"]
                for k, (store, v) in enumerate(vals):
                    if isinstance(store, int) and store in by_off:
                        name = by_off[store]
                    else:
                        # Reflection store: first unused field between neighbouring known offsets.
                        lo = max([s for s, _ in vals[:k] if isinstance(s, int)] or [0])
                        hi = min([s for s, _ in vals[k + 1:] if isinstance(s, int)] or [1 << 30])
                        cands = [n for n, o in flist if lo < o < hi and n not in used]
                        # *Frames fields are runtime-derived (never serialized); prefer the source field.
                        cands = [n for n in cands if not n.endswith("Frames")] or cands
                        name = cands[0] if cands else f"?{store}@{k}"
                    used.add(name)
                    ref = is_ref(full, name.lstrip("_"))
                    if ref is None:
                        ref = is_ref(full, name)
                    if ref and isinstance(v, int) and not isinstance(v, bool):
                        v = None if v == 0 else {"$ref": v}
                    named[name] = v
            out.write(json.dumps({"i": rec["i"], "type": rec["type"], "f": named}) + "\n")
    # Containers: arrays/lists/dicts with resolved items.
    with open(os.path.join(out_dir, "containers.json"), "w", encoding="utf-8") as cf:
        json.dump({k: v for k, v in values.items()}, cf)
    print("unresolved types:", sorted(unresolved)[:20], len(unresolved))


if __name__ == "__main__":
    main()
