"""Translate a Unity shader's forward pass (D3D11 DXBC, shader model 4/5) to GLSL 330.

Used by export_model.py for map materials whose look comes from their shader (water ripples,
panning water, waterfalls, god rays...). The bytecode is disassembled with d3dcompiler_47's
D3DDisassemble and each instruction becomes one GLSL statement over vec4 registers, so the
result does exactly what the game's shader does. Integer / mask instructions work on the raw
bits (floatBitsToUint / uintBitsToFloat), as on the GPU.

translate(shader_obj) -> dict:
  vs, fs            GLSL 330 sources
  cbuffers          [{"name", "slot", "regs", "vectors": [[name, byte offset, dims]], "matrices": [[name, offset]]}]
                    per stage: uniform vec4 vs_cb<slot>[regs] / fs_cb<slot>[regs]
  textures          [[property name, slot]]  ->  uniform sampler2D t<slot>
  state             zwrite, ztest, cull, src / dst blend (UnityEngine.Rendering enums), queue
  attributes        what the vertex shader reads: POSITION, NORMAL, TEXCOORD0, COLOR...

Output is derived from the game's shaders: it is written under local_extracted/ only.
"""
import ctypes
import re
import struct

import lz4.block

_d3d = None


def disassemble(code):
    global _d3d
    if _d3d is None:
        _d3d = ctypes.WinDLL("d3dcompiler_47.dll")
        _d3d.D3DDisassemble.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_uint, ctypes.c_char_p,
                                        ctypes.POINTER(ctypes.c_void_p)]
    p = ctypes.c_void_p()
    if _d3d.D3DDisassemble(code, len(code), 0, None, ctypes.byref(p)) != 0:
        raise ValueError("D3DDisassemble failed")
    vt = ctypes.cast(ctypes.cast(p, ctypes.POINTER(ctypes.c_void_p))[0], ctypes.POINTER(ctypes.c_void_p))
    get_ptr = ctypes.WINFUNCTYPE(ctypes.c_void_p, ctypes.c_void_p)(vt[3])
    get_size = ctypes.WINFUNCTYPE(ctypes.c_size_t, ctypes.c_void_p)(vt[4])
    txt = ctypes.string_at(get_ptr(p), get_size(p)).decode(errors="replace")
    ctypes.WINFUNCTYPE(ctypes.c_ulong, ctypes.c_void_p)(vt[2])(p)  # Release
    return txt.replace("\x00", "")


def blob_entries(t):
    """The shader's platform-0 blob, split into its serialized programs / parameter blocks."""
    blob = bytes(t["compressedBlob"])
    out = []
    offs, cls, dls = t["offsets"], t["compressedLengths"], t["decompressedLengths"]
    off, cl, dl = offs[0], cls[0], dls[0]
    off = off if isinstance(off, list) else [off]
    cl = cl if isinstance(cl, list) else [cl]
    dl = dl if isinstance(dl, list) else [dl]
    for a, b, c in zip(off, cl, dl):
        data = lz4.block.decompress(blob[a:a + b], uncompressed_size=c)
        # count, then (offset, length, segment) per entry.
        n = struct.unpack_from("<I", data, 0)[0]
        for i in range(n):
            o, ln, _seg = struct.unpack_from("<III", data, 4 + 12 * i)
            out.append(data[o:o + ln])
    return out


def dxbc_in(entry):
    i = entry.find(b"DXBC")
    if i < 0:
        return None
    size = struct.unpack_from("<I", entry, i + 24)[0]
    return entry[i:i + size]


SKIP_PASSES = {"shadowcaster", "depthonly", "depthnormals", "depthnormalsonly", "motionvectors", "meta", "universal2d",
               "scenepass", "scene selection pass", "scenepicking", "picking"}


def pick_subprogram(prog):
    """The keyword-free variant of a stage (first tier that has one)."""
    tiers = prog.get("m_PlayerSubPrograms") or []
    best = None
    for tier in tiers:
        for sp in tier:
            if best is None or (not sp.get("m_KeywordIndices") and best.get("m_KeywordIndices")):
                best = sp
    if best is None:
        subs = prog.get("m_SubPrograms") or []
        for sp in subs:
            if best is None or not sp.get("m_KeywordIndices"):
                best = sp
    return best


# ------------------------------------------------------------------ translation

