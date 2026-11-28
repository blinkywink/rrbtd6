"""Writes the home page's progress section from website/progress.json.

    python website/build_progress.py

Replaces everything between the <!-- progress:start --> and <!-- progress:end --> markers in
website/public/index.html. A line on what a scenario is and when the counts were updated, the
totals with one overall bar, the rows that don't match yet, the sections that match completely as
one line each, then a block per remaining section (Towers, Heroes, Monkey Knowledge, ...) with a row
and bar per item (recorded
scenarios that match the real game frame by frame; website/count_progress.py fills them), and a bar
per area (a rough estimate out of ten of the whole area, including what no recording covers yet).
"""
import datetime
import html
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
PAGE = os.path.join(HERE, "public", "index.html")
START, END = "<!-- progress:start -->", "<!-- progress:end -->"


def row(name, done, total, count, state=""):
    bar = f'<span class="pbar"><i style="--p:{100 * done / total:.0f}%"></i></span>' if total else '<span class="pbar"></span>'
    return (f'      <li class="prow {state}"><span class="pname">{html.escape(name)}</span>{bar}'
            f'<span class="pcount">{count}</span></li>')


def build(data):
    sections = data["sections"]
    rows_all = [r for _, rows in sections for r in rows]
    done = sum(r[1] for r in rows_all)
    total = sum(r[2] for r in rows_all)
    towers = next(rows for title, rows in sections if title == "Towers")
    finished = sum(1 for t in towers if t[2] and t[1] == t[2])
    state = lambda d, t: "done" if t and d == t else ("none" if t == 0 else "")
    out = []
    out.append('  <section id="progress" class="block">')
    out.append("    <h2>Progress</h2>")
    day = datetime.date.fromisoformat(data["updated"])
    out.append('    <p class="intro">Each scenario is a short run recorded in the real game, then replayed in RustTD6 and '
               'compared frame by frame. These counts cover what has been recorded so far; the estimates at the bottom '
               f'cover the rest. <span class="updated">Updated {day.day} {day:%B %Y}.</span></p>')
    out.append('    <div class="progress-total">')
    out.append(f'      <div><span class="pnum">{done}</span><span class="of">of {total}</span><p>recorded scenarios match</p></div>')
    out.append(f'      <div><span class="pnum">{finished}</span><span class="of">of {len(towers)}</span><p>towers fully matched</p></div>')
    out.append("    </div>")
    out.append(f'    <div class="poverall"><span class="pbar"><i style="--p:{100 * done / total:.0f}%"></i></span>'
               f'<span class="ppct">{100 * done / total:.0f}%</span></div>')
    left = [(title, name, rd, rt) for title, rows in sections for name, rd, rt in rows if rt and rd < rt]
    if left:
        out.append('    <h3 class="sub">Still to match</h3>')
        out.append('    <ul class="todo">')
        for title, name, rd, rt in left:
            label = f"{name} ({title})" if name == "Heroes" else name
            out.append(f'      <li>{html.escape(label)} <span>{rt - rd} of {rt} left</span></li>')
        out.append("    </ul>")
    full = [(title, sum(r[2] for r in rows)) for title, rows in sections
            if rows and all(r[2] and r[1] == r[2] for r in rows)]
    if full:
        out.append('    <h3 class="sub">Fully matched</h3>')
        out.append('    <ul class="todo all">')
        for title, t in full:
            out.append(f'      <li>{html.escape(title)} <span>all {t} match</span></li>')
        out.append("    </ul>")
    for title, rows in sections:
        if title in dict(full):
            continue
        d = sum(r[1] for r in rows)
        t = sum(r[2] for r in rows)
        out.append(f'    <h3 class="sub">{html.escape(title)} <span>({d}/{t} match)</span></h3>' if t else
                   f'    <h3 class="sub">{html.escape(title)} <span>(not recorded yet)</span></h3>')
        out.append('    <ul class="prows">')
        for name, rd, rt in rows:
            out.append(row(name, rd, rt, f"{rd}/{rt}" if rt else "not recorded yet", state(rd, rt)))
        out.append("    </ul>")
    out.append('    <h3 class="sub">The whole game <span>(rough estimates out of 10)</span></h3>')
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
