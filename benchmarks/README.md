# Benchmarks

```
cargo run -p sim-bench --release -- <bloons> <frames>
```

The harness prints one JSON line with `ns_per_frame`, `ns_per_bloon_frame` and `allocs_in_timed_frames`. A counting global allocator is installed, and the target for the timed frames is **0** allocations.

## Results

| Date | Bench | Bloons | ns/frame | ns/bloon-frame | Allocs | File |
|---|---|---|---|---|---|---|
| 2026-10-21 | movement | 1,000 | 3,887 | 3.89 | 0 | `results/2026-10-21-movement.jsonl` |
| 2026-10-21 | movement | 10,000 | 39,734 | 3.97 | 0 | same |
| 2026-10-21 | movement | 100,000 | 1,177,369 | 11.77 | 0 | same |
| 2026-10-21 | movement, **exact path table** (AoS) | 1,000 | 16,559 | 16.56 | 0 | `results/2026-10-21-movement-exact.jsonl` |
| 2026-10-21 | movement, exact path table (AoS) | 10,000 | 96,200 | 9.62 | 0 | same |
| 2026-10-21 | movement, exact path table (AoS) | 100,000 | 905,615 | 9.06 | 0 | same |

Since the second batch, movement uses the original's exact 0.01-step position table instead of per-segment interpolation, so lookups are memory-bound. The synthetic path is 22,000 units long, so the table is 2.2M entries (44 MB). Storing the table as 20-byte records (one cache miss per lookup) instead of five columns improved 1k bloons from 26.0 to 16.6 ns and 10k from 14.5 to 9.6 ns. 100k went from 7.5 to 9.1 ns, which is unexplained and possibly run-to-run noise. Real maps are much shorter (hundreds of units, so a table of 1–4 MB), which means real workloads will cache better than this test.

In the first batch, the jump at 100k bloons comes from the working set (about 3.6 MB of columns) falling out of L2 cache. It is worth revisiting when bloon state is packed more tightly.

## What these numbers do **not** show

These numbers are **not** a comparison with BTD6. They cover only the movement subsystem, run on a synthetic path with synthetic data. The original has not been measured yet.

A matched baseline needs:
1. The same scenario in the original game, for example sandbox mode with N bloons of one type on one map and no towers.
2. A measurement of `Simulation.Simulate()` time per frame in the original. A MelonLoader timing patch, run offline and launched by the user, is the least intrusive option. Whole-process CPU usage from Windows performance counters is a fallback.
3. The same map path and bloon stats in `btd6-sim`, which depends on the `sim_model` decode.

No speedup claim should be made until both sides are measured on this machine.

## Playable prototype stress test (renderer + prototype rules)

```
RRBTD6_FLOOD=Ceramic:4 RRBTD6_CASH=400000 RRBTD6_TOWERS=023 RRBTD6_PATHS=0,2,0,2 RRBTD6_NOSELECT=1 RRBTD6_SPEED=1 RRBTD6_FRAMES=2000 RRBTD6_AUTOTEST=out.png cargo game
```

This spawns 4 Ceramics every rendered frame against three 5-0-2 towers. The autotest log prints the average and worst `Game::step` time every 100 frames. The machine and resolution are the same for both runs (1600x900, 4x MSAA).

| Change (2026-10-21) | Bloons | Sim step avg / max | Frame rate |
|---|---|---|---|
| Before | ~10k | 3.1 ms / 153 ms | 15-19 fps |
| Cached bloon positions + no rescan after hits + batched bloon sprites | ~16k | 0.56 ms / 2.6 ms | ~40 fps (117-129 fps up to ~9k) |

What changed:
- **Bloon positions:** `Bloon.pos` is cached (it was a path lookup per tower and projectile check).
- **Collision after a pop:** the loop continues from the changed slot instead of restarting at bloon 0. That restart was quadratic during pop cascades.
- **Bloon sprites:** same-texture bloon sprites go into one draw call (`render3d::SpriteBatch`) instead of one material switch and draw per bloon.

