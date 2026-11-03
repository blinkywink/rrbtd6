"""Summarise an effect capture (.fx): SimRecorder's of the real game (tools/mod/record.sh with a
scenario's "fx": true, "fx_particles": true) or ours (RRBTD6_FXDUMP=<file> with RRBTD6_SPEED=1, so
the effects advance in small steps). For each system whose path contains <filter>, at each effect
age (seconds after the first frame that drew one): the particle count, quad edge lengths and
colours (q lines), or the mesh bounds (m lines).
  python tools/mod/fx_summary.py research/dumps/recordings/dart_paragon_fxp.fx ParagonFXDuring 0.5,1.2
"""
import collections
import math
import sys

path, sys_filter = sys.argv[1], sys.argv[2]
ages = [float(a) for a in sys.argv[3].split(",")]
frames = []  # (time, {system: [lines]})
cur = None
sysname = None
for line in open(path, errors="replace"):
    p = line.split()
    if len(p) < 2:
        continue
    if p[0] == "U":
        cur = (float(p[3]), collections.OrderedDict())
        frames.append(cur)
        sysname = None
    elif p[0] == "S" and cur is not None:
        sysname = p[1]
        if sys_filter in sysname:
            cur[1].setdefault(sysname, [])
    elif p[0] in ("q", "m") and sysname and sys_filter in sysname:
        cur[1][sysname].append(p)
spawn = next((t for t, d in frames if any(d.values())), None)
if spawn is None:
    sys.exit(f"no {sys_filter} particles in {path}")
for age in ages:
    t, d = min(frames, key=lambda f: abs(f[0] - spawn - age))
    print(f"== age {age} (frame at {t - spawn:.3f})")
    for name, qs in d.items():
        if not qs:
            continue
        short = name.split("/")[-1]
        if qs[0][0] == "m":
            x = qs[0]
            lo, hi = list(map(float, x[3:6])), list(map(float, x[6:9]))
            print(f"  {short:32} mesh verts {x[1]} size {[round(h - l, 1) for l, h in zip(lo, hi)]}")
            continue
        sizes, cols = [], []
        for q in qs:
            v = [list(map(float, q[1 + 3 * k:4 + 3 * k])) for k in range(4)]
            sizes.append((round(math.dist(v[0], v[1]), 1), round(math.dist(v[1], v[2]), 1)))
            cols.append(tuple(int(c) for c in q[17:21]))
        print(f"  {short:32} n {len(qs):3} sizes {sizes[:4]} cols {cols[:3]}")
