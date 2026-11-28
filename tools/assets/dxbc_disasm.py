"""A D3D10/11 shader bytecode (DXBC, shader model 4 / 5) disassembler in plain Python.

disassemble(code) -> str: the same text as d3dcompiler_47's D3DDisassemble(code, 0), so dxbc_glsl.py
translates the game's shaders the same way on macOS and Linux (which have no d3dcompiler) as on
Windows. tools/tests/test_dxbc_disasm.py compares the two on every program in the game.

The container holds chunks: ISGN / OSGN (signatures, printed as comment tables), SHDR / SHEX (the
program: a version token, a length, then instructions), STAT (the instruction-slot count).
"""
import struct

# ---------------------------------------------------------------- tables

SYSVAL = {0: "NONE", 1: "POS", 2: "CLIPDST", 3: "CULLDST", 4: "RTINDEX", 5: "VPINDEX", 6: "VERTID",
          7: "PRIMID", 8: "INSTID", 9: "FFACE", 10: "SAMPLE", 11: "QUADEDGE", 12: "QUADINT",
          13: "TRIEDGE", 14: "TRIINT", 15: "LINEDET", 16: "LINEDEN", 64: "TARGET", 65: "DEPTH",
          66: "COVERAGE", 67: "DEPTHGE", 68: "DEPTHLE", 69: "STENCILREF", 70: "INNERCOV"}
COMPTYPE = {0: "unknown", 1: "uint", 2: "int", 3: "float"}

# Declaration system values (D3D10_SB_NAME).
SV_NAMES = {0: "undefined", 1: "position", 2: "clip_distance", 3: "cull_distance",
            4: "rendertarget_array_index", 5: "viewport_array_index", 6: "vertex_id", 7: "primitive_id",
            8: "instance_id", 9: "is_front_face", 10: "sampleIndex", 11: "finalQuadUeq0EdgeTessFactor",
            12: "finalQuadVeq0EdgeTessFactor", 13: "finalQuadUeq1EdgeTessFactor",
            14: "finalQuadVeq1EdgeTessFactor", 15: "finalQuadUInsideTessFactor",
            16: "finalQuadVInsideTessFactor", 17: "finalTriUeq0EdgeTessFactor",
            18: "finalTriVeq0EdgeTessFactor", 19: "finalTriWeq0EdgeTessFactor",
            20: "finalTriInsideTessFactor", 21: "finalLineDetailTessFactor", 22: "finalLineDensityTessFactor"}

INTERP = {0: "", 1: "constant", 2: "linear", 3: "linear centroid", 4: "linear noperspective",
          5: "linear noperspective centroid", 6: "linear sample", 7: "linear noperspective sample"}

RES_DIM = {1: "buffer", 2: "texture1d", 3: "texture2d", 4: "texture2dms", 5: "texture3d", 6: "texturecube",
           7: "texture1darray", 8: "texture2darray", 9: "texture2dmsarray", 10: "texturecubearray",
           11: "raw_buffer", 12: "structured_buffer"}
RET_TYPE = {1: "unorm", 2: "snorm", 3: "sint", 4: "uint", 5: "float", 6: "mixed", 7: "double", 8: "continued"}

# Operand register types: their printed prefix.
REG = {0: "r", 1: "v", 2: "o", 3: "x", 4: "l", 5: "d", 6: "s", 7: "t", 8: "cb", 9: "icb", 10: "label",
       11: "vPrim", 12: "oDepth", 13: "null", 14: "rasterizer", 15: "oMask", 16: "m", 17: "fb", 18: "ft",
       19: "fp", 20: "fi", 21: "fo", 22: "vOutputControlPointID", 23: "vForkInstanceID",
       24: "vJoinInstanceID", 25: "vicp", 26: "vocp", 27: "vpc", 28: "vDomain", 29: "this", 30: "u", 31: "g",
       32: "vThreadID", 33: "vThreadGroupID", 34: "vThreadIDInGroup", 35: "vCoverage",
       36: "vThreadIDInGroupFlattened", 37: "vGSInstanceID", 38: "oDepthGE", 39: "oDepthLE",
       40: "vCycleCounter", 41: "oStencilRef", 42: "vInnerCoverage"}

