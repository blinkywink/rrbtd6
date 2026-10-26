# Observations from the original game

Traces recorded from the real BTD6 install. They are the ground truth for compatibility tests.

**Status:** none recorded yet. Every test that depends on one reports `SKIPPED`.

## Format

Plain text, one sample per line, with `#` comments:

```
# map=<id> mode=<id> round=<n> build=57.2.12199 recorded=<date> method=<how>
frame <int> bloon <id> distance <float>
```

## Recording method (planned, needs your approval)

Values have to be read from the running game's simulation objects, for example `Bloon.distanceTraveled` once per `Simulation.Simulate()`. MelonLoader and BTD6 Mod Helper are already installed, so a small offline-only logging mod can do this.

Constraints:
- offline single-player only
- no Epic or Ninja Kiwi online services
- you launch the game yourself

## Planned experiments

### E1: Red bloon leak time on Monkey Meadow (round 1)

The engine's prediction: `cargo run -p btd6-sim --release --example round_replay`.

| Quantity | Predicted | Observed |
|---|---|---|
| Path length (sim units) | 1276.3029 | — |
| First Red leak, **Easy** (×1.0) | 3065 frames = 51.083 s | — |
| Last round-1 leak, Easy | 4115 frames = 68.583 s | — |
| First Red leak, **Medium** (×1.1) | 2786 frames = 46.433 s | — |
| Last round-1 leak, Medium | 3836 frames = 63.933 s | — |
| First Red leak, **Hard** (×1.25) | 2452 frames = 40.867 s | — |
| Last round-1 leak, Hard | 3502 frames = 58.367 s | — |

Difficulty speed comes from `GlobalSpeedModModel` (verified, research/04 §9). Times are measured from pressing Start at 1× speed. Run `round_replay -- tutorial DefaultRoundSet 1 <easy|medium|hard>`.

A stopwatch measurement gives about ±0.2 s, which is enough to catch a gross error in units or speed. Frame-exact checks need the logging mod.
