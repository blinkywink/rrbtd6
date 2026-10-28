# PROGRESS

Last updated: 2026-10-22 (session 5)

## Summary

- **Runtime:** BTD6 57.2.12199 (Epic) runs on Unity 6000.3.23f1 with IL2CPP x64, metadata v39.
- **Native analysis:** working, with self-written tools plus the `dumpbin` that ships with Visual Studio. There's a metadata v39 parser, a method→address map (121,788 methods), annotated disassembly and cross-references (`tools/il2cpp/`).
- **Game data:** fully decoded. The `sim_model` decoder belongs to the forked session. The round sets, game modes and income sets are its exports too.
- **Verified from machine code and implemented in Rust:**
  - the 60 Hz frame clock and fast-forward
  - the per-tick order
  - all three RNGs
  - the path position table
  - bloon movement and leaks
  - leak damage
  - group → emission expansion, emission timing, and round-set unit conversion
- **Validation so far:** reference-sequence tests for the RNGs. Leak damage reproduces the known values (MOAB 616, BFB 3164, ZOMG 16656, DDT 816, BAD 55760). Round 1 data agrees across two independent sources.
- **Not yet done:** no mechanic has been compared against a **live game trace**. Every row is verified-disasm or verified-data, not verified-obs.

## Sessions working in this folder

| Session | Owns |
|---|---|
| This one | `engine/`, `specs/`, `research/0*.md`, `PROGRESS.md`, `tools/il2cpp/{metadata_v39,codegen,calls,disasm,methods_of,xrefs}.py`, `tools/data_export/` |
| Forked session | `tools/sim_model/`, `tools/il2cpp/{setter_schema,field_offsets}.py`, `tools/assets/`, `renderer/` (playable private prototype: `cargo run -p btd6-game --release`, see `renderer/README.md`) |

## Completed

### Session 1
Environment, metadata dump, `sim_model` located, Rust workspace, first benchmark. See `research/00`–`03`.

### Session 2
- [x] **Native analysis toolchain**, std-only Python plus VS `dumpbin`:
  - global-metadata v39 parser
  - code-gen module locator (pointer count matches the metadata exactly)
  - address map, disassembly with call and vtable annotation, cross-references
- [x] **Game loop:** `InGame.UpdateSimulation`, `TimeManager`, `Simulation.Simulate`. Also the `GameModelLoader` stream layout, which was shared with the forked session's decoder.
- [x] **RNG:**
  - `SeededRandom` (Park–Miller)
  - `NinjaKiwi.Common.Random` (byte-identical to .NET Framework `System.Random`)
  - `BufferedRandom`
  - All implemented in `engine/src/rng.rs`, with reference tests.
- [x] **Paths:** `CalcPointInfo` (the 0.01-step table, lerp form, recomputed distances), `DistanceToPointInfo(Bloon)`, `DistanceToPath`, `IsEndOfMap`, `Vector3` primitives, `CapAngle`. Implemented in `engine/src/path.rs` and `math.rs`.
- [x] **Bloons:** `Process`, `TotalSpeedFrames`, `UpdatePositionAlongTrack`, `Move`, `SetPath`, `Leaked`, `set_Speed`, `GetTotalLeakDamage` (with ties-to-even rounding).
- [x] **Rounds:** `GetEmissions`, `AddEmissions`, `Spawner.Process`/`Emit`, `RoundModel.get_emissions`, and the seconds→frames conversion in `RoundData.GetRoundDef`.
- [x] **Engine rewrite** on verified behaviour:
  - creation-order bloon storage
  - exact due-frame scheduling, which matches the original's per-frame scan without rescanning
  - `data.rs` loaders for the decoded bloons and round sets
  - 23 engine tests passing
- [x] **Benchmarks:** exact path-table movement runs at 9.6 ns per bloon-frame at 10k bloons, with 0 allocations per tick (`benchmarks/README.md`). This is still not a comparison with the original.
- [x] **Docs:** `research/04-native-analysis.md` and an updated `specs/compatibility.md`.

