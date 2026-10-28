"""Export the game's own window icon from the user's BloonsTD6.exe.

Usage:
    tools/.venv/Scripts/python.exe -I tools/assets/export_game_icon.py <game install> <out_dir>

Reads the exe's icon resources (RT_GROUP_ICON / RT_ICON), takes the largest image and writes
<out_dir>/icon16.rgba, icon32.rgba, icon64.rgba (raw RGBA, what miniquad's window icon takes) and
icon.png. Proprietary art: local use only, never committed or shipped.
"""
import io
import os
import struct
import sys

from PIL import Image

RT_ICON, RT_GROUP_ICON = 3, 14


def resources(d):
    pe = struct.unpack_from("<I", d, 0x3C)[0]
    nsec = struct.unpack_from("<H", d, pe + 6)[0]
    optsz = struct.unpack_from("<H", d, pe + 20)[0]
    opt = pe + 24
    rva = struct.unpack_from("<I", d, opt + 112 + 2 * 8)[0]  # data directory 2: resources
    secs = []
    for i in range(nsec):
        s = pe + 24 + optsz + 40 * i
        vs, va, rs, rp = struct.unpack_from("<IIII", d, s + 8)
        secs.append((va, max(vs, rs), rp))

    def off(r):
        for va, size, rp in secs:
            if va <= r < va + size:
                return r - va + rp
        raise ValueError(hex(r))

    base = off(rva)

    def entries(dir_off):
        named, ids = struct.unpack_from("<HH", d, dir_off + 12)
        for k in range(named + ids):
            name, target = struct.unpack_from("<II", d, dir_off + 16 + 8 * k)
            yield name, target

    out = {}
    for typ, t1 in entries(base):
        if typ not in (RT_ICON, RT_GROUP_ICON) or not t1 & 0x80000000:
            continue
        for ident, t2 in entries(base + (t1 & 0x7FFFFFFF)):
            for _lang, t3 in entries(base + (t2 & 0x7FFFFFFF)):
                data_rva, size = struct.unpack_from("<II", d, base + t3)
                out[(typ, ident)] = d[off(data_rva):off(data_rva) + size]
                break
    return out


def largest_icon(res):
    groups = [v for (t, _), v in res.items() if t == RT_GROUP_ICON]
    best = None
    for g in groups:
        count = struct.unpack_from("<H", g, 4)[0]
        for k in range(count):
            w, h, _c, _r, _p, bpp, _size, ident = struct.unpack_from("<BBBBHHIH", g, 6 + 14 * k)
            w, h = w or 256, h or 256
            if best is None or (w * h, bpp) > best[0]:
                best = ((w * h, bpp), ident, w, h, bpp)
    if best is None:
        raise SystemExit("no icon in the exe")
    _, ident, w, h, bpp = best
    blob = res[(RT_ICON, ident)]
    if blob[:8] == b"\x89PNG\r\n\x1a\n":
        return Image.open(io.BytesIO(blob)).convert("RGBA")
    # A DIB without its file header: wrap it in a one-image .ico for PIL.
    ico = struct.pack("<HHH", 0, 1, 1) + struct.pack("<BBBBHHII", w % 256, h % 256, 0, 0, 1, bpp, len(blob), 22) + blob
    return Image.open(io.BytesIO(ico)).convert("RGBA")


def main():
    game, out = sys.argv[1], sys.argv[2]
    data = open(os.path.join(game, "BloonsTD6.exe"), "rb").read()
    img = largest_icon(resources(data))
    os.makedirs(out, exist_ok=True)
    img.save(os.path.join(out, "icon.png"))
    for n in (16, 32, 64):
        with open(os.path.join(out, f"icon{n}.rgba"), "wb") as f:
            f.write(img.resize((n, n), Image.LANCZOS).tobytes())
    print(f"icon {img.size} -> {out}")


main()