# Opcode -> (name, how its literals print: "f" float, "i" int, "u" uint, "x" bits, "?" guess).
F, I, U, X, G = "f", "i", "u", "x", "?"
OPS = {
    0: ("add", F), 1: ("and", X), 2: ("break", G), 3: ("breakc", I), 4: ("call", G), 5: ("callc", I),
    6: ("case", I), 7: ("continue", G), 8: ("continuec", I), 9: ("cut", G), 10: ("default", G),
    11: ("deriv_rtx", F), 12: ("deriv_rty", F), 13: ("discard", I), 14: ("div", F), 15: ("dp2", F),
    16: ("dp3", F), 17: ("dp4", F), 18: ("else", G), 19: ("emit", G), 20: ("emitThenCut", G),
    21: ("endif", G), 22: ("endloop", G), 23: ("endswitch", G), 24: ("eq", F), 25: ("exp", F),
    26: ("frc", F), 27: ("ftoi", F), 28: ("ftou", F), 29: ("ge", F), 30: ("iadd", I), 31: ("if", I),
    32: ("ieq", I), 33: ("ige", I), 34: ("ilt", I), 35: ("imad", I), 36: ("imax", I), 37: ("imin", I),
    38: ("imul", I), 39: ("ine", I), 40: ("ineg", I), 41: ("ishl", I), 42: ("ishr", I), 43: ("itof", I),
    44: ("label", G), 45: ("ld", I), 46: ("ld_ms", I), 47: ("log", F), 48: ("loop", G), 49: ("lt", F),
    50: ("mad", F), 51: ("min", F), 52: ("max", F), 53: ("customdata", G), 54: ("mov", G), 55: ("movc", G),
    56: ("mul", F), 57: ("ne", F), 58: ("nop", G), 59: ("not", X), 60: ("or", X), 61: ("resinfo", I),
    62: ("ret", G), 63: ("retc", I), 64: ("round_ne", F), 65: ("round_ni", F), 66: ("round_pi", F),
    67: ("round_z", F), 68: ("rsq", F), 69: ("sample", F), 70: ("sample_c", F), 71: ("sample_c_lz", F),
    72: ("sample_l", F), 73: ("sample_d", F), 74: ("sample_b", F), 75: ("sqrt", F), 76: ("switch", I),
    77: ("sincos", F), 78: ("udiv", U), 79: ("ult", U), 80: ("uge", U), 81: ("umul", U), 82: ("umad", U),
    83: ("umax", U), 84: ("umin", U), 85: ("ushr", U), 86: ("utof", U), 87: ("xor", X),
    107: ("lod", F), 108: ("gather4", F), 109: ("sample_pos", I), 110: ("sample_info", I),
    121: ("bufinfo", I), 122: ("deriv_rtx_coarse", F), 123: ("deriv_rtx_fine", F),
    124: ("deriv_rty_coarse", F), 125: ("deriv_rty_fine", F), 126: ("gather4_c", F), 127: ("gather4_po", F),
    128: ("gather4_po_c", F), 129: ("rcp", F), 130: ("f32tof16", F), 131: ("f16tof32", U),
    132: ("uaddc", U), 133: ("usubb", U), 134: ("countbits", U), 135: ("firstbit_hi", U),
    136: ("firstbit_lo", U), 137: ("firstbit_shi", I), 138: ("ubfe", U), 139: ("ibfe", I), 140: ("bfi", U),
    141: ("bfrev", U), 142: ("swapc", G), 163: ("ld_uav_typed", I), 164: ("store_uav_typed", G),
    165: ("ld_raw", I), 166: ("store_raw", G), 167: ("ld_structured", I), 168: ("store_structured", G),
    190: ("sync", G),
}
# Instructions whose operand is tested against zero (_z / _nz).
TESTED = {3, 5, 8, 13, 31, 63}
# Sampling and loading instructions that SM5 prints with their resource ("_indexable(texture2d)...").
RESOURCE_OPS = {45, 46, 61, 69, 70, 71, 72, 73, 74, 107, 108, 109, 110, 121, 126, 127, 128, 163, 165, 167}
DECLS = set(range(88, 107)) | set(range(143, 163)) | {113, 114, 115, 116, 117, 118, 119, 120}
INDENT_OPEN = {31, 48, 76}          # if, loop, switch
INDENT_CLOSE = {21, 22, 23}         # endif, endloop, endswitch
INDENT_MID = {18, 6, 10}            # else, case, default