SWZ = "xyzw"


class Operand:
    def __init__(self, text, stage):
        self.raw = text
        t = text.strip()
        self.neg = t.startswith("-")
        if self.neg:
            t = t[1:]
        self.abs = t.startswith("|") and t.endswith("|")
        if self.abs:
            t = t[1:-1]
        self.lit = None
        self.swz = "xyzw"
        if t.startswith("l("):
            self.lit = [v.strip() for v in t[2:-1].split(",")]
            self.base = None
            return
        m = re.fullmatch(r"([a-z]+)(\d+)(\[[^\]]+\])?(?:\.([xyzw]+))?", t)
        if not m:
            raise ValueError(f"operand {text!r}")
        kind, num, idx, swz = m.groups()
        self.kind, self.num = kind, int(num)
        if swz:
            self.swz = swz
        if kind == "cb":
            reg = idx[1:-1]
            reg = re.sub(r"r(\d+)\.([xyzw])", lambda mm: f"int(floatBitsToUint(r{mm.group(1)}.{mm.group(2)}))", reg)
            self.base = f"{stage}_cb{num}[{reg}]"
        elif kind == "icb":
            raise ValueError("immediate constant buffer")
        else:
            self.base = f"{kind}{num}"

    def vec4(self, as_int=False):
        """The operand as a vec4 whose component i is what destination component i reads."""
        if self.lit is not None:
            vals = self.lit if len(self.lit) == 4 else self.lit * 4
            comps = []
            for v in vals:
                if v.lower().startswith("0x") or (as_int and re.fullmatch(r"-?\d+", v)):
                    n = int(v, 16) if v.lower().startswith("0x") else int(v)
                    comps.append(f"uintBitsToFloat({n & 0xFFFFFFFF}u)")
                else:
                    comps.append(_flt(v))
            e = f"vec4({', '.join(comps)})"
        else:
            s = self.swz if len(self.swz) == 4 else (self.swz * 4)[:4] if len(self.swz) == 1 else (self.swz + self.swz[-1] * 4)[:4]
            e = f"{self.base}.{s}"
        if self.abs:
            e = f"abs({e})"
        if self.neg:
            e = f"(-{e})" if not as_int else f"ineg4({e})"
        return e


def _flt(v):
    f = float(v)
    if f != f:
        return "uintBitsToFloat(0x7fc00000u)"
    if f in (float("inf"), float("-inf")):
        return "uintBitsToFloat(0x7f800000u)" if f > 0 else "uintBitsToFloat(0xff800000u)"
    return repr(f) if "e" not in repr(f) else f"{f:.9e}"


def split_args(s):
    out, depth, cur = [], 0, ""
    for ch in s:
        if ch in "([":
            depth += 1
        elif ch in ")]":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


HELPERS = """
vec4 bmask(bvec4 b) { return uintBitsToFloat(uvec4(b) * 0xFFFFFFFFu); }
uvec4 ub(vec4 v) { return floatBitsToUint(v); }
vec4 fb(uvec4 v) { return uintBitsToFloat(v); }
vec4 ineg4(vec4 v) { return fb(uvec4(-ivec4(ub(v)))); }
vec4 movc4(vec4 c, vec4 a, vec4 b) { return mix(b, a, vec4(notEqual(ub(c), uvec4(0u)))); }
"""


