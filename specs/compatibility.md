# Compatibility specification and matrix

Target: BTD6 build **57.2.12199** (Epic), Unity 6000.3.23f1, IL2CPP. Evidence for every *verified-disasm* row is in `research/04-native-analysis.md`.

## Status vocabulary

| Status | Meaning |
|---|---|
| **verified-obs** | Matches traces recorded from the running game. |
| **verified-disasm** | Transcribed instruction by instruction from `GameAssembly.dll`, and unit-tested. Not yet compared against a live trace. |
| **verified-data** | Values decoded from `sim_model` and cross-checked against known values. |
| **implemented-hypothesis** | Code exists but depends on an open hypothesis. It must not be described as compatible. |
| **structure-known** | Data shape known from metadata. No implementation yet. |
| **missing** | Nothing yet. |

No row is **verified-obs** yet. That needs recorded traces (`tests/observations/`).

## Matrix

| Area | Mechanic | Status | Evidence / open items | Code |
|---|---|---|---|---|
| Time | 60 ticks/s, accumulator, FF ×3, `timeScale + 1` cap | verified-disasm | §1 | `engine/src/time.rs` |
| Time | Per-tick order (`elapsed++`, process list in order) | verified-disasm | §2; `roundTime` gating partly decoded | `engine/src/sim.rs` |
| Time | New objects start processing next frame | implemented-hypothesis | H-PATH-3 | Exported world positions equal Unity's f32 `Transform.position` | Compare against positions logged in game, or reproduce Unity's TRS math in f32 |
| H-MODE-2 | Impoppable (and other modes) inherit their parent difficulty's mods | Transcribe `GameModel.CreateModded` / `ModModel.Merge` |
| H-ORDER-3 | `engine/src/sim.rs` |
| RNG | `SeededRandom` (Park–Miller) | verified-disasm | §3; reference sequence tests | `engine/src/rng.rs` |
| RNG | `NinjaKiwi.Common.Random` (= .NET Framework `System.Random`) | verified-disasm | §3; `Random(0)` reference test | `engine/src/rng.rs` |
| RNG | `BufferedRandom` | verified-disasm | §3 | `engine/src/rng.rs` |
| RNG | Seeding of `syncedRandom` from `GameModel.randomSeed` | missing | Find the `Simulation` ctor/Initialise site | — |
| Math | Vector3 distance/lerp/equality, `CapAngle`, rotation = `atan2(x, y)` | verified-disasm | §4; `atan2f` ulp parity is H-MATH-1 | `engine/src/math.rs` |
| Track | `CalcPointInfo` distances and 0.01-step table | verified-disasm | §4 | `engine/src/path.rs` |
| Track | Lookup without interpolation, `y-4`/height offset | verified-disasm | §4 | `engine/src/path.rs` |
| Track | Scene → PathModel: Bezier sampling (30/segment), `ToSimulationSpace` with 60° tilt | verified-disasm | §8 | `engine/src/mapbuild.rs` |
| Track | Node flattening order, PointSource defaults, spawn/leak components | verified-disasm | §8 | `engine/src/mapbuild.rs` |
| Track | Exported world positions bit-exact | implemented-hypothesis | H-PATH-3 (Python vs. Unity f32 transform math) | exporter in `tools/assets/` (forked session) |
| Track | Splitters / entry-exit routers / multi-path spawn selection | structure-known | `DistanceToPath` loops verified; routers not implemented (H-SPAWN-1) | — |
| Track | Per-point extra settings (speed multiplier, scale, untargetable) | structure-known | `DistanceToExtraPointInfoBloon` not transcribed | — |
| Bloons | Per-frame movement `((speedFrames*trackMult)*mapWide)*behaviours` | verified-disasm | §5; behaviour multipliers not modelled | `engine/src/sim.rs` |
| Bloons | `speedFrames = speed * (1/60f)`; `isImmuneToSlow` guard | verified-disasm | `BloonModel.set_Speed` | `engine/src/data.rs` |
| Bloons | Spawn clamp, end-of-map, leak-point tests | verified-disasm | §5 | `engine/src/sim.rs` |
| Bloons | Collision radius = `BloonModel.radius` | implemented-hypothesis | H-BLOON-1 | `engine/src/bloons.rs` |
| Bloons | Rotation smoothing (`LerpAngle`, factor `tsf*0.5`) | implemented-hypothesis | H-ROT-1: snaps to target for now | `engine/src/sim.rs` |
| Bloons | Stats (speed, health, radius, children, leak) | verified-data | MOAB 616 / BFB 3164 / ZOMG 16656 / DDT 816 / BAD 55760 match known values | `engine/src/data.rs` |
| Bloons | `GetTotalLeakDamage` incl. ties-to-even rounding | verified-disasm | §5 | `engine/src/data.rs` |
| Bloons | Lives lost via `CalcDamageFromBloon` (difficulty, shields, mutators) | implemented-hypothesis | H-LEAK-2: no modifiers yet | `engine/src/sim.rs` |
| Bloons | Pop/degrade, children placement, layers, armour, camo, regrow | structure-known | `Bloon.Degrade`, `BloonDegradeStepper` | — |
| Rounds | Group → emission expansion | verified-disasm | §6 | `engine/src/rounds.rs` |
| Rounds | Emission timing in frames, list-order firing | verified-disasm | §6 | `engine/src/rounds.rs` |
| Rounds | Round list = groups' emissions concatenated in group order | verified-disasm | `RoundModel.get_emissions` @0x1810D5710 | `engine/src/rounds.rs` |
| Rounds | Round-set seconds → frames: `(scale*start)*60f`, `(scale*(duration+start))*60f` | verified-disasm | `RoundData.GetRoundDef` lambda @0x1813358C0 | `engine/src/data.rs` |
| Rounds | Round-set data (54 sets; DefaultRoundSet = 140 rounds) | verified-data | `RoundSet` MonoBehaviours in `game_data_assets` (forked session's export). DefaultRoundSet round 1 matches the `sim_model` group (1050.75 frames) | `engine/src/data.rs` |
| Towers / projectiles / damage / pierce / targeting | — | structure-known | ~1,500 model and simulation behaviour types | — |
| Map | Placement-area polygons: `ToSimulationSpace(TransformPoint(p), tilt 60).xy` | verified-disasm | §10; the placement rules themselves (tower footprint vs. polygon) are not transcribed | exporter (forked session) |
| Placement | `Map.CanPlace` structure: first containing area, 0.75×radius footprint vs. similar-height areas, tower overlap `(r1+r2)² > d²` | verified-disasm (partial) | §11; polygon intersection helpers and two sub-passes still [I] | `renderer/src/game.rs` `can_place` |
| Placement | `BlockedByBlockerZone` (`(rb+r)² >= d²`), `IsPointOnMap` (±150 / ±116, border −radius), base area first in `map.areas`, blocker circles in sim space at area height | verified-disasm | §11; base-area shape and the blocker flag's meaning are [I] | `renderer/src/game.rs` `can_place` |
| Modes | Difficulty bloon speed (`GlobalSpeedModModel`) | verified-disasm | §9; mod inheritance is H-MODE-2 | `engine/src/data.rs` |
| Economy | Cash as f64 (KonFuze), income, end-of-round bonus | structure-known | | — |
| Collision | Uniform grid broad phase | structure-known | `CollisionGrid<T>` | — |
| Data | `sim_model` full decode | verified-data | Decoder in `tools/sim_model/` (forked session): consumes all but the 4-byte root index, `m[]` fills to 354,277 | — |
| Rendering | Prototype | see `renderer/README.md` | Separate crate (forked session) | `renderer/` |

## Open hypotheses

| ID | Statement | How to resolve |
|---|---|---|
| H-MATH-1 | Rust `f32::atan2` matches the game's CRT `atan2f` bit-for-bit | Compare outputs on sample inputs, or disassemble `0x1804CCA30` |
| H-ORDER-3 | Objects created in frame F first `Process` on frame F+1 | Trace `RootObject` creation into `process` / `FlushProcessList` |
| H-SPAWN-1 | Single spawn path | Implement `Splitter` / `spawnJunction` routing |
| H-BLOON-1 | Bloon +0x150 radius = `BloonModel.radius` | Trace `Bloon.Initialise` / `Collidable` radius set |
| H-ROT-1 | Rotation smoothing | Transcribe `SMath.Math.LerpAngle` @ `0x180EA0ED0` |
| H-LEAK-2 | Lives lost equals total leak damage | Transcribe `Simulation.CalcDamageFromBloon` @ `0x180A71E30` |

## Resolved

| ID | Resolution |
|---|---|
| H-TIME-1 | 60 frames/s: `deltaTime * 60.0f` in `InGame.UpdateSimulation` |
| H-MATH-2 | Lerp is `p0*(1-t) + p1*t` (`CalcPointInfo`) |
| H-PATH-1 | Distances are recomputed as cumulative 3D `Vector3.Distance`, overwriting `PointInfo.distance` |
| H-MOVE-1 | Movement formula and `speedFrames = speed * 1/60f` |
| H-LEAK-1 | Leak by clamped end-of-map equality, or the `leakDist` radius test |
| H-ROUND-1 | Emission times are frames; fire when `abs <= (float)elapsed` |
| H-ORDER-2 | Phase order is by process list (spawner before bloons, creation order) |
| H-DATA-1 | Superseded by the full decoder |
| H-ROUND-2 | `RoundModel.get_emissions` concatenates in group order |
| H-MODE-1 | `GlobalSpeedModModel`: `speed*mult + add` on every BloonModel (§9) |