# ---------------------------------------------------------------- numbers

def fmt_float(bits):
    """A float literal as D3DDisassemble prints it (%f)."""
    v = struct.unpack("<f", struct.pack("<I", bits))[0]
    if v != v:
        return "nan"
    if v in (float("inf"), float("-inf")):
        return "inf" if v > 0 else "-inf"
    return f"{v:f}"


def fmt_int(bits):
    """Small integers in decimal, the rest (hash constants, masks) in hex."""
    v = struct.unpack("<i", struct.pack("<I", bits))[0]
    return str(v) if -10000 < v < 10000 else f"0x{bits:08x}"


def fmt_literal(bits, kind):
    if kind == F:
        return fmt_float(bits)
    if kind == I:
        return fmt_int(bits)
    if kind in (U, X):
        return str(bits) if bits < 10000 else fmt_int(bits)
    # Untyped (mov, movc...): a float when the bits look like an ordinary float, else an integer.
    exp = (bits >> 23) & 0xFF
    if 0 < exp < 0xFF and 0x30 <= exp <= 0xC8:
        return fmt_float(bits)
    return fmt_int(bits)


# ---------------------------------------------------------------- container

def chunks(code):
    if code[:4] != b"DXBC":
        raise ValueError("not DXBC")
    count = struct.unpack_from("<I", code, 28)[0]
    out = {}
    for i in range(count):
        off = struct.unpack_from("<I", code, 32 + 4 * i)[0]
        tag = code[off:off + 4].decode("ascii", "replace")
        size = struct.unpack_from("<I", code, off + 4)[0]
        out.setdefault(tag, code[off + 8:off + 8 + size])
    return out


def signature(data, tag):
    """Elements of an ISGN / OSGN (/ ISG1 / OSG5 / OSG1) chunk."""
    count = struct.unpack_from("<I", data, 0)[0]
    stride = {"OSG5": 28, "ISG1": 32, "OSG1": 32}.get(tag, 24)
    out = []
    for i in range(count):
        at = 8 + i * stride
        if stride == 28:
            _stream, name_off, idx, sv, comp, reg, mask, used = struct.unpack_from("<IIIIIIBB", data, at)
        elif stride == 32:
            _stream, name_off, idx, sv, comp, reg, mask, used = struct.unpack_from("<IIIIIIBB", data, at)
        else:
            name_off, idx, sv, comp, reg, mask, used = struct.unpack_from("<IIIIIBB", data, at)
        end = data.index(b"\0", name_off)
        out.append((data[name_off:end].decode("ascii", "replace"), idx, sv, comp, reg, mask, used))
    return out


def mask_text(m):
    return "".join("xyzw"[i] if m & (1 << i) else " " for i in range(4))


