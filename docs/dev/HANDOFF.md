# Handoff: continue exactly where the last session stopped

Every session working on rrbtd6 reads this first and keeps it current. The goal now is making
every tower's simulation **exact**: each recording of the real game matches the clone frame for
frame. A session can run out of context at any moment, so progress is saved in small steps:

1. Pick the top item of **Now** (below). Work on it.
2. As soon as something is verified (a recording newly matches, a rule is confirmed in the
   decompile, or a dead end is ruled out): update **Status**, **Now** and **Log** in this file,
   then run `sh tools/checkpoint.sh "<one line: what changed>"`. That commits on `master` in
   this folder, so the next session sees it immediately. Don't wait for a big batch.
3. Never leave work only in your head: a half-finished idea goes in **Now** as notes
   (file:line, decompile address, recording + roundTime) and gets checkpointed.

Work directly in `C:\Users\Admin\Desktop\rrbtd6` on `master`. No worktrees, no new branches,
no pushing (the `public` remote is the published release repo; the owner decides what goes
there).

## Asset formats changed (2026-11-01)

Only matters when exporting or inspecting assets (simulation code is unaffected; details in
tools/EXTRACTION.md):
- Animated models: `skin.bin` (tools/assets/skin_anims.py) instead of `anim.bin` / `anim_q.bin*`. The
  runtime rebuilds the same frames. Read one with `skin_anims.float_frames(folder)`. Set
  `RRBTD6_VERTEX_ANIMS=1` on export_animated.py to get the old `anim.bin`. A few models keep vertex
  frames: grafted Sun Temple ones and any f32 can't match (IceMonkey AbsoluteZero).
- Audio: the game's own Vorbis as `audio/<clip>.ogg` instead of `.flac`.
- `tools/build_local_assets.sh` now just runs `tools/extract_all.py`; add new exports there, or a
  fresh install won't get them.
- Don't verify by launching the game unless the owner asks; check files and code.

Map mechanics: `maps/<key>/mechanics.json` (tools/assets/export_map_mechanics.py) has every game script of
a scene with its fields: Removeable (cost, area), MapEvent with triggers and actions, clickable items, route
rules (MoabOnlySplitter, AlternateRoundSplitter...), area buffs, prop animators. Nothing reads it yet; the
runtime knows paths, areas and the spawner rules below. `summarize_map_mechanics.py` prints what each map
needs. `mechanics.json` "parts" gives each model part's scene object (null: a NestedPrefab source, none left
after re-export); `maps/map_clips.json` and `maps/<key>/binds.json` have prop clips, controllers and bone
bindings. Map models are still static.

Map spawning: `renderer/src/map_routes.rs` routes AlternateRound, MoabOnly, BloonTagOnly and
AlternateRoundMultiPath spawners from map.json "spawner" (the map's first active PathSpawner, as
GatherMapBloonPathSpawners). Still open: RuleSplitter, AlternateBloonSplitter, BloonTagSplitter; exit splitters
of those types (Splitter.Rout is still round-robin).
Removeables and map events: `renderer/src/map_logic.rs` from map.json "logic". Events run on AfterRound, StartMap,
PreBloonSpawn and RemoveableSold, with Path, Area, Removeable, Sell and Event actions; `Game::areas` is now live
(disabled areas start off it). The buy button is hud.rs `remove_menu`. Not modelled: StartRound, SimTime and click
triggers, other actions (moving areas, projectiles, towers), RegenRemovable, Removeable.textKey and cost formulas,
towers destroyed by a new area, Path deactivation (no map uses it).
Start-of-round, sim time and lives-lost triggers are hooked in `game.rs`; lives are only hooked at leaks.
Not built yet: clickables, prop animation (bought objects without a display just vanish), NestedPrefab placement.
Bosses: docs/dev/BOSSES.md, with `renderer/src/boss_logic.rs` written and tested but not hooked up.
2 tests fail (2026-11-02): beast_tests::sub_commander and tests::necromancer_graveyard. (heroes_pop passes again:
lightning (Geraldo, Corvus, Druid) had stopped hitting once dontUseCollisionChecker skipped the collision pass;
`ProjectileSpec::lightning` keeps it there, P-LIGHTNING. The real Lightning.Process (@0x180cc84d0: GetTargets,
CreateSplit, CollideBloons) isn't built; Geraldo level 20 pops nothing in that test.)

Speed-only changes to `game.rs` must leave the simulation bit-identical. Check them with
`RRBTD6_SIGNATURE_TRACE=<file> cargo test --profile game -p btd6-game signature_trace -- --ignored` before and
after (simulation_signature every frame of a heavy mixed load, fixed seed; the files must be identical), plus the
recording regression with the same per-recording lines. `sim_bench` (RRBTD6_BENCH_BLOON picks the flood) shows
where the time goes.

Ground rules follow.

## Ground rules (from the owner)

- (2026-11-01) "Done" = fully exact: bloons AND projectile positions bit-identical (float drift
  isn't done). One tower at a time, one session; re-record that tower's scenarios with `# seed`
  just before working on it (only Ninja has seeds for all; Wizard half).

- Never guess: every behaviour comes from the decompiled game or a recording. Unproven rules are
  marked `HYPOTHESIS P-NAME:` in comments; verified ones cite `VERIFIED-DISASM @0x...` or the
  recording that shows them. The only intended differences are in DEVIATIONS.md.
- Don't ask the owner which task is next: take the top of **Now**. Only stop for real blockers.
- Commit whole files, including other sessions' old uncommitted work in them, as long as it
  builds and the regression below doesn't drop.
- Everything is unlocked; Monkey XP is display only. No tower or upgrade locks.
- NK online calls: allowed for features, never in bursts.
- Memory files live in `C:\Users\Admin\.claude\projects\C--Users-Admin-Desktop-rrbtd6\memory\`.

## Tools

- **Compare recordings**: `sh tools/rec/compare.sh wizard sub` (pending sets),
  `sh tools/rec/compare.sh --root dart boomerang` (top-level), `sh tools/rec/compare.sh --one
  pending/wizard/wizard_005.rec` (one, with the first-difference report). In `renderer/`,
  `RRBTD6_REC_AT=a..b` dumps the clone's bloons / darts / towers per roundTime for that range.
  Compare only CHIMPS (`mode=Clicks`) recordings: Monkey Knowledge is on in the owner's
  Standard games and absent in the clone.
- **Recording format** (`research/dumps/recordings/**.rec`): `F frame roundTime round cash`,
  `B id kind distance x y hp`, `W towerId weaponIndex lastEmission animStart rateFrames
  missedRate rotation x y`, `P id x y pierce rotation startX startY ...`, `H projectile bloon
  ...` (hits). Game coordinates; the clone's y is flipped.
- **Record in the real game**: `sh tools/mod/record.sh tools/mod/scenarios/<name>.json` (one
  launch per scenario, kills the game, replaces the Monkey Meadow save). First check
  BloonsTD6 isn't running (the owner may be playing). The game's `Mods` folder must hold only
  `SimRecorder.dll` (and `RustTD6SetupPreview/`); the owner's own mods (Mod Helper,
  HiddenIdentities) live in `DisabledMods` while recording. New recordings carry `# seed`
  (the game's random seed); copy them into `research/dumps/recordings/pending/<set>/`.
  Menu screenshots: a scenario with `"menu": "<MenuName>"`, optional `"menu_clicks"`.
  Rebuild the recorder after editing it: `sh tools/mod/build.sh`.
  Never let a scenario reach a game's last round end (CHIMPS round 100): the win still reached
  RewardsUI and gave the owner an Insta Monkey on 2026-11-02 (wizard_500_bad, now capped at
  7400 frames, before round 100's end at 7498).
- **Decompile**: `python tools/il2cpp/decompile.py "Type::Method"` (or a VA),
  `python tools/il2cpp/methods_of.py "Type"`, offsets in `research/dumps/field_offsets.json`,
  string/float literals `python tools/il2cpp/literal.py 0x...`, the class behind a TypeInfo slot
  `python tools/il2cpp/type_usage.py 0x...`. A vtable call at offset X is
  slot (X - 0x138) / 16.
- **Game data**: after editing `tools/assets/build_game_data.py`, regenerate with
  `ROOT=$(tail -c 4 local_extracted/game_model/sim_model.bytes | python -I -c "import
  sys,struct;print(struct.unpack('<i',sys.stdin.buffer.read())[0])"); python -I
  tools/assets/build_game_data.py local_extracted tutorial "$ROOT"`.
- **Build the game**: `RUST_MIN_STACK=268435456 CARGO_BUILD_JOBS=2 sh tools/build_exe.sh`
  (BTD6.exe in the project folder). If a build fails with a linker "unresolved external" after
  a crash, delete `target/game/game/incremental/btd6_game-*` and build again.

## Regression baseline (must not drop)

Current (2026-11-15): every set full. Run all of: `sh tools/rec/compare.sh ninja wizard alch bomb glue ice
sniper tack sub beast engineer desperado dartling mortar bucc druid super ace heli spike farm mermonkey village
paragon sky quincy moab logs maps maptowers` and `sh tools/rec/compare.sh --root dart boomerang` (maps 82, maptowers 67, sky 39, quincy 14, moab 5,
logs 3, dart 70, boomerang 99, paragon 12).

Last full run (2026-11-07): dart 70/70, boomerang 99/99, wizard 35/35, ninja 34/34, bomb 34/34,
glue 34/34, ice 25/25, sniper 34/34, tack 25/25, sub 32/32, beast 34/34, engineer 34/34, desperado 34/34
(+ pending/_desperado_g 5/5), and 0 for ace, alch, bucc, dartling, druid, farm, heli, mermonkey, mortar,
spike, super, village. Alchemist 34/34. Druid 34/34 (2026-11-09). Mermonkey 34/34, Banana Farm 25/25, Monkey Village 25/25, Spike Factory 24/25, Super Monkey 28/34 (2026-11-10; heli / ace / super re-recorded, old ones in pending/<set>_old). Full run 2026-11-10: every set above at full count (mortar 25/25, bucc 34/34). Monkey Ace 25/25 (2026-11-10): add `ace` to the regression run.
Unit tests (`cargo test -p btd6-game --release`, 2026-11-16): all 237 pass. Fixed the 5 that failed: paragon_degree_and_upgrade / round_bench (sacrificed towers stay until the end of the next step: tests step, the bench calls Game::remove_sacrificed), sub_commander (supports reach towers from the frame after their tower's first; Sub Commander's pierce is `pierce_all`), necromancer_graveyard (a new Necromancer attack has no track points until a round event: the test uses set_round), targeted_darkshift (co-op Darkshift now waits ds.delay through Tower.shift like single player; it teleported at once).
compare.sh now builds once and runs recordings in parallel (RRBTD6_JOBS, default 12): the whole
regression takes ~1.5 min. Run `sh tools/rec/compare.sh --root dart boomerang` and the sets you touched before each
checkpoint that changes `renderer/src/game.rs`.

## Status (recordings matching; update as they change)

| Tower | Match | Notes |
|---|---|---|
| Dart | 70/70 | done |
| Boomerang | 99/99 | done (paragon too) |
| Tack | 25/25 | EXACT (2026-11-05): seeking launch along the normalized line; a TravelStrait lifespan and an AgeModel each end a shot, the shorter wins (tack_520's meteors, 20 s). |
| Sub | 32/32 | done (2026-11-04) |
| Wizard | 34/34 | all re-recorded with R lines; done |
| Glue | 34/34 | done (mutator order, PostDegrade) |
| Ice | 25/25 | done (re-recorded with seeds 2026-11-02) |
| Sniper | 34/34 | EXACT (2026-11-05). Lost double shots end on MapBlockers (CollisionChecker, VERIFIED by the SimRecorder XS stack log) from the frame after they are made; instant shots take their bloon's height; Elite: SwitchTargetSupplierOnUpgrade also on placing at the tier, MOAB targets at the centre (GetTargets), the key from TargetEliteTargetting.Sort (@0x180de19d0, .ctor buffers), equal keys in GetTargets' order (leader, segments front to back, each in list order; SimRecorder ES lines); bounces from where the shot's Instant left it, lastBloonPosition on SetTarget, instant shots meet bloons by the grid's cell; RandomPositionBasic's angle uses 2 pi as a float; ability weapons numbered by ability order. |
| Bomb | 34/34 | EXACT (2026-11-05): ArcEmission.GetDirection takes the owner tower's current rotation (bomb_032's frags). |
| Desperado | 34/34 | EXACT (2026-11-06): an `unplaced` bloon (spawned, or made by an instant / line shot's pop, not moved yet) isn't met by grid collisions at all (040 rt 678). Earlier notes:  (2026-11-04: TheBlazingSun found through SimRecorder Q lines, 13 -> 19) re-recorded with seeds 2026-11-04. BloonDistanceRateBonus uses Attack.GetVisibleTarget Close, StandoffModel ported, Instant shots from throw markers start on the eject point. Open: 002 rt 1614, 003, 004, 005, 020, 025, 03x, 04x, 05x, 205, 250, 300, 320, 420_x, 50x, 520, upgrades_r30 |
; 2026-11-05 (first diffs moved, count unchanged): plain LineProjectileEmission ported (Emit @0x180c42910: 400 long, CollisionGrid.GetInLine samples at thickness, 3x, ... (MoveNext @0x18281ef30, address found with SimRecorder's getinline_movenext probe), nearest first, collided at once; the projectile sits at the start); instant / line hits now land as each attack fires (instant_pass before every attack: desperado_050 rt 73, pistol mark and rifle pop before the mark attack aims); FilterIfAttackHasTarget (@0x180c1ec30) on Attack.hadValidTarget (FindTarget's result when the weapon is ready). Next: BurstWeaponBehavior.Process with alwaysEmptyClip (@0x180c6c250, the shotgun's clip; desperado_005 rt 73-100: lastEmission 18 then 100). DONE 2026-11-05: fireBetweenRounds weapons run on Simulation.time (sim_elapsed, not steps); Vagrant counts in the weapon's own attack range (Q lines: pistol 60 counts a MOAB 42 away, shotgun 36 doesn't); BurstWeaponBehavior.OnRoundEnd per weapon slot (two weapons named "Weapon"). Next: 005 rt 78 a pellet's y step is 5.036886 in the game vs 5.0368857 (direction rounding), bloons differ from rt 90; pistol held too long (80 vs 12 throws). alwaysEmptyClip DONE (2026-11-05): a ready weapon with a part-used clip (from the frame after it became ready) aims at the point its attack range ahead along the tower's rotation (005 2360 -> 24 bloon frames, 205 bloons exact). NEXT 205 rt 139: those pellets start 2.05 off along the facing (eject point for an override target?). 250 (2026-11-06): bloons + weapons exact: a plain line collides inside Emit (before CalcRateFrames), and AttackMinimumRange.FilterTarget (@0x180dac2f0) applies in the visible-target queries of BloonDistanceRateBonus / Standoff. 005h MATCHES (2026-11-06, all 5 fresh runs match): VigilanteMutator.Mutate (@0x18124f180) also adds loseLifeRangeBuff / speedFrames x 1/60 to the first TravelStrait lifespan of each attack without fixedRange (shotgun pellets 15 frames under the buff, rt 3038). 032 MATCHES (2026-11-06): Zoned<Bloon>.BehaviorCreated: a bloon the round spawns goes into each Vagrant bloon zone it is in at once (rt 1001: a Black spawned 47.9 away counts for the rifle's shot; INFERRED not pops' children, desperado_003 rt 2634); this also covers 052 rt 2598 (replaces the buff-end zone refresh). 025 MATCHES (2026-11-06, fresh 025g too): EmitOnDamage's fragments are made before Damage.Collide's crit roll and carry the shot's crit roll as it is then (unrolled: each rolls at its own first damaging hit; rolled: never again). 004 MATCHES (2026-11-06): a bloon already knocked back takes Knockback again (a new mutator, full lifespan; INFERRED: Knockback.Collide's IsMutatedBy check isn't its own mutator's id): rt 261, a pellet a frame after the first two keeps the MOAB going back 12 frames. 052 MATCHES (2026-11-06): a mutator ending (Vigilante buff) comes after that frame's Zoned<Bloon>.Process, still at the old range: the rate is worked out from the zone refreshed with this frame's bloons at the zone's last range (rt 2598: 6 Zebras within 96, one new this frame, 117 frames). 003 MATCHES (2026-11-06): BurstWeaponBehavior.Process (@0x180c6c250) stores the alwaysEmptyClip point (Weapon+0xdc) as the weapon's frame ends, with the tower's rotation then; the next frame's shot uses that stored point (rt 952: the pistol turning the tower to a Lead first doesn't move the shotgun's point). (040 / 005h: one-off position frames.) EMITONDAMAGE + VIGILANTE WEAPONS (2026-11-06, 005 MATCHES, 003 bloons exact, fresh 005h 6 position frames): EmitOnDamage.Collide (@0xcbb8c0) asks Bloon.WillDamageBloon, its fragments are made before the hit's pops (005h rt 1338: pellet Shrapnel ids before the Desert Phantom explosion of the Yellow it popped); the Vigilante rate buff finds each weapon by (name, occurrence) (two weapons are named "Weapon": the shotgun took the pistol's rate, 005h rt 3002). 040: the fresh run (pending/_desperado_g/desperado_040g) matches; the stored one's rt 678 is a rare child-in-grid case. VAGRANT RANGE DONE (2026-11-06): VigilanteMutator.Mutate (@0x126e830) adds loseLifeRangeBuff to the tower and to every attack without AttackModel.fixedRange (+0x66); the 0-3-x attack is fixed at 40, the 0-0-2 one isn't (fresh 032g now matches, 002 still does). 030 / 050 / 250 MATCH: a Line emission's start adds the turned eject offset first, then the height (P-LINEEJECT). Open: 003 004 005 025 032 040 052. EMIT FROM TARGET (2026-11-06): the 030-040 rifle line starts on the Target, emitFromTargetLength (36) long, the shot itself at the barrel: 030 / 040 bloons + weapons exact (an instant shot is limited to its Target only with FilterAllExceptTarget (@0x180c1d3c0, ObjectId equality): 040's mark shot hits whatever it meets. Left: 040 rt 678 that mark shot meets child Ceramic 586 a frame later in the game (586 made at 677 by the MOAB's pop); 030 rt 673 1 ulp), 032 / 052 improved. 050 (2026-11-06): bloons exact to the end once the rifle's line shot can't collide after its sweep (Emit ends with canCollideWithBloons = false); 6 position frames left: rt 2938 a rifle shot's start 1 ulp off in y. MARKED TO POP PORTED (2026-11-06; 050 bloons exact to rt 1602, weapons same): Game::mtp_step + the activation's own check (no marking target -> execution at once, firing the next frame), FilterMutatedTarget inverse (need_effects), FilterMarkedToPop lineages; Instant destroyIfInvalid shots go at the frame's end once their target is gone. Was: MARKED TO POP (next, 050/052/250 rt 120): port MarkedToPop (Activate @0x180e3b6d0, Process @0x180e3c0b0): phase 1 marking attack (others paused) while its weapon is ready, it has a valid target and now - activatedAt < markingTimeFrames; else phase 2: marking paused, execution unpaused, executionStartedAt; FilterMarkedToPop lineages cleared. Models (Desperado-050 'Marked To Pop2'): marking attack (W3, rate 0.2) filters FilterMutatedTarget DesperadoMark (skip marked) + FilterMarkedToPop; its instant shot (destroyIfInvalid) adds DesperadoMark (layers 1, endless, kept on degrade) and, through MarkedToPopProjectileBehavior.SetTarget (@0x180ccc970), the target's lineage to its attack's FilterMarkedToPop (FilterBloon @0x180c1fe30 rejects any bloon with a lineage id in it). Execution attack (W4, rate 0.05): FilterMutatedTarget inverse (marked only) + FilterMarkedToPop; 300 damage instant. In 050 the marking attack has no target at 120 (the mark attack marked them), so execution fires at once (W4 at 120). DONE: an Instant shot with no lifespan lives till its target / pierce goes (050 rt 105 mark shot). 205 MATCHES (2026-11-06): the empty-clip override point starts from the tower raised 10 in display space (BurstWeaponBehavior.Process: +10, or ejectZ with RotateToTarget.useThrowMarkerHeight), so the tower doesn't turn for that shell. 005: the stored recording's rt 90 explosion split doesn't happen in fresh runs (pending/_desperado_g, 005g matches); 005h (fresh, HB radius lines) differs from rt ~? -> next. rt 92, bloons rt 173). 205 (2026-11-05): HYPOTHESIS P-DESTROYEMIT, a child made and popped through by the same hit emits no EmitOnDestroy (Desert Phantom explosions only where the Ceramics went; survivors' later ones as before); first position diff now rt 139, bloons rt 1260. DONE: RandomEmission's speed multiplier scales speedFrames (speed x 1/60) after it is computed (pellet steps exact). VAGRANT ZONE DONE (2026-11-05, 002 MATCHES): each Vagrant weapon keeps Zoned<Bloon>.inZone (IsInZone @0x18239f320, Process @0x18239f600): after the tower's attacks, the bloons within its attack range (2D, no margin); a shot counts that list, destroyed bloons included. upgrades_r30 MATCHES (2026-11-05): a new tower model (place / upgrade) recalculates rateFrames with BloonDistanceRateBonus / Standoff on the bloons then. 300 MATCHES (2026-11-05): bloons made anywhere in this step (also the towers' instant pops) skip this frame's Grow tick. 420_close MATCHES (2026-11-05): an instant shot on a bloon is pre-filtered by the bloon's grid cell (cell_pos) too, as its hit test (SimRecorder HS lines: the hit is CollisionChecker.Process -> CollideBloons, the shot still on its eject point) |
| Beast Handler | 34/34 | Exact. Re-recorded with seeds 2026-11-04. GyrfalconPattern ported from the disasm (fly_birds, GyrState): targets through GetGlobalBest (bestFirst/bestLast) then segment brackets of the handler's target type, snatch/hit around the target by Close bracket order, grapple uses the initial damage, held bloons are off track (Bloon.held: not filed, taken by range + radius) with blockSpawnChildrenCount (unmake_bloon), idle moves through Map bounds. Pets start a frame after they're made and sync the handler's target. CycleAnimation randomize draws. InstantDamage shot with a dead target stays till round end. |
| Engineer | 34/34 | EXACT (2026-11-05): Pin re-stun slot rule and duplicate non-cascading slows (SimRecorder S / M lines). |
| Ninja | 34/34 | done (all R-line recordings) |
| Alchemist | 34/34 | EXACT (2026-11-07): DamageOverTime ticks run in the order the behaviours were made, across bloons (Effect.seq), not the bloons' order: the list-order frames of 005 / 025 / 205 / 052. Re-recorded with seeds 2026-11-06. upgrades_r30 MATCHES (2026-11-07): attacks run in creation order across towers (an attack newer than one on a later tower is put off, `deferred_attack`: the Alchemist's brew attack made at rt 600 acts after the older Dart Monkey's), and a brew goes with its last shot, in that frame. ArriveAtTarget at a bloon times from its Position.z (20 once moved). TargetTrackOrDefaultAcidPool ported (Close-ordered in-range segments shuffled with syncedRandom.Next(i, n); points in turn, 2 up). A throw started at a point keeps it (and its height) until release. The acid-pool attack's bloon targets are points too (the bloon's Position, z less 180 boss / 120 MOAB / 20), and it takes its next track point only as a throw starts with no bloon (one per such throw). BREWS PORTED: thrown potions (ArriveAtTarget to the tower, z 0) landing on expiry (AddAcidicMixture/BerserkerBrewToProjectile.Exhausted): acid via TargetFriendly (MutatedOrder: one synced NextDouble per tower, unbrewed first; ignore list with the Village / GlueGunner rules; AcidicMixtureBuff vs PermaBuff exclusive), count maxCount (10), a re-brew adds maxCount up to cap (30); berserk via BrewTargetting (GetClosestTowerInRange, <=, later ties win; no tower already berserked); rateUp takes that fraction off the interval; a brew acts and counts down from the frame after it lands, its timer only while rounds run. ACID POOLS PORTED: AcidPoolEmission.OnEmit counts emits per tower (INFERRED), every alwaysPoolOnCount-th splash always pools; AcidPool.Process pools at lifespanFrames if alwaysMakePool or nothing hit, lasting lifespanFramesIfMisses more (002 / 003 / 502x match). Acid-pool points reshuffle on any model change, range included, even while the attack is off (052: transformation 121 / its end 1320), keeping the enumeration index; a throw at a point turns the tower as it starts; Total Transformation's morph ends lifespan + 1 frames on with fresh weapons (052 now bloon / position exact; 005 / 025 / 205 / 052 differ only in list order: simultaneous acid pops order their second children by path list, e.g. 052 rt 2577 8455 < 8600 < 8033, though b.order / path_rank as the DoT order breaks glue). A missed-frame double shot doesn't use up a brew's count (250 rt 137). The acid brew clears the Lead immunity bit of the tower's DamageModels (AddAcidicMixtureToProjectileMutator.Mutate); a brew's last frame doesn't buff that frame's shots (300 rt 799). A mutator with no exported behaviour (Rubber to Gold) is still kept (kind mark) for FilterMutatedTarget, its GiveCashOnPop cashMultiplier 1 as the mark's pop cash x2 (GiveCashOnPop.PreDegrade: pops x income x cashMultiplier + flat); Lead to Gold shows its LeadToGoldPopEffect with "${0}" of the cash (Simulation.CreateTextEffect), not the CashUp "+$"; bloon overlays come from GameData.bloonOverlayData (tools/assets/export_overlays.py: AlchemistGold -> <class>RubberToGold); RemoveBloonTypeModifier as RemoveDamageTypeModifier; lifespan -1 mutators stay (UnstableExplosion). MorphBloon ported (Bloon Master Alchemist: Damage for total health - 1 with children blocked, then the model becomes Red); a round-spawned bloon's Position.z is set in its spawn frame (120 MOAB / 20), so ArriveAtTarget at it arcs right; 005 / 025 / 205 are bloon- and position-exact, only list order differs (rt 1672: three acid pops of 2783, 2382, 2988 make their second children in path-list order 2783 < 2382, not creation order; ordering the DoT ticks by path rank broke glue). ArriveAtTarget.SetTarget on its bloon's destroy retimes an altSpeed shot from startPos to the new Target (3D, the bloon's z), startFrame kept (000_close). |
| Ace | 25/25 | EXACT (2026-11-10). Re-recorded with SimRecorder PM lines (PathMovement planes' AirUnit transform, t, onPath, takingOff, tt, pitch, lastBank, tpf; compared as kind 'plane'). PathMovement ported (Process @0x180d58bd0, FindCurrSupplier @0x180d583f0, TakeOffPath @0x180d5f100, Circle / FigureEight / CenterElipse GetPathPosition and lengths, TruncateVector2 and heading truncation, DeltaAngle / CapAngle180 / LerpAngle bank, TakeOffAnimation) in `fly_pattern`. Process order: the Attacks the tower has when its plane first moves come before its PathMovement (they fire from last frame's spot and rotation); an Attack an upgrade adds comes after it (fires from the moved plane: upgrades_r30's 1-1-0 pineapple). An upgrade re-picks the path supplier (one catch-up step). Weapons: only FireFromAirUnit ones start at the plane (Weapon.GetEjectPosition; Ground Zero at the tower), z = the plane's (0 before its first move, 250 after) + ejectZ (a z-0 Spectre dart hit a tree); non-Arc ones leave along the AirUnit rotation (EmitRotation) without a seek launch; an Arc is round 0 (or the rotation with useAirUnitRotation); ThrowMarker offsets spread across the shots turned with the rotation (Dart Storm). Pineapples: CheckAirUnitOverTrack.FilterEmission (@0x180c6df80, a targetable segment within 5), EmissionOverTime for air attacks (Emit @0x180c3a5b0 / Process @0x180c3a920, in Simulation.time, from the plane at z 250 toward TargetInFrontOfAirUnit), FallToGround (@0x180cbce30 / @0x180cbcd50). AdoraTrackTarget ported (`adora_step`: Process @0x180c84eb0, UpdateVelocity @0x180c85730, UpdateRotation @0x180c85340, Collide @0x180c84860; after the bloons move; search clock from creation; *Frames = x (1/60)). |
| Dartling | 25/25 | Re-recorded with seeds and cash 2026-11-07 (Locked at (-130, -22.9)). Weapons same in all 25. Done: RotateToPointer.Process (@0xdd6270) = SmoothDampAngle (@0xf20a60, Unity's SmoothDamp @0x183971800 in f32) toward the point raised by weaponEjectZ / tan 60, smoothTime = |RoundAngle(want - cur)| / rate, yVelocity per tower (`turn_vel`); the harness reads `# point`; EmissionWithOffsets randomRotationCone (one synced NextDouble a shot, (float)(d x cone) - cone x 0.5, the direction only); CreateProjectileOnExhaustPierce (@0xca4050 / Process @0xca48a0): where the shot is unless useBloonPosition, count / minimumTimeDifferenceInFrames per shot, not on map hits, and canCollideWithBloons = gap <= now - last + 1 (later hits in the spawning frame refused, and the next frame's); CreateProjectileOnBlockerCollide exported (map hits only); TravelStraitSlowdown.Process (@0xcf65c0): no lifespan, expires at maxDistance, slowdown / min *Frames are 0 (not serialized), so each pellet keeps its speed. Rocket Storm: TravelCurvyModel exported as straight travel, and TravelCurvy.Initialise (@0xcf5ee0) draws a syncedRandom Next() per rocket after the emission's draws (d d d n): 040 / 250 now bloon-exact. ROCKETS DO SWING (2026-11-07, recording 250 D lines): TravelCurvy.Process turns the direction 12 degrees a frame (turnRatePerFrame = 720 / 60 IS set on load) between initial +- maxTurnAngle (45), reversing at the bound (that frame's step is (rot - min) + min, i.e. it holds a frame); incrementing = (TravelCurvy.Initialise's Next() is even) (odd: first steps go down: 593 -39.2, -51.2, -63.2, -75.2, -75.2, -63.2 ...); the P line's rotation stays the initial one, only the direction (D) turns. PORTED (2026-11-07): TravelCurvy swing (`curvy`, `curvy_turn`) and the draw order (BaseEmit makes the rocket, so its Initialise Next() comes before RandomEmission's rotation draw: rotation = 4th draw x angle + tower - angle / 2, exact for all ten in 250): 040 / 050 / 052 / 250 bloon-exact; 040 / 050 / 052 / 250 MATCH: a RandomEmission with useMainAttackRotation takes the tower's stored rotation as its base and the projectile keeps the rotation it drew (not one rebuilt from radians / the direction). The initial rotations still don't fit RandomEmission with one base (250, angle 36, useMainAttackRotation): probe RandomEmission.Emit's base (main attack rotation chain) with SimRecorder. DARTLING 4xx BEAM (LineProjectileEmission @0xc43e80 with useLengthSpeed): the emission object keeps currentMagnitude across shots; Emit sets targetMagnitude = |target - start| and draws the line (Simulation.EmitLine) at currentMagnitude; Process (@0xc45bb0) moves currentMagnitude toward targetMagnitude by lengthPerFrame (6) each frame; projectileAtEnd (endProjectileSharesPierce) via emissionAtEnd at the line's end. Recording 400: each shot makes the line projectile at the eject point and the end projectile (pierce 50 both); their gaps per shot are 1.0 (rt 4, the first: currentMagnitude 0 then, so EmitLine seems to give 1 at least), 17.4848 (rt 16: = |target - start|, the beam grown past it), 7.3285, then 1.4835 for every later shot (the barrel 19.5 out reaching the Locked point as the tower turns): the end goes to the target once currentMagnitude >= targetMagnitude. The line projectile's D points back along the beam (414: (0.069, -0.998) with the tower at -3.97), the end one's forward. upgrades_r30 differs only from rt 1312, the same beam (4-1-0 bought at rt 1300). PORTED (2026-11-07, 400 / upgrades_r30 MATCH): currentMagnitude / targetMagnitude start at -1 (the 1.0 first gap is a length of -1: Simulation.EmitLine @0xb460a0 puts the end at start + (0, 1) turned by the tower's rotation x length, so it points back), Emit sets targetMagnitude = |Target - start| and draws at the current length, Process grows it by lengthPerFrame; the line collides along it at once and the end projectile is emitted at its end (`line_grow`, `line_end`, WeaponState.line_mag). Map-blocker clipping (FindBlockerIntersection) isn't ported. Open: 500 / 502 / 520 (Ray of Doom, first diff 500 rt 64: the clone's MOAB survives). Its LineProjectileEmissionModel: useTargetAsEndPoint and useTowerRotation off, useLengthSpeed on but no target length (so no growth: Emit's length is the constant 400 @0x183e5005c), ignoreBlockers off (Map.FindBlockerIntersection clips the end), projectileInitialHitModel (pierce 1, damage 140, LaserShock4 +20) and emissionAtEnd; the line projectile pierce 999 damage 30. Emit @0xc43e80: with an initial-hit model (and the line able to collide) each bloon on the line passing Projectile.IsCollisionValid gets an initial-hit projectile emitted at it and CollideBloon'd at once (H lines: 500 rt 52 shot 468 on MOAB 442 at health 60, rt 64 shot 487 at 10); the 50 between (30 + LaserShock 20) is the line projectile's own hit through the list CollideBloons the recorder doesn't log. Rotation with useTowerRotation off: the attack entity's parent rotation if any, else Vector2.get_Rotation(Target - start). TRIED (2026-11-07, reverted): 400 long along the tower's rotation + an initial-hit projectile per listed bloon gets the MOAB's first hit at rt 52 like the game (a 400 line along -45.9 passes ~24 from it: its circles reach), but the game's MOAB health goes 200 -> 60 (rt 52, initial 140 only) -> 10 (rt 64, then popped by the rt 64 initial hit): the line projectile's own 30 (+20 LaserShock) lands at rt 64 only, not with every list hit, and before that frame's initial hit. Leads: the line projectile collides normally at its own (start) position during its 3-frame life (MOAB 23.8 from the start at 64, 30.7 at 52), (LaserShock4 IS a DamageOverTimeModel, 20 every 0.95 s from 52 -> first tick ~109, so not it; the rt 64 +20 is the line's DamageModifierForBloonState on a shocked bloon). PORTED (2026-11-07): Emit's branch (@0xc43e80): with an initial-hit model and the line able to collide (Projectile.CanProjectileCollide @0xc8b920: not destroyed, canCollideWithBloons, pierce > 0) it walks the list to the FIRST bloon passing IsCollisionValid, makes the initial-hit projectile on it and CollideBloon's it, then the line CollideBloons the list after that one; the Ray of Doom draws 400 (no useTargetAsEndPoint) clipped by Map.FindBlockerIntersection (@0xf57390, `Game::blocker_clip`). 500 now exact to rt 75 (the MOAB 200 -> 60 -> popped at 52 / 64 as the game). NEXT 500 rt 76: the initial hit takes Ceramic 442 (nearest), the line then hits 506 and 515 but not 497 of three stacked Ceramics (H lines): CollideBloons' count argument is (int)param_2->0x3c - 1 (param_2 = Emit's 2nd argument: find which object / field, maybe a max-collisions count) or IsCollisionValid refuses 497. Checked (2026-11-07): param_2 is the line's ProjectileModel and +0x3c its pierce (count 998); CollideBloons/3 (@0xc8bd70) walks the list (IsCollisionValid @0xc8d840: destroyed / vtable 0x778 / FilterBloon / map LOS) and stops at the first CollideBloon that returns false. The clone's line hits all three stacked Ceramics; the game only two: add a SimRecorder probe logging the GetInLine list (ids in order) for the Ray of Doom's Emit. Keeping list collisions for both broke 500 / 502 / 520 further; growing the RoD line (target length) kept upgrades_r30 matching, so check whether 4-x-x -> 5-x-x keeps the emission state. NEXT (040 rt 185, 250 rt 125): the rockets' rotations don't fit RandomEmission's d x angle + base - angle / 2 with one base (250 rt 124: -39.2, -29.3, -55.5 from tower -41.32); RandomEmission.Emit (@0xc4a7a0) with useMainAttackRotation takes the main attack entity's rotation (+0x48 -> +0x60 -> +0x58 / +0x50 chain), and TravelCurvy.Process may swing them if turnRatePerFrame is set on load (check: the swing is +-maxTurnAngle 45 at turnRate 720 / 60). 004 / 025 / 205 MATCH (2026-11-07): RateBasedAnimationOffset.Initialise (@0xc908d0) sets the weapon's animationOffsetFrames = (int)(rate / baseRate x offset x 60) (the 0-0-4 guns leave 0 / 18 / 36 / 54 frames after their shared throw), and a RotateToPointer with rotateTower off turns its own attack from 0 (`aim_rots`), the gun's shots along it. 005 MATCHES: KnockbackMutator.Mutate (@0x1811e7110) picks light / heavy / MOAB by the bloon's current model, so a knocked Ceramic's children go back at the light multiplier (`Knock { m: [light, heavy, moab] }`). DONE 2026-11-08 (25/25): the Ray of Doom's Emit (@0xc43e80) passes CollideBloons OrderBy(GetInLine).Skip(i + 1), all lazy (SimRecorder CB lines: an Enumerable.Skip iterator over an OrderedEnumerable over GetInLine's iterator). Count / ElementAt find the first bloon passing the line projectile's IsCollisionValid (index i; none, no initial hit: a Camo Rainbow the 5-0-0 can't see). The initial-hit projectile is made and CollideBloon'd, and only then is the line's list walked: GetInLine again, so a bloon the initial hit destroyed is gone and Skip skips the next one (500 rt 76: three stacked Ceramics, the line on two). |
| Buccaneer 34/34 (re-recorded with seeds 2026-11-07), Mortar 25/25, Druid 34/34 (2026-11-09), Banana Farm 25/25 (2026-11-10) | | |
| Super Monkey | 34/34 | EXACT (2026-11-12): super_upgrades_r30 matches. Projectile.CollideBloon (VERIFIED-DECOMP @0x180c8b970) runs the projectile's behaviours in model order: the Super Monkey blasts' chance-0 WindModel is listed before the DamageModel, so Wind.Collide rolls on the bloon before the hit (rstack: the roll of 4287's hit that pops Red 4150); exporter `wind.first`, clone `pre_roll` / `wind_valid`. Earlier: DONE (2026-11-10): the Robo hands split (TargetRightHand <GetTargets>d__9.MoveNext @0x180df3eb0: A for the right hand's type, B for TargetLeftHand's (ctor @0x180de7b70: strong), m = Normalize((A - T) + (B - T)) + T, A stays right when (A.y - T.y)(m.x - T.x) - (m.y - T.y)(A.x - T.x) >= 0, else swapped; both must be valid or neither fires; `left_hand`), and an arm turned with rotateTower off stores its rotation relative to the tower and ejects through the tower's child transform (super_030 rt 973): 050 / 052 now match. FOUND (2026-11-11, not fixed yet): super_032's 5 position frames (rt 352-354 dart 2662, rt 352 / 364-365 dart 2953) are the RandomArcEmission angle 1 ulp low: game rotation 25.528013, clone 25.528011 (recording P/D lines; with the game's angle, D = Vector2.Rotate((0,1)) and v = D.x x 350/60 = 2.5138888 reproduce every game x bit for bit, the clone's 2.5138886 drifts only from rt 352). TravelStrait.Process is pos + velocity x elapsed (same as the clone). GetDirection (@0x180c4a000) sums (((r - angle/2) + offset) + k x slice + (float)NextDouble x randomAngle) - randomAngle x 0.5 in the clone's order, so suspect r (read from the arm entity's transform, +0x60 -> +0x10, i.e. the arm's stored rotation after RotateToMiddleOfTargets / RotateToTarget; dart made rt 319) or the draw: print r / rarc / sum in emit_native's RandomArc arm for rt 319. Back-solved from the recording's draws (rt 318 block: arm A Emit 0.8926 / 0.8378, dir 0.20800033; arm B Emit 0.02195 / 0.50166, dir 0.67234113): the game's arm rotation is exactly 27.86401 (arm A) or 24.149284 (arm B), the only f32 that sums to 25.528013; the clone's r is so 1 ulp low, most likely in the body + (arm - body) composition from 8c12257. Suspect: the clone composes with ONE body rotation (tr0 + (g - tr0), at both the throw-start store ~14323 and the release store ~14620), but the game stores g - body when the arm turns and adds the body back when GetDirection reads the transform; if RotateToMiddleOfTargets moves the body in between (store B1, read B2), the game's r is B2 + (g - B1). Test: print tr0 at the store and game_rot at the emission for rt 319 and see whether B2 + (g - B1) gives 27.86401 / 24.149284. Earlier: 030 21 / 032 7 position frames (rt 973: still 1 ulp in x after GetEjectPosition's single Vector2.Rotate by the arm's stored rotation, VERIFIED-DECOMP @0x180c4f390; next suspect the add order onto the tower position or the first move), upgrades_r30 rt 1572. SUN TEMPLE (2026-11-10): a tower made at a temple tier now runs temple_upgraded (OnTowerCreated -> ActOnSacrifice), mute reset to the harness's frame-adjusted weaponDelay (super_400 bloon frames 1691 -> 1440). Still: the game fires ~25 weapons (3..24) and a second entity ('tower 1', 124 throws from rt 226) with NOTHING sacrificed; the clone none. Not the CheckTempleUnderLevel mutators: Evaluate (@0x180c1ce10) = templeId matches && MonkeyTemple.ContainsTowerSet(set) && GetTowerSetWorth(set) < cost, false with no sacrifice. FOUND (2026-11-10): a temple MADE at its tier presets its worth (MonkeyTemple.OnTowerCreated / OnInstaPlacement @0x180d405c0: templeTowers = {1, 2, 4: 40001 (@0x183e501d4), 8: 0}; tier 5 trueTempleTowers {1, 2, 4, 8: 40001}; SetTempleSacrificeValuesFromModel @0x180d42d60 applies a group when its set is selected and worth >= (int)cost): `temple_created` (super_400 bloon frames 1440 -> 822, weapon 0 131 vs 129 throws). DONE (2026-11-10, later): the temple's mutators are applied one by one in the game's order. MonkeyTemple inserts each applied group's mutators (model order) into mutatorsToApply with a sorted insert (@0x181ffcdb0, recursive binary search @0x18014ca90 by SortMutators: priority descending, an equal priority goes in AT the probed midpoint), then OnTowerCreated mutates the model in that order (MutateFromCache). Simulated, this gives exactly the probe's weapon order (missiles v2, v1, plane, storms, arcane v3/v2/v1, missiles v3, glaives v3, blade, glaives v2/v1). Consequences now in temple_tier: the priority-20 stat mutators touch only the temple's own attacks (the sacrifice weapons fire at their raw rates: 30/48/78/120/300); Range (1) runs last over all. Mutate bodies VERIFIED-DECOMP: Reload @0x180c225f0 rate - rate*m; Speed @0x180c22830 speed*m + speed (compounds: 1.12*1.24*1.36); Size @0x180c21e70 radius*m + radius (assetSizeModifier on scale); Pierce @0x180c22180 + (int); Range @0x180c22a70 tower and attack range + v. Stat mutators carry their own CheckTempleUnderLevel too, so the exporter writes every mutator (TempleGroup.muts: priority, kind, value, attack index, condition, supports) and the upgrade path drops each failed one. The missiles never firing is just AttackMinimumRange 100 + MOAB-only filter (the MOAB sat ~40 from the tower). super_400 first position diff rt 225 (many projectiles) -> 226 (the plane's first shot only). DONE (2026-11-10, Ace exact): the PlaneSpawner sub-tower flies the Ace PathMovement port (its CirclePattern isn't selectable; a plane whose target type names no pattern flies its first). UseParentEject (EmitPosition @0x180c978e0): a weapon ejects from the tower's first Attack's first Weapon (the Arcane Blasts from the main blast, turned by its stored rotation, z 21.58). A SingleEmmisionTowardsTarget shot isn't seek-launched (its offset stands: Arcane Blasts +-40 / 80 / 120). FollowPath + TravelStrait (glaives): each frame's path point gets one TravelStrait step on top, TravelStrait's lifespan holds; FollowPath advances speed x (1/60) (1 ulp; boomerang still 99/99). Probe super_400: every weapon's throws now match. DONE: a carried parent's children are all made at its z (the blown MOAB's four Ceramics at 120, so they drift back in 3D at the game's speed); not for an uncarried parent (druid_500 rt 138). DONE: MoveWithWind.Attatched (VERIFIED-DECOMP @0x180ffec70): a distanceMax over 99999 is the bloon's distanceTraveled (the blown MOAB 54.31 back; super_400 0 bloon frames). DONE: AlternateProjectile.GetEmission (VERIFIED-DECOMP @0x180c69170): an alternate without its own emissionModel uses the weapon's (the plane's bombs go at their target). DONE: super_400 MATCHES. A towards-target shot whose target died in the throw aims at the Target's cached spot (emit_aim; a glaive along -67.23). TravelStraitModel.set_Lifespan (VERIFIED-DECOMP @0x181202b90): lifespanFrames = (int)(lifespan x 60 + 0.5), half up (a 0.875 s plane shot moves 53 frames); AgeModel's stays half to even. DONE: RetargetOnContact.Retarget (VERIFIED-DECOMP @0x180cd8170) on a FollowPath shot sets its Direction and Rotation; FollowPath.ProcessFrame lays the path from the start point turned by the rotation, so the glaive is re-laid along the new heading the frame it bounces (the game retargets before the frame's path move). NEXT: super_420_close rt 231: glaive 1031 (pierce 49) hits 11 of 12 Zebras in the game, skipping 1101 (same lineage 1106,1049,747 as 1092, which it hits) and hitting 1111..1148 twice at pierce 39; the clone pops all 12: collision candidate order / grid cells. Detail (2026-11-12): all Ceramics and their Zebra children sit at the entrance clamp (-73.98, dist 173.6); the hits on Zebras do nothing (0), so nothing degrades after the Ceramics; the visit order [1043, 1053, 1063, 1083, 1092, 1111..1148, 1111..1148 again, 1157] skipping 1101 (child k = 3 of 1043, spawned 20 behind) fits candidates gathered cell by cell from the BloonGrid, a bloon filed in two queried cells visited twice and 1101 filed (by its spawn spot, before the entrance hold) in a cell the glaive doesn't query. RESOLVED 2026-11-12: SimRecorder can now replay a game's seed (scenario "seed": SeedPatch sets GameModel.randomSeed before Simulation.Initialise @0x180b48de0); super_420_close / last / strong re-recorded with their own seeds all match (the old ones, object ids 9 apart, in pending/super_old/*_unseeded.rec), so that run's difference wasn't reproducible. super_030 / 032 / upgrades_r30 re-recorded with their seeds reproduce their differences exactly: real. DONE: RotateToMiddleOfTargets.ApplyRotation (@0x180dd5530) sets the tower's rotation to Rotation(sum of Normalized(Target.Position - tower) over the hands' attacks with a target); the arms store theirs relative to it (super_030 exact, super_032 rt 301). DONE 2026-11-12 (SimRecorder AT lines: each Attack's Target per frame): the body is set once, in the first hand's attack at its release, from its new Target and the other hand's still-held one at their frame-start positions; super_032 exact. (Old analysis: the game applies it only while a weapon is in a throw (model +0x28), with each hand's current Target, and before the arms compose (352: arm 0 with the frame's final 6.6377); the clone takes the last stored aims as each attack sets its aim (363: 50.63 vs the game's 50.11).) DONE (5xx): AttackMinimumRange applies to carried (off-track) targets too (no missiles at a MOAB blown back 71 from the temple); SubTowerFilter's createdProjectiles is per weapon (the True Sun God's second plane spawner isn't held by the first's projectile). super_500 re-recorded with WN / PM lines as pending/super_probe/super_500.rec: weapons all match. DONE: RangeSupport's lifespan step lands on the attack's first TravelStrait descendant: with an AlternateProjectile on the first weapon, the alternate's (the True planes' bombs 100 -> 120 frames, darts keep 53). super_500 / 520 match. DONE: a carried bloon moves at its speedFrames without Knockback's slowing (super_502 rt 289: knocked at 287, carried at the full 0.5208; a freeze still holds it, druid_400); super_502_* match. NEXT: super_upgrades_r30 (2026-11-12, SimRecorder rstack: synced draws with their native callers): the extra early draws (0.2430 at rt 1550, 0.1273 at 1551) come from Wind.Collide (@0x180cf7fd0) called by Projectile.CollideBloon (@0x180c8b970) during the main blast's repeated no-pierce contacts with Reds 4305 / 4150 (H lines with pierce unchanged every frame from 1546; rolls only from 1550, logged after that frame's H lines, so possibly a CollideBloon whose postfix didn't run); the clone never calls the wind code for such contacts. Earlier note: rt 1551: the 4-1-0's main blast (RandomArc, randomAngle 8) leaves along 70.65 in the game, 73.07 in the clone. The game's angle is exactly the clone's next-frame collision-pass draw (wind roll, 0.07544 from T = 74.0457): in the game the two wind rolls of projectile 4262's hits on Black 4158 / White 4166 (main blasts carry WindModel chance 0, a roll per valid hit) come BEFORE the tower's rt-1551 emission, in the clone a step later. Positions at frame 1550 are 22.4 apart (radius 9), at 1551 14.2: find why the game collides there before the throw (weapon animation 1: a deferred release?). Then super_420_* rt 231 (glaive 1031 skips Zebra 1101), super_030 / 032 (1 ulp), upgrades_r30 rt 1572. Then super_420_* rt 231, super_5xx (main weapon 246 vs 155 throws). Earlier: (2026-11-10, later) CritMultiplier ported (Initialise / GetProjectile @0x180c71590 / 0x180c71010: nextCritAt = syncedRandom.Next(lower, upper + 1) when the weapon is made, its ability attacks' weapons right after the tower's own (`temp_crit`), and after each crit). DistributeToChildrenBloonModifier (Collide @0x180cb9360): the hit bloon's own layer passes the overkill on (Tech Terror's blast through a MOAB's Ceramics). A Sun Temple made at its tier holds fire weaponDelay frames from Simulation.time 0 (MonkeyTemple.CanFire @0x180d3f010; the harness offsets by the recording's first frame). OPEN (NEXT): Robo / Tech Terror / Anti-Bloon hands: TargetRightHand's <GetTargets>d__9.MoveNext (@0x180df3eb0) takes its own type's target A and the left hand's type's target B, m = Normalize((A - T) + (B - T)) + T, and if (A.y - T.y)(m.x - T.x) - (m.y - T.y)(A.x - T.x) >= 0 the right hand gets A and the left B, else swapped (super_050 rt 118: left -46.54, right -64.5; the clone gives both the same target); super_040 rt 189 the right arm's eject; 030 21 position frames; 050 2 bloon frames; 4xx / 5xx: the temple's other weapons (3, 4) never fire in the clone. Earlier: | Re-recorded 2026-11-10. RandomArcEmission ported (Emit @0x180c49d20: two synced draws, an offset the start doesn't use; GetDirection @0x180c4a000: (rotation - angle / 2) + offset + k x slice + NextDouble x randomAngle - randomAngle / 2). Darkshift (StartDarkshift / Process / EndDarkshift @0x180e22bd0 / 0x180e225e0): the tower moves towerSpawnDelay frames after the activation, after that frame's attacks; a point-less Activate (the recorder's) goes to (0, 0). Knockback isn't applied by the hit that pops a MOAB-class bloon (INFERRED; super_004 rt 128). RetargetOnContact behind a TravelStrait in the behaviour list re-aims after the frame's move (super_004 rt 139). rotateTower-off attacks of a tower with RotateToMiddleOfTargets eject along their own aim (INFERRED scope). NEXT super_040 rt 74: the Tech Terror arms aim differently in the clone (game: both at -68.67, the shared target); then 030/032 (Robo), 050/052, 205/250, 300/320, 400-520 (Sun Avatar+), upgrades_r30. |
| Spike Factory | 25/25 | Re-recorded 2026-11-10 (old in pending/spike_old). checkCollisionInterval's timer runs only while canCollideWithBloons (Projectile.HasValidBloonCollision @0x180cabcf0); a CreateProjectileOnExhaustPierce with no gap turns canCollide back on every frame, so Super Mines piles collide in flight from their second frame. RandomRotationWeaponBehavior: one synced draw per shot at ProjectilesCreated, once for a weapon's missed-frame shots. One TrackTargetSupplier per TargetTrack attack (`Tower.track_sets`); ability attacks' lists are made after the tower's own first attack and kept between activations. StartOfRoundRateBuff on/off remakes the points (a round start only refreshes it). ActivateAbilityAfterInterval.Process (@0x180ce2b60): round time, fires when more than intervalFrames since the last try (first at once), Activate(true) (no cooldown; IsReady refuses while active, Ability.get_IsActive); its attacks act that frame and count their lifespan from the next. AgeRandom: Next(min, max) frames per shot. Ability weapons keep missedRate between activations (`temp_missed`, reset at round start). OPEN 052: the first storm fires on the placement frame (rt 1) where 050's fires at 2; 052 also has StartOfRoundRateBuff, whose mutator goes on in that frame. Both towers' weapon 0 fires at 1, so the tower does process at 1: ActivateAbilityAfterInterval.Process (@0x180ce2b60) only gates on Ability.CanUseAbility (@0x180df8db0: a count at f42080 == 0 and every behaviour of interface 0x184f6ffe8 on the tower's entity allowing it, vtable +0x8e8) and the interval, so in 050 one of those blocks at rt 1 and in 052 the model update clears it. The behaviours are the ability's IAbilityBehaviors (AbilityBehavior.CanUseAbility, slot 123): for Spike Storm ActivateAttack.CanUseAbility (@0x180e029b0): refused while active (slot 124); with rounds not active, only with model +0x71 and a weapon check. So at 050's rt 1 the round is likely not active yet as the tower processes; in 052 the round-start mutator's model update (StartOfRoundRateBuff.OnRoundStartSim) must let it through: check the round-start event order against Tower.UpdatedModel next. PORTED 2026-11-12: cancelIfNoTargets wants a tower attack with fireWithoutTarget or Attack.CanHaveTarget (@0x180dcb260: its activeTargetSupplier, set up at its first Process or a model update); with StartOfRoundRateBuff going on in the placement frame the storm goes off ahead of the tower's attack: Spike Storm's four lists made at Activate, its shots, the tower's list and shot, then the four lists again with the cursor carried (spike_052 exact to rt 129). Then: StartOfRoundRateBuff going on at each round start remakes the ability attacks' lists (cursor carried) before the tower acts (rt 129), coming off remakes only the tower's own (rt 308); the early storm's spikes start ageing next frame. spike_052 EXACT. |
| Monkey Village | 25/25 | EXACT (2026-11-10, re-recorded; old ones in pending/village_old). RangeSupport.MutatorTower (@0x180d795b0) also adds the model's TravelStrait lifespan x multiplier (or additive / speedFrames / 60) to each attack's first TravelStrait (both kinds add; ability range zones use the plain Mutator @0x180d79db0, range only: `Support.range_life`). Call to Arms (CallToArms.Activate @0x180e19600, CallToArmsMutator @0x181274ad0): global unless useRadius, never the village, mutates at once (no next-frame delay, `Support.instant`), rate / multiplier and pierce x multiplier on every projectile (`Buff.pierce_x`). Grow Blocker zones test the bloon's frame-start position. Monkeyopolis: cratesPerRound and (max(worth / valueRequired, 1) x cashFromCrate + baseIncome) / cratesPerRound per crate; STAND-IN: farms in range aren't absorbed yet (needs a recording with farms). RotateToTarget.ApplyRotation (@0x180dd7930): useThrowMarkerHeight aims from the tower raised by the weapons' highest ejectZ; rotateTower off turns only the attack (`Tower.ent_rots`), the shot ejecting along it, re-aimed at the release. SingleEmmisionTowardsTarget.GetDirection (@0x180c4d0f0): normalized(Target - eject point). A tower made at a tier takes its model's first TargetTypes entry (Tower.UpdateTargetType @0x180c19880; the 5-x-x ballista lists Last first). |
| Heli | 26/26 EXACT | Re-recorded 2026-11-11 with SimRecorder HM (heliTransformBehavior) and IN (the tower owner's InputManager cursor, active, ignore) lines; the harness compares HM positions as kind 2 and feeds IN as `Game.cursor`. PORTED (VERIFIED-DECOMP/DISASM): FollowTouchSetting.Process (@0x180db8c70: currentPosition = cursor while active, not ignored and IsPointOnMap; starts at the heli's spot), HeliMovement.Process destination switch (@0x180d2c950), ApplyMovement (@0x180d29c30), GetPointWithinMap (+-150 / +-116), UpdateRotation (@0x180d2d460: no turn within strafeDistance or while a RotateToTargetAirUnit applied; LerpAngle by rotationSpeed toward atan2(dir.x, dir.y)), RotateToTargetAirUnit.ApplyRotation (@0x180dd6e00: snaps to Rotation(target - heli) before the heli moves; recording heli_000 rt 14 -84.26471). Order: the heli's attacks, then its movement (move_helis after towers_phase). A Python f32 check of the movement matches heli_000-003/030/032 bit for bit over 3600 frames. FireFromAirUnit for helis: shots from the heli (z 250 once it has moved) along its rotation, the EmissionWithOffsets markers turned with it (recording heli_000 rt 14). Also: the heli transform's z is 250 from placement (heli_032/500); RotateToTargetAirUnit aims at Target.Position (a MOAB's targeted circle, heli_032) and on cooldown frames the attack keeps its target, the heli still turning to where it is now (heli_400 rt 388, heli_500 rt 152); TargetFirst/Last/StrongAirUnit go through BloonManager.GetTarget -> Targetting.GetTargets (MOAB centre, heli_400); AngleToMapCenter (@0x180c69c90) for the Support Chinook crates, synced Range(0, 1000) each (INFERRED P-CRATEROT: from rotation 0). MoabShoveZone.Process (@0x180d3bb00) ported: every 30 roundTime frames BloonManager.GetTarget First from the heli with IsBloonValid (@0x180d3b7b0: MOAB-class, filters, not another zone's, dot(heading, bloon - heli) <= 0, PercThroughMap >= 0.01) gets MoabShoveMutator (@0x18121ff50: speed x moab/bfb/zomg cap by baseId) for 30 frames of movement (heli_004 rt 99-128). Comanche Defence (not immediate) PORTED, heli_004 exact: decided in the tower's turn (bloon positions as the frame began), helis made after the towers act; Process gets elapsed (1), so they never retreat and only OnRoundStarted clears the flags; the minis stay locked where made, facing Rotation(-spot). Comanche Commander PORTED (heli_005/025/205 exact): its minis are made at the tower in the tower's turn (before its HeliMovement; they act and move from the next frame) in FollowTouch from there; HeliGroupOffset (CalculateAvailableOffsets @0x18102dbf0: (0,0), then rings of 15 r) gives every heli of the target type its slot in tower order; heliPositionsInRepulseRange (INFERRED P-HELIREPULSE, fitted exactly to heli_005 rt 2-40 by brute force over the lists): each heli, after an ApplyMovement, lists the others now within range (enter order kept), in use from its next move. RotateToTargetAirUnit turns first to the attack's held Target (alive, any range), then to a new one its search finds (heli_025 rt 2357). PursuitSetting.Attatched sets the target type Pursuit (an upgrade into 2-x-x switches). NEXT heli_upgrades_r30 rt 615: the 2-1-0 (Pursuit, weapons new: W 0/1 lastEmission int.MinValue) shoots a Lead 48 from the heli (attack range 42) a frame before the clone: in Pursuit the attack's target supplier is the PursuitSetting (PathSupplierHeli.GetTargets @0x180dcd9c0 -> Targets(attack.range) -> BloonManager.GetTargets from the heli, the static TargetType at +0x18); FOUND 2026-11-12: its turn -147.27158 = Rotation(the Lead after its rt 615 move - the heli after its 615 move), so the weapons act after the bloons AND the heli have moved; an upgrade that changes a kept Attack's weapon count rebuilds all its Weapons (recording: at rt 200 the 1-0-0 resets w0 = 59 to int.MinValue too), new objects at the end of the process list. DONE: the w0 reset is ResetRateOnInitialise (VERIFIED-DECOMP @0x180c90a80, ported); a heli's own HeliMovement now runs before its young weapons act. DONE too: a heli's young weapon (ready) moves the heli, then the bloons, then does its own FindTarget (fresh, First from the heli) and turns the heli to it (rt 615, 649, 1193); a heli's Attack made after it (3-1-0 Rotor Blades) acts after its HeliMovement (rt 901). The young weapon's own search is final (none: no shot, rt 1638), and the weapons are listed with the attacks in the order they were made (the 4-1-0's missiles after the Rotor Blades): heli_upgrades_r30 EXACT. Wind-carried copies: spaced k x min(26.7 / count, 8.6) behind the blown parent over the final child list (a multi-layer pop flat), then each its own MoveWithWind draw (heli_040 / heli_250 exact). DONE 2026-11-12: heli_050 re-recorded (seed pinned, with AT lines) matches; the old run (pending/heli_old/heli_050_unseeded.rec) can't be replayed (the heli follows the mouse, which differs per run). Old note: heli_050 rt 238 (checked 2026-11-12: Target.IsValid @0x180c09350 keeps a degraded bloon valid (same object, same id), and taking the degraded / new bloons off the segment lists changed nothing, so the clone and the game likely hold different bloons since an earlier throw, stacked at one spot until 238): the Marine (made at 123, so before these bloons) fires after this frame's collisions (Black 1481 degraded to Yellow, children 1894/1904/1914 made) and takes a bloon at -131.437, not 1481 (first in Path.bloonsList, at -132.17); all are held at 173.6 on one segment. Likely the segment lists' own order (a degraded bloon re-filed at the end of its PathSegment list) - check with a SimRecorder dump of the segment lists. heli_040/250: Downdraft wind children; heli_050 rt 238: Marine target at round end. Earlier notes, from the decomp in research/dumps/decomp/ComancheDefence: Process (@0x180d0bdf0: not immediate: activated25/50/75 by GetGlobalBest First's PercThroughMap >= 0.25/0.5/0.75 -> GetMeDaChoppas(1/2/3, its position), activationTime = roundTime; after durationFrames every heli Retreat; HasRetreated (DistanceSquared to its startingPosition < const @0x183e50040) destroys it; cooldownFrames after activation clears all three flags; OnRoundStarted clears them; immediate: keep reinforcementCount helis made at the tower), GetPositionOnEdgeOfMapClosestToo (@0x180d0b600: nearest of the four world edges, pushed 50 past it), CreateDefenceHeli (@0x180d0a950: TowerManager.CreateTower, target type, LockInPlace at its start, PatrolPoints default path, heli rotation = Rotation(start - ...)), Retreat (LockInPlace target type), CancelRetreat (Pursuit). PursuitSetting PORTED (recording heli_200_pursuit exact; Path.DistanceToPoint is the path's own line, 4 above where bloons are drawn on Tutorial): GetPath = lastDestination = FinalTweak(GetPursuitDestination(GetPursuitTarget())) (FinalTweak is identity), path [lastDestination]; GetPursuitTarget (@0x180dd1c40) = PathSupplierHeli.Targets(range 99999) (@0x180dcda20: BloonManager.GetTargets(range, the static First type, the heli's position, count 999999999, the attack's BloonFilter, cantTargetMoab/Camo)).FirstOrDefault; GetPursuitDestination (@0x180dd1730): no valid Target (or destroyed) -> lastDestination; else HeliGroupOffset.Update, heli lastGroupOffset = GetAvailableOffset, off = Bloon.radius (+0x150, from BloonModel.radius) x 0.5 + pursuitDistance (model +0x34), d = distanceTraveled + off; 0 <= d <= path.totalPathLength (@0x180f6f920): p = Path.DistanceToPoint(d) else p = bloon position + Bloon.Heading (@0x180fb8b10: (0,1) Vector2.Rotate by the path point's rotation) x off; IsPointOnMap(p) -> p, else Map.WrapLineAroundMapEdge(bloon position, p) (@0x180f61540: Math.FindIntersectionLineLine with each world edge, then WrapLineAroundMapEdgeIntersect @0x180f613f0 carries the rest of the length along the edge toward the nearer angle) -> it, else lastDestination. OnSwitchedTo sets lastDestination = the AirUnit's position. HeliMovement's Pursuit branch adds lastGroupOffset and GetPointWithinMap. Then heli_040 rt 2310 / heli_250 rt 2413 (wind children: Downdraft), upgrades_r30. heli_050/052 (Special Poperations): the Marine is made by the Spawner attack's CreateTowerModel positionAtTarget at the UsePresetTarget point (UsePresetTarget.SetTarget @0x180e0c390: the player's pick); recording heli_050 has it at (-92.96, -28.49), 20 from the heli, the clone at the tower: the harness takes it from the first tower made within 10 frames after the abilities; CreateTowerModel.useProjectileRotation off keeps the Marine's own rotation. heli_050 left: rt 238, the Marine's last shot of the round at a Yellow just made in a bunched group (target pick). OPEN: Pursuit (STAND-IN P-HELIPURSUIT), PatrolPoints, HeliGroupOffset (Comanche groups), repulsion list membership (INFERRED P-HELIREPULSE: HeliEnter/HeliExit are delegates), canMoveOutsideScreen / ignoreRepulsion flags, the heli step inside each tower's processing rather than after all towers. |
| Paragons (pending/paragon) | 12/12 (wizard, ice, tack, bomb, ninja, spike, druid, sub, bucc, ace, engineer, sniper exact) | 2026-11-11: 10 more paragon recordings (tools/mod/scenarios/*_paragon_r40.json: three 5-0-0s, 0-0-5, 0-5-0, upgrade to paragon at frame 60). BOMB: CollisionChecker.Process (@0x180f31590) runs before the projectile lifespan Process, so a projectile at frames_left 0 still hits map blockers (rt 766). NINJA exact: supports go on in process order (a support tower before a tower placed the same frame reaches it on its first frame, its own after its first attack: one Caltrops list); a sacrificed tower does not process in its last frame (no W/AT lines at rt 61, only its sub-towers act); ProjectileOverTime.OnDestroy (@0x181004b20) with emitOnDestroy emits once more when the bloon object goes (MOAB-class pop / no children) or the mutator ends; Tower.AddMutator (@0x180c0a530) refuses every mutator on a Paragon or its sub-towers except artifact ones and AddParagonMutator (Support.AddMutator uses it only for onlyAffectParagon), so no Shinobi Tactics / Village buffs on paragons. SPIKE exact: an ability's TargetTrack lists are made in their tower's turn after its attacks (9, 25, 128x4, then the 0-0-5's); SpikeParagonDamageZone is SpiritOfTheForest with one zone (GeneratePositions / Update / AddObjectAtPosition the same; GetRange @0x180cb58b0 = currentGeneratedRadius) plus its DamageModifierForTag mods, Initialised at the purchase (184 draws at rt 61); a zone made after bloons runs after their Process (spirit_born, like the boomerang paragon orbit); the zone's ring growth runs in the tower's turn after its attacks; TargetTrack points are remade at the Paragon purchase and the next frame, under the current attack range; CreateProjectilesOnTrackOnExhaustFraction.Initialise shuffles the segments within range of (0,0) (the shot's Target isn't set yet: 22 every time), and at each round end every live pile reshuffles round its position (INFERRED: 25/12/27/12 at rt 2845, no MiniSpikes). DRUID exact: a zone made by a purchase doesn't run on its creation frame (the MOAB 47.97 away at rt 61 enters the 48 zone at 62); RandomPosition.Initialise shuffles at the purchase, after the vines' 184 positions (its 1771 Next(count) draws aren't in the R lines; the recording's first 184 placement draws aren't either). NOT YET MODELLED: DamageModifierForCashAmount on the vines' DoT (bloons die either way in this recording). SNIPER open: a bloon whose Process ran ahead of a newer attack (process_bloons_before) pops from where it is now (rt 181, the Paragon's shot after the MOAB's move); NEXT: TrackTargetAfterDelay (the shrapnel, no TravelStrait; decomp in research/dumps/decomp/TrackTargetAfterDelay): Initialise velocity 0, lastTargetSearch = roundTime, random = new System.Random(emmisionIndex + Simulation.time); OnEmit currentVelocity = maximumSpeedFrames, movementDirection = the shot's direction, Target cleared; Process: after delayFrames GetNewTarget (Random type: Next(0,4) -> First / Last / Close / Strong, BloonManager.GetTarget), moves (UpdateVelocity: accelerate unless the target is over startDecelerating degrees off, clamp min / max speed frames, turn up to rotation degrees, expire after lifespanFrames) while roundTime - lastSearch < 5, else retargets if invalid. PORTED (2026-11-12, uncommitted work of the previous session committed): AdoraSpec.after_delay / target_type, Projectile.tad (completedIntialDelay + System.Random(emmisionIndex + time)), FilterCollision until the delay, GetNewTarget clearing collidedWith and taking a Random type through typed_target_from; the shrapnel's path is exact to rt 226. Still first off at rt 227: shrapnel 684 (born 181) takes Ceramic 513 from health 10 to a Rainbow at 1 (H's third field is the bloon's health after the hit: 10 damage), while 674 a frame later takes the Rainbow to a Green (5 damage, the model's). The clone's 684 does 5. No R lines, no crit on the weapon, degree too low for +5; the Explosion child (damage 500, no filter) touches the Ceramic at 182-187 without damage (health stays 10). In frame 227 the order is "M deplete 513 1", "M copy 727", then "X expire 684" (stack through TrackTargetAfterDelay.Collide @0x180cef9b0: it Expires the shot when the bloon it collides with is its own Target; the shrapnel's damage behaviours run first), then the H line; nothing else touches 513 that frame and the weapon has no crit. A Harmony prefix on Bloon.Damage (13 params: float, object, 3 bytes, object, two BloonProperties, 4 bytes, object) does NOT work: the Il2CppInterop trampoline throws (234 NullReferenceExceptions in Memmove per run, no DG lines, the run's sim suspect), with __args or typed params alike; such a probe was removed and its recordings deleted. Established: the paragon degree is 1 (as the other 9 paragons), 613 is the main shot and 634-684 its six shrapnel (same model); 684's hit leaves the Ceramic a Rainbow at health 1 (10 damage) and 674's leaves a Rainbow a Green (5, or the excess spread over its children). SimRecorder now writes "HP proj bloon health-before bloonModel projectileModel" before each CollideBloon (works, no interop errors); probe recording pending/paragon_probe/sniper_paragon_r40.rec (same first difference). It shows: the single CollideBloon of shrapnel 684 takes Ceramic 513 from 10 to a Rainbow at 1, with no nested collision (the Explosion made on contact logs no HP/H of its own); shrapnel hits never hurt Blacks (1 -> 1 at rt 627-635, 653, 660), i.e. their damage is explosion-type although the Shrapnel's DamageModel has immuneBloonProperties 0; the main shot 613 hits the Ceramic at 182-187 for 0. SOLVED: the paragon's main shot has AddBehaviorToSingleBloonModel "SniperParagon:Inisghts" (collisionPass 1, layers 1, isUnique) with IncreaseDamageFromAllTypesModel amount 5 (GetModifiedDamage @0x180ff8470: damage + amount unless the bloon has armour): the MOAB popped by the shot at 181 leaves the mark on the Ceramic it became, so shrapnel 684's 5 is 10; the pop spends the layer, so 674's next hit is 5. Exported as a brittle effect after damage; sniper_paragon_r40 now to rt 228 (positions) / 233 (bloons). NOT YET: the "single" part (AddBehaviorToSingleBloon.Collide @0x180c81f70 builds the mutation id from the tower / projectile and may move the mark off the previous bloon) and the armour exception. FIXED: the shrapnel's System.Random is seeded with Simulation.time alone (emmisionIndex isn't set for an on-contact emission: all six of a hit draw the same target types, rt 193 / 228 all on 513 then all on 794); TrackTargetAfterDelay.FilterCollision (@0x180cefa30) after the delay lets through only its Target while that is valid (anything when it isn't; rt 233, shrapnel heading for 794 passes a Green). sniper_paragon_r40 now to rt 973 (positions) / 1004 (bloons). rt 973: the paragon's two shots at 941 make 12 shrapnel (1448-1558); at 953 (delay done) eleven take Black 945 (the furthest and the nearest: First / Close / Strong agree), only 1558 takes Last (1420), while the clone's 1558 takes 945 too. With seed = Simulation.time (957) Random.Next(0,4) gives 2 (Close) for all (the clone's DotNetRandom checked against a reference: 196 -> 0, 957 -> 2, 960 / 962 / 941 -> 1); no per-index seed (957 + k) fits eleven non-Last and one Last. GetNewTarget's mapping is 0 First, 1 +0x30 (Last), 2 +0x48 (Close), 3 +0x78. SimRecorder "TI proj emmisionIndex time roundTime" (TrackTargetAfterDelay.Initialise postfix) in pending/paragon_probe/sniper_paragon_r40_ti.rec: emmisionIndex is 0 for every shrapnel and the seed time is the F line's time of the frame they are made, so a batch of six shares one Random sequence (as the clone now does); the old recording's "12 at 941" were two batches (932 / 941). In the _ti recording (first off at rt 977, same kind): batch 941 (1460-1510): 1470-1490 take Black 935 at 957, but 1460 has no Target at 957 (PT -1) and takes Black 1340 (277.66, mid-track) at 958; the clone's twin (uid39) takes First -> 427 at 958. So the game's 1460 found nothing on its first GetNewTarget (filtered: line of sight / its collidedWith?) and searched again the next frame (not 5 frames later): PORTED: on the frame the delay ends Process searches (GetNewTarget), then — the frames since lastTargetSearchElapsed (Initialise) being >= 5 — goes on into the every-5-frames check the same frame, which searches again when the Target is invalid (a second Random draw). The _ti batch's seed 955 draws Close, First, Last: 1460's first two find nothing, its 958 search (third draw) Last -> 1340. _ti now to rt 985. GetNewTarget's BloonFilter is IsBloonValid (@0x180c84d00, AdoraTrackTarget's): Projectile.FilterBloon and, unless the model ignores blockers, Map.IsTargetLineClear(bloon, shot) — likely why 1460 (leftmost of its batch, off the top of the map) finds nothing (the clone's seek_valid already tests line of sight from the bloon's height; using the shot's z 120 instead changed nothing). STILL OPEN: the original recording's 1558 (seed 957 draws Close, Close, Close, Strong, Last, yet 1558 has 1420 at 953); _ti rt 985: shrapnel 1420 (batch 933, seed 947: Last, Close, Close, Close, Strong, First, ...) took 1372 at 945 (Last), lost it at 953 (Process top: FilterBloon / line of sight), 1340 at 955 (Close), lost at 956, nothing at 960 / 965, then 1521 (198.9, between 1539 at 181.1 and 1372 at 216.7) at 970; the clone's twin takes the 1372-like 611 at 970. SOLVED (rt 970): BloonManager.GetTargets (@0x180fcacc0) sorts the brackets only for First / Last / Close; for Strong, Targetting.GetTargets (@0x180fdbe70) takes Targetting.GetGlobalBest (@0x180fdb5f0: Path.FindBestBloon's strongest, highest danger, ties strictly further along) when it passes the filter, else the first valid bloon in GetPathSegmentsInRange order (paths in order, segments ascending = least far along first, list order within) — strong_target_from; the clone's 1420 now takes 1521. _ti now to rt 998 (pending/paragon still 973; the paragon_probe/sniper_paragon_r40.rec HP probe was already off at rt 213 before this change). rt 998: when Black 935 dies (997), batch 941's shrapnel 1470/1480/1490/1510 retarget to the leader 1140 (First, their second draw of seed 955) but 1500 takes 1547 (189.5, near the start: Last, i.e. a third draw), while the clone's twins all take the leader. No tree blocker lies between 1500 (-87, 43.6) and 1140 (-50.8, 1.7), so not line of sight: 1500 must have made one more draw earlier (a failed first search at the delay end, as 1460 did, or a lost-target retry); BloonManager.GetClosestTarget (@0x180fc9bf0) is the clone's seek_pick (nearest bracket first, its first valid bloon). Clone draw log (uid39-44 = 1460-1510): uid43 (1500) draws Close at 953 and First at 998, as its batch-mates; the game's 1500 acts as if on its third draw (Last). The polygons (clone frame) never block 1500 from 935 (953-997) nor from 1140 at 997, so not line of sight; TrackTargetAfterDelay.Process (@0x180cf02c0) drops the Target at the top when Projectile.FilterBloon or Map.IsTargetLineClear(bloon, shot) fails, and its every-5-frames check (958, 963, ..., 993) re-draws when invalid, so some frame made 1500's 935 invalid for one check only (it re-took 935 with First). Projectile.FilterBloon (@0x180c8c980) is HaveAlreadyCollidedWithBloonInLineage plus the shot's static filters, so not that either (after the delay it only collides with its Target, which expires it). Target.get_IsValid (@0x180c09350): the bloon not destroyed, its ObjectId (+0x14) still the Target's cached one, and a virtual (+0x778, likely untargetable) false. FOUND (2026-11-13): the shrapnel's System.Random seed is the emmisionIndex its POOLED OBJECT held from its previous projectile + time: the factory creates and Initialises the projectile (TrackTargetAfterDelay.Initialise reads emmisionIndex) before Emission.BaseEmit (@0x180c3dce0) writes the new one (ArcEmission passes 0..n-1, Single 0). Probe recordings pending/paragon_probe/sniper_paragon_r40_tg.rec (TG: every GetNewTarget, same run as the main recording) and sniper_paragon_r40_pool.rec (XO: each destroyed projectile's object pointer + emmisionIndex; TO: each shrapnel's pointer): batch 941's 1548 / 1558 had indices 4 / 3 from reused shrapnel objects; the rt 605 / 613 batches got NEW objects (0) although the rt 181 batch's six were freed at 226-233 (other projectiles took them); later batches reuse shrapnel / explosion objects, ranks not plain LIFO. PORTED: Projectile.pool_emit (the object's previous emit_index via proj_obj_emit) seeds the Random. NOT EXACT: the clone's projectile pool (proj_pool, LIFO, freed at proj_flush) hands out different objects (its rt 607 batch gets 3 / 4 / 2); FIFO is no better. Pool data (sniper_paragon_r40_pool.rec now also has PO: every Projectile.Initialise with its object pointer, stale emmisionIndex and model id; 332 projectiles, the 0-5-0 / 0-0-5 shots being instant): one pool shared by all projectile models (Shrapnel reuses Projectile / Explosion objects, Collectables reuse Shrapnel ones); within a batch the reuse runs from the most recently freed object back (stack-like), but objects freed at rt 226-234 (just before round 40 starts at 235) are not taken until rt 1868 while NEW objects are made at 605 / 613 with them free; a plain LIFO / FIFO / per-model replay of the XO / PO lines fits 8-23 of ~170 reuses. HYPOTHESES to test on this data: the push order at the flush differs from the X destroy order; objects freed across a round start sit elsewhere; an object returns only once its display is gone (would not be reproducible from the sim). (Emission.BaseEmit makes the projectile with FUN_1818625c0 = Entity.AddBehaviorFromModel on an Entity it got from *(emission.sim + 0x60): the pooled object is the behaviour on a pooled Entity; trace the entity factory's Create / Destroy next. Behaviours come from FactoryFactory.CreateBehavior (@0x180fa1020), a Factory`1 per type, flushed every 60 frames as the clone's proj_flush; yet the rt 181 batch's six objects, X-destroyed at 226-233, stay out of the pool until rt 1868 while only 17 objects exist by rt 606: the entity / factory entry likely stays live after Projectile.OnDestroy (another behaviour or its display keeping it) - check what releases it. A replay of the PO / XO lines with the clone's own rules (Factory flush every 60 sim frames, dead entries swap-removed front to back into the pool, newest taken first) is wrong from rt 605 on: 9 objects in the pool, yet the game makes 7 new ones; so a destroyed projectile's object isn't in the projectile pool at the next flush.) NEXT: make the pool exact from these lines; then 1500's extra draw (ti recording) likely falls out of it. (935's own history: hit without damage by 1440 / 1450 at 991-993, degraded Black -> Red by 1430 at 995 (BL layer id 940 -> 1560 at 996, so the cached id check fails from then: PT 935 0 at 997), popped by Explosion 1562 at 996; none of it singles out 1500.) NEXT: Target.get_IsValid in the 5-frame check (the bloon object / its id changing, e.g. another shot degrading it), or record a probe of TrackTargetAfterDelay.GetNewTarget calls (shot, roundTime) to see when 1500 draws.
ACE paragon (2026-11-13): CenterElipsePattern.Attatched (@0x180db2ad0) sets the tower's target type to Centered whenever a new one is attached: the Paragon (from a Circle 5-0-0) flies the 90 ellipse round (0, 0) (tpf 40 / (2pi 90) / 60 = 0.00117893); a tower placed with one still flies its first type (ace_002: Circle). FilterTargetAngle.FilterBloon (@0x180c402b0) on a tower with an AirUnit measures from the AirUnit's position and rotation (the Paragon's Red Tipped Darts: 60 degree cone ahead of the plane, first at rt 322). A TrackTarget shot's CreateProjectileOnExhaustFraction at its lifespan's end emits where it was before the frame (TravelStrait expires it instead of moving it; rt 360); NOT for every lifespan expiry (Alchemist / Engineer / Boomerang shots then go off). ENGINEER EXACT (2026-11-13): the upgraded 5-0-0's own Sentry Champion sentry fires its CreateProjectileOnTowerDestroy (BeforeExplosion) as the purchase removes it, its fuse from the next frame (the purchase is processed then); RoundBasedAttackSpeed (OnRoundComplete @0x180d87120): numRoundsPassed + 1 at each round end, a mutator of bonusAttackSpeedPerRoundPercent x rounds (120-frame weapons to 92 at rt 183; maxWeaponAttackSpeed NOT modelled); RotateToDefaultPositionTower on the Paragon's model (Rotation 0 from rt 61); a plain RotateToTarget with rotateTower off stores the attack's own rotation (tower + (aim - tower)) and the eject turns with it (the nail guns at rt 181).
ACE EXACT: TargetStrongAirUnit.GetTarget is BloonManager.GetTarget Strong: the global strongest (Path.FindBestBloon) if it passes the filters, else segments ascending (rt 2897: the cone misses the furthest Pink, so the least progressed Pink). the shrapnel's raw model (ProjectileModel_Shrapnel 223809) has only DamageModel 5, DamageModifierForTag Moabs +135, CreateProjectileOnContact Explosion (500, AgeModel 1 s, SlowModel stun) and TrackTargetAfterDelay. BUCC exact: the upgraded tower's own sub-towers go at the purchase (the 5-0-0's plane has no W lines from rt 61); the Flagship rate buff refuses Paragons and their sub-towers; TargetGrapplable attacks only take MOAB-class bloons, no Boss / BAD (MultiHookManager's hook sharing / reload NOT modelled: no hook fires); a Weapon belongs to its Attack (kept by AttackModel name), so a new attack's weapons start afresh. SUB exact: PreEmptiveStrikeLauncher.OnBloonEnterTrack (@0x180d5e670): a Paragon's fires at every bloon entering (once its ParagonTower is active), others only at MOAB-class; Launch (@0x180d5e190) uses throwOffsetSecond on even launches when set, the offset turned by minus the tower's rotation; a kept-at-0-pierce shot whose lifespan ends is still Expired (CreateProjectileOnExpire); a strike's Target holds the bloon object, so a TargetDamage inherits the pooled object's new bloon (1512 -> 1570); an instant target-only shot with no Target hits nothing; the round end flushes the path lists but not the bloon factory (a Zebra popped at round 41's end keeps its object out of the pool). WIZARD: the Dark Phoenix is an isAirBasedTower (TowerCreateParagonTower.Process @0x180da3fe0 creates it at z 250, constant @0x183e4fed8), so its balls fly at 257 over the map blockers; MagusPerfectusGraveyardStateManager.UpdateGraveyardFillThreshold (@0x180d37d30): on each state switch, AvailableMana / max >= rateBonusThreshold puts RateSupportMutator(rateMultplierAmount) (get_RateMutator @0x18121bc70) on the paragon (the Arcane Spike at 8 frames, not 16). ICE: SimRecorder's CB line now enumerates the OrderedEnumerable it walks: CollisionGrid.GetInLine yields a bloon at every sample that meets it (the MOAB's object four times), so Skip(1) after the initial hit leaves it; the list is ordered by distance to the emit's ejectPoint (the BaseProjectile's spot) and holds no bloon made since the grid was filled (`unplaced`); children made after the frame's movement (a projectile's Process) aren't held from the next frame's move; a MOAB-class pop drops its carried projectiles at once (CarryProjectile.OnDestroy: the Icicles at age 0); TravelStraitSlowdown's slowdownSpeedFrames / minimumSpeedFrames apply for a paragon's (run-time cloned) models, 0 for loaded ones (dartling); the paragon starts on its model's first target type (INFERRED: First after a Close 5-0-0). Earlier work on these: Magus graveyard states, weapon gates, consume / generate, sacrificed towers' last frame, OrbitingTower, SpawnZombieOnBloonDestroyed; Ice beam exhaust_line. |

TOWERLESS RECORDINGS (found 2026-11-07, owner saw it on screen): the game starts with 650 and
CreateTowerAt refuses a tower it can't pay for, silently (log: `placed ... False`), so nearly all old
pending dartling (23/23), ace, heli, spike (24/25 each) and super (32/33) recordings have no tower (no
W lines). All their scenarios now have "cash": 10000000 (gen_scenarios.py gives every scenario that);
re-record them before use. Dartling's targetTypes ids are "Normal" (pointer) and "Locked" (selected
point), not "Point": scenarios use Locked. Normal (follow the mouse) can't be recorded by hand; plan:
have SimRecorder set the input manager's cursor from the scenario each frame, so it's scripted.

TOWERS.md has the longer per-tower notes.

## Now (top = next)

### DONE: Navarch Final Strike ability (FinalStrike; 2026-11-16)
EXACT: pending/maptowers/map_lotusisland_sub_paragon_040_r40 (3600 frames). Ported (Ability `final_strike`, Tower `fstrike`, Game::final_strike_step after the Comanche step): Activate sets countingFrom = Simulation.time, index -1; while it runs the tower's Attacks and its PreEmptiveStrikeLauncher are paused; after countdownFrames, a missile when none yet or launchRateFrames since the last: Strong, First, Close over the map (IsBloonValid: alive, on the map; STAND-IN: BloonModel.isInvulnerable not exported, and the Strong pick's LINQ steps taken as the Strong target type), from the tower + throwOffset turned by its rotation (as Pre-Emptive Strike), at the bloon; after the third it ends. CreateProjectilesOnTrackOnExpireModel exported (`track_expire`: projectile, emission, count, range, useRotation) and ported (Game::track_expire_emit, VERIFIED-DECOMP Expired @0x180caa930: targetable segments within range of the shot in path order, count x syncedRandom.Next(0, n - 1) picks taken out, the projectile at each segment's point): the Aftershock's 5 Fallouts at rt 1643. STAND-IN: useRotation (false on the Fallout) not applied; the ability's cooldown starts at activation, not at EndCountdown (the 240 s cooldown never comes up in a recording).
Notes from the first read (kept):
The Navarch (MonkeySub-Paragon) has AbilityModel "FinalStrikeAbility" with FinalStrikeModel (countdown 15.05 s, launchRate 0.2 s, FinalStrikeProjectile: AgeModel 0.6, Instant, CreateProjectileOnExpire -> 0.05 s -> SlowModel 15 s + DamageModel + CreateProjectilesOnTrackOnExpire (a 36 s track projectile with DamageModifierForTag, RefreshPierce, ClearHitBloons, DestroyProjectileIfTowerDestroyed)). The export gives the paragon `abilities: []`, so the clone has none. Recording pending/maptowers_open/map_lotusisland_sub_paragon_040_r40 (scenario of that name: Navarch + a 0-4-0 Sub placed at frame 120, `ability` 700 activates every ability, so the Final Strike too): bloons exact until rt 787; the game's Navarch launches no Pre-Emptive Strike from rt 700 to 1627 (its PreEmptiveStrikeLauncher behaviour paused: OnBloonEnterTrack @0x180d5e670 returns while RootObject.IsPaused @0x180f42060, while the tower IsStunned (Tower vtable slot 102), and for a Paragon while its TargetType id is "Submerge"), the clone launches at every entry. Code read so far (VERIFIED-DECOMP, research/dumps/decomp/FinalStrike/): Activate (@0x180e2f340) / StartCountdown (@0x180e30520): countingFrom (+0x70) = Simulation.time, currentMissileIndex (+0x74) = -1; for every behaviour of the tower of some type (TypeInfo 0x184f71978) bump a pause counter (get @0x180f42080 / set @0x180f422f0) and SubmergeEffect.TowerSuspendedStateChanged, the same on the behaviour of type 0x184f63f60 (likely the PreEmptiveStrikeLauncher); then by the tower's TargetType ("Submerge" or not) the model's +0x78 / +0x7c counts. EndCountdown (@0x180e2f3f0): countingFrom = -1, lastMissileFiredAt (+0x78) = -1, Ability cooldown set, the pause counters decremented, the countdown effect destroyed. Process (@0x180e30120): only while active (vtable +0x8f8); while the tower is stunned or flag +0xbc, the countdown start moves on a frame (+0x70 += 1); else cooldown 0 and UpdateMissileLaunching (@0x180e30c60, NOT read), plus the per-second countdown text. Also to read: LaunchMissile (@0x180e2fa80), IsBloonValid (@0x180e2f9f0), CanUseAbility (@0x180e2f380). READ (2026-11-16): UpdateMissileLaunching (@0x180e30c60): while Simulation.time - countingFrom < model +0x2c (countdown frames): only spawns the countdown effect once (+0x90 slot, model +0x60 effect at the tower) and returns. After it: a launch when lastMissileFiredAt (+0x78) < 0 or time - lastMissileFiredAt >= model +0x34 (launch frames), and the ability is active (vtable +0x8f8). currentMissileIndex (+0x74) -1 / 0 / 1 picks a TargetType static: TargetType's statics are 0x18-byte structs in declaration order (None 0x00, First 0x18, Last 0x30, Close 0x48, Far 0x60, Strong 0x78, StrongExceptMoab 0x90, ...; Assets.Scripts.Models.Towers.TargetType in research/dumps/Assembly-CSharp.txt), so the three missiles target Strong, then First, then Close; any other index returns. If that TargetType's intID equals Strong's (static +0x84), i.e. the first missile: BloonManager.GetTargets(range 999999999 from the tower, BloonFilter(IsBloonValid)) then two LINQ steps (OrderBy-style with cached delegates, FUN_1818460b0 / FUN_181852500) and the first; else BloonManager.GetTarget(same). No bloon: no launch. The first launch (index -1) spawns the launch effect (model +0x68) at the tower. IsBloonValid (@0x180e2f9f0): not destroyed (+0x21), BloonModel +0x8f off (likely isBoss / untargetable flag; check), its position on the map (Map.IsPointOnMap). LaunchMissile (@0x180e2fa80): Target(bloon); start = model offset (+0x50,+0x54,+0x58) turned by the tower's rotation (Vector2.Rotate), AddDisplaySpaceHeight, plus the tower position; Emission.BaseEmit(emission +0x80, model +0x40 projectile, start, target, the tower's ...); an effect (model +0x70) at the start; the bloon id into targettedBloons (+0x88); currentMissileIndex += 1, lastMissileFiredAt = time; after the third (index > 1) EndCountdown. So: three missiles, launchRate apart, after the 15.05 s countdown. The First Strike (0-4-0) part of the recording can only be checked once this is ported (the FirstStrike damage skip of the support is still unverified by a recording).

### DONE: Sub Paragon support (MonkeySubParagonSupport; 2026-11-16)
EXACT: pending/maptowers/map_lotusisland_sub_paragon_quincy_r40 (Navarch at (-112, 66), Quincy L1 at (-74, 58); scenario tools/mod/scenarios/map_lotusisland_sub_paragon_quincy_r40.json) matches all 3600 frames (bloons, positions, weapons; maptowers 36/36).
- MutatorTower.Mutate (VERIFIED-DISASM @0x180d45640, token 0x6002ef4): only for baseId MonkeySub or IsHero (@0x1811b40c0). Every ProjectileModel (sub-projectiles too): its DamageModel, unless named FirstStrike, damage += damage x (hero ? heroBonusDamage 5 : subBonusDamage 6) (DamageModifierForTag additives untouched); pierce += pierce x (hero ? 2 : 2); hasDamageModifiers = true. A HeroModel tower also gets xpScale x heroXpMultiplier (5) and every weapon SetRate(rate x heroRateMultiplier 1.3); Subs no rate. Export: support kinds `subparagon` (mult = damage bonus, add = pierce bonus; clone `Buff.sub_paragon`), `rate` (hero), `hero_xp` (hero; `add_hero_xp` multiplies xpScale by it). HX at rt 904 (1220 x 5 -> L5) now matches.
- Shared range: TargetSupplierSharedRange.PressTheGridsTogether (VERIFIED-DECOMP @0x180df0320) walks the attack's own towerFullSightPathSegments (+0x70); per segment with grid entries (the SharedTowerGrid dictionary): kept only if some entry passes the specificTowerType filters (empty passes); canSeeCamo = some entry inRange && camo && LOS; inRange = some entry inRange (else skipped); lineOfSight = (shareLineOfSight (+0x6b) || the own entry's LOS) && some entry inRange && LOS. SimRecorder SP lines (segprobe; probe recording in a scratch dir, not kept) confirmed: the Navarch can't see segments 63 / 64 from its spot, so its first shot is on 65 (rt 844). Clone: `shared_in` needs `attack.shared_los || seg_clear` (export `shared_los`).
- Sub in range EXACT (2026-11-16): pending/maptowers/map_lotusisland_sub_paragon_subs_r40 (scenario of the same name: + a 2-0-0 and a 0-2-2 Sub placed at recorded frame 120, after the purchase) matches all 3600 frames (maptowers 37/37). SimRecorder: TowerSpec `place_frame` (CreateTowerAt between recorded frames; list such towers last), written `# place k frame`; the harness (compare_one `late`) places them before that frame's step.
- Targetting.GetTargets (VERIFIED-DECOMP @0x180fdbe70) per visible segment entry: a camo bloon needs the entry's canSeeCamo; inRange && LOS -> plain target; otherwise a MOAB-class bloon goes through CanHitMoab (@0x180fdb200): per collision circle, unless usingSharedRange the circle within range + its radius, then the entry's LOS or Map.IsTargetLineClear(tower -> circle). So a shared-range attack hits a MOAB on a pressed segment it can't see when the line to a circle is clear, at any distance (the 2-0-0 Sub fires at rt 853, 90 from the MOAB). Clone: `g_in` / `g_los` (pressed flags, grid entries within range + 2 = PathSegment.IsInRange, the tower's own addsToSharedGrid attacks included) and the circle path without the distance test. TargetSupplierSharedRange presses only when sharedTowerGridChanged; towerFullSightPathSegments is made once (GenerateTowerFullSightGrid / first GetTargets) and on ThrowCacheUpdated.
- Shared camo EXACT (2026-11-16): pending/maptowers/map_lotusisland_sub_200_quincy_camo_r33 (2-0-0 Sub + Quincy L1, round 33's camo Reds / Yellows): the game's Sub never fires. Every Sub attack has FilterInvisibleSubIntelModel (isActive): CantTargetCamo = isActive (@0x180c1f8c0, shared with FilterInvisible), FilterBloon always true (@0x1804ee640). Attack.CalcCantTargetCamo (@0x180dac510): any attack behaviour's CantTargetCamo (vtable +0x8a8). GetPathSegmentsInRange (@0x180dad8f0) sets each entry's canSeeCamo = !cantTargetCamo, so Subs' own entries never see camo; PressTheGridsTogether's canSeeCamo = some entry inRange && camo && LOS; GetTargets lets a camo bloon through only on an entry with canSeeCamo (QuickTargetCheck @0x180fdc9d0 skips its camo test with usingSharedRange). Export: Sub attacks `camo` false, `intel_camo` true; clone: grid entries carry `a.camo || buff camo`, shared attacks need `g_camo`.
- NOT DONE: Silas's shared-range attacks have shareLineOfSight true and specificTowerType "IceMonkey" (only Ice Monkeys' grid entries count): the type filter isn't exported yet (Silas is unported).

### Next tower: Quincy (hero; 2026-11-15)
Every buyable tower and paragon is exact; Quincy is the shop's one hero. Scenarios tools/mod/scenarios/quincy_*.json
(SimRecorder now places a hero at a level: tower "level": N -> Hero.UpgradeToLevel after placing, written as
`# herolevel k N model xp`, which the compare harness applies; HX lines log Hero.AddXp with heroXpMultiplier
and endOfRoundHeroXpScale). Recordings in pending/quincy: quincy_r04, r30 (level 1, levels up at round end),
l05_r30, l07_r40, l10_r40 (Storm of Arrows), l14_r50, l20_r60: 7/7 MATCH (`sh tools/rec/compare.sh quincy`).
Ported for them:
- A delay-0 RetargetOnContact in the collision pass turns the shot before its TravelStrait moves it (this
  frame's move redone from the frame-start spot).
- BloonManager.GetClosestTarget (@0x180fc9bf0): during the collision pass (Path.Process runs after the
  CollisionChecker) a bloon is on the segment of its distance before its latest move (`seg_dist`; a first
  child keeps its parent's), a copy made in this pass isn't filed, a bloon not yet moved isn't, and the
  path lists are in their pre-flush order (`plist_rank_pre`); later in the frame, the frame-start distance.
- FilterWithChance (@0x180c408a0): filterChance > syncedRandom.NextDouble(), one draw per bloon asked.
- DestroyWhenHitImmuneModel.getsDestroyed false: Projectile.CanBeDepleted false, so Bloon.Damage's
  immune Deplete doesn't end it (`immune_keep`).
2026-11-15: 7 more recorded (l03_r20, l12_r60, l17_r70, l19_r80 with abilities; l10_r40_last / strong /
close): quincy 14/14 MATCH. DONE: CollisionGroupData.precalculatedOffset (MOAB-class circles) lives on the shared
BloonModel; Bloon.SetRotation (@0x180fb6750, from UpdatePositionAlongTrack and SetPath) rewrites it for the whole
model, so Targetting.CanHitMoab, the collision pass and the line checks measure every MOAB with the offsets of
the model's LAST SetRotation (`kind_rot`, `mrot`): probe pending/quincy_probe/quincy_l12_r60_cm.rec (scenario with
"canhitmoab": true -> CM lines with each circle's precalculatedOffset, SR lines with each SetRotation; seed
811805639 replays quincy_l12_r60): at rt 3033 MOAB 463 (rotation 180) is measured with MOAB 2380's -128.13.
A new bloon's SetPath sets it too (quincy_l19_r80 rt 2893: a BFB's MOABs).
NEW COVERAGE (2026-11-15): pending/moab (scenarios moab_*.json, seed 777123, rounds 60-78 single towers):
5/5 MATCH. CAUTION: a Harmony patch on Targetting.CanHitMoab (its by-ref Vector3 offset) perturbs the game:
with it installed the game aims at the MOAB's centre (the first moab_* recordings and the *_cm probes were made
so and were wrong; the patch is removed and the set re-recorded). Probes that only read (SetRotation,
RotateToTarget.ApplyRotation: SR / RT lines) are fine. DONE moab_dart_502_r60 rt 2732: a Juggernaut ball crossing two overlapping tree areas bounces off the
first in Map.areas order (ProjectileBlockerCollisionRebound.Process), and the clone's map areas were in the
asset file's object order. SimRecorder now writes "# area index name active blocker height" header lines;
Map.areas is the scene hierarchy in REVERSE (depth first by sibling index): tools/assets/export_map.py sorts
areas so. The local Tutorial / Logs map.json had only their areas reordered (export_map.py alone writes the raw bezier
path; the installed files' path (682 points with dists / speeds) comes from export_map_pair.py's later step, as
extract_all runs it: no issue). All 89 local maps' areas were then
reordered the same way, by name (plus first point for duplicate names) against a fresh export.
MORE MAPS (2026-11-15): pending/maps (scenarios map_<key>_dart_205_r40.json, land spots picked 16-22 from the
track): run tools/mod/import_paths.py <recording dir>/<Map>.map <key> first for a newly recorded map (the game's own
path points / quick table / area outlines; it now also puts the areas in the dump's order). Cubism, InTheLoop MATCH.
DONE map_treestump_dart_205_r40 (maps 3/3): the per-frame targeting uses the segment list InitTargetPathSegments keeps, from Attack.GetPathSegmentsInRange (@0x180dad8f0, not the Fast one): a segment clear from the tower is checked again from the tower plus CheckFromOffset (@0x180dac780: the first WeaponModel ejectX / ejectY) turned by RotateToTarget additionalRotation + Vector2.get_Rotation(point - tower) (= atan2 - 90). Probe scenario tools/mod/scenarios/map_treestump_dart_205_r40_seg.json (segprobe: SG / SGR lines with each PathSegmentInRange flag, AI lines for Area.Intersects hits; lines from before the .rec opens are buffered) in pending/maps_probe: segment 66 clear from (5, 22), blocked by TreeStumpBlocker from (1.40, 18.45). `seg_line_clear` in game.rs. DONE (2026-11-15): CheckFromOffset returns zero when the attack has a CheckTargetsWithoutOffsetsModel (0x184f67028 is GetDescendant<that>; 85 attack models): exported as attack "los_no_offset", Los::of. No recording exercises it on a blocker map yet. visible_segments (TargetTrack / necro / acid pool lists, the same cached list) now uses the same check (`Los`, `los_clear`); full regression unchanged.
FIXED (2026-11-15): import_paths.py keyed dumped areas by name, so maps with repeated area names (Cornfield's thirteen CornfieldBlocking1, Quad, MoonLanding, Haunted and 14 more) had every duplicate on ONE outline. patch_areas now matches each by name + nearest first point; tools/mod/repair_dup_areas.py restored the 18 affected local maps (fresh export_map.py outlines matched by each area's untouched `blockers`, ids kept, then the dump's outlines and order). Also: every path's Path.bloonsList is the one Bloon factory list (PathManager.Init), and First / Last take one global leader across paths (PathManager.FindGlobalBest; comparer INFERRED as distance until leak). maps 11/12 (Quad, Cornfield, MoonLanding, FourCircles new). DONE map_haunted (maps 12/12): the BloonGrid's buckets follow the shared factory list's order on multi-path maps too (the grid key was creation order there), and distance until leak counts to each path's leakDist, not its length (rt 813: First takes 963, 392.2 from Path's 729.59 leak, over 971, 410.1 from Path (1)'s 798.19). Cornfield's extra dump area is BaseArea, the game's runtime default (every map).
MAP TOWERS 16/16 (2026-11-15): map_quad_ace_204_r40 MATCH (moved to pending/maptowers). DONE: the cross-path First/Last leader comparison (PathManager.FindGlobalBest @0x180f6a7d0 with CompareBracketFirst @0x180dacb90 / CompareBracketLast) compares the bracket's PathSegment +0x38 = distanceUntilLeak (VERIFIED-DISASM), not each bloon's own distance until leak: rt 2534 Red 14259 on BottomtoTop segment 29 (288.74) leads Yellow 13949 on TopToBottom segment 34 (289.63). Next: more map-tower coverage (other towers on Quad / chained-path maps) or the next tower.
MAP TOWERS 34/34 (2026-11-16): + map_quad_ninja_paragon_r40 MATCH (moved to pending/maptowers; maptowers_open now holds only map_kartsndarts_druid_052_r40). DONE: RotateToTarget.Process (VERIFIED-DECOMP @0x180dd80d0) with onlyRotateDuringThrow and rotateOnlyOnThrow off calls ApplyRotation (@0x180dd7930) every frame, weapons ready or not; with Attack.HasValidTarget (@0x180daeee0: frameTarget (+0xb0, a Target struct kept until the next FindTarget) not destroyed, same setBloonId) it turns the tower to that Target where it is now. rt 182: the Paragon's reloading Sticky Bomb attack (rotate_always, a new Attack, after the MOAB's move) turns it to -3.965 while Attack 0's shurikens leave along -4.52 (the earlier note's 'aims at its rt-182 spot' was this turn, not the throw); the Sabotage zone remade at 241 uses that rotation. Tower.rot_held (id + aim offset from the bloon; INFERRED offset kept as found), applied in the all-weapons-cooling skip. Not modelled: HasValidTarget's vfunc 0x778 test.
MAP TOWERS 33/33 (2026-11-16): + map_sunkencolumns_dart_paragon_r40 MATCH (moved to pending/maptowers). DONE: ProjectileBlockerCollisionRebound.Process (VERIFIED-DECOMP @0x180cd1110) skips an area that isn't isBlocker (AreaModel +0x68) or whose height (+0x40) is below lastPosition.z (Vector3 at +0x70; SetStartPosition sets it, each Process ends by setting it to the position): rt 586, the Ultra-Juggernaut shards at z 72 (the paragon stands on SunkenColumns' raised land) fly over BlockingRight (height 50); the clone tested height >= 0 (`z_last` in collide_pass).
MAP TOWERS 42/42 (2026-11-16): + map_bazaar_wizard_502_r40, map_kartsndarts_glue_502_r40, map_mesa_ice_520_r40, map_lotusisland_sub_paragon_040_r40 (Final Strike) MATCH. map_erosion_mortar_502_r40 MATCH (maptowers 43/43). DONE (2026-11-16): the CollisionGrid query is CollisionState.Initialize @0x1824b3ed0 / MoveNext @0x1824b4110 (addresses from the SimRecorder collisionstate_method probe). Its bucket "inside" box test (all bloons of the bucket count, no distance) compares Bucket.centre, which is grid-local ((i + 0.5) x 10, (j + 0.5) x 10; the map dumps' blocker buckets show it), with the world-space query box (extent sqrt(200)), so a big query takes whole cells shifted by (-202, -168). rt 1114: Shockwave 802 at (122.08, 113.62) takes Blacks 137 / 154 away in cells (21, 19) / (20, 18), not 562 in (22, 21) at 120; 618 in (18, 17) is past the walk (rank 996 of count 961). grid_hit now does this; P-CELLCIRCLE retired. First difference now rt 3035: the clone pops Black 2671 with Shockwave 3005, which the game skips through DamageInRingRadius: 48.4 away in 2D, the bloon at z 28.2 and the blast at Position.z 8.9 (the shell lands at RandomTargetSpread's Target z, Map.GetTerrainHeight at the point; P lines: 10 / 8.9 on Erosion), 3D 52.1 < 55.5. The clone keeps the shell at 41.5 and takes the blast at 0 (the aim_z STAND-IN in the target_spread branch, and the ring test). DONE: a RandomTargetSpread Target is at Map.GetTerrainHeight there, and the instant shell takes that z (8.9 / 10 on Erosion, 0 on Tutorial); DamageInRingRadius measures z from the blast's own z (inherited from the shell). MAP TOWERS 47/47 (2026-11-16): + map_sunkencolumns_mortar_204_r40, map_mesa_mortar_052_r40, map_erosion_bomb_204_r40 MATCH first time; map_erosion_engineer_204_r40 MATCH after two fixes. (1) RandomPosition.CheckPositions (@0x180dd27f0) keeps each point with Map.GetTerrainHeight as its z, so the Sentry's ArriveAtTarget eases in over the 3D distance to 10-high land (rt 2). (2) Map.CanPlace (@0x180f513d0; constants from the DLL: -5, 5, 0.75, 250, 0.5): when the 0.75 x radius circle crosses the edge of the area holding the point, the point must also be inside another active placeable area within -5..=5 of its height whose edge the circle doesn't cross (land_clear; rt 301: (102, 101.42), 0.6 inside LandArea (2), is refused). The SimRecorder map dump now writes areaflags lines (prohibitTrackPlacement, ignoreZAxisTowerCollision, renderHeightOffset); prohibitTrackPlacement is False on every Erosion area. Still open in CanPlace: the footprint AABB terms, IsPlaceableInAreaType beyond land, and the 250 height cap. MAP TOWERS 51/51 (2026-11-16): + map_erosion_tack_204_r40, map_mesa_wizard_204_r40 MATCH first time; map_sunkencolumns_engineer_204_r40 MATCH after: land_clear takes the game's GetAreaAtPoint (the first scene area, the base last), not the clone's base first (a Sentry spot on 50-high LandRight over WaterRight at 1); Map.BlockedByBlockerZone (@0x180f4fd80) only has scene BlockerZone components (UnityToSimulation.AddBlockerZone @0x181580b50, inverse), not areas' blockerCircles, so those no longer refuse placement (STAND-IN: scene BlockerZones not exported); the BaseArea's type comes from the map dumps (map.json base_type via import_paths.py: water on Cargo / Erosion / FloodedValley / LotusIsland / Peninsula / Polyphemus / SpiceIslands, unplaceable on Geared / Infernal / MushroomGrotto / OneTwoTree / Sanctuary / SkullTweak / SulfurSprings / SunkenColumns). map_erosion_druid_204_r40 MATCH after: the attack's Target (and so RotateToTarget) is the latest throw's, ties to the lower weapon index (rt 2411: the thorns' throw turns the tower off the lightning's two frames older one). Unit tests 239 pass. MAP TOWERS 55/55 (2026-11-16): + map_polyphemus_engineer_204_r40, map_infernal_alch_204_r40 MATCH first time; map_infernal_engineer_204_r40 MATCH after: land_clear's 0..=5-above unplaceable check is Area.Intersects (@0x180f494b0 -> Polygon.Intersects @0x180f233e0), edges only, so an area holding the whole circle (Infernal / Geared's unplaceable BaseArea) doesn't refuse. map_geared_engineer_204_r40 MATCH after: Map.RotateArea (@0x180f5e130) turns every tower in Map.areaTowers for the area, Sentries included (map_logic.rs); EatBloon.EmitCash (@0x180cb98d0) replaces a full trap by its EatBloonModel.projectile (Full, AgeModel 5 s: build_game_data eat.full_lifespan), so the trap goes 300 frames after it fills (rt 2169 -> 2470). EmitCash (@0x180cb98d0) emits the Full projectile (CashModel + PickupModel collectRadius 100, AgeModel 5 s; build_game_data eat.full_cash) with min = max = (rbeCashMultiplier - 1) x cashCollected (K = 1.0 read from the DLL); uncollected, its cash goes with it. INFERRED: the clone pays no pop cash for eaten bloons, so its trap value is RBE x rbeCashMultiplier (the game pops them through Bloon.Damage and counts layers x the round's income multiplier). Note EatBloon.Collide adds GetModifiedTotalLeakDamage, not RBE, to rbeCollected (the clone uses RBE; same for these bloons). MAP TOWERS 58/58 (2026-11-16): + map_infernal_boomerang_204_r40, map_polyphemus_sniper_204_r40, map_geared_druid_204_r40 MATCH first time. Ported (decomp, not yet proven by a recording): RotateAreaAction.Process (@0x180f8d720) passes updateThrowCache on the latest spin's last frame ((frame - its start) + 1 == durationFrames) -> Tower.UpdateThrowCache -> TrackTargetSupplier.ThrowCacheUpdated (@0x180e09ec0, generatePoints) -> the next Process (@0x180e09e90) remakes and reshuffles the points (map_logic.rs logic_throw_cache). map_geared_spike_204_r40 MATCH (maptowers 59/59) after: (1) Attack.useOnTheMoveTargetting (+0xa4; Tower.PositionTower @0x180c15230 and MoveTower set it to !updateThrowCache, Map.EndTowerMovementOnArea clears it): while set, TrackTargetSupplier's GetTargets (@0x180e0b7d0) calls GeneratePoints first at every pick, and the points come from Attack.visiblePathSegmentsUnordered (path order, no Close sort). SimRecorder OM lines (new probe) show it on from rt 1 for a tower on Geared's gear (never on Tutorial): the game places the tower and the start's catch-up (GotoPostAction, Map.RotateArea with updateThrowCache off) then moves it; the clone, which catches up before placing, marks towers placed that step on a caught-up area (logic_placed_on_caught_area, STAND-IN for the order); their first points (made at placement) are Close-ordered. Spin frames set it, the spin's last frame clears it and remakes the points (logic_throw_cache). (2) RotateAreaAction.Process runs after the towers: every throw during a turn ejects from the tower's previous-frame spot (logic_spin moved after towers_phase). Next: more map towers.
MAP TOWERS 32/32 (2026-11-16, paragons): + map_kartsndarts_tack_paragon_r40 MATCH. DONE: Bloon.nonTargetable timing in the collision pass (bloon_nontarget_now): Bloon.Process clears and re-sets it on the track (Bloon.processed, cell_kind / cell_dist at the grid fill; off-track: not set), a degrade's first child (the same object) keeps it, and a bloon made since (a pop's copies: their first Process is next frame's, after that frame's collision pass) has what Bloon.SetPath (@0x180fd5050) gave it at its placed distance (its distance now; INFERRED: one held at the spawn point was placed further on); Bloon.PreCheckDamageOutcome (@0x180fd4160) gives a nonTargetable bloon no damage even when an ignoreNonTargetable projectile meets it (map_quad_ace_204_r40 rt 1627: a Spectre bomb blast spends pierce on a Red copy in Quad's entrance stretch, map_quad_ninja_paragon_r40 rt 183 / 184: a MOAB popped at 221 is still hit as its Ceramic, its copies held at 158.52 are hit, map_quad_tack_502_r40 rt 146: copies placed at 182-196 aren't). OPEN (pending/maptowers_open): map_quad_ninja_paragon_r40 (bloons exact; rt 182 the Paragon's weapon-0 throw at MOAB 260 aims at its rt-182 spot, the clone at its 181 one, so the tower's rotation (-3.965 vs -4.522) and the Sabotage zone remade at 241 differ: the game's MOAB (spawned rt 30) moves before the Paragon's attack (bought at frame 60, new objects?), but treating the Paragon's attacks as new breaks ninja / ice / sub paragons at rt 182 (tried: clearing attack_born, re-stamping it, and only for process_bloons_before)); map_sunkencolumns_dart_paragon_r40 DONE (see MAP TOWERS 33/33).
MAP TOWERS 31/31 (2026-11-16, eighth batch, tier 5s): + map_haunted_sniper_250_r40, map_mesa_bomb_052_r40 MATCH; map_quad_tack_502_r40 MATCH after: TrackTarget.IsBloonValid (@0x180d12ff0, keeping its current Target) has no nonTargetable test (FilterBloon, the angle, IsTargetLineClear unless ignoreBlockers); only a new Target (BloonManager) can't be nonTargetable (rt 635: the Inferno Ring fireball keeps a Black gone into Quad's exit stretch). Also DONE: Track::length is the game's own point distance (3D: totalPathLength) when imported, not the 2D sum of the points (KartsNDarts' tunnel dips to z -42: 35 short, 5 segments fewer); SpiritOfTheForest.GeneratePositions skips bloonsInvulnerable segments (track_seg_inv). map_kartsndarts_druid_052_r40 MATCH (moved to pending/maptowers). DONE (2026-11-16): (1) rt 481's thorn pairs are the CollisionGrid own-cell rule (every thorn in the MOAB's cell (22,19) pairs, even 11.6 away; those in (22,20) don't); the pairs on the destroyed MOAB leave pierce and health alone (H lines are health-after / pierce-after), so 481 already matched. (2) DamageOverTime.DealDamage (VERIFIED-DECOMP @0x180fe51a0) calls Bloon.Damage with ignoreNonTargetable false: PreCheckDamageOutcome (@0x180fb5650: destroyed, or IsInvulnerable without ignoreInvulnerable, or +0x218 nonTargetable without ignoreNonTargetable -> nothing) refuses ticks on nonTargetable bloons (the Spirit's zones on the MOAB's Ceramics in the tunnel, rt 482). (3) The path point of a distance is the quick table's: entry (uint)(distance / 0.01) (Path.DistanceToPointInfo @0x180f6e0d0), so the last point at or before that entry's distance (`quick_point`, used by bloon_nontarget_at / _kind and path_seg_inv): a Green at 344.80695 is on point 141 (point 142 is at 344.80457), still nonTargetable (rt 500 / 501). (4) A refused or immune DoT tick now restores the bloon's later behaviours' lastTick like an ordinary non-degrading tick (they ticked again next frame). (5) The Spirit zones run (BloonEnter) in the tower's turn after the collision pass: copies a pop made in that pass enter the zones the same frame (rt 481: the MOAB's Ceramics have Medium at -999999999 by the frame's end; the clone's zone pass runs before collisions, so those copies now enter right after the pass). DONE rt 542 / 2743 -> MATCH (2026-11-16, maptowers 35/35, pending/maptowers_open empty): (6) DamageOverTime.OnDestroy (VERIFIED-DECOMP @0x180fe5480) runs CheckTick when its mutator goes (lastTick -999999999 first only with damageOnDestroy, exported as zone `on_destroy`, false for the Spirit zones): a zone exit deals a due tick then, in the tower's turn before the bloon moves (`Game::exit_ticks`; rt 542 the Yellow takes Far + Medium, 3 layers, and its Red steps 0.52). (7) The Spirit's Zoned.Run (enter / exit) now runs right after the collision pass (it ran before it): a bloon popped in the pass never enters (rt 2743: a Zebra that came into the Far zone with its last move is popped first, its Blues carry no DoT), and a pass's copies enter that frame (replaces (5)'s copies-only pass). DONE part: Bloon.nonTargetable is set in Bloon.Process, so a degrade's first child (the same object) keeps the parent model's flag until it moves (Bloon.cell_kind; bloon_nontarget_kind in the collision check): a MOAB's Ceramic in the tunnel (bloonsInvulnerable, not moabsInvulnerable) is still hit that frame. Probe (SimRecorder "bloonscale": BS lines, trackScale / baseScale / globalBloonScale / Radius per bloon; map dump p lines now end with moabScale): pending/maptowers_probe/map_kartsndarts_druid_052_r40_bs.rec, all scales 1, MOAB Radius 21. STILL OPEN rt 481: of the 8 thorns (BaseProjectile, radius 0) near MOAB 130, the first pops it; then the game collides (HP lines) 285, 293, 301, 309, 325 with 130 as a Ceramic / Black (309, 325 after it is destroyed: the pair still forms) at 8.65-9.45 / 7.35 from its centre, and never pairs 277 / 317 at 10.44 / 10.83: an effective reach about 10 that neither the Ceramic's radius 8 (clone: 2 hits) nor the MOAB's shape kept until the next Process (clone: 4, wrongly with 277) gives; a +2 projectile radius breaks every druid recording.
MAP TOWERS 28/28 (2026-11-16, seventh batch): + map_mesa_farm_204_r40 (farm + 2-0-5 dart), map_erosion_ace_024_r40, map_sunkencolumns_ice_204_r40 MATCH; map_kartsndarts_quincy_l10_r40 MATCH after: closest_target (BloonManager.GetClosestTarget @0x180fc9bf0, RetargetOnContact's bounce) skips nonTargetable bloons (rt 1069: Quincy's arrows find no bounce in the tunnel and fly on).
MAP TOWERS 24/24 (2026-11-16, sixth batch): + map_lotusisland_sub_204_r40, map_downstream_bucc_204_r40 (water spots: area type 1 by the first holding area plus 8 footprint points, 16-22 from the track), map_quad_sky_204_r40 MATCH; map_mesa_sky_024_r40 MATCH after: a bloon's Position.z is the path's quick-table point.z plus 20 (120 MOAB-class) (Path.DistanceToPointInfoBloon @0x180f6e030; the clone had a flat 20 / 120): an arc at a Black on Mesa's 49.3 track aims at z 69.3, which sets ArriveAtTarget's 3D timeToTake (rt 1784). import_paths.py writes map.json path "quick_z" (30 local maps have non-zero heights, put on from their mapdump dumps); Game::track_qz / track_height (Bloon.Move and spawn). STAND-IN: reverse mode drops quick_z (the forward table doesn't fit).
MAP TOWERS 20/20 (2026-11-15, fifth batch): + map_bloodypuddles_mermonkey_204_r40 MATCH; map_rake_heli_204_r40, map_candyfalls_dartling_204_r40 (Locked point on the nearest segment) MATCH after: (1) MoabShoveZone acts in its heli's turn: the shove step now also runs (once a frame) before process_bloons_before moves bloons early for a newer attack (the 2-0-4's Comanche helis made the MOAB move before the shove was given: 29 shoved frames for the game's 30, rt 875); (2) ComancheDefence's Process argument is Simulation.time (RootObject.ProcessRootObject passes it; the old note "elapsed 1, never retreat" was wrong): every activation (25 / 50 / 75%, the flags cleared each round by OnRoundStarted) restamps activationTime even with all helis out, and at activationTime + durationFrames (900) the helis retreat; made 50 past the edge they're there already and are destroyed at once (rt 2308); (3) a pass-1 AddBehaviorToBloon (e.g. Laser Shock) is never added to a copy the hit made (no M add on a copy in its pop frame in the whole recording): a copy keeps the mutator it copied with its timer (rt 1720: 42 frames left, gone by 1774, so the Black isn't popped 2 extra layers); INFERRED: a copy without it still gets it (removing that broke rt 1720). Only for DamageOverTime effects: a copied emitter (Super Glue's splat) is initialised again on the copy and the re-add stands in for that (glue_005 / glue_025 lost a splat otherwise). Exported AddBehaviorToBloon after-damage effects carry "kids" (cascadeMutators). DONE map_cornfield_beast_204_r40 (maptowers 20/20): (a) the 2-0-4's Condor (a grapple pet whose tier has an AirUnit with no pattern) logs PM lines: the harness now compares its bird (Tower.air.pos, the pet's spot during its first frame) as a plane; (b) keepPickingUpBloons' TargetCheck (@0x180dc7a30: bloon distanceTraveled >= the destination segment's max, then within range of the AirUnit) measures to a bloon newer than the bird at its frame-start spot (gyr_at; rt 2873, a Black 31.4 away after its move, 32.4 before, isn't taken at keep range 32). Tried and wrong: limiting GetDestinationPathSegment to visible / targetable segments (rt 689 broke).
MAP TOWERS 16/16 (2026-11-15, fourth batch): + map_encrypted_tack_204_r40, map_sunkencolumns_super_204_r40, map_haunted_desperado_204_r40 MATCH. DONE (quad ace, Neva-Miss): TrackTarget's new targets skip nonTargetable bloons (Targetting.GetTargets / GetGlobalBest / BloonManager.GetClosestTarget all test Bloon +0x218 and IsInvulnerable), judged as Bloon.Process left the flag: a bloon newer than the projectile at its frame-start distance (bloon_nontarget_at; rt 432, a Black at 212.885 just out of TopToBottom's stretch was still in it). The flag is set by Bloon.Process, SetPath and UpdatePositionAlongTrack; WillDamageBloon refuses a nonTargetable bloon too (not ported: no case yet). The point index is the quick table's pointIndex at (int)(distance / 0.01), the same as the last point at or before it. DONE map_quad_ace_204_r40 (maptowers 16/16): the cross-path leader (PathManager.FindGlobalBest) compares the paths' bests by their PathSegment.distanceUntilLeak (the segment each is filed on), as BloonManager.GetTargets' First bracket comparer does: rt 2534, Red 14252 popped in that frame's collision pass (Path.Process runs after the CollisionChecker), so Red 14259 on BottomtoTop segment 29 (288.74) leads Yellow 13949 on TopToBottom segment 34 (289.63) although the Yellow's own distance until leak is less. BloonManager.GetTargets (@0x180fcacc0) sorts its brackets with a comparer for First / Last / Close (TargetType statics +0x18 / +0x30 / +0x48), unsorted otherwise.
MAP TOWERS 12/12 (2026-11-15, third batch): + map_cubism_sniper_204_r40, map_highfinance_mortar_204_r40 (point on the nearest targetable segment), map_quad_druid_204_r40, map_bazaar_alch_204_r40. DONE (quad druid, Heart of Thunder bolts on a map with invulnerable entrance and exit stretches): (1) the bounce search (BloonManager.GetTargets) skips nonTargetable bloons and lists only targetable segments; (2) its brackets are unsorted, path by path in PathManager order, each path's segments entrance first (the clone sorted by distance until leak across paths; on one path both agree); (3) PathSegment.GatherPathInfoData (@0x180f6ce20) takes a segment's bloonsInvulnerable / moabsInvulnerable from DistanceToExtraPointInfoBloon at its centre, and Path.IsSegmentInRange (@0x180f6fa50) never lists a segment with both, nor a bloon-invulnerable one for cantTargetMoab (or a MOAB-invulnerable one for onlyTargetsMoab; not ported: no attack here sets it): Game::path_seg_inv, track_seg_inv, folded into path_seg_ok; (4) a bolt whose Target is nonTargetable sits on it and starts its delay (Lightning.Process sets hitTarget off after CollideBloons whatever happened), like the refused hit.
MAP TOWERS (2026-11-15, set pending/maptowers): other towers on maps with mechanics, at the dart scenarios' spots: map_kartsndarts_bomb_204_r40 (tunnel), map_workshop_boomerang_204_r40 (belts, chained paths), map_sunkencolumns_wizard_024_r40 (tower on raised terrain), map_bazaar_ninja_204_r40 (two spawners). Recorded back to back: maptowers 4/4 MATCH. Second batch: map_workshop_spike_204_r40, map_cargo_ice_204_r40, map_erosion_glue_204_r40 MATCH (maptowers 7/7). DONE (workshop spike): PathManager.GetPathSegmentsInRange (@0x180f6ba10) skips a path only when Path.isHidden (+0x21) and not isActive (+0x20), so a TrackTargetSupplier / TargetTrackOrDefault also lists an inactive path's segments (the Spike Factory's piles land on P3-Extra; pops 0, weak coverage). import_paths.py now writes map.json path "hidden" (put on the 6 local maps with hidden paths from their mapdump dumps); Game::idle_segs / visible_points (the bloon targeting's own segment lists still have only active paths: no bloons on the others). DONE map_kartsndarts_engineer_204_r40 (maptowers 8/8): the First / Last leader (Path.Process's best, PathManager.FindGlobalBest) is worked out from the bloons' frame-start distances (path_leads uses prev_dist): Path.Process runs near the top of Simulation.process. SimRecorder probe "proclist_from" / "proclist_to" (recorded frames) writes PR lines (Simulation.process in order: index, type, id, destroyed); pending/maptowers_probe/map_kartsndarts_engineer_204_r40_pr.rec shows CollisionChecker, Removeables, PathManager, Spawner, ..., then towers / weapons / bloons / projectiles in creation order (Sentry 2606 after Black 2259), so at rt 2833 the Sentry acts after 2259 moved past Green 2015 and still takes 2015, the leader as the frame began. DONE (sunkencolumns wizard_024): NecromancerZone.OnBloonDegrade (@0x180d4c220) skips a degrade only when the graveyard manager's last (bloon Id +0x14, layerId +0x170) is the same, so a bloon degraded twice in one frame gathers twice (rt 1599: Yellow 1038 to Green to Red; Game::grave_last); the first child stays subscribed (the object's delegate). This replaces HYPOTHESIS P-GRAVESUB (a heard degrade used the subscription up). Note: SimRecorder logs BufferedRandom.Next(max) as R m lines (the graveyard pick, the undead's path draw): scratchpad rngx-style aligners must include them.
MAPS 56/56 (2026-11-15): + CoveredGarden, GlacialTrail, LastResort, LostCrevasse. DONE CoveredGarden: CycleAreasAction ported (PerformAction @0x180f7b680; index = GetCurrentRound / roundsPerRotate; with invert area (k - index) % n for k = 0 destroyed, the rest made: INFERRED order from the recording, round 40 without WindowUpperLeft, round 41 without WindowLowerLeft; at a round's end GetCurrentRound is already the next round's index). AfterRoundTrigger.OnRoundSet (@0x180f7a500) now re-checks TriggerCond(1) when the round is set (logic_round_set: the compare harness and set_round; RotateArea turns at once there, Geared). Map.GetTerrainHeight counts unplaceable areas for the tower's z (probe RT lines: z 200 in WindowLowerLeft, 0 once it's destroyed). CoveredGarden's map.json got area ids (two BushesUnplaceable told apart by position). DONE KartsNDarts (pending/maps/map_kartsndarts_dart_205_r40 MATCH; probe maps_probe/*_seg): the tunnel's points 276.6-345.8 have PointInfo.bloonsInvulnerable (map.json path invuln / moab_invuln, 9 maps; Game::track_inv). Bloon.Process sets nonTargetable (Bloon +0x218) from DistanceToExtraPointInfoBloon @0x180f6dc40: point i AND point i+1 flagged (bloon_nontarget), MOAB-class from moabsInvulnerable. A nonTargetable bloon is skipped by targeting (the 2-0-5 drops its Ceramic at rt 428, retargets at 437) and by Projectile.IsCollisionValid (@0x180cac350) unless ProjectileModel.ignoreNonTargetable (+0x78, exported as ignore_nontarget): darts touching a Ceramic in the tunnel do no damage and spend no pierce (rt 438).
MAPS 51/52 (2026-11-15): + AnotherBrick, Balance, Cracked, Ravine, Blons, BloodyPuddles, Carved, Infernal, CandyFalls, FiringRange, FrozenOver (recorded back to back: the owner wants no pauses). DONE Balance: Spawner.StartRound (@0x180f753e0) resets the spawn junction and PathManager.Reset (@0x180f6c840) every path's entry / exit Splitter.nextPathIndex to 0 each round (Game::routs cleared in start_round). OPEN CoveredGarden: 60 projectile position frames differ (bloons exact).
MAP TOWERS 67/67 (2026-11-17): + map_shipcapture_engineer_204, map_mushroomgrotto_spike_204, map_midnightmansion_sniper_204, map_darkdungeons_tack_204, map_sanctuary_ice_204, map_sulfursprings_bomb_204, map_tinkerton_wizard_204 (all _r40) MATCH first time; map_threeminesaround_alch_204_r40 MATCH after: Bloon.Move sets the whole Position, so a bloon's z (height) is the track height at its new distance, set in move_bloon_track where its position is (the update at the start of the move had it one frame stale: rt 449, a MOAB at 217.7077 on z 121.842, not 121.897; the potion's ArriveAtTarget eased over the 3D distance to it). A wind carry or a trance keeps its own z (they return before the track move).
MAPS 82/82 (2026-11-16): every map with a dump has a 2-0-5 Dart scenario (Logs has its own set): + SkullTweak, SpiceIslands, Streambed, SulfurSprings, TheCabin, ThreeMinesAround, Tinkerton, Underground, WaterPark, WinterPark, all MATCH first time. AncientPortal and MuddyPuddles re-spotted with gen_map_scenarios.py --near (their first recordings' bloon positions; old ones in pending/maps_old): both MATCH with real hits now. Next: other towers on the newer maps (maptowers), starting with maps with mechanics (ShipCapture, MidnightMansion, MushroomGrotto, DarkDungeons, Sanctuary).
MAPS 72/72 (2026-11-16): + PartyParade, PatsPond, Peninsula, ProtectTheYacht, QuietStreet, Sanctuary, Scrapyard MATCH first time; ShipCapture MATCH after: ShipCaptureController ported (map.json logic.ship_capture from export_map_mechanics.py: each side's paths; ShipCaptureControllerModel.GetLeft/RightShipStatus @0x181131a10 / @0x181131a20 by round index; OnRoundSet / OnRoundStart -> UpdateActivePaths @0x180f8f010: a side's paths active iff its ship is docked, on a state change), with path deactivation (Action Path active=false: LogicState.inactive, the spawn route and spawn tracks skip them; bloons on them carry on); and Game::leak_dist uses maximumDistanceUntilLeak (until_leak) only for a path with an exit router (PathSegment ctor @0x180f6d050), else leakDist (ShipCapture's untilLeak is float.MaxValue: First tied every path). Remaining maps without scenarios: SkullTweak SpiceIslands Streambed SulfurSprings TheCabin ThreeMinesAround Tinkerton Underground WaterPark WinterPark.
MAPS 64/64 (2026-11-16): + AncientPortal (pops nothing: weak), Ascent, DarkDungeons, LuminousCove, MuddyPuddles (pops nothing: weak), OffTheCoast MATCH first time; MidnightMansion MATCH after: Path.speed_mult takes its segment from the quick table (quick_point), so just past a point t runs over 1 (rt 81: a MOAB 0.0026 past the 10x -> 1x point steps at x 0.999825), and its map.json logic was stale (re-exported: the CycleAreasAction event that swaps the magic-fire blocker pairs every 2 rounds; SimRecorder AA lines, a new per-frame area-change probe, show the swap at the round change, rt 819). MushroomGrotto MATCH after: EnablePathTargetZoneAction (EnablePathTargetZone @0x180f7c900) ported: the dark track (PointInfo.disablePathTargeting everywhere) is targetable on the segments within an enabled zone's range of its object; enableOnInit zones at the start (map.json logic.target_zones, action EnableTargetZone; export_map_mechanics.py). A scratch re-export of every map's mechanics changed no other events. Remaining maps without scenarios (21): OffTheCoast done; PartyParade PatsPond Peninsula ProtectTheYacht QuietStreet Sanctuary Scrapyard ShipCapture SkullTweak SpiceIslands Streambed SulfurSprings TheCabin ThreeMinesAround Tinkerton Underground WaterPark WinterPark (each: import_paths.py, gen_map_scenarios.py, record).
MAPS 40/40 (2026-11-15): Erosion DONE. RotateToTarget.ApplyRotation (@0x180dd7930) aims from the tower's Position through Vector3.ToDisplaySpace (display y + 10, or the throw-marker height), then ToSimulationSpace: in f32, y' = (y + z / tan60) - (z + 10) / tan60, which rounds differently from y - 10 / tan60 once z isn't 0 (aim_origin_y; probe pending/maps_probe/map_erosion_dart_205_r40_sr: RT lines show the tower at z 10, target (51.395832, 56.187622, 128.2) -> -115.85982). Used at the main aim sites, the ability turn and rotate_always; GetOrigin (offset_rot) sites unchanged. Next: more maps (about one batch of 4 an hour), the 7 maps without dumps, other towers on raised maps.
MAPS 39/40 (2026-11-15): + FloodedValley (pops nothing: weak), SunkenColumns. DONE SunkenColumns: a tower's Position.z is Map.GetTerrainHeight where it stands (InputManager.QueryPlacementPosition @0x180f83240: the first active area holding it, unplaceable / type 5 skipped, height + renderHeightOffset); lines of sight start at that z (Attack.GetPathSegmentsInRange @0x180dad8f0; Map.IsTargetLineClear skips areas no taller) and so do its shots (tower_height). Game::terrain_z, Area.render_offset. STAND-IN: the origin's entity height term (vtable 0x228) taken as 0; z from where the tower is now. OPEN Erosion (new spot (134, 102), pops 2015; old recording pending/maps_old/map_erosion_dart_205_r40_spot2.rec): rt 582 a projectile's y is 95.56948 for the game's 95.56949 (1 ulp; bloons stay exact), 51 position frames. The tower's rotation is already an ulp off (-115.85983 for -115.85982) aiming at MOAB 131 (z 128.2 on Erosion's raised path); the same 1 ulp before the terrain-z change. Ruled out: adding the tower's terrain z (10) to the aim lift (DISPLAY_TOWER_HEIGHT) throws everything off, so the lift stays 10. Next suspect: the MOAB circle offset (kind_rot / mrot) on a path with z, or the target point CanHitMoab picks.
MAPS 38/38 (2026-11-15): Geared DONE (back in pending/maps). RotateAreaAction ported (map_logic.rs Action::RotateArea, logic_spin / logic_turn_area): Initialise rotationPerFrame = rotation / duration x 1/60 (the model's radians run the other way), durationFrames = duration x 60; PerformAction queues the next frame; Process turns the area by (spins running) x rotationPerFrame until durationFrames pass; Map.RotateArea turns the outline / holes about AreaModel.pivotPoint and each tower on it (Vector2.RotateAroundPoint by -angle, f32 sin / cos; RotateTower adds the angle to its rotation). Exported by export_map_mechanics.py (RotateArea with the pivotPoint object's world position, world_pos); Geared's map.json got its area ids by name so its logic exports. STAND-IN: the start-round catch-up (GotoPostAction with the trigger count) turns the areas only; the game's round-40 start state disagrees with a plain count (tower +90 deg, outline 225 deg), so the compare harness places a tower where the game's first W line has it on maps whose logic turns areas (logic_turns_areas). Not modelled: pets / grounded air units / tower-made areas turning.
MAPS 37/37 (2026-11-15, Geared moved to pending/maps_open): + MiddleOfTheRoad, Encrypted. CAUTION: the real game froze during FloodedValley (the ~19th launch in ~70 min, batches of 4 with 90 s pauses): batch stopped, game closed. Not yet recorded: FloodedValley, SunkenColumns (scenarios exist). Keep to about one batch of 4 per hour. Stopping a batch's background task leaves record.sh running (it relaunches the game): kill it too.
MAPS 35/36 (2026-11-15): + SpaPits, Chutes, AdorasTemple. OPEN Geared (pending/maps_open/map_geared_dart_205_r40, out of the maps set): MapEvent 1199 (AfterRound 1, everyNthRound 1, repeating) runs RotateAreaAction 1197 (area 1186, 45 deg over 4 s) and 1200 (area 1204, 108 deg); a tower on a gear turns with it (the game's W position moves from rt 803: (86.75, 10.75) on; the scenario asked for (-8, -68), so the start-round catch-up / StartMap already moved it). The clone has no Geared logic at all (map.json has no "logic": export_map_mechanics.py's areas didn't line up) and no area / tower rotation. Needs: RotateAreaAction (decompile), the catch-up rotations, towers on rotating areas, then re-check. Use a different map for coverage meanwhile. Decompiles in research/dumps/decomp/RotateAreaAction/: PerformAction @0x180f8d600 (pushes round+1 onto spinList, adds the Process once), Process @0x180f8d720 (per frame; a spin ends after durationFrames), RotateAreaBy @0x180f8db70, GotoPostAction @0x180f8d190 (catch-up: Map.RotateArea(area, n x rotation - currRotation) then the models); still to read: Map.RotateArea / RotateAreaAroundPivot (towers, paths, blockers on the area).
MAPS 32/32 (2026-11-15): Workshop DONE: First across chained paths counts from Path.maximumDistanceUntilLeak (the exit path's leakDist, else totalPathLength + the exit router's paths'; PathSegment ctor @0x180f6d050 sets distanceUntilLeak = it - the centre). Imported from the dump as map.json path "until_leak" (import_paths.py; all 83 maps patched), Game::track_until; leak_dist and seg_until_leak (was HYPOTHESIS P-SEGLEAK: length - centre) use it. Next: more maps in batches of 4 (left without a recording: run gen_map_scenarios.py on the remaining mapdump maps), and the 7 maps without dumps.
MAPS 31/32 (2026-11-15): + Spillway, SunsetGulch, BloonariusPrime. DONE BloonariusPrime: import_paths.py matched dumped paths by nearest first point and swapped PathStandard / PathBoss (same start; also SpaPits' PathBeginner / PathE): it now matches by name first (dump names can hold spaces: Quarry's "PathStandard 1"; read up to "active="); both maps re-imported. DONE (part) Workshop: MapMods.bloonSpeed 1.5 -> Map.mapWideBloonSpeed (+0xc0), a factor of Bloon.get_TotalSpeedFrames (@0x180fb8c20; the clone had 1): map.json "bloon_speed" (export_map_mechanics.py; only Workshop has MapMods), Game::map_speed in the move, knockback redo, pop children's first step and bird release. And every path's Path.spawnDistance / leakDistance now come from the dump (import_paths.py "spawn" / "leak", -1 = none; all 83 local maps patched; MapPath.spawn / leak; dropped in reverse mode): Workshop's PathStart crossing gave 71.06 for the game's 71.07. OPEN Workshop rt 1727-1728: the game's 2-0-5 fires at rt 1727 aiming down-right (projectiles at (101.5, 54.4) / (104.1, 56.0) at 1728), the clone fires at 1728 at Zebra 1524 (440, x 57.97); all the Zebras are on the belts at y = -1. Check which bloon / path segment the game targets (First across Workshop's 15 chained paths: untilLeak per path?).
MAPS 29/29 (2026-11-15): + Erosion re-recorded at (102, -48) (still pops nothing: round 40's bloons don't pass it; weak coverage), OneTwoTree, Cargo, Quarry. DONE Cargo / Quarry: a path the map logic turns on (Path action: Cargo's PathSpecial, Quarry's PathMoab, by AfterRound 39 or PreBloonSpawn Moabs) got only its Track; track_ends / track_segs / track_rot / track_seg_ok now get its entries too (Game::path_ends / path_seg_ok), so its bloons are held at its spawnDistance - radius x 1.25 like any path's (the MOAB at 106.24 = 132.49 - 26.25). gen_map_scenarios.py also falls back to 30-40 from the track (Spillway's wide channel). Next batch recording: Spillway, BloonariusPrime, SunsetGulch, Workshop.
MAPS 25/25 (2026-11-15): + Tutorial, Hedge, EndOfTheRoad, TownCentre (one batch of 4, 90 s apart: no NK trouble). DONE Erosion's refusal: AfterRoundTrigger.triggerOnceIfRoundMissed (TriggerCond @0x180f7a5b0, CheckIfTriggered @0x180f79f20: TriggerCond(0) || TriggerCond(1) at the start, then the trigger count; with the flag, a round index past `round` fires once and sets hasTriggeredFromRoundSkip, after which a count of 0 always fires). The clone's catch-up only fired on the exact round, so a round-40 start kept Erosion's round-18 / 36 land (LandArea 4067 under the refused (-44, 78)). Exported as AfterRound "missed" (export_map_mechanics.py; the 8 local maps with such triggers patched from mechanics.json: erosion, onetwotree, cargo, quarry, polyphemus, bloonariusprime, sunsetgulch, workshop). gen_map_scenarios.py applies AfterRound area actions fired by the start round before picking a spot (Erosion now (102, -48) on LandArea (4); old recording in pending/maps_old). Next batch recording: Erosion, OneTwoTree, Cargo, Quarry.
MAPS 21/21 (2026-11-15): + DarkPath, Downstream, Polyphemus, LotusIsland, Erosion. gen_map_scenarios.py now judges a spot by the FIRST area holding it (Map.areas order; BaseArea last is the fallback, water on Polyphemus) and 8 footprint points, widening to 12-30 from the track when 16-22 finds nothing. OPEN Erosion: the first spot (-44, 78) on a LandArea the game refused (no W lines; INFERRED: that land has eroded by round 40 via map logic), the clone placed it anyway; the scenario now uses (144, -48) on LandArea (5), which pops nothing (weak coverage). Model the erosion events / placement validity, then re-record a spot in range.
MAPS 16/16 (2026-11-15): + Mesa, HighFinance, Rake, Bazaar. Bazaar has two PathSpawners on one object; GetComponentInChildren takes the first (AlternateRoundSplitter: round 40 on Path2, 41 on Path1), export_map_mechanics.py took the last (fixed; only Bazaar has two). That exporter also wiped map.json "logic" when areas weren't in scene order (they are now in Map.areas order): it now accepts areas whose ids match (Bazaar's logic restored). CAUTION: re-running export_map_mechanics.py on other maps was safe only after that fix.
MAP COVERAGE (2026-11-15): tools/mod/gen_map_scenarios.py <mapdump dir> <Map> ... writes map_<key>_dart_205_r40.json (a land spot 16-22 from the track, 6 clear of track / water / unplaceable areas, nearest the path's middle). ParkPath, AlpineRun, Resort, Skates recorded: maps 7/7 MATCH. Spillway: no spot found (check its area types). Next: more maps in batches of 4-5 (each launch logs in to NK), then other towers on maps.
DONE (2026-11-15): 83 maps have the game's own paths / quick tables / area outlines and order imported
(tools/mod/scenarios/mapdump/mapdump_<key>.json, 60-frame runs, dumps in research/dumps/recordings/pending/mapdump;
then import_paths.py <Map>.map <key> for each). LEFT (their dump runs failed: no mode screen): -ouch (its map id
isn't the key; try "#ouch"), castlerevenge, darkcastle, enchantedglade, springspring, trickytracks, xfactor
(probably locked / special maps in this profile). A failed run leaves the game waiting on the menu: close it.
CAUTION: that ~85-launch batch left the real game stuck on its title screen (each launch logs in to NK): record in
small batches with pauses, no retry loops.
Next: the next hero / tower if the shop gains one; otherwise more crosspath coverage (e.g. Quincy with
other towers' MOAB-class targets on curves, where this shared offset matters most).

### Next tower: Skywarden (2026-11-13)
All towers and 12/12 paragons are exact. Skywarden has 34 scenarios (tools/mod/scenarios/sky_*.json) and no recordings: recording them into pending/sky (2026-11-13), then compare (`sh tools/rec/compare.sh sky`) and work through them. First findings (sky_000, before the batch finished): weapons same, first off at rt 21: the bolt slows 7.5 -> 7.167 -> 6.833 (TravelStraitSlowdown active: DamageBasedAttackSpeed.CreateMutators @0x180d19990 makes a mutator per stack from 0 and one is always on, so the models are run-time copies); DONE (2026-11-13): slow_frames for towers with damage_speed; damageCap (OnBloonDamaged @0x180d19ee0) as damage_speed.cap in rage_add; the 4xx BurstWeaponIncreasingArc as burst + arcs (randomAngle per shot); CreateDistanceProjectileOnExhaustFraction n = (int)(|Target.Position (pr.aim) - tower| / distanceInterval) (CheckFraction @0x180c9d8a0). MATCH (pending/sky, 34 recorded 2026-11-13): sky_000 (+ close / last / strong), 001, 002, 010, 020, 030, 100, 200 (11/34). Distance projectile: n from the Target's Position after the bloons moved (spawn_exhausted; sky_030 rt 557), with the aimed collision circle (Exhaust.target_off from pr.inst_off, set for arrive shots too; sky_040 rt 47: MOAB front circle 48.94, n 4). ArriveAtTarget z eases to the Target's Position.z (arrive_tz; sky_040: the arc climbs 7 -> 119.6 to a MOAB, so its blast at z 120 is over the entrance trees). DONE (2026-11-13): Galvanized stages + stun (Game::galvanize after the hit's damage; stage 1 on a projectile's first hit, then min(v, 2) + 1, Stun on stage 3 + first hit: stunDamage + a SlowMutator for mutatorDurations[bloon id] (from the GalvanizedProjectile's BloonTagDamageOverrideModels: Moab / Ddt 2, Bfb 1, Zomg 0.5 for the x-4-x) or stunDuration): sky_040 / 032 MATCH (13/34). NOT YET: GalvanizedBehavior's magnet / chain lightning, the stage passed to children. WINDLASH (3xx and the 4xx / 5xx bolts; decompiled research/dumps/decomp/Windlash + WindlashBehavior): Windlash.Collide (@0x180cf8360), if Bloon.WillDamageBloon: no Windlash mutator -> mutators[0] only on this projectile's first hit (hitBloon); with one of stage v (mutator +0x84): v > 1 -> EmitArrows (@0x180cf87d0: the WindlashArrow projectile, pierce 4 damage 1, e.g. sky_400 rt 102 two at the hitting bolt's direction +-45 from the bloon), v > 2 -> EmitWindVacuum (@0x180cf8f10); then the old ones removed and (first hit ? mutators : mutatorsWithoutWind)[min(v, 2)] added. WindlashModel: windlashDuration 2, distanceMin 10 / Max 20, windInterval 0.5, speedMultiplier 1, affectMoab false. The bloon side, WindlashBehavior (model fields: distanceMin +0x30, distanceMax +0x34, speedMultiplier +0x38, windInterval +0x3c, windlashDuration +0x40, stage +0x44, an ignore-MOAB flag +0x48, applyWind +0x49 (the `mutators` set have it, `mutatorsWithoutWind` not)): Initialise: lifetime 0, windActive false, stage, applyWind. Process (@0x18103c220): lifetime + 1; at (int)(duration x 60) the Windlash mutators go; else ProcessStage1, then with applyWind and not windActive interval + 1 and at (int)(windInterval x 60) StartStage1. StartStage1 (@0x18103c530): unless windActive / !applyWind / MOAB-class (unless the flag) / tag bits 0x1000 or 0x8000 / !Bloon.IsBloonValidForPushback: windActive, interval 0, offTrackCount + 1, Bloon.Move(-(BufferedRandom.NextDouble x (max - min) + min)) (max above a constant: the bloon's distanceTraveled instead). ProcessStage1 (@0x18103bf10): while windActive, target = Path.DistanceToPointInfoBloon(distanceTraveled); within (speedFrames x const)^2 of it: offTrackCount - 1, windActive false, interval 0; else position += Normalize(target - position) x bloonModel.speedFrames x speedMultiplier. PORTED (2026-11-13): Game::windlash_hit (after the hit's damage: arrows at stage > 1 from the bloon, with the hitting projectile's Rotation as emit_rot and emit_dir_rot (its ArcEmission carries EmissionRotationOffProjectileDirection [-45, 45, centre]) and its z; then stage min(v, 2) + 1 / 1, applyWind on the projectile's first hit) and Game::windlash_step (start of move_bloon: lifetime, interval, StartStage1 through push_back + the MoveWithWind carry, windActive cleared when the carry ends). sky_320 now off at rt 298 (was 223), sky_420_close at 153 (was 79). The arrows' spread: ArcEmission.GetDirection (@0x180c37590) takes the rotation its behaviours give (EmissionRotationOffProjectileDirection.OnGetDirection @0x180c3c100: k x angleInBetween + the projectile's Direction rotation + start, centred -(n-1) x step / 2) and adds - angle / 2 + offset + k x slice on top: the 3-0-0 / 4-0-0 arrows (angle 45, [-45, 45, centre]) +-45, the 3-2-0's (angle 0, [45, -45, centre]) 88.06 / 43.06; now sky_320 off at 298, sky_400 at 105, sky_420_close at 153. DONE (2026-11-13): EmitWindVacuum (stage 3): the WindVacuum made at the bloon's Position (sky_400 rt 176: (-100.73, -22.9)), then travelling along the nearest path segment's track from its distance; DamageBasedAttackSpeed.Process runs after its tower's weapons (update_rage after towers_phase: sky_400's rate goes 72 -> 66 at rt 177 but the burst's next shot waits for 178); the alwaysEmptyClip override is armed by IsReloadReady as the weapon's frame ended, with that frame's rateFrames (`empty_chk`), and its point is raised by the weapon's ejectZ with useThrowMarkerHeight (`empty_raise`); the old WindlashModel -> WindModel stand-in in build_game_data.py removed (it blew a popped MOAB's Ceramics back at once, sky_500 rt 571). sky_400 now off at rt 1928 (was 176). DONE (2026-11-14): Windlash runs in collisionPass -1 (before the DamageModel): a hit that pops a MOAB still sends its stage-2 arrows (sky_500 rt 571); the arrows may hit the bloon they came from (sky_400 487 / 497 on 432), the vacuum may not, nor its children (layer pushed; sky_520 rt 301: H lines with pierce unchanged, no wind roll); the mutators are kept through a degrade and copied (`parent_wl`; a copy's behaviour not processed in the pop frame: `fresh`); WindlashBehavior.Process runs after the bloon's own move (move_bloon -> move_bloon_track, then windlash_step) and ends, every frame, with Position += Normalize(track point - Position) x speedFrames (Process tail @0x18103c220), so a carried bloon goes 2 x speedFrames a frame and the push frame steps back once (sky_300 rt 2585-2588 exact); StartStage1 clamps at spawnDist - radius x 1.25 and checks IsBloonValidForPushback; CollideReducePierceForBloonStateModel (pass -1, before the immune bounce) draws one synced NextDouble per hit (Collide @0x180c9cf60; its percentChance 50 against a 0..1 draw never adds pierce; `pierce_draw`, sky_300 rt 28 on a Lead); the old WindlashModel -> WindModel stand-in is gone. Draws in sky_300 now match to rt 2642. OPEN (probe recording research/dumps/recordings/sky_300_wl.rec, SimRecorder WI / WD / WS lines: WindlashBehavior Initialise / OnDestroy / StartStage1 with object pointer and interval): (1) DONE (2026-11-14): StartStage1 runs right after Initialise, inside Windlash.Collide (pass -1, before the damage) on the hit bloon (sky_400 rt 2008: a push draw on a Blue the hit destroys), and on each copy a degrade of a Windlash bloon makes (`WlState.start`, run after the hit's damage in child order; validity at the copy's spaced distance: sky_400 rt 3050, a Zebra's fourth Pink near the entrance not blown); a re-hit's old behaviour going ends its carry (OnDestroy: offTrackCount - 1); copies of a blown bloon aren't carried (a Windlash carry isn't a mutator) and show at its Position until their first Process (`at_parent`), else on their track spot (sky_300 rt 2555); a MOAB-class pop drops the Windlash (sky_500 rt 571, sky_520 rt 301). Now sky_300 to rt 3350, sky_400 3068, sky_320 2081, sky_500 747, sky_520 309. DONE (2026-11-14, sky_300 MATCH, sky 14/34): the sky_320 rt 2081 "double push" was the WindVacuum's own WindModel (distance 20-40, speedMultiplier 2: carried 2 x speed + the Windlash tail); a WindlashBehavior made in this frame's collisions doesn't process until the next frame (`fresh` on new behaviours; sky_500 rt 747), but an older one does on a bloon a WindModel just blew back (sky_320 rt 2081); only copies are fresh after a degrade (the first child keeps the bloon's behaviour); a behaviour expiring while it carries its bloon goes before ProcessStage1 (no move that frame) and OnDestroy ends the carry (sky_300 rt 3350). Now sky_320 to rt 2360 (1 ulp in the tail step's y), sky_400 to 3068 (a pooled behaviour's stale interval: Pink 3489 made at 3050 blown at 3068), sky_500 773, sky_520 581. (2) DONE (2026-11-14): Initialise does NOT reset interval (0x78), so a pooled WindlashBehavior comes back with its old one. Factory`1<WindlashBehavior> (shared generics; SimRecorder's map dump now logs factory_method addresses: Flush @0x1829c94d0, Create @0x1829c8be0, CreateBehavior @0x1829c8540 in this build; fields up +0x10, pool +0x18, freePtr +0x38, createdSinceLastFlush +0x3c): Create: created + 1, pool[freePtr++] when one is free, else a new object appended; Flush(false) every 60 frames (FactoryFactory.Flush, FF lines): each destroyed object in [0, freePtr) swaps with pool[--freePtr] (index rechecked), then the free tail keeps max((int)(0.9 x free), min(created, free)) (0.1 @0x183e4fef0) and the rest are dropped; up = pool[0, freePtr). Replays every WI / WD of sky_300_wl and sky_320_wf (SimRecorder WF lines: freePtr, created, up, pool) exactly. Ported: Game::wlf_* (create in windlash_hit and for copies after the hit's damage, destroy on expiry / re-hit / bloon removal / a pop that ends the object (damage wrapper)), flushed with the other factories. sky_400 now to rt 3454. OPEN: sky_400 rt 3451: two Windlash interval pushes in one frame take the frame's draws in the other order (the clone's bloon process order); sky_500 rt 773: four Windlash arrows each pop four stacked bloons in the game, the later ones hitting children the earlier ones made in the same collision pass (1494, 1503 ...); the clone's arrows don't find those children (CollisionGrid membership of a new child: its object's dead bloon still in the grid?). DONE (2026-11-14): sky_520 MATCH (sky 15/34). Weapon.Process (@0x180c50670) uses overrideTargetPosition whenever it is set (ClearTarget, SetTarget at that point) and clears it at the end of every Process; the miss was the clone's fallback for a frame whose weapon pass left no record: it checked IsReloadReady with this frame's rate (85 after a rage stack) instead of last frame's (93), so it aimed at the override point; the check is now taken at the start of the tower's turn, before the rate is recomputed (WeaponState.empty_chk). Burst shots thrown at the empty-clip point (its Target at (x, y, 20), 20 @0x183e4fc48) are made at z 20 and eased to 20 (WeaponState.empty_throw; sky_420_close rt 318, emittedFrom.z 20), and CreateDistanceProjectileOnExhaustFraction measures to that point (the shot's exhaust spot) when the Target isn't a bloon (rt 322: pierce 4 + 5); sky_420_* now to rt 2305-3021. DONE (2026-11-14): every Factory`1 trims its free objects at the flush (max((int)(0.9 x free), min(createdSinceLastFlush, free)) kept, the rest dropped: trim_pool for Factory<Bloon> and Factory<Projectile>); sniper_paragon_r40 now to rt 1868 (was 642), every synced draw matching to rt 3507: (a) AddBehaviorToSingleBloon.Collide (@0x180c81f70) with chance > 0 draws one NextDouble (the Insights' chance is 100 against a 0..1 draw) after applyOnlyIfDamaged's WillDamageBloon; with firstHitOnly it returns once its object hasCollided, which a pooled object keeps: the Insights mark goes on once in the whole run (paragon_probe: one M add, rt 181), ported as Projectile.single_used / Game.single_free (objects freed at the projectile flush); (b) the Supply Drop: the attack's random target drawn the frame the between-rounds weapon fires (rt 1622, 12 draws), the EmissionOverTime (initialFireDelay 2 s, 0.5 s apart, 10) with EmissionSupplyRandomTarget (OnEmit @0x180c3ce60, GetTargets MoveNext @0x180c4d360: r and angle draws, IsPointOnMap(-mapBorder), 100 tries): crate at a spot 7-400 from the tower, its Target 7-30 on (the crate goes there next frame, z 2); the crate's CreateProjectileOnExhaustFraction (durationfraction 0.3 of 5 s) is another EmissionOverTime (10 Collectables 6 frames apart, each arriving at a spot 7-30 from the crate, altSpeed 100). DONE (2026-11-14, sniper_paragon_r40 MATCH, paragons 12/12): (c) a shrapnel turning for a bloon made in the same frame after it (the Ceramics of the MOAB its shot popped) aims at its spot before it moved (born, then cseq: sniper_paragon_r40_pool rt 213); (d) an EmissionOverTime made by a projectile's exhaust (the crate's Collectables) goes on in that projectile's Process, after the towers (po rt 2104: the paragon's shot and shrapnel get their pooled objects first, so the stale emmisionIndex 3 that seeds TrackTargetAfterDelay's System.Random(emmisionIndex + time) went to the wrong shrapnel at 1856); (e) a weapon's EmissionOverTime object is kept, so its next Emit skips the initial delay (Emit sets initalDelayStartedAt / delayCompleted only once; rt 3507: round 41's first crate the frame the Supply Drop fires); (f) what EmissionOverTime.Process makes isn't the weapon's ProjectilesCreated (no Offset / AngleToMapCenter draw on the fire-frame crate). Factory`1.Flush (@0x1829c94d0) RemoveRange's the trimmed free objects for good: the PO probe's pointers that come back later are new objects at a freed address (compare the stale emmisionIndex, not pointers: RRBTD6_POLOG prints the clone's as its 5th field). The round-end FactoryFactory.Flush(true) (FF 1) keeps only min(created, free) (trim_pool force). Probe recordings: research/dumps/recordings/pending/paragon_probe/sniper_paragon_r40_pool.rec (PO / XO, matches to rt 2952: an Explosion only in the game), research/dumps/recordings/sniper_paragon_r40_po.rec (PO / XO / FF, to rt 1022). Copies of a blown bloon also take its spot as their frame-start position (prev) for the next collision pass (sky_500 rt 773: Windlash arrows pop them; now to rt 2984). DONE (2026-11-14, sky 21/34: sky_500, 502, 502_close / last / strong, 420_close MATCH): bloons move in creation order (cseq, a first child keeping its parent's), not the clone's Vec order (swap_remove scrambles it): two Windlash pushes in one frame take the frame's draws older bloon first (sky_502_last rt 2533, sky_400 rt 3076). DONE (2026-11-14, x-5-x lightning rods; sky_050 / 250 now to rt 497-500, sky_052 to 436): LightningRodEmit.Collide (@0x180cc9580, pass -1): activeCooldown (int)(3 s x 60), counted down in Process (EmitOnDamage.Process @0x180cbbf50); if WillDamage and no tower rod within minEmitDistance 10, the rod at the bloon's Position (EmissionAtClosestPathSegment range 99999 @0x183e4fe00), the tower's rotation, the bolt's z (`rod_emit_hit`); Thunder Charge's ActivateLightningRodZap (Activate @0x180e05320, Process @0x180e059e0, Emit @0x180e053c0): duration 15 s, zapCooldown rate x 60 when no unlisted rod is left (list cleared), a found rod goes in rodIgnoreList even while animationDelay (delayPerJump x 60) only counts down, Emit from the tower (first rod) or emitPosition along a ThunderLightning line to the rod (`rod_zap_step` / `rod_zap_line`; reproduces sky_050's zaps at 121, 157, 193, 229, 249, then every 20 frames); SlowModel.chance (Slow.Collide @0x180cde360: a synced draw, nothing when chance < draw; the zap's 35% stun); Galvanized.Stun's Bloon.Damage counts as another OnBloonDamaged for DamageBasedAttackSpeed (rt 288: rate 72 -> 66), its duration is picked after that damage (a MOAB it pops: a 3 s Ceramic), the stun mutator has no layer count (children keep it) and the hit's copies take it. NOT YET: LightningRodManager.OnBloonDamaged's zaps (rods zapping a bloon within their radius hit by Energy / Plasma damage; zapCooldown 0.5 s), the ability's support zones (projectile speed, ricochet, damage bypass). DONE (2026-11-14, sky_050 / 250 MATCH, sky 23/34): the Galvanised stage goes on to a degrade's children and the hit's copies (a MOAB-class pop drops it); the collision loop follows its candidates by bloon id, as one hit's cascade (a stun popping several layers) can swap-remove more than the hit bloon (sky_050 rt 497). DONE (2026-11-14, sky_052 MATCH, sky 24/34): ActivateLightningRodZap processes after its tower's attacks (rt 496: the attack takes a Black the zap then destroys); a shot the collision pass found arrived goes off where its Target died later that frame (ArriveAtTarget.OnBloonDestroyEvent @0x180c86d60 SetTarget's a point Target; `Exhaust.arrive_target`, `destroyed_now`; rt 436); a bloon the spawner emitted last frame is on the zap's GetInLine (rt 456); a throw whose bloon died before the release eases in over the 2D distance (rt 502) and, with an emission rotation offset (the Skywarden's 0), the tower keeps its facing at the release (rt 560); offset 0 also turns the tower as the throw starts (rt 496). DONE (2026-11-14, sky_400 MATCH, sky 25/34): a WindlashBehavior processes in its own place in the process list (its creation; a re-hit makes a new one), not its bloon's: the move loop orders a bloon with a made behaviour by the behaviour's creation (`WlState.seq`; recording sky_420_strong_ot rt 2302, WS for 1557, 2473, 2294: neither the bloon factory's order, the Windlash factory's, the path list's nor the bloons' creation order). SimRecorder `"bloonfactory": true` writes BF lines (Factory<Bloon>.up with ids; up is only rebuilt at a flush, so new bloons show after it), and RRBTD6_BFLOG prints the clone's (BFC); recording research/dumps/recordings/sky_420_strong_bf.rec. The game's round-end FactoryFactory.Flush(true) empties the dead MOAB from the bloon factory (BF at sky_420_strong_bf rt 1837), but flushing the clone's bloon factory there broke sub_paragon_r40 rt 3441 (an object reused through the round end): left out, OPEN. DONE (2026-11-14, sky_420_last MATCH, sky 26/34): a Windlash carry ended by a re-hit leaves the bloon (its first child too) met by the next frame's collision pass at the carried spot (`Bloon.coll_hold`; sky_420_strong rt 2720, arrows 7 from a Blue's track spot pass it, blown 20 off; sky_420_last rt 3338, arrows 10.2 from a Green's track spot hit it, 8 from its carried one); sky_420_strong now to rt 3539, sky_320 36 frames off. INFERRED: the bloon's collision-grid entry moves only with its next Process. OPEN sky_420_strong rt 2304: Yellow 2473, new that frame at its parent's spot (-102.79, dist 227.77), then moves 1 x speedFrames a frame toward its track point with its distance held (only the Process tail, no ProcessStage1): a copy off its track (offTrackCount from its carried parent?) whose own behaviour isn't blowing; the clone carries it 2 x (sky_400 rt 3050's not-blown fourth copy instead walked its track: find what differs, e.g. with a probe of offTrackCount on copies). A weapon that gains its burst with an upgrade starts the clip full (the behaviour is new, Initialise: currentCount = count; sky_upgrades_r30 rt 1327, now to rt 2636). OPEN sky_upgrades_r30 rt 2630: the 5-1-0's second burst shot winds up at the override point (tower held at 68.69 from 2626 to 2636), so its burst check at the end of 2629 saw rate 85 (the frame's hit, H 921 882, already in), while in sky_400 at rt 177 the same check saw 72, not that frame's 66 (the tower turns at 178): find what differs (DamageBasedAttackSpeed vs BurstWeaponIncreasingArc process order per tier, or which hit changes the rate in which frame: OnBloonDamaged / currentDamageThisFrame timing). sky_300 rt 2584 (a bloon 1.8 further back). DONE (2026-11-14, sky_004 MATCH, sky 27/34): Chilled.Collide (@0x180c99c50, pass -1): only if WillDamageBloon, on the projectile's first such hit (hitBloon, `chill_hit`), nothing over its slow already on (GetMutatorById on the slow mutator's id, the MOAB one sharing it), a MOAB-class bloon with affectMoab at moabStageOneSlowMod (0.9; sky_004 rt 80). A slowed bloon's step is speedFrames of (multiplier x speed) (SlowMutator.Mutate @0x18121e5a0 set_Speed; sky_003 rt 2603, a chilled Pink's step rounding at 256); the Chilled slow goes when one hit pops 2 layers or more at once, not after two single-layer hits (INFERRED per-hit DepleteMutatorsForNumLayers: the clone's layers 2, reset to 2 after each hit; sky_004 rt 1910 / sky_003 rt 2555 and 3467: sky_003 MATCH, sky 31/34). NOT YET: the Chilled stage mutators (v > 1 SlowNearby @0x180c9b870, v > 2 EmitIceShrapnel @0x180c9a170 once, then mutators[min(v, 2)]; moabStageTwoSlowMod); Chilled.Initialise doesn't decompile (PIC branches) and disasm.py picks another method by name. sky_320 rt 2360: 1 ulp in a carried Blue's y on a vertical segment (x -50.799995 vs the track's -50.8): the clone's Windlash carry isn't WindlashBehavior's own Normalize(DistanceToPointInfoBloon - Position) x speedFrames step (Process @0x18103c220). DONE (2026-11-14, sky_005 MATCH, sky 28/34): Chilled's stage (`Bloon.chill`: set to 1 or raised to min(v, 2) + 1 on every hit that will damage, for chilledDuration 2 s, kept by children) puts the bloon in the Freeze state, and CollideReducePierceForBloonState (@0x180c9cf60) then gives the hit its extraPierce back, whatever its draw (sky_005 rt 2326 / 2328: draws 0.60 and 0.32, pierce kept; the field it compares at pierceModel +0x40 isn't the serialized 50). The stage still counts in the frame it ends (sky_205 rt 3383, a stage from 3263: sky_205 MATCH, sky 29/34). DONE (2026-11-14, freeze state, part 1): EmitOnDamageWithState (OnBloonDegrade @0x180cbafd0 / OnBloonDestroy @0x180cbb010 / TryEmit @0x180cbb160; no Collide): a degrade the projectile made (mustBePopped: of a bloon it destroyed) passing FilterBloonType (ids) and FilterBloonStates (Freeze mutators) emits FrozenRemains at the bloon (EmissionAtClosestPathSegment), the bloon in its collidedWith (`Proj.state_emits`, `Game::state_emit`); FrozenRemainsExplosionModel exported as another exhaust (same fields as CreateProjectileOnExhaustFraction). sky_025 now to rt 1997. OPEN sky_025 rt 1997: the second of a FrozenRemains' two blasts hits the Reds the first just froze (pierce used, M add of the freeze again) but does no damage; WillDamageBloon (@0x180fd6ae0) is immune & model properties (the freeze Mutator @0x1811e9860 ORs Frozen 0x10 in; the blast's immune is 12) then GetModifiedBloonDamage (@0x180fb33b0) through the bloon behaviours: the freeze mutator also adds behaviours (its +0x88 model, a FreezeImmunityRemovalModel from +0x90); one of them must zero the damage (BloonProperties: Lead 1, Black 2, White 4, Purple 8, Frozen 16, so 12 doesn't cover Frozen). Not the freeze either: at rt 2101 another FrozenRemains blast (1050) hits Red 726, frozen no more (M add ... before -1), and again only freezes it (H 1050 726 1 0); these Reds (first children of Blacks a blast popped 5 layers) take no damage from the Frigid blasts at all: look at what a 5-layer pop leaves on them (a mutator / model change: compare with Reds that do take blast damage). DONE (2026-11-14, sky_420_strong MATCH, sky 30/34): a bloon off its track spot (a copy still at its blown parent's Position) popped by a hit keeps that Position (and frame-start position) for its first child, the same object: the final respacing no longer puts it on the track (rt 3539: two Windlash arrows each take a copy, the second meeting the Green the first left at the parent's spot). Bloon.Damage and Bloon.Degrade can't be Harmony-patched (Il2CppInterop trampoline throws in Buffer.Memmove and perturbs the run). REMAINING sky groups (first diffs): x-5-x lightning rods (050 / 052 / 250, rt 49: a rod projectile at the track start); 3xx: the explosion's WindlashModel (windlashDuration 2, distance 10-20, windInterval 0.5) and, in 320 / 420 / 502 / 520, a contact ArcEmission (2 shots +-22.5 deg off the hitting projectile's direction from the bloon hit; sky_320 rt 223: 511 / 521 at 88.06 / 43.06 from (-76.26, -22.9)) plus EmissionAtClosestPathSegment (decompiled: Emit @0x180c382c0 takes the closest path segment within a constant range of the eject point and emits `count` projectiles whose TravelAlongPath starts at the segment's distance (+0x28) - i x spacing, Target set to the segment when invalid); 400 / 500: the burst (sky_400: bolts at rt 79 / 90 / 101, 11 frames apart, each at the tower's aim with 3 synced draws before it and 1 after; the angleIncreases override bent them and was removed: OPEN which emission it bends) and, from rt 102, the closest-segment shots (487 / 497 at -107.96 / -17.96 from (-139.28, -22.9)); 003 / 205: the Freeze state system; upgrades_r30 rt 1342. PREVIOUS NOTE - GalvanizedModel (on the x-3+-x blast; decompiled research/dumps/decomp/Galvanized): Collide (@0x180cbf710), if Bloon.WillDamageBloon: no Galvanised mutator -> mutators[0] (stage 1); with one, its stage value v (mutator +0x80): v > 2 and this projectile's first hit (hitBloon) -> Stun; then the old ones removed and mutators[min(v, 2)] added (stage 2 / 3). Stun (@0x180cc0270): unless the bloon has the stun mutator, Bloon.Damage(stunDamage 10) and the stun mutator for mutatorDurations[its type] or stunDuration 3 s (x 60), stunSpeedMultiplier 0. Model: duration 2 s, radius 30, turnStrength 0.3, maxAngle 160 (magnet: magnetBloonProperties Purple / Glass), chainLightningDamageTypes Sharp / Shatter -> GalvanizedProjectile (pierce 4, damage 3). sky_040 / 032 rt 303: the MOAB 188 -> 174 (4 + 10) and stopped from 304; The stages are AddBehaviorToBloonMutators (Galvanized.Initialise: id Galvanised, the model, stage 1 / 2 / 3) putting GalvanizedBehavior (Assets.Scripts.Simulation.Bloons.Behaviors; decompiled in research/dumps/decomp/GalvanizedBehavior: Initialise, OnBloonDamaged -> EmitProjectile (chain lightning on chainLightningDamageTypes), OnProjectileEnter / ProjectileFilter / UpdateRotation / IsTargetInSeekAngleRange (the magnet: projectiles within radius turned toward the bloon by turnStrength, maxAngle), Process, OnDestroy) on the bloon; the stun is a SlowMutator (stunSpeedMultiplier). then wind (3xx: WindModel / WindlashModel / TravelAlongPath / EmissionAtClosestPathSegment); sky_003 / 004 / 005 / 025 / 032 need the Freeze state system (bloons slowed from rt 28 / 49 / 80). NOT MODELLED: DamageBasedAttackSpeed.roundStartAdditionalTimeBonus (+0x80, added to the idle timeout; only reset to 0 in OnBloonDamaged in the methods decompiled; its setter not found). NOT EXPORTED AT ALL (tools/assets/build_game_data.py has none; tiers using each): every tier: CollideReducePierceForBloonStateModel, DamageImmunityBypassModel, DamageModifierForBloonStateAndTypeModel (a bloon "state" system: heat / frozen...; on the base tiers they only act on bloons already in a state, e.g. DamageModifierForBloonStateAndType: bloonStates [Freeze], types Blue / Green / Yellow / Pink, x1 +0, so a lone Skywarden's low tiers don't need it; x-x-3+ AddHeat makes states). Fields: AddHeatToBloonModel heatAmount -2000 (AddHeatToBloon.Collide @0x180c84050: OverheatTrigger.AddHeat on the bloon's OverheatTrigger behaviour, if it has one; only 10 OverheatTriggerModels exist (a boss's), so on ordinary bloons it does nothing); the bottom path freezes bloons itself from x-x-3 (FreezeModel), so the Freeze-state effects act from x-x-3 (SlowModel from x-x-4 / x-5-x). ORDER: the x-x-0..2 / top / middle recordings first (no states), then the state system for x-x-3+. 4xx burst (BurstWeaponIncreasingArcBehaviorModel interval 0.15 s / 9 frames, count 3, angleIncreases 35 / 45 / 55, alwaysEmptyClip): ProjectilesCreated (@0x180c6cf50) arcIndex + 1, currentCount - 1; under 1 it resets (count, arcIndex 0), else weapon.lastEmission (+0x6c) = intervalFrames - rateFrames (+0xa0) + now (roundTime or sim time by the weapon's flag +0x59), so the next shot comes intervalFrames later; Process (@0x180c6cb20) with alwaysEmptyClip waits for Weapon.IsReloadReady; GetRandomArcEmission picks the arc. Decompiled in research/dumps/decomp/BurstWeaponIncreasingArcBehavior. Middle path: x-4-x ActivateRicochetSupportZoneModel (ability: towers in range 48 / tower range get mutator ThunderAbilityRicochet for 9 s: shots bounce up to 2 times, Close, distance 100, delay 0.05 s); x-5-x LightningRodEmitModel plants LightningRod projectiles (AgeModel 12 s round time, radius 26 scaled by tower radius / 48) whose LightningRodModel zaps a Lightning (pierce 5, damage 40, zapCooldown 0.5 s) when LightningRodManagerModel sees Energy / Plasma damage; ActivateLightningRodZapModel (ability: rate 0.25, damage 50, piercePerJump 8, delayPerJump 0.05, duration 15).; CollideReducePierceForBloonStateModel [Freeze] extraPierce 1, percentChanceOfPierceReduce 50 (a random draw per hit); StatePoppedBasedPierceModel [Freeze] buffDuration 1 s, pierce x1.5; DamageImmunityBypassModel properties Frozen / None.; x-x-2+: StatePoppedBasedPierceModel; x-x-3+: AddHeatToBloonModel; x-x-4+: EmissionAtClosestPathSegmentModel, EmitOnDamageWithStateModel; x-x-5: FrozenRemainsExplosionModel, BloonTagDamageOverrideModel (also x-3+-x); x-4+-x: ActivateRicochetSupportZoneModel; x-5-x: LightningRod / LightningRodManager / LightningRodEmit / ActivateLightningRodZap; 1+-x-x: FilterHighestTowerTierModel; 4+-x-x: BurstWeaponIncreasingArcBehaviorModel, CamoBlockZoneModel.

Status (2026-11-15): sky 39/39 (5 new scenarios: sky_galv_dart, sky_rod_super, sky_ricochet_dart,
sky_chill_pair, sky_ricochet_r34), every tower and paragon exact. Ported for them:
- Galvanized chain lightning (OnBloonDamaged @0x180feec30 / EmitProjectile @0x180fede60): a hit whose DamageModel
  type is Sharp / Shatter emits the GalvanizedProjectile at the bloon, Target the bloon, its layer in
  collidedWith; a lightning whose Target is in its collidedWith sits on it and waits its delay; the stage and
  the chain go to a degrade's copies (first-hit stage 1 too); targetLast lightning searches Last (TargetType
  statics are 24-byte structs: none 0, first 0x18, last 0x30, close 0x48, ..., random 0xa8).
- Galvanized magnet (Process @0x180feefb0, ProjectileFilter @0x180fef1f0, IsTargetInSeekAngleRange @0x180fee990,
  UpdateRotation @0x180fef420): TravelStrait projectiles whose DamageModel immunity has Purple + Glass (Plasma /
  Energy) in the zone (radius 30, last frame's) turn by 0.3 x AngleBetween(dir, Target.Position - pos), clamped
  to 160, toward their own Target (aimed circle; a dead Target's cached spot, `aim_cache`). The behaviour runs in
  its own process-list place (`galv_seq`): projectiles made before it have moved (turn for their next move),
  newer ones haven't (turn and redo this frame's move); a new behaviour (every stage set / raise, a copy's)
  fills its zone from its second Process (`galv_zone_skip`).
- LightningRodManager.OnBloonDamaged (@0x180d32ea0): another tower's Energy / Plasma hit (the bloon popped or
  not) within a live rod's radius emits the rod's Lightning at the bloon; cooldown zapCooldown x 60.
- ThunderCharge's ProjectileSpeedMutator (@0x180e25680) ADDS speedIncrease to every TravelStrait speed
  (`speed_add`; the dart 350 + 200).
- Chilled.SlowNearby (@0x180c9b870) on a hit of a bloon past stage 1: bloons within slowRadius 8 of the
  projectile's frame-start spot get a radiusMultiplier 0.8 slow of their own (none in the projectile's first
  collision pass: its zone fills in its Process); EmitIceShrapnel (@0x180c9a170), once per projectile past
  stage 2: IceShards from the hit bloon at the shrapnelCount closest other bloons within shrapnelRange.
- ThunderCharge's ricochet zone (RicochetMutator.Mutate @0x180e26bf0): every TravelStrait, non-Instant
  projectile of a tower in the zone gets the zone's RetargetOnContact (distance 100, 2 bounces, delay 0.05 s,
  Close), after its TravelStrait (`Buffs.ricochet`); sky_ricochet_r34 bounces from rt 767 MATCH.
Skywarden: every behaviour the scenarios reach is exact. Next: the next tower (or more Skywarden
crosspaths if a new behaviour turns up).
- DONE: MoveWithWind is its own bloon behaviour in creation order: an older WindlashBehavior steps before a
  MoveWithWind attached later (sky_320 rt 2360, y - 0.729 - 1.458 vs - 1.458 - 0.729, 1 ulp). sky_320 MATCH.
- DONE (INFERRED): FrozenRemainsExplosionModel's blast is exported with damage 0 (in every pair the second
  blast hits without damage: sky_025 rt 1997 / 2101 / 2131). sky_025 now first off at rt 2100.
- DONE (INFERRED): ChilledModel.pierceCost -1: any projectile with a DamageModel gets its pierce back
  on a chilled bloon (sky_025 rt 2100 / 2272: blasts pop chilled Blues for free, the damage-0 one too; the
  FrozenRemains, no DamageModel, pay). The refund's code path isn't found (not in Projectile.CollideBloon,
  Damage / Freeze / Chilled.Collide or ChilledBehavior). rngcmp's "first diff at 1892" is a false alarm
  (the game logs that draw as "R r 120 240 124").
- DONE: a frozen bloon alone refunds nothing (sky_025 rt 2156, a bolt pays on six frozen Reds; the bolt's
  CollideReducePierceForBloonState 50 never passes); no chill refund on MOAB-class (rt 138). Exhaust
  projectiles of shots spent earlier in a collision pass are made before a later FrozenRemains
  (Projectile.Exhausted runs OnExhaust at once, VERIFIED-DECOMP @0x180c8c570; rt 2491). sky_025 to rt 3155.
- DONE: an immune hit destroys a DamageModel projectile whatever its damage (sky_025 rt 3155, the damage-0
  blast exhausted on a Zebra). sky_025: bloons match all 3600 frames.
- DONE: an ArriveAtTarget shot whose target died before the release goes at the spot the throw was aimed
  at as it started, over the 2D distance (sky_025 rt 3014: fits (-158.6, -22.9) exactly, 0.625 / 0.949),
  not at a new target. sky_025 MATCH.
- DONE: a burst gained by upgrade (its BurstWeaponBehavior newer than the tower's DamageBasedAttackSpeed)
  checks IsReloadReady for the empty-clip point with the rate after that frame's rage update
  (sky_upgrades_r30 rt 2629: 89 -> 85, the second shot at 2630 goes at the point). sky_upgrades_r30 MATCH.
  Sky 33/34.

### Owner-reported (2026-11-10, must be done exactly; not forgotten)
Owner's rule: nothing half done. Each fix must look and play exactly like the game (verify against
the game code / recordings), or it hides problems later.
- Monkey Village: the fire animation disappears, then reappears (visual).
  FIXED (2026-11-12): its VillageFire system (looping, max 1 particle, lifetime 1 s, 1 a second) went out for
  a cycle: particles.rs emitted before ageing the live particles, so the replacement was refused by the
  dying one at max 1 (and its count dropped). Emission now comes after the update, as in Unity. Not checked
  on screen; a test needs a GL context (EffectDef loads textures).
- Alchemist: buffs it gives other towers don't show their icon over those towers' heads.
  FIXED (2026-11-12): brews and Overclock add their mutators' indicators (AddAcidicMixture /
  AddBerserkerBrewToProjectileModel and OverclockModel ctors: buffLocsName / buffIconName, not stackable).
  DONE (2026-11-15): icon order. Not the mutator list: BuffIndicatorUi.Awake (@0x180a412f0) makes one
  icon per GameModel.buffIndicatorModels entry (99, keyed by FullName = iconName + buffName,
  @0x181118700) under buffsUi, and UpdatePlacedTowerBuffs (@0x180a43b60) only Shows the ones activeBuffs
  name (allBuffIcons TryGetValue), so icons sit in that list's order and a FullName missing from it isn't
  shown. Exported as game.json `buff_order`; Game::buff_icons sorts by it (test buff_icons_in_model_order).
  (For reference: Mutable.mutators is a SortedListLite<TimedMutator> by DESCENDING
  BehaviorMutator.priority (+0x34; SortMutators @0x180f373d0), and SortedInsert (@0x18014ca90) puts a new
  equal-priority one BEFORE the first equal element its binary search probes, not at the end.)
  UpdateBuffs (@0x180c186b0) order, READ 2026-11-16: (1) the tower's TimedMutators with a
  buffIndicator (done); (2) every Support in the game whose CanShowInactiveBuffIcon (slot 140) is true
  for this tower: an INACTIVE icon (canEventuallyBuff 1, canCurrentlyBuff 0); the base returns false,
  only MonkeySubParagonSupport overrides it (OPEN, sub paragon only); (3) AddDiscountZoneBuffs
  (iterator MoveNext @0x180c2f200, READ): Tower +0x288 (a set of DiscountZone) cleared; for path
  0..2 with the tower's TowerModel.tiers[path], if get_owner (PlayerOwnerId +0xb8) == -1 (as
  decompiled; single player's owner IS -1: every recording's IN lines end in -1), not isParagon, not
  TowerManager.IsTowerPathTierLocked and not Tower.IsUpgradeBlocked: the set UnionWith
  Tower.FindDiscountZones(path) (MoveNext @0x180c30a00: TowerManager.GetZoneDiscount(towerModel,
  position, path, tier + 1, owner, the path's next upgrade name) -> its zones); then one ACTIVE query
  (canCurrentlyBuff 1) per zone with its TowerBehaviorBuffModel indicator. IsTowerPathTierLocked
  (@0x180c2c9f0): locked unless TowerModel.upgrades (+0xe0) has an upgrade on the path, then tier
  locks / TowerInventory (never here). DONE (2026-11-16): ported with GetZoneDiscount
  (@0x180c2bd90, Game::zone_discount): every DiscountZone (tower not IsSuspended) unless isGlobal
  needs DistanceSquared(zone tower, p) <= zone TowerModel.range^2 and, at d^2 <= 1.4e-45 (its own
  spot), affectSelf; for an upgrade (path >= 0) not 0 < tierCap < tier; tierMin <= tier;
  dontAffectOthersInCoop: same owner; upgradeId (if set) == the upgrade's name; towerBaseIdList
  holds the baseId. Grouped by stackName in TowerManager.discountStacks (a Dictionary kept for
  the game: `discount_keys` keeps its key order), each group Sorted by discountMultiplier
  (b__6_0 @0x180c32c90) then Reversed (List.Reverse @0x181c484a0), cut to its first zone's
  stackLimit; GetDiscountMultiplier (@0x180c29250) sums the kept multipliers in f32.
  GetTowerUpgradeCost (@0x180c2acc0): cost (or the override) -> TowerUpgradeLocks behaviours ->
  GetFreeUpgrade -> discount = zone + GetAreaDiscount (map area +0x48 per area model) +
  ComeOnEverybody (SimulationBehaviorDiscount) -> cost x (1 - discount) (0 when free); mode 13
  (and an artifact flag) extras; tower behaviour cost multipliers. DiscountZoneModel is now
  game.json tier `discount_zones` (no longer a "discount" support; that kind is only
  FreeUpgradeSupport, still a STAND-IN for GetFreeUpgrade). Village xx1 / xx2 icons
  (BuffIconVillagexx1 / xx2) and Sun Temple / hero ones come from the same zones. Test
  village_discount_zones. NOT PORTED: GetAreaDiscount, ComeOnEverybody, GetFreeUpgrade, the
  final rounding (P-DISCOUNT). (4) every GenericTowerBehaviorBuff (only BuffIconSheriffGiddyUp, a
  hero), CashbackZone, SellIncreaseZone (none in the models), each via TestPlacementTowerBuff (slot
  137), added only when canCurrentlyBuff (+0x19); (5) AddOtherBuffs (iterator MoveNext
  @0x180c2f870, READ 2026-11-16): the tower's weapon behaviours (vfunc 0x958), ability behaviours
  and tower behaviours each yield their own GetBuffs(); overrides: BuffIconPerTowerInRange (Druid
  x-x-x with BuffIconDruidBananaFarm: GetBuffs MoveNext @0x180d11ab0 yields one query, active,
  availableBuffCount = the count of its zone list (behaviour +0x70, list +0xa8), nothing when 0;
  Filter @0x180d04d70: a Tower whose baseId is in towerIdFilter; GetTowerRange @0x180d04ef0:
  rangeOverride if > 0 else the tower's TowerModel.range), VagrantWeaponBehavior (Desperado
  BuffIconWanderer / Shotgun / Rifle), Overcharge (BuffIconOvercharge, ParagonOvercharge).
  DONE (2026-11-16): BuffIconPerTowerInRange (Druid x-4+-x; game.json tier `tower_count_icon`): its
  towerZone is a Zoned<Tower> (+0x70), counted live as Zoned<Bloon> tests (centre distance^2 <=
  range^2); the query's availableBuffCount is the count, and GroupStackableBuffs stores a new
  FullName with it (adds 1 only for queries with availableBuffCount 0; buff_icons now carries it).
  BuffIcon.Show (@0x180a41020): "x" + availableBuffCount when >= 1 (just the number with dontShowX,
  not modelled), else roundsRemaining for a timed one, else empty. Test druid_banana_farm_icon.
  READ (2026-11-16): VagrantWeaponBehavior.GetBuffs (MoveNext @0x180c97370): one query per Vagrant
  weapon with an indicator (BuffIconWanderer / Shotgun / Rifle, stackable false, max 0):
  n = max(0, Math.Round(noTowersRateBonus / towerRangeBonusReduction) - its tower zone's count)
  (0.45 / 0.09 = 5), availableBuffCount n, canCurrentlyBuff n > 0, canEventuallyBuff n == 0: the icon
  always shows, INACTIVE at 0 (BuffIcon.Show: inactiveObj instead of activeObj). DONE (2026-11-16):
  ported (weapon `vagrant_icon`, count from Tower.vagrant_zone / vagrant_counts; f32 ratio rounded
  half-to-even); Game::buff_icons_shown carries the stored query's canCurrentlyBuff and the HUD
  draws an inactive icon as BuffManager.prefab's BuffPrefab/Inactive: the Image 11.7 lower with
  the GrayscaleUI material (uitree::with_gray). Test vagrant_buff_icon. Not checked on screen.
  BuffPrefab also has TextMaxReached (red, same rect as Text): "x" + unavailableBuffCount when
  availableBuffCount is 0 and unavailable >= 1 (no clone query has unavailable counts yet).
  CLOSED (2026-11-16): Overcharge GetBuffs (MoveNext @0x180e65430: one active query, availableBuffCount = currentStack (+0x78), indicator from OverchargeModel.GetBuffIndicatorModel, when stacks > 0): the model's only OverchargeModel (mutationId ParagonOvercharge, BuffIconOvercharge) is referenced by no tower in the game data, so it never shows. OPEN: roundsRemaining text (TimedMutator +0x20 roundsRemaining; BuffIcon.Show shows it when availableBuffCount < 1): which mutators are added with roundsRemaining > 0 isn't found yet (Tower.AddMutator is virtual, 133 callers of AddMutatorIncludeSubTowers alone).
  BuffQuery: indicator +0, timedMutator +8, tower +0x10, canEventuallyBuff +0x18, canCurrentlyBuff
  +0x19, availableBuffCount +0x1c, unavailableBuffCount +0x20. GroupStackableBuffs: a stackable
  (+0x38) FullName adds 1 per query with availableBuffCount 0, up to maxStackSize (+0x3c).
  DONE (2026-11-16): Favored Trades' icon. CashbackZone.TestPlacementTowerBuff (@0x180d06500):
  nothing for a sub tower (TowerModel.isSubTower +0x100, exported as SubTowerSpec.is_sub) or a
  paragon (+0x12c); else canCurrentlyBuff = Vector2.DistanceSquared(zone tower, tower) <= zone
  TowerModel.range^2 (Tower.towerModel is the mutated model: Tower.UpdatedModel). The zone's own
  ship counts. BuffIconBuccaneerxx4 (maxStackSize 3) exported as tier `cashback.icon / buff_name /
  buff_max`; test favored_trades_buff_icon. Not checked on screen.
  OPEN: Ultraboost's permanent stack icon (OverclockPermanentModel reads its indicator from its
  behaviours), roundsRemaining text for timed ones, (2) / (3) / (5) above.
  PLAN (2026-11-12, read): the icons are Tower.activeBuffs (BuffQuery: buffIndicator, timedMutator, tower,
  canEventuallyBuff, canCurrentlyBuff, availableBuffCount, unavailableBuffCount), rebuilt by Tower.UpdateBuffs
  (@0x180c186b0): one per mutator on the tower whose BehaviorMutator has a buffIndicator (+0x40; with
  onlyShowBuffIfMutated (+0x41) only if WasModelMutated), availableBuffCount from the mutator's vfunc 0x188; then
  every tower's supporting behaviours (TowerBehaviorBuffModel.GetBuffIndicatorModel, range check vfunc 0x9f8)
  and AddDiscountZoneBuffs / AddOtherBuffs. BuffIndicatorUi.UpdatePlacedTowerBuffs (@0x180a43b60) groups them by
  BuffIndicatorModel.FullName (GroupStackableBuffs @0x180a42c00: the copy with the larger maxStackSize kept;
  a stackable one adds 1 per copy up to maxStackSize, or its unavailable counts) and BuffIcon.Show
  (@0x180a41020) shows "x" + count unless dontShowX, the inactive image when not canCurrentlyBuff. The clone's
  Game::buff_icons only sums supports (so one Village's x2): port activeBuffs from the mutators the clone
  applies (overclock, brews, village...), with BuffIndicatorModel exported per mutator.
- Monkey Village 3xx / 4xx / 5xx buffs (and MIB) show a stack count (2x) with only one village;
  these buffs don't stack in the game.
  FIXED (2026-11-12): Game::buff_icons groups as BuffIndicatorUi.GroupStackableBuffs: one icon per FullName
  (iconName + buffName), counted only when stackable (maxStackSize > 1, BuffIndicatorModel.ctor
  @0x181118650), "x" + n from BuffIcon.Show; the Village's three Primary Training supports share one icon and
  maxStackSize 0, so no count. Supports export buff_name / buff_max.
- Monkey Village middle-path ability: no buff icons or animations shown, and it may not actually
  raise attack speed as the game does.
  CHECKED (2026-11-12): the rate is right: village_040 / 050 (scenario ability at 120 with a Dart beside it)
  match frame for frame, throws included. Its buff icon (CallToArmsBuff, BuffIconVillagex4x) and the cast
  effect (CallToArms prefab) are exported and shown; between rounds it can be used (canActivateBetweenRounds).
- Monkey Village bottom path: the 5th tier must only be buyable with a Banana Farm in range; its cost
  depends on the farms in range, and it absorbs them. Implement exactly. All of the bottom path's
  extra money effects must work exactly.
  DONE (2026-11-12): MonkeyopolisUpgradeCost (IsUpgradeBlocked @0x180d48d60, GetUpgradeCost @0x180d489a0):
  on its path the 0-0-5 needs a same-owner Banana Farm below tier 5 in range (squared distance < range^2,
  GetTowersInRange <..>d__31.MoveNext @0x180c31d90) and costs costPerFarm (5000) per farm in place of the
  model's cost, before the discounts; the button shows MonkeyVillageRequirement ("Requires Banana Farm",
  UpgradeButton.UpdateSpecialLockUI @0x180b9bca0). Monkeyopolis.Process (@0x180d49350), in the tower's
  turn: every such farm in max(rangeBeforeUpgrade, range) is destroyed without refund, its (int) worth added
  to the village's; crates = max(total / 2000, 1); crate cash (crates x 200 + 1000) / 10 (EmissionMutator).
  OPEN: the bottom path's other money effects (Monkey Commerce discount, Monkey Town / City income) are
  still to check one by one. FOUND: Monkey Town's CashIncrease (OnBloonDamaged @0x180d05d80) puts its
  mutator on every bloon the buffed tower damages (Bloon vtable +0x2a8 AddMutator, while it has health);
  the clone instead pays the buffed tower's own pops x multiplier (game.rs "Monkey Town (CashIncreaseModel)").
  RESOLVED (2026-11-12): the mutator (BloonWorthMutator @0x180d03a30: DistributeCashModel multiplier + (m-1))
  isn't dontRemoveOnBloonDegrade, so it is gone before the next DistributeCash.PostDegrade (@0x180fea0e0:
  cash x incomeMultForRound x multiplier + additive, x sim modifier) reads the model: recording village_003's
  F cash, every pop pays $1 (Dart 816 degrades Black 752 at rt 2804, its next pop at 2989 pays $1). The clone
  no longer pays Monkey Town's tower pops x1.5. Tool: RRBTD6_REC_CASH=1 with --one prints the clone's cash per
  recorded frame (DBGAIM CASH rt cash) to set against the F lines; village_002 / 003 / 004 / 005 match except
  the round-end bonus the harness doesn't pay. CHIMPS has no Village income (crates, Monkey City), so those
  need a non-CHIMPS recording to check.
- Banana Farm: picking up bananas should play a sound.
  CHECKED (2026-11-12): the game plays none. A pickup sound only comes from CreateSoundOnPickupModel (54 in the
  model data, on Collectables), and no Banana Farm cash projectile has one; Pickup.Process (@0x180ccf400) only
  calls the projectile's Pickup, Cash.Pickup (@0x180c99510) adds cash and CreateTextEffect (CashUp), and neither
  Simulation.AddCash, CashDisplay, nor the Banana / BananaCrate / CashUp prefabs (no AudioSource or sound
  script) play anything. The clone already plays CreateSoundOnPickupModel where it exists (pickup_sounds).
  If the owner hears one in the real game, record it (SimRecorder) to find its source.
- Engineer: the ability does nothing. FIXED (2026-11-12): Overclock / Ultraboost (and Take Aim, Darkshift, Mega
  Mine...) have AbilityModel.canActivateBetweenRounds, which Ability.IsReady (@0x180df9e80) honours; the clone
  refused every targeted ability between rounds (`Ability::between_rounds`, `Game::ability_ready`). Targets per
  TapTowerAbilityBehavior.IsBanned (@0x180e5dcb0, `Tier::tap_banned` / `tap_beasts`, the user itself allowed);
  duration (int)(lifespanFrames x tierBasedDurationMultiplier[min(tier, n-1)], a hero's last). OverclockInput:
  "Select a Monkey" helper message (GameMessageManager / HintMessage slide, helper_message.rs), the
  OverclockTowerMarker / TakeAimTowerMarker on every valid tower, a banned tower only shows InvalidTarget and the
  mode stays, empty ground does nothing; Enter/ExitCustomMode keep and restore the selection and show the cancel
  button. The pulse (initialEffect) and the buff display (OverclockBuff / UltraBoostBuff / TakeAimBuffedFx) play.
  DONE (commit 76c51f4): TakeAim.Activate's Tower.AddMutatorIncludeSubTowers (@0x180e5cf10) reaches child towers
  (`include_subs`). DONE (2026-11-16): the point inputs' helper texts: RepositionTowerInput / RedeployInput.GetHelperMessage (@0x180a3ee30) return their CIData's helperMsg, Darkshift.GetCustomInputData (@0x180e21b30) / Redeploy (@0x180e522c0) set it to the model's helperMessageLocsKey (DarkKnightDarkshift x-x-3, DarkChampionDarkshift x-x-4/5, "Heli Pilot Redeploy Message"; exported as darkshift.helper); DeployInput (@0x180a59b20) reads DeployCIData.helperMsg, which PlaceProjectileAt (@0x180e4dcf0, Mega Mine) sets to "MortarMessage". Texts' "{INPUT_TYPE}" is LocsTextReplacer's (Main.<LoadLocs>d__32 @0x1805007a0): GetText("Click") on PC (menus::load_loc; INFERRED "Tap" on mobile). Test point_ability_helpers. Not checked on screen. Still OPEN: the buff icon over the tower (BuffManager,
  buffIconName) as for the Alchemist item below.
- Sandbox: clicks on its menus still reach the menu behind (the bloon menu overlays the monkey
  purchase menu); nothing behind it should be clickable anywhere (the top area already blocks).
  FIXED (2026-11-12): while the cursor is over the open sandbox bloon menu or the Powers menu, Hud::draw sets
  uitree::block_input for everything handled before them (shop buttons, Go, TSM, Geraldo shop); map input was
  already blocked (Hud::blocks). Not checked on screen.
- Beast Handler: on upgrade, the beast sub-tower (e.g. the T-Rex) jumps around; it should stay where it
  was and upgrade in place. Each beast's target location must be settable exactly as in the game.
  FIXED (2026-11-12): BeastHandlerLeash.UpdatedModel (@0x180d02300) keeps the leashed pet (same tower: spot,
  id, process order) and UpdateRootModel gives it the new pet model, slots swapped by baseId; only an empty
  slot gets a new pet. sync_pets now rebuilds a kept pet in place (test beast_pet_stays_on_upgrade).
  DONE (2026-11-16): the move buttons (Game::pet_move_state / pet_move_ok / move_pet, hud MovePet,
  main.rs pet_pick; test beast_pet_move). Ported from the research below: CanMovePet (pet `can_move`),
  the 120-frame cooldown and its fill (timesOfLastMoves from -999999999, Simulation.time), the point's
  validity (pet areas without 4 / 6, the handler's range), the pet turned when rotateWhenMoved (pet
  `rotate_moved`: Microraptor), placed there, the handler turned to it (RotateToLeashedTower @0x180d00ae0,
  leash `leash_rotate`), OnPlace's PlacementEffect. STAND-IN: Merge / contributing handlers, the handler's
  moveAnimationState clip, and co-op (no pet move action) aren't done; not checked on screen.
  RESEARCHED (2026-11-16). Move flow, VERIFIED-DECOMP:
  TSMThemeBeastHandler.CanMovePet (@0x180b8d110): the slot's pet model has BeastHandlerPetModel (TypeInfo slot
  0x184f66a10) with canMove (+0x34) and a live pet of that def. The button is TSM key "MoveButton" /
  "MoveButtonSecond"; UpdateFromSimInfo (@0x180b8d9b0) reads sim info (lastMove1 +0x10, lastMove2 +0x14, now
  +0x18): interactable when now - last > 119, else the cooldownFade image shows with fill 1 - (now - last) / 120
  (constant @0x183e4ff80 = 120.0f, VERIFIED). BeastHandlerLeash.GetTowerBehaviorCustomInputData (@0x180cfc930) for
  Move: none if the slot has no pet or the pet is destroyed (+0x21); a DeployCIData: helperMsg "Select Point In
  Range", towerId = handler id, towerModelToDeploy = a copy of the pet's TowerModel with areaTypes 4 and 6
  removed, withinBaseTowerId "BeastHandler", withinBaseTowerPath = the pet behaviour's +0x2c,
  withinCircleIntersectRange = the handler TowerModel.range, towerPos = the handler's position,
  useBeastHandlerContribution true (DeployCIData.ctor @0x18119e210 defaults: withinRange -1, tier -1, path -1, circle -1; the pet sets path only). Merge
  keys "MergeButton" / "MergeButtonSecond" list the towers IsTowerValidForMerge accepts. ApplyCustomInputData
  (@0x180cfa7d0), Move: pet = leashedTower (+0x80) or leashedTowerSecond (+0x88), skipped if destroyed; if the pet
  behaviour's +0x35 (rotate flag) the pet is turned to face the new point (RotateTower((angle - current) x
  constant)); then its Z = Map.GetTerrainHeight(point), AreaPlacedOn = the area at the point (or default),
  Tower.PositionTower(point), the pet's display offset from its model, RotateToLeashedTower(handler, pet),
  timesOfLastMoves[slot] = Simulation +0x30 -> +0x10 (the sim time used by the cooldown), Tower.OnPlace(pet).
  Merge: Contribute(target handler's BeastHandlerLeash, this, towerModel / towerModelSecond).
  Point validity, VERIFIED-DECOMP DeployInput.IsPositionValid (@0x180a59dd0): false unless the pet model can be
  placed there (the normal placement check, the copied model's areaTypes); withinRange < 0 skips the distance
  limit; with withinBaseTowerId set and useBeastHandlerContribution: valid if IsSimTowerValid(the handler), else
  if any handler in the handler's ContributingHandlersByBeast[beastTowerTypeByPath[path]] is. IsSimTowerValid
  (@0x180a5a460): tier -1 skips the tier test; circle >= 0 needs dist(that tower, the input's handler) <=
  that tower's TowerModel.range + circle (= the handler's range); and dist(point, that tower) <= that tower's
  range. So a pet moves anywhere placeable within its handler's range, or within a contributing handler's range
  whose circle meets it.
- Missing textures / animations, e.g. the Wizard and Ice Monkey paragons turning wrongly: fix them as
  they're found.

### Owner-reported UI fixes (2026-11-01, must be done; not forgotten)
- (2026-11-02, owner screenshot: the Prince's undead bloons were huge grey ovals, its undead MOAB
  off the track) FIXED in the exporters, all projectiles re-exported: export_projectiles.py drew
  each atlas-trimmed sprite image over its full m_Rect, stretching it (UnpoppedArmyBloon's 120 x
  170 art over 256 x 256 = 23.5 units, a bloon is ~12); images are now padded back to the rect
  (552 PNGs changed: nearly every projectile sprite was too big and distorted). export_model.py
  baked a prefab root's own position in (UnpoppedArmyMoab 64.9, Bfb 128.5, Barracuda, Piranha,
  ammo crates, Goliath missile); prefab exports now drop it. Not checked on screen.
  Then (owner: undead MOABs turned, bloons odd, flashing; the Prince stuck facing away): the prefab
  root's rotation is dropped too (UnpoppedArmyMoab / Bfb roots turned 90 about x; Moab3D's is
  plain); layers record a mirroring parent as flip y and the renderer mirrors about the pivot (18
  projectiles, incl. UnpoppedArmyBloon, WallOfFire, MagusWallOfFire, Nail: likely the owner's Wall
  of Fire "angle"); TravelAlongPath projectiles face the track (Game::track_heading) since their
  `vel` swings -0.42 / +0.01 every other frame (the undead flipped 180 each frame); a tower none of
  whose attacks turns it any more is drawn at its placement facing (Prince of Darkness).
0. (2026-11-02, owner) Wizard: Wall of Fire's look is close but its angle is wrong; the Necromancer
   raises no undead in sandbox (the game wakes NecromancerTargetTrackWithinRange only on two round
   events TargetTrack.Initialise @0xe07c50 subscribes, generic MethodRefs 0x184f598c8 / 0x184f59790
   not named yet; owner: the real sandbox raises undead from sent bloons' pops, so HYPOTHESIS
   P-SANDBOXNECRO: sandbox Necromancers always have their track points; FIXED in the clone); sandbox abilities were dead (FIXED: Ability.IsReady
   / Process use AreRoundsActive, true in sandbox) and Reset Cooldowns gave 33% (FIXED: entries set to
   0); an upgrade's new ability now starts at 33% like Initialise. Owner doubted a lone Archmage
   soloing a BAD: recording wizard_500_bad (round 100, CHIMPS) shows the real one does, and the
   clone MATCHES it (7683 frames).
1. DONE (2026-11-02, not hand-tested): Glaive Lord's orbiting glaives are drawn where the sim's
   Orbit puts them (eased 3 <-> 15 degrees a frame, exact circle), each turned by its RotateModel
   (-720 x 0.016666668 = -12 degrees a frame, RotateModel.ctor @0x1811faeb0; `OrbitState::age`), with
   its purple Glow trail (proj_ effect). They wobbled: export_projectiles.py took a tilted node's
   height (4 up) as an in-plane offset and dropped every z; sprite layers now use full 3D transforms
   (all 368 layered prefabs re-exported, 22 changed: the 3 orbit glaives and acid puddles lost a height,
   WallOfFire, WolfSpirit, Sky arrows, NauticSeige dart, Herald lance and others gained their forward /
   back z). A sprite turned -90 about x (UnpoppedArmyBloon) lies along -z: its image is mirrored, which
   the layer format can't say (not new). Open, not done: plain spinning projectiles (boomerangs) use +spin and ignore
   their start rotation; the y flip says it should be -(start + spin x age), unverified on screen.
2. DONE (2026-11-01, not hand-tested): the "i" button on a selected tower shows a description for
   all 3 paths at once, not just the highest (owner's meaning): the next upgrade, else the one the
   path has, else a closed path's first upgrade.
3. DONE (2026-11-01, not hand-tested): the upgrade-icon strip that shows the tower's current tiers
   isn't selectable (no press animation); holding the mouse on an icon shows that upgrade's
   description pop-up on the left, which goes away when the mouse is released.
4. DONE (2026-11-01, not hand-tested): Sandbox: with the bloon menu open, a press on the map only
   closes it and never selects a tower.
5. In-game Powers menu: the MENU is built (2026-11-02, not hand-tested), `renderer/src/powers_menu.rs`:
   the shop's PowersButton opens / closes the game's PowersMenu prefab with its PowersSlideOut clips
   (Open / Close, Wide in the bottom-bar layout: HYPOTHESIS P-POWERSWIDE), one button per Power not
   isHidden that SkuSettings.CanShowPower allows (as the Powers screen), in ui/runtime/powers.json order
   (HYPOTHESIS P-POWERORDER: RebuildPowers' generic wrapper @0x184f63918 not named), its bg from
   StandardPowerButton.UpdateIcon, its count from local_profile.powers (N0; "∞" in sandbox,
   HYPOTHESIS P-SANDBOXCOUNT), the icon in lockedColor when it can't be used (no count, CHIMPS via
   ArePowersAllowed's "Clicks", or RoundDisabled), wheel scrolling. Covers the shop / map input under it.
   USING Powers (2026-11-02): a press on a usable Power takes it in hand (Action::Power, main.rs
   `power_pick`, its Selected glow), the next map click runs Game::use_power (`game/powers.rs`,
   PowerManager.Activate @0x180f37920) and PowerUseSuccess takes one from the profile (not in sandbox:
   HYPOTHESIS P-SANDBOXNOCONSUME, dontUsePlayerInventory). Built: Super Monkey Storm (DamageAll.Activate
   @0x180f31f00) and Monkey Boost (MonkeyBoost.Activate @0x180f34c60, durationFrames = (int)(duration x 60),
   TimedMutator like Vigilante's), Thrive (Thrive.Activate @0x180f42a60: roundsLeft 2, OnRoundStart
   @0x180f42ee0 counts down; pop cash x GetCashModifierFromSimBehaviors @0x180b46ae0 in
   DistributeCash.PostDegrade @0x180fea0e0, isUnique so two don't stack; HUD ThriveGlow), Cash Drop
   (CashDrop.Activate @0x180f312d0: game.json `power_projectiles`, a 2500 collectable picked up like
   bananas; its CashDropCollect mesh planned by export_powers.py, display_plans/Powers.json). Dart Time
   (DartTime.Activate @0x180f32170: endFrame = now + (int)(duration x 60); TowerTimeSlowMutatorClass.Mutate
   @0x1811138a0: WeaponModel.rate x 1 / towerAttackSpeed (and animationSpeed, Heli / Path / Fighter
   movement speeds x towerAttackSpeed: not modelled); BloonTimeSlowMutatorClass.Mutate @0x1810fd010:
   BloonModel.speed x bloonSpeed as a "DartTime" Slow effect (GrowModel / bloonSpeed not modelled);
   BloonSpawned / TowerCreated: new ones too, hooked in spawn_at and compute_buffs). Types behind
   il2cpp TypeInfo usages: `python -I tools/il2cpp/type_usage.py 0x...` names them, through the binary's
   Il2CppMetadataRegistration.types (4 slots before fieldOffsetsCount).
   Track items DONE: Road Spikes, Glue Trap, MOAB Mine, Camo Trap (TrackPower.CheckLocation @0x180f43250:
   a path within 5, HYPOTHESIS P-TRACKPOWERRANGE; CreateProjectile @0x180f43420: the projectileModel at
   the spot), from game.json power_projectiles, their displays planned in display_plans/Powers.json. Then
   Tower Powers: game.json `power_towers` (build_game_data.py power_towers(): each Power's TowerModel through
   tower() / tower_tiers(), plus "unmodelled": its tower behaviours the clone doesn't simulate); Game::new
   adds the ones with none as extra tower defs (never in the shop) and use_power places them free
   (HYPOTHESIS P-POWERTOWERPLACE). Cave Monkey and Banana Farmer (CollectCashZone as Banana Farm x-x-5's,
   P-COLLECT) qualify; Dungeon Statue (hidden, Dark Dungeons) has no tower display to export. Setup exports all 17 models
   (export_tower_assets.py in bloon_models v7). NEXT for the rest, by what each needs: TowerExpireModel
   (Sword of Power, She-Ra, Skeletor), IgnoreAllMutatorsTowerModel, AbilityModel on a Power tower (Tech
   Bot, Battle Cat, beacons), CollectCashZoneModel for Banana Farmer Pro (with PowerProTowerModel), AddMakeshiftAreaModel (Pontoon,
   Portable Lakes), EnergisingTotemBehaviorModel, SavedSubTowerModel (Dungeon Statue), PowerProTowerModel.
   Not done for any: the full-screen effects, sounds, the real placement drag (StartPowerPlacement
   @0x180ac09b0) and CheckLocation. Also not done: the
   Powers / Pro filter toggles (ToggleFilterPowers @0x180ae25b0, ToggleFilterPowersPro @0x180ae2380), GetMoreButton
   (ShowPurchasePowerUpScreen), touch-drag scrolling.
6. DONE (2026-11-02, not hand-tested): sandbox Powers button. The clone hid the bloon menu's
   SBPowersBtn; it now opens / closes the Powers menu (BloonMenu.TogglePowers @0x180b5e500; its
   powersAnimator icon flip not done). The shop's PowersButton is off in sandbox: ToggleBloons has its
   exact rect (HYPOTHESIS P-SANDBOXPOWERSBTN).
7. Sandbox test bloons (2026-11-01, not hand-tested): rendering FIXED: every bloon sprite is DrawMode
   Simple, so size comes from pixels / ppu, not the stale m_Size (Test Bloon was stretched to 23.7 x
   23.7, Regrow bloons 18% too wide, plain ones 5% too big); art trim applied; Test MOAB turns along the
   track. Health CHECKED: 999999 / 100000000 are the game's BloonModel values. Menu position FIXED: first
   two items, per the owner (BloonMenu.SortBloons wasn't conclusive).
8. DONE (2026-11-01): Glaive Dominus T-posed because its clips are relative to the model under the
   Animator's LOD_1; export_animated.py now finds the clips' root. Fixed 31 models: all five frozen
   paragons, Dan D Monke, Druid's tree and others.

### Tower parity (the main goal)
0. **Desperado 19/34 (2026-11-04): SOLVED the rate mystery below with SimRecorder's new Q lines**
   (`Q <behaviour> <rate in> <rate out>`, every weapon GetRate in call order; tools/mod/SimRecorder,
   recordings with them in `pending/_desperado_q/`). Desperado 5-x-x has **TheBlazingSun** (a weapon
   behaviour the export didn't carry): GetRate (@0x180c95dd0) = rate / (currentStacks x
   attackSpeedPerStack + 1) (attackSpeedPerStack -0.01, maxStacks 75, so it SLOWS); EmissionStarted
   (@0x180c95d90) +1 per regular shot (not the missed-frame extras), lastEmissionAt = roundTime;
   Process (@0x180c96150) resets to 0 after debuffExpiresAfterFrames (53) without one;
   OnBloonDestroy (@0x180c96080) -1 when the tower's hit destroys its bloon's own object (a degrade
   keeps the object: a MOAB popping into Ceramics is no destroy). Also: a burst's ProjectilesCreated
   runs once per SpawnDart, so missed-frame extra shots don't count as burst shots. NOT built yet:
   from stacksToBurn (50) the shots set bloons on fire (ProjectilesCreated, burnBehaviorModel /
   burnBehaviorArea / the overheat eject effect). The notes below were the hunt; their
   "distance" theories were wrong (the distance is the clone's).
0. **Desperado 13/34 (2026-11-04, superseded above).** desperado_502 (5-0-2, burst 12 x 3 frames) first differs at
   throw 3: game lastEmission 77, clone 78. The clone's rateFrames after the throw at 79 is 4 (game
   5). From the recording's missedRate deltas (x = rateFrames / (1 + delta)), the product
   vagrant x standoff x distance factor the game used per throw is 4.386, 4.293, 4.396, 4.302,
   4.210, 4.120, 4.030, 3.941, 3.854 (rt 73, 76, .., 97); only one bloon is in range (closest 42.3
   at rt 72 falling ~0.51 a frame). With Vagrant 1.45 x 1.02 and Standoff 2.2 the implied distance
   is 34.8, 31.9, 35.1, 32.2, 29.4, ..: it falls ~2.9 per throw and resets after two, so
   BloonDistanceRateBonus (@0xc6b080) or VagrantWeaponBehavior.GetRate (@0xcb69e0) reads something
   other than the closest bloon's distance at the clone's moment (a projectile? the eject point?
   the burst?). BloonDistanceRateBonus.GetRate (@0x180c6b080) re-read: rate / (max x per x
   (min(d, max) / max) + 1), d = Vector2.Distance(Target.Position of Attack.GetVisibleTarget(<static
   TargetType>), the attack entity's position): as the clone. The game is slower than the clone by
   a factor growing ~1% a throw (1.010, 1.020, 1.031, 1.042, 1.052 at rt 73..85), so look for
   something per shot (502 also has `vigilante`).
   desperado_500 (no Vagrant, rate > 0.1 so no missedRate): throws at 70, 73, 76, 79, 82, 85, **87**,
   90, 93, 96, 98, 106 while the recorded rateFrames is 6 until rt 88 and 7 from rt 91. The 85 -> 87
   gap is 2 with a 3-frame burst: the clone's `last = now + intervalFrames - rateFrames` (with the
   rateFrames just calculated) gives 88. Read BurstWeaponBehavior.ProjectilesCreated (@0xc75760)
   and Weapon.SpawnDart / CalcRateFrames order again: lastEmission may use the rateFrames from
   before this throw's recalculation, or Weapon.Process may compare against a rateFrames that
   changed in between.
   Ruled out (2026-11-04): lastEmission from the rateFrames before the recalculation (13 -> 7/34).
   Read (2026-11-04): Weapon.SpawnDart (@0x180c51420) = Emit, CalcRateFrames, UpdateMissedFrames
   (+ missed shots), then each weapon behaviour's ProjectilesCreated (vtable 0x908), so
   BurstWeaponBehavior.ProjectilesCreated (@0x180c6c5d0) sets lastEmission = clock + intervalFrames
   - the NEW rateFrames, clock = sim +0x38 if weaponModel +0x59 is 0 else +0x30. Weapon.Process
   (@0x180c50670) fires when rateFrames + lastEmission <= its clock (same choice). With one
   rateFrames between the two, the next burst shot is always 3 later; the game's 85 -> 87 means the
   two clocks differ at that moment (+0x30 vs +0x38: which is round time, which elapsed?) or 87
   is another weapon's / an extra shot. Checked: +0x59 is WeaponModel.fireBetweenRounds and the
   clocks are Simulation.time (+0x30) / roundTime (+0x38), as the clone's `clock`; so not the clocks.
   CORRECTION: the W line's lastEmission isn't the throw time; a burst throw at rt t writes
   t + 3 - rateFrames (desperado_500: throws every 3 frames, 73..106). The real difference is the
   rate: at rt 91 the game's rateFrames is 7, the clone's 6 (same in 502: the game is slower).
   desperado_500 is clean (one MOAB alone on the map until rt ~101, no Vagrant): with Standoff 2.2
   (one target) the game's rateFrames 6 / 7 / 8 at rt 73-88 / 91-100 / 103 need
   BloonDistanceRateBonus's distance to be the tower -> MOAB centre distance minus c, c in
   [7.33, 8.05) (MOAB radius is 21, so not the radius). Ruled out: a second Standoff target (no
   other bloon exists), y-flipped Vector2.ToUnity (gives rateFrames 5), an AttackModel offset
   (Desperado's is 0). Candidates: Target.Position isn't the bloon centre (a lead / predicted
   point ~15 frames ahead at MOAB speed?), or the measuring entity isn't the tower. Next: read
   Target.get_Position and what Attack.GetVisibleTarget(Close) stores in it.
   Read: Target.GetPosition (@0x180c08bd0) = bloon position + the Target's stored offset;
   BloonManager.GetClosestTarget (@0x180fc9bf0) builds Target(bloon, offset) for bloons whose model
   +0x88 flag is set (MOAB class) with Targetting.CanHitMoab's (@0x180fdb200) offset: the first
   collision circle within range + its radius, its precalculatedOffset (MOAB circle 0: 6.3
   forward, r 14.7). Ceramics get no offset (fits rt 103). Tried in visible_close (MOAB: always
   the first in-range circle, env EXP_MOABOFF, reverted): desperado_502's first difference moves
   from throw 2 to throw 9, desperado_500 unchanged (with same-frame positions it needs an offset
   of 6.79..8.31 towards the tower, circle 0 gives ~6.3: one frame of MOAB travel short), and the
   set drops 13 -> 9. So the offset is real but something else (which frame's bloon position, or
   which circle / range CanHitMoab gets from GetClosestTarget) is still off; check the callers'
   range argument (param_4) and when Attack.GetVisibleTarget's brackets were built.
   Then (2026-11-04): Attack.GetVisibleTarget (@0x180dae830) -> Targetting.GetTarget
   (@0x180fdbba0) -> GetTargets (@0x180fdbe70), which first asks GetGlobalBest (@0x180fdb5f0):
   it returns a plain Target (centre) only when the bracket entry's struct (+8) has byte 0 AND
   byte 2 set (uVar6 there; the entry's high int is a segment index); otherwise a MOAB-class bloon
   (model +0x88) gets CanHitMoab's offset with range = the attack range (param_6). GetTargets'
   per-segment loop does the same for MOABs. The clone's visible_spot returns the centre whenever
   the segment point is within range + 2, so those two flags are the thing to identify (likely
   "segment fully in range" / "...") to know when a MOAB near the tower still gets its offset.
   Those flags are the ones the clone already has (GetPathSegmentsInRangeFast: point within range
   + 2, and line clear; game.rs towers_phase notes), and for desperado_500's MOAB both hold, so
   GetGlobalBest should give its centre: that contradicts the ~7 shortfall. BLOCKED on a
   recording: next needs SimRecorder to log BloonDistanceRateBonus.GetRate's Target (bloon,
   offset, position) and the measuring position per call (tools/mod), then a re-record of
   desperado_500 / 502 in the real game (needs the owner's go-ahead: no launching the game
   unasked).
0. **Engineer 30/34 (re-recorded with seeds 2026-11-03)**, next to work on. Done: RandomTargetSpread exact
   (Vector3.RandomOffsetFromRadius on syncedRandom), ArriveAtTarget with timeToTake 0 (3D distance from the eject
   point to the ground target / altSpeed, then its curve), SlowOnPopModel (Pin: a Slow only when
   Bloon.WillPopBloon). HYPOTHESIS P-SLOWONPOPUNIQUE: a SlowOnPop re-applied over its own stun keeps the time
   left (only layer charges reset); Super Glue's "Stun" (same isUnique / no cascade) does refresh, so find what
   tells them apart in Mutable.AddMutator / Slow.Collide (isExclusiveInMutationList and AddMutatorForNumLayers'
   cascade-as-refresh switch were the leads). Sentries (2026-11-03): RandomPosition (Land) is now the game's
   algorithm (VERIFIED-DISASM: rings, syncedRandom shuffle, CheckPositions' four lists, GetTargets' index + 1),
   so engineer_100's first Sentry lands at the game's (-82.31, -40.56); a sub-tower turret aimed at a bloon
   ejects along its aim (HYPOTHESIS P-SUBEJECT). Targeting files bloons on the path segment of the distance
   they had when the frame began (prev_dist), in both the target index and the per-bloon segment test: a
   tower processed after the bloons (a Sentry) fires the frame after one comes in range, aimed at where it
   is now (by recording: all six Sentries of engineer_100). engineer_100 now first differs at rt 601 (a
   projectile). A projectile-made Sentry's turret angle is stored relative to its rotation (the projectile's,
   e.g. 97.260574), so its shot rotation is base + (aim - base) in f32 (engineer_200 rt 17: -55.11087); expired
   RandomPosition blocking ignores sub-towers. Sub-towers leave the process list by ordered RemoveAt
   (RootObject.Destroy -> @0x1d996a0), so towers process in creation order; the W lines' order is the factory
   list, which swap-removes the dead tower ~30 frames later (don't mistake it for the process order).
   CreateTowerModel.positionAtTarget puts the Sentry on the projectile's Target point. TowerExpire counts
   round time ((int)(lifespan x 60) <= roundTime - made at). Sentry Champion sentries (SentryParagon) fire
   CreateProjectileOnTowerDestroyModel's BeforeExplosion at their base rotation on expiry; the explosion's
   radius is the sentry's range (LinkProjectileRadiusToTowerRange). useParentTargetPriority gives the Sentry
   the Engineer's target type. The First/Last/Strong keys and Close's rank use the frame-start (filed)
   segment too. A projectile's active FilterInvisible passes no camo (025). 025/100/200/250/300/320/500/502*/520 match.
   Sentry Expert: CreateTypedTower.CalculateTowerModel ported (type chosen when fired). Freeze skips bloons
   its DamageModel is immune to (400 matches). Sentries are selectable/sellable/retargetable
   (ignoreTowerForSelection); TODO: a selected Sentry's panel still shows the Engineer's name and paths, not
   the game's Sentry panel. Typed sentries always take the Engineer's target type (420_* match). CloseTargetTrack (Bloon Trap) and
   LimitProjectile's delayInFrames ported (004/005); RemoveBloonModifiers cleanses lead / regrow / fortified (030).
   Left: 032/052 (Pin re-stun, see above), upgrades_r30, 205: the game makes 200 syncedRandom draws before the
   trap's two at placement, the clone 102 (one RandomPosition shuffle of 102 points). The spawner and trap
   attacks each have a behaviour model plus an identical targetProvider model (two instances); a second
   shuffle (204) is wrong too, and breaks 250 (whose first logged draw follows exactly one 239-point shuffle).
   Find the other 98 draws (RandomPosition.Initialise / Shuffle per instance?).
   OPEN (032 / 052, Cleansing Foam): PathSegment's point has pointHeight = PointInfo z + 2 (VERIFIED-DISASM
   ctor @0xf6d050, literal 2.0 at 0x183e4fc3c), and ArriveAtTarget.SetTarget times altSpeed shots by the 3D
   |targetPos - origPos| (@0xc87ad0). engineer_032's foam to (-113.1, -18.9) eases in as if the target is at z 2
   (0.662 / 0.974 of the way). Making start_track_point targets z 2 fixed 032 to rt 1990 but broke 040 / 050 / 250,
   whose foam gets a RandomTargetSpread offset and is timed as if z 0, although RandomTargetSpread.Emit
   (@0xc4b720) passes Target.Position's z on to RandomOffsetFromRadius (z + 0). Find what drops the z there
   SOLVED: RandomTargetSpread.Emit makes the new Target at Map.GetTerrainHeight (committed, 032/052 to rt 2116).
   Layers-0 slows now survive pops (AddMutatorForNumLayers adds no charge for 0).
   Engineer 32/34 (2026-11-04): 205 and upgrades_r30 solved: RandomPosition's shuffle buffer starts at 200 (R m lines:
   Next(102) x 200, then Next(239) x 239), runs when the tower gets the model and again only when the model's
   fields change; CloseTargetTrack's draws come ahead of the Spawner's shot. Left: 032 / 052 (next line).
   P-SLOWONPOPUNIQUE table ($TEMP corr.py idea: per stun, later pop, dur): stops 2056 (988, 1064, 1089) refresh at the
   pop 2099 (dur 101), 2014 (1028) and 2098 (1163) don't; all pass under the foam. Tried and wrong: any other
   mutator on the bloon, the stun first in the list. Same nail model, damage 1, layers 2, isUnique.
   OPEN P-SLOWONPOPUNIQUE: engineer_032 rt 2099 contradicts it. Green 1064 was pinned at 2057 (Pin: Stun:Weak,
   1 s, layers 2, isUnique) and popped again at 2099; the game keeps it pinned to 2158 (a full refresh), while
   engineer_002 keeps the old time. Refreshing for all drops 002/003/004 (Engineer 19). AddMutatorForNumLayers
   (@0xfb01f0): with the model's +0x4e flag it resets the existing charge entry (ResetMutatorChargeCountForId),
   else it appends a new one; the time refresh is in Mutable.AddMutator (vtable +0x2a8). Find what differs
   between the two cases (charges left? which bloon the stun is added to, before or after Degrade?).
   (2026-11-04) Mutable.AddMutator (@0x180f34f20, live build) read in full: an isUnique mutator
   (+0x30) whose id the bloon already has makes it RETURN at once (no time refresh, no new entry);
   otherwise with updateDuration it finds the entry by id and, if its time left < the new time,
   sets it to that entry's own original time (+0x28); else appends. So engineer_032's re-pin at
   2099 can only be a NEW stun after the old one was removed: Green 1064's old stun must have run
   out of layer charges (2 layers) before the nail's SlowOnPop at 2099, e.g. the pin's own pop at
   2057 or another hit used one, while in engineer_002 a charge was left. Next: per hit at
   2057 / 2099 on 1064 and its children, which projectile (P lines) popped it and in what order
   (H lines: projectile bloon ...), and whether the stun was added to the parent before Degrade
   (its child then starts with layers - 1, game.rs 7564).
   Recording facts (engineer_032): 1064 Pink -> Yellow at 2057 by nail 1192 (pinned), only foam
   (1020 / 946, no pops) touches it until nail 1232 hits at 2098; Green at 2099, pinned to 2158.
   Ruled out: a unique re-add leaving the layer charges alone (game.rs "Some(x) if unique": no
   reset) drops Engineer 32 -> 25 and moves 032's first difference to rt 1862.
   Bloon.AddMutatorForNumLayers (@0x180fb01f0, live) read: it calls AddMutator (vtable +0x2a8),
   then, unless its updateDuration-like flag (param_7) is set, APPENDS a charge entry (count,
   count, the mutator INSTANCE) even when AddMutator returned for a unique duplicate; with the
   flag it resets the existing entry. Ruled out: "an old stun with 1 charge left is replaced by a
   fresh one" (EXP, reverted): 032's first difference moves to 1911 and engineer_002 breaks at rt
   74 (a 1-charge case there keeps its time). Next: Bloon.Degrade (@0x180fb1ad0), how the charge
   entries decrement and which mutator instances the child gets (and with what time).
   Read (2026-11-04): Bloon.DepleteMutatorsForNumLayers (@0x180fb2420): every chargedMutators entry
   count -= the layers popped; at <= 0 Mutable.RemoveMutator(that entry's INSTANCE) (a never-added
   duplicate's instance removes nothing). Bloon.CopyMutatorsFromBloon (@0x180fb0f40): the child
   copies all charge entries, then Mutable.CopyMutatorsFrom (the mutators actually on it).
   Ruled out as an emulation (EXP, reverted): "unique re-add: if the old has <= 1 charge, a fresh
   stun with layers + 1, else unchanged" -> Engineer 22/34. The clone's `layers` (one number per
   effect, filtered `layers > 1` at each degrade) can't hold two entries per id; model
   chargedMutators as a list of (count, effect instance) per bloon, deplete by layers popped and
   copy to children, before trying the order again.
   BETTER LEAD (2026-11-04): SlowOnPop.Collide (@0x180cde190) = Bloon.WillPopBloon then
   Slow.Collide (@0x180cde360), which calls AddMutatorForNumLayers(bloon, mutator, time, layers
   (+ damage if SlowModel.matchLayersWithDamage), updateDuration 1, applyMutation 1,
   SlowModel.cascadeMutators, ...): the append-vs-reset choice is cascadeMutators, and AddMutator
   runs with updateDuration: an existing id whose time left < the new time is set back to its own
   original time. engineer_032 fits that alone (the 2057 pin had 19 frames left at 2098 < 60 -> 60
   again, pinned to 2158). So engineer_002's kept time must come from the NEW time: AddMutator
   scales it by the smallest multiplier from the list the bloon's vtable +0x298 call fills
   (entries (float, enabled); "Stun:Weak"?). Also check that the BehaviorMutator's own isUnique
   (+0x30, the early return) is really set for Pin's stun. Next: name vtable +0x298 on Bloon and
   find which multiplier 002's bloon has. Bloon.FilterMutation (vtable slot 22, @0x180fb2fa0) only asks
   each bloon behaviour (IBloonBehavior.FilterMutation); plain Pinks / Yellows have none, so no
   multiplier there. Tried the plain refresh (EXP, reverted): Engineer 25/34, 002 off at rt 74.
   The two cases side by side: engineer_002 bloon 414 Black -> Pink pinned at rt 15, Pink ->
   Yellow by a nail at 57 (17 frames of stun left), moves again at 74 = the ORIGINAL end (no
   refresh, and not removed by charges); engineer_032 bloon 1064 Pink -> Yellow pinned at 2057,
   Yellow -> Green by a nail at 2098 (19 left), pinned to 2158 (refreshed). Same gap, opposite
   result: look for what differs (032 is 0-3-2 with Cleansing Foam hitting the bloon every
   frame; which tower / nail model made each hit; whether 032's 2098 hit came with a second hit
   that frame).
   RECORDED (2026-11-04, SimRecorder "M" lines: AddMutatorForNumLayers with the mutator's removeAt
   before/after via GetMutatorById, DepleteMutatorsForNumLayers, CopyMutatorsFromBloon; new
   recordings in pending/_engineer_m/, the clone MATCHES their 002 and diverges in 032 at 2116 as
   before). engineer_002 bloon 414: stun at 14 (60), deplete 1; at 56 re-add "before 18 after 60"
   (GetMutatorById's removeAt refreshed), deplete 1; yet it moves at 74 = the ORIGINAL end.
   engineer_032 bloon 1064: stun at 2056 (60); at 2098 re-add "before 18 after 18" (not
   refreshed), deplete 1; yet it stays pinned to ~2158 = the NEW end. So the TimedMutator that
   GetMutatorById returns isn't what holds the bloon in one of the two (two instances with the id,
   the first one returned). Next: log Mutable.RemoveMutator and every TimedMutator on the bloon
   (id, removeAt) each frame for those bloons, and the Bloon speed / stun state.
   SOLVED the re-pin rule (2026-11-05, SimRecorder per-frame "S bloon id:removeAt ..." lines = the
   bloon's whole Mutable.mutators list in order): a repeat Pin adds a SECOND Stun:Weak TimedMutator
   at the binary-search slot (`mutator_slot`, len / 2); this pop's depletion empties the old
   charge entry and RemoveMutator takes the FIRST TimedMutator with that shared mutator: new one in
   front -> the old time stays (002), behind -> a fresh 60 (032). Built (game.rs, the `unique`
   SlowOnPop branch). Left for 032 / 052: the clone's effect list must hold the game's DUPLICATE
   instances too: Cleansing Foam's Slow (unique, refresh in the export) is added as a second
   TimedMutator by each puddle (032 bloon 967 at 1958: [Slow, Stun, Slow]), which moves the stun's
   slot; the clone merges them ([Stun, Slow]) and keeps a stun the game drops (032 now first
   differs at 2032). Model duplicate instances (only the first per id applied: ApplyMutation) in
   `Bloon::effects` before going on. DONE (2026-11-05): a slow put on again without cascadeMutators
   (AddMutator @0x180f34f20 refreshes the one it has only with cascade) is a second instance at
   `mutator_slot`; the glue level still rises on the first (highestGlueLevel). Engineer 34/34. Then the 003 / 032 / 052 / 250 / 300 / 320 / 400 / 420_* / 500 / 502_* / 520 /
   upgrades_r30 groups.
0. **Sub 32/32 EXACT (2026-11-04)**. Learned (VERIFIED-DISASM unless noted): Emission.BaseEmit raises the emit point to
   Target z less the bloon model's height (a MOAB just popped into a Ceramic: 120 - 20 = 100); CollisionChecker: a
   blocker hit is CollideMap + RootObject.Destroy, no Expire; Weapon.Process does nothing between rounds, a throw in
   progress waits too; CreateEffectProjectileAfterTime useTargetPosition (Ballistic Missile, 18 frames) gives the shot
   a new Position at GetPositionOfTarget for that frame; Target.GetPosition: destroyed bloon = the cached spot (where
   it was when the throw started; readers hold copies of the struct), a REUSED factory object = the new bloon's
   position (PT lines show it), so a blast can land on a new bloon, the frame's spawn included; a throw whose target
   died: an instant shot sits on the eject point after its first frame and keeps the Target; Submerge.Attatched sets
   target type Submerge on the upgrade that adds it. P-POOLBACK (from recordings): a bloon made and fully popped in
   the same frame frees its object at once.
0. **Sub, older notes**: sub_005 / sub_025 (Sub Commander) first
   differ at rt 675: the main dart (TrackTargetModel, pierce 1, recorded at z 8.7, P line's emittedFrom z and
   Position z) expires one frame after it's made and CreateProjectileOnExpire.Expired (@0xca4a00) emits the
   Airburst splits at the projectile's vtable-0x828 position, which the recording shows at z 100 (85 such
   lines, only in the x-x-5 recordings); the clone keeps 8.7, so a split meets a tree blocker the game's flies
   over. Find what puts the dart at z 100 (TrackTarget? the Commander buff?).
   Earlier notes: Airburst's split angles are k x step + (rotation +
   startingOffset), the sum grouped (002 / 004 / 502_* match). Open groups:
   - 040 / 050 / 052 / 250 fire their first shot one frame early (clone 476, game 477, the same aim
     -27.256102): the bloon is in range from 474, so it's the line-of-sight test (map Logs). The only data
     difference from 030 (which fires right) is the eject point (y 25 vs 19.5): check whether
     Map.IsTargetLineClear runs from the eject point, or the tower position plus something.
   - SubCommanderSupport (VERIFIED-DISASM @0x180d910d0) now exact: DamageModels not named FirstStrike x damageScale
     + damageIncrease, every DamageModifierForTag / ForBloonType additive x damageScale, +pierceIncrease on every
     ProjectileModel (sub-projectiles too). 005 / 025 are bloon-exact; open: five Airburst splits at rt 664
     start at z 100 in the game (CreateProjectileOnExhaustFraction emits at the parent's 3D position, so the
     parent dart was at z 100 when its pierce ran out on the MOAB, though recorded at 8.7): a low split then
     meets a height-100 log in the clone. 205 still diverges (look from its first difference).
   - 030: instant shots made this frame no longer age in the instant pass (Ballistic Missile's 0.6 s = 36
     frames). Now first differs at rt 955: an Instant missile whose target went at 946 holds still, then at 955
     jumps back to its first target point (Target.Position falls back to the stored point some frames after
     the bloon goes: find what marks the bloon destroyed then).
   - 040 / 050 / 052 / 250: the first shot is a frame early with the same aim (game 477, clone 476). Not line
     of sight (no blocker near the line) and not range (in range from 474). All four used First Strike
     (a one-shot processOnActivate ActivateAttack) at 120: check whether that reorders the tower's attack and
     weapon processing (weapon reading the attack's target from the frame before).
   - 001 / upgrades_r30: bloon-exact, projectile positions only.
0. **Tack 21/25 (2026-11-03)**, re-recorded with seeds. Spin (VERIFIED-DISASM @0x180c92af0 / b30): every
   SpinModel weapon of the tier (ability attacks' too) has an angle from when the tower has the model,
   + rotationPerSecond x 1/60 per frame, wrapped only at >= 360; a shot reads it before that frame's step and
   is turned by it (Vector3Boxed.Rotate) with rotation + angle (Blade Maelstrom at -810.00006). Ability
   attacks' lifespan counts round-clock frames (a held clock frame isn't counted) and an activation between
   frames doesn't count the others again. Open: tack_500/502, the Inferno Ring meteor's y is 1 ulp off at
   rt 522 (TrackTarget / TravelStrait / Vector3Boxed.Rotate all match the decompile: look at AngleBetween's
   atan2f and the target point); tack_520; tack_upgrades_r30.
   SOLVED 500 / 502 / upgrades_r30 (2026-11-05) with SimRecorder's new D lines (each projectile's
   Direction every frame): the meteor's LAUNCH direction was already 1 ulp off, it is the line to
   the target divided by its f32 length, not rebuilt from the atan2 angle (game.rs `seek_launch`).
   tack_520 SOLVED too: the meteor has TravelStrait 99999 s and AgeModel 20 s; the export kept only
   the first as `lifespan`; each ends the shot on its own, so the shorter wins. Tack 25/25.
0. **Ice 25/25 (done 2026-11-03)**, all re-recorded with seeds. What it took, all VERIFIED-DISASM:
   AddBehaviorToBloon.Collide draws syncedRandom for chance > 0 (Icicles 25%) after its filters, also on a
   hit immune to the damage (Projectile.CollideBloon's passes before the DamageModel run regardless:
   `pre_damage_effects`); Slow.Collide skips immune bloons only for "Permafrost" ids; FilterMutatedTarget
   (projectiles, `skip_mutated`) with Mutable.IsMutatedBy's id / idMajor / idMajorMinor match (icicles ignore
   bloons carrying Ice:*); copies made by a degrade get their mutators with the parent's time left as their
   total (a refresh restores that), their own CarryProjectile icicles (collidedWith = the popped bloon's
   layerId), and EmitOnPop re-armed only on copies; RemoveMutatorsById destroys carried projectiles at once.
   OPEN (INFERRED): CopyMutatorsFrom passes the source TimedMutator's removeAt (0x18) as the copy's duration,
   so a copy's totalDuration should be the parent's time left; ice_025 needs that for freezes, but glue_005 /
   glue_025 need copied glue mutators to keep the full lifespan on refresh. Applied to freezes only; find what
   differs (removeAt may be absolute, or Slow / AddBehaviorToBloon refresh another way).
0. **Glue 34/34 (done 2026-11-02)**: the last recording (glue_upgrades_r30) needed the game's mutator and
   behaviour order, now modelled from the disassembly: Mutable.AddMutator's SortedListLite insert (@0x181ffcdb0,
   binary search @0x18014ca90, Mutable.SortMutators by priority; Slow and AddBehaviorToBloon mutators are all
   priority 1) puts a new mutator at index count / 2; CopyMutatorsFrom (@0x180f35ac0) re-adds the parent's in order
   on each new child; a bloon keeps its behaviours when its model changes and adds new ones last (`Effect::seq`);
   DamageOverTime.PostDegrade (@0x180fe54d0) gives each child's first DamageOverTime the parent's last lastTick
   (only the first mutator with an id is applied). New children's DoTs start over (Initialise). The fitted
   P-DOTDEGRADE hypotheses are gone. Every other set is unchanged.
1. **Bomb (33/34, done but bomb_032's 1-ulp frags)**: re-record its scenarios with `# seed` first (ground rule), then
   work through the first differences (open: 003/005/025/205 throws, 004/032 positions).
   Re-recording (2026-11-02): bomb_000 re-recorded with its seed (no R lines: a 0-0-0 makes no synced
   draws) and MATCHES; all 34 now re-recorded with seeds: still 28/34. DIFF: 003 (throw #12), 005 / 205 (first
   throw game 132, clone 127), 025 (throw #38), 004 (801 position frames only), 032 (9 position frames).
   bomb_005 (0-0-5): SOLVED rt 78: the Cluster fragments' ProjectileModel has dontUseCollisionChecker
   (CollisionChecker.Process @0x180f31590 skips it; `Proj::no_checker`, not in the instant pass), so they
   never hit the MOAB they're made in. SOLVED rt 87: RandomRangeTravelStrait.Initialise (@0x180cd46c0)
   draws lifespanFrames = (int)(((max - min) x synced NextDouble + min) / speed x 60 + 0.5) per fragment
   (`random_range`; the clone's draws now equal the R lines). NEXT: throw #1 (the second throw) is at 132 in
   the game, 127 in the clone (first position difference rt 88, empty lists: a count difference).
   (Fixed too: compare.sh --one dropped RNG lines, a missing `|` in its filter.)
   SOLVED rt 88: the formula fits once read with Bomb Blitz's own range (20..30, not another model's
   22..40): expiry = made + lifespanFrames; the fragment was cut short by a tree, but CollisionChecker
   skips map blockers too for dontUseCollisionChecker. SOLVED rt 120: AbilityDamageAll.Activate
   (@0x180df6800) for the scenario's ability (Bomb Storm). 003 and 004 now MATCH. NEXT (005 / 025 / 205, rt
   1190) SOLVED: AbilityModel.activateOnPreLeak (Ability.OnPreLivesLost @0x180dfa8e0 -> Activate -> IsReady):
   the Bomb Storm goes off when a bloon is about to leak and pops it too; abilities start at cooldownFrames x
   0.33 (Ability.Initialise; none with startOffCooldown, all with resetCooldownOnTierUpgrade; P-ABILITYINIT:
   set on the first tick); maxActivationsPerRound counted (reset at round start); the harness's scenario
   ability is Activate(true), which keeps the cooldown; this frame's Spawner bloons exist for the storm
   (`frame_spawns`). 005 / 025 / 205 MATCH. bomb_032 (left, bloons and weapons exact): at rt 1738 four of a
   0-3-2 blast's 8 frags (rotations -100.13512, -55.135117, 79.86488, 124.86489) are 1 ulp off in position
   because one direction component is (velocity 5 x dir, so sin/cos of the angle): the same open 1-ulp
   Vector2.Rotate question as the Lord Phoenix drift (Wizard notes). Left for the sin/cos investigation.
   (2026-11-04) Frags move start + 5 x (sin r, cos r) in the recording's frame. The P line's
   rotation isn't the emission angle: ArcEmission's (r - 180) + k x 45 from the bomb's 34.864876
   gives 79.864876 for k = 5, the recording says 79.86488 (and 34.864883 for k = 4), so the
   recorded rotation is derived back from the direction; fitting sin/cos against it is misleading
   (with it, correctly rounded double sin/cos match 14 of 16 coordinates, a cumulative 45-degree
   Vector2 rotation 8). Next: compare the clone's direction vector per frag (game_dir of the
   emission angle) to the recorded displacement / 5 directly. Done: with exact sin of the clone's angles
   ((34.864876 - 180) + k x 45), frags 1 and 5 (|sin| ~ 0.984) need |dx| 1-2 ulp larger, i.e. the
   game's angles are slightly larger (towards +90 / -90 from 79.86 / -100.14), as the recorded
   34.864883 for k = 4 suggests: the emission's base rotation may be 1-2 ulp above the bomb's own
   34.864876 (Weapon / emit rotation computed separately, e.g. from the eject direction).
   (2026-11-05) The P line's rotation field IS EjectRotation, so the frags' 79.86488 etc. are their
   emission angles: ArcEmission ran from a base of 34.864883, while the explosion projectile 2564
   made the same frame carries EjectRotation 34.864876 (the bomb's). Not the base: atan2 or
   Vector2.Angle of the bomb's rotation-made direction (both give back 34.864876), nor the hit
   bloon 2409's direction. CreateProjectileOnContact.Collide (@0x180ca2a60) passes Emission.Emit
   (vtable +0x298) the projectile MODEL, spawnPosition, a Target(bloon), sim time, EmittedBy and
   the projectile's +0x1e0: no rotation, so the arc's rotation comes from inside Emit (ArcEmission's
   own field, or +0x1e0). Next: decompile ArcEmission.Emit for where `rotation` comes from, or log
   it with SimRecorder (a postfix on ArcEmission.Emit / GetDirection).
   SOLVED (2026-11-05): ArcEmission.GetDirection (@0x180c37590) takes the rotation from the OWNER
   TOWER (FUN_1824b5050 on param_7), then its emission behaviours (vtable 0x818), then (rot -
   angle/2 + offset) + k x slice. The tower had turned on to 34.864883 since throwing the bomb.
   Built for CreateProjectileOnContact Arc emissions without a direction-offset behaviour (not
   EmitOnDamage, which Sniper's fans use). Bomb 34/34.
2. **NOW: Sniper 19/34** (see Sniper notes): all 34 re-recorded with seeds (2026-11-02, still 19). sniper_040
   (Supply Drop): toggled-off abilities can't activate (ToggleableAbilityBehavior via CanUseAbility; AmmoSwap
   starts off: `toggled_off`, P-TOGGLE); RandomPositionBasic.GetTargets exact (MoveNext @0x180dd9e70: r =
   d1 x (max - min) + min, angle = (float)(d2 x 2 pi) in double, redrawn until Map.IsPointOnMap(p,
   -mapBorder)); an Instant shot follows a newer bloon at its previous spot (the TrackTarget born rule).
   First difference 121 -> 1902: Bouncing Bullet 1541 (pierce 4 -> 1, bounces used up) is gone in the game at
   1902 (Retarget doesn't expire it: Collide just returns at currentBounceCount 0); the clone keeps it.
   Find what ends it (the Instant's target, the frags' EmitOnDamage, or pierce). Its RetargetOnContact: distance
   40, maxBounces 9999999, delay 0.1, expireIfNoTargetFound; Collide (@0xcf6ae0) spends a bounce and destroys
   it when GetTarget finds nothing (no pierce check); pierce is 1 on both sides at 1901.
   NOTE: H lines are written BEFORE their frame's F line (they belong to the next F, SimRecorder.cs
   CollideBloonPatch). Read that way, sniper_205 (and its _close / _last / _strong, 10 position frames each)
   rt 954-955: shot 2643 leaves bloon 2626 at 0 hp at 954; shot 2633, made at 954 aimed at it, collides with
   it at 955 (pierce stays 1) and is removed that frame (PL x2633); the clone's shot outlives its target by
   5 frames. sniper_040's bullet also goes in the frame of its rt 1902 collision. Likely one rule: an Instant
   shot is removed after it collides with its Target (or with a Target that died); confirm in InstantModel /
   Projectile.CollideBloon before coding it. Checked: Instant.Process (@0x180cc1b30) doesn't: with an
   invalid Target it destroys only with destroyIfInvalid (+0x30); when the Target's bloon id changes
   (lastBloonId +0x70) it becomes a position Target at lastBloonPosition (+0x74). So look at the emission
   (InstantDamageEmission) and the projectile's AgeModel next: in sniper_205 the clone's shot had 6 frames
   of life left at 954. Also checked: InstantDamageEmission.Process (@0x180c421e0) collides its proj
   with its target once, the frame after Emit, whatever the bloon's state; Projectile.CollideBloon
   (@0x180c8b970) returns at IsCollisionValid false without using pierce (so the H line of a dead
   target shows pierce unchanged); InstantDamageEmission.OnDestroy only clears fields. destroyIfInvalid is
   false for the 2-0-5 shot (lifespan 0.1 s). Projectile.IsCollisionValid (@0x180c8d840) has no destroy either. Still open: what removes it at 955 (
   @? and its callers, Exhausted, PL list handling in SimRecorder).
   sniper_005 (5 position frames, first rt 457) is the same pattern: shot 3838 (the missed-frame second shot)
   targets 3801, which shot 3848 pops at 456; at 457 its H lines are with dead 3801 and with Yellow 3819 (an old
   bloon, not a child) and it's gone that frame. KEY: SimRecorder logs H on every Projectile.CollideBloon CALL,
   valid or not (hp / pierce unchanged = refused). FilterAllExceptTarget.FilterBloon (@0x180c1d3c0): valid
   Target -> ObjectId.op_Equality(target, bloon); invalid Target -> false, so 3819 was refused. So in all three
   recordings (040, 205_x, 005) the game removes an Instant shot whose Target is dead / done at its next
   collision step, without any valid hit. Still to find in the decompile: what does it (candidates: the
   CollisionChecker's handling of a refused CollideBloon, Age). Live Projectile.CollideBloons @0x180c8bd70 (grid)
   calls IsCollisionValid first and only then CollideBloon(force=1), so a refused bloon there makes no H line;
   the H with Yellow 3819 must come from the other overload @0x180c8bce0 (FUN_1824b4110 loop) or another
   caller: find which, then what destroys the shot.
   Lifetimes in sniper_205 (P lines): 7 shots live the full 6 frames (lifespan 0.1) though their Target is dead and
   their H lines are refused too (e.g. 4727 at 3206: target 4649 already dead when made) - the clone gets these
   right. The 3 that live 1 frame (1893 at 198, 2633 at 954, 3363 at 1869) had a target alive when made and killed
   that same frame by the sibling shot of a missed-frame double (2633 made before 2643, which pops 2626). So the
   rule is about that order (Instant lastBloonId set at creation, then the bloon dies; or the emission's once
   collide), not a dead target as such.
   (2026-11-02, later) This one rule is what 9 of the 15 failing recordings show (005, 020, 025, 205 x4,
   420_close, 520: bloons and weapons exact, only position frames). Ruled out in the decompile, none
   destroys or ages the shot: CollisionChecker.Process (@0x180f31590: grid query -> Projectile.CollideBloons
   list overload @0x180c8bce0, the source of the refused H lines; its only Destroy is a map-blocker hit),
   Projectile.CollideBloon (@0x180c8b970: Bloon.PreCollision first, a refused one returns 1),
   Bloon.PreCollision (@0x180fb59f0: bloon behaviours only), ProjectileFilter.FilterCollision
   (@0x180cd1f90), FilterAllExceptTarget.FilterBloon (@0x180c1d3c0), InstantDamageEmission.Emit / Process,
   Emission.BaseEmit (@0x180c3dce0: the double-shot flag isn't passed on; slot 105 is set_EmittedBy),
   Weapon.SpawnDart (@0x180c51420: the missed shot is Emit(..., doubleShot=1)). Age.Process (@0x180c862d0)
   expires at lifespanFrames <= now - the projectile's creation time (an interface call, slot 0xf); its
   only caller of SetToExpire is CreateProjectileOnTowerDestroy.CreateProjectile (@0x180d14910: unrelated). Note InstantDamageEmission keeps ONE `proj`:
   a double's second Emit overwrites the first, so the first never gets the once-collide. Next:
   check where a Projectile's creation time is set (could a double shot's be backdated
   by the missed frame?), and the map-blocker branch (an instant shot at its dead target's
   lastBloonPosition, z?). The game also draws
   `R r 0 1000` at 119 (Supply Drop) that the clone doesn't (no later draws, so no desync yet). Then Tack 17/25, Sub 16/32, Beast Handler 9/34, Engineer 7/34,
   Desperado 6/34, then the 0/x towers.
3. Powers still to build (item 5 above has the list): the tower Powers that need TowerExpire on a
   parentless tower (spawn_sub needs a parent), abilities on Power towers, makeshift water, Energising
   Totem, Pro Powers; their effects / sounds / placement drag.

## Wizard notes (done, 34/34; kept for reference)

0. **Wizard re-recorded (2026-11-01, R lines in 10)**. Necromancer graveyard: NecromancerZone hears
   a degrade only from a bloon it subscribed (BloonEnter, @0xd4bd80); a heard degrade uses the
   subscription up, every new bloon (first child too) enters afresh on the next Process
   (P-GRAVESUB, `grave_zone`, `grave_zones()` before the tower phase); with a Prince (tier 5)
   every degrade is heard (P-GRAVET5). wizard_004 / 005 / 025 MATCH. Phoenix 040/050/052/250
   MATCH: PathMovementFromScreenCenter.Process moves the Phoenix in 3D, Vector3.MoveTowards
   (@0xf47cf0, dx² + dy² + dz² unfused) to (point, 250) by speedFrames = speed x 0.016666668 (the
   model's .ctor @0x1225980); the Phoenix starts at z 250 (P-PHOENIXZ0); the point switch is
   Vector2.DistanceSquared unfused. wizard_052: OverrideCamoDetectionModel (Monkey Sense) gives
   the tower and its sub-towers camo detection (`Tier.detect_camo`, a buff), so the Phoenix takes
   a Rainbow Camo off the map's edge (Tutorial's path points are all targetable).
   500 and 520 MATCH (a non-turning attack ejects from the tower's rotation in degrees);
   502_strong MATCHES: TargetStrongPrioCamo's FindBestBloon lead is camo bloons only; without one,
   TargetStrong's own order (the path's First lead first, then First's order, strictly higher
   danger wins) for both prio and plain Strong.
1. **Lord Phoenix (wizard_050 / 052 / 250)**: flight ported (committed): harness `sync_pets`; `circle_subs` = PathMovementFromScreenCenter.Process (360 points
   radius 70 round (0, 0), closest first, next within speedFrames x 5, MoveTowards, LerpAngle 0.1);
   starts on its tower and waits its first frame; shoots from its rotation (`circling` grot).
   Committed: circling pets start at age -1 (HYPOTHESIS
   P-PETFRAME), so the summoned Phoenix turns the frame after the ability; wizard_050 bloon
   frames 1707 -> 663, first at rt 222; rt 37 still a 1-ulp y on a Phoenix shot in flight.
   Committed: the path point switch tests DistanceSquared (663 ->
   403 bloon frames); the Summon Phoenix mute lasts lifespan x 60 + 1 frames (HYPOTHESIS
   P-MUTEFRAME; main tower weapons now all match). Committed:
   Phoenix shots start 250 up (`tower_height`, HYPOTHESIS P-PHOENIXZ; recording z 257), so map
   blockers don't eat them: wizard_050 first bloon difference rt 341 -> 3341 (258 frames). The
   harness doesn't pair the Phoenixes' weapons ("tower 1 weapon 0 ... clone None").
   Committed: shots with FilterInvisibleModel from a tower
   without camo detection (`pr.camo` false) pass camo bloons in the collision test; this made
   wizard_050 bloon-exact (0 bloon frames). Leftover: 1-ulp Phoenix drift from rt 79 (position
   and rotation together; NOT a point switch: point 212 at rt 78 and 79, so MoveTowards
   arithmetic or the point's exact value; the game's cos/sin helper (FUN_18048c000) is unknown).
   wizard_250 is bloon-exact too.
   Committed: ProjectileModel.ignorePierceExhaustion
   (`keep_spent`, 143 projectiles): spent, it stays (immune-destroy now also zeroes frames_left).
   wizard_040 bloon frames 3089 -> 7, first rt 234; plus RefreshPierce applied after the
   collision pass (`refresh_due`): wizard_040 bloon-exact, 020 still matches.
   Phoenix 1-ulp drift (blocks 040 / 050 / 250 from full MATCH): Vector3.MoveTowards' squared
   length is fma(dy, dy, dx x dx) (fixed rt 79; committed). Next ulp at rt 91
   (050): with point 221 the game's step needs the target 1 ulp off; the game's cosf/sinf
   (FMA3 UCRT, constants identical to ucrt_* in game.rs) give the clone's values, so suspect the
   circle centre (func_0x180f45ed0 offset), the radius, or the Vector2/Vector3 conversions.
   Python harness for this: emulate f32 with struct, FMA with fractions.Fraction.
   Committed: RotateToTargetModel.rotateTower off
   (`turn_body` false) keeps the tower's rotation and ejects from it, the shot still aimed
   (`body_rot`; not for base-rotation turrets, dart_paragon); wizard_025 MATCHES.
   Guided Magic rule FOUND (committed): TrackTarget steers at a
   bloon's previous-frame spot when the bloon was created after the projectile (process-list
   order: it hasn't moved yet), else its moved spot (`born` comparison in track_targets).
   wizard_205 first diff 1859 -> 2821, wizard_500 12 position frames left (rt 3355),
   wizard_300 2600 -> 3105. (Bloon::first_move is set but now unused by it.)
   Next (wizard_205 rt 2820): the game's bolt re-targets the frame its Red (popped by the Prince
   ghost in 2820's collisions) dies; the clone looks a frame later on its 5-frame clock. Tried
   and rejected: "look at once when the target is gone" (breaks 205 / 300 / 500 earlier), "look
   clock from the creation frame" (no change). Open: why the game's look lands on 2820.
1. Done (committed): 3-x-x Wall of Fire (TargetSelectedPointModel
   startWithClosestTrackPoint) takes the visible in-range segment point nearest the tower in
   TargetSelectedPoint.Process (@0xdecc20), set in the attack loop (`start_track_point`). That
   made wizard_420_close / 520 bloon-exact (only homing-bolt positions differ) and fixed 320's
   wall. Remaining: Guided Magic steering (300 rt 2600, 320 rt 1902, 400, 420, 520, 205 rt 1859,
   upgrades_r30 rt 2819: small homing position drift), Phoenix 050 / 250 (rt 31: the game has a
   projectile at (-78.47, -24.42) the clone doesn't), 025 / 040 / 052.
1. Other Wizard recordings (025, 040, 050, 052, 205, 250, 300, 320, 420_x, 520; 400 /
   upgrades_r30 differ only in projectile positions): all now carry the seed; work through their first differences.
2. **wizard_500** (Guided Magic steering, open rule in TOWERS.md) and wizard_502_strong (2
   frames).

## Sniper notes (2026-11-01)

Committed: Elite Defender's LeakDangerAttackSpeed now uses the
game's PercThroughMap ((d + ((T - sd) - T)) / (((T - sd) - T) + ld), f32) computed just before
the towers act (`compute_danger`), and a missed-frame InstantDamage extra shot hits in the next
collision pass (HYPOTHESIS P-INSTANTDOUBLE). sniper_005: bloon-exact, weapons same, 5 position
frames (first rt 457). Open: rt 310 with no bloons the game's missedRate is 0.0.
Committed: EmitOnDamage (Shrapnel) emits at once from the bloon
along the shot's EjectRotation (an instant shot's: its tower's rotation) and the new fragments
can't hit that bloon; sniper_025 bloon-exact (26 frames of 1-ulp positions, first rt 114).
Committed: new child objects from an instant shot's pop are kept
out of the next frame's grid until they move (`in_instant`, HYPOTHESIS P-INSTANTCHILD):
sniper_500 / 502 MATCH, 520 bloon-exact.
Latest survey: 005 / 020 / 025 / 205_x / 420_close / 520 bloon-exact (position ulps or an
off-map shot); 030 rt 2591, 032 / 040 / 050 / 052 / 250 first differ at rt 73-74 (an instant
shot at an entrance bloon: game (-147.04, -22.9) vs clone (-153.34, -22.9)).
Committed: instant shots keep their collision-circle offset from
the target as they follow it (`inst_off`); sniper_032's rt 74 difference gone (now 1-ulp at
873; bloons rt 1867: Bouncing Bullet 2530 jumps to Blue 2523 at 1866 and pops it, the clone's
doesn't: RetargetOnContact timing, not looked at yet).
Committed: a retarget (Bouncing Bullet) moves an instant shot's
Target too (`retarget`); sniper_032 bloons now first differ at rt 2209 (three Shrapnel
fragments on one Black in a frame: game leaves a Green at 343.29, clone a Blue at 343.08).
Survey of the other sniper DIFFs (before this): many first differ at rt 73-74 (a cluster of
shrapnel-like projectiles at ~(-156, -21)) and later pop MOABs early; sniper_020 rt 12 (a Yellow
in the game only), 500 / 502 a Ceramic.

## Ninja notes (2026-11-01)

All 34 ninja recordings re-recorded with `# seed` (pending/ninja). Committed:
- TargetTrack (Caltrops; Spike Factory's plain Track mode): `supplied_track_point` = visible
  in-range segment points sorted by squared distance from the tower (stable), forward
  Fisher-Yates with syncedRandom (Next(i, n)), then each throw takes the next point plus a
  two-draw offset (VERIFIED by recording ninja_002: the game's -134.1, -99.1, -113.1, -92.1,
  -106.1 sequence and offsets match exactly).
- TrackTarget's look clock starts when the shot is made; a shot thrown with a target doesn't
  look on its first frame (ninja_002: a shuriken keeps a Black behind the entrance trees).
Committed: Sabotage's SlowMinusAbilityDurationModel exported as a
slow (`minus_age`: lifespan less the projectile's age); ninja_040 first bloon diff 121 -> 2504.
Committed: Wind (Distraction) chance and distance now use the
synced RNG (Wind.Collide @0xcf7fd0, MoveWithWind.Attatched @0xffec70). ninja_040 2504 -> 2682,
ninja_010 240 -> 402. Still to port (decompiled, see those addresses): Wind.Collide skips (no
draw) a bloon already in wind (Bloon +0x224 windCount, held by MoveWithWind until it reaches its
new track spot; MoveWithWind.Process @0xffeeb0 moves its position toward it at the bloon's
speedFrames x a constant x speedMultiplier), not IsBloonValidForPushback (@0xfb4240: on the map,
not behind spawnDist - radius x k), with tagFilter / Moab / boss tags; the clone teleports.
ninja_010 rt 402: the game rolls no wind on a Yellow the clone pushes back.
Uncommitted until the regression passes: MoveWithWind carry ported (Bloon::wind: no track move
while carried, straight at its track spot at speedFrames x speedMultiplier, ends within 3 x
speedFrames), Wind.Collide's skips (already carried, not valid for pushback, tagFilter), the
bloon looked up by id after the hit. OPEN (ninja_010): the clone's wind rolls are the seed's raw
NextDouble sequence from draw 0 (draw 13 = 0.0919 gives a wind at rt 402), but the game's first
wind is at rt 927 and draws 13 and 14 are both <= 0.15, so the game makes extra synced draws
(or skips rolls) before rt 402 that the clone doesn't. Find what else uses syncedRandom for a
0-1-0 Ninja (decompile Wind.Collide's caller order, other projectile behaviours).
Distraction wind DONE (ninja_010 MATCHES; regression held: ninja 7, others unchanged): carry with
`wind` / `wind_new` (holds still the frame it's attached, the first child inherits it with the
parent's place), validity = alive (hp > 0), not carried, on the map, not behind spawnDist - r x
1.25, not within 10 past the first on-map segment's centre, tagFilter; Wind runs after the
DamageModel. SimRecorder now logs every syncedRandom draw as `R d <value>` lines (before the
frame's F line): compare with the clone's draws to find RNG mismatches in minutes. Only
ninja_010 has R lines so far; re-record a tower (with the new recorder, installed) to get them.
ninja_002 MATCHES: Seeking Shuriken is TrackTargetWithinTimeModel (timeInFrames 3.48): the
clamped turn / (timeInFrames / elapsed), clamped again (@0xcf3560); caltrops (ArriveAtTarget,
filterCollisionWhileMoving) set canCollideWithBloons in the Process that arrives, after that
frame's collisions, so they hit from the next frame. Ninja 9/34. Re-recording (with R lines) was
paused: the PC ran out of commit memory (Ghidra/java can't start, one game launch crashed); only
ninja_000_close and ninja_010 are new. Also done: Shinobi Tactics' pierce (MutatorTower.Mutate
@0xd92090: + (1 - multiplier) x the original pierce per stack, `Buff.pierce_pct`); CollideBloon
exhausts once pierce < 1 (a 0.16 rest is spent); RandomAngleOffset (@0xc8fe40) turns each shot by
a synced draw after the eject point. Ninja 12/34. OPEN: with Shinobi, about 1 shot in 6 is made
with +1 pierce (3.16, ninja_030/040: irregular, not the target, not Wind.Initialise): likely a
synced roll; needs a recording with R lines. ("weapon 1 first throw 120 vs 134" in 040/050/250
is only units: the game's frame count, rt + 14.) TrackTarget's new-target pick takes, on the
nearest segment, the valid bloon first in Path.bloonsList order (path_rank), not storage order:
ninja_502/502_close MATCH, wizard 21 -> 26, sub 14 -> 16.
Wind, more (ninja_020 MATCHES, R lines now in pending ninja_020/030/040): no roll after an immune
hit (Leads); the hit's other children take the MoveWithWind mutator, each with its own distance
draw (a third synced draw that frame), clamped the same; a carried bloon stays filed on the
segment it was blown from (`wind_seg`: towers in range still target it); a carried child's
first-Process `fresh` is used up. RRBTD6_RNG_LOG=1 with `--one` prints the clone's draws;
compare them to the R lines (scratch script: pair R lines by the next F's rt, find the first
draw whose rt differs). Ninja 16/34.
OPEN, Shinobi +1 pierce (030/040): some throws are made with 3.16 instead of 2.16, every 6-8
throws in a run (030: rt 223, 427, 631, 869, 2649, 2853 ...), no synced draw at the throw, not
the target, eject point, rotation, missed frames (W rate 34, missed 0) or pooled leftovers.
AlternateProjectile counts per weapon (GetAnimation @0xc690c0: count + 1, back to 1 past interval,
alternate at count == interval): Flash Bomb every 2nd throw of that weapon. Slow.Collide (@0xcde360) with
matchLayersWithDamage: layers + (int) the projectile's damage (Flash Bomb's stun 1 + 3 = 4).
Grow (@0xff1b70): on its first Process on a model with overrideChildWithColor (White, Purple:
Pink; Lead: Black) the bloon keeps (child colour, parent colour) and every child copies them; a
bloon whose base has the child colour grows into growToId with its colour swapped (a White's
Pinks grow back into Whites). A White a hit passes straight through never sets them (dart_400).
ninja_003 MATCHES. Then ninja_004/005 MATCH: EmitOnTime (Sticky Bomb splash) waits a frame
like a new DoT, fires from where its bloon is after the move, with that bloon in its
collidedWith; bloon states match mutators whose id contains the state (GetMutatorsByKey
@0xf36040: "StickyBomb" = StickyBombSplash); P-LIFEFRAMES mutator lifespans truncated to whole
frames. TrackTarget's seek-angle and line checks take a bloon newer than the projectile at its
previous spot (ninja_502_last/strong). Ninja 22/34.
Wind, more (320/420_* re-recorded with R lines; 420_strong MATCHES): MoveWithWind.Attatched
(@0xffec70) adds offTrackCount and takes the distance off distanceTraveled at once (clamped);
OnDestroy (@0xffee80) takes the count off. A carried bloon is rolled for again only when the hit
pops it into more children (its behaviours are remade); every child of a carried bloon copies the
wind (no draw) from the parent's spot and distance; IsBloonValidForPushback's behind test is
strict (<). OPEN (ninja_320 rt 3150 / 3176, 420_close 3286, 420_last 3019): the game rolls for a
carried White at distanceTraveled 173.6, which the clone's "entering" rule (first on-map segment
centre + 10 = 195.5) rejects; that rule came from ninja_010 rt 15 (no roll on a Black at 186.7,
x -146), which the game's Map.IsPointOnMap may explain instead: decompile it and compare with
`on_map`. Ninja 23/34.
Shinobi + Caltrops (032/052/025/050/250/520 re-recorded with R lines; the scratch draw compare must
read every R kind: `R r lo hi res` is Next(lo,hi) = (int)(d x (hi - lo)) + lo): Shinobi's pierce
share comes from the first projectile in the tower with the same id (the caltrops get 8% of the
shuriken's 2: `shinobi_base`); support mutators reach a tower from the frame after it is placed
(its rt 1 Caltrops 6 pierce); a model change (support set) remakes the TrackTargetSupplier points
on the attack's next Process, firing or not, and the cursor carries on (P-TRACKCURSOR).
The +1 pierce SOLVED: Weapon.fractionalPierce (as EmitOnDamage.Collide @0xcbb8c0) adds up each
shot's pierce - (int)pierce; at 1 that shot gets +1 (Shinobi's 0.16: every 6-7 throws). Between-
rounds weapons (Sabotage) keep lastEmission on sim_elapsed (P-SIMTIME). Ninja 27/34. An ability's
temp support reaches towers from the frame after activation and leaves a frame after it expires;
the TrackTargetSupplier points use the tower's range at the time (ninja_052). Ninja 28/34. Left:
Wind model SOLVED (supersedes the guesses above): a bloon the wind carries (offTrackCount, from
Attatched at once) is never rolled for; the success's hit children take the mutator, each with its
own Attatched draw; and every NEW object a carried bloon's pop makes copies the mutator
(CopyMutatorsFromBloon @0xfb0f40 -> Mutable.CopyMutatorsFrom @0xf35ac0: AddMutator, ApplyMutation),
so it gets its own Attatched: a distance draw and push from the parent's spot and distance (the
first child, the parent object, keeps its wind, no draw). The entering rule stands. ninja_320 and
420_close MATCH. Ninja 30/34. A carried bloon that degrades is filed on its path again at its
distanceTraveled then (no longer targeted from the segment it was blown off: 420_last MATCHES).
Ninja 31/34. The success's child draws go once per object (a child taken further is listed again);
P-SEEKOFFTRACK: TrackTarget doesn't take a carried bloon as a new target (032 MATCHES). Ninja 32/34.
NINJA DONE, 34/34: Path.Process (@0xf6fe60) files on its segments only the bloons with
offTrackCount 0, so a carried bloon is on no segment; Targetting.GetTargets (@0xffa980) takes the
off-track ones after the segments' targets (P-OFFTRACKRANGE: within range + radius;
P-OFFTRACKORDER: newest factory object first). (The wind_seg filing above now only matters for
seek / closest, which skip carried bloons anyway.)

## Log (newest first; one entry per checkpoint)

- 2026-11-02: Sniper notes: H-line timing, the Instant-shot removal pattern in sniper_040 / 205_x.

- 2026-11-02: Toggled-off abilities, exact RandomPositionBasic, Instant follows newer bloons' previous spot.
  sniper_040 121 -> 1902. Regression unchanged.

- 2026-11-02: Pre-leak abilities (Bomb Blitz), abilities start at 33% cooldown, per-round limits, harness keeps
  cooldown on the scenario ability. Bomb 30 -> 33/34; rest unchanged.

- 2026-11-02: AbilityDamageAll (Bomb Storm), dontUseCollisionChecker skips map blockers too. Bomb 28 -> 30/34;
  rest of the regression unchanged.

- 2026-11-02: dontUseCollisionChecker projectiles skip collisions; RandomRangeTravelStrait lifespans drawn from
  syncedRandom. bomb_005 first difference 78 -> 88. Full regression unchanged.

- 2026-11-02: Now reordered: Wizard is done (its notes moved to Wizard notes), Bomb is next.

- 2026-11-02: Banana Farmer Power placeable (CollectCashZone already simulated); 16 Power tower models exported.
  Regression: dart 70/70, boomerang 99/99.

- 2026-11-02: Tower Powers: data for all 17, Cave Monkey placeable (the only one with every behaviour
  simulated). Full regression unchanged: dart 70, boomerang 99, wizard 34, sub 16, tack 17, bomb 28,
  sniper 19, ninja 34, engineer 7, desperado 6.

- 2026-11-02: tools/il2cpp/type_usage.py names the class behind a TypeInfo slot.

- 2026-11-02: Dart Time Power (bloon speed x 0.3, tower rate x 1 / 0.7 for 10 s). Regression: dart 70/70,
  boomerang 99/99.

- 2026-11-02: Track Powers (Road Spikes, Glue Trap, MOAB Mine, Camo Trap) and their displays. Regression:
  dart 70/70, boomerang 99/99.

- 2026-11-02: Cash Drop Power (its crate projectile and mesh). Regression: dart 70/70, boomerang 99/99.

- 2026-11-02: Thrive Power (pop cash x 1.25 this round and the next, ThriveGlow). Regression: dart 70/70,
  boomerang 99/99.

- 2026-11-02: Using Powers: Super Monkey Storm and Monkey Boost (pick in the menu, click the map). Tests pass;
  regression dart 70/70, boomerang 99/99.

- 2026-11-02: game/powers.json keyed by the Powers' ids (Pro: powerId), so the menu finds Pro rules.

- 2026-11-02: In-game Powers menu (open / close, Power buttons, counts, states); sandbox SBPowersBtn opens it.
  Using a Power not built yet. Tests pass (powers_menu, sandbox, hud).

- 2026-11-02: Setup exports the in-game Powers menu's Animators (slide in / out).

- 2026-11-02: Glaive Lord orbit glaives drawn from the sim (eased, circular), RotateModel spin, Glow
  trail; sprite layer export in 3D. Regression: dart 70/70, boomerang 99/99.

- 2026-11-01: Wind chance / distance from the synced RNG. Regression unchanged.

- 2026-11-01: Sabotage (SlowMinusAbilityDuration) slows; ninja_040 first diff 121 -> 2504. Regression
  unchanged.

- 2026-11-01: TargetTrack shuffled segment list (Caltrops), TrackTarget look clock from creation.
  Wizard 21/34; ninja re-recorded with seeds (6/34); regression otherwise unchanged.

- 2026-11-01: Bouncing Bullet retarget moves an instant shot's Target. Regression unchanged.

- 2026-11-01: instant shots follow their target with the collision-circle offset. Regression unchanged
  (sniper 19, wizard 20, dart 70, boomerang 99).

- 2026-11-01: instant-pass pop children wait a frame for the grid (P-INSTANTCHILD). Sniper 19/34;
  regression otherwise unchanged.

- 2026-11-01: EmitOnDamage emits at once along the shot's EjectRotation, its fragments can't hit
  the bloon. Sniper 17/34; regression otherwise unchanged.

- 2026-11-01: Elite Defender leak danger (PercThroughMap, at shot time), missed-frame instant shot
  next pass; sniper_005 bloon-exact. Regression unchanged.

- 2026-11-01: TrackTarget steers at a newer bloon's previous-frame spot (process order). Wizard 20/34;
  regression otherwise unchanged (dart 70, boomerang 99, sub 14, tack 17, bomb 28, sniper 14, ninja 6,
  engineer 7, beast 9, ice 20, glue 33, desperado 6).

- 2026-11-01: rotateTower off keeps the tower's rotation for the eject (wizard_025 matches).
  Regression: dart 70, boomerang 99, wizard 19, tack 17, bomb 28, sniper 14, sub 14, ice 20, glue 33,
  ninja 6, engineer 7, beast 9, desperado 6.

- 2026-11-01: Phoenix MoveTowards squared length fused like the game's build (wizard_050 exact to rt 90).
  Regression unchanged (dart 70, boomerang 99, wizard 18, beast 9, engineer 7).

- 2026-11-01: ignorePierceExhaustion (spent shots stay), RefreshPierce after the collision pass.
  wizard_040 / 050 / 250 bloon-exact (only Phoenix 1-ulp positions differ). Regression unchanged.

- 2026-11-01: FilterInvisible shots without camo detection pass camo bloons; Phoenix LerpAngle in
  Unity's float steps. wizard_050 bloon-exact. Regression: dart 70, boomerang 99, wizard 18, tack 17,
  bomb 28, sniper 14, sub 14, ice 20, glue 33, ninja 6, engineer 7, beast 9, desperado 6.

- 2026-11-01: Phoenix shots start 250 up (P-PHOENIXZ): wizard_050 258 bloon frames (first rt 3341),
  wizard_052 11. Regression: dart 70, boomerang 99, wizard 18, beast 9, engineer 7.

- 2026-11-01: Phoenix path switch on DistanceSquared; Summon Phoenix mute + 1 frame (P-MUTEFRAME).
  wizard_050 bloon frames 663 -> 403, main weapons all match. Regression: dart 70, boomerang 99,
  wizard 18, mermonkey 0/34 (not compared before), beast 9, engineer 7.

- 2026-11-01: Phoenix pets start a frame later (P-PETFRAME); wizard_050 bloon frames 1707 -> 663.
  Regression unchanged (dart 70, boomerang 99, wizard 18, beast 9, engineer 7).

- 2026-11-01: Lord Phoenix flight (PathMovementFromScreenCenter), harness spawns tier pets, Phoenix
  shoots from its rotation. wizard_050 first difference rt 31 -> 37/123. Regression: dart 70/70,
  boomerang 99/99, wizard 18/34, beast 9/34, engineer 7/34.

- 2026-11-01: Wall of Fire start point (startWithClosestTrackPoint) from TargetSelectedPoint.Process;
  wizard_420_close / 520 bloon-exact. Regression: dart 70/70, boomerang 99/99, wizard 18/34, spike
  0/25 and engineer 7/34 unchanged vs before.

- 2026-11-01: wizard_030 matches: an immune hit that destroys the shot leaves no after-damage spawn.
  Regression: dart 70/70, boomerang 99/99, wizard 18/34, tack 17/25, bomb 28/34, sniper 19/34,
  druid 0/34, desperado 6/34.

- 2026-11-01: wizard_005 matches. Prince's own path range / offset; live rate-stack mutator x (1 - 0.1 k);
  damage stacks on every DamageModel of the tower; undead lose the zombie lifespan buff while a rate
  stack is on. Regression: dart 70/70, boomerang 99/99, wizard 17/34.

- 2026-11-01: wizard_004 matches. NecromancerZone.OnBloonDegrade / AssignRbeToTower (numOfDegrades,
  a synced draw per degrade, Next(7, 11) past round 80), Next(1, 1) still draws, undead pierce =
  base + share, undead y offset sign. Regression: dart 70/70, boomerang 99/99, wizard 16/34.

- 2026-11-01: wizard_020 matches. CollisionGrid bucket is where the bloon was filed
  (BloonGrid::visits); CreateProjectileOnContact after the DamageModel (collisionPass, then
  model order) runs after the damage, at the shot when the bloon was destroyed.
  Regression: dart 70/70, boomerang 99/99, wizard 15/34, tack 17/25, sniper 14/34.

- 2026-10-31 688a544: Prince of Darkness aims at its bloon (EmissionRotationOffBloonDirection),
  Necromancer graveyard gate / First-list order / synced RNG, Boomerang hand buttons.
  Regression: dart 70/70, boomerang 99/99, wizard 13/34, ace 0/25.
- 2026-10-31 bdb1023: Wall of Fire start cooldown, track point and seed; trophies; co-op
  profiles; UI work.

## Mortar 25/25 (2026-11-08)

- Seeded recordings are in research/dumps/recordings/pending/mortar. All 25 match.
- Done:
  - RotateToPointer rotateOnlyOnEmit: one SmoothDampAngle step per throw start.
  - Target spread: sin/cos x (d x radius).
  - applyOnlyIfDamaged refuses immune bloons.
  - P-INSTANTFRESH: an instant shot doesn't meet objects popped out earlier in the same collision pass.
  - DamageInRingRadius: 3D distance, bloon at its Position.z (`height`: 20 / MOAB 120; new
    unmoved objects 20, P-RINGZ), blast at 0.
  - (P-CELLCIRCLE retired 2026-11-16: CollisionState.MoveNext @0x1824b4110 tests the bucket's
    grid-local centre against the world-space query box; see grid_hit.)
  - DoT intervalFrames = (int)(interval x 60), truncated.
  - Projectile FilterWithTagModel (MoabBurn).
  - P-ABSMARKER: Mortar 0-4-x+ shells start at the weapon's ThrowMarkerOffset markers, cycled by
    the throw count (a missed-frame double takes the next without advancing), turned and raised
    from (0,0). Not traced in code: no behaviour overrides EmitPosition, so whatever supplies the
    eject point here skips the tower position.
- Open:
  - 050 rt 178: two blasts made the same frame (rate 4.05 frames) live 2 and 3 frames in the game
    (1549 and 1535), the other way round in the clone. Look at the creation phase of the second
    (double) shell's exhaust.
  - 052 rt 1198 and 250 rt 673: one bloon each. In 250, blast 5331 destroys a Black (H health 0) at
    rt 671 and again hits its new Green child 5346 at rt 673 (pierce 44 -> 43, health 0); the clone's
    child inherits the blast's hit (lineage) and is spared. The child sat behind the entrance (166.9)
    and is snapped to 173.6 that frame. When does a child not inherit collidedWith?
    Checked 2026-11-08: SpawnChildren.CreatedChildren (@0x181008930) copies the parent's list at
    +0x180 onto every child it makes (copies too), then Bloon.UpdateLayerId on the parent. Here the
    parent 5298 itself was destroyed (H health 0; Degrade's RootObject.Destroy path), leaving only
    the copy 5346. Next: read Projectile.CollideBloon to see which id the blast records (the parent's
    id before or after UpdateLayerId) and whether +0x180 holds the parent's own current id.
    (Now: 025 first differs at rt 853, 052 at rt 1198 and 250 at rt 673, all on a bloon at the
    173.6 entrance snap.)
  - DONE 2026-11-08 (025 matches, mortar 22/25):
    - An instant shot's candidates are always put in grid order, even with pierce for all of them:
      the order sets the order their children are listed in (rt 945).
    - A bloon in the grid ranks by the bucket it was filed in (cell_pos), then its slot in that
      bucket (path-list order at the fill), not its current position / creation order (rt 2329: a
      White listed before an older Black in their cell is hit first).
    - HYPOTHESIS P-DEADCOPIES: when a projectile's hit destroys the bloon's own object (no first
      child carrying it on), the copies that pop made don't carry the layers up to the hit one, so
      the same projectile can hit them again (250 rt 633 / 652 / 672 / 843 re-hits). Not for DoT
      damage (glue_205's Ceramics stay protected). Not traced in code: SpawnChildren
      .CreatedChildren copies the parent's lineage (+0x180) to every child and UpdateLayerId lists
      each new id. Treating spawned or copied bloons' first ids as unlisted broke every MOAB.
  - DONE 2026-11-08 (050 052 250 match, mortar 25/25): Projectile.Process runs in creation
    order (the process list), not the factory list's order after a flush. Exhaust / expire
    projectiles are made in the creation order of the shots they came from (Exhaust.src; 250 rt
    670: shell 5086 expires before 5098, listed after it, so its blast takes the Black first).

## Banana Farm 25/25 (2026-11-10, DONE; re-recorded with seeds, old ones in pending/farm_old)

- CHIMPS refuses a Banana Farm in InputManager.CreateTowerAt; SimRecorder now makes such a tower
  with TowerManager.CreateTower (Scenario.forcePlace, default on), as the clone's harness does.
- EmissionsPerRoundFilter: RoundStart @0x180c76150 (roundTime + 1, RoundData.roundDuration = the
  latest emission time, Spawner.AddEmissions @0x180f71bf0), Process @0x180c75cb0 every frame:
  banana i due at (int)(i x duration / (count - 1) + start), FilterEmission first so it goes the
  next frame. RandomPositionBasic draws its point the frame before the weapon is ready, once per
  banana. Weapons made after the round began get its entry.

## Mermonkey 34/34 (2026-11-10, DONE; re-recorded with seeds, old ones in pending/mermonkey_old)

- DONE: SupportStackingRange (MutatorTower.Mutate @0x180d91c50): per Mermonkey x-x-2+ (global, max 10)
  range, attack ranges and the first TravelStrait lifespan x 1.075 (set_Lifespan rounds half up).
- DONE: Trance. TranceBloon.Collide @0x180cf4b30 adds the mutator (999 layers) and SetOrbitTower;
  MermonkeyTrance.Process @0x180fbf560: 3 a frame to tower + Rotate((0, 30), angle), circle 3 deg a
  frame at z +45, back to its track spot, then cooldown; Bloon.Process comes first (the frame the walk
  ends it still holds). The ring sits on its tower (TranceBloon.Process). Tranced bloons are off
  track; in a tower's off-track search they count in their current cell (P-TRANCECELL), in the
  CollisionChecker a frame behind. PauseOtherAttacks @0x180c802d0. LinkProjectileRadiusToTowerRange
  matches weapons by projectile id (the trident gets the ring's radius).
- DONE: 3xx-5xx: FilterTargetAngle @0x180c402b0 (flat, <= fov/2), RotateToTargetAttackOffset
  (SetRotation @0x180ddb500 from the offset spot raised 10; lost target: RotateToStartingPoint; the
  tower rotation untouched), plain-emission Instant shots start at the eject point, ignoreTargetZ,
  a placed dontFollowTarget shot outlives its target.
- DONE (030 / 040 match): ScaleProjectileOverTime (Process @0x180cdaa20: radius = scale x model,
  scale = t/60 x sps + base, <= 0 expires; CreateBonusProjectile @0x180cd9bb0 once on expire / exhaust /
  map hit, map hit vsBlockerRadius x 3 back, the bonus's baseScale = min(parent scale a Process back,
  max); ArcEmission.useProjectileRotation). RotateToTarget every frame (P-ROTALWAYS, target a frame
  back). A freeze doesn't hold a bloon its damageModel is immune to (P-FREEZEIMMUNE). processOnActivate
  attacks act lifespan + 1 frames (P-ACTIVATEFRAME). AddHeatToBloon only acts on OverheatTrigger bloons.
- DONE (050/052/250): a DamageModifierForBloonState counts the shot's own pre-damage effects
  (Snowstorm slow 'Freeze:...' makes its hit +70); MapBorderRebound.clearCollidedWith; useProjectileRotation
  follows a turned Direction (dir_rot); a TravelStrait-expired bonus starts where its parent was.
- DONE (004/005/025/205): TranceTotem as a pet, made the frame after (spawner Process), on the
  Mermonkey's spot, with its range mutators (SyncMutators); TrackTarget keeps tranced targets;
  IsTargetLineClear from the bloon's height (a MOAB at 120 over a log).

## Druid 34/34 (2026-11-09, Druid DONE)

- SOLVED druid_420_strong rt 3457 (MATCHES): CollisionGrid MoveNext (current build @0x1824b4110,
  shared generic code; GetInRange @0x1824b34e0, Initialize @0x1824b3ed0) takes every bloon in the
  query's own bucket (i == 0) with no distance test; other buckets: inside the query box, else
  distance <= (withRadiusOverride if >= 0, else the bloon's radius) + the query's radius, then
  AdditionalCollisions. Thorn 5718 at (-100.69, 21.04 clone frame) shares cell (10, 14) with Red
  5620 filed at its frame-start spot -92.74 (9.8 apart now). The Red is the Yellow a lightning
  bolt degraded after its move at rt 3456: the same object, so the grid still has it where it was
  filed. Clone: Bloon.cell_next, set on the first child of a pop made after the parent moved
  (pos != prev), the parent's prev (or its own cell_next through a chain of degrades: Yellow ->
  Green -> Blue -> Red is three pops); the next frame's re-file takes it instead of prev.
  (Setting it for every pop broke 30 recordings in other towers: stale on fresh children / CC pops.)
- SOLVED druid_502_strong rt 2018 / 2112 (MATCHES): RandomEmission.Emit (@0x180c4a7a0) from a
  projectile (CreateProjectileOnInterval) bases its angle on Vector3Boxed.get_Rotation of that
  projectile's Direction (+0xc0: atan2(x, y) x Rad2Deg), not its velocity. The clone's
  Projectile.dir is that Direction in the GAME frame (y up; vel is clone frame); MapBorderRebound
  now updates it too (normalized reflection, as the D lines show: -0.9337585 -> 0.933759).

## Druid 23/34 (2026-11-08)

- SOLVED druid_052 rt 285 (druid_052 MATCHES): JungleVineLimitProjectile. FilterEmission
  (@0x180c7b880) lets the vine weapon shoot only while one of its `limit` slots (UpdateLimits:
  made at -999999999) is at or before Simulation.roundTime. JungleVineEffect.Collide
  (@0x180cc23b0), not on MOAB-class: duration = (sqrt(GetTotalLeakDamage) + 1, +5 with the Lead
  tag, +10 with Ceramic) x 0.125; AddLimiter (@0x180c7b790) on the emitting weapon fills the
  first free slot with roundTime + delayInFrames + ageingDestroyModel.timeFrames + (int)(duration
  x 60). timeFrames = (int)(time x 60) (AgeingDestroyModel.set_Time). The 0-5-x vine's
  destroyAfterPopTime is 0.3 (18 frames, not 84): the Ceramic grabbed at 143 holds the vine to
  143 + 18 + 158 = 319. Exported as projectile vine_frames and weapon vine_limit.
- DONE: Ball Lightning's CreateProjectileOnInterval (Process @0x180ca55e0): BloonManager.GetTarget
  filters with Projectile.FilterBloon (@0x180c8c980), which skips bloons whose lineage the ball has
  already hit (druid_500 rt 97-196: ball 491 among the carried MOAB and Ceramics it touches never
  fires; the clone bolted them at 96 and 138). With onlyIfHasTarget and none it returns before
  createNextProjAt moves, so it tries again each frame. druid_500 / 502 / 520 now go to rt 138,
  druid_400 to rt 967.
- DONE: MoveWithWind.Process (@0x180ffeeb0) steps in 3D: from Position (z included) to the path
  point at its distance, whose z Path.DistanceToPointInfoBloon (@0x180f6e030) makes 180 for
  isBoss, 120 for isMoab, else 20. Step = normalised 3D direction x BloonModel.speedFrames x
  speedMultiplier; the wind ends within 3 x speedFrames (3D). A carried bloon runs no Bloon.Move,
  so it keeps its z (Bloon.height no longer reset while carried): a MOAB's first Ceramic child
  keeps the MOAB object's 120 and goes 0.945 a frame across where the other Ceramics go 2.604
  (druid_500 Ceramic 443). druid_5xx bloons now match to rt 448-469.
- DONE: MoveWithWind is a mutator keyed "Wind" (WindModel.get_mutator @0x181203f00), so
  FilterMutatedTarget "Wind" (the Tornado's) passes over carried bloons (druid_500 rt 141: the
  game's Tornado keeps pierce 60 and turns for Ceramic 443). druid_5xx positions match to 440-448.
- DONE (druid_500 / 520: bloons and weapons match all 3600 frames; 87 projectile-position frames
  left from rt 1546):
  - A carried bloon's pop: BloonDegradeStepper settles what the damage leaves before children are
    made, so a new object the same hit destroys gets no MoveWithWind and no distance draw (rt 316:
    the Superstorm's 100 popping carried Blacks outright draws nothing). The draws now wait to the
    end of the top-level damage, for the children still there.
  - CreateProjectileOnInterval: OnEmit sets createNextProjAt = roundTime + intervalFrames as it is
    made, so the creation frame doesn't count (ball made at 448 bolts at 469; a Superstorm from 316
    spawns its ball at 496). Its target (onlyIfHasTarget) is BloonManager.GetTarget: TargetType
    Random goes through GetTargets unsorted (no PathManager global best), so it is the first bloon
    in segment order (GetPathSegmentsInRange: range + 40 unless it can't hit MOABs, paths in order,
    segments ascending; Path.bloonsList order; plain within range + 2, else MOAB circles), then the
    carried ones (no range) in BloonGrid order; no draw. With range > 0 the emit point moves by
    Vector3.RandomOffsetFromRadius (@0x180f47ff0): r = (float)NextDouble x range, angle Deg2Rad x
    -(NextDouble x 360), x + cos x r, y + sin x r (two draws per bolt). The bolt sits at that point
    on its first frame, then Instant puts it on its Target. Without a target the emission turns from
    the emitting projectile's heading (the Superstorm's 57.56 degrees + the RandomEmission's spread).
- DONE (druid_500 / druid_520 MATCH): Lightning.Process, TrackTarget and CreateProjectileOnInterval
  run in process-list (creation, uid) order when bolts are out: Game::lightning_phase(bound) runs
  the bolts made before each seeker / interval projectile first (a bolt that has hit or bounced
  isn't due again in the frame, so the later full pass is safe). druid_500 rt 1546: Tornado 2205
  loses Zebra 2148 to the earlier Lightning 2195 and flies straight; druid_400 rt 2073: Ball 778
  looks before Lightning 1242 pops the Black beside it and finds nothing.
- DONE (druid_400 to rt 2369): a freeze sets a non-MOAB model's speed to its own (0), and
  MoveWithWind steps by BloonModel.speedFrames, so a frozen carried bloon holds still (rt 2220). A
  carried copy made in a Process (a lightning pop, after the moves) steps on the next frame; only
  collision-pass copies wait a frame (rt 2250). Such a copy is `unplaced` (not in the grid until
  its next fill); a carried bloon clears that at its MoveWithWind Process, not only at a track
  move (rt 2267: thorns hit both frozen Greens).
- DONE (Druid 27/34, druid_502 matches): interval shots run in uid (process-list) order even
  with no bolts out (rt 1569: Superstorm 1205's ball spawn draws before Ball 2252's bolt). The
  interval target sees the frame-start filing: a bloon blown off this frame (wind_since this
  frame) is still on its segment and not on the off-track list; off-track ones only within range
  + their radius (the BloonGrid query; druid_400 rt 2369). A spawn without a target turns from the
  emitter's direction as atan2(x, y) of its velocity in the game's frame (rt 1629: one float step).
- DONE (druid_upgrades_r30 matches, Druid 28/34): an attack lists its weapons in creation order
  (Attack.UpdatedModel keeps those it had and adds new ones after; WeaponState.seq), which is how
  the W lines number them. HYPOTHESIS P-ROTLIST: RotateToTarget.Process (@0x180dd80d0) turns the
  tower while a weapon in its cached list is in its throw, and a weapon an upgrade added isn't in
  that list, so its throw turns the tower only as it starts (rt 2547-2549: the 5-1-0 Superstorm's
  throw holds its facing). WeaponState.initial marks the placement weapons. Every tower's upgrade
  recording still matches.
- DONE (Druid 29/34): each lightning shot does its bounce search or its hit in its own Process,
  in creation order (Game::lightning_phase runs lightning_phase_one per uid; the pass is
  idempotent). druid_400 rt 2625: bolt 2577 pops a Black before bolt 2587's search picks the Green
  it became, so 2587's family holds the Green's layer and 2587 can take the Black's copy at 2629.
- DONE (Druid 30/34, druid_420_last matches; druid_502_strong bloons + weapons exact):
  - Bloons and projectiles share a creation count (cseq): a lightning Target with the shot's own
    born stamp but made after it (a copy its first hit made) is hit where it stood (420_last rt 2766).
  - A frozen Target newer than the shot is moved after the hit too: if the hit thaws it
    (SpawnChildren.CreatedChildren depletes charges by DamageResult.changeInLayers, Yellow -> Red
    = 3) it steps that frame (420_last rt 3028).
  - CreateProjectileOnInterval's off-track candidates: any carried bloon in a grid cell of the
    square range around the shooter, no distance of its own (502_strong rt 1903: one 52 away taken
    at range 43; druid_400 rt 2369: one 62 away in a cell outside left).
- DONE: HYPOTHESIS P-OFFCELL: the interval target's carried candidates, within one grid cell, the
  later-filed first (502_strong rt 1903: Greens 2845 / 2854, ranks 1 / 2, 2854 taken; bloons and
  weapons exact there now). A lightning bounce skips bloons not yet moved (`unplaced`: Bloon.Move
  sets the path Path.Process files by) unless carried (420_strong rt 2625: a pop's copy behind the
  entrance left; 420_strong to rt 3121, 420_close to rt 3357).
- OPEN: 502_strong rt 2112, a Superstorm ball one float step off after 95 frames: its spawn base
  rotation from atan2 of the storm's velocity; atan2 of velocity / speedFrames fixes it but breaks
  502 / 502_last the same way, so neither is the game's (find the storm's own direction).
- DONE (Druid 31/34, druid_400 matches):
  - A lightning bounce's carried candidates, within one grid cell, in the bucket's fill order
    (ggrid slot): 420_strong rt 3180 (4361 slot 0 over 4245 slot 2), druid_400 rt 2252.
  - HYPOTHESIS P-OFFSTRONG: a Strong attack's off-track walk takes the strongest (danger), grid
    order among equals (420_strong rt 3121: carried Yellows over the Reds beside them).
  - Removed "blown off this frame stays on its segment": 420_strong rt 3419, a Red a Tornado blew
    that frame is a Ball Lightning's off-track target (druid_400 rt 2369 is the grid-cell square).
- DONE (Druid 32/34, druid_420_close matches): a lightning bounce's carried candidate is ranked by
  the grid cell of where it is at the bolt's turn in the process list: moved if its MoveWithWind
  (wind_since) comes before the bolt, else at its frame-start spot (prev). 420_close rt 3435: a
  Yellow picked up after the bolt was made, moved out of the bolt's cell, is still taken first;
  druid_320 rt 3113 (the older carried Blues, moved) still fits. (The ggrid cell alone broke 320.)
- NEXT: 420_strong rt 3456/3457: the game's thorn 5718 (radius 0, pierce 1, at -100.69, -21.04
  after its move) destroys Red 5620 (at -91.07, -22.9, radius 8: 9.8 apart) in the collision pass,
  so bolt 5650's bounce finds nothing; the clone's thorn misses it (grid_hit false), and the bolt
  takes the Red. Work out what puts them in reach (the bloon's spot at the pass, a swept test, the
  grid's shared-cell rule) before changing anything. LEAD (2026-11-09): the thorn (speed 340, 5.67 a
  frame along D 0.6837 / 0.7298) does pop it (H health is after the hit, HB "True" = destroyed).
  Endpoints: 9.8 from the Red's spot then (-91.07), 8.16 from its frame-start spot (-92.74); the
  thorn's next step swept against -92.74 comes within 7.07 (< 8). Check Projectile.CollideBloons /
  CollisionChecker for a swept or look-ahead test before porting. 420_close rt 3436. DBGE=1: "DBGAIM throw / interval /
  offtrack / hit / bounce" (bounce prints each candidate's segment-point distance and the range).

## Druid 22/34 (2026-11-08)

- SOLVED druid_320 rt 3394 (druid_320 MATCHES). SimRecorder now logs wind-carried bloons: "WC id
  offTrackCount x y distance" as each frame starts, "WM bloon model speedFrames x y" after every
  MoveWithWind.Process, "BO id offTrackCount" in the frame's bloon dump (a Harmony patch on
  Bloon.Degrade broke the game: its trampoline throws, so degrades never happen; don't patch it).
  Fresh probe recording pending/druid_probe/druid_320w (matches).
  - A carried bloon steps in its MoveWithWind behaviour's Process, whose place in the process list
    is its pickup (WM lines interleave with the bolts' CB hits, old bloons stepping after newer
    bolts). A lightning Target picked up after the bolt was made is hit at its frame-start spot and
    steps after at what it has become (rt 3394: two Yellows turned Blue step 0.729); one picked up
    before steps first (probe rt 3394: Green 5240 steps as a Green, then turns Red; rt 2838). No
    degrade redoes a step that is done. Bloon.wind_since = pickup time x 4 + phase (the first child
    keeps it, other children of a carried pop get theirs at the pop).
  - The lightning's carried candidates go in collisionChecker.BloonGrid query order from the bolt:
    cell rank of where each one is now (moved this frame), same cell by creation (INFERRED). Probe
    druid_320w rt 3113 (2810, 3161, 3152 in cells (7, 3), (8, 3), (8, 4)) and druid_300 probe b rt 875
    (Red 875 a cell nearer than 790; same start-of-frame cell) both fit; factory-list, pickup-order and
    object-number orders each fail one of them.

## Druid 21/34 (2026-11-08)

- Re-recorded with seeds into research/dumps/recordings/pending/druid (old unseeded ones in
  pending/druid_old). The full regression now includes `druid`.
- DONE: OffsetModel (thorns, range 45): Offset.ProjectilesCreated (@0x180c7fa10). After the
  RandomEmission has made all five (three NextDouble each: offset direction, radius, rotation),
  each thorn in turn gets angle = Next(0, 1000) x range / 1000 - range / 2 + angleOffset. Its
  direction is turned by it (Vector3Boxed.Rotate) and its rotation + it. 000 / 000_* / 001 / 010
  / 020 / 100 match.
- DONE: LifeBasedAttackSpeed.GetRate (@0x180c7bfb0): rate / (baseRateIncrease + 1) at or above
  calculateFrom (Attatched @0x180c7bf20: the lives then, clamped to 0..Simulation.startingHealth),
  under it / (min(lost, lifeCap) x ratePerLife + 1 + baseRateIncrease) (002: 66 -> 60 frames).
- DONE: DamageBasedAttackSpeed (OnBloonDamaged @0x180d19ee0, Process @0x180d1a0a0, UpdateRate
  @0x180d1a160): currentDamage accumulates, stacks = (int)(currentDamage / threshold) clamped to
  maxStacks (nothing taken off per stack); idleFrames after the last damage it is re-rated and
  zeroed; each stack's mutator is a rate / (1 + stacks x increase). HYPOTHESIS P-OWNDAMAGE: a
  hit counts its damage at most the health down the hit bloon's own line, not what the copies
  took (005 rt 931-963). 002 003 004 005 match.
- DONE (030 032 040 match): the Jungle Vine attack's filters. FilterOutOffscreen
  (@0x180c3f5c0): Map.IsPointOnMap of the bloon, with its radius as border when
  includeBloonRadius (0-3-0 has both, so its centre must be on; 0-3-2 has only the radius one).
  FilterWithTagsModel (the list form, inclusive Moabs / Frontier: no MOAB-class) exported like
  FilterWithTagModel.
- Open: tornado 3xx+, 05x (Spirit of the Forest: 050 bloon-exact to rt 157), 4xx/5xx.
- Spirit of the Forest PORTED (2026-11-08):
  - SpiritOfTheForest.Initialise (@0x180d93d40) / GeneratePositions (@0x180d93560): every second
    path segment (0, 2, 4...), not bloonsInvulnerable, gets two NextDouble draws (x, y: point +
    d x offsetMax - offsetMax x 0.5), kept when on the map. The Tutorial has 184 segments: 184
    synced draws at placement (before the recorder opens its file, so unlogged; the clone's
    draw 185 is the recording's first).
  - Process (@0x180d94c20) / Update (@0x180d95240): currentRadius starts at the model range, +=
    round frames x amountPerFrame (0.05) to circleRadius (360); while it is over
    currentGeneratedRadius by more than generateRadius (6), the kept positions in the next ring
    (over gen^2, under (gen + 6)^2 from the tower) each AddObjectAtPosition (@0x180d92f10: three
    NextDouble: object, rotation, scale), gen += 6. Rings every 120 frames (rt 180, 300, 420...).
  - DamageOverTimeZones (Far, Middle, Close, in that order): ranges currentGeneratedRadius, min(100,
    gen), min(50, gen) (GetRangeMiddle / Close @0x180d93c90 / @0x180d93c60). Zoned<Bloon> runs
    before the round's bloons move: a centre within range (off the map too) gets the mutator, a
    DamageOverTimeCustom (triggerImmediate, distributeToChildren; damage + additive on its tags);
    out of range, BloonExit takes it off. Mutators go in at Mutable.AddMutator's count / 2 slot,
    and pass to children through MOAB pops (rt 153 / 154).
  - Bloon.Process after a DamageOverTime degrades the bloon goes on down the (kept) behaviour list:
    a later DoT that is due ticks the same frame (rt 153: Middle pops the MOAB, Close hits the
    Ceramic it became).
  - Zoned.Run keeps inZone: BloonEnter only for a bloon newly in, BloonExit only for one it had in,
    so a copy born outside keeps the mutators it copied (rt 158). BehaviorCreated: a spawned bloon
    enters at once (rt 156 / 157).
  - A DoT continuing after a degrade goes in behaviour (seq) order, not mutator order (rt 157).
  - Within one Damage every copy copies the bloon first hit ("copy ... from 6701" for all four of a
    RainbowRegrow's): a copy made in the same Damage keeps its source's (already copied) order
    (rt 2933); across frames a copy's copies are copy_order'd again (rt 2935).
  - ActivateAttack.Activate (@0x180e01df0) with isOneShot / processOnActivate runs its weapons once
    at the activation; lifespanFrames 0, it then RemoveProcess: Vine Crush used with only a MOAB
    about never fires (the old 1 s grace stays for other one-shots).
  - druid_050 MATCHES.
  - SimRecorder now logs "BD id index lastTick model" for every DamageOverTime behaviour and lists
    Spirit mutators in the "S" line (ground truth for the rules below; druid_052 re-recorded with
    them, extra probe recordings in pending/druid_probe).
  - A zone's mutator id is mutatorId, or the zone model's name when that is "" (0-x-2 / 2-x-0's Middle:
    "DamageOverTimeZoneModel_MediumSpiritOfTheForest", not in BloonManager's keep list, so a MOAB pop
    strips it: druid_052 rt 142).
  - PostDegrade for every copy of one Damage uses the first-hit bloon's DoTs (its last one's
    lastTick), not an intermediate copy's (druid_250 rt 154). A DoT continuation skips behaviours made
    this frame (druid_052 rt 143).
  - Open: druid_052 rt 285, the Jungle Vine fires in the clone at a DoT-popped Blue (1543) the game
    never takes, though it later takes 1570 with the same history; 1543 stays in the path's
    bloonsList after it is destroyed at 309. The two Blues' state (position, BD, S) is identical
    at 299 and 318; rateFrames stays 142 and the thorn W0 keeps its 60. Not explained yet.
  - Lightning bounces search in process-list (creation) order, not the projectile factory list's
    (which a swap-remove reorders): probe druid_200 rt 2026 MATCHES with it.
  - SimRecorder probes: "LG proj hitTarget currentDelayFrames shared:.. collided:.." per lightning,
    "BL bloon layerId lineage". Probe recordings: pending/druid_probe (druid_200 matches; druid_300 /
    druid_300b with LG, 300b also BL).
  - The search rule from the decompile (BloonManager.GetTargets @0x180fcacc0 -> Targetting.GetTargets
    @0x180fdbe70): (1) Targetting.GetGlobalBest (@0x180fdb5f0): the path's cached best for the type,
    if on a fetched segment and passing the BloonFilter, goes first (AddTemporaryTarget); (2) the
    brackets of the path segments within range + 40 (constant 0x183e4ff98), sorted by the type's
    comparer, each bloon taken with NO distance check unless MOAB-class (CanHitMoab); (3) then the
    off-track (wind-carried) bloons (pathManager +0x69 / list at +0x120). Tried in the clone: range
    + 40 segments without the global best breaks druid_200 at 306; "range + 2" by position fixes
    300b rt 319 (Green 719 at 87.19 taken) but not probe 200 rt 2716 (Green 2163 at 87.9 left by
    2442, taken by 2454: likely 2442's own collidedWith through the lineage; check with BL).
  - PORTED: each fetched segment's bracket is flagged PathSegment.IsInRange (@0x180f6ced0: segment
    point within range + 2); a bloon of a flagged segment is taken whatever its own distance, a
    MOAB-class one of another through its circles; wind-carried (off-track) bloons come after the
    brackets, by position within range + 2. targetLast off (+0xa8) matches none of First / Last /
    Close in GetTargets, so the brackets stay in fetch order (entrance first). The global best is
    not ported (no cache entry for that type, INFERRED). druid_300 probes now to rt 497 / 651 / 725.
  - Next, druid_300 (probe b) rt 494: the thorn attack starts its throw (W0 anim 509) with the tower
    turned from 61.14 to -2.73 in the game, 33.6 in the clone, then turning about -3.8 a frame
    (clone about -3.0). The candidates are wind-carried Greens circling the tower 8-21 away (distance
    173.6). The RNG stays aligned to the end, so it is the thorn attack's target / turn against
    carried bloons (Targetting's off-track list for tower attacks?), not a draw.
    Solved the aim: the game's rotation is exactly atan2 from (tower + 10 / tan60) to Green 1056's
    previous-frame position (the clone's RotateToTarget formula), so only the target choice
    differs. The four carried Greens tie at distance 173.6 (the Lead is sharp-immune); the
    tie-break is likely their wind_seg / path rank.
  - FIXED: a tower attack's off-track pass takes the newest carried bloon by creation (id), not by
    reused factory slot (P-OFFTRACKORDER's obj): rt 494, the thorns take Green 1056. ninja stays
    34/34.
  - FIXED: the lightning bounce takes carried bloons after the brackets within range + their radius
    (rt 875: a Red 93.5 away at range 86), newest factory object first (obj; by id breaks rt 737).
  - REPLACED the id order: Targetting.GetTargets walks the off-track bloons through
    collisionChecker.BloonGrid's query (sim + 0x120 -> +0x60), so a tower attack's off-track pass now
    uses CollisionGrid query order from the tower (`grid_query_key`, the instant-hit sort): probe
    druid_300 rt 648 (oldest of four carried Pinks) and probe b rt 494 (newest of four Greens) both
    fit. The lightning's carried candidates keep factory-object order (grid order breaks probe b rt
    879).
  - A bloon whose wind ended this frame is in neither list for the lightning (Bloon.wind_end; probe b
    rt 3046).
  - Probe druid_300 MATCHES; probe b is bloon-exact, positions to rt 3489 (a thorn 0.07 off).
  - One RotateToTarget per attack, at the attack's Target: the first of the attack's weapons in its
    throw (counted from the frame its throw starts) turns the tower, and a later weapon releasing
    during that throw shoots from that facing at that weapon's target (druid_300 rt 724-726: the
    lightning lands on the thorns' Pink; druid_205 rt 172: the thorns keep their own target under
    the lightning's later throw). druid_300 MATCHES.
  - Lightning hits go one shot at a time in creation order (each its own instant pass): druid_320
    rt 2148, two lightning pops' copies listed in their shots' order.
  - TrackTarget's validity includes the projectile's FilterBloonIfDamageTypeModel (cant_hit):
    druid_320 rt 3060, the tornado (can't hit White) seeks a Blue past a nearer White.
  - druid_320 now to rt 3394. There a carried Yellow (5004, also 5087) popped to a Blue by a
    lightning split (5633, newer than the bloon, so hitting in its Process after the bloon moved)
    moves that frame at Blue speed in the game (0.73) and Yellow speed in the clone (1.66). The
    tornado 5369 also touches it that frame (H lines, no pierce change). Next: re-record 3-2-0 with
    the probes and log MoveWithWind state (offTrackCount, the wind's speed) per carried bloon to see
    whether the degrade re-attaches or re-runs the move.
    Tried (reverted, uncommitted): MoveWithWind has only its model (field_offsets), so the step is the
    bloon's model speed when it moves. (a) Moving a carried lightning Target after the hit (as a newer
    Target) fixes rt 3394 (bloons exact to 3570) but breaks rt 2838, where a carried Blue the bolt
    destroys was hit at its moved spot (the bolt sits at -85.578, the moved position). (b) Redoing
    the move for a carried Target the hit degrades: the clone's `prev` already equals the moved spot
    for carried bloons, so the redo must start from the frame's start (cell_pos is a candidate,
    untested). Both cases fit "move first, and a degrade by a later bolt redoes the step at the new
    speed", or a degrade that resets the position to the frame start; a probe recording should settle
    it.
  - druid_300 (probe b) was exact in positions to rt 3046 before these (weapons same). Still open: the pending
    druid_300 (rt 725), probe druid_300 (rt 651), 3-2-0 (497 bloon frames), 4xx / 5xx.
  - Open, druid_300 rt 319 (3-0-0's lightning with the tornado): the bounce search of bolt 817 at
    -149.07 takes wind-carried Blue 590 / Red 599 (distance 173.6, blown back since before 296);
    the clone's search skips carried bloons (`b.wind.is_none()`) and finds none. Allowing them
    breaks rt 315, where the game's search from -90.11 takes Lead 810 and Pink 801 and leaves 599
    (34 away, least far along) out. Neither the wind state nor the lineage differs between 315 and
    319: look at the segment GetTargets files a carried bloon on (wind_seg) against the search's
    segment range next.
  - BloonEnter adds the mutator even when the bloon has one with that id (a copy entering): a second
    entry at count / 2 that only shapes the mutator order (behaviours are one per id, the old one keeps
    its lastTick). druid_250 rt 154 (S lines [Far, Medium, Close, Far, Close]). druid_250 MATCHES.
- Lightning PORTED 2026-11-08 (druid_200 exact to rt 1680, weapons same). Game::step_native's
  lightning block after the bloons move:
  - It stays at the eject point when made and isn't Instant-followed.
  - Its hit is in a pass of its own after the move (children start where the bloon is, not
    `fresh`). A Target newer than the shot is put back to its start-of-frame spot for the hit,
    then moved as what it became.
  - It sits where it hit. delayFrames later it looks within splitRange (First / Last order,
    off-map too, past its group's sharedCollidedWith = all hit / picked layers of the shot and
    its splits): the first is its Target, further ones (up to `splits`) are split copies there,
    all hitting next frame; none, it goes. A shot that has hit nothing goes when its Target is
    gone.
  - The P-LIGHTNING retarget export is gone (`lightning_split`).
  - The bounce search order: targetLast off takes the TargetType static at +0xa8, which behaves
    as Last (INFERRED, rt 1679: the two least far along in range).
  - sharedCollidedWith is one list per family, appended by every bounce search's picks (not the
    first Target). A search excludes its own collidedWith plus that list; splits start with an
    empty collidedWith (rt 306 / 310 / 1683).
  druid_200 MATCHES.
- Lightning, from the decompile of Lightning.Process (@0x180cc84d0):
  - LightningModel: delay 0.05 s (delayFrames at +0x48), splitRange (+0x30), splits,
    targetLast (+0x4c: TargetType First or Last).
  - Process, while not hitTarget: currentDelayFrames -= 1, and nothing until it is <= 0. Then
    BloonManager.GetTargets from the projectile's position, splitRange, BloonFilter. The first
    target is the projectile's Target (its layerId into sharedCollidedWith); each further one
    gets a CreateSplit (also into sharedCollidedWith). A valid Target: currentDelayFrames =
    delayFrames, hitTarget = 1, projectile.SetTarget (vtable 0x7e8). None: the projectile goes
    (0x808).
  - With hitTarget: the projectile's Target (0x7d8); invalid, it goes.
  - Recording druid_200 rt 22-23: lightning 466 is made at the eject point at 22 (P line). It
    hits Black 419 on frame 23 after the Black has moved at Black speed (Yellows at 194.22 /
    185.62), so the damage is in its Process (after the older bloons move), not the collision
    pass. The clone hits in the collision pass, before the move (194.95 / 184.69).

## Buccaneer 34/34 (2026-11-08)

- The 34 seeded recordings are in research/dumps/recordings/pending/bucc (old unseeded ones in pending/bucc_old).
  All 34 match.
- Done: broadside. RotateToTarget turns the tower to the target's rotation + additionalRotation
  (-90) in game degrees; the eject point turns with the tower; EmissionRotationOffTowerDirection
  (@0xc3c3c0) sends the shot at the tower's rotation + offsetRotation.
- Open, the secondary gun (FireWhenAlternateWeaponIsReady + FilterTargetAngleFilter). From the
  decompile (FilterEmission @0x180c76350):
  - shareFilterTargets: the result is re-evaluated only when clock - timeOfLastTargetUpdate >
    minTimeBetweenFilterTargetsFrames (56), else the cached lastFireResult; it is shared by the
    tower's filters.
  - It walks Attack.GetTargets: a target passes if Vector3.AngleBetween(bloon - tower, Rotate(up,
    display rotation + baseTowerRotationOffset)) is strictly inside +-fieldOfView/2.
  - FireWhenAlternateWeaponIsReady: set_FireWeapon (@0xc77eb0) lasts until clock + 1; Process
    clears it after that; EmissionStarted clears it; its FilterEmission comes first in the list.
  - Unresolved, recording bucc_001: at rt 2170 the game holds the secondary off with a Yellow 70.4
    away at -43.6 flat (it passes in 2D); at rt 538 it fires with a Yellow 70.5 away at -44.5 flat
    (it fails in 3D with z 20). So it's neither plain 2D nor 3D with the bloon at 20. Check what
    Attack.GetTargets holds (which bloons and positions) and the display rotation (+0x70 display
    behaviour).
  - DONE 2026-11-07 (bucc 10/34): the filter goes over Attack.GetTargets. With no target
    supplier list that is GetVisibleTargets (the clone's visible_spot: track segments in range + 2),
    with a flat angle; 3D with the bloon's height loses recordings. The 56-frame cache didn't change
    any recording.
- DONE (040/050/052/250 match): MOAB Takedown is a real hook (Game::hooks_step, after the bloons
  move: LinearTravel out in 3D, kill at the snap, back at 2.5/frame, gone within 6), the round end
  waits for it, and it counts as the ability weapon's throw. Grape Shot's
  EmissionArcRotationOffTowerDirection: the arc is built round the tower and each shot turns by the
  offset after (read back from its direction). The angle filters share one result for 56 frames
  (shareFilterTargets).
- DONE: DamageModifierForCashAmount, one per stackId (the last whose threshold is met), applied
  after the tag modifiers (004 / 005 match).
- Open: 010 020 032 320 (the secondary gun's rotation, below), upgrades_r30, and the Aircraft
  Carrier family 400 420_* 500 502* 520. Recording bucc_400: the planes are sub-towers flying
  FighterPilotPattern* paths (towers 1 and 2 throw 72 / 85 times in the game, 184 / 180 in the
  clone, from rt 31); the ship's own darts (weapon 0) never fire in the clone. A port of its own.
  Findings (bucc_400):
  - The ship's dart attack fires from rt 427 at rotation 0. The carrier doesn't turn; in the clone
    the planes kill everything first.
  - Each plane is a sub-tower (W ids 249 / 440 / 766) with three weapons: rate frames 13, 72 and
    180. W's position stays at the spawn spot (-2.8, -1.04).
  - Weapon 1 (72) first fires at rt 3: an 8-way radial burst (rotations 180, -135 ... 135) from
    (-2.8, 8.958549) at z 257, each dart 1.667 a frame outward (P 300-349, rt 28).
  - The clone's planes throw 184 / 180 times vs 72 / 85: rates or weapons are wrong. Start from the
    BuccaneerPlane TowerModel's weapons and the FighterPilotPattern* paths (P-FIGHTER is a stand-in).
  - BuccaneerPlane (create_tower from the "Plane Spawner" attack, max 3): air speed 50, accel 105;
    air_modes exported empty, so the FighterPilotPattern{First,Close,Last,Strong} paths aren't
    exported. Attacks: twin darts (0.21 s, markers +-2.5, no target types: fired by a filter like
    FilterBloonsToWithinAngle; the game throws 72 times vs the clone's 184), Radial 8-way arc
    (1.2 s), Anti-MOAB missile (3 s, targets First...). Port the pattern movement and the dart
    filter first.
  - The plane's dart attack has FighterPilotPattern{First,Close,Last,Strong} (offsetDistance 25) as its
    target suppliers (Initialise @0x180db72b0, GetTarget @0x180db7180, GetTargets @0x180db7230,
    GetPursuitDestination @0x180db7030, SearchForTarget @0x180db73f0, IsBloonValidAndCamo).
    Its AirUnitModel has FighterMovementModel (maxSpeed 50, turningSpeed 0.035,
    minDistanceToTargetBeforeFlyover 40, distanceOfFlyover 75, bank 70 / 0.025, roll 2.3 s /
    run-up 100 / every >= 12 s / chance 0.02 a second, loop 4.6 s / run-up 100 / >= 20 s / 0.015 a
    second / radius 36.6). FighterMovement.Process @0x180d23c90 (it inlines ApplyMovement
    @0x180d22930): Quaternion.AngleAxis turns, Math.LerpAngle, Math.TruncateVector3, flyover state,
    and barrel-roll / loop checks (lastTimeCheckedForABarrelRoll: check whether those draw synced
    randoms; R lines in the recording). Init @0x180d233a0.
  - The radial attack has fireWithoutTarget; the missile attack targets with *AirUnit suppliers
    and an attack offset (-199.1, 743.1).
  - PORT SPEC (decompiled 2026-11-07):
    - FighterMovement.Process each frame:
      - timer += 1; outside a flyover, every updatePathTime frames (Init) take currPath =
        supplier.GetPath() = [GetPursuitDestination()]: SearchForTarget's attackTarget position,
        else the parent tower's position. It keeps a valid target for 15 frames, then searches
        again: BloonManager.GetTarget from the parent tower's position, infinite range, the
        pattern's mode (First / Close / Last / Strong), the attack's filter.
      - target = currPath[0], or flyoverTarget while flyoverEngaged; TruncateVector3(target, 3).
      - dir = target - pos.
      - newRot = Math.LerpAngle(transform rotation, get_Rotation(dir), turningSpeed 0.035):
        d = (b - a) - floor((b - a) / 360) x 360, clamped to [0, 360], minus 360 if over 180;
        a + clamp01(t) x d.
      - rotation = Rotate((0, 1), newRot); pos = Truncate3(pos + rotation x (maxSpeed x 1/60));
        transform rotation = get_Rotation(Truncate3(rotation)).
      - If Distance(old pos, target) <= 40 (minDistanceToTargetBeforeFlyover): when not in a
        flyover, start one with flyoverTarget = pos + Rotate((0, 1), rotation) x 75 x
        (_UNK_183e4fc3c); otherwise the flyover ends.
      - Barrel rolls are deterministic and display only.
    - TruncateVector3(v, n): (float)(Math.Truncate((double)(v x 10^n)) / 10^n) per component.
    - Darts: AttackFilter FilterTargetAngleModel (fov 90, offset 0, the plane's rotation) +
      FilterInvisible; target = the pattern's attackTarget when Attack.FilterTarget passes.
  - PORTED (2026-11-07):
    - Game::fighter_step / fighter_search; the plane doesn't move on the frame it is made; no
      on_map filter in the search (the MOAB entering counts).
    - The plane's shots fire along its rotation (FireFromAirUnit), no seek-launch.
    - Twin darts: shot k from marker k.
    - Accelerate comes after the move and grows speedFrames / turnRatePerFrame for tracking shots.
    - bucc_400 is exact to rt 147.
  - bucc_upgrades_r30 progress (2026-11-07):
    - P-NEWWEAPONS: weapons an upgrade adds to a kept Attack get their own place in the process
      list (Tower.weapon_born): they aim after the bloons have moved (the 1-1-0's Grape Shot at
      rt 547).
    - RotateToDefaultPositionTower.OnUpgraded (@0x180d86c70): reaching tier 4 / 5 turns the
      tower to 0 by RotateTower((0 - rot) x Deg2Rad), leaving 3.8146973e-06.
    - Aircraft Carrier (RotateToTarget rotateTower off, EmissionRotationOffDisplay): the parallel
      darts are laid out on the tower's rotation and sent at the display's rotation + 90; the angle
      filter reads the display's rotation (Tower.disp_rot).
    - Carrier display (RotateToTarget.ApplyRotation @0x180dd7930, rotateTower off): the attack's
      transform turns to tower + ((additionalRotation + target rotation) - tower). The shots go at
      display - offsetRotation (main -151.11, secondary 28.889 for a display at -61.11). The main
      darts are laid out on the tower's rotation; the angle filter's facing is display + base.
      bucc_upgrades_r30 is exact to rt 1364 (then a plane shot one float step off, like bucc_400 rt
      147).
    - Lead for the plane drift: the path point is the target bloon's position when the plane reads
      it. In the game, a bloon older than the plane has already moved by then and a newer one hasn't;
      the clone reads every bloon before the move (move_air runs before the bloons move, and must
      stay before the plane's attacks). TRIED (2026-11-07): running fighter_step in the towers phase at the plane's
      place after process_bloons_before(its earliest attack stamp), once a frame before its position
      is read. It is worse (bucc_400 rt 110, upgrades rt 1312), so not that.
    - SOLVED with the FM probe. SimRecorder now writes "FM tower x y rotX rotY targetX targetY timer
      flyover" for each FighterMovement every frame; bucc_400 was re-recorded with it.
      - The plane moves after the towers' attacks each frame (from the frame after it is made),
        before the bloons move.
      - Its path point reads a bloon newer than the plane where it was at the frame's start (prev
        if the towers phase already moved it); an older bloon where it is now.
      - The radial ArcEmission (no useAirUnitRotation) goes from 0, its eject point still turned
        with the plane.
      - Planes 1 and 2 now match the game's FM lines every frame.
    - DONE: SubTowerFilter.FilterEmission counts live createdProjectiles + createdSubTowers.
      (P-SPAWNPROJ, "counts for 90 frames", is gone: 2026-11-08 below.)
    - DONE: the plane tower's own Rotation stays 0 (W lines); only the air unit turns.
      - The eject point turns with the plane (FireFromAirUnit.EmitPosition).
      - The FighterPilotPattern darts leave along the plane's heading.
      - The missile and the radial leave from 0.
    - DONE: TrackTarget validity uses the projectile's tag filters (the missile lets go of a MOAB
      that became a Ceramic, rt 311).
    - Next, bucc_400:
      - DONE (default): the carrier's darts go at display + offset, laid out normally. bucc_400 is
        now exact to rt 452 (positions), while bucc_upgrades_r30 drops back to rt 1306 (its first shot
        after the upgrade went the other way; RRBTD6_CARRIER_FIRST=1 brings back the old rule:
        display - offset, laid out on the tower). Find what makes that first shot differ.
      - Was rt 427: the carrier's first dart pair. The game sends it at -21.56, laid out along -21.56,
        i.e. display + 90 for display = target - 90. In bucc_upgrades_r30 rt 1306 the same 4-x-0
        weapon went at display - 90, laid out on 0. The two disagree under one rule; maybe the
        display rotation's state matters (it rotates every frame, onlyRotateDuringThrow off, and the
        4-1-0's was new at the upgrade). Add a probe of the attack display's rotation (Attack's
        transform +0x84 / world) per frame, like the FM line.
      - DONE 2026-11-08, bucc_400 MATCHES (all 3600 frames):
        - SearchForTarget's BloonManager.GetTarget is GetTargets(count 1). For First / Last it
          takes the path's leading bloon (Path.bestFirst, furthest along) before the segment
          order. Plane 2 at rt 452 took the Pink furthest along of four on one segment.
        - The twin darts are made in marker order on every throw: the first at marker 0 (+2.5).
          The collision pass takes them in that order (rt 416 / 466).
        - Note: the H / M / X lines after a frame's dump belong to the next frame.
      - DONE 2026-11-08, bucc_500 MATCHES (re-recorded with FM lines, as were 420_* 502* 520
        upgrades_r30):
        - EmissionWithOffsets.rotateProjectileWithTower (Emit @0x180c3d170) is off on the 5-x-x
          planes: each dart goes along the flat line from the plane to its Target (exported as
          Emission.with_tower).
        - SubTowerFilter.createdProjectiles holds pooled Projectile objects. FilterEmission (only
          as the weapon is ready) drops those whose RootObject.isDestroyed is set. A flushed object
          taken by a newer projectile (Factory: the last flushed first) counts again while that one
          lives. The third plane comes 91 / 101 frames after the second (a radial dart's object).
          Game::proj_obj / proj_pool; a projectile the clone's projectile phase ended this frame
          still holds its object at the tower's turn (proj_objs_start).
      - DONE 2026-11-08 (bucc 27/34; 502 502_close 502_last 502_strong 520 420_last match):
        - The planes' FighterPilotPattern follows the carrier's targeting (the plane tower's own is
          Pursuit), and uses its attack's camo filter (the 5-0-2's planes see camo).
        - Close is GetClosestTarget: the nearest track segment from the carrier that holds a
          bloon, its first in list order (as TrackTarget's seek_pick).
        - ArcEmission.useAirUnitRotation (4-2-0 / 5-2-0 planes' grape arc): round the plane's
          rotation, not 0 (exported as Emission.air_rot).
      - DONE: track_targets runs after the bloons' DoT damage (an older bloon's DamageOverTime ticks
        before a newer dart's TrackTarget; 420_close rt 1750, a dart whose Green burned up flies
        straight). bucc 28/34.
      - DONE (bucc 29/34): Strong for the planes is Targetting.GetTargets' Path.FindBestBloon (per
        path in list order, highest danger, ties to the one strictly further along), on the
        distances as the frame began (Path.Process files them then; 420_strong rt 400: a White 0.5
        ahead of a Black listed first). The carrier's Secondary (no EmissionRotationOffDisplay) is
        laid out on the tower's rotation and sent at display - 90; the RRBTD6_CARRIER_FIRST flag is
        gone (upgrades_r30 now exact to rt 1423).
      - DONE 2026-11-08 (bucc 31/34; 010 020 match): FilterTargetAngleFilter's shared result is
        per filter: each looks again once more than its own minTimeBetweenFilterTargetsFrames after
        its timeOfLastTargetUpdate. A look hands its time and result to the filters made before it
        (filterTargetAngleFilterBehaviors), not later ones (WeaponState.fov_last). SimRecorder now
        writes "FA tower weapon displayRotation towerZ timeOfLastTargetUpdate lastFireResult" each
        frame, and "BZ id z" for a bloon whose Position.z isn't 0 (on Logs every bloon is at 20, the
        tower at 0; the Buccaneer's filter has no display, so it uses the tower's rotation).
      - DONE (bucc 33/34; 032 320 match):
        - Dot:Buccaneer stacks as many copies (isUnique, inserted mid-list); only the first copy
          with its id drives the bloon's DamageOverTime. When copies run out together, the first
          one's lastTick goes on to the next live copy. Before, the second expiring copy
          overwrote it (bucc_320 rt 831: the MOAB's next tick at 853, not 832).
        - HYPOTHESIS P-DOTCHILD: the new objects a DamageOverTime pop makes in Bloon.Process are
          `unplaced` (not in the CollisionGrid until they move), and so are the further layers of
          one made in the same pop (bucc_320 rt 1141: a burnt Black's second Pink, on to a Yellow,
          isn't hit until 1143).
      - DONE 2026-11-08, upgrades_r30 MATCHES (bucc 34/34; re-recorded with FA lines):
        - An upgraded tower's angle filters share their look both ways (UpdatedModel regathers
          every filter; Initialise only the ones made before it): `Tower.upgraded`.
        - The carrier plane's dart rotation is the air unit's as it is. The projectile-made
          sub-tower's b + (g - b) cost an ulp at 78.049355 (rt 1423).
        - Upgrading to a tier whose plane model differs (AircraftCarrierPlane ->
          CarrierFlagshipPlane) removes the old planes (STAND-IN: told by the air display).
        - FighterMovement behaviours are pooled: a gone plane's goes back at the next 60-frame
          flush, and the next plane made takes the last flushed. It keeps that one's timer (11:
          no search on its first move) and its path point is (0, 0): it turns from 0 toward the
          origin and flies over at once (rt 1862). A fresh one has timer 15 (`fm_pool`).
        - FireWhenAlternateWeaponIsReady's set_FireWeapon lasts until clock + 1: a secondary whose
          filter fails as its main fires goes the frame after if the shared look turns true (rt
          1892; `Tower.alt_at`).
  - Open, rt 147: plane 1's radial burst x -12.256329 vs -12.255328: the plane's x truncates to
    -5.741 vs the game's -5.740, a float-step difference in the turn or step math built up since
    rt 75. Check Vector2.Rotate / get_Rotation (atan2f) against the clone's game_dir / crt_atan2,
    and the bloon position the path point takes.
  - 010 rt 665: the dart secondary fails in the game with a Black 27 away at 35.5 degrees flat, using
    the post-turn rotation (62.35). It would fail with the pre-turn rotation (-196.75), or 3D with
    z >= 15.5. But 000 rt 932 needs the post-turn rotation, and 001 rt 538 needs z <= 9.3 in 3D. The
    filter reads the display's rotation (FilterTargetAngleFilter +0x70, FindDisplay): probe when the
    display node's rotation is updated, e.g. a SimRecorder line with the display rotation vs
    Tower.Rotation at each secondary check.
- Was: 040/050/052/250 (Monkey Pirates' MOAB Takedown, scenario "ability 120"): the clone's
  P-TAKEDOWN destroys the MOAB at once; the game hooks it. Recording bucc_040: the hook projectile
  (LinearTravel, speed 2000, forceCollisionOnSnap) leaves at rt 121 (~26/frame), meets the MOAB at
  ~127-129, then the MOAB is dragged toward the ship at 150 (2.5/frame; TravelTowardsEmitTower,
  delayedActivation, lifespan 6.67) until within destroyBloonRadius 6 (MoabTakedown: x2 worth).
  Methods: LinearTravel.Process @0x180ccb620; MoabTakedown Collide @0x180cccc50 / Process
  @0x180ccd6b0 / DestroyIfReachedTarget @0x180ccd450; TravelTowardsEmitTower.Process @0x180cf6e10.
  The ability's attack: GrappleEmission with EmissionRotationOffTowerDirection(90),
  RotateToTarget(-90), weapon eject (0, 2, 3.5), TargetGrapplable hooks 1.
  LinearTravel.Process (decompiled): while age < lifespanFrames and the Target is alive:
  - d = Target.Position - Position (Vector3, so the MOAB's z 120 counts).
  - If speed^2 < |d.xy|^2 - (model +0x2a float x the same): Position += Normalized(d) x speed
    (model +0x38), 3D, so the flat step is under speed (recording: ~26/frame flat for 33.3).
  - Else: Position = Target.Position; with forceCollisionOnSnap and not yet forced,
    Projectile.CollideBloon(target, 1) and forcedCollision = true.
  MoabTakedown.Collide (@0xcccc50), first contact only (startPullingBloonTowardsTower): the bloon must
  carry tag 0x2000. Bloon.Damage with a huge amount (the bloon is destroyed at contact, worth through
  bloonWorthMutator x2). A display effect (bloonEffect) and the rope follow the projectile.
  Spawner.BlockRoundEnding until it's done. MoabTakedown.Process / TravelTowardsEmitTower (150) then
  bring the projectile back to the ship; DestroyIfReachedTarget ends it (destroyBloonRadius 6).
  To port: make the hook a real projectile (its P positions are compared): its first frame one
  LinearTravel step from the eject point (rt 121 at (-24.44,-13.78) for a ship at (-2.8,3)), the kill
  at the snap, then 2.5/frame back to the tower. Replace P-TAKEDOWN's instant kill.