This is still not a comparison with BTD6, because no matched baseline from the original has been recorded.

## Prototype sim, late-game load (2026-10-22)

```
RRBTD6_BENCH_RATE=4 RRBTD6_BENCH_BLOONS=6000 cargo test --profile game -p btd6-game sim_bench -- --ignored --nocapture
```

Headless `Game::step` with 19 tier-5 towers (Dart 0-2-5, Tack 5-0-2, Bomb 2-0-5, Ninja 5-0-2, Wizard 0-5-2, Super 2-0-5, Boomerang 5-2-0, Ice 2-0-5, Sniper 0-2-5, Druid 5-0-2, twice) on Monkey Meadow and a flood of Ceramics (all spawned at the track start). The last 2,400 of 3,000 steps are timed.

| Change | Load | Step avg | Step worst | Pops (same = same outcome) |
|---|---|---|---|---|
| Before | 4/frame, ~2.5k bloons | 0.94 ms | 13.5 ms | 1,032,915 |
| Attack skips its target search while all its weapons are on cooldown; range test before the other filters (tower scan and projectile collision) | same | 0.54 ms | 9.8 ms | 1,034,885 (*) |
| Pops: no sound-list cloning; sounds the audio drain would drop aren't queued; child types resolved once | same | 0.43 ms | 1.6 ms | 1,034,885 |
| Projectile broad phase: uniform grid (24 units), kept in step with swap-removes and spawns, candidates in storage order | same | 0.40 ms | 1.6 ms | 1,034,885 |
| same | 40/frame, ~24.6k bloons | 1.09 ms | 5.6 ms | 533,656 |

(*) The only change in outcome: some of the prototype's own stand-in targeting (random spots, track spots) drew random numbers every frame even while reloading; now only when it can fire. None of those are verified game behaviour.

The grid visits candidates in the same order a full scan would, including the re-check of a slot after a swap-remove, so the per-hit results are unchanged (identical pop counts before and after).

Rendering, same flood test as above: the CPU side of drawing ~16k bloons is ~0.9 ms per frame, so the remaining ~25 ms frames at 16k bloons are GPU fill (overlapping sprites at 4x MSAA). Pop flipbooks are now batched per frame texture (they were one draw call each).

Still not a comparison with BTD6: the original hasn't been timed on this machine.

## Rendering, startup and loading (2026-10-22, later)

Same flood test (Ceramics at 4 a frame, 1600x900, 4x MSAA, Intel Arc B390):

| Change | Bloons | Frame rate | Sprite draw calls / frame |
|---|---|---|---|
| Before | ~16k | ~40 fps (25 ms: 4.5 ms CPU + 19 ms in `next_frame`) | ~3,400 |
| Bloon sprite atlas (one texture for every bloon sprite) | ~21-24k | 81-120 fps | ~33 |

The 19 ms wasn't pixel fill: a smaller window and MSAA off changed nothing. It was draw calls. Sorted far-to-near, a crowd of mixed bloons switched texture almost every sprite, so each batch held one or two quads. The atlas gives each sprite a 2-texel border copied from its edge texels, so linear filtering matches the separate clamped textures (`render3d::atlas_tests` samples every sprite both ways). Painter's order is unchanged.

Startup (launch to first frame of the autotest; `RRBTD6_TIMING=1` prints the phases):

| | Before | After |
|---|---|---|
| Load phase | 2,160 ms | ~300 ms |
| Whole run (launch, one frame, exit) | 3.3 s | 0.95 s |

- Tower base models (0.8 s) and projectile sprites (0.26 s) load on first use. Base models also load one per frame between rounds.
- The 11.8 Mpx terrain image loads the first time the 2D view shows (the 3D view never draws it).
- PNGs at startup and in `render3d::Model::load` are decoded on worker threads (`render3d::decode_batch`).
- `anim::Prefetch`: models are read and decoded on a background thread (`AnimModel::stage`), then uploaded on the render thread (`from_staged`). The selected tower's next upgrade on each path, the tower in hand and every MOAB-class model are requested ahead of time, so upgrades and the first MOAB don't stall.
- The remaining ~100 ms is the 4096x4096 map texture's PNG decode.

