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

Last full run (2026-11-02, every set): dart 70/70, boomerang 99/99, wizard 34/34, ninja 34/34, bomb 33/34,
glue 34/34, ice 25/25, sniper 19/34, tack 21/25, sub 21/32, beast 9/34, engineer 9/34, desperado 6/34, and 0
for ace, alch, bucc, dartling, druid, farm, heli, mermonkey, mortar, spike, super, village.
Run `sh tools/rec/compare.sh --root dart boomerang` and the sets you touched before each
checkpoint that changes `renderer/src/game.rs`.

## Status (recordings matching; update as they change)

| Tower | Match | Notes |
|---|---|---|
| Dart | 70/70 | done |
| Boomerang | 99/99 | done (paragon too) |
| Tack | 21/25 | re-recorded with seeds 2026-11-03; open: 500/502 (meteor 1 ulp at rt 522), 520, upgrades_r30 |
| Sub | 21/32 | re-recorded with seeds 2026-11-03; see Sub notes (Tack section) |
| Wizard | 34/34 | all re-recorded with R lines; done |
| Glue | 34/34 | done (mutator order, PostDegrade) |
| Ice | 25/25 | done (re-recorded with seeds 2026-11-02) |
| Sniper | 19/34 | see Sniper notes |
| Bomb | 33/34 | open: 032 (9 position frames) |
| Desperado | 6/34 | |
| Beast Handler | 9/34 | |
| Engineer | 9/34 | re-recorded with seeds 2026-11-03; RandomTargetSpread and ArriveAtTarget altSpeed exact |
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
0. **Sub 21/32 (re-recorded with seeds 2026-11-03)**: Airburst's split angles are k x step + (rotation +
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
