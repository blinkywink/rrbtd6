"""Apply Monkey Knowledge to the decoded game model, from the game's own modded models.

SimRecorder's `mkdump` (scenario keys mkdump / mk) walks Simulation.model (GameModel.CreateModded with
the game's knowledge, difficulty and mode) beside Game.instance.model and writes where they differ
(tab separated: "MD tower path base modded", "MS tower path baseCount moddedCount", "MA tower path
type name" with "MF tower path value" lines for every field of the added element). Two dumps of the
same mode and difficulty, one with all knowledge and one with none, leave only what knowledge
changes: a change in the all-knowledge dump that the no-knowledge dump doesn't have (or has with
another modded value).

    python -I tools/assets/apply_mk.py local_extracted research/dumps/mk/all.mkdump \
        research/dumps/mk/none.mkdump local_extracted/decoded_mk

Writes <out>/named/objects.jsonl and <out>/values.json: the decoded set with each knowledge change
applied to that tower's own copy of the models on the path (copy on write: models are shared
between towers and tiers). Then build_game_data.py with RRBTD6_DECODED=<out> and
RRBTD6_GAME_JSON=game_mk.json exports the knowledge variant. Output is proprietary game data: keep it
local.
"""
import collections
import copy
import json
import os
import re
import sys

# The decoded set's name where it differs from the model's field (name_records' offset naming).
ALIAS = {("ArcEmissionModel", "count"): "SliceSize"}
# Model properties the dump shows under a decoded field's name (ArcEmissionModel.SliceSize is
# angle / count; the decoded "SliceSize" is the count).
SKIP = {("ArcEmissionModel", "SliceSize")}

# BloonProperties (flags), as the dump prints them by name.
BLOON_PROPS = {"None": 0, "Lead": 1, "Black": 2, "White": 4, "Purple": 8, "Frozen": 16}


def parse_dump(path):
    md, ms, ma, mf = {}, {}, {}, collections.defaultdict(list)
    for line in open(path, encoding="utf-8"):
        line = line.rstrip("\n")
        if not line or line.startswith("#"):
            continue
        p = line.split("\t")
        kind = p[0]
        if kind == "MD" and len(p) >= 5:
            md[(p[1], p[2])] = (p[3], p[4])
        elif kind == "MS" and len(p) >= 5:
            ms[(p[1], p[2])] = (int(p[3]), int(p[4]))
        elif kind == "MA" and len(p) >= 4:
            ma[(p[1], p[2])] = p[3]
        elif kind == "MF" and len(p) >= 4:
            mf[p[1]].append((p[2], p[3]))
    return md, ms, ma, mf


STEP = re.compile(r"/([A-Za-z_0-9<>$]+)|\[(\d+)(?::([A-Za-z_0-9]+))?\]")


def steps(path):
    return [(m.group(1), int(m.group(2)) if m.group(2) else None, m.group(3)) for m in STEP.finditer(path)]


def convert(text, old):
    """The dump's text for a field, typed like the decoded value it replaces."""
    if text in ("null", "obj"):
        return None
    if isinstance(old, bool):
        return text == "True"
    if isinstance(old, int):
        if all(x.strip() in BLOON_PROPS for x in text.split(",")):
            return sum(BLOON_PROPS[x.strip()] for x in text.split(","))
        try:
            return int(text)
        except ValueError:
            try:
                return int(float(text))
            except ValueError:
                return None
    if isinstance(old, float):
        return float(text)
    if isinstance(old, str) or old is None:
        return text[1:-1] if text.startswith('"') and text.endswith('"') else text
    return None