Guard: `perf_budget` (in `cargo test --profile game -p btd6-game`) fails if the late-game sim load goes over 1.2 ms average or 6 ms worst per step.

## New sim baseline after data fixes (2026-10-22)

The data corrections in this session make the bench scenario a different (more correct) game, so the step time isn't comparable with the table above:
- Medium bloons at x1.1 speed (`GlobalSpeedModModel`).
- `LinkProjectileRadiusToTowerRangeModel` resizes only its own projectile, so the other weapons fire real projectiles instead of range-sized rings.
- The inclusive `FilterWithTagModel`.

Same machine, same bench:

| Mode | Step avg | Step worst | Pops |
|---|---|---|---|
| Medium (default) | 0.53 ms | 1.8-2.0 ms | 1,045,734 |
| Easy speed (`RRBTD6_TEST_MODE=Easy`) | 0.57 ms | 2.4 ms | 1,037,234 |

`perf_budget` (1.2 ms avg / 6 ms worst) still passes with room to spare.

## Repeatable whole-frame captures (2026-10-26)

Run `python -I tools/benchmark_frames.py --exe BTD6.exe`. Results go to
`local_extracted/frame-bench/` (ignored): every sampled frame in CSV, screenshots,
logs, and `results.json` containing per-run FPS, median/p95/p99/worst frame time,
frames over 16.67/33.33 ms, CPU submission time, executable SHA256 and settings.
FPS is 1000 divided by mean frame duration, not an average of instantaneous FPS.
CPU submission excludes presentation and is not GPU execution time. Whole-frame
duration includes submission and presentation; this does not measure input latency.

The default comparison alternates three pairs with UI text caching off/on in the
same executable. For a future optimization, preserve this instrumented baseline
executable and pass `--reference-exe PATH_TO_BASELINE --exe PATH_TO_CANDIDATE`.
Both binaries must support `RRBTD6_FRAME_BENCH`. The runner launches them serially,
uses a 1600x900 window, disables the FPS cap/vsync/FPS-title counter, restores those
settings afterward, records the remaining graphics settings, and excludes 300
warm-up frames. Keep the game window unobscured and other GPU workloads idle.

The benchmark advances two simulation steps per rendered frame and supplies a
fixed Ceramic flood, instead of letting faster rendering alter the amount of
simulation performed. Benchmark-only lives prevent defeat truncating the test;
incomplete captures are rejected. Recorded bloon/projectile/tower populations must
match for a useful comparison (the JSON reports this separately; equal counts do
not prove complete simulation equivalence). Wall-clock-driven animation/effects
can still differ between runs. Normal play uses its original time accumulator.
The final screenshot/readback and initial loading are outside the frame samples.

Use `--flood Ceramic:40 --frames 1000` for the heavier crowd scenario. A workload
growing over time is compared at matching frame indices, not at a single fixed
population. For different machines, power modes or graphics settings, establish
a new baseline. Small differences within the spread of repeated runs are noise,
not a verified performance improvement.

First text-cache A/B results (three alternating pairs, full textures, native
1600x900, render scale 1, MSAA 1): the light flood's median run FPS was
501.9 reference / 506.7 cached (+1.0%); median run p99 was 3.19 / 3.23 ms.
The heavy flood was 72.51 / 72.40 FPS (-0.15%); median run p99 was
19.21 / 20.32 ms. Population traces matched in both scenarios. Repeats varied
substantially, including a 247 ms reference spike and a cached heavy run with
98 frames over 33.33 ms. These results do not establish an overall FPS gain
from text caching or stutter-free performance. Raw spikes remain in the CSVs;
median-of-run summaries must not be mistaken for the worst observed frame.
Local records: `frame-bench-light-valid/results.json` and
`frame-bench-heavy-valid/results.json` under `local_extracted/`.

