"""Exercise two real clients over relays, including incompatible invite rejection.

Build first: cargo build --profile game -p rrbtd6-coop --bin coop-probe
Pass the built coop-probe executable as the first argument. No game assets required.
"""
import argparse
import json
import subprocess
import tempfile
import time
import shutil
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("--mods", action="store_true", help="Transfer a host-selected data mod before room readiness")
    parser.add_argument("--match", action="store_true", help="Verify match loading and 120 ordered command batches")
    parser.add_argument("--desync", action="store_true", help="Verify different initial simulation checksums stop the match")
    parser.add_argument("--players", type=int, choices=(2, 3, 4), default=2)
    args = parser.parse_args()
    exe = str(args.executable.resolve(strict=True))
    flags = getattr(subprocess, "CREATE_NO_WINDOW", 0)
    with tempfile.TemporaryDirectory(prefix="rrbtd6-coop-test-") as directory:
        root = Path(directory)
        mode = ["--match"] if args.match or args.desync else []
        if args.desync:
            mode.append("--desync")
        mode.append(f"--players={args.players}")
        if args.mods:
            mode.append("--mods")
        host = subprocess.Popen([exe, "host", directory, "--relay-only", *mode],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                creationflags=flags)
        try:
            deadline = time.monotonic() + 40
            while not (root / "invite.txt").exists():
                if host.poll() is not None:
                    raise RuntimeError("Host exited before creating its invite")
                if time.monotonic() > deadline:
                    raise TimeoutError("Relay did not become available")
                time.sleep(0.1)
            for options in (["--mismatch", "--expect-reject"],
                            ["--wrong-token", "--expect-reject"]):
                subprocess.run([exe, "join", directory, "--relay-only", *mode, *options],
                               check=True, timeout=45, creationflags=flags)
            guests = []
            for slot in range(1, args.players):
                guest = root / f"guest-{slot}"
                guest.mkdir()
                shutil.copyfile(root / "invite.txt", guest / "invite.txt")
                guests.append(guest)
            def join(guest):
                subprocess.run([exe, "join", str(guest), "--relay-only", *mode],
                               check=True, timeout=45, creationflags=flags)
            with ThreadPoolExecutor(max_workers=args.players - 1) as pool:
                list(pool.map(join, guests))
            if host.wait(timeout=15) != 0:
                raise RuntimeError("Host verification failed")
            if args.desync:
                assert json.loads((root / "host-desync.json").read_text())["stopped"] is True
                for guest in guests:
                    assert json.loads((guest / "join-desync.json").read_text())["stopped"] is True
                print("Relay match: different initial checksums stopped both clients before simulation")
                return
            if args.match:
                host_result = json.loads((root / "host-match.json").read_text())
                for guest in guests:
                    guest_result = json.loads((guest / "join-match.json").read_text())
                    assert host_result == guest_result, "Client command histories differ"
                assert host_result["frames"] == 120
                print(f"Relay match: {args.players} players, 120 ordered frames with matching command-history checksums")
                return
            for path in [root / "host-result.json", *(guest / "join-result.json" for guest in guests)]:
                result = json.loads(path.read_text())
                assert all(value is True for key, value in result.items() if key != "rttMs"), result
                print(path.name, result)
        finally:
            if host.poll() is None:
                host.terminate()
                host.wait(timeout=5)


if __name__ == "__main__":
    main()
