"""Compare the game's cash (F lines) with the clone's (RRBTD6_REC_CASH's `DBGAIM CASH rt cash`).

    python -I tools/rec/cashcmp.py <recording.rec> <clone output> [tolerance]

Both as deltas from their first frame, matched by roundTime (the F line's 3rd field). Prints the
first differing frames (at most 9) and a summary line.
"""
import re
import sys

rec, out = sys.argv[1], sys.argv[2]
tol = float(sys.argv[3]) if len(sys.argv) > 3 else 1e-5
game = [(int(l.split()[2]), float(l.split()[4])) for l in open(rec) if l.startswith('F ')]
clone = [(int(m.group(1)), float(m.group(2)))
         for m in re.finditer(r'DBGAIM CASH (\d+) ([-0-9.e]+)', open(out, errors='replace').read())]
g0, c0 = game[0][1], clone[0][1] if clone else 0.0
cd = {}
for rt, v in clone:
    cd.setdefault(rt, v - c0)
n = 0
for rt, v in game:
    if rt in cd and abs((v - g0) - cd[rt]) > tol:
        print('rt', rt, 'game', v - g0, 'clone', cd[rt])
        n += 1
        if n > 8:
            break
print('frames', len(game), 'clone', len(clone), 'diffs shown', n)