### Session 3 (Cursor, continuing both Claude sessions after they hit the usage limit)
- [x] Prototype window: high-DPI and 4× MSAA (the "blurry monkey"), with the 3D viewport scaled to physical pixels.
- [x] Auto-start toggle (A key or panel button).
- [x] Per-tier Dart Monkey models in the renderer (52 exported prefabs, loaded on first use), with generic attack and idle clip lookup.
- [x] Per-tier projectile displays: dart, arrow and triple-dart sprites, plus the Spike-o-pult and Juggernaut meshes. The forked session's export used the wrong bundle (`asset_library`, not `asset_references`).
- [x] Placement: transcribed `BlockedByBlockerZone`, `IsPointOnMap` with its constants, and the base-area-first order. The renderer's `can_place` now follows `Map.CanPlace`. The exporter now writes area height and blocker circles.
- [x] The game's range colours (`Assets/Monkeys/Materials/Range.png` lookup) and its hand cursor.
- [x] New tools: `build_dart_display_plan.py` (since generalized to `build_display_plan.py`; replaces the forked session's ad-hoc scripts; reproduces its plan exactly), `export_projectiles.py` and `export_textures.py`. All are wired into `tools/build_local_assets.sh`.
- [x] In-game UI rebuilt from the game's own prefabs instead of a hand-made layout:
  - New tools: `dump_ui_prefab.py` (RectTransform trees with Image, TMP and layout components, plus TMP material properties), `export_font.py` (TMP SDF atlases and glyph tables) and `print_ui_tree.py`. `export_sprites.py` now exports every atlas with its 9-slice borders.
  - New renderer modules: `uitree.rs` (Unity anchoring, layout groups and image modes), `ui.rs` (a TMP SDF shader port and the sprite library) and `hud.rs` (state bindings).
  - Towers now count their own pops, which the upgrade menu shows.
  - Full shop (hero plus 26 towers, from `ShopTowerDetailsModel`), the Medium end round (60), RectMask2D clipping and scrolling, ContentSizeFitter, the locked upgrade style, and "$1,400" money formatting.
- [x] Fixed square cut-outs around bloons: sprite billboards no longer write depth, and range circles are drawn before the bloons.
- [x] Added the `cargo game` build profile (no LTO, incremental): about 2 s rebuilds instead of about 14 s.
- [x] Fixed doubled-looking darts. Weapons now use `WeaponModel.animationOffset`, so the throw animation starts 0.1 s before the shot, and `ejectX/Y` to set the launch point. HYPOTHESIS P-EJECT covers the axes (x right, y forward).
- [x] Shop icons keep their aspect ratio. The shop scrolls by wheel or drag, with `ScrollRect` inertia.
- [x] Menus from the game's UI scenes (`src/menus.rs`):
  - The gear button opens the pause screen. Restart, Continue, Auto Start and Game Hints work; the rest are inert.
  - The Upgrades button opens the upgrade tree: every path, the paragon, the portraits, the cost, the description popup and tower switching.
  - New tools: `export_text.py` (English.xml), `export_bundle_sprites.py` (sprites outside the atlases), and `dump_ui_prefab.py` with `scene:` and `field:` modes.
- [x] UI polish:
  - Atlas sprites are padded back to their full `m_Rect`, which fixes the oversized targeting arrows.
  - The TSM "i" button opens the game's `InfoPanel` (pops and the next upgrade on each path, with its description).
  - The Perks button is hidden on the pause screen.
  - Menu backdrops get the `VignetteUI` darkening.
- [x] Monkey shading from the disassembled DXBC shaders (`d3dcompiler_47` `D3DDisassemble` via ctypes):
  - `GrayscaleShader` and `LitMonkey` are unlit.
  - `SimpleUnlitOutline` pushes vertices out by 0.7 × `_Outline` in the view plane and is stencil-masked to the silhouette (URP renderer "Tower Outlines" RenderObjects feature).
  - Model textures are mipmapped.
- [x] Second playable tower, the Tack Shooter, with a tower-generic pipeline:
  - `build_display_plan.py` replaces `build_dart_display_plan.py`, and `export_tower_tiers.py` replaces `export_dart_tiers.py`.
  - `export_projectiles.py` now keys projectile displays by their `DisplayModel` GUID.
  - `export_animated.py` handles prefabs whose Animator has no controller.
  - `build_game_data.py` exports a `PLAYABLE` list of towers. Each attack gets its rotate flag and targeting modes, each projectile its display and `AgeModel` lifespan, and the ring towers their linked radius.
  - New rules: P-NOROTATE, P-RING, and one tier 5 per path per tower type.
- [x] Third playable tower, the Bomb Shooter:
  - On-contact explosions, frags and cluster bombs (P-CONTACT, P-RANDRANGE), with explosion and fire-ring burst stand-ins.
  - Tower order now follows the shop, so 1 / 2 / 3 select the Dart Monkey, Bomb Shooter and Tack Shooter.
- [x] HUD fixes:
  - The title font is now the English variant, Luckiest Guy. `localization_assets_all` holds one `Btd6FontTitleSDF` per script, and the export had picked the Bloons TD one.
  - When the upgrade menu opens on the left, lives and cash slide out of its way, using the decoded `StatsSlideInFromLeft` clip.
  - New autotest options: `RRBTD6_PLACE` and `RRBTD6_SELECT`.
- [x] Fourth playable tower, the Sniper Monkey:
  - Instant hits, map-wide range, Shrapnel Shot (`EmitOnDamageModel`) and Bouncing Bullet (`RetargetOnContactModel`). New rules: P-INSTANT, P-BOUNCE and P-OFFSCREEN (no targeting off the map edge, the `Map.IsPointOnMap` test that `FilterOutOffscreen` uses).
  - `export_animated.py` now follows Animator controllers in other bundles. When an Animator has no controller, it takes the clips from the animation-states MonoBehaviour next to it (Sniper 000 and Elite Sniper, plus six Tack Shooter tiers that had been static).
- [x] Drag-to-place: pull a tower sideways out of the shop and release it on the map.
- [x] Fifth playable tower, the Boomerang Monkey:
  - Follow-path boomerangs (`FollowPathModel`, P-PATH), periodic re-hits (`ClearHitBloonsModel`), travelling bounces for glaives (P-BOUNCE-TRAVEL).
  - MOAB Press: tag-filtered attacks (`FilterWithTagModel` / `FilterOutTagModel`), `ArriveAtTargetModel` (P-ARRIVE) and `PushBackModel` (P-PUSH).
  - Glaive Lord: fire-without-target attacks, and the `OrbitModel` glaives drawn around the tower.
  - Projectiles spin at their `RotateModel` rate. Tower order (shop order) is now Dart, Boomerang, Bomb, Tack, Sniper, on keys 1-5.
  - Things to check by hand are listed in `TO_TEST.md`.
- [x] Tower look: baked-mesh outlines from vertex colours (`outline.bin`), the `RadialShadow` ground shadow, a white outline on the selected tower, and the Sniper's 20-unit range circle.
- [x] Bloon effects: slow, freeze (Frozen property 16), damage over time, brittle, glue levels, per-tag modifiers, inheritance through `layers`, glue/frozen/tag filters, and Arctic Wind's slow zone (P-SLOW, P-SLOWMOD).
- [x] Sixth and seventh playable towers: Ice Monkey and Glue Gunner, with their projectile prefabs (`GlueYellow/Green/Pink`, `FreezeBomb`, `ImpaleIcicle`, from the `asset_library` bundle). Keys 1-7 follow shop order: Dart, Boomerang, Bomb, Tack, Ice, Glue, Sniper.
- [x] Eighth playable tower, the Ninja Monkey: camo priority, seeking (P-SEEK), wind blow-back (P-WIND), camo removal, track-targeted caltrops (P-TRACKTARGET, arrive-and-stay), alternate projectiles (P-ALT, Flash Bomb), state damage bonuses, MOAB-only sticky bombs with delayed splash (`ProjectileOverTimeModel`).
- [x] Wizard Monkey and Super Monkey: closest-track targeting for Wall of Fire (`TargetTrackOrDefaultModel`), instant projectiles at a point, Shimmer's damage-free camo removal, and all their projectile art. Keys 1-9 and 0 now cover the ten towers in shop order.
- [x] Spike Factory, Banana Farm and Monkey Village (shop only; no number key):
  - Spikes: round-time lifespans that survive `AgeModel.rounds` round ends (P-ROUNDS), the start-of-round production burst (P-ROUNDRATE), pierce-indexed pile sprites (`SetSpriteFromPierceModel`, mapped by list in `build_display_plan.py`), and bloon-type damage bonuses (`DamageModifierForBloonTypeModel`).
  - Bananas: random drop points (P-RANDPOS), a per-round shot quota (`EmissionsPerRoundFilterModel`), hover-to-collect cash with a "+$" popup, instant cash for Marketplace, and the Monkey Bank (P-BANK; C collects).
  - Village: a generic support-buff system (range, rate, pierce, projectile speed, camo, damage type, pop cash, discounts; P-SUPPORT, P-DISCOUNT). It also gives Shinobi Tactics its stacking Ninja buff.
  - Particle-only projectiles (Wizard bolts, fireballs) export the first cell of their particle texture as a stand-in sprite.
- [x] Druid: random-spread thorns, chaining lightning (P-LIGHTNING), Ball Lightning interval shots (P-INTERVAL), tornadoes that strip glue / ice (P-REMOVEMUT) and map-wide Jungle Vines (P-ATTARGET) with pierce-refreshing roots. All 64 tiers, models, icons and projectile art. Display GUIDs were matched to prefabs by which crosspaths use them.
  - `export_projectiles.py` takes optional names to re-export, and retries a failed mesh export after freeing its loaded bundles (this fixed the tornado, Shuriken and InfernoRingMeteor decompress failures).
- [x] Alchemist: acid splashes and pools (P-ACIDPOOL, track fallback targeting), Acidic Mixture / Berserker / Permanent brews on nearby towers (P-BREW, P-BREWTARGET), Lead to Gold cash (P-LEADGOLD). All 64 tiers, models, icons and potion art. Tier models are now limited to `<Tower>` / `<Tower>-xyz` (the Transformed Monkey models share the Alchemist's baseId). Sprite exports apply the SpriteRenderer colour, and upgrade icons fall back to the name without its first word.
- [x] Desperado: burst fire (P-BURST), the rifle's minimum range and line shot (P-LINE), the shotgun, `TargetDesperado*` targeting. Its tier-coded upgrade icons are exported with a tower prefix, and the icon lookup falls back to tower + tier code. It sits before the Sniper in the shop, so the number keys shifted (7 Desperado ... 0 Super).
- [x] Sound and music: `catalog_audio.py` maps Addressables GUIDs to clip names (checked against every named `SoundModel`) and, given `prefab`, to prefab names; `export_audio.py` decodes the clips game.json uses plus a music playlist to WAV; `audio.rs` plays tower, weapon, projectile, bloon and pop sounds with the game's pop-group limits (P-SOUNDGROUP).
- [x] MOAB-class 3D models with damage states (`export_bloon_models.py`), drawn at height 120 along the track. The upgrade panel says "Max Upgrades" (not "Path Closed") for a started path that crosspathing caps.
- [x] Mortar Monkey and Dartling Gunner: cursor ("Normal") and set-point ("Locked") targeting with the game's Set Target buttons and marker (P-POINTER, P-TARGETSPREAD, P-POINTDEFAULT), turn rates, shells that explode through every on-exhaust projectile, Buckshot (P-SLOWDOWN), Rocket Storm blasts per hit (P-EXHAUSTPIERCE), map-crossing Ray of Doom. `export_tower_assets.py` exports a new tower in one command; display plans fall back to the catalog's GUID-to-prefab map.
- [x] Engineer Monkey: sentries as sub-towers (`CreateTowerModel` / `CreateTypedTowerModel` on the throw projectile, `TowerExpireModel` lifespan, pops credited to the Engineer, stand plus turret models in `models/subtowers/`; P-RANDLAND, P-TYPEDTOWER, P-SUBBLOCK), Cleansing Foam's set point with `startWithClosestTrackPoint`, Bloon Trap thrown to the nearest track spot that eats bloons up to its RBE capacity and is collected for cash (P-EAT, `LimitProjectileModel`). Fixes found on the way: `catalog_prefabs.json` is one entry off for GUIDs that `catalog_guids.json` has (the latter wins now); arrive-and-expire projectiles ignored `filterCollisionWhileMoving`; `export_projectiles.py` now holds one bundle at a time (it ran out of memory) and only exports what's missing unless given names.
- [x] Heli Pilot: the pad as the tower and the helicopter as a steered air unit (`AirUnitModel` + `HeliMovementModel`, P-HELI) that all attacks fire from; Follow Touch / Lock In Place / Pursuit modes; per-tier heli models in `models/air/`; tower displays also fall back to `catalog_prefabs.json` for GUIDs `catalog_guids.json` lacks.
- [x] Monkey Ace: the plane as a path-following air unit (`PathMovementModel`) flying Circle / Figure Eight / Infinite / Centered Path patterns (P-ACEPATH), `AttackAirUnitModel` attacks read like `AttackModel`s, runway and per-tier plane models.
- [x] Abilities (first pass): ability buttons, cooldowns, `ActivateAttackModel` attacks and `TurboModel` (P-ABILITYCD, P-ABILITYSTART); `AgeRandomModel` lifespans.
- [x] Abilities (second pass): timed support zones (Call to Arms, Absolute Zero), IMF Loan with debt repayment.
- Next: the remaining towers. Sub and Buccaneer need moving air units and water (Monkey Meadow has no water, so Sub and Buccaneer can't be placed there in the game either); Mermonkey, Skywarden and Beast Handler each have their own systems. Sub-towers (Phoenix, Sun Temple) and abilities are shared work across many towers.

### Session 4 (Claude, forked session, 2026-10-21)

- Quincy (hero XP and levels), working pause-screen volume sliders and jukebox, window resizing that keeps the map visible, a faster sprite batch for bloons (about 40 fps at 16k bloons), and a portable copy (`tools/package_desktop.sh` builds `Desktop/BTD6-rrbtd6.zip`). Baked animations are quantized (`compact_anims.py`), so the data is about 1.7 GB instead of 14 GB.
- Beast Handler. Pets spawn from BeastHandlerLeashModel. The dinosaurs fight with their own attacks. The birds use GyrfalconPatternModel: they dive, grab up to grabAmount (MOAB-class bloons cost extra), deal initial plus grapple damage and hold bloons still (STAND-IN P-BIRD). Pet models are exported per tier.
- Monkey Fan Club (MonkeyFanClubModel). The fans use the ability's towerDisplay model (HYPOTHESIS P-FANCLUB covers who counts as nearby).
- Skywarden:
  - ToggleFocusStanceModel, with weapons matched by name (HYPOTHESIS P-STANCE) and the G key.
  - DamageBasedAttackSpeedModel (HYPOTHESIS P-RAGE).
  - CreateDistanceProjectileOnExhaustFractionModel, so the arcs grow with the distance flown.
  - Not done: its status effects.
- Upgrade names in the tower panel now come from the localization file. Non-ASCII icon names and mixed-case portrait names are normalized at export.
- Renderer unit tests (`cargo test -p btd6-game`) cover the fan club, birds and Skywarden.

### Session 5 (Claude, 2026-10-22)

- Logs map (first map with water) wired into `build_local_assets.sh`; N switches maps.
- Water towers' own mechanics: Buccaneer broadsides and paired guns (P-BROADSIDE, which also fixes the second gun firing every 0.05 s), Merchantman / Favored Trades sell bonus / Trade Empire / Flagship, Sub shared range and Submerge mode, Mermonkey water range, time-scaled damage and Alluring Melody's trance.
- Data fixes: `FilterWithTagModel.inclusive` means exclude; `LinkProjectileRadiusToTowerRangeModel` only resizes its own projectile.
- Aircraft Carrier planes: sub-towers with `FighterMovementModel` flight (STAND-IN P-FIGHTER), capped by `SubTowerFilterModel`, removed with their ship.
- Performance (see `benchmarks/README.md`): sim step at ~2.5k bloons with 19 tier-5 towers went from 0.94 ms avg / 13.5 ms worst to 0.40 / 1.6 ms with the same results; a projectile broad-phase grid; batched pop sprites. At 16k bloons the frame is GPU-bound.
- Rendering and loading: a bloon sprite atlas took draw calls from ~3,400 to ~33 a frame (40 fps at 16k bloons became 81-120 fps at 21-24k). Startup went from 2.2 s to 0.3 s: models and sprites load on first use, PNGs decode on worker threads, and next upgrades and MOAB models are prefetched in the background. `perf_budget` test guards the sim; the "Keeping it fast" rules are in `renderer/README.md`.
- More water-tower and hero work: MOAB / ZOMG Takedown, First Strike, Pre-emptive Strike, Sub Commander, Flagship deck placement, Carrier planes, and generic hero ability pieces (projectile boosts, MOAB Barrage, Biohack, Long Arm of Light, Drone Swarm, UCAV, Mega Mine, Quick Thinking). `every_ability_runs` activates every ability of every tower and hero tier.
- Maps: all ~81 exportable maps (`tools/assets/export_all_maps.sh`), with one track per `entrySplitter` and round bloons taking the tracks in turn (HYPOTHESIS P-MULTIPATH; maps other than Monkey Meadow and Logs are experimental).
- Paragons: all 14. The degree comes from the game's own code (`ParagonTower.GetTowerInvestment`, `CombineInvestments`, `GetDegreeFromInvestment`, `GetDegreeMutator`, `PowerDegreeMutator.MutateTower`), with their models (legs plus top).
- Difficulties: Easy / Medium / Hard / Impoppable / CHIMPS from `modes.json`. Exactness fixes found on the way:
  - The playable build didn't apply `GlobalSpeedModModel`, so Medium bloons were 10% too slow.
  - Prices now use the game's cost rounding (`TowerManager.GetTowerCost`: x / 5 rounded half to even, f32).
- Tests: `difficulty_costs_and_modes`, `paragon_degree_and_upgrade`, `takedown_hooks`, `first_strike`, `preemptive_strike`, `sub_commander`, `flagship_deck`, `hero_barrage_and_biohack`, `mega_mine`, `every_ability_runs`, `perf_budget`, `atlas_samples_match`, `carrier_planes`, `buccaneer_broadside`, `buccaneer_trade`, `sub_submerges`, `mermonkey_trance_and_water_range`, and an ignored per-tier sweep (`cargo test -p btd6-game water_tiers_pop -- --ignored`) that checks every water-tower tier pops.

## Blockers and decisions needed from you

1. **Live game traces (most important):** one is needed to move anything to verified-obs. The plan is an offline-only MelonLoader logging mod that records `Bloon.distanceTraveled` per frame on one map in round 1. Building it needs the .NET SDK, which isn't installed, and you'd launch the game yourself.
2. **REA:** `npx rea-agents` is still blocked by this session's permission policy, so all native work used self-written tools plus VS `dumpbin`. To use REA, add a permission rule for it in Claude Code's settings.
3. **Legal:** see the earlier note. The EULA likely restricts reverse engineering. Everything proprietary stays in git-ignored `local_extracted/` and `research/dumps/`.

## Next steps

1. **Map paths:** find the code that builds `PathModel.points` from the map scenes. The forked session currently samples Bezier curves itself (H-PATH-2). Then load real paths, including spawn and leak points.
2. **Remaining movement hypotheses:**
   - `DistanceToExtraPointInfoBloon`: per-point speed multiplier and untargetable zones.
   - `LerpAngle` (H-ROT-1).
   - `CalcDamageFromBloon` (H-LEAK-2).
   - Bloon radius origin (H-BLOON-1).
   - The H-ORDER-3 creation path.
3. **Bloon damage:** `Bloon.Damage`/`Degrade`/`BloonDegradeStepper` and child spawning (placement distances, RNG use).
4. **First tower:** Dart Monkey 0-0-0. That covers targeting (`Path.bestFirst/Last/Strong` caches), the attack cooldown, the projectile (`TravelStraitModel`), pierce, the `CollisionGrid` broad phase, and damage.
5. **Baseline:** a timing patch for `Simulation.Simulate` in the original (after item 1 of the blockers), then a matched benchmark.
6. **Prototype towers:**
   - Adding a tower now takes one `PLAYABLE` entry, its prefab line and its projectile GUIDs in `build_display_plan.py`, plus any behaviours the rules don't cover yet.
   - Next candidates and what each needs:
     - Desperado, then the military towers (Sub, Buccaneer, Ace, Heli, Mortar, Dartling), which need water placement, moving towers and aimed attacks.
     - The effects system also covers MOAB Domination's burn, MOAB Press's stun and Bloon Impact's stun (a `SlowModel` with multiplier 0). The Sniper's Maim / Cripple MOAB (`SlowMaimMoabModel`) uses it with per-class durations.
   - Find where the game stops map-wide towers from targeting off-screen bloons (P-OFFSCREEN is a stand-in).
   - Also still to do: stun (`SlowModel`), tower abilities, the explosion and Ring of Fire particle effects, the game's real music selection and sound limiter, the pause-screen volume sliders.

## Layout

| Dir | Contents |
|---|---|
| `research/` | Findings (`00`–`04`). `dumps/` (git-ignored) holds the metadata dumps, address map and `asm/` listings. |
| `specs/` | `compatibility.md`: matrix and hypothesis registry |
| `engine/` | `btd6-sim`: `time`, `rng`, `math`, `path`, `bloons`, `rounds`, `sim`, `data` |
| `renderer/` | `btd6-game` prototype (forked session) |
| `tests/` | `compat-tests` and `observations/` (no traces yet) |
| `benchmarks/` | `sim-bench` and `results/` |
| `tools/` | `dotnet_meta`, `unityfs`, `il2cpp/`, `data_export/`, `sim_model/` and `assets/` (forked session). `external/` is git-ignored. |
| `local_extracted/` | Proprietary extracted data, **git-ignored, never commit** |


2026-10-27 menu pass: the daily chest collider now opens the native reward
calendar. It reads the cached server cycle/reset offset and saved reward index,
previews a claim with the native tick animation, and counts down to the next
reset without crediting rewards or writing claims. Removed the home TV and its
hit target. Perks is hide-only, without a foreground toggle. Profile category
selection, imported-save markers/details and a fresh-game confirmation are wired;
full save restore/overwrite, friends downloads and Legends-specific stat rows
remain open in ISSUES.txt. Validation: 121 Rust tests and 21 offline Python tests;
screenshots cover reward availability/opening, home TV removal and save markers.

2026-10-27 storage pass: added verified lossless animation packing and backwards
compatible streaming reads. Local models 19.488 -> 9.939 GB (49% smaller),
9.549 GB reclaimed; animation payloads 54.13% smaller. Every frame/normal byte
is preserved. New installs pack at finish step v3; existing unpacked exports
remain readable. General model staging now uses the CPU worker and cooperative
uploads. Synchronous demand callers can still wait on an unprepared model.
123 Rust tests, 29 offline tooling checks, four exact prefetch GPU comparisons,
and packed gameplay/home smoke checks passed. Claude's engine/simulation changes
and recorder scenarios were left untouched. See tools/EXTRACTION.md for limits.

2026-10-27 storage follow-up: exact frame-difference prediction and existing-runtime
quantization at installation reduce models further to 5.080 GB (74% below the
initial 19.488 GB; 4.860 GB saved in this pass). Raw float staging no longer
allocates a second full float vector. Identical whole frames can share RAM with
no per-vertex lookup changes. Measured Village animation RAM 80.9 -> 9.1 KB;
Mermonkey conversion 24.38 -> 3.63 ms; Pat prediction load 97.17 -> 80.69 ms.
These are asset-specific measurements. Total audited runtime asset folders remain
8.448 GB versus the official install's 3.362 GB, so the total-size target remains
open. Native bone/curve storage and decoded WAV data remain major gaps.
Finish v5 requires the matching executable, canonical_anims.py and delta_anims.py;
all old layouts remain readable. Validation: 130 Rust / 36 offline Python checks,
real-asset exact comparisons, four GPU comparisons and menu/gameplay captures.
Claude's simulation work and proprietary outputs remain unstaged.


2026-10-27 audio storage: all 2,806 PCM16 WAVs replaced with verified lossless
FLAC: 1.990 -> 0.845 GB, saving 1.145 GB (57.55%). No WAVs remain in runtime
audio. Clean exports write FLAC directly; audio step v2 migrates old installs.
Same mixer/resampler samples verified bit-for-bit, including the largest track.
Music decode and shared PCM preparation run on workers; mixer queue avoids the
large track copy. Max two workers / one music lookahead. Cold SFX can still wait;
playback PCM memory does not shrink with the compressed disk files. Root BTD6.exe
updated and audio-enabled gameplay/home checks passed (five effects, zero missing,
music playing). 131 Rust game tests passed; explicit equivalence and mixer tests
passed. Runtime asset audit now totals 7.099 GB; still larger than official BTD6.
Claude's simultaneous simulation/data and other UI edits are left out of this
commit. See tools/EXTRACTION.md for measurements and limits.

Audio follow-up tooling validation: 42 offline Python checks passed; the release
ZIP includes the compressor and excludes private account tools and scenarios.
Synthetic FLAC/mixer equivalence is also wired into the release workflow.


2026-10-27 shared textures: finish step v6 shares exact referenced model PNG bytes
through ordinary relative metadata paths. Net 1.089 GB saved, with no downscaling,
re-encoding or runtime decompression. Model folder is now 3.991 GB and audited
runtime asset folders 6.010 GB. Still above the official install's 3.362 GB; baked
animations (~3.23 GB) are the remaining major storage problem. Independent checks
preserved every existing image referenced by all 15,108 texture fields in 2,020
model metadata files, plus every non-path metadata value. Repeat pass changed
nothing. Existing exe passed gameplay (Dart/Wizard) and home-menu smoke captures.
No renderer/simulation code changed. Claude's engine/game and website edits remain
outside this change.


2026-10-27 animation recompression: packed animations now use zlib level 6 (pack_anims.py,
delta_anims.py) and finish step v7 runs repack_anims.py to migrate existing installs. Every
replacement is streamed, verified byte-for-byte against the original payload and swapped
atomically only when smaller. Local migration: 1,841 animations, 3.232 -> 2.833 GB, saving
0.399 GB (12.3%) in 257 s with two workers; no pending files remain. Models folder
3.991 -> 3.591 GB. Runtime decoding is unchanged (any zlib level reads the same); rebuilt
BTD6.exe gameplay smoke (Heli 5-0-0, Wizard, Alchemist, Ninja, MOAB) rendered correctly.

2026-10-27 duplicate assets: dedupe step v1 (link_duplicates.py) stores byte-identical runtime
assets once as NTFS hard links; every path and byte is unchanged, so no runtime change. Only
binary assets (.png/.dz/.z/.bin/.flac/...) are linked, never metadata. Before any later
re-export the driver splits links back into separate copies (exporters overwrite in place) and
dedupe relinks at the end. Local: 3,802 files, 0.328 GB stored once (mostly tower animations
repeated across upgrade tiers and the same shadow.png in ~1,000 model folders). Runtime asset
folders 5.614 -> 5.285 GB. Gameplay smoke with the rebuilt BTD6.exe rendered normally.


2026-10-28: finish v8 measured lossless .wz animation migration: 1,059 accepted,
547 retained old format; 629 MB logical savings. Runtime folders now 5.138 GB
logical / 4.781 GB unique file bytes after hard-link deduplication, versus
5.767 / 5.439 GB immediately before. Accepted stream decode medians sum to
12.120 -> 9.980 s; no per-frame simulation/rendering change. Exact native/Python
verification, 60 tooling tests and six storage tests passed. Full game suite
still has the separate Preemptive Strike failure. See tools/EXTRACTION.md.

Sandbox: opening Bloons clears selection in the same frame. Selected right-side
upgrade panels no longer make the Bloons button appear dead. Fresh installed
GameAssembly trace confirms Show calls SetSelected(null); broader native external
hide/position wiring is not yet fully established. Selected Druid smoke passed.
