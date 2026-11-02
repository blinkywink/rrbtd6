"""Writes the home page's progress section from website/progress.json.

    python website/build_progress.py

Replaces everything between the <!-- progress:start --> and <!-- progress:end --> markers in
website/public/index.html. One square per recorded tower scenario (filled: it matches the real
game frame by frame), and ten squares per area (a rough estimate).
"""
import html
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
PAGE = os.path.join(HERE, "public", "index.html")
START, END = "<!-- progress:start -->", "<!-- progress:end -->"


def squares(done, total, cls="sq"):
    return "".join(f'<i class="{cls}{" on" if k < done else ""}"></i>' for k in range(total))


def build(data):
    towers = data["towers"]
    done = sum(t[1] for t in towers)
    total = sum(t[2] for t in towers)
    finished = sum(1 for t in towers if t[2] and t[1] == t[2])
    out = []
    out.append('  <section id="progress" class="block">')
    out.append("    <h2>Progress</h2>")
    out.append('    <p class="progress-lede">Every tower is checked against recordings of the real game: a recorder mod plays a scenario in BTD6, RustTD6 replays it, and a square fills in when every bloon and projectile matches frame by frame.</p>')
    out.append('    <div class="progress-total">')
    out.append(f'      <div><span class="pnum">{done}</span><span class="of">of {total}</span><p>recorded scenarios match</p></div>')
    out.append(f'      <div><span class="pnum">{finished}</span><span class="of">of {len(towers)}</span><p>towers fully matched</p></div>')
    out.append("    </div>")
    out.append('    <div class="towers-grid">')
    for name, d, t in towers:
        state = "done" if t and d == t else ("none" if t == 0 else "")
        count = f"{d}/{t}" if t else "not recorded yet"
        out.append(f'      <div class="tower-card {state}"><div class="tower-head"><strong>{html.escape(name)}</strong><span>{count}</span></div>'
                   f'<div class="squares">{squares(d, t) if t else squares(0, 10, "sq ghost")}</div></div>')
    out.append("    </div>")
    out.append('    <h3 class="progress-sub">Everything else <span>(rough estimates)</span></h3>')
    out.append('    <div class="areas">')
    for name, n in data["areas"]:
        out.append(f'      <div class="area"><strong>{html.escape(name)}</strong><div class="squares">{squares(n, 10, "sq big")}</div></div>')
    out.append("    </div>")
    out.append(f'    <p class="aside">Updated {html.escape(data["updated"])}. Towers are matched one at a time; the details live in <a href="https://github.com/blinkywink/rrbtd6/blob/main/docs/dev/TOWERS.md">TOWERS.md</a>.</p>')
    out.append("  </section>")
    return "\n".join(out)


def main():
    data = json.load(open(os.path.join(HERE, "progress.json"), encoding="utf-8"))
    page = open(PAGE, encoding="utf-8").read()
    a, b = page.index(START), page.index(END)
    page = page[:a + len(START)] + "\n" + build(data) + "\n  " + page[b:]
    with open(PAGE, "w", encoding="utf-8", newline="\n") as f:
        f.write(page)
    print("wrote", PAGE)


if __name__ == "__main__":
    main()
