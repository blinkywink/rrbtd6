"""Dependency-free UnityFS (asset bundle) reader.

Decompresses a UnityFS bundle (LZ4/LZ4HC block compression) and lists or
extracts its directory nodes (usually one SerializedFile CAB-* plus .resS
resources). Read-only with respect to the source file.

Usage:
    python -I unityfs.py list    <bundle>
    python -I unityfs.py extract <bundle> <out_dir>
"""
import os
import struct
import sys


def lz4_block(src: bytes, usize: int) -> bytes:
    dst = bytearray()
    i, n = 0, len(src)
    while i < n:
        tok = src[i]
        i += 1
        lit = tok >> 4
        if lit == 15:
            while True:
                b = src[i]
                i += 1
                lit += b
                if b != 255:
                    break
        dst += src[i:i + lit]
        i += lit
        if i >= n:
            break
        off = src[i] | (src[i + 1] << 8)
        i += 2
        ml = tok & 15
        if ml == 15:
            while True:
                b = src[i]
                i += 1
                ml += b
                if b != 255:
                    break
        ml += 4
        start = len(dst) - off
        if off >= ml:
            dst += dst[start:start + ml]
        else:
            for k in range(ml):
                dst.append(dst[start + k])
    assert len(dst) == usize, (len(dst), usize)
    return bytes(dst)


def decompress(kind, data, usize):
    if kind == 0:
        return data
    if kind in (2, 3):
        return lz4_block(data, usize)
    raise NotImplementedError(f"compression {kind} (LZMA) not implemented")


def cstr(d, p):
    e = d.index(b"\0", p)
    return d[p:e].decode(), e + 1


def read_bundle(path):
    d = open(path, "rb").read()
    sig, p = cstr(d, 0)
    assert sig == "UnityFS", sig
    fmt = struct.unpack_from(">I", d, p)[0]
    p += 4
    _, p = cstr(d, p)
    engine, p = cstr(d, p)
    _, csz, usz, flags = struct.unpack_from(">qIII", d, p)
    p += 20
    if fmt >= 7:
        p = (p + 15) & ~15
    if flags & 0x80:  # blocks info at end
        info_c = d[len(d) - csz:]
    else:
        info_c = d[p:p + csz]
        p += csz
    info = decompress(flags & 0x3F, info_c, usz)
    if flags & 0x200:  # block info needs padding at start
        p = (p + 15) & ~15
    q = 16  # skip uncompressed data hash
    nblocks = struct.unpack_from(">I", info, q)[0]
    q += 4
    blocks = []
    for _ in range(nblocks):
        bu, bc, bf = struct.unpack_from(">IIH", info, q)
        q += 10
        blocks.append((bu, bc, bf))
    nnodes = struct.unpack_from(">I", info, q)[0]
    q += 4
    nodes = []
    for _ in range(nnodes):
        off, size, nf = struct.unpack_from(">qqI", info, q)
        q += 20
        name, q = cstr(info, q)
        nodes.append((name, off, size, nf))
    raw = bytearray()
    for bu, bc, bf in blocks:
        raw += decompress(bf & 0x3F, d[p:p + bc], bu)
        p += bc
    return engine, nodes, bytes(raw)


def main():
    cmd, path = sys.argv[1], sys.argv[2]
    engine, nodes, raw = read_bundle(path)
    print("engine", engine, "decompressed", len(raw))
    for name, off, size, nf in nodes:
        print(f"  {name}  off={off} size={size} flags={nf:#x}")
    if cmd == "extract":
        out = sys.argv[3]
        os.makedirs(out, exist_ok=True)
        for name, off, size, _ in nodes:
            with open(os.path.join(out, name.replace("/", "_")), "wb") as f:
                f.write(raw[off:off + size])


if __name__ == "__main__":
    main()
