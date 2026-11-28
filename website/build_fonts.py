"""Writes the site's web fonts (public/fonts/*.woff2) from the full TTFs in website/fonts-src/.

    python website/build_fonts.py

Each font is cut down to the characters the site can show (Latin, Latin-1, punctuation, arrows and a
few symbols) and compressed as WOFF2, which takes Inter from ~325 KB to a fraction of that. Inter
keeps its weight axis, since the pages use weights 400 to 800.
"""
import os

from fontTools import subset
from fontTools.ttLib import TTFont

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "fonts-src")
OUT = os.path.join(HERE, "public", "fonts")
FONTS = ["Inter.ttf", "LuckiestGuy-Regular.ttf", "ComicNeue-Bold.ttf"]
UNICODES = "U+0020-007E,U+00A0-00FF,U+0131,U+0152-0153,U+02C6,U+02DA,U+02DC,U+2010-2027,U+2030-205E,U+20AC,U+2122,U+2190-21FF,U+2212,U+2715,U+2713"


def main():
    for name in FONTS:
        font = TTFont(os.path.join(SRC, name))
        options = subset.Options()
        options.flavor = "woff2"
        options.layout_features = ["kern", "liga", "calt", "tnum", "case"]
        options.name_IDs = ["*"]
        options.notdef_outline = True
        sub = subset.Subsetter(options)
        sub.populate(unicodes=subset.parse_unicodes(UNICODES))
        sub.subset(font)
        out = os.path.join(OUT, os.path.splitext(name)[0] + ".woff2")
        font.flavor = "woff2"
        font.save(out)
        print(f"{name}: {os.path.getsize(os.path.join(SRC, name)) // 1024} KB -> {os.path.getsize(out) // 1024} KB")


if __name__ == "__main__":
    main()