def signature_table(title, elements):
    lines = ["//", f"// {title}:", "//"]
    if not elements:
        lines.append(f"// no {title.split()[0]}")
        return lines + ["//"]
    lines.append("// Name                 Index   Mask Register SysValue  Format   Used")
    lines.append("// -------------------- ----- ------ -------- -------- ------- ------")
    output = title.startswith("Output")
    for name, idx, sv, comp, reg, mask, used in elements:
        regt = str(reg) if reg != 0xFFFFFFFF else "N/A"
        if output:
            # An output's byte marks the components it never writes.
            used = mask & ~used
            # Pixel shader outputs carry no system value; their names say what they are.
            up = name.upper()
            if sv == 0 and up in ("SV_TARGET", "SV_DEPTH", "SV_COVERAGE", "SV_DEPTHGREATEREQUAL", "SV_DEPTHLESSEQUAL", "SV_STENCILREF"):
                sv = {"SV_TARGET": 64, "SV_DEPTH": 65, "SV_COVERAGE": 66, "SV_DEPTHGREATEREQUAL": 67,
                      "SV_DEPTHLESSEQUAL": 68, "SV_STENCILREF": 69}[up]
        lines.append(f"// {name:<20} {idx:>5}   {mask_text(mask):<4} {regt:>8} {SYSVAL.get(sv, str(sv)):>8} "
                     f"{COMPTYPE.get(comp, str(comp)):>7}   {mask_text(used):<4}")
    return lines + ["//"]


# ---------------------------------------------------------------- operands

class Reader:
    def __init__(self, toks, pos):
        self.toks, self.pos = toks, pos

    def take(self):
        v = self.toks[self.pos]
        self.pos += 1
        return v


def operand(r, kind, bare=False):
    """One operand's text, reading its tokens from r (bare: no components, as declarations of
    constant buffers, samplers and resources print them)."""
    tok = r.take()
    ncomp = tok & 3
    mode = (tok >> 2) & 3
    rtype = (tok >> 12) & 0xFF
    dims = (tok >> 20) & 3
    reps = [(tok >> 22) & 7, (tok >> 25) & 7, (tok >> 28) & 7]
    neg = absolute = False
    precise = None
    if tok & 0x80000000:
        ext = r.take()
        if ext & 0x3F == 1:
            mod = (ext >> 6) & 0xFF
            neg = mod in (1, 3)
            absolute = mod in (2, 3)
    if rtype in (4, 5):  # immediate32 / immediate64
        n = 1 if ncomp == 1 else 4
        vals = [r.take() for _ in range(n)]
        # Untyped instructions (mov, movc...) print their values without spaces, typed ones with.
        text = "l(" + ("," if kind == G else ", ").join(fmt_literal(v, kind) for v in vals) + ")"
        return wrap(text, neg, absolute)
    idx = []
    for d in range(dims):
        rep = reps[d]
        if rep == 0:
            idx.append(str(r.take()))
        elif rep == 1:
            hi, lo = r.take(), r.take()
            idx.append(str((hi << 32) | lo))
        elif rep == 2:
            idx.append(operand(r, I))
        elif rep == 3:
            base = r.take()
            idx.append(f"{operand(r, I)} + {base}")
        else:
            raise ValueError(f"index representation {rep}")
    name = REG.get(rtype, f"?{rtype}")
    if rtype in (11, 12, 13, 15, 22, 23, 24, 28, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42) and not idx:
        text = name
    elif rtype in (8, 9, 1, 2, 3, 25, 26, 27, 30, 31) and len(idx) == 2:
        text = f"{name}{idx[0]}[{idx[1]}]" if rtype != 9 else f"{name}[{idx[0]}]"
    elif rtype == 9:
        text = f"icb[{idx[0]}]" if idx else "icb"
    elif idx:
        first = idx[0]
        text = f"{name}{first}" if first.isdigit() else f"{name}[{first}]"
        if len(idx) > 1:
            text += "".join(f"[{i}]" for i in idx[1:])
    else:
        text = name
    if ncomp == 2 and not bare:
        if mode == 0:
            m = (tok >> 4) & 0xF
            if m and m != 0xF or True:
                text += "." + "".join("xyzw"[i] for i in range(4) if m & (1 << i)) if m else ""
        elif mode == 1:
            sw = (tok >> 4) & 0xFF
            text += "." + "".join("xyzw"[(sw >> (2 * i)) & 3] for i in range(4))
        elif mode == 2:
            text += "." + "xyzw"[(tok >> 4) & 3]
    return wrap(text, neg, absolute)