def translate_program(asm, stage, tex_dims):
    """GLSL body + declarations for one disassembled program. stage: 'vs' or 'fs'."""
    lines = [l.strip() for l in asm.splitlines()]
    decls, body = [], []
    cbs = {}
    inputs, outputs = {}, {}
    temps = 0
    samplers = set()
    indent = 1
    # Signatures (from the disassembly comments): register -> semantic.
    sig = {"in": {}, "out": {}}
    which = None
    for l in lines:
        if l.startswith("// Input signature"):
            which = "in"
        elif l.startswith("// Output signature"):
            which = "out"
        elif which and l.startswith("//"):
            m = re.match(r"//\s+(\w+)\s+(\d+)\s+([xyzw]+)\s+(\d+)\s+(\w+)", l)
            if m:
                name, idx, mask, reg, sysv = m.groups()
                sig[which].setdefault(int(reg), (name, int(idx), sysv))
        elif not l.startswith("//"):
            which = None
    for l in lines:
        if not l or l.startswith("//") or l.startswith("vs_") or l.startswith("ps_"):
            continue
        op, _, rest = l.partition(" ")
        args = split_args(rest)
        if op == "dcl_constantbuffer":
            m = re.match(r"CB(\d+)\[(\d+)\]", args[0])
            cbs[int(m.group(1))] = int(m.group(2))
            continue
        if op == "dcl_temps":
            temps = int(args[0])
            continue
        if op.startswith("dcl_input"):
            m = re.search(r"(?:^| )v([0-9]+)", args[0])
            if m:
                inputs[int(m.group(1))] = True
            continue
        if op.startswith("dcl_output"):
            m = re.match(r"o(\d+)", args[0])
            if m:
                outputs[int(m.group(1))] = True
            continue
        if op.startswith("dcl_resource_texture2d"):
            samplers.add(int(re.search(r" t([0-9]+)", " " + args[-1]).group(1)))
            continue
        if op.startswith("dcl_"):
            continue
        sat = op.endswith("_sat")
        if sat:
            op = op[:-4]
        ind = "    " * indent
        if op == "ret":
            body.append(ind + "return;")
            continue
        if op in ("if_nz", "if_z"):
            a = Operand(args[0], stage)
            cmp = "!=" if op == "if_nz" else "=="
            body.append(f"{ind}if (floatBitsToUint({a.vec4()}.x) {cmp} 0u) {{")
            indent += 1
            continue
        if op == "else":
            body.append("    " * (indent - 1) + "} else {")
            continue
        if op == "endif":
            indent -= 1
            body.append("    " * indent + "}")
            continue
        if op in ("discard_nz", "discard_z"):
            a = Operand(args[0], stage)
            cmp = "!=" if op == "discard_nz" else "=="
            body.append(f"{ind}if (floatBitsToUint({a.vec4()}.x) {cmp} 0u) discard;")
            continue
        if op == "sincos":
            src = Operand(args[2], stage).vec4()
            for d, fn in ((args[0], "sin"), (args[1], "cos")):
                if d != "null":
                    dst = Operand(d, stage)
                    body.append(f"{ind}{dst.base}.{dst.swz} = {_sat(f'{fn}({src})', sat)}.{dst.swz};")
            continue
        if op in ("imul", "umul", "udiv") and args[0] == "null":
            # Two-destination ops (hi / lo, quotient / remainder) with only the second used.
            args = args[1:]
        elif op in ("imul", "umul", "udiv"):
            raise ValueError(f"unsupported two-destination {op}: {l}")
        dst = Operand(args[0], stage)
        srcs = [Operand(a, stage) for a in args[1:]]
        int_ops = {"and", "or", "xor", "not", "iadd", "imul", "umul", "ishl", "ishr", "ushr", "ieq", "ine", "ilt", "ige", "ult", "uge",
                   "itof", "utof", "imax", "imin", "umax", "umin", "movc", "bfi", "ubfe", "ibfe", "udiv"}
        v = [s.vec4(as_int=op in int_ops and op not in ("itof", "utof", "movc")) for s in srcs]
        e = None
        if op == "mov":
            e = v[0]
        elif op == "add":
            e = f"({v[0]} + {v[1]})"
        elif op == "mul":
            e = f"({v[0]} * {v[1]})"
        elif op == "mad":
            e = f"({v[0]} * {v[1]} + {v[2]})"
        elif op == "div":
            e = f"({v[0]} / {v[1]})"
        elif op in ("dp2", "dp3", "dp4"):
            n = int(op[2])
            e = f"vec4(dot({v[0]}.{SWZ[:n]}, {v[1]}.{SWZ[:n]}))"
        elif op == "min":
            e = f"min({v[0]}, {v[1]})"
        elif op == "max":
            e = f"max({v[0]}, {v[1]})"
        elif op == "rsq":
            e = f"inversesqrt({v[0]})"
        elif op == "sqrt":
            e = f"sqrt({v[0]})"
        elif op == "exp":
            e = f"exp2({v[0]})"
        elif op == "log":
            e = f"log2({v[0]})"
        elif op == "rcp":
            e = f"(1.0 / {v[0]})"
        elif op == "frc":
            e = f"fract({v[0]})"
        elif op == "round_ni":
            e = f"floor({v[0]})"
        elif op == "round_pi":
            e = f"ceil({v[0]})"
        elif op == "round_z":
            e = f"trunc({v[0]})"
        elif op == "round_ne":
            e = f"roundEven({v[0]})"
        elif op in ("ge", "lt", "eq", "ne"):
            fn = {"ge": "greaterThanEqual", "lt": "lessThan", "eq": "equal", "ne": "notEqual"}[op]
            e = f"bmask({fn}({v[0]}, {v[1]}))"
        elif op in ("ieq", "ine", "ilt", "ige"):
            fn = {"ieq": "equal", "ine": "notEqual", "ilt": "lessThan", "ige": "greaterThanEqual"}[op]
            e = f"bmask({fn}(ivec4(ub({v[0]})), ivec4(ub({v[1]}))))"
        elif op in ("ult", "uge"):
            fn = {"ult": "lessThan", "uge": "greaterThanEqual"}[op]
            e = f"bmask({fn}(ub({v[0]}), ub({v[1]})))"
        elif op == "and":
            e = f"fb(ub({v[0]}) & ub({v[1]}))"
        elif op == "or":
            e = f"fb(ub({v[0]}) | ub({v[1]}))"
        elif op == "xor":
            e = f"fb(ub({v[0]}) ^ ub({v[1]}))"
        elif op == "not":
            e = f"fb(~ub({v[0]}))"
        elif op == "iadd":
            e = f"fb(ub({v[0]}) + ub({v[1]}))"
        elif op == "ishl":
            e = f"fb(ub({v[0]}) << (ub({v[1]}) & 31u))"
        elif op == "ushr":
            e = f"fb(ub({v[0]}) >> (ub({v[1]}) & 31u))"
        elif op == "ishr":
            e = f"fb(uvec4(ivec4(ub({v[0]})) >> ivec4(ub({v[1]}) & 31u)))"
        elif op in ("imax", "imin"):
            e = f"fb(uvec4({op[1:]}(ivec4(ub({v[0]})), ivec4(ub({v[1]})))))"
        elif op in ("umax", "umin"):
            e = f"fb({op[1:]}(ub({v[0]}), ub({v[1]})))"
        elif op in ("imul", "umul"):
            e = f"fb(ub({v[0]}) * ub({v[1]}))"
        elif op == "itof":
            e = f"vec4(ivec4(floatBitsToUint({v[0]})))"
        elif op == "utof":
            e = f"vec4(floatBitsToUint({v[0]}))"
        elif op == "ftoi":
            e = f"fb(uvec4(ivec4({v[0]})))"
        elif op == "ftou":
            e = f"fb(uvec4({v[0]}))"
        elif op == "movc":
            e = f"movc4({v[0]}, {v[1]}, {v[2]})"
        elif op in ("sample", "sample_b", "sample_l", "sample_d", "sample_c_lz"):
            t = srcs[1]
            coord = srcs[0].vec4()
            tex = f"t{t.num}"
            # UnityPy exports Texture2D images top-down (its default vertical flip).
            # Mesh inputs above restore Unity UVs, whose v=0 addresses the bottom row.
            # Restore that texture-space convention at every source shader sample.
            uv = f"vec2(({coord}).x, 1.0 - ({coord}).y)"
            if op == "sample_b":
                e = f"texture({tex}, {uv}, {srcs[3].vec4()}.x)"
            elif op == "sample_l":
                e = f"textureLod({tex}, {uv}, {srcs[3].vec4()}.x)"
            elif op == "sample_d":
                dx, dy = srcs[3].vec4(), srcs[4].vec4()
                e = f"textureGrad({tex}, {uv}, vec2(({dx}).x, -({dx}).y), vec2(({dy}).x, -({dy}).y))"
            else:
                e = f"texture({tex}, {uv})"
            e = f"{e}.{t.swz if len(t.swz) == 4 else (t.swz * 4)[:4]}"
        elif op == "deriv_rtx" or op == "deriv_rtx_coarse" or op == "deriv_rtx_fine":
            e = f"dFdx({v[0]})"
        elif op == "deriv_rty" or op == "deriv_rty_coarse" or op == "deriv_rty_fine":
            e = f"(-dFdy({v[0]}))"
        else:
            raise ValueError(f"unsupported instruction {op}: {l}")
        body.append(f"{ind}{dst.base}.{dst.swz} = {_sat(e, sat)}.{dst.swz};")
    return {"cbs": cbs, "temps": temps, "inputs": sorted(inputs), "outputs": sorted(outputs), "samplers": sorted(samplers),
            "sig": sig, "body": body}