## Late-game paragon round suite

`python -I tools/benchmark_frames.py --exe BTD6.exe --suite --output local_extracted/paragon-round-baseline`

Runs each complete round three times: 98 with Dart; 140 with Dart/Boomerang;
200 with Dart/Boomerang/Ninja/Wizard. All are degree 20 (`--degree` overrides it).
Uses Monkey Meadow/tutorial on Medium with fixed freeplay seed 12345, no abilities
automated, no synthetic bloon flood, and no between-round purchases. Lives/cash
are benchmark setup values. The first 300 rendered frames warm tower models with
simulation frozen, then the round runs at two simulation steps per frame until
completion. The client has 140 exported rounds; 200 uses its freeplay generator.
This measures this client's implementation, not verified parity with the original
game. Wall-clock animations can differ at different frame rates.

The CSV additionally records actual round, active state and paragon count; the
runner rejects wrong scenes and unfinished rounds. PNG dimensions record actual
framebuffer size. `summary.md` and `suite.json` retain results, including 1% low FPS
(1000 divided by the mean of the slowest 1% of frames). Use `--reference-exe` to
compare a future optimization against a saved instrumented build. Preserve raw
CSV spikes; the median of run worst-frame values is not the worst observed frame.

Initial baseline on Intel Arc B390, 1600x900 framebuffer, full textures, render
scale 1, MSAA 1, uncapped/vsync off: median run FPS / 1% low was 465.4 / 65.8
on 98, 367.8 / 95.4 on 140, and 442.9 / 88.2 on 200. Worst frames across all
repeats were 32.60, 35.22 and 76.61 ms respectively. Each scene had identical
population traces across repeats. Combat asset first-use stalls are included;
initial loading is excluded. These are baselines, not claimed optimization gains.

Shared-renderer A/B follow-up: three alternating pairs against the saved baseline
(`local_extracted/paragon-broad-render-ab/comparison.md`). Average FPS moved about
-0.9% / +1.0% / +1.1% on rounds 98/140/200, within run variation. Round 200's
worst observed frame fell from 72.73 to 35.45 ms, and median-run 1% low improved
from 89.7 to 106.3 FPS. All population traces matched. Round 140's 1% low was
worse in this capture (95.8 to 88.8), so this is not a universal frame-rate gain.
The changes reuse common geometry/order buffers and share repeated particle
textures; they do not alter tower logic. GPU model/sprite checks and particle
texture readbacks/sampling matched exactly. First-use stalls still remain.

Animation-buffer reuse follow-up uses `--suite --same-build-ab --ab-feature
pose-reuse`, so parallel gameplay edits cannot differ between reference/candidate
binaries. The old scratch/copy implementation and pooled ownership-transfer path
run in the same executable. Real-model vertex bits and GPU pixels matched.
Three alternating pairs recorded round 98 FPS 457.6 -> 452.9 (no gain), round
140 337.3 -> 345.8 (+2.5%), and round 200 352.1 -> 414.2 (+17.6%). Round 200 CPU
frame work was 1.443 -> 1.261 ms (-12.7%). Repeats varied substantially; these
are observed results, not guaranteed gains at every workload. Peak private memory
was essentially flat on 200 (1219 MiB), with no consistent RAM reduction across
the suite. See `local_extracted/paragon-pose-controlled-ab/comparison.md`.

Windows runs now also record whole-process CPU seconds and OS peak private commit
and working-set counters sampled every 100 ms. These include startup/warm-up and
are not gameplay utilization percentages or dedicated VRAM figures. Samples can
miss a peak occurring only in the final 100 ms before exit. The frame CPU column
continues to measure active gameplay separately. Absolute results from different
source/data snapshots are not interchangeable.

