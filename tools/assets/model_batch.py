"""Bound memory with fresh processes between groups; reuse Unity bundles inside a group."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from collections import deque
from concurrent.futures import ThreadPoolExecutor, as_completed
import threading
sys.path.insert(0, str(Path(__file__).resolve().parent))
from export_animated_batch import complete


def chunks(jobs, size=16):
    pending = sorted((job for job in jobs if not complete(job[0])), key=lambda j: j[1])
    for i in range(0, len(pending), max(1, min(32, size))):
        yield pending[i:i + max(1, min(32, size))]


def available_memory():
    try:
        if os.name == "nt":
            import ctypes
            class Memory(ctypes.Structure):
                _fields_ = [("length", ctypes.c_uint32), ("load", ctypes.c_uint32)] + [
                    (name, ctypes.c_uint64) for name in ("total", "available", "page", "free_page", "virtual", "free_virtual", "extended")]
            memory = Memory();memory.length = ctypes.sizeof(memory)
            if ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(memory)):
                # Free RAM can exceed what Windows can still commit (a small page file):
                # workers then fail with MemoryError although RAM looks available.
                return min(memory.available, memory.free_page)
        elif sys.platform == "darwin":
            # macOS keeps most idle memory as inactive / purgeable cache, which it hands back on
            # demand, so free pages alone read near zero (and sysconf has no SC_AVPHYS_PAGES).
            import re
            import subprocess
            out = subprocess.run(["vm_stat"], capture_output=True, text=True, check=True).stdout
            page = int(re.search(r"page size of (\d+) bytes", out).group(1))
            pages = lambda name: int(re.search(rf"{name}:\s+(\d+)", out).group(1)) if re.search(rf"{name}:\s+(\d+)", out) else 0
            return page * sum(pages(n) for n in ("Pages free", "Pages inactive", "Pages speculative", "Pages purgeable"))
        else:
            # Linux: the kernel's estimate of memory available without swapping, page cache included.
            with open("/proc/meminfo", encoding="ascii") as info:
                for line in info:
                    if line.startswith("MemAvailable:"):
                        return int(line.split()[1]) * 1024
            return os.sysconf("SC_AVPHYS_PAGES") * os.sysconf("SC_PAGE_SIZE")
    except Exception:
        # Unknown: callers then use one worker, which is slow but always fits.
        pass
    return 0


def cores():
    """Physical cores: half the logical ones where they come in hyperthread pairs (most Windows and
    Linux PCs); Apple silicon has none, so every core counts there."""
    logical = os.cpu_count() or 1
    return logical if sys.platform == "darwin" else max(1, logical // 2)


def worker_count(cpu=None, free=None, per_worker=4 << 30, max_workers=4):
    """One single-threaded exporter per physical core, keeping 2 GiB free; callers may use a
    measured budget for a known bundle family. (cpu: a logical count, as on a hyperthreaded PC.)"""
    physical = max(1, cpu // 2) if cpu is not None else cores()
    free = free if free is not None else available_memory()
    return max(1, min(max_workers, physical, max(1, (free - (2 << 30)) // per_worker)))


def run(game, jobs, size=16, workers=None, *, progress_offset=0, progress_total=None, keep_floats=False, retries=2):
    unique = {}
    for job in jobs:
        key = str(Path(job[0]).resolve())
        if key in unique and unique[key] != job:
            raise ValueError("Conflicting model outputs: " + key)
        unique[key] = job
    groups = list(chunks(unique.values(), size))
    total = sum(map(len, groups))
    if not total:
        return
    pending = [job for group in groups for job in group]
    generated = [job for job in pending if job[1] == "generated_assets_all"]
    original = [job for job in pending if job[1] != "generated_assets_all"]
    if original and generated:
        # Finish the high-memory phase first. A single original asset must not
        # reduce concurrency for every generated model in the whole game.
        overall = progress_total if progress_total is not None else total
        run(game, original, size, workers, progress_offset=progress_offset, progress_total=overall, keep_floats=keep_floats)
        run(game, generated, size, workers, progress_offset=progress_offset + len(original), progress_total=overall, keep_floats=keep_floats)
        return
    requested = workers if workers is not None else int(os.environ.get("RRBTD6_SETUP_WORKERS", "12"))
    # Measured 32 mixed generated models at 1.12 GiB peak commit and 60 asset_references models
    # at 0.86 GiB (its files are mapped, not loaded). Keep ~2x headroom; a batch that still runs
    # out of memory is retried with one worker below.
    capacity = worker_count(per_worker=2 << 30, max_workers=12)
    count = min(len(groups), capacity, max(1, requested))
    print(f"Model preparation: {count} parallel worker(s), {total} models (memory bounded)", flush=True)
    completed = [0] * len(groups)
    lock, stopped = threading.Lock(), threading.Event()

    def report(index, done, label):
        with lock:
            completed[index] = max(completed[index], min(len(groups[index]), done))
            print("@@RRBTD6_PROGRESS " + json.dumps({"done": progress_offset + sum(completed), "total": progress_total if progress_total is not None else total,
                  "label": label}), flush=True)

    # Workers stay up across groups (export_animated_batch --serve): importing UnityPy and reading
    # a big bundle's object table cost seconds a process, more than many groups' own work. Each is
    # replaced after RECYCLE groups, so what it keeps loaded stays bounded.
    RECYCLE = 6
    script = str(Path(__file__).with_name("export_animated_batch.py"))
    queue = deque(enumerate(groups))
    failures = []

    def start():
        cmd = [sys.executable, "-X", "utf8", "-I", script, str(game), "--serve"]
        if keep_floats:
            cmd.append("--keep-floats")
        # Each exporter already uses a core; avoid NumPy's nested thread pools.
        env = dict(os.environ, OPENBLAS_NUM_THREADS="1", OMP_NUM_THREADS="1", MKL_NUM_THREADS="1")
        return subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                text=True, encoding="utf-8", errors="replace", bufsize=1, env=env), cmd

    def worker():
        child, cmd, served, tail = None, None, 0, deque(maxlen=25)
        try:
            while not stopped.is_set():
                with lock:
                    if not queue:
                        break
                    index, group = queue.popleft()
                if child is None:
                    child, cmd = start()
                    served = 0
                fd, manifest = tempfile.mkstemp(prefix="rusttd6-model-jobs-", suffix=".json")
                try:
                    with os.fdopen(fd, "w", encoding="utf-8") as stream:
                        json.dump(group, stream)
                    child.stdin.write(manifest + "\n")
                    child.stdin.flush()
                    done = False
                    for line in child.stdout:
                        if line.startswith("@@RRBTD6_GROUP_DONE"):
                            done = True
                            break
                        if line.startswith("@@RRBTD6_PROGRESS "):
                            data = json.loads(line[len("@@RRBTD6_PROGRESS "):])
                            report(index, int(data["done"]), data.get("label", "Preparing models"))
                        else:
                            tail.append(line.rstrip())
                            if os.environ.get("RRBTD6_SETUP_VERBOSE") == "1" or line.startswith("model export failed: "):
                                with lock:
                                    print(line, end="", flush=True)
                    if not done:
                        code = child.wait()
                        child = None
                        stopped.set()
                        with lock:
                            print("\n".join(tail), flush=True)
                        failures.append(subprocess.CalledProcessError(code or 1, cmd))
                        return
                    report(index, len(group), "Models: preparing remaining batches")
                finally:
                    Path(manifest).unlink(missing_ok=True)
                served += 1
                if served >= RECYCLE:
                    child.stdin.close()
                    child.wait()
                    child = None
        except BaseException as error:  # reported by run() below, not lost in this thread
            stopped.set()
            failures.append(error)
        finally:
            if child is not None:
                try:
                    child.stdin.close()
                except OSError:
                    pass
                if child.wait() and not failures and not stopped.is_set():
                    failures.append(subprocess.CalledProcessError(child.returncode, cmd))

    threads = [threading.Thread(target=worker, daemon=True) for _ in range(count)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    failure = failures[0] if failures else None
    if failure is not None and not isinstance(failure, subprocess.CalledProcessError):
        raise failure
    if failure is not None:
        if retries <= 0:
            raise failure
        # Usually memory (other programs can use it up after the workers were sized). Finished
        # models are kept, so the rest continue one worker at a time in a fresh process.
        remaining = [job for job in unique.values() if not complete(job[0])]
        print(f"Model preparation stopped; retrying {len(remaining)} models with one worker", flush=True)
        run(game, remaining, size, 1, progress_offset=progress_offset + total - len(remaining),
            progress_total=progress_total if progress_total is not None else total, keep_floats=keep_floats, retries=retries - 1)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("game")
    parser.add_argument("jobs", type=Path)
    parser.add_argument("--chunk-size", type=int, default=16)
    parser.add_argument("--workers", type=int, default=None)
    parser.add_argument("--keep-floats", action="store_true", help="defer quantization to the normal finish pass")
    args = parser.parse_args()
    run(args.game, json.loads(args.jobs.read_text(encoding="utf-8")), args.chunk_size, args.workers, keep_floats=args.keep_floats)
