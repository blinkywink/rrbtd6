"""Our app icon: a white Comic Sans "R" on a rounded square of the website's blue (#2f80f5).

    python -I tools/assets/make_icon.py

Writes renderer/assets/icon/: icon.ico (the EXE's icon, all sizes), icon.png (256 px),
icon{16,32,64}.rgba (the window icon) and icon.res, the .ico compiled by the Windows SDK's
rc.exe, which renderer/build.rs links into BTD6.exe (and launcher/build.rs into the launcher).
Comic Sans comes from the system's fonts (Windows: comicbd.ttf).
"""
import glob
import os
import subprocess

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
OUT = os.path.join(ROOT, "renderer", "assets", "icon")
BLUE = (0x2F, 0x80, 0xF5, 255)
SIZES = [16, 24, 32, 48, 64, 128, 256]


def font_path():
    windir = os.environ.get("WINDIR", r"C:\Windows")
    for p in [os.path.join(windir, "Fonts", "comicbd.ttf"), os.path.join(windir, "Fonts", "comic.ttf"),
              "/System/Library/Fonts/Supplemental/Comic Sans MS Bold.ttf",
              "/usr/share/fonts/truetype/msttcorefonts/Comic_Sans_MS_Bold.ttf"]:
        if os.path.exists(p):
            return p
    raise SystemExit("Comic Sans not found")


def icon(size, font):
    # Drawn at 4x and scaled down for smooth edges.
    s = size * 4
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle([0, 0, s - 1, s - 1], radius=int(s * 0.18), fill=BLUE)
    f = ImageFont.truetype(font, int(s * 0.78))
    l, t, r, b = d.textbbox((0, 0), "R", font=f)
    d.text(((s - (r - l)) / 2 - l, (s - (b - t)) / 2 - t), "R", font=f, fill=(255, 255, 255, 255))
    return img.resize((size, size), Image.LANCZOS)


def main():
    os.makedirs(OUT, exist_ok=True)
    font = font_path()
    imgs = {n: icon(n, font) for n in SIZES}
    imgs[256].save(os.path.join(OUT, "icon.png"))
    imgs[256].save(os.path.join(OUT, "icon.ico"), sizes=[(n, n) for n in SIZES])
    for n in (16, 32, 64):
        with open(os.path.join(OUT, f"icon{n}.rgba"), "wb") as fh:
            fh.write(imgs[n].tobytes())
    rc = os.path.join(OUT, "icon.rc")
    with open(rc, "w") as fh:
        fh.write('1 ICON "icon.ico"\n')
    exe = sorted(glob.glob(r"C:\Program Files (x86)\Windows Kits\10\bin\*\x64\rc.exe"))
    if exe:
        subprocess.run([exe[-1], "/nologo", "/fo", "icon.res", "icon.rc"], cwd=OUT, check=True)
    else:
        print("rc.exe not found: icon.res not rebuilt")
    os.remove(rc)
    print("wrote", OUT)


if __name__ == "__main__":
    main()