def wrap(text, neg, absolute):
    if absolute:
        text = f"|{text}|"
    if neg:
        text = f"-{text}"
    return text


# ---------------------------------------------------------------- instructions

def resource_suffix(exts):
    """SM5 extended opcode tokens: sample offsets, resource dimension and return types."""
    out = ""
    for e in exts:
        t = e & 0x3F
        if t == 1:  # sample controls: immediate offsets
            def s4(v):
                v &= 0xF
                return v - 16 if v & 8 else v
            out += f"_aoffimmi({s4(e >> 9)},{s4(e >> 13)},{s4(e >> 17)})"
        elif t == 2:
            dim = (e >> 6) & 0x1F
            out += f"_indexable({RES_DIM.get(dim, dim)}"
            stride = (e >> 11) & 0xFFF
            out += f", stride={stride})" if dim == 12 else ")"
        elif t == 3:
            rt = [RET_TYPE.get((e >> (6 + 4 * i)) & 0xF, "?") for i in range(4)]
            out += "(" + ",".join(rt) + ")"
    return out


def declaration(op, tok, r, end):
    ctl = (tok >> 11) & 0x1FFF
    if op == 106:  # dcl_globalFlags
        names = [(1, "refactoringAllowed"), (2, "enableDoublePrecisionFloatOps"),
                 (4, "forceEarlyDepthStencil"), (8, "enableRawAndStructuredBuffers"),
                 (16, "skipOptimization"), (32, "enableMinimumPrecision"),
                 (64, "enable11_1DoubleExtensions"), (128, "enable11_1ShaderExtensions")]
        return "dcl_globalFlags " + " | ".join(n for b, n in names if ctl & b)
    if op == 89:  # dcl_constantbuffer
        o = operand(r, I, bare=True)
        cb = o.replace("cb", "CB", 1)
        return f"dcl_constantbuffer {cb}, {'dynamicIndexed' if ctl & 1 else 'immediateIndexed'}"
    if op == 90:  # dcl_sampler
        mode = {0: "mode_default", 1: "mode_comparison", 2: "mode_mono"}.get(ctl & 0xF, "mode_default")
        return f"dcl_sampler {operand(r, I, bare=True)}, {mode}"
    if op == 88:  # dcl_resource
        dim = ctl & 0x1F
        o = operand(r, I, bare=True)
        ret = r.take()
        rt = ",".join(RET_TYPE.get((ret >> (4 * i)) & 0xF, "?") for i in range(4))
        samples = (tok >> 16) & 0x7F
        tail = f", {samples}" if dim in (4, 9) and samples else ""
        return f"dcl_resource_{RES_DIM.get(dim, dim)} ({rt}) {o}{tail}"
    if op == 161:  # dcl_resource_raw
        return f"dcl_resource_raw {operand(r, I, bare=True)}"
    if op == 162:  # dcl_resource_structured
        o = operand(r, I, bare=True)
        return f"dcl_resource_structured {o}, {r.take()}"
    if op == 104:  # dcl_temps
        return f"dcl_temps {r.take()}"
    if op == 105:  # dcl_indexableTemp
        reg, size, comps = r.take(), r.take(), r.take()
        return f"dcl_indexableTemp x{reg}[{size}], {comps}"
    if op in (95, 98, 101):  # dcl_input, dcl_input_ps, dcl_output
        name = {95: "dcl_input", 98: "dcl_input_ps", 101: "dcl_output"}[op]
        interp = INTERP.get(ctl & 0xF, "") if op == 98 else ""
        return f"{name} {interp + ' ' if interp else ''}{operand(r, I)}"
    if op in (96, 97, 99, 100, 102, 103):  # *_sgv / *_siv
        name = {96: "dcl_input_sgv", 97: "dcl_input_siv", 99: "dcl_input_ps_sgv", 100: "dcl_input_ps_siv",
                102: "dcl_output_sgv", 103: "dcl_output_siv"}[op]
        interp = INTERP.get(ctl & 0xF, "") if op in (99, 100) else ""
        o = operand(r, I)
        sv = SV_NAMES.get(r.take(), "?")
        return f"{name} {interp + ' ' if interp else ''}{o}, {sv}"
    raise ValueError(f"declaration opcode {op}")


