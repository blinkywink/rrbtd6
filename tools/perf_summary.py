"""Summarise real-game performance runs (SimRecorder `perf` scenarios, <name>.perf).

    python tools/perf_summary.py local_extracted/frame-bench/real_game/perf_bad40.perf [...]

From the frame the bloons are sent (the first with more bloons than the one before by over 10, else
the start) to the end: average FPS, 1% low FPS (the mean of the slowest 1% frame times), median /
p95 / p99 / worst frame time, seconds, and peak private memory.
"""
import statistics
import sys


def summary(path):
    rows = []
    for line in open(path):
        if line.startswith("#"):
            continue
        f, ms, n, mb = line.split()
        rows.append((int(f), float(ms), int(n), int(mb)))
    start = 0
    for k in range(1, len(rows)):
        if rows[k][2] - rows[k - 1][2] > 10:
            start = k
            break
    ms = [r[1] for r in rows[start:] if r[1] > 0]
    s = sorted(ms)
    pick = lambda q: s[min(len(s) - 1, int(q * len(s)))]
    low = s[-max(1, len(s) // 100):]
    return {
        "frames": len(ms), "seconds": round(sum(ms) / 1000, 1), "avg_fps": round(1000 * len(ms) / sum(ms), 1),
        "1%_low_fps": round(1000 / statistics.mean(low), 1), "median_ms": round(statistics.median(ms), 2),
        "p95_ms": round(pick(0.95), 1), "p99_ms": round(pick(0.99), 1), "worst_ms": round(s[-1], 1),
        "peak_bloons": max(r[2] for r in rows), "peak_private_mb": max(r[3] for r in rows),
    }


if __name__ == "__main__":
    for p in sys.argv[1:]:
        print(p, summary(p))
