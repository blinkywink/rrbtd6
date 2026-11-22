"""Plot frame-bench CSVs (benchmark_frames.py output) as FPS over time, one figure per run plus a
comparison figure.

    python tools/plot_bench.py OUT_DIR LABEL=path/to/run.csv [LABEL=path/to/run.csv ...]

Each run gets OUT_DIR/<label>.png (FPS per frame and a 30-frame rolling average, with the bloon count
on a second axis) and OUT_DIR/compare.png overlays the rolling averages. Frames are counted from the
first measured frame (the spawn) so the runs line up.
"""
import csv
import os
import statistics
import sys

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402


def load(path):
    rows = list(csv.DictReader(open(path, newline="")))
    rows = [r for r in rows if int(r["frame"]) > 300]
    t, fps, bloons, ms = [], [], [], []
    clock = 0.0
    for r in rows:
        w = float(r["wall_ms"])
        clock += w / 1000.0
        t.append(clock)
        ms.append(w)
        fps.append(1000.0 / w if w > 0 else 0.0)
        bloons.append(int(r["bloons"]))
    return t, fps, bloons, ms


def rolling(v, n=30):
    out, acc = [], 0.0
    for i, x in enumerate(v):
        acc += x
        if i >= n:
            acc -= v[i - n]
        out.append(acc / min(i + 1, n))
    return out


def summary(ms):
    s = sorted(ms)
    pick = lambda q: s[min(len(s) - 1, int(q * len(s)))]
    return {"frames": len(ms), "seconds": sum(ms) / 1000.0, "avg_fps": 1000.0 * len(ms) / sum(ms),
            "median_ms": statistics.median(ms), "p95_ms": pick(0.95), "p99_ms": pick(0.99), "worst_ms": s[-1]}


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    runs = [a.split("=", 1) for a in sys.argv[2:]]
    data = {label: load(path) for label, path in runs}
    colors = ["#c0392b", "#2471a3", "#1e8449", "#7d3c98"]
    for k, (label, (t, fps, bloons, ms)) in enumerate(data.items()):
        st = summary(ms)
        fig, ax = plt.subplots(figsize=(11, 4.5))
        # Frame rate as the moving average of frame time (an average of per-frame FPS overweights fast frames).
        ax.plot(t, [1000.0 / x for x in rolling(ms)], color=colors[k % 4], lw=1.6, label="FPS (30-frame average)")
        ax.plot(t, fps, color=colors[k % 4], lw=0.4, alpha=0.25, label="FPS (each frame)")
        ax.set_ylim(0, max(200, min(600, max(fps) * 1.05)))
        ax.set_xlabel("seconds since the BADs were sent")
        ax.set_ylabel("frames per second")
        ax2 = ax.twinx()
        ax2.fill_between(t, bloons, color="0.5", alpha=0.15, lw=0)
        ax2.set_ylabel("bloons on screen", color="0.4")
        ax.set_title(f"{label}: 500 BADs vs 4 paragons  |  avg {st['avg_fps']:.0f} fps, median {st['median_ms']:.1f} ms, "
                     f"p99 {st['p99_ms']:.0f} ms, worst {st['worst_ms']:.0f} ms, clear {st['seconds']:.1f} s")
        ax.legend(loc="upper right")
        fig.tight_layout()
        fig.savefig(os.path.join(out, f"{label}.png"), dpi=110)
        plt.close(fig)
        print(label, {k2: round(v, 2) for k2, v in st.items()})
    fig, ax = plt.subplots(figsize=(11, 4.5))
    for k, (label, (t, fps, bloons, ms)) in enumerate(data.items()):
        ax.plot(t, [1000.0 / x for x in rolling(ms)], color=colors[k % 4], lw=1.6, label=f"{label} (avg {summary(ms)['avg_fps']:.0f} fps)")
    ax.set_xlabel("seconds since the BADs were sent")
    ax.set_ylabel("frames per second (30-frame average)")
    ax.set_title("500 BADs vs 4 paragons: frame rate until everything is popped")
    ax.legend(loc="upper right")
    fig.tight_layout()
    fig.savefig(os.path.join(out, "compare.png"), dpi=110)
    print("wrote", out)


if __name__ == "__main__":
    main()
