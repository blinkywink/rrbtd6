"""Writes the home page's progress section from website/progress.json.

    python website/build_progress.py

Replaces everything between the <!-- progress:start --> and <!-- progress:end --> markers in
website/public/index.html. A line on how matching works, one summary card (percentage, counts, date,
overall bar and the rows that don't match yet), and behind "Show all details" a bar per area of the
whole game (rough estimates out of ten) and a block per section (Towers, Heroes, Maps, ...) with a row
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
    out.append('  <section id="progress" class="block band">')
    out.append("    <h2>Progress</h2>")
    day = datetime.date.fromisoformat(data["updated"])
    pct = f"{100 * done / total:.0f}%"
    out.append('    <p class="intro">Runs recorded in the real game are replayed in RustTD6 and compared frame by frame.</p>')
    out.append('    <div class="psummary">')
    out.append(f'      <div class="psummary-top"><span class="ppct">{pct}</span><p><strong>{done} of {total}</strong> recorded runs match'
               f'<span>{finished} of {len(towers)} towers exact · updated {day.day} {day:%B %Y}</span></p></div>')
    out.append(f'      <span class="pbar"><i style="--p:{pct}"></i></span>')
    left = [(title, name, rd, rt) for title, rows in sections for name, rd, rt in rows if rt and rd < rt]
    if left:
        out.append('      <p class="todo-label">Still to match</p>')
        out.append('      <ul class="todo">')
        for title, name, rd, rt in left:
            label = f"{name} ({title})" if name == "Heroes" else name
            out.append(f'        <li>{html.escape(label)} <span>{rt - rd} left</span></li>')
        out.append("      </ul>")
    out.append("    </div>")
    out.append('    <details class="progress-more">')
    out.append('      <summary>Show all details<span class="plus" aria-hidden="true"></span></summary>')
    out.append('      <h3 class="sub">The whole game <span>(rough estimates out of 10)</span></h3>')
    out.append('      <ul class="prows">')
    for name, n in data["areas"]:
        out.append("  " + row(name, n, 10, f"{n}/10"))
    out.append("      </ul>")
    for title, rows in sections:
        d = sum(r[1] for r in rows)
        t = sum(r[2] for r in rows)
        out.append(f'      <h3 class="sub">{html.escape(title)} <span>({d}/{t} match)</span></h3>' if t else
                   f'      <h3 class="sub">{html.escape(title)} <span>(not recorded yet)</span></h3>')
        out.append('      <ul class="prows">')
        for name, rd, rt in rows:
            out.append("  " + row(name, rd, rt, f"{rd}/{rt}" if rt else "not recorded yet", state(rd, rt)))
        out.append("      </ul>")
    out.append("    </details>")
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