Sprite-storage A/B (`--suite --same-build-ab --ab-feature sprite-storage`): three
alternating pairs in the same build reduced median peak private commit by 53.1,
56.2 and 53.7 MiB on rounds 98, 140 and 200. Startup no longer retains 24 MiB of
decoded sprite pixels, and fallback/UI icon uploads share identical PNG content.
All 44 icon GPU readbacks and 2D/3D smoke checks passed; 98 renderer tests passed.
FPS varied (+14.7% / -0.6% / -2.5%), so this is a demonstrated memory reduction,
not a claimed general FPS gain; round 200 p99 also worsened in this sample.
Details: `local_extracted/paragon-sprite-storage-ab/comparison.md`.

Shadow texture sharing (`--ab-feature shadow-textures`) retains identical
mipmapped pixels with weak ownership. A real-asset base/final-tier shadow audit
reduced 74 uploads to two and their RGBA mip-chain storage from 24.67 to 0.67 MiB.
It loads shadows only, not full towers; these are resource sizes, not measured
process RAM. GPU readbacks, three sampling sizes, content isolation and weak
lifetime checks passed; 98 renderer tests passed.

The three-pair same-build round suite recorded FPS changes of -0.4%, -7.5% and
-2.2% on 98/140/200. A five-pair repeat of 140 reversed to +2.0% (347.8 -> 354.8
FPS); CPU frame work was 1.677 -> 1.673 ms. Observed stalls varied heavily on
both paths. No reliable frame-rate gain is established. These sparse-paragon
scenes load few tower shadows (bloon models have none), and median peak private
commit moved +0.2 / -1.3 / -3.5 MiB in the suite, with -3.3 MiB on the 140 repeat.
All population traces matched. Full local captures are in
`local_extracted/paragon-shadow-sharing-ab` and
`local_extracted/paragon-shadow-sharing-repeat-140`.

Renderer streaming-storage A/B (`--suite --same-build-ab --ab-feature
render-storage`): three alternating pairs at 1600x900 compare the same executable
with original 10,000-vertex buffers/material resets against 5,000-vertex buffers,
safe mesh remapping and consecutive opaque-material batching. No rustc/link
process was observed by the separate 500 ms compiler monitor during this suite.
Desktop applications remained open; these are observed local measurements.

- Round 98: 439.2 -> 534.6 FPS (+21.7%); peak private commit 917.0 -> 827.3 MiB.
- Round 140: 320.7 -> 341.5 FPS (+6.5%); 1070.3 -> 984.0 MiB.
- Round 200: 417.4 -> 446.6 FPS (+7.0%); 1139.9 -> 1092.6 MiB.

Active CPU frame work fell 1.007 -> 0.977, 1.726 -> 1.598 and 1.319 -> 1.285 ms.
Population traces matched throughout; no draw-capacity clamping warnings occurred.
99 renderer tests passed. Eight fixed real-scene GPU comparisons matched, and
their combined fingerprint matched between old/new capacities. Eight animated
model and six sprite GPU comparisons also passed in each configuration. These
changes preserve geometry, texture sampling, shaders, draw order and simulation.
P99 did not improve on 98 (7.19 -> 7.48 ms), and 200 still had a worst observed
frame above 33 ms; higher average FPS does not establish stutter-free playback.
Raw captures: `local_extracted/paragon-render-storage-ab`; conditions and limits
are recorded in its `comparison.md`. Earlier compiler-profile and map-only runs
overlapped other compilation and are excluded from accepted timing results.

Persistent map GPU follow-up: plain/gradient map geometry is uploaded once to
immutable buffers, with special shaders streamed in their original order.
20 exact GPU/state comparisons cover four maps, hidden parts, viewports, scaled
targets, MSAA and subsequent streaming geometry. Buffer lifetime checks and all
99 renderer tests passed. The isolated feature (`--ab-feature static-maps`) gave
observed average FPS gains of +10.5% / +11.1% / +7.9% on 98/140/200; the accepted
200 result is the repeat in `paragon-static-map-repeat-200`, because its initial
suite capture overlapped compilation. These are matched pairs within each scene,
not cumulative percentages to add to earlier measurements.

