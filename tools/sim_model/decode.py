"""Decode BTD6's `sim_model` TextAsset (GameModelLoader binary format).

The format below was recovered from GameModelLoader.Load and its helpers in
GameAssembly.dll (see research/03-sim-model-format.md). Per-type field read
sequences come from research/dumps/setter_schemas.json
(tools/il2cpp/setter_schema.py). The call order of Load is re-extracted here.

Usage:
    tools/.venv/Scripts/python.exe tools/sim_model/decode.py <GameAssembly.dll> <methods-all.jsonl> \
        <setter_schemas.json> <sim_model.bytes> <out_dir>

Writes <out_dir>/records.jsonl (one decoded record per object index) and
<out_dir>/summary.json. Output is proprietary game data: keep it local.
"""
import bisect
import json
import os
import struct
import sys

import capstone

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "il2cpp"))
from il2cpp_meta import PE  # noqa: E402

# GameModelLoader's generic helpers (CreateArraySet<T>, LinkArray<T>, ...) have no metadata
# names and move with every game build; resolve_helpers() finds them in Load by structure.
CREATE_ARRAY_SET = CREATE_LIST_SET = CREATE_DICT_SET = CREATE_RECORDS = None
LINK_ARRAY = LINK_LIST = LINK_DICT = None


class Reader:
    def __init__(self, data):
        self.d = data
        self.p = 0

    def i32(self):
        v = struct.unpack_from("<i", self.d, self.p)[0]
        self.p += 4
        return v

    def u16(self):
        v = struct.unpack_from("<H", self.d, self.p)[0]
        self.p += 2
        return v

    def f32(self):
        v = struct.unpack_from("<f", self.d, self.p)[0]
        self.p += 4
        return v

    def bool(self):
        v = self.d[self.p]
        assert v in (0, 1), f"bad bool {v} at {self.p}"
        self.p += 1
        return bool(v)

    def str(self):
        n, shift = 0, 0
        while True:
            b = self.d[self.p]
            self.p += 1
            n |= (b & 0x7F) << shift
            shift += 7
            if b < 0x80:
                break
        s = self.d[self.p:self.p + n].decode("utf-8")
        self.p += n
        return s

    def nstr(self):
        return None if self.bool() else self.str()


def load_call_sequence(pe, names, load_va):
    """Ordered list of (target_va) for direct calls in GameModelLoader.Load."""
    starts = sorted(names)
    end = starts[bisect.bisect_right(starts, load_va)]
    off = pe.va_to_off(load_va)
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_64)
    seq = []
    for ins in md.disasm(pe.d[off:off + (end - load_va)], load_va):
        if ins.mnemonic == "call" and ins.op_str.startswith("0x"):
            seq.append(int(ins.op_str, 16))
    return seq


def resolve_helpers(calls, names):
    """Unnamed Load call targets by role, in the order Load uses them.

    Between the first Read_* and the first Set_v_* call: CreateArraySet (interleaved with the
    Read_a_* readers), CreateListSet, CreateDictSet, CreateRecords. After the last Set_v_*:
    LinkArray, LinkList, LinkDictionary.
    """
    def short(c):
        return names.get(c, "").split("::")[-1]
    first_read = next(i for i, c in enumerate(calls) if short(c).startswith("Read_"))
    first_set = next(i for i, c in enumerate(calls) if short(c).startswith("Set_v_"))
    last_set = max(i for i, c in enumerate(calls) if short(c).startswith("Set_v_"))

    def unnamed(lo, hi):
        seen = []
        for c in calls[lo:hi]:
            if c not in names and c not in seen:
                seen.append(c)
        return seen
    create = unnamed(first_read - 1, first_set)
    link = unnamed(last_set + 1, len(calls))
    if len(create) < 4 or len(link) < 3:
        raise RuntimeError(f"GameModelLoader.Load layout changed: create={list(map(hex, create))} link={list(map(hex, link))}")
    return create[:4], link[:3]