def _sat(e, sat):
    return f"clamp({e}, 0.0, 1.0)" if sat else e


def _varying(sem):
    name, idx, _ = sem
    return f"v_{name.lower()}{idx}"


ATTRIB = {
    # Unity semantic -> expression from macroquad's attributes (GL space; z flipped back, UV v flipped back).
    "POSITION": "vec4(position.x, position.y, -position.z, 1.0)",
    "NORMAL": "vec4(normal.x, normal.y, -normal.z, 0.0)",
    "TEXCOORD": "vec4(texcoord.x, 1.0 - texcoord.y, 0.0, 0.0)",
    "COLOR": "(color0 / 255.0)",
    "TANGENT": "vec4(1.0, 0.0, 0.0, 1.0)",
}


def build(vs_asm, fs_asm):
    vs = translate_program(vs_asm, "vs", {})
    fs = translate_program(fs_asm, "fs", {})
    # Vertex shader.
    out = ["#version 330", "in vec3 position;", "in vec2 texcoord;", "in vec4 color0;", "in vec4 normal;"]
    for slot, n in sorted(vs["cbs"].items()):
        out.append(f"uniform vec4 vs_cb{slot}[{n}];")
    vary = {}
    for reg in vs["outputs"]:
        sem = vs["sig"]["out"].get(reg)
        if sem and sem[2] != "POS":
            vary[reg] = _varying(sem)
            out.append(f"out vec4 {vary[reg]};")
    out.append(HELPERS)
    out.append("void main() {")
    for i in range(vs["temps"]):
        out.append(f"    vec4 r{i} = vec4(0.0);")
    for reg in vs["inputs"]:
        sem = vs["sig"]["in"].get(reg, ("POSITION", 0, "NONE"))
        out.append(f"    vec4 v{reg} = {ATTRIB.get(sem[0], 'vec4(0.0)')};")
    for reg in vs["outputs"]:
        out.append(f"    vec4 o{reg} = vec4(0.0);")
    out += ["    do {"] + ["    " + l for l in vs["body"]] + ["    } while (false);"]
    for reg in vs["outputs"]:
        sem = vs["sig"]["out"].get(reg)
        if sem and sem[2] == "POS":
            out.append(f"    gl_Position = o{reg};")
        elif reg in vary:
            out.append(f"    {vary[reg]} = o{reg};")
    out.append("}")
    vs_src = "\n".join(l.replace("return;", "break;") for l in out)
    # Fragment shader.
    out = ["#version 330"]
    for slot, n in sorted(fs["cbs"].items()):
        out.append(f"uniform vec4 fs_cb{slot}[{n}];")
    for t in fs["samplers"]:
        out.append(f"uniform sampler2D t{t};")
    by_sem = {(s[0], s[1]): _varying(s) for s in vs["sig"]["out"].values() if s[2] != "POS"}
    fin = {}
    for reg in fs["inputs"]:
        sem = fs["sig"]["in"].get(reg)
        if sem and sem[2] == "POS":
            fin[reg] = "gl_FragCoord"
        elif sem:
            name = by_sem.get((sem[0], sem[1]))
            if name:
                out.append(f"in vec4 {name};")
            fin[reg] = name or "vec4(0.0)"
    out.append("out vec4 fragColor;")
    out.append(HELPERS)
    out.append("void main() {")
    for i in range(fs["temps"]):
        out.append(f"    vec4 r{i} = vec4(0.0);")
    for reg, e in fin.items():
        out.append(f"    vec4 v{reg} = {e};")
    for reg in fs["outputs"]:
        out.append(f"    vec4 o{reg} = vec4(0.0);")
    out += ["    do {"] + ["    " + l for l in fs["body"]] + ["    } while (false);"]
    out.append("    fragColor = o0;")
    out.append("}")
    fs_src = "\n".join(l.replace("return;", "break;") for l in out)
    attrs = sorted({vs["sig"]["in"].get(r, ("?",))[0] for r in vs["inputs"]})
    return vs_src, fs_src, vs["cbs"], fs["cbs"], fs["samplers"], attrs


