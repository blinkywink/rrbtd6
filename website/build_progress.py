"""Writes the home page's progress section from website/progress.json.

    python website/build_progress.py

Replaces everything between the <!-- progress:start --> and <!-- progress:end --> markers in
website/public/index.html. The totals with one overall bar, then one row and bar per tower (recorded scenarios that match the real game
frame by frame), with every scenario as a square behind "Show every recorded scenario", and a bar
per area (a rough estimate out of ten).
"""
import html
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
PAGE = os.path.join(HERE, "public", "index.html")
START, END = "<!-- progress:start -->", "<!-- progress:end -->"


def squares(done, total, cls="sq"):
    return "".join(f'<i class="{cls}{" on" if k < done else ""}"></i>' for k in range(total))


def row(name, done, total, count, state=""):
    bar = f'<span class="pbar"><i style="--p:{100 * done / total:.0f}%"></i></span>' if total else '<span class="pbar"></span>'
    return (f'      <li class="prow {state}"><span class="pname">{html.escape(name)}</span>{bar}'
            f'<span class="pcount">{count}</span></li>')


def build(data):
    towers = data["towers"]
    done = sum(t[1] for t in towers)
    total = sum(t[2] for t in towers)
    finished = sum(1 for t in towers if t[2] and t[1] == t[2])
    out = []
    out.append('  <section id="progress" class="block">')
    out.append("    <h2>Progress</h2>")
    out.append('    <div class="progress-total">')
    out.append(f'      <div><span class="pnum">{done}</span><span class="of">of {total}</span><p>recorded scenarios match</p></div>')
    out.append(f'      <div><span class="pnum">{finished}</span><span class="of">of {len(towers)}</span><p>towers fully matched</p></div>')
    out.append("    </div>")
    out.append(f'    <div class="poverall"><span class="pbar"><i style="--p:{100 * done / total:.0f}%"></i></span>'
               f'<span class="ppct">{100 * done / total:.0f}%</span></div>')
    state = lambda d, t: "done" if t and d == t else ("none" if t == 0 else "")
    out.append('    <ul class="prows">')
    for name, d, t in towers:
        out.append(row(name, d, t, f"{d}/{t}" if t else "not recorded", state(d, t)))
    out.append("    </ul>")
    out.append('    <details class="progress-more">')
    out.append('      <summary>Show every recorded scenario</summary>')
    out.append('      <div class="towers-grid">')
    for name, d, t in towers:
        count = f"{d}/{t}" if t else "not recorded yet"
        out.append(f'        <div class="tower-card {state(d, t)}"><div class="tower-head"><strong>{html.escape(name)}</strong><span>{count}</span></div>'
                   f'<div class="squares">{squares(d, t) if t else squares(0, 10, "sq ghost")}</div></div>')
    out.append("      </div>")
    out.append("    </details>")
    out.append('    <h3 class="sub">Everything else <span>(rough estimates)</span></h3>')
    out.append('    <ul class="prows">')
    for name, n in data["areas"]:
        out.append(row(name, n, 10, f"{n}/10"))
    out.append("    </ul>")
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