def main():
    bin_path, names_path, schema_path, blob_path, out_dir = sys.argv[1:6]
    pe = PE(bin_path)
    names = {}
    for line in open(names_path, encoding="utf-8"):
        r = json.loads(line)
        if r["va"]:
            names.setdefault(int(r["va"], 16), f'{r["type"]}::{r["name"]}')
    load_va = next(va for va, n in names.items() if n == "GameModelLoader::Load")
    schemas = json.load(open(schema_path))
    calls = load_call_sequence(pe, names, load_va)
    global CREATE_ARRAY_SET, CREATE_LIST_SET, CREATE_DICT_SET, CREATE_RECORDS, LINK_ARRAY, LINK_LIST, LINK_DICT
    (CREATE_ARRAY_SET, CREATE_LIST_SET, CREATE_DICT_SET, CREATE_RECORDS), (LINK_ARRAY, LINK_LIST, LINK_DICT) = resolve_helpers(calls, names)
    print("loader helpers:", *map(hex, (CREATE_ARRAY_SET, CREATE_LIST_SET, CREATE_DICT_SET, CREATE_RECORDS, LINK_ARRAY, LINK_LIST, LINK_DICT)))

    r = Reader(open(blob_path, "rb").read())
    m = [None] * r.i32()
    mi = int(os.environ.get("MI0", "1"))  # m[0] is null: references use index 0 for null

    def put(v):
        nonlocal mi
        m[mi] = v
        mi += 1

    def read_arrays(elem):
        for _ in range(r.i32()):
            put([elem() for _ in range(r.i32())])

    readers = {
        "Read_a_Int32_Array": lambda: read_arrays(r.i32),
        "Read_a_AreaType_Array": lambda: read_arrays(r.i32),
        "Read_a_TowerSet_Array": lambda: read_arrays(r.i32),
        "Read_a_String_Array": lambda: read_arrays(r.nstr),
        "Read_a_TargetType_Array": lambda: read_arrays(lambda: {"id": r.str(), "isActionable": r.bool()}),
        "Read_a_Vector3_Array": lambda: read_arrays(lambda: [r.f32(), r.f32(), r.f32()]),
        "Read_a_Single_Array": lambda: read_arrays(r.f32),
        "Read_a_PrefabReference_Array": lambda: read_arrays(r.str),
        "Read_a_SpriteReference_Array": lambda: read_arrays(r.str),
        "Read_a_AudioClipReference_Array": lambda: read_arrays(r.str),
        "Read_a_Keyframe_Array": lambda: read_arrays(lambda: [r.f32() for _ in range(6)]),
        "Read_a_TowerTypeImmunity_Array": lambda: read_arrays(lambda: {"id": r.str(), "v": r.i32()}),
        "Read_a_Bounds_Array": lambda: read_arrays(lambda: [r.i32(), r.i32()]),
        "Read_l_Single_List": lambda: read_arrays(r.f32),
        "Read_String_v_Single_Dictionary": lambda: read_arrays(lambda: (r.nstr(), r.f32())),
        "Read_String_v_String_Dictionary": lambda: read_arrays(lambda: (r.nstr(), r.nstr())),
    }

    def container_set(kind):
        for _ in range(r.i32()):
            put({"_container": kind, "len": r.i32()})

    records = {}   # m index -> {"type", "fields"}

    def read_op(op, out):
        k = op["op"]
        if k == "i32":
            v = r.i32()
        elif k == "f32":
            v = r.f32()
        elif k == "u16":
            v = r.u16()
        elif k == "str":
            v = r.str()
        elif k == "bool":
            v = r.bool()
            if "if_false" in op or "if_true" in op:
                inner = op.get("if_false") if not v else op.get("if_true")
                if inner:
                    for o in inner:
                        read_op(o, out)
                    return
                out.append((op.get("store"), None))
                return
        else:
            raise NotImplementedError(f"op {k}")
        out.append((op.get("store"), v))

    def run_setter(fn, start, count):
        tname = fn[len("Set_v_"):-len("_Fields")]
        for ph in schemas[fn]["ops"]:
            if "base" in ph:
                run_setter(ph["base"], start, count)
                continue
            for i in range(start, start + count):
                rec = records.setdefault(i, {"type": tname, "fields": []})
                out = []
                p0 = r.p
                try:
                    for o in ph["ops"]:
                        read_op(o, out)
                except Exception as e:
                    raise RuntimeError(f"{fn} record {i} (#{i - start} of {count}) started at {p0}, "
                                       f"read so far {out}; prev setters {history[-4:]}") from e
                rec["fields"].append({"from": tname, "values": out})

    n_create = 0
    links = 0
    history = []
    set_calls = 0
    for ci, tgt in enumerate(calls):
        if os.environ.get("TRACE") and tgt in (CREATE_ARRAY_SET, CREATE_LIST_SET, CREATE_DICT_SET) or (os.environ.get("TRACE") and "Read_" in names.get(tgt, "")):
            print(ci, names.get(tgt, hex(tgt)), "pos", r.p, "mi", mi)
        n = names.get(tgt, "")
        short = n.split("::")[-1]
        if short in readers:
            readers[short]()
        elif tgt == CREATE_ARRAY_SET:
            container_set("array")
        elif tgt == CREATE_LIST_SET:
            container_set("list")
        elif tgt == CREATE_DICT_SET:
            container_set("dict")
        elif tgt == CREATE_RECORDS:
            cnt = r.i32()
            for _ in range(cnt):
                put({"_record": True})
            n_create += 1
        elif short.startswith("Set_v_"):
            start, count = r.i32(), r.i32()
            history.append((short, start, count, r.p))
            run_setter(short, start, count)
            records[start]["_setter_entry"] = True if count else None
            for i in range(start, start + count):
                records[i]["type"] = short[len("Set_v_"):-len("_Fields")]
            set_calls += 1
        elif tgt in (LINK_ARRAY, LINK_LIST):
            for _ in range(r.i32()):
                c = m[r.i32()]
                assert isinstance(c, dict) and "_container" in c, c
                c["items"] = [r.i32() for _ in range(c["len"])]
            links += 1
        elif tgt == LINK_DICT:
            for _ in range(r.i32()):
                c = m[r.i32()]
                assert isinstance(c, dict) and c.get("_container") == "dict", c
                c["items"] = [(r.str(), r.i32()) for _ in range(r.i32())]
            links += 1
        elif short.startswith(("Link", "Read_", "Create")):
            print("unhandled loader call", short, "at stream", r.p)
            break
    print(f"links={links} trailing bytes={r.d[r.p:r.p+32].hex()}")
    print(f"m={len(m)} filled={mi} create_records={n_create} setters={set_calls} stream {r.p}/{len(r.d)}")

    os.makedirs(out_dir, exist_ok=True)
    with open(os.path.join(out_dir, "records.jsonl"), "w", encoding="utf-8") as f:
        for i in sorted(records):
            f.write(json.dumps({"i": i, **records[i]}) + "\n")
    with open(os.path.join(out_dir, "values.json"), "w", encoding="utf-8") as f:
        json.dump({i: v for i, v in enumerate(m) if v is not None and not (isinstance(v, dict) and v.get("_record"))}, f)
    json.dump({"m": len(m), "filled": mi, "stream_pos": r.p, "stream_len": len(r.d),
               "create_records": n_create, "setters": set_calls},
              open(os.path.join(out_dir, "summary.json"), "w"), indent=1)


if __name__ == "__main__":
    main()
