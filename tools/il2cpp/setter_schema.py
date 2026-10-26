"""Extract the BinaryReader read sequence of every GameModelLoader.Set_v_*_Fields.

Static analysis only. For each setter, walk the x64 code and record, in address
order:
  base      call to another Set_v_* (base-class fields, read first)
  i32/f32/bool/str/...   a BinaryReader virtual read (by vtable slot)
  {"if_false"/"if_true": [...]}   reads guarded by a preceding bool read
plus the object field offset each value is stored to, when visible.

Usage:
    tools/.venv/Scripts/python.exe tools/il2cpp/setter_schema.py <GameAssembly.dll> <methods-all.jsonl> <out.json>
"""
import bisect
import json
import sys

import capstone

sys.path.insert(0, __file__.rsplit("\\", 1)[0].rsplit("/", 1)[0])
from il2cpp_meta import PE  # noqa: E402

# BinaryReader vtable slot -> read kind. Offsets are into the klass (vtable
# entries are {methodPtr, methodInfo} = 16 bytes); thunk index = (off-0x138)/16.
SLOTS = {0x1c8: "bool", 0x228: "i32", 0x268: "f32", 0x298: "str",
         0x1d8: "u8", 0x1e8: "i8", 0x208: "i16", 0x218: "u16", 0x238: "u32",
         0x248: "i64", 0x258: "u64", 0x278: "f64", 0x288: "decimal"}
THUNK = 0x180003F20


def slot_kind(off):
    return SLOTS.get(off, f"slot{off:#x}")


def load_funcs(path):
    names = {}
    for line in open(path, encoding="utf-8"):
        r = json.loads(line)
        if r["va"]:
            names.setdefault(int(r["va"], 16), f'{r["type"]}::{r["name"]}')
    return names


def extract(pe, md, va, end, names):
    off = pe.va_to_off(va)
    insns = list(md.disasm(pe.d[off:off + (end - va)], va))
    ops = []          # flat list of (addr, op dict)
    warnings = []
    pending_slot = None
    pending_reg = None
    pending_thunk = None
    for k, ins in enumerate(insns):
        m, o = ins.mnemonic, ins.op_str
        if m == "mov" and ", qword ptr [rdx + 0x" in o and o.endswith("]") and not o.startswith("rdx,"):
            pending_reg = o.split(",")[0]
            pending_slot = int(o.split("+ ")[1].rstrip("]"), 16)
            continue
        if m == "mov" and o.startswith("ecx, 0x") and k + 1 < len(insns) and insns[k + 1].mnemonic == "call" \
                and insns[k + 1].op_str == hex(THUNK):
            pending_thunk = int(o.split(", ")[1], 16)
            continue
        if m == "call":
            if pending_slot is not None and o == pending_reg:
                ops.append((ins.address, {"op": slot_kind(pending_slot)}))
                pending_slot = None
            elif o == hex(THUNK) and pending_thunk is not None:
                ops.append((ins.address, {"op": slot_kind(0x138 + 16 * pending_thunk)}))
                pending_thunk = None
            elif o.startswith("0x"):
                tgt = int(o, 16)
                n = names.get(tgt, "")
                if "::Set_v_" in n:
                    ops.append((ins.address, {"op": "base", "fn": n.split("::")[1]}))
                elif "FieldInfo::SetValue" in n and ops:
                    ops[-1][1].setdefault("store", "reflect")
            continue
        if m == "mov" and o.startswith(("byte ptr [rbx + 0x", "dword ptr [rbx + 0x", "qword ptr [rbx + 0x")) \
                and ops and "store" not in ops[-1][1] and ops[-1][1]["op"] != "base":
            ops[-1][1]["store"] = int(o.split("+ ")[1].split("]")[0], 16)
        if m in ("movss", "movsd") and o.startswith(("dword ptr [rbx + 0x", "qword ptr [rbx + 0x")) \
                and ops and "store" not in ops[-1][1]:
            ops[-1][1]["store"] = int(o.split("+ ")[1].split("]")[0], 16)

    # Bool guards: bool read; test al, al; je/jne L1 ... jmp L2; [L1, L2) conditional.
    addr_index = {i.address: n for n, i in enumerate(insns)}
    guards = []
    for n, (a, op) in enumerate(ops):
        if op["op"] != "bool":
            continue
        k = addr_index[a]
        window = insns[k + 1:k + 4]
        if len(window) >= 2 and window[0].mnemonic == "test" and window[0].op_str == "al, al" \
                and window[1].mnemonic in ("je", "jne"):
            l1 = int(window[1].op_str, 16)
            l2 = None
            for j in range(k + 2, len(insns)):
                if insns[j].address >= l1:
                    break
                if insns[j].mnemonic == "jmp":
                    l2 = int(insns[j].op_str, 16)
                    break
            if l2 is not None and l2 > l1:
                cond = "if_false" if window[1].mnemonic == "je" else "if_true"
                guards.append((n, l1, l2, cond))
            else:
                warnings.append(f"unresolved guard at {a:#x}")

    # Loops: each back-edge (jcc to a lower address) closes a loop [target, edge].
    # Setters run one or more *sequential* loops over the whole record range;
    # each loop reads its own subset of fields for every record in turn.
    loops = sorted((int(i.op_str, 16), i.address) for i in insns
                   if i.mnemonic.startswith("j") and i.op_str.startswith("0x")
                   and int(i.op_str, 16) < i.address)
    for (a0, a1), (b0, b1) in zip(loops, loops[1:]):
        if b0 <= a1:
            warnings.append(f"overlapping/nested loops {a0:#x}-{a1:#x} / {b0:#x}-{b1:#x}")

    # Fold guarded reads into their guard.
    folded, consumed = [], set()
    for n, (a, op) in enumerate(ops):
        if n in consumed:
            continue
        g = next((g for g in guards if g[0] == n), None)
        if g:
            _, l1, l2, cond = g
            inner = []
            for m2 in range(n + 1, len(ops)):
                if l1 <= ops[m2][0] < l2:
                    inner.append(ops[m2][1])
                    consumed.add(m2)
            op = dict(op, **{cond: inner})
        folded.append((a, op))

    # Group into phases: "base" calls stand alone; reads go to their loop.
    phases = []
    for a, op in folded:
        if op["op"] == "base":
            phases.append({"base": op["fn"]})
            continue
        loop = next((k for k, (l0, l1) in enumerate(loops) if l0 <= a <= l1), None)
        if loop is None:
            warnings.append(f"read outside any loop at {a:#x}")
        if not phases or phases[-1].get("loop") != loop:
            phases.append({"loop": loop, "ops": []})
        phases[-1]["ops"].append(op)
    return phases, warnings


def main():
    bin_path, names_path, out = sys.argv[1:4]
    pe = PE(bin_path)
    names = load_funcs(names_path)
    starts = sorted(names)
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    schemas = {}
    nwarn = 0
    for va, n in sorted(names.items()):
        if not n.startswith("GameModelLoader::Set_v_"):
            continue
        i = bisect.bisect_right(starts, va)
        end = starts[i] if i < len(starts) else va + 0x10000
        ops, warns = extract(pe, md, va, end, names)
        schemas[n.split("::")[1]] = {"va": hex(va), "ops": ops, "warnings": warns}
        nwarn += bool(warns)
    json.dump(schemas, open(out, "w"), indent=1)
    print(len(schemas), "setters,", nwarn, "with warnings")


if __name__ == "__main__":
    main()
