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

Last full run (2026-11-07): dart 70/70, boomerang 99/99, wizard 35/35, ninja 34/34, bomb 34/34,
glue 34/34, ice 25/25, sniper 34/34, tack 25/25, sub 32/32, beast 34/34, engineer 34/34, desperado 34/34
(+ pending/_desperado_g 5/5), and 0 for ace, alch, bucc, dartling, druid, farm, heli, mermonkey, mortar,
spike, super, village. Alchemist 34/34. Druid 34/34 (2026-11-09). Mermonkey 34/34, Banana Farm 25/25, Monkey Village 25/25, Spike Factory 24/25, Super Monkey 28/34 (2026-11-10; heli / ace / super re-recorded, old ones in pending/<set>_old). Full run 2026-11-10: every set above at full count (mortar 25/25, bucc 34/34). Monkey Ace 25/25 (2026-11-10): add `ace` to the regression run.
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
| Super Monkey | 32/34 | DONE (2026-11-10): the Robo hands split (TargetRightHand <GetTargets>d__9.MoveNext @0x180df3eb0: A for the right hand's type, B for TargetLeftHand's (ctor @0x180de7b70: strong), m = Normalize((A - T) + (B - T)) + T, A stays right when (A.y - T.y)(m.x - T.x) - (m.y - T.y)(A.x - T.x) >= 0, else swapped; both must be valid or neither fires; `left_hand`), and an arm turned with rotateTower off stores its rotation relative to the tower and ejects through the tower's child transform (super_030 rt 973): 050 / 052 now match. FOUND (2026-11-11, not fixed yet): super_032's 5 position frames (rt 352-354 dart 2662, rt 352 / 364-365 dart 2953) are the RandomArcEmission angle 1 ulp low: game rotation 25.528013, clone 25.528011 (recording P/D lines; with the game's angle, D = Vector2.Rotate((0,1)) and v = D.x x 350/60 = 2.5138888 reproduce every game x bit for bit, the clone's 2.5138886 drifts only from rt 352). TravelStrait.Process is pos + velocity x elapsed (same as the clone). GetDirection (@0x180c4a000) sums (((r - angle/2) + offset) + k x slice + (float)NextDouble x randomAngle) - randomAngle x 0.5 in the clone's order, so suspect r (read from the arm entity's transform, +0x60 -> +0x10, i.e. the arm's stored rotation after RotateToMiddleOfTargets / RotateToTarget; dart made rt 319) or the draw: print r / rarc / sum in emit_native's RandomArc arm for rt 319. Back-solved from the recording's draws (rt 318 block: arm A Emit 0.8926 / 0.8378, dir 0.20800033; arm B Emit 0.02195 / 0.50166, dir 0.67234113): the game's arm rotation is exactly 27.86401 (arm A) or 24.149284 (arm B), the only f32 that sums to 25.528013; the clone's r is so 1 ulp low, most likely in the body + (arm - body) composition from 8c12257. Suspect: the clone composes with ONE body rotation (tr0 + (g - tr0), at both the throw-start store ~14323 and the release store ~14620), but the game stores g - body when the arm turns and adds the body back when GetDirection reads the transform; if RotateToMiddleOfTargets moves the body in between (store B1, read B2), the game's r is B2 + (g - B1). Test: print tr0 at the store and game_rot at the emission for rt 319 and see whether B2 + (g - B1) gives 27.86401 / 24.149284. Earlier: 030 21 / 032 7 position frames (rt 973: still 1 ulp in x after GetEjectPosition's single Vector2.Rotate by the arm's stored rotation, VERIFIED-DECOMP @0x180c4f390; next suspect the add order onto the tower position or the first move), upgrades_r30 rt 1572. SUN TEMPLE (2026-11-10): a tower made at a temple tier now runs temple_upgraded (OnTowerCreated -> ActOnSacrifice), mute reset to the harness's frame-adjusted weaponDelay (super_400 bloon frames 1691 -> 1440). Still: the game fires ~25 weapons (3..24) and a second entity ('tower 1', 124 throws from rt 226) with NOTHING sacrificed; the clone none. Not the CheckTempleUnderLevel mutators: Evaluate (@0x180c1ce10) = templeId matches && MonkeyTemple.ContainsTowerSet(set) && GetTowerSetWorth(set) < cost, false with no sacrifice. FOUND (2026-11-10): a temple MADE at its tier presets its worth (MonkeyTemple.OnTowerCreated / OnInstaPlacement @0x180d405c0: templeTowers = {1, 2, 4: 40001 (@0x183e501d4), 8: 0}; tier 5 trueTempleTowers {1, 2, 4, 8: 40001}; SetTempleSacrificeValuesFromModel @0x180d42d60 applies a group when its set is selected and worth >= (int)cost): `temple_created` (super_400 bloon frames 1440 -> 822, weapon 0 131 vs 129 throws). DONE (2026-11-10, later): the temple's mutators are applied one by one in the game's order. MonkeyTemple inserts each applied group's mutators (model order) into mutatorsToApply with a sorted insert (@0x181ffcdb0, recursive binary search @0x18014ca90 by SortMutators: priority descending, an equal priority goes in AT the probed midpoint), then OnTowerCreated mutates the model in that order (MutateFromCache). Simulated, this gives exactly the probe's weapon order (missiles v2, v1, plane, storms, arcane v3/v2/v1, missiles v3, glaives v3, blade, glaives v2/v1). Consequences now in temple_tier: the priority-20 stat mutators touch only the temple's own attacks (the sacrifice weapons fire at their raw rates: 30/48/78/120/300); Range (1) runs last over all. Mutate bodies VERIFIED-DECOMP: Reload @0x180c225f0 rate - rate*m; Speed @0x180c22830 speed*m + speed (compounds: 1.12*1.24*1.36); Size @0x180c21e70 radius*m + radius (assetSizeModifier on scale); Pierce @0x180c22180 + (int); Range @0x180c22a70 tower and attack range + v. Stat mutators carry their own CheckTempleUnderLevel too, so the exporter writes every mutator (TempleGroup.muts: priority, kind, value, attack index, condition, supports) and the upgrade path drops each failed one. The missiles never firing is just AttackMinimumRange 100 + MOAB-only filter (the MOAB sat ~40 from the tower). super_400 first position diff rt 225 (many projectiles) -> 226 (the plane's first shot only). DONE (2026-11-10, Ace exact): the PlaneSpawner sub-tower flies the Ace PathMovement port (its CirclePattern isn't selectable; a plane whose target type names no pattern flies its first). UseParentEject (EmitPosition @0x180c978e0): a weapon ejects from the tower's first Attack's first Weapon (the Arcane Blasts from the main blast, turned by its stored rotation, z 21.58). A SingleEmmisionTowardsTarget shot isn't seek-launched (its offset stands: Arcane Blasts +-40 / 80 / 120). FollowPath + TravelStrait (glaives): each frame's path point gets one TravelStrait step on top, TravelStrait's lifespan holds; FollowPath advances speed x (1/60) (1 ulp; boomerang still 99/99). Probe super_400: every weapon's throws now match. DONE: a carried parent's children are all made at its z (the blown MOAB's four Ceramics at 120, so they drift back in 3D at the game's speed); not for an uncarried parent (druid_500 rt 138). DONE: MoveWithWind.Attatched (VERIFIED-DECOMP @0x180ffec70): a distanceMax over 99999 is the bloon's distanceTraveled (the blown MOAB 54.31 back; super_400 0 bloon frames). DONE: AlternateProjectile.GetEmission (VERIFIED-DECOMP @0x180c69170): an alternate without its own emissionModel uses the weapon's (the plane's bombs go at their target). DONE: super_400 MATCHES. A towards-target shot whose target died in the throw aims at the Target's cached spot (emit_aim; a glaive along -67.23). TravelStraitModel.set_Lifespan (VERIFIED-DECOMP @0x181202b90): lifespanFrames = (int)(lifespan x 60 + 0.5), half up (a 0.875 s plane shot moves 53 frames); AgeModel's stays half to even. DONE: RetargetOnContact.Retarget (VERIFIED-DECOMP @0x180cd8170) on a FollowPath shot sets its Direction and Rotation; FollowPath.ProcessFrame lays the path from the start point turned by the rotation, so the glaive is re-laid along the new heading the frame it bounces (the game retargets before the frame's path move). NEXT: super_420_close rt 231: glaive 1031 (pierce 49) hits 11 of 12 Zebras in the game, skipping 1101 (same lineage 1106,1049,747 as 1092, which it hits) and hitting 1111..1148 twice at pierce 39; the clone pops all 12: collision candidate order / grid cells. Detail (2026-11-12): all Ceramics and their Zebra children sit at the entrance clamp (-73.98, dist 173.6); the hits on Zebras do nothing (0), so nothing degrades after the Ceramics; the visit order [1043, 1053, 1063, 1083, 1092, 1111..1148, 1111..1148 again, 1157] skipping 1101 (child k = 3 of 1043, spawned 20 behind) fits candidates gathered cell by cell from the BloonGrid, a bloon filed in two queried cells visited twice and 1101 filed (by its spawn spot, before the entrance hold) in a cell the glaive doesn't query. RESOLVED 2026-11-12: SimRecorder can now replay a game's seed (scenario "seed": SeedPatch sets GameModel.randomSeed before Simulation.Initialise @0x180b48de0); super_420_close / last / strong re-recorded with their own seeds all match (the old ones, object ids 9 apart, in pending/super_old/*_unseeded.rec), so that run's difference wasn't reproducible. super_030 / 032 / upgrades_r30 re-recorded with their seeds reproduce their differences exactly: real. DONE: RotateToMiddleOfTargets.ApplyRotation (@0x180dd5530) sets the tower's rotation to Rotation(sum of Normalized(Target.Position - tower) over the hands' attacks with a target); the arms store theirs relative to it (super_030 exact, super_032 rt 301). LEFT super_032 rt 352 / 364 (ulps): the game applies it only while a weapon is in a throw (model +0x28), with each hand's current Target, and before the arms compose (352: arm 0 with the frame's final 6.6377); the clone takes the last stored aims as each attack sets its aim (363: 50.63 vs the game's 50.11). DONE (5xx): AttackMinimumRange applies to carried (off-track) targets too (no missiles at a MOAB blown back 71 from the temple); SubTowerFilter's createdProjectiles is per weapon (the True Sun God's second plane spawner isn't held by the first's projectile). super_500 re-recorded with WN / PM lines as pending/super_probe/super_500.rec: weapons all match. DONE: RangeSupport's lifespan step lands on the attack's first TravelStrait descendant: with an AlternateProjectile on the first weapon, the alternate's (the True planes' bombs 100 -> 120 frames, darts keep 53). super_500 / 520 match. DONE: a carried bloon moves at its speedFrames without Knockback's slowing (super_502 rt 289: knocked at 287, carried at the full 0.5208; a freeze still holds it, druid_400); super_502_* match. NEXT: super_upgrades_r30 rt 1551: the 4-1-0's main blast (RandomArc, randomAngle 8) leaves along 70.65 in the game, 73.07 in the clone. The game's angle is exactly the clone's next-frame collision-pass draw (wind roll, 0.07544 from T = 74.0457): in the game the two wind rolls of projectile 4262's hits on Black 4158 / White 4166 (main blasts carry WindModel chance 0, a roll per valid hit) come BEFORE the tower's rt-1551 emission, in the clone a step later. Positions at frame 1550 are 22.4 apart (radius 9), at 1551 14.2: find why the game collides there before the throw (weapon animation 1: a deferred release?). Then super_420_* rt 231 (glaive 1031 skips Zebra 1101), super_030 / 032 (1 ulp), upgrades_r30 rt 1572. Then super_420_* rt 231, super_5xx (main weapon 246 vs 155 throws). Earlier: (2026-11-10, later) CritMultiplier ported (Initialise / GetProjectile @0x180c71590 / 0x180c71010: nextCritAt = syncedRandom.Next(lower, upper + 1) when the weapon is made, its ability attacks' weapons right after the tower's own (`temp_crit`), and after each crit). DistributeToChildrenBloonModifier (Collide @0x180cb9360): the hit bloon's own layer passes the overkill on (Tech Terror's blast through a MOAB's Ceramics). A Sun Temple made at its tier holds fire weaponDelay frames from Simulation.time 0 (MonkeyTemple.CanFire @0x180d3f010; the harness offsets by the recording's first frame). OPEN (NEXT): Robo / Tech Terror / Anti-Bloon hands: TargetRightHand's <GetTargets>d__9.MoveNext (@0x180df3eb0) takes its own type's target A and the left hand's type's target B, m = Normalize((A - T) + (B - T)) + T, and if (A.y - T.y)(m.x - T.x) - (m.y - T.y)(A.x - T.x) >= 0 the right hand gets A and the left B, else swapped (super_050 rt 118: left -46.54, right -64.5; the clone gives both the same target); super_040 rt 189 the right arm's eject; 030 21 position frames; 050 2 bloon frames; 4xx / 5xx: the temple's other weapons (3, 4) never fire in the clone. Earlier: | Re-recorded 2026-11-10. RandomArcEmission ported (Emit @0x180c49d20: two synced draws, an offset the start doesn't use; GetDirection @0x180c4a000: (rotation - angle / 2) + offset + k x slice + NextDouble x randomAngle - randomAngle / 2). Darkshift (StartDarkshift / Process / EndDarkshift @0x180e22bd0 / 0x180e225e0): the tower moves towerSpawnDelay frames after the activation, after that frame's attacks; a point-less Activate (the recorder's) goes to (0, 0). Knockback isn't applied by the hit that pops a MOAB-class bloon (INFERRED; super_004 rt 128). RetargetOnContact behind a TravelStrait in the behaviour list re-aims after the frame's move (super_004 rt 139). rotateTower-off attacks of a tower with RotateToMiddleOfTargets eject along their own aim (INFERRED scope). NEXT super_040 rt 74: the Tech Terror arms aim differently in the clone (game: both at -68.67, the shared target); then 030/032 (Robo), 050/052, 205/250, 300/320, 400-520 (Sun Avatar+), upgrades_r30. |
| Spike Factory | 25/25 | Re-recorded 2026-11-10 (old in pending/spike_old). checkCollisionInterval's timer runs only while canCollideWithBloons (Projectile.HasValidBloonCollision @0x180cabcf0); a CreateProjectileOnExhaustPierce with no gap turns canCollide back on every frame, so Super Mines piles collide in flight from their second frame. RandomRotationWeaponBehavior: one synced draw per shot at ProjectilesCreated, once for a weapon's missed-frame shots. One TrackTargetSupplier per TargetTrack attack (`Tower.track_sets`); ability attacks' lists are made after the tower's own first attack and kept between activations. StartOfRoundRateBuff on/off remakes the points (a round start only refreshes it). ActivateAbilityAfterInterval.Process (@0x180ce2b60): round time, fires when more than intervalFrames since the last try (first at once), Activate(true) (no cooldown; IsReady refuses while active, Ability.get_IsActive); its attacks act that frame and count their lifespan from the next. AgeRandom: Next(min, max) frames per shot. Ability weapons keep missedRate between activations (`temp_missed`, reset at round start). OPEN 052: the first storm fires on the placement frame (rt 1) where 050's fires at 2; 052 also has StartOfRoundRateBuff, whose mutator goes on in that frame. Both towers' weapon 0 fires at 1, so the tower does process at 1: ActivateAbilityAfterInterval.Process (@0x180ce2b60) only gates on Ability.CanUseAbility (@0x180df8db0: a count at f42080 == 0 and every behaviour of interface 0x184f6ffe8 on the tower's entity allowing it, vtable +0x8e8) and the interval, so in 050 one of those blocks at rt 1 and in 052 the model update clears it. The behaviours are the ability's IAbilityBehaviors (AbilityBehavior.CanUseAbility, slot 123): for Spike Storm ActivateAttack.CanUseAbility (@0x180e029b0): refused while active (slot 124); with rounds not active, only with model +0x71 and a weapon check. So at 050's rt 1 the round is likely not active yet as the tower processes; in 052 the round-start mutator's model update (StartOfRoundRateBuff.OnRoundStartSim) must let it through: check the round-start event order against Tower.UpdatedModel next. PORTED 2026-11-12: cancelIfNoTargets wants a tower attack with fireWithoutTarget or Attack.CanHaveTarget (@0x180dcb260: its activeTargetSupplier, set up at its first Process or a model update); with StartOfRoundRateBuff going on in the placement frame the storm goes off ahead of the tower's attack: Spike Storm's four lists made at Activate, its shots, the tower's list and shot, then the four lists again with the cursor carried (spike_052 exact to rt 129). Then: StartOfRoundRateBuff going on at each round start remakes the ability attacks' lists (cursor carried) before the tower acts (rt 129), coming off remakes only the tower's own (rt 308); the early storm's spikes start ageing next frame. spike_052 EXACT. |
| Monkey Village | 25/25 | EXACT (2026-11-10, re-recorded; old ones in pending/village_old). RangeSupport.MutatorTower (@0x180d795b0) also adds the model's TravelStrait lifespan x multiplier (or additive / speedFrames / 60) to each attack's first TravelStrait (both kinds add; ability range zones use the plain Mutator @0x180d79db0, range only: `Support.range_life`). Call to Arms (CallToArms.Activate @0x180e19600, CallToArmsMutator @0x181274ad0): global unless useRadius, never the village, mutates at once (no next-frame delay, `Support.instant`), rate / multiplier and pierce x multiplier on every projectile (`Buff.pierce_x`). Grow Blocker zones test the bloon's frame-start position. Monkeyopolis: cratesPerRound and (max(worth / valueRequired, 1) x cashFromCrate + baseIncome) / cratesPerRound per crate; STAND-IN: farms in range aren't absorbed yet (needs a recording with farms). RotateToTarget.ApplyRotation (@0x180dd7930): useThrowMarkerHeight aims from the tower raised by the weapons' highest ejectZ; rotateTower off turns only the attack (`Tower.ent_rots`), the shot ejecting along it, re-aimed at the release. SingleEmmisionTowardsTarget.GetDirection (@0x180c4d0f0): normalized(Target - eject point). A tower made at a tier takes its model's first TargetTypes entry (Tower.UpdateTargetType @0x180c19880; the 5-x-x ballista lists Last first). |
| Heli | 25/26 (all but 050) | Re-recorded 2026-11-11 with SimRecorder HM (heliTransformBehavior) and IN (the tower owner's InputManager cursor, active, ignore) lines; the harness compares HM positions as kind 2 and feeds IN as `Game.cursor`. PORTED (VERIFIED-DECOMP/DISASM): FollowTouchSetting.Process (@0x180db8c70: currentPosition = cursor while active, not ignored and IsPointOnMap; starts at the heli's spot), HeliMovement.Process destination switch (@0x180d2c950), ApplyMovement (@0x180d29c30), GetPointWithinMap (+-150 / +-116), UpdateRotation (@0x180d2d460: no turn within strafeDistance or while a RotateToTargetAirUnit applied; LerpAngle by rotationSpeed toward atan2(dir.x, dir.y)), RotateToTargetAirUnit.ApplyRotation (@0x180dd6e00: snaps to Rotation(target - heli) before the heli moves; recording heli_000 rt 14 -84.26471). Order: the heli's attacks, then its movement (move_helis after towers_phase). A Python f32 check of the movement matches heli_000-003/030/032 bit for bit over 3600 frames. FireFromAirUnit for helis: shots from the heli (z 250 once it has moved) along its rotation, the EmissionWithOffsets markers turned with it (recording heli_000 rt 14). Also: the heli transform's z is 250 from placement (heli_032/500); RotateToTargetAirUnit aims at Target.Position (a MOAB's targeted circle, heli_032) and on cooldown frames the attack keeps its target, the heli still turning to where it is now (heli_400 rt 388, heli_500 rt 152); TargetFirst/Last/StrongAirUnit go through BloonManager.GetTarget -> Targetting.GetTargets (MOAB centre, heli_400); AngleToMapCenter (@0x180c69c90) for the Support Chinook crates, synced Range(0, 1000) each (INFERRED P-CRATEROT: from rotation 0). MoabShoveZone.Process (@0x180d3bb00) ported: every 30 roundTime frames BloonManager.GetTarget First from the heli with IsBloonValid (@0x180d3b7b0: MOAB-class, filters, not another zone's, dot(heading, bloon - heli) <= 0, PercThroughMap >= 0.01) gets MoabShoveMutator (@0x18121ff50: speed x moab/bfb/zomg cap by baseId) for 30 frames of movement (heli_004 rt 99-128). Comanche Defence (not immediate) PORTED, heli_004 exact: decided in the tower's turn (bloon positions as the frame began), helis made after the towers act; Process gets elapsed (1), so they never retreat and only OnRoundStarted clears the flags; the minis stay locked where made, facing Rotation(-spot). Comanche Commander PORTED (heli_005/025/205 exact): its minis are made at the tower in the tower's turn (before its HeliMovement; they act and move from the next frame) in FollowTouch from there; HeliGroupOffset (CalculateAvailableOffsets @0x18102dbf0: (0,0), then rings of 15 r) gives every heli of the target type its slot in tower order; heliPositionsInRepulseRange (INFERRED P-HELIREPULSE, fitted exactly to heli_005 rt 2-40 by brute force over the lists): each heli, after an ApplyMovement, lists the others now within range (enter order kept), in use from its next move. RotateToTargetAirUnit turns first to the attack's held Target (alive, any range), then to a new one its search finds (heli_025 rt 2357). PursuitSetting.Attatched sets the target type Pursuit (an upgrade into 2-x-x switches). NEXT heli_upgrades_r30 rt 615: the 2-1-0 (Pursuit, weapons new: W 0/1 lastEmission int.MinValue) shoots a Lead 48 from the heli (attack range 42) a frame before the clone: in Pursuit the attack's target supplier is the PursuitSetting (PathSupplierHeli.GetTargets @0x180dcd9c0 -> Targets(attack.range) -> BloonManager.GetTargets from the heli, the static TargetType at +0x18); FOUND 2026-11-12: its turn -147.27158 = Rotation(the Lead after its rt 615 move - the heli after its 615 move), so the weapons act after the bloons AND the heli have moved; an upgrade that changes a kept Attack's weapon count rebuilds all its Weapons (recording: at rt 200 the 1-0-0 resets w0 = 59 to int.MinValue too), new objects at the end of the process list. DONE: the w0 reset is ResetRateOnInitialise (VERIFIED-DECOMP @0x180c90a80, ported); a heli's own HeliMovement now runs before its young weapons act. DONE too: a heli's young weapon (ready) moves the heli, then the bloons, then does its own FindTarget (fresh, First from the heli) and turns the heli to it (rt 615, 649, 1193); a heli's Attack made after it (3-1-0 Rotor Blades) acts after its HeliMovement (rt 901). The young weapon's own search is final (none: no shot, rt 1638), and the weapons are listed with the attacks in the order they were made (the 4-1-0's missiles after the Rotor Blades): heli_upgrades_r30 EXACT. Wind-carried copies: spaced k x min(26.7 / count, 8.6) behind the blown parent over the final child list (a multi-layer pop flat), then each its own MoveWithWind draw (heli_040 / heli_250 exact). NEXT for Heli: heli_050 rt 238 (checked 2026-11-12: Target.IsValid @0x180c09350 keeps a degraded bloon valid (same object, same id), and taking the degraded / new bloons off the segment lists changed nothing, so the clone and the game likely hold different bloons since an earlier throw, stacked at one spot until 238): the Marine (made at 123, so before these bloons) fires after this frame's collisions (Black 1481 degraded to Yellow, children 1894/1904/1914 made) and takes a bloon at -131.437, not 1481 (first in Path.bloonsList, at -132.17); all are held at 173.6 on one segment. Likely the segment lists' own order (a degraded bloon re-filed at the end of its PathSegment list) - check with a SimRecorder dump of the segment lists. heli_040/250: Downdraft wind children; heli_050 rt 238: Marine target at round end. Earlier notes, from the decomp in research/dumps/decomp/ComancheDefence: Process (@0x180d0bdf0: not immediate: activated25/50/75 by GetGlobalBest First's PercThroughMap >= 0.25/0.5/0.75 -> GetMeDaChoppas(1/2/3, its position), activationTime = roundTime; after durationFrames every heli Retreat; HasRetreated (DistanceSquared to its startingPosition < const @0x183e50040) destroys it; cooldownFrames after activation clears all three flags; OnRoundStarted clears them; immediate: keep reinforcementCount helis made at the tower), GetPositionOnEdgeOfMapClosestToo (@0x180d0b600: nearest of the four world edges, pushed 50 past it), CreateDefenceHeli (@0x180d0a950: TowerManager.CreateTower, target type, LockInPlace at its start, PatrolPoints default path, heli rotation = Rotation(start - ...)), Retreat (LockInPlace target type), CancelRetreat (Pursuit). PursuitSetting PORTED (recording heli_200_pursuit exact; Path.DistanceToPoint is the path's own line, 4 above where bloons are drawn on Tutorial): GetPath = lastDestination = FinalTweak(GetPursuitDestination(GetPursuitTarget())) (FinalTweak is identity), path [lastDestination]; GetPursuitTarget (@0x180dd1c40) = PathSupplierHeli.Targets(range 99999) (@0x180dcda20: BloonManager.GetTargets(range, the static First type, the heli's position, count 999999999, the attack's BloonFilter, cantTargetMoab/Camo)).FirstOrDefault; GetPursuitDestination (@0x180dd1730): no valid Target (or destroyed) -> lastDestination; else HeliGroupOffset.Update, heli lastGroupOffset = GetAvailableOffset, off = Bloon.radius (+0x150, from BloonModel.radius) x 0.5 + pursuitDistance (model +0x34), d = distanceTraveled + off; 0 <= d <= path.totalPathLength (@0x180f6f920): p = Path.DistanceToPoint(d) else p = bloon position + Bloon.Heading (@0x180fb8b10: (0,1) Vector2.Rotate by the path point's rotation) x off; IsPointOnMap(p) -> p, else Map.WrapLineAroundMapEdge(bloon position, p) (@0x180f61540: Math.FindIntersectionLineLine with each world edge, then WrapLineAroundMapEdgeIntersect @0x180f613f0 carries the rest of the length along the edge toward the nearer angle) -> it, else lastDestination. OnSwitchedTo sets lastDestination = the AirUnit's position. HeliMovement's Pursuit branch adds lastGroupOffset and GetPointWithinMap. Then heli_040 rt 2310 / heli_250 rt 2413 (wind children: Downdraft), upgrades_r30. heli_050/052 (Special Poperations): the Marine is made by the Spawner attack's CreateTowerModel positionAtTarget at the UsePresetTarget point (UsePresetTarget.SetTarget @0x180e0c390: the player's pick); recording heli_050 has it at (-92.96, -28.49), 20 from the heli, the clone at the tower: the harness takes it from the first tower made within 10 frames after the abilities; CreateTowerModel.useProjectileRotation off keeps the Marine's own rotation. heli_050 left: rt 238, the Marine's last shot of the round at a Yellow just made in a bunched group (target pick). OPEN: Pursuit (STAND-IN P-HELIPURSUIT), PatrolPoints, HeliGroupOffset (Comanche groups), repulsion list membership (INFERRED P-HELIREPULSE: HeliEnter/HeliExit are delegates), canMoveOutsideScreen / ignoreRepulsion flags, the heli step inside each tower's processing rather than after all towers. |

TOWERLESS RECORDINGS (found 2026-11-07, owner saw it on screen): the game starts with 650 and
CreateTowerAt refuses a tower it can't pay for, silently (log: `placed ... False`), so nearly all old
pending dartling (23/23), ace, heli, spike (24/25 each) and super (32/33) recordings have no tower (no
W lines). All their scenarios now have "cash": 10000000 (gen_scenarios.py gives every scenario that);
re-record them before use. Dartling's targetTypes ids are "Normal" (pointer) and "Locked" (selected
point), not "Point": scenarios use Locked. Normal (follow the mouse) can't be recorded by hand; plan:
have SimRecorder set the input manager's cursor from the scenario each frame, so it's scripted.

TOWERS.md has the longer per-tower notes.

## Now (top = next)

### Owner-reported (2026-11-10, must be done exactly; not forgotten)
Owner's rule: nothing half done. Each fix must look and play exactly like the game (verify against
the game code / recordings), or it hides problems later.
- Monkey Village: the fire animation disappears, then reappears (visual).
- Alchemist: buffs it gives other towers don't show their icon over those towers' heads.
- Monkey Village 3xx / 4xx / 5xx buffs (and MIB) show a stack count (2x) with only one village;
  these buffs don't stack in the game.
- Monkey Village middle-path ability: no buff icons or animations shown, and it may not actually
  raise attack speed as the game does.
- Monkey Village bottom path: the 5th tier must only be buyable with a Banana Farm in range; its cost
  depends on the farms in range, and it absorbs them. Implement exactly. All of the bottom path's
  extra money effects must work exactly.
- Banana Farm: picking up bananas should play a sound.
- Engineer: the ability does nothing.
- Sandbox: clicks on its menus still reach the menu behind (the bloon menu overlays the monkey
  purchase menu); nothing behind it should be clickable anywhere (the top area already blocks).
- Beast Handler: on upgrade, the beast sub-tower (e.g. the T-Rex) jumps around; it should stay where it
  was and upgrade in place. Each beast's target location must be settable exactly as in the game.
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
  - P-CELLCIRCLE: a whole grid cell counts for a big query only if its circumscribed circle is
    inside the query circle.
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
