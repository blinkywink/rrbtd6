"""Exploratory walker for the sim_model blob (see research/03-sim-model-format.md).

Each phase is decoded with an element reader chosen from what has been
identified so far; unknown phases stop the walk and print a hexdump.

Usage: python -I probe.py <sim_model.bytes> [phase_spec...]
"""
import struct
import sys


class R:
    def __init__(self, d, p=0):
        self.d, self.p = d, p

    def i32(self):
        v = struct.unpack_from("<i", self.d, self.p)[0]
        self.p += 4
        return v

    def f32(self):
        v = struct.unpack_from("<f", self.d, self.p)[0]
        self.p += 4
        return v

    def u8(self):
        v = self.d[self.p]
        self.p += 1
        return v

    def s7(self):
        n = sh = 0
        while True:
            b = self.u8()
            n |= (b & 0x7F) << sh
            sh += 7
            if b < 0x80:
                break
        v = self.d[self.p:self.p + n].decode("utf-8", "replace")
        self.p += n
        return v

    def nstr(self):
        return None if self.u8() else self.s7()

    def hexdump(self, n=128):
        for o in range(self.p, self.p + n, 16):
            row = self.d[o:o + 16]
            print(f"{o:9d} {row.hex(' '):48s} {''.join(chr(b) if 32 <= b < 127 else '.' for b in row)}")


ELEM = {
    "i32": R.i32,
    "f32": R.f32,
    "nstr": R.nstr,
    "target": lambda r: (r.s7(), r.u8()),
}


def array_phase(r, elem):
    n = r.i32()
    out = []
    for _ in range(n):
        L = r.i32()
        if not 0 <= L <= 1_000_000:
            raise ValueError(f"bad length {L} at {r.p - 4}")
        out.append([elem(r) for _ in range(L)])
    return out


def lengths_phase(r):
    n = r.i32()
    return list(struct.unpack_from(f"<{n}i", r.d, r.p)), setattr(r, "p", r.p + 4 * n)


def main():
    d = open(sys.argv[1], "rb").read()
    r = R(d)
    total = r.i32()
    hdr = [r.i32() for _ in range(r.i32())]
    print("total objects", total, "header", hdr)
    spec = sys.argv[2:] or ["a:i32", "a:i32", "len", "len", "a:nstr", "a:target"]
    for s in spec:
        start = r.p
        if s == "len":
            v, _ = lengths_phase(r)
            print(f"[{start}] lengths n={len(v)} sum={sum(v)}")
        else:
            kind = s.split(":")[1]
            v = array_phase(r, ELEM[kind])
            sample = [a for a in v if a][:3]
            print(f"[{start}] array<{kind}> n={len(v)} sample={sample}")
    print("stopped at", r.p)
    r.hexdump(160)


if __name__ == "__main__":
    main()