Final combined comparison (`--ab-feature render-storage`): original streaming
capacities, redundant static arrays and per-part resets versus the reduced
buffers, compatible batching and persistent map geometry. Three alternating
pairs per round, same executable and frozen simulation inputs across all rounds:

- Round 98: 464.4 -> 588.1 FPS (+26.6%); peak private commit 915.6 -> 815.4 MiB.
- Round 140: 381.2 -> 435.9 FPS (+14.4%); 1058.6 -> 962.4 MiB.
- Round 200: 442.8 -> 546.8 FPS (+23.5%); 1168.6 -> 1100.3 MiB.

Active CPU frame work fell 0.924 -> 0.803, 1.522 -> 1.369 and 1.002 -> 0.858 ms
(about 10-14%). All 18 accepted captures had no detected compiler overlap, and
all population traces matched. Median p99 improved in all three cases, but the
worst candidate frames were 32.73 / 33.30 / 33.70 ms. Round 98's worst individual
frame worsened, and 140's median 1% low was essentially flat/slightly lower.
This is not a stutter-free result or a comparison with official BTD6.

The runner now snapshots game/map JSON once per suite and writes only private
benchmark preferences. Game/map hashes and the isolated-input mode marker are
verified, so live exports and user-settings edits cannot silently change these
inputs mid-comparison. Windows compiler-name polling is 500 ms, with a two-second
quiet period before launch; contaminated captures are preserved and retried, up
to five attempts. The guard was checked against a live stdin-blocked rustc, and
snapshot isolation against source edits. Real-app smoke testing confirmed live
preferences stayed byte-for-byte unchanged. Art assets remain shared, and other
desktop/GPU activity is not suppressed. Old binaries lacking snapshot support
must be rebuilt before use with the updated runner.

Accepted final captures: `local_extracted/paragon-static-map-combined-final`.
Its `comparison.md` records scope and limits. Earlier combined attempts affected
by compilation or live data exports are excluded. These two-renderer-change
gains must not be described as the cumulative gain since the earliest prototype.

Combat asset preparation follow-up (`--suite --same-build-ab --ab-feature
background-assets`): three alternating pairs per scene, identical executable,
frozen simulation inputs and degree-20 paragons, 300 frozen-simulation warm-up
frames, then two simulation steps per rendered frame at 1600x900. The reference
retains the previous four-job startup prefetch. The candidate prepares reachable
bloon/damage models and the placed towers' weapon art and particle definitions
incrementally before first use. Startup/warm-up is excluded; any unfinished
first-use work and all combat stalls remain in measured frames.

- Round 98: worst across repeats 24.74 -> 16.12 ms; median p99 6.11 -> 4.40 ms;
  median 1% low 81.1 -> 138.3 FPS; average 610.8 -> 633.3 FPS.
- Round 140: worst 31.46 -> 7.22 ms; p99 5.45 -> 3.96 ms;
  1% low 105.1 -> 210.1 FPS; average 467.7 -> 474.0 FPS.
- Round 200: worst 31.37 -> 3.75 ms; p99 3.17 -> 2.61 ms;
  1% low 123.7 -> 346.0 FPS; average 565.3 -> 592.8 FPS.

Frames over 16.67 ms fell from 15/18,339 to 0/18,339 in the final accepted
captures. All population traces matched; all 18 accepted runs had no detected
compiler overlap. One affected capture was preserved/retried by the guard.
Peak private commit changed 834.2 -> 817.7, 983.7 -> 993.1 and 1114.0 ->
1107.4 MiB: preparing assets earlier can raise memory in a scene. The measured
binary SHA256 is e8657419df22d4a3fb544354d2622f5cdeddd047f12b98d85d79f98b205e3191.
Raw reports: `local_extracted/spike-background-assets-final`.

