"""Print a dump_ui_prefab.py JSON tree compactly: name, rect, sprites, text.

Usage: python tools/assets/print_ui_tree.py <tree.json> [max depth]
"""
import json
import sys

tree = json.load(open(sys.argv[1]))
MAX = int(sys.argv[2]) if len(sys.argv) > 2 else 99


def fmt(n):
    r = n.get("rect")
    s = ""
    if r:
        s += f" a{r['anchor_min']}-{r['anchor_max']} p{r['pos']} s{r['size']} pv{r['pivot']}"
        if r["scale"] != [1, 1]:
            s += f" x{r['scale']}"
        if r["rot_z"]:
            s += f" r{r['rot_z']}"
    for c in n["comps"]:
        t = c["type"]
        if t == "Image":
            s += f" [Img {c.get('sprite')} t{c.get('m_Type')} c{c.get('color')}{'' if c['enabled'] else ' OFF'}]"
        elif t == "TextMeshProUGUI":
            s += f" [Txt {c.get('m_text')!r} {c.get('font')} {c.get('m_fontSize')} c{c.get('color')} mat={c.get('material')} al{c.get('m_HorizontalAlignment')}/{c.get('m_VerticalAlignment')}]"
        elif t in ("HorizontalLayoutGroup", "VerticalLayoutGroup", "GridLayoutGroup"):
            s += f" [{t[:4]} sp{c.get('m_Spacing')} pad{c.get('m_Padding')} al{c.get('m_ChildAlignment')} cell{c.get('m_CellSize')}]"
        elif t in ("RawImage",):
            s += f" [Raw {c.get('texture')}]"
        else:
            s += f" [{t}]"
    return s


def walk(n, d):
    if d > MAX:
        return
    print("  " * d + ("" if n["active"] else "(off) ") + n["name"] + fmt(n))
    for c in n["children"]:
        walk(c, d + 1)


walk(tree, 0)
