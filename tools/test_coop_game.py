"""Run 2–4 rendered game copies through relay-only co-op using local extracted assets.

Each player places and upgrades a Dart Monkey; the host starts a round and player
two sells their tower. Compare actual simulation state at shared frame 180.
Only test-created processes are stopped. Run from the repository root.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("--players", type=int, choices=(2, 3, 4), default=2)
    parser.add_argument("--reduced-textures", action="store_true", help="Reduce texture memory when running several clients on one GPU")
    args = parser.parse_args()
    exe = str(args.executable.resolve(strict=True))
    root = Path.cwd()
    settings = root / "local_extracted/settings.json"
    old_map = json.loads(settings.read_text()).get("map")
    old_settings = json.loads(settings.read_text())
    old_full = old_settings.get("advanced", {}).get("full_textures")
    if args.reduced_textures:
        old_settings.setdefault("advanced", {})["full_textures"] = False
        settings.write_text(json.dumps(old_settings, indent=2))
    out = root / f"local_extracted/coop-game-{args.players}-test"
    out.mkdir(exist_ok=True)
    results = [out / f"player-{i}.json" for i in range(args.players)]
    for path in [out / "invite.txt", out / "done", *results]:
        path.unlink(missing_ok=True)
    env = os.environ.copy()
    env.update(RRBTD6_FRONT="coop_lobby", RRBTD6_COOP_RELAY_ONLY="1",
               RRBTD6_COOP_PLAYTEST="1", RRBTD6_COOP_TEST_PLAYERS=str(args.players),
               RRBTD6_COOP_TEST_DONE=str(out / "done"), RRBTD6_COOP_PANEL="1", RRBTD6_TIMING="1", RRBTD6_FRAMES="100000")
    processes, logs = [], []
    startup = None
    if os.name == "nt":
        startup = subprocess.STARTUPINFO()
        startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow = 0

    def launch(slot):
        log = open(out / f"player-{slot}.log", "w")
        logs.append(log)
        local = env.copy()
        local["RRBTD6_COOP_RESULT_OUT"] = str(results[slot])
        local["RRBTD6_AUTOTEST"] = str(out / f"player-{slot}.png")
        if slot == 0:
            local["RRBTD6_COOP_HOST"] = "1"
            local["RRBTD6_COOP_INVITE_OUT"] = str(out / "invite.txt")
            local.pop("RRBTD6_COOP_JOIN", None)
        else:
            local.pop("RRBTD6_COOP_HOST", None)
            local.pop("RRBTD6_COOP_INVITE_OUT", None)
            local["RRBTD6_COOP_JOIN"] = (out / "invite.txt").read_text()
        processes.append(subprocess.Popen([exe], env=local, stdout=log, stderr=log,
                                          startupinfo=startup,
                                          creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0)))

    def wait_for(condition, seconds):
        deadline = time.monotonic() + seconds
        while not condition():
            if any(p.poll() is not None for p in processes):
                exited = [(i, p.returncode) for i, p in enumerate(processes) if p.poll() is not None]
                raise RuntimeError(f"Test clients exited early: {exited}; inspect {out}")
            if time.monotonic() > deadline:
                raise TimeoutError(f"Co-op test timed out; inspect {out}")
            time.sleep(0.1)

    try:
        launch(0)
        wait_for(lambda: (out / "invite.txt").exists(), 45)
        for slot in range(1, args.players):
            launch(slot)
            # Avoid overlapping each process's temporary startup decode buffers.
            wait_for(lambda: "load: loop" in (out / f"player-{slot}.log").read_text(errors="replace"), 60)
        wait_for(lambda: all(p.exists() for p in results), 120)
        states = [json.loads(p.read_text()) for p in results]
        assert all(s == states[0] for s in states), "Simulation states differ"
        state = states[0]
        assert state["frame"] == 180 and state["popped"] > 0, state
        assert len(state["towers"]) == args.players - 1, state
        assert all(t[2] == [1, 0, 0] for t in state["towers"]), state
        wallets = state["wallets"]
        assert len(wallets) == 4 and all(v >= 0 for v in wallets), state
        assert abs(sum(wallets) - state["cash"]) < 1e-8, state
        assert wallets[1] > wallets[0], "Sale refund must stay with player two"
        assert state["debts"] == [0, 0, 0, 0], state
        for slot in range(2, args.players):
            assert abs(wallets[slot] - wallets[0] - 100.0) < 1e-8, "Only the host pays the donation"
        (out / "done").touch()
        for process in processes:
            assert process.wait(timeout=20) == 0
        print(f"{args.players} rendered clients matched after placement, upgrades, combat and sale.")
        print({k: state[k] for k in ["frame", "cash", "wallets", "popped", "round", "towers", "owners"]})
    finally:
        for process in processes:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)
        for log in logs:
            log.close()
        current = json.loads(settings.read_text())
        if args.reduced_textures:
            if old_full is None:
                current.setdefault("advanced", {}).pop("full_textures", None)
            else:
                current.setdefault("advanced", {})["full_textures"] = old_full
        if old_map is None:
            current.pop("map", None)
        else:
            current["map"] = old_map
        settings.write_text(json.dumps(current, indent=2))
        (out / "invite.txt").unlink(missing_ok=True)


if __name__ == "__main__":
    main()