101 tests passed, including read-only preparation/state-digest coverage and a
nonblocking-worker check. Four real-model background/takeover GPU comparisons,
cooperative particle texture/sampling comparisons, and the 20 static-map GPU/state
checks passed. Preparation creates no particle instances and advances no RNG.
These are short, local measurements, not a stutter-free guarantee. An earlier
intermediate build still recorded a 42 ms frame, much of it outside CPU submission;
other desktop/GPU activity remained open. Loading and immediate unforeseen
placement/upgrade assets can still stall. Round 200 uses this client's generated
freeplay data; this is not a comparison against official BTD6.

Redundant fallback-model A/B: once a tier display is loaded, its unused root base
meshes/animations are omitted. `--suite --same-build-ab --ab-feature fallback-models`
retains the original duplicate loads in reference runs, using the same executable
and frozen inputs, three alternating pairs each at 1600x900:

- Round 98: median peak private commit 822.3 -> 820.0 MiB; average 654.2 -> 648.2 FPS.
- Round 140: 950.9 -> 874.7 MiB (76.2 MiB saved); 573.2 -> 564.1 FPS.
- Round 200: 1165.9 -> 927.4 MiB (238.5 MiB saved); 586.4 -> 584.9 FPS.

This is a memory/first-use-work improvement, not an established average-FPS gain.
FPS changed -0.9%, -1.6% and -0.3%; median p99 changed 3.89 -> 4.00, 3.41 ->
3.44 and 2.59 -> 2.75 ms. Candidate worst frames were 12.19 / 10.30 / 5.05 ms;
all 18,063 measured candidate frames stayed below 16.67 ms. Population traces
matched and every accepted capture had no detected compiler overlap; affected
captures were preserved/retried. Desktop activity remained open. The timing
snapshot SHA256 is 37b35bcbe80dd5f04ab726f907a343850c8108582146841644244ff73cf57c85,
saved locally as `frame-bench-baselines/fallback-models.exe`. Raw reports:
`local_extracted/fallback-models-final`. Subsequent placement-shadow/key-reuse
cleanup and concurrent gameplay edits can make installed builds differ from that
snapshot; these paired figures apply to the recorded binary.

Separate presentation audit reproduced three blank physical frames during cold
Bomb Shooter placement, caused by awaited asset work suspending the root draw
future. The foreground completion fix gave zero blanks across four 119-frame
late-placement captures (Bomb, Ninja, Super, Sniper). Those audits read back every
presentation and are excluded from performance timings. Completely cold assets
can still pause a frame. Startup/warm-up remains excluded from gameplay timing.
The final build passed 102 tests and six strict late-placement/preview captures
(714 presentations, zero blanks), adding Ace and Dartling plus in-hand previews.
The tier/root placement-shadow records matched on all 26 compared tower exports;
all 17 corresponding shadow PNG pairs were byte-identical. Real-model prefetch,
particle texture/sampling and 20 map GPU/state comparisons also passed.

Selection prediction follow-up: retire completed obsolete staging jobs, return
unuploaded decode claims when their owner is discarded, reuse prediction paths,
and prepare the selected tower's next upgrade effect. Unfinished jobs still count
toward the existing four-model cap, with no detached workers. A five-pair alternating
same-build test selects Dart, switches to Super on frame 40, and buys Super Range
on frame 100; frame 101 includes first-use rendering. Private settings cap both
variants at 120 FPS. After ten warm-up frames, 695 frames per variant were measured.

- Median upgrade-frame CPU submission: 11.574 -> 10.447 ms (9.7% lower).
- Median upgrade-frame wall time: 12.642 -> 11.597 ms (8.3% lower).
- Median peak private commit: 771.5 -> 752.1 MiB (19.3 MiB lower).
- Overall average FPS: 109.02 -> 109.11; no demonstrated throughput gain.
- Median p99: 9.91 -> 10.51 ms; worst across captures 15.75 -> 12.93 ms.