def main():
    root, all_p, none_p, out = sys.argv[1:5]
    objs = {}
    for line in open(os.path.join(root, "decoded", "named", "objects.jsonl"), encoding="utf-8"):
        r = json.loads(line)
        objs[r["i"]] = r
    values = {int(k): v for k, v in json.load(open(os.path.join(root, "decoded", "values.json"))).items()}
    next_id = [max(max(objs), max(values)) + 1]

    def new_id():
        next_id[0] += 1
        return next_id[0]

    a_md, a_ms, a_ma, a_mf = parse_dump(all_p)
    n_md, n_ms, n_ma, _ = parse_dump(none_p)
    # Knowledge-only changes.
    md = {k: v for k, v in a_md.items() if n_md.get(k, (None, None))[1] != v[1]}
    ms = {k: v for k, v in a_ms.items() if n_ms.get(k) != v}
    ma = {k: v for k, v in a_ma.items() if k not in n_ma}

    # GameModel.towers, each given its own root object (a tower model is also referenced from inside
    # others, a CreateTowerModel's tower: those keep the base model, as the game's do).
    gm = next(o for o in objs.values() if o["type"] == "GameModel")
    gcont = gm["f"]["towers"]["$ref"]
    # Per tower: which object ids are already this tower's own copies.
    own = collections.defaultdict(set)

    def copy_tree(tid, mine):
        """A copy of the tower's whole model tree, objects shared within it kept shared (the game
        changes a shared model once, seen from every path; the dump names only the first)."""
        memo = {}

        def cp(oid):
            if oid in memo:
                return memo[oid]
            nid = new_id()
            memo[oid] = nid
            o = copy.deepcopy(objs[oid])
            o["i"] = nid
            objs[nid] = o
            mine.add(nid)
            for k, v in list(o["f"].items()):
                if isinstance(v, dict) and "$ref" in v:
                    r = v["$ref"]
                    if r in objs:
                        o["f"][k] = {"$ref": cp(r)}
                    elif isinstance(values.get(r), dict) and "items" in values[r]:
                        if ("c", r) not in memo:
                            cid = new_id()
                            memo[("c", r)] = cid
                            c = copy.deepcopy(values[r])
                            values[cid] = c
                            mine.add(cid)
                            c["items"] = [cp(e) if isinstance(e, int) and e in objs else e for e in c["items"]]
                        o["f"][k] = {"$ref": memo[("c", r)]}
            return nid
        return cp(tid)

    towers = {}
    for k, tid in enumerate(values[gcont]["items"]):
        if tid not in objs:
            continue
        nm = objs[tid]["f"].get("_name")
        if nm in towers:
            continue
        towers[nm] = tid
    # (All copied from the base model before any change: a tower model is also inside others.)
    roots = {}
    for nm, tid in towers.items():
        roots[nm] = copy_tree(tid, own[nm])
    for k, tid in enumerate(values[gcont]["items"]):
        nm = objs[tid]["f"].get("_name") if tid in objs else None
        if nm in roots and towers[nm] == tid:
            values[gcont]["items"][k] = roots[nm]
    towers = roots

    def own_obj(tower, oid):
        """This tower's copy of object `oid` (made on first write)."""
        if oid in own[tower]:
            return oid
        nid = new_id()
        o = copy.deepcopy(objs[oid])
        o["i"] = nid
        objs[nid] = o
        own[tower].add(nid)
        return nid

    def own_container(tower, cid):
        if cid in own[tower]:
            return cid
        nid = new_id()
        values[nid] = copy.deepcopy(values[cid])
        own[tower].add(nid)
        return nid

    def walk_to(tower, root_id, st, make_own):
        """Follow the steps to the object holding the last field; (object id, field) or None."""
        cur = root_id
        for k, (name, idx, _ty) in enumerate(st[:-1]):
            o = objs[cur]
            if name is not None:
                v = o["f"].get(name)
                if not (isinstance(v, dict) and "$ref" in v):
                    return None
                ref = v["$ref"]
                nxt = st[k + 1]
                if nxt[1] is not None:
                    # A container; the element by index.
                    if ref not in values or not isinstance(values[ref], dict) or "items" not in values[ref]:
                        return None
                    if make_own:
                        c = own_container(tower, ref)
                        o["f"][name] = {"$ref": c}
                        ref = c
                    its = values[ref]["items"]
                    if nxt[1] >= len(its) or its[nxt[1]] not in objs:
                        return None
                    # (The base element's type, as the dump names it: same element.)
                    if nxt[2] and objs[its[nxt[1]]]["type"] != nxt[2]:
                        return None
                    if make_own:
                        its[nxt[1]] = own_obj(tower, its[nxt[1]])
                    cur = its[nxt[1]]
                    continue
                if ref not in objs:
                    return None
                if make_own:
                    nref = own_obj(tower, ref)
                    o["f"][name] = {"$ref": nref}
                    ref = nref
                cur = ref
            # (an index step was consumed with its container above)
        return cur

    applied, skipped = 0, collections.Counter()
    nofield = collections.Counter()
    for (tower, path), (_base, modv) in sorted(md.items()):
        root_id = towers.get(tower)
        if root_id is None:
            skipped["no tower"] += 1
            continue
        st = steps(path)
        if not st or st[-1][0] is None:
            skipped["index leaf"] += 1
            continue
        if tower not in own or root_id not in own[tower]:
            own[tower].add(root_id)
        # Dry run first: skip paths the decoded set doesn't have.
        hold = walk_to(tower, root_id, st, False)
        if hold is not None and (objs[hold]["type"], st[-1][0]) in SKIP:
            skipped["derived"] += 1
            continue
        if hold is not None and (objs[hold]["type"], st[-1][0]) in ALIAS:
            st[-1] = (ALIAS[(objs[hold]["type"], st[-1][0])], None, None)
        if hold is None or st[-1][0] not in objs[hold]["f"]:
            skipped["no field"] += 1
            nofield[(st[-1][0], hold is None)] += 1
            continue
        old = objs[hold]["f"][st[-1][0]]
        if isinstance(old, dict):
            skipped["ref field"] += 1
            continue
        nv = convert(modv, old)
        if nv is None and old is not None:
            skipped["untyped " + st[-1][0]] += 1
            continue
        hold = walk_to(tower, root_id, st, True)
        objs[hold]["f"][st[-1][0]] = nv
        applied += 1
    print(f"applied {applied} field changes; skipped {dict(skipped)}")
    print("no field:", nofield.most_common(12))

    # Added elements: built from their MF lines (fields only, per field_offsets.json).
    fo = json.load(open(os.path.join(os.path.dirname(__file__), "..", "..", "research", "dumps", "field_offsets.json")))
    fields = {}
    for full, fl in fo.items():
        short = full.split(".")[-1].split("/")[-1]
        fields.setdefault(short, [n for n, off in fl if off >= 16])
    mf_by_tower = {t: dict(v) for t, v in a_mf.items()}

    def prim(text):
        if text == "null":
            return None
        if text in ("True", "False"):
            return text == "True"
        if text.startswith('"') and text.endswith('"'):
            return text[1:-1]
        if re.fullmatch(r"-?\d+", text):
            return int(text)
        try:
            return float(text)
        except ValueError:
            if all(x.strip() in BLOON_PROPS for x in text.split(",")):
                return sum(BLOON_PROPS[x.strip()] for x in text.split(","))
            return text

    unknown = collections.Counter()

    def build(lines, pre):
        ty = lines.get(pre + "/$type")
        if ty is None:
            return None
        oid = new_id()
        f = {}
        # (Its own fields and the inherited Model / behaviour ones.)
        for name in fields.get(ty, []) + [x for x in ("_name", "collisionPass", "runtimeTypeIndex", "isTower") if x not in fields.get(ty, [])]:
            sub = pre + "/" + name
            if sub + "/$type" in lines:
                f[name] = {"$ref": build(lines, sub)}
            elif sub + "/$count" in lines:
                n = int(lines[sub + "/$count"])
                cid = new_id()
                its = []
                for k in range(n):
                    e = f"{sub}[{k}]"
                    if e + "/$type" in lines:
                        its.append(build(lines, e))
                    elif e in lines:
                        vid = new_id()
                        values[vid] = prim(lines[e])
                        its.append(vid)
                values[cid] = {"_container": "array", "len": len(its), "items": its}
                f[name] = {"$ref": cid}
            elif sub in lines:
                f[name] = prim(lines[sub])
        if ty not in fields:
            unknown[ty] += 1
        objs[oid] = {"i": oid, "type": ty, "f": f}
        return oid

    added = 0
    noparent_ex = {}
    idx_ex = []
    def path_key(kv):
        return (kv[0][0], [(n or "", -1 if i is None else i) for n, i, _ in steps(kv[0][1])])

    for (tower, path), _what in sorted(ma.items(), key=path_key):
        root_id = towers.get(tower)
        lines = mf_by_tower.get(tower)
        if root_id is None or lines is None:
            continue
        st = steps(path)
        if not st or st[-1][1] is None or len(st) < 2:
            continue
        # Owner of the container: walk to the field before the index.
        hold = walk_to(tower, root_id, st[:-1], False)
        cont_field = st[-2][0]
        if hold is None or cont_field is None or not isinstance(objs[hold]["f"].get(cont_field), dict):
            skipped["MA no parent"] += 1
            if "/path[" not in path and "/easePath[" not in path and noparent_ex.setdefault(len(noparent_ex), (tower, path)) and len(noparent_ex) > 8:
                pass
            continue
        hold = walk_to(tower, root_id, st[:-1], True)
        ref = objs[hold]["f"][cont_field]["$ref"]
        if ref not in values or "items" not in values[ref]:
            skipped["MA no container"] += 1
            continue
        c = own_container(tower, ref)
        objs[hold]["f"][cont_field] = {"$ref": c}
        its = values[c]["items"]
        if st[-1][1] != len(its):
            skipped["MA index"] += 1
            if len(idx_ex) < 6:
                idx_ex.append((tower, path, len(its)))
            continue
        nid = build(lines, path)
        if nid is None:
            skipped["MA unbuilt"] += 1
            continue
        own[tower].add(nid)
        its.append(nid)
        values[c]["len"] = len(its)
        added += 1
    for x in idx_ex:
        print("  index:", *x)
    for k in list(noparent_ex)[:3]:
        print("  no parent:", *noparent_ex[k])
    print(f"added {added} elements; skipped {dict(skipped)}; types without field lists {dict(unknown)}")

    os.makedirs(os.path.join(out, "named"), exist_ok=True)
    with open(os.path.join(out, "named", "objects.jsonl"), "w", encoding="utf-8") as f:
        for i in sorted(objs):
            f.write(json.dumps(objs[i]) + "\n")
    json.dump({str(k): v for k, v in values.items()}, open(os.path.join(out, "values.json"), "w"))


if __name__ == "__main__":
    main()