def cb_layout(pass_, prog, names):
    cp = pass_[prog]["m_CommonParameters"]
    binds = {names.get(b["m_NameIndex"]): b["m_Index"] for b in cp["m_ConstantBufferBindings"]}
    out = []
    for c in cp["m_ConstantBuffers"]:
        nm = names.get(c["m_NameIndex"])
        out.append({"name": nm, "slot": binds.get(nm), "size": c.get("m_Size"),
                    "vectors": [[names.get(v["m_NameIndex"]), v["m_Index"], v["m_Dim"]] for v in c["m_VectorParams"]],
                    "matrices": [[names.get(v["m_NameIndex"]), v["m_Index"]] for v in c["m_MatrixParams"]]})
    tex = [[names.get(x["m_NameIndex"]), x["m_Index"]] for x in cp["m_TextureParams"]]
    # Shader-model-4 "$Globals" vectors outside any named buffer (legacy shaders).
    loose = [[names.get(v["m_NameIndex"]), v["m_Index"], v["m_Dim"]] for v in cp.get("m_VectorParams", [])]
    return out, tex, loose


def translate(shader_obj):
    t = shader_obj.read_typetree()
    pf = t["m_ParsedForm"]
    entries = blob_entries(t)
    ss = pf["m_SubShaders"][0]
    for pi, p in enumerate(ss["m_Passes"]):
        st = p["m_State"]
        if (st.get("m_Name") or "").lower() in SKIP_PASSES:
            continue
        tags = dict(p["m_Tags"]["tags"]) if p.get("m_Tags") else {}
        if tags.get("LightMode", "").lower() in SKIP_PASSES:
            continue
        vsp, fsp = pick_subprogram(p["progVertex"]), pick_subprogram(p["progFragment"])
        if not vsp or not fsp:
            continue
        vcode, fcode = dxbc_in(entries[vsp["m_BlobIndex"]]), dxbc_in(entries[fsp["m_BlobIndex"]])
        if not vcode or not fcode:
            continue
        vs_asm, fs_asm = disassemble(vcode), disassemble(fcode)
        vs_src, fs_src, vcb, fcb, samplers, attrs = build(vs_asm, fs_asm)
        names = {v: k for k, v in p["m_NameIndices"]}
        vlay, _, _ = cb_layout(p, "progVertex", names)
        flay, tex, _ = cb_layout(p, "progFragment", names)
        sub_tags = dict(ss["m_Tags"]["tags"]) if ss.get("m_Tags") else {}
        q = sub_tags.get("QUEUE") or sub_tags.get("Queue") or "Geometry"
        rb = st["rtBlend0"]
        return {
            "name": pf["m_Name"],
            "vs": vs_src,
            "fs": fs_src,
            "vs_cbuffers": [dict(c, regs=vcb.get(c["slot"], 0)) for c in vlay if c["slot"] in vcb],
            "fs_cbuffers": [dict(c, regs=fcb.get(c["slot"], 0)) for c in flay if c["slot"] in fcb],
            "textures": [x for x in tex if x[1] in samplers],
            "attributes": attrs,
            "state": {"zwrite": st["zWrite"]["val"], "ztest": st["zTest"]["val"], "cull": st["culling"]["val"],
                      "src": rb["srcBlend"]["val"], "dst": rb["destBlend"]["val"], "queue": q,
                      "zwrite_prop": st["zWrite"].get("name"), "src_prop": rb["srcBlend"].get("name"),
                      "dst_prop": rb["destBlend"].get("name"), "cull_prop": st["culling"].get("name")},
            "asm": {"vs": vs_asm, "fs": fs_asm},
        }
    raise ValueError("no forward pass")


if __name__ == "__main__":
    import glob
    import os
    import sys

    import UnityPy
    G, bundle, name = sys.argv[1:4]
    aa = os.path.join(G, "BloonsTD6_Data", "StreamingAssets", "aa", "StandaloneWindows64", "Full")
    env = UnityPy.load(*glob.glob(os.path.join(aa, bundle)))
    for o in env.objects:
        if o.type.name == "Shader" and o.read_typetree()["m_ParsedForm"]["m_Name"] == name:
            r = translate(o)
            print(r["vs"])
            print("-----")
            print(r["fs"])
            print({k: v for k, v in r.items() if k not in ("vs", "fs", "asm")})
            break