Preparation does earlier work, so an improved purchase frame does not establish
an improvement in every tail metric. All population traces matched and all ten
accepted captures had no detected compiler overlap. Desktop activity remained
open. This is a small selection/upgrade workload, not a general speedup percentage
for the game. Raw data: `local_extracted/prediction-effects-final-ab`; measured
SHA256 a54f83f77f72e102dd47f5e5143f48747476dc509ff7f4c8c9c4ac21555bc835,
saved as `frame-bench-baselines/prediction-effects.exe`. Later verification-only
code, the hero lookahead correction and concurrent gameplay edits can make the
installed executable differ from this snapshot.

106 tests passed, including obsolete-job limits, running-worker retention and
decode-claim cancellation/upload lifetimes. The GPU audit matched 80 upgrade
animation frames and RNG states, four model prefetch/takeover cases, particle
texture sampling and 20 static-map/state cases. A strict selection-and-upgrade
presentation capture recorded 149 presentations and zero blanks. These readbacks
are separate from performance measurements.

A round-200 steady-play check was repeated after an initial single pair showed
556.4 -> 521.8 FPS (-6.2%). The three-pair follow-up measured median 557.1 ->
555.2 FPS (-0.35%), p99 2.95 -> 2.91 ms, and matching population traces with no
compiler overlap in accepted runs. This supports no claimed overall FPS gain;
desktop timing variation remains. Both reports are retained as
`prediction-round-200-check` and `prediction-round-200-repeat` under
`local_extracted/`. That repeat used a later verification build with concurrent
gameplay edits; each A/B pair used one identical binary and frozen inputs.


Frame limiter follow-up (2026-10-26): replace per-frame relative waiting with an
absolute presentation schedule and a private Windows high-resolution waitable
timer. The old limiter added submission/swap time after each complete frame budget
and burned CPU spinning for the final wait. Missed deadlines now reset without
catch-up bursts; cap changes and Unlimited reset the schedule. Render content and
simulation code are unchanged by this change.

Three alternating same-build pairs, 1600x900, fixed inputs, 120 FPS cap, existing
selection/upgrade fixture, 480 frames/run with ten warm-up frames excluded:

- Average displayed rate: 109.31 -> 119.86 FPS (+9.6%, cap accuracy).
- Median run p99: 9.60 -> 9.14 ms.
- Worst individual frame across captures: 14.97 -> 13.04 ms.
- Median total process CPU time/run: 3.125 -> 2.750 seconds (12% less).
- Median peak private commit: 776.7 -> 777.8 MiB (no memory gain).

CPU totals include startup/shutdown and all threads, unlike CPU submission times
which exclude the limiter. They compare the same 480-frame workload, not steady
CPU utilization. All 1,407 measured frames per variant had matching population
traces; no accepted capture overlapped a detected compiler. These results are
about capped pacing, not a higher maximum rendering throughput.

The uncapped control used round 200 with four degree-20 paragons, three alternating
pairs, 10,047 measured frames per variant: 622.45 -> 624.50 FPS (+0.33%, effectively
unchanged), median p99 2.84 -> 2.75 ms. Population traces matched and no accepted
capture overlapped a compiler. This remains the prototype's generated freeplay
workload, not a measurement against official BTD6. Desktop/GPU activity was not
suppressed. A follow-up 60-cap experiment completed three captures around 57/60
FPS, then Windows commit exhaustion aborted another capture; that incomplete
comparison is retained but excluded from these aggregates. The 144-cap game test
was skipped after that failure; isolated limiter tests cover it separately.

Raw accepted data: `local_extracted/pacing-120-ab` and `pacing-uncapped-ab`.
Measured executable SHA256:
577af50b6cf18657b3d7ccd48d36d58627ee42de0f92a355b7b85a758756a30b,
saved as `frame-bench-baselines/frame-pacing.exe`. Four isolated pacer tests cover
submission compensation, stalls, cap changes/Unlimited and native timer deadlines.
The full renderer test suite passed: 110 tests, 14 ignored. A separate native-FFI
check also passed all four pacer tests after aligning the test-only process-handle
declaration with the existing memory counter. The installed executable matches the
measured SHA256 above. Its strict selection/upgrade presentation audit recorded
149 presentations and zero blanks. These readbacks ran separately from timings.