def immediate_constant_buffer(toks, pos, length):
    vals = toks[pos:pos + length]
    rows = []
    for i in range(0, len(vals), 4):
        row = vals[i:i + 4]
        rows.append("{ " + ", ".join(fmt_literal(v, G) for v in row) + "}")
    return "dcl_immediateConstantBuffer { " + ",\n                              ".join(rows) + " }"


def program(data):
    toks = struct.unpack(f"<{len(data) // 4}I", data[:len(data) // 4 * 4])
    version, length = toks[0], toks[1]
    kind = {0: "ps", 1: "vs", 2: "gs", 3: "hs", 4: "ds", 5: "cs"}.get(version >> 16, "?")
    lines = [f"{kind}_{(version >> 4) & 0xF}_{version & 0xF}"]
    pos, depth, slots = 2, 0, 0
    while pos < min(length, len(toks)):
        tok = toks[pos]
        op = tok & 0x7FF
        if op == 53:  # customdata: class in bits 11+, length in the next token
            size = toks[pos + 1]
            cls = tok >> 11
            if cls == 3:
                lines.append(immediate_constant_buffer(toks, pos + 2, size - 2))
            pos += size
            continue
        size = (tok >> 24) & 0x7F
        r = Reader(toks, pos + 1)
        exts = []
        ext = bool(tok & 0x80000000)
        while ext:
            e = r.take()
            exts.append(e)
            ext = bool(e & 0x80000000)
        end = pos + size
        if op in DECLS or op == 106:
            text = declaration(op, tok, r, end)
        else:
            name, litkind = OPS.get(op, (f"op{op}", G))
            if op in RESOURCE_OPS:
                name += resource_suffix(exts)
            if op in TESTED:
                name += "_nz" if tok & (1 << 18) else "_z"
            if tok & (1 << 13):
                name += "_sat"
            if op == 61:  # resinfo return type
                name += {0: "", 1: "_rcpFloat", 2: "_uint"}.get((tok >> 11) & 3, "")
            ops = []
            while r.pos < end:
                ops.append(operand(r, litkind))
            if op in INDENT_CLOSE or op in INDENT_MID:
                depth = max(0, depth - 1)
            text = "  " * depth + name + " " + ", ".join(ops)
            if op in INDENT_OPEN or op in INDENT_MID:
                depth += 1
            slots += 1
        lines.append(text)
        pos = end if size else pos + 1
    return lines


def disassemble(code):
    ch = chunks(bytes(code))
    lines = ["//", "// Generated by Microsoft (R) D3D Shader Disassembler", "//"]
    for tag, title in (("ISGN", "Input signature"), ("ISG1", "Input signature"),
                       ("OSGN", "Output signature"), ("OSG5", "Output signature"), ("OSG1", "Output signature")):
        if tag in ch:
            lines += signature_table(title, signature(ch[tag], tag))
    prog = ch.get("SHEX") or ch.get("SHDR")
    if prog is None:
        raise ValueError("no program chunk")
    lines += program(prog)
    count = 0
    if "STAT" in ch:
        count = struct.unpack_from("<I", ch["STAT"], 0)[0]
    lines.append(f"// Approximately {count} instruction slots used")
    return "\n".join(lines) + "\n"
