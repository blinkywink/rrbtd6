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

Last full run (2026-11-06): dart 70/70, boomerang 99/99, wizard 35/35, ninja 34/34, bomb 34/34,
glue 34/34, ice 25/25, sniper 34/34, tack 25/25, sub 32/32, beast 34/34, engineer 34/34, desperado 34/34
(+ pending/_desperado_g 5/5), and 0 for ace, alch, bucc, dartling, druid, farm, heli, mermonkey, mortar,
spike, super, village. Next tower: Alchemist (throws match; potion hits differ from rt 132).
Run `sh tools/rec/compare.sh --root dart boomerang` and the sets you touched before each
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
| Desperado | 34/34 | EXACT (2026-11-06): an `unplaced` bloon (spawned, or made by an instant / line shot's pop, not moved yet) isn't met by grid collisions at all (040 rt 678). Earlier notes:  (2026-11-04: TheBlazingSun found through SimRecorder Q lines, 13 -> 19) re-recorded with seeds 2026-11-04. BloonDistanceRateBonus uses Attack.GetVisibleTarget Close, StandoffModel ported, Instant shots from throw markers start on the eject point. Open: 002 rt 1614, 003, 004, 005, 020, 025, 03x, 04x, 05x, 205, 250, 300, 320, 420_x, 50x, 520, upgrades_r30 |; 2026-11-05 (first diffs moved, count unchanged): plain LineProjectileEmission ported (Emit @0x180c42910: 400 long, CollisionGrid.GetInLine samples at thickness, 3x, ... (MoveNext @0x18281ef30, address found with SimRecorder's getinline_movenext probe), nearest first, collided at once; the projectile sits at the start); instant / line hits now land as each attack fires (instant_pass before every attack: desperado_050 rt 73, pistol mark and rifle pop before the mark attack aims); FilterIfAttackHasTarget (@0x180c1ec30) on Attack.hadValidTarget (FindTarget's result when the weapon is ready). Next: BurstWeaponBehavior.Process with alwaysEmptyClip (@0x180c6c250, the shotgun's clip; desperado_005 rt 73-100: lastEmission 18 then 100). DONE 2026-11-05: fireBetweenRounds weapons run on Simulation.time (sim_elapsed, not steps); Vagrant counts in the weapon's own attack range (Q lines: pistol 60 counts a MOAB 42 away, shotgun 36 doesn't); BurstWeaponBehavior.OnRoundEnd per weapon slot (two weapons named "Weapon"). Next: 005 rt 78 a pellet's y step is 5.036886 in the game vs 5.0368857 (direction rounding), bloons differ from rt 90; pistol held too long (80 vs 12 throws). alwaysEmptyClip DONE (2026-11-05): a ready weapon with a part-used clip (from the frame after it became ready) aims at the point its attack range ahead along the tower's rotation (005 2360 -> 24 bloon frames, 205 bloons exact). NEXT 205 rt 139: those pellets start 2.05 off along the facing (eject point for an override target?). 250 (2026-11-06): bloons + weapons exact: a plain line collides inside Emit (before CalcRateFrames), and AttackMinimumRange.FilterTarget (@0x180dac2f0) applies in the visible-target queries of BloonDistanceRateBonus / Standoff. 005h MATCHES (2026-11-06, all 5 fresh runs match): VigilanteMutator.Mutate (@0x18124f180) also adds loseLifeRangeBuff / speedFrames x 1/60 to the first TravelStrait lifespan of each attack without fixedRange (shotgun pellets 15 frames under the buff, rt 3038). 032 MATCHES (2026-11-06): Zoned<Bloon>.BehaviorCreated: a bloon the round spawns goes into each Vagrant bloon zone it is in at once (rt 1001: a Black spawned 47.9 away counts for the rifle's shot; INFERRED not pops' children, desperado_003 rt 2634); this also covers 052 rt 2598 (replaces the buff-end zone refresh). 025 MATCHES (2026-11-06, fresh 025g too): EmitOnDamage's fragments are made before Damage.Collide's crit roll and carry the shot's crit roll as it is then (unrolled: each rolls at its own first damaging hit; rolled: never again). 004 MATCHES (2026-11-06): a bloon already knocked back takes Knockback again (a new mutator, full lifespan; INFERRED: Knockback.Collide's IsMutatedBy check isn't its own mutator's id): rt 261, a pellet a frame after the first two keeps the MOAB going back 12 frames. 052 MATCHES (2026-11-06): a mutator ending (Vigilante buff) comes after that frame's Zoned<Bloon>.Process, still at the old range: the rate is worked out from the zone refreshed with this frame's bloons at the zone's last range (rt 2598: 6 Zebras within 96, one new this frame, 117 frames). 003 MATCHES (2026-11-06): BurstWeaponBehavior.Process (@0x180c6c250) stores the alwaysEmptyClip point (Weapon+0xdc) as the weapon's frame ends, with the tower's rotation then; the next frame's shot uses that stored point (rt 952: the pistol turning the tower to a Lead first doesn't move the shotgun's point). (040 / 005h: one-off position frames.) EMITONDAMAGE + VIGILANTE WEAPONS (2026-11-06, 005 MATCHES, 003 bloons exact, fresh 005h 6 position frames): EmitOnDamage.Collide (@0xcbb8c0) asks Bloon.WillDamageBloon, its fragments are made before the hit's pops (005h rt 1338: pellet Shrapnel ids before the Desert Phantom explosion of the Yellow it popped); the Vigilante rate buff finds each weapon by (name, occurrence) (two weapons are named "Weapon": the shotgun took the pistol's rate, 005h rt 3002). 040: the fresh run (pending/_desperado_g/desperado_040g) matches; the stored one's rt 678 is a rare child-in-grid case. VAGRANT RANGE DONE (2026-11-06): VigilanteMutator.Mutate (@0x126e830) adds loseLifeRangeBuff to the tower and to every attack without AttackModel.fixedRange (+0x66); the 0-3-x attack is fixed at 40, the 0-0-2 one isn't (fresh 032g now matches, 002 still does). 030 / 050 / 250 MATCH: a Line emission's start adds the turned eject offset first, then the height (P-LINEEJECT). Open: 003 004 005 025 032 040 052. EMIT FROM TARGET (2026-11-06): the 030-040 rifle line starts on the Target, emitFromTargetLength (36) long, the shot itself at the barrel: 030 / 040 bloons + weapons exact (an instant shot is limited to its Target only with FilterAllExceptTarget (@0x180c1d3c0, ObjectId equality): 040's mark shot hits whatever it meets. Left: 040 rt 678 that mark shot meets child Ceramic 586 a frame later in the game (586 made at 677 by the MOAB's pop); 030 rt 673 1 ulp), 032 / 052 improved. 050 (2026-11-06): bloons exact to the end once the rifle's line shot can't collide after its sweep (Emit ends with canCollideWithBloons = false); 6 position frames left: rt 2938 a rifle shot's start 1 ulp off in y. MARKED TO POP PORTED (2026-11-06; 050 bloons exact to rt 1602, weapons same): Game::mtp_step + the activation's own check (no marking target -> execution at once, firing the next frame), FilterMutatedTarget inverse (need_effects), FilterMarkedToPop lineages; Instant destroyIfInvalid shots go at the frame's end once their target is gone. Was: MARKED TO POP (next, 050/052/250 rt 120): port MarkedToPop (Activate @0x180e3b6d0, Process @0x180e3c0b0): phase 1 marking attack (others paused) while its weapon is ready, it has a valid target and now - activatedAt < markingTimeFrames; else phase 2: marking paused, execution unpaused, executionStartedAt; FilterMarkedToPop lineages cleared. Models (Desperado-050 'Marked To Pop2'): marking attack (W3, rate 0.2) filters FilterMutatedTarget DesperadoMark (skip marked) + FilterMarkedToPop; its instant shot (destroyIfInvalid) adds DesperadoMark (layers 1, endless, kept on degrade) and, through MarkedToPopProjectileBehavior.SetTarget (@0x180ccc970), the target's lineage to its attack's FilterMarkedToPop (FilterBloon @0x180c1fe30 rejects any bloon with a lineage id in it). Execution attack (W4, rate 0.05): FilterMutatedTarget inverse (marked only) + FilterMarkedToPop; 300 damage instant. In 050 the marking attack has no target at 120 (the mark attack marked them), so execution fires at once (W4 at 120). DONE: an Instant shot with no lifespan lives till its target / pierce goes (050 rt 105 mark shot). 205 MATCHES (2026-11-06): the empty-clip override point starts from the tower raised 10 in display space (BurstWeaponBehavior.Process: +10, or ejectZ with RotateToTarget.useThrowMarkerHeight), so the tower doesn't turn for that shell. 005: the stored recording's rt 90 explosion split doesn't happen in fresh runs (pending/_desperado_g, 005g matches); 005h (fresh, HB radius lines) differs from rt ~? -> next. rt 92, bloons rt 173). 205 (2026-11-05): HYPOTHESIS P-DESTROYEMIT, a child made and popped through by the same hit emits no EmitOnDestroy (Desert Phantom explosions only where the Ceramics went; survivors' later ones as before); first position diff now rt 139, bloons rt 1260. DONE: RandomEmission's speed multiplier scales speedFrames (speed x 1/60) after it is computed (pellet steps exact). VAGRANT ZONE DONE (2026-11-05, 002 MATCHES): each Vagrant weapon keeps Zoned<Bloon>.inZone (IsInZone @0x18239f320, Process @0x18239f600): after the tower's attacks, the bloons within its attack range (2D, no margin); a shot counts that list, destroyed bloons included. upgrades_r30 MATCHES (2026-11-05): a new tower model (place / upgrade) recalculates rateFrames with BloonDistanceRateBonus / Standoff on the bloons then. 300 MATCHES (2026-11-05): bloons made anywhere in this step (also the towers' instant pops) skip this frame's Grow tick. 420_close MATCHES (2026-11-05): an instant shot on a bloon is pre-filtered by the bloon's grid cell (cell_pos) too, as its hit test (SimRecorder HS lines: the hit is CollisionChecker.Process -> CollideBloons, the shot still on its eject point) |
| Beast Handler | 34/34 | Exact. Re-recorded with seeds 2026-11-04. GyrfalconPattern ported from the disasm (fly_birds, GyrState): targets through GetGlobalBest (bestFirst/bestLast) then segment brackets of the handler's target type, snatch/hit around the target by Close bracket order, grapple uses the initial damage, held bloons are off track (Bloon.held: not filed, taken by range + radius) with blockSpawnChildrenCount (unmake_bloon), idle moves through Map bounds. Pets start a frame after they're made and sync the handler's target. CycleAnimation randomize draws. InstantDamage shot with a dead target stays till round end. |
| Engineer | 34/34 | EXACT (2026-11-05): Pin re-stun slot rule and duplicate non-cascading slows (SimRecorder S / M lines). |
| Ninja | 34/34 | done (all R-line recordings) |
| Alchemist | 0/34 | throws match; potion hits differ from rt 132 |
| Ace | 0/25 | recorder doesn't log the plane yet |
| Buccaneer 0/33, Dartling 0/23, Mortar 0/24, Druid 0/34, Banana Farm 0/25 | | Farm: EmissionsPerRoundFilter spacing (see TOWERS.md) |

TOWERS.md has the longer per-tower notes.

## Now (top = next)

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
