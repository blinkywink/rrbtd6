"""Run independent exporter processes side by side, as many as free memory allows.

together([(argv, env), ...]): each job's output is printed whole when it ends (in job order), so
lines from different jobs never interleave; the first failure is raised after all have ended.
With too little memory for two, they run one after another, as before.
"""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from model_batch import worker_count  # noqa: E402


def together(jobs, per_worker=2 << 30):
    if not jobs:
        return
    count = min(len(jobs), worker_count(per_worker=per_worker, max_workers=len(jobs)))

    def call(job):
        argv, env = job
        return subprocess.run(argv, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                              text=True, encoding="utf-8", errors="replace")

    with ThreadPoolExecutor(count) as pool:
        results = list(pool.map(call, jobs))
    for (argv, _), result in zip(jobs, results):
        print(result.stdout, end="", flush=True)
    for (argv, _), result in zip(jobs, results):
        if result.returncode:
            raise subprocess.CalledProcessError(result.returncode, argv)
