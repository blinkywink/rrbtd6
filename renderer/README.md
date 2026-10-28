# btd6-game: playable local prototype

This prototype plays Monkey Meadow on Medium with the Dart Monkey, Boomerang Monkey, Bomb Shooter, Tack Shooter, Ice Monkey, Glue Gunner, Desperado, Sniper Monkey, Mortar Monkey, Dartling Gunner, Wizard Monkey, Super Monkey, Ninja Monkey, Alchemist, Druid, Banana Farm, Spike Factory and Monkey Village. All data and art are loaded at runtime from `local_extracted/`, which is built from your own BTD6 install and git-ignored. Nothing proprietary is in the source.

## Run

```
bash tools/build_local_assets.sh      # once, and again after each game update
cargo game                            # = cargo run -p btd6-game --profile game
```

`cargo game` uses the `game` profile: fully optimized, but without link-time optimization and with incremental builds, so a code change rebuilds in about 2 seconds. `--release` (used for the engine benchmarks) takes about 14 seconds on the final step.

## Controls

| Input | Action |
|---|---|
| Click a placeable tower in the shop (or press 1-9 and 0: Dart, Boomerang, Bomb, Tack, Ice, Glue, Desperado, Sniper, Mortar, Dartling), then click the map | Place a tower |
| "Set Target" in the upgrade menu (Mortar, or the Dartling in Locked mode), then click the map | Set where it fires (right click / Esc cancels). A new Mortar asks for its target straight away |
| Drag a tower sideways out of the shop and release it on the map | Place a tower (released on a blocked spot it stays in hand; released over the HUD it's cancelled) |
| Right click / Esc | Cancel placement (Esc with nothing selected opens the pause screen) |
| Mouse wheel, or drag the shop list | Scroll the tower shop (with inertia, like the game) |
| Gear button (by the round counter) | Pause screen: Auto Start and Game Hints toggles, Restart, Continue (other buttons are there but do nothing yet) |
| Upgrades button (above the shop) | Upgrade tree for the selected shop tower: click an upgrade for its description, the arrows switch tower, Back closes |
| Space, or the Go button (bottom right) | Start the next round; during a round the button toggles 1x / 3x |
| F | Toggle 1x / 3x speed |
| A (or Auto Start on the pause screen) | Toggle auto-start: after you start a round, each next round starts when the last one ends |
| P | Show where a Dart Monkey can be placed (2D view, green = valid) |
| V | Toggle the 3D BTD6 camera and the 2D top-down view |
| Click a placed monkey | Open the upgrade menu: upgrade buttons, targeting arrows (hidden for towers with one targeting mode), Sell, X to close, and the "i" button for the info panel (pops and the next upgrade on each path) |
| `,` `.` `/` | Upgrade path 1 / 2 / 3 of the selected monkey (BTD6 hotkeys) |
| Tab | Cycle targeting (First, Last, Close, Strong; Normal / Locked for the Dartling) |
| Backspace | Sell the selected monkey (70% back, Medium) |
| Move the mouse over a banana | Collect it |
| C | Empty the selected Monkey Bank into your cash |
| M | Mute / unmute the music |
| U | Pick up the hero |
| H / Shift+H | Before placing anything: next / previous hero (saved in `local_extracted/settings.json`) |
| G | Skywarden: switch the selected one between Spread and Focus stance |
| D / Shift+D | Before doing anything: next / previous difficulty (Easy, Medium, Hard, Impoppable, CHIMPS; saved in `local_extracted/settings.json`) |
| N / Shift+N | Before placing anything: next / previous map (Monkey Meadow, Logs; saved in `local_extracted/settings.json`) |
| Esc → pause screen | Music / sound effects / hero voice sliders (saved to `local_extracted/settings.json`), jukebox next / previous / loop / shuffle |

## Keeping it fast (rules for new content)

The sim has to stay exact, so speed comes from how the work is organised, never from skipping or approximating game rules. `benchmarks/README.md` has the numbers.

- **Run the guard.** `cargo test --profile game -p btd6-game` includes `perf_budget` (late-game sim load: 1.2 ms average / 6 ms worst per step). For details, `sim_bench` prints per-phase and slow-frame timings: `cargo test --profile game -p btd6-game sim_bench -- --ignored --nocapture`.
- **Per-bloon and per-projectile loops:**
  - Cheapest test first, and the range test before string or tag filters.
  - No allocation, no `String` hashing or cloning, no linear `contains` on growing lists (`HitSet`).
  - Resolve names to indices once at load (`child_kinds`, `by_name`).
- **Collision:** use the projectile grid (`BloonGrid`). Anything that adds or removes bloons inside the projectile phase must go through `spawn` / `remove_bloon` so the grid stays in step.
- **Idle work:** skip work that can't have an effect this frame (an attack whose weapons are all reloading skips its target search). Never skip anything that changes game state or consumes the sim RNG.
- **Sounds:** queue them through `sound` / `sound_room`, which drop what the audio limits would drop anyway.
- **Rendering:**
  - Sprites drawn in large numbers share a texture, through an atlas (`render3d::build_atlas`), and go through `SpriteBatch`.
  - Each texture switch is a draw call; thousands of draw calls is what made 16k bloons run at 40 fps.
- **Loading:**
  - Nothing big loads at startup unless the first frame shows it.
  - Load on first use, or ahead of time with `anim::Prefetch::request` (decoded on a background thread, uploaded on the render thread).
  - Decode batches of PNGs with `render3d::decode_batch`.
  - `RRBTD6_TIMING=1` prints load phases.

## Portable copy

`bash tools/package_desktop.sh` builds a static-CRT exe and copies it with the runtime data into `Desktop/BTD6-rrbtd6/`, zipped (about 1.7 GB). The exe looks for `local_extracted/` next to itself first, so the folder runs anywhere. Model animations are stored compactly (`tools/assets/compact_anims.py`): only the idle and attack clips, with quantized positions and normals. That took the models from about 14 GB to 2.5 GB. It's private use only.

## Window size

The map retains its native 16:9 reference composition when the window is resized.
Its camera uses the extracted 60-degree pitch and 100.458946 orthographic size;
the reference view scales uniformly into the window with padding outside it,
rather than changing horizontal zoom. HUD layout remains responsive. Projection,
mouse picking, particle sizing and render-scale output share the same framing.
The fixed reference framing is a client policy; original-game behavior at every
unsupported window shape has not been verified. `RRBTD6_WINDOW=WxH` sets the start
size; `RRBTD6_VIEWPORT_CHECK=1` checks the reference width and ground-picking
round trips in the real graphics context.
On Windows the window's client area itself is locked to 16:9 during edge/corner
dragging, programmatic resizing, snapping and maximizing. Title-bar/border pixels
are excluded from the constraint. `RRBTD6_NATIVE_ASPECT_CHECK=1` exercises all
eight native sizing messages plus actual maximize/restore; `RRBTD6_RESIZE_CHECK=1`
with an autotest exercises live size requests and saves intermediate screenshots.

## Autotest

`RRBTD6_AUTOTEST=<png>` places three towers, plays, saves a screenshot and exits. Options:
- `RRBTD6_FRAMES`: frame count (default 420).
- `RRBTD6_BURST`: also save that many frames before the last one.
- `RRBTD6_SPEED`: game speed.
- `RRBTD6_CASH`: starting cash.
- `RRBTD6_ROUND`: start at this round (40 = first MOAB).
- `RRBTD6_TOWERS`: tower indices to cycle through, in shop order (0 = Dart Monkey, 1 = Boomerang Monkey, 2 = Bomb Shooter, 3 = Tack Shooter, 4 = Ice Monkey, 5 = Glue Gunner, 6 = Desperado, 7 = Sniper Monkey, 8 = Monkey Ace, 9 = Heli Pilot, a = Mortar Monkey, b = Dartling Gunner, c = Wizard Monkey, d = Super Monkey, e = Ninja Monkey, f = Alchemist, g = Druid, h = Banana Farm, i = Spike Factory, j = Monkey Village, k = Engineer Monkey; `RRBTD6_MODE` (e.g. `Pursuit`, `Point`, `Infinite`) sets every placed tower's targeting / movement mode; e.g. `30`).
- `RRBTD6_PATHS`: upgrade path order, applied at once and between rounds.
- `RRBTD6_PLACE`: comma-separated track distances to place the towers next to (default `300,520,700`; `900` lands on the right half, so the upgrade menu opens on the left).
- `RRBTD6_SELECT`: which placed tower starts selected (default 0).
- `RRBTD6_NOSELECT`: don't open the upgrade menu.
- `RRBTD6_INFO`: open the info panel.
- `RRBTD6_MENU=pause|upgrades`: open that menu.
- `RRBTD6_PICK`: pick an upgrade in the upgrades screen.
- `RRBTD6_OUTLINE`: outline scale (default 0.7).

## What is real and what is a stand-in

**Real, from the game files:**
- All 140 default rounds (bloon types, counts and timing).
- Bloon speed, health, children and radius, and Dart Monkey cost, range, attack rate, pierce, damage and dart speed and lifespan.
- Medium starting cash and lives, plus the end-of-round bonus.
- The Monkey Meadow track, its placement areas, tree blocker circles and its terrain.
- Tower placement follows the game's `Map.CanPlace`: a 0.75 × radius footprint against track areas at the same height, the full radius against blocker circles and other towers, and the ±150 / ±116 world bounds. Only the polygon-overlap helper is a standard-geometry stand-in.
- The 3D trees, grass and Dart Monkey model, the bloon sprites, and the camera angle.
- The range circle colours (the game's `Range.png` lookup), and the BTD6 hand cursor.
- The in-game UI. `src/hud.rs` loads the game's own prefabs (`MainHudLeftAlign`, `MainHudRightAlign`, `RightPanel`, `TowerSelectionMenu`, `TSMThemeDefault`, the shop's tower button), dumped by `tools/assets/dump_ui_prefab.py`:
  - `src/uitree.rs` lays them out with Unity's RectTransform anchoring, layout groups and the 3840×2160 "Expand" canvas scaler.
  - Images use the atlas sprites with their 9-slice borders.
  - Text is rendered with the game's TextMeshPro SDF fonts and a port of TMP's SDF shader, using each text's own material (outline colour, dilate, underlay). `Btd6FontTitleSDF` is the English variant, Luckiest Guy; the game ships one asset of that name per script (Bloons TD, Fira Sans, Noto CJK and others). `Btd6FontBodySDF` (Source Sans Pro SemiBold) is used for descriptions.
  - When the upgrade menu opens on the left, lives and cash slide to the right of it (`StatsPanel` controller, `StatsSlideInFromLeft` curve), and they snap back when it closes.
  - The HUD code only binds game state and hides the objects that belong to other game modes.
  - The shop lists the default hero and every tower in `ShopTowerDetailsModel` order, with their Medium costs and category containers. It scrolls inside its `RectMask2D` with the wheel or by dragging (`ScrollRect` drag threshold and deceleration rate; a drag cancels the button press).
  - Menus (`src/menus.rs`): the pause screen (`PauseMenuPanel`) and the upgrades screen (`UpgradeScreen` with its `UpgradeDetails` cards, description popup and the common Back button) come from the game's UI scenes. They are drawn over a blurred, tinted snapshot of the game like `CommonBackground`, darkened at the edges by the game's `NinjaKiwi/BTD6/VignetteUI` formula (the upgrades screen has no background image of its own; the game assigns this snapshot at runtime). Names and descriptions come from the game's `English.xml`. Nodes can be rotated: half turns flip both axes, and other angles rotate the node's own graphic. TMP word wrap and auto-size are ported. The Perks button is hidden, as in this mode. Only the Dart Monkey, Boomerang Monkey, Bomb Shooter, Tack Shooter, Ice Monkey, Glue Gunner, Desperado, Sniper Monkey, Mortar Monkey, Dartling Gunner, Wizard Monkey, Super Monkey, Ninja Monkey, Alchemist, Druid, Banana Farm, Spike Factory and Monkey Village can be placed so far; the others can only be selected.
  - Atlas sprites are padded back to their full `Sprite.m_Rect` (the packed texture is trimmed), so icons such as the targeting arrows lay out at the game's size.
  - Unaffordable upgrades use the game's locked state: the `GreenArrowBtnLocked` sprite and the OutlineBrown text material (`UpgradeButton.lockedMaterial`).
  - Stand-ins: the menu and panel animations aren't played (buttons only shrink slightly while pressed), and the round panel's slide-aside offset is measured rather than read from its animation clip.

**Not yet verified:** the gameplay rules in `src/game.rs`, each tagged `HYPOTHESIS P-*`. These cover:
- targeting
- hit tests
- pop and children placement
- cash per pop
- leak cost
- time units

They will be replaced by the verified `btd6-sim` engine as the native code is decoded.

**Dart Monkey, as of now:**
- All 64 tier combinations with their real stats: range, rate, pierce, damage, speed, lifespan, arc and parallel shots, crits, Ceramic/Fortified/Lead bonuses, camo detection and Lead/frozen immunity.
- Upgrade costs and the crosspath rule (two paths at most, one above tier 2).
- The real 3D model with its Animator clips baked from the game: idle loop and the attack throw. It also blinks.
- Shading follows the game's disassembled shaders: the monkey materials (`Shader Graphs/GrayscaleShader`, `NinjaKiwi/BTD6/LitMonkey`) are unlit, plain texture colour with mipmapped sampling. The outline is the URP "Tower Outlines" pass (`NinjaKiwi/SimpleUnlitOutline`): each vertex pushed out by 0.7 × `_Outline` along its view-plane normal, flat `_OutlineColor`, and stencil-masked so only the silhouette shows. The stencil is reproduced by drawing all outline meshes first, then the body over them.
- Baked "Flat_" meshes (Boomerang, Bomb, Tack, Sniper and most tier models) use the `BAKED_MESHES` variant, which has no `_Outline` property: the outline colour is in the vertex colour's rgb and whether to draw it in its alpha. The width there is INFERRED (0.97, about the Dart Monkey's `_Outline`).
- The selected tower's outline is drawn white (STAND-IN for the game's selection highlight).
- Towers cast the prefab's `RadialShadow` sprite (`MonkeyRadialShadow`, scale 0.9) flat on the ground.
- Every tier shows its own display prefab (Crossbow, Juggernaut, Ultra-Juggernaut, ...) with its own baked idle and attack clips, loaded the first time a monkey reaches that tier.
- Projectiles use each tier's display: dart, triple dart, arrow and Crossbow Master arrow sprites, and the 3D Spike-o-pult, Juggernaut and Ultra-Juggernaut balls. The projectile-to-tier mapping is inferred by tier family.
- The Pop effect is the real 4-frame flipbook.
- Not yet: Fan Club abilities, knockback, and Ultra-Juggernaut split timing (uses a guess).

**Tack Shooter, as of now:**
- All 64 tier combinations with their real stats, upgrade icons, portraits and descriptions, and each tier's display prefab with its baked clips.
- It doesn't turn (no `RotateToTargetModel`) and targets Close only (its single `TargetCloseModel`), so the targeting arrows are hidden. Volleys of 8 / 10 / 12 / 16 / 32 tacks spread evenly around its facing.
- Tack, Hot Tack and Blade sprites, and the Inferno Ring meteor mesh.
- Ring of Fire and Inferno Ring: a stationary hit with radius = tower range (`LinkProjectileRadiusToTowerRangeModel`) for the `AgeModel` lifespan. The burst graphic is a stand-in, since the game's `RingOfFireBurst` is a particle effect.
- Not yet: the Inferno Ring meteor homing (`TrackTargetModel`; flies straight), the Super Maelstrom ability and the Overdrive / Tack Zone specifics beyond their stats.
- Only one of each tier 5 per tower type (all towers).

**Bomb Shooter, as of now:**
- All 64 tier combinations with their real stats, models, icons, portraits and descriptions.
- Bombs, missiles and MOAB Mauler / Assassin / Eliminator rockets do no damage themselves. On contact they spawn the explosion projectile (`CreateProjectileOnContactModel`: radius, pierce, damage, Black immunity, MOAB damage bonuses), which hits everything it covers for its `AgeModel` lifespan.
- Frag Bombs throw 8 frags, and Cluster Bombs throw 8 small bombs that explode where they land (Recursive Cluster nests this once more). The frags' random range uses its midpoint (HYPOTHESIS P-RANDRANGE).
- Projectile art: bomb, big bomb, really big bomb and cluster sprites, frag sprites, and missile meshes (inferred by tier family; the missile crosspaths are the least certain).
- The explosion graphic is a stand-in: the game's is a particle effect.
- Not yet: Bloon Impact / Bloon Crush stun (`SlowModel`), MOAB Assassin / Eliminator abilities, Bomb Blitz.

**Sniper Monkey, as of now:**
- All 64 tier combinations with their real stats, models (with their idle, attack, salute and present-arms clips), icons, portraits and descriptions.
- Map-wide range (`isGlobalRange`): the range circle shows its `TowerModel.range` (20), not the attack's 9999999. Bloons off the map edge can't be targeted (HYPOTHESIS P-OFFSCREEN), so it starts shooting when they come on screen.
- Instant shots (`InstantModel`): the hit lands on the target the frame it fires. Lead and frozen immunity, Ceramic and camo bonuses, and camo detection from Night Vision Goggles come from the data.
- Shrapnel Shot: 5 shrapnel pieces in a 45° arc along the shot's direction, only on hits that deal damage (`EmitOnDamageModel`).
- Bouncing Bullet: after each hit, it jumps to the closest unhit bloon within 40 units, 0.1 s later, until its pierce runs out (HYPOTHESIS P-BOUNCE).
- Cripple MOAB's explosion on contact works like the Bomb Shooter's.
- The shot is drawn as a short fading tracer; the game's muzzle flash and bounce streak are particle effects.
- Maim MOAB / Cripple MOAB (`SlowMaimMoabModel`): stops MOAB-class bloons for their class's duration (MOAB, BFB, ZOMG, DDT, BAD) and, for Cripple MOAB, adds its extra damage to every hit meanwhile.
- Not yet: Supply Drop, Elite Sniper and Elite Defender abilities, and Elite Sniper's buff to other Snipers.

**Boomerang Monkey, as of now:**
- All 64 tier combinations with their real stats, models, icons, portraits and descriptions.
- Boomerangs fly the game's own path (`FollowPathModel`: a fixed list of points, 175 or 227.5 units/s, gone at the end). The path is turned so it points at the target, unscaled (HYPOTHESIS P-PATH); Long Range Rangs uses its longer loop, and Kylie Boomerang its straight 81-unit line.
- Kylie Boomerang forgets which bloons it hit every 0.3 s (`ClearHitBloonsModel`), so it can hit them again.
- Glaive Ricochet and up throw straight glaives that bounce to the closest unhit bloon within 80 units and home in on it (HYPOTHESIS P-BOUNCE-TRAVEL).
- MOAB Press: a second attack that only targets MOAB-class bloons (never BADs). It flies to where the target was over 1.26 s (HYPOTHESIS P-ARRIVE: straight, without the game's ease curve), hits everything in its way, forgets hits every 0.1 s, and pushes MOABs back 2 units per hit (BFB 1, ZOMG and DDT 0.5; HYPOTHESIS P-PUSH).
- Projectiles use each tier family's prefab (boomerang, Red Hot, Kylie, glaives, MOAB Press) and spin at their `RotateModel` rate.
- Glaive Lord's orbiting glaives (`OrbitModel` display, plus a fire-without-target attack that hits around the tower). The orbit speed is a guess.
- Not yet: MOAB Domination's damage over time and slow, the Turbo Charge / Perma Charge abilities, and the flip-throw-side button (`FlipFollowPathModel`; always the default side).

**Bloon effects (all towers):**
- Projectiles put effects on the bloons they hit, from their `SlowModel`, `SlowForBloonModel`, `FreezeModel`, `AddBehaviorToBloonModel` (`DamageOverTimeModel`) and `AddBonusDamagePerHitToBloonModel`. Each lasts its `lifespan` and for `layers` layers: children of a popped bloon inherit it while layers remain. Hitting again with the same id refreshes it unless `dontRefreshDuration`.
- Slows with different ids multiply; a freeze stops the bloon and sets the Frozen property (16), so projectiles immune to Frozen (darts, tacks) can't damage it (HYPOTHESIS P-SLOW).
- Per-tag modifiers: `FreezeModifierForTagsModel` scales freeze time (Ceramic 0.5); `SlowModifierForTagModel` replaces the multiplier when `resetToUnmodified`, otherwise scales it (HYPOTHESIS P-SLOWMOD).
- Damage over time ticks every `interval` and counts as pops for the tower that applied it. Brittle adds its damage to each later hit.
- Filters: `FilterGlueLevelModel` skips bloons glued at that level or more, `FilterFrozenBloonsModel` skips frozen ones, and `FilterOutTagModel` skips tags, for both targeting and hits.
- Arctic Wind's `SlowBloonsZoneModel` slows bloons within range + 5 to 0.6 (not Ice, Lead, MOABs or camo).
- Frozen bloons are tinted blue and glued ones yellow; these tints stand in for the game's overlays.

**Wizard Monkey, as of now:**
- All 64 tier combinations with their real stats, models, icons and portraits. The bolt, blast, intense magic, fireball and Dragon's Breath prefabs are particle systems only, so `export_projectiles.py` takes the first flipbook cell of each one's particle texture as a STAND-IN sprite (the flame mask is tinted with its material's flame colours). Shimmer has no art yet.
- Guided Magic uses the seeking rule (P-SEEK). Fireballs explode on contact. Dragon's Breath is its fast short-range flame attack.
- Wall of Fire: `TargetTrackOrDefaultModel` (useClosestTrack) puts an instant projectile on the track point nearest the Wizard, up to 4 units off, for 4.5 s. Its graphic is a particle effect, so it's drawn as a stand-in fiery disc.
- Shimmer removes camo from everything in its radius without dealing damage (`cleanseOnlyIfDamaged` false).
- Not yet: Phoenix and Wizard Lord Phoenix (sub-towers), Necromancer and Prince of Darkness undead bloons (they need movement along the path), Archmage's extra attacks beyond the generic rules.

**Super Monkey, as of now:**
- All 64 tier combinations with their real stats, models, icons and portraits; laser, plasma, sun, Dark Knight / Dark Champion blade and Tech Terror / Anti-Bloon blast art.
- Robo Monkey's second arm is an extra attack with the tower's target mode. Ultravision's camo detection comes from the data.
- Not yet: Sun Temple / True Sun God sacrifices (`MonkeyTempleModel`), Tech Terror and Anti-Bloon abilities, Dark Knight's Darkshift, and Legend of the Night's abilities.

**Ninja Monkey, as of now:**
- All 64 tier combinations with their real stats, models, icons and portraits; shuriken, sharp shuriken, caltrop and sticky bomb art.
- Camo first (`Target*PrioCamoModel`). Seeking shurikens (`TrackTargetModel` / `TrackTargetWithinTimeModel`) turn at `turnRate` and pick the closest unhit bloon within half of `maxSeekAngle` of their heading once their target is gone (HYPOTHESIS P-SEEK).
- Distraction (`WindModel`): `chance` per hit to move the bloon `distanceMin..Max` back along the track, scaled for Ceramics (HYPOTHESIS P-WIND). Counter-Espionage (`RemoveBloonModifiersModel`): damaged bloons and their children lose camo.
- Caltrops: `TargetTrackModel` aims at a random track point in range (HYPOTHESIS P-TRACKTARGET); `ArriveAtTargetModel` with `expireOnArrival` false flies there without hitting anything, then stays for its lifespan.
- Flash Bomb: `AlternateProjectileModel` replaces every 4th throw (HYPOTHESIS P-ALT). Bonus damage to stunned and sticky-bombed bloons (`DamageModifierForBloonStateModel`).
- Sticky Bomb / Master Bomber: MOAB-only attack skipping bloons that already carry one (`FilterMutatedTargetModel`), a one-tick damage over time plus a delayed splash (`ProjectileOverTimeModel`).
- Random spread and blow-back use a stand-in random generator, not the game's seeded one.
- Shinobi Tactics (`SupportShinobiTacticsModel`): every Shinobi in range, itself included, multiplies Ninja attack intervals by 0.92, stacking up to 20 times (through the support system below).
- Not yet: Bloon Sabotage and Grand Saboteur abilities, Grandmaster Ninja's `DamagePercentOfMaxModel`.

**Spike Factory, as of now:**
- All 64 tier combinations with their real stats, models, icons and portraits.
- During rounds it throws spikes at a random in-range track point (P-TRACKTARGET); they fly there without hitting anything, then stay. Their lifespan only runs during rounds (`AgeModel.useRoundTime`), and they're cleared at round end unless `AgeModel.rounds` lets them survive more round ends (Long Life Spikes 1, Perma-Spike 3; HYPOTHESIS P-ROUNDS).
- The pile art follows the pierce left (`SetSpriteFromPierceModel`: entry n shows n spikes, clamped; HYPOTHESIS P-PIERCESPRITE). The sprite lists are matched to the `SpikePile`, `HotSpikePile`, `SpikedBall`, `SpikedMines` and `SuperMines` prefab families by tier.
- Faster Production's `StartOfRoundRateBuffModel` multiplies the interval by 0.25 for the first 3 s (10 s at x-0-5) of each round (HYPOTHESIS P-ROUNDRATE). Spiked Balls' Ceramic bonus uses `DamageModifierForBloonTypeModel`. Spiked Mines explode when used up or expired.
- Not yet: the Close / Smart / Far / chosen-point target modes, Spike Storm and Carpet of Spikes abilities.

**Banana Farm, as of now:**
- All 64 tier combinations with their real stats, models, icons and portraits; banana, Long Life banana and crate art.
- Each round it fires its `EmissionsPerRoundFilterModel` count of bananas at the weapon rate. Each one flies at `altSpeed` to a random spot 20-40 units away (HYPOTHESIS P-RANDPOS) and lasts its `AgeModel` lifespan (bananas aren't cleared at round end).
- Move the cursor over a banana to collect its `CashModel` value. The reach is a STAND-IN (a quarter of `PickupModel.collectRadius`). Collected cash shows a rising "+$" text.
- Marketplace and later (no `PickupModel`) pay out straight away. Monkey Bank (`SendToBankModel`) stores income up to `capacity`, adds `interest` at each round end (HYPOTHESIS P-BANK), and pays out with C or on sale. The upgrade panel title shows the balance.
- Not yet: IMF Loan, Monkey-nomics, Central Market and Banana Central buffs, Monkey Wall Street's per-round bonus and lives.

**Monkey Village and support buffs, as of now:**
- All 64 tier combinations with their real stats, models, icons and portraits.
- Support models (`RangeSupportModel`, `RateSupportModel`, `PierceSupportModel`, `ProjectileSpeedSupportModel`, `VisibilitySupportModel`, `DamageTypeSupportModel`, Monkey Town's `CashIncreaseModel`, `SupportShinobiTacticsModel`) buff every tower whose footprint touches the source's range (or custom radius), filtered by tower set or base id. Each mutator id counts once (strongest) unless it stacks (HYPOTHESIS P-SUPPORT). Range buffs also grow the range circle.
- `DiscountZoneModel`: towers bought or upgraded (up to `tierCap`) in range cost less; Monkey Business 10% (one stack) plus Monkey Commerce 5% (up to three). Discounted prices round to 5 (HYPOTHESIS P-DISCOUNT). The shop buttons still show the full price.
- Not yet: Grow Blocker, Call to Arms, Homeland Defense, Primary Mentoring's free tier 1 and ability cooldowns, Monkey City income, Monkeyopolis.

**Desperado, as of now:**
- All 64 tier combinations with their real stats, models, portraits and upgrade icons. The game names its icons by tier code (`100UpgradeIcon`), so they're exported as `Desperado100UpgradeIcon` and looked up by tower and tier.
- `TargetDesperado*Model` give the usual First / Last / Close / Strong modes. Pistol shots are instant hits drawn as tracers. `BurstWeaponBehaviorModel` fires each shot `count` times, `intervalFrames` apart, at the same bloon (HYPOTHESIS P-BURST).
- The rifle (x-3-x+) only shoots bloons beyond `AttackMinimumRangeModel.minimumRange`. Its `LineProjectileEmissionModel` shot sweeps along its range over its lifespan and hits everything on the line, up to its pierce (HYPOTHESIS P-LINE). The shotgun (x-x-3+) fires pellets with the generic projectile code.
- Not yet: Standoff and `BloonDistanceRateBonusModel` rate bonuses, distance crits, marked-to-pop, Vigilante, Bounty Hunter, The Blazing Sun, Take Aim, and the pistol holding fire while the rifle has a target (`FilterIfAttackHasTargetModel`).

**Alchemist, as of now:**
- All 64 tier combinations with their real stats, models, icons and portraits; every potion, brew and acid puddle (puddles tinted by their SpriteRenderer colour).
- Potions fly to their target and burst into an acid splash that hits and puts the acid damage-over-time on bloons. A splash that hits nothing becomes an acid pool (`AcidPoolModel` lifespanIfMisses / radiusIfMisses / pierce; HYPOTHESIS P-ACIDPOOL). With Acid Pool (x-x-2+), the Alchemist throws at the track when no bloon is in range (`TargetTrackOrDefaultAcidPoolModel`).
- Brews (`AddAcidicMixtureToProjectileModel`, `AddBerserkerBrewToProjectileModel`) go to the closest attacking tower in range that doesn't have them yet, skipping the ignore list (HYPOTHESIS P-BREWTARGET). They're applied instantly, with a flash on the tower, and only during rounds. Acidic Mixture adds its per-tag damage for 30 shots. Berserker Brew adds damage, pierce, attack speed and range for 25 shots or 5 s, whichever ends first; Stronger Stimulant makes that 40 shots or 12 s and replaces the weaker brew. Permanent Brew never runs out (HYPOTHESIS P-BREW).
- Lead to Gold pays $50 when its splash pops a Lead bloon (HYPOTHESIS P-LEADGOLD). Comma-separated tag lists in `DamageModifierForTagModel` ("Lead, Ddt") now match, and `mustIncludeAllTags` is honoured.
- Not yet: Rubber to Gold's gold bloons, Bloon Master Alchemist's shrink to Red (`MorphBloonModel`), Unstable Concoction's MOAB explosion, Transforming Tonic / Total Transformation abilities, the brew's flight to the tower, buff icons.

**Druid, as of now:**
- All 64 tier combinations with their real stats, models, icons and portraits; thorn colours per crosspath (purple with Heart of Thunder, green with Heart of Oak, Poplust and Avatar of Wrath thorns), ball lightning, vine roots and tornadoes (the tornadoes are particle stand-ins).
- Thorns use `RandomEmissionModel`: each one leaves `startOffset` ahead of the Druid, spread uniformly over `angle` (HYPOTHESIS P-EMIT-RANDOM).
- Heart of Thunder's `LightningModel` is an instant hit that jumps to the closest unhit bloon within `splitRange` every `delay` until its pierce is spent. The split into several branches isn't modelled (HYPOTHESIS P-LIGHTNING), and lightning has no art yet.
- Ball Lightning's `CreateProjectileOnIntervalModel` fires a sub-shot at a random bloon in `range` every `intervalFrames` (HYPOTHESIS P-INTERVAL). Tornadoes blow bloons back and strip glue and ice (`RemoveMutatorsFromBloonModel`; HYPOTHESIS P-REMOVEMUT).
- Jungle Vines (x-3-x): a map-wide attack that lands on its target (`InstantDamageEmissionModel`, HYPOTHESIS P-ATTARGET) and destroys a non-MOAB bloon outright, then leaves a vine root that hits passers-by, its pierce refreshed every 0.3 s (`RefreshPierceModel`). The game holds the grabbed bloon for 1.4 s first (`JungleVineEffectModel`); here it pops at once.
- Not yet: Heart of Oak's regrow removal, Heart of Vengeance and Avatar of Wrath's attack speed from lives lost, Poplust's stacking buff, Jungle's Bounty cash and ability, Spirit of the Forest's vines along the track, Superstorm's ability, the Druid paragon.

**Ice Monkey and Glue Gunner, as of now:**
- All 64 tier combinations with their real stats, models, icons and portraits. The Ice Monkey's freeze ring is a stand-in light-blue flash.
- Projectile art: Glue Gunner blobs (yellow, green from 3-x-x, pink on path 3), Cryo Cannon's freeze bomb and Icicle Impale's icicle, matched by tier family.
- Not yet: Snowstorm, Absolute Zero, Glue Strike and Glue Storm abilities; Icicles, Glue Splatter, Bloon Solver and Cryo Cannon extras beyond what the generic projectile code covers.

**Mortar Monkey and Dartling Gunner, as of now:**
- All 64 tier combinations of each with their real stats, models, icons and portraits. `tools/assets/export_tower_assets.py` exports a new tower's plan, tier models, base model, icons, portraits and projectiles in one go, and `build_display_plan.py` now finds projectile prefabs through the catalog's GUID-to-prefab map when they aren't in its hand-made table.
- `TargetSelectedPointModel` and `TargetPointerModel` add the "Locked" (a point you set) and "Normal" (the cursor) modes. Both fire whenever a bloon they could hit is on the map (HYPOTHESIS P-POINTER). The upgrade menu gets the game's "Set Target" buttons (the long one from `TSMThemeMortarMonkey`, the square one from `TSMThemeSelectPointInput`), and the set point shows the `MortarTarget` marker while the tower is selected. Its size and the starting point (the nearest track point, P-POINTDEFAULT) are stand-ins.
- Mortar: each shell is an instant projectile that hits nothing (`FilterAllModel`) for 0.5 s at a random point within `RandomTargetSpreadModel.spread` of the target (HYPOTHESIS P-TARGETSPREAD), then sets off every `CreateProjectileOnExhaustFractionModel` (explosion, burn, camo and Fortified strip).
- Dartling: turns towards the cursor at `RotateToPointerModel.rate` and fires along its barrel with the random spread. It remembers the last cursor spot on the map (`DartlingMaintainLastPosModel`). Buckshot's slowing pellets fly straight (HYPOTHESIS P-SLOWDOWN); Rocket Storm missiles spend 2 pierce per hit and blast at each one (P-EXHAUSTPIERCE); Ray of Doom's line reaches across the map.
- Not yet: Pop and Awe, Rocket Storm and M.A.D abilities, Artillery Command's buff, B.A.D.S.' independent sub-guns beyond the generic rules, shell flight art, the explosion particles (stand-in glow).

**Engineer Monkey, as of now:**
- All 64 tier combinations with their real stats, models, icons and portraits; nail gun, Oversized Nails, Pin and Double Gun through the generic rules (Double Gun's two barrels fire from the tower centre).
- Sentries are sub-towers: the throw projectile (`CreateTowerModel`, or `CreateTypedTowerModel` at Sentry Expert, whose crushing / boom / cold / energy sentries take turns, HYPOTHESIS P-TYPEDTOWER) flies to a random land spot 8-40 units away (`RandomPositionModel`, HYPOTHESIS P-RANDLAND) and leaves a sentry there with its own tier (range, attack, projectile art). Sentries last `TowerExpireModel.lifespan` (25 s), can't be selected, sold or upgraded, don't block placement (STAND-IN) and credit their pops to the Engineer (`CreditPopsToParentTowerModel`). Each is drawn as the game's stand (`SentryGunLegs`) plus its turret prefab, which turns to aim; `export_tower_assets.py` exports them to `models/subtowers/`.
- Cleansing Foam: a side attack aimed at a set point that starts on the nearest track spot (`startWithClosestTrackPoint`); the square Set Target button moves it, the targeting arrows still pick the nail gun's mode. Foam patches strip camo (`RemoveBloonModifiersModel`); regrow removal isn't modelled.
- Bloon Trap / XXXL Trap: thrown onto the nearest track spot (`CloseTargetTrackModel`), one unfilled trap per Engineer at a time (`LimitProjectileModel`). It swallows non-MOAB bloons whole (`EatBloonModel`: no pops, no pop cash) until it holds 500 RBE (XXXL: 10,000), then stops and waits to be collected like a banana for the eaten RBE x2 (XXXL x3) (HYPOTHESIS P-EAT). The full-trap art isn't swapped in yet.
- Not yet: Overclock / Ultraboost (tower-targeted ability), Sentry Champion's explode-on-expire, the full-trap display, sentries scaling with Engineer buffs beyond the generic support rules.
- Asset catalog note: `catalog_prefabs.json` (location-based) is one entry off for GUIDs that `catalog_guids.json` also has, so the pipeline asks `catalog_guids.json` first (INFERRED from the tower displays: `DartMonkey`'s GUID gives `DartMonkey.prefab` there and `CrossbowMaster.prefab` in the other).

**Abilities, as of now:**
- Round buttons along the bottom left of the map, one per ability name (several towers with the same ability share a button; the number is how many are ready). Each shows its upgrade's icon, a dark pie and the seconds left while cooling down. Click to use one; they only work during a round (`canActivateBetweenRounds` is false for these towers).
- `ActivateAttackModel`: the ability's attacks are parsed like tower attacks and run alongside the tower's own for `lifespan` seconds (`turnOffExisting` pauses the tower's own); `isOneShot` ones fire a single volley. Covers Spike Storm, Snowstorm / Absolute Zero, Glue Strike / Storm, Tech Terror, Assassinate MOAB, Sabotage, Blade Maelstrom, Rocket Storm, MOAB Assassin and the like as far as their projectiles are already modelled. `TurboModel` (Turbo Charge): weapon intervals x `multiplier` (0.2) and +`extraDamage` for its lifespan.
- Cooldowns tick only during rounds (HYPOTHESIS P-ABILITYCD); a newly gained ability starts on full cooldown unless `startOffCooldown` (STAND-IN P-ABILITYSTART). `AgeRandomModel` projectiles (Spike Storm spikes) live the middle of their random range (STAND-IN).
- Timed support zones: `Activate{Rate,Range,Pierce,Visibility,TowerDamage}SupportZoneModel` and `CallToArmsModel` (rate x 1/`multiplier`) act as the tower's support for their lifespan (Call to Arms, Absolute Zero's Ice Monkey attack-speed boost, Homeland Defense...). Call to Arms' pierce multiplier isn't applied.
- IMF Loan (`ImfLoanModel`): +`amount` cash now, and `incomeRecoveryRate` (50%) of all later income goes to repaying it; the debt shows in red under the cash. Selling isn't taxed.
- Not yet: the other ability behaviours (Fan Club / Plasma Fan Club, Supply Drop, Overclock, Darkshift, Total Transformation...), hotkeys, the game's own ability button art.

**Heli Pilot, as of now:**
- The tower is the heli pad (`TowerModel.display`, `HeliPad` / `HeliPad400` / `HeliPad500`, found through `catalog_prefabs.json` since `catalog_guids.json` lacks them). The helicopter is the `AirUnitModel`: its display prefab differs per tier and crosspath (`models/air/<prefab>`, exported by `export_tower_assets.py`), and every attack aims from and fires from it; the range circle follows it.
- Movement (`HeliMovementModel`, STAND-IN P-HELI): steers toward its goal at `maxSpeed` (40; Downdraft 70, Apache Prime 50) with acceleration `movementForceStart`, slowing linearly within `slowdownRadiusMax` (15). Modes come from the main attack's setting models: Follow Touch (`FollowTouchSettingModel`, the cursor), Lock In Place (`LockInPlaceSettingModel`, a set point via the square Set Target button; until set it holds position), Pursuit (`PursuitSettingModel`, the bloon furthest along the track).
- It faces its target for a second after each shot (`RotateToTargetAirUnitModel`), else its heading. Drawn hovering at height 40 (STAND-IN P-HELIHEIGHT), moved back along the camera tilt like the MOABs.
- Helis closer than `otherHeliRepulsionRange` (30) push each other apart (STAND-IN strength). Pursuit ignores bloons off the map.
- Not yet: Patrol Points, Downdraft's blowback, MOAB Shove, Comanche Defense mini-helis, Support Chinook / Special Poperations abilities, IFR's camo detection for others, the heli's tilt and rotor animation beyond its idle clip.

**Monkey Ace, as of now:**
- The tower is the runway (`MonkeyAceBase*`); the plane is its `AirUnitModel` (`PathMovementModel`, speed 60, Fighter Plane / Sky Shredder 75), drawn from `models/air/<prefab>` like the Heli. Its attacks are `AttackAirUnitModel`s (read like `AttackModel`s): the radial darts fire without a target relative to the plane's heading; missiles / extra guns use their own Target*AirUnitModel modes.
- Flight patterns are the selectable `*PatternModel`s, cycled with the targeting arrows (STAND-IN P-ACEPATH): Circle (radius 80 round the runway), Figure Eight and Infinite (lemniscates through the runway with loops of radius 40; Infinite is the `rotated` one), and Centered Path (Neva-Miss / Flying Fortress paths: a 90-radius ellipse round a point you set with the square button, the map centre by default). Off its pattern (just placed, or the mode changed) the plane flies to it at speed x `catchUpSpeed`.
- Not yet: Exploding Pineapple / bombs only over the track (`CheckAirUnitOverTrackModel`), Ground Zero / Tsar Bomba abilities, Spectre's alternating payloads beyond the generic rules, take-off animation, wingmonkey's camo / MOAB tweaks beyond the generic filters. Some projectile displays may be the wrong prefab (the catalog map gave `FighterPlane` / `BomberAce` as projectiles).

**MOAB-class bloons:**
- The game's 3D prefabs (`Assets/Bloons/Graphics/PrefabsMoabs3D`), found from each `BloonModel.display` GUID through the Addressables catalog (`catalog_audio.py ... prefab`), exported by `export_bloon_models.py`.
- `DamageStateModel`s swap in the cracked models at 80 / 60 / 40 / 20% health.
- Drawn at sim height 120 (`Path.DistanceToPointInfoBloon`), turned along the track (`rotateToFollowPath`). The propeller spin isn't animated (the idle clip has no curves). The 2D view still draws their icons.

**Sound and music:**
- Clips come from the game's catalog (`catalog_audio.py`: Addressables GUID to `.wav` name, checked against every named `SoundModel`) and are decoded to WAV by `export_audio.py` (macroquad's `audio` feature plays them).
- Data-driven: tower place / upgrade / sell (`CreateSoundOnTowerPlaceModel` ...), weapon shots (`CreateSoundOnProjectileCreatedModel`), projectile hit / expire / exhaust, bloon damage (`CreateSoundOnDamageBloonModel`) and pops (`PopEffectModel` with its sound group limit and frame delay; HYPOTHESIS P-SOUNDGROUP). One clip is picked at random from each model's list.
- Stand-ins: other clips are capped per family (3 at once, one start per 3 frames) and 24 overall (P-SFXLIMIT); expire sounds play whenever a projectile goes (P-EXPIRESOUND); the music is a fixed playlist of the game's tracks; the HUD click is `UIClick01`; winning plays `VictoryCampaign`. The pause screen's volume sliders don't work yet (M mutes music).

**Logs map and water towers, as of now:**
- Maps: every map exported to `local_extracted/maps/<key>/` (`export_map.py`) with its scene model (`models/map_<key>`) is playable; N switches before anything is placed. `build_local_assets.sh` exports Monkey Meadow (`tutorial`) and Logs (`logs`), the first map with water.
- Placement: towers whose `areaTypes` include water (1) go on water areas, land-only towers stay off them (HYPOTHESIS P-WATER). The Mermonkey (`[1, 2]`) goes on either and gets x`PlacementAreaTypeRangeBuffModel.rangeMultiplier` (1.35) range on water (HYPOTHESIS P-WATERRANGE).
- Monkey Sub, Monkey Buccaneer and Mermonkey: all tiers with their models, icons and portraits (`export_tower_assets.py`).
- Buccaneer broadsides: `RotateToTargetModel.additionalRotation` (-90) turns the ship side-on to its target; each gun fires at the ship's facing + `Emission*RotationOff*Model.offsetRotation` (+90 at the target, -90 out the other side). A `FireWhenAlternateWeaponIsReadyModel` gun fires only when the gun with the matching `FireAlternateWeaponModel.weaponId` does (its 0.05 s rate only stops repeats), and with `FilterTargetAngleFilterModel` only if a bloon is in range within its 90° arc (HYPOTHESIS P-BROADSIDE). Cannon Ship's tier keeps an orphaned dart gun (id 1, no partner) that never fires.
- Buccaneer economy: Merchantman / Favored Trades round cash (`PerRoundCashBonusTowerModel`). Favored Trades (`CashbackZoneModel`): towers in range sell for 70% + 4% per zone (up to 3), max 95% (STAND-IN P-CASHBACK). Trade Empire (`TradeEmpireBuffModel`): +$10 per other merchant ship (up to 20) and +$20 per Favored Trades each round, and +1 damage / +1 Ceramic / +1 MOAB to merchant ships (HYPOTHESIS P-TRADEEMPIRE). Carrier Flagship (`FlagshipAttackSpeedIncreaseModel`): towers on water attack at x0.8 interval (STAND-IN P-FLAGSHIP).
- Sub: Advanced Intel's `Target*SharedRangeModel` targets bloons in range of any other tower (STAND-IN P-SHARED). Submerge and Support adds a "Submerge" targeting mode: the Sub sinks by `submergeDepth` and only runs `submergeAttackModel` (camo-stripping pulse; Bloontonium Reactor / Energizer also damage), while ability cooldowns of towers in its range run x`abilityCooldownSpeedScale` faster (Energizer: x1.2 for every tower) and heroes in range get x`heroXpScale` XP (HYPOTHESIS P-SUBMERGE).
- Mermonkey: `ScaleDamageWithTimeModel` tridents hit harder the longer they fly (HYPOTHESIS P-SCALEDMG). Alluring Melody (`TranceBloonModel`): non-MOAB bloons in range leave the track and circle the Mermonkey at `orbitRadius` for `duration` (6 s), then return to their track spot; each can't be entranced again for `cooldown` more seconds (HYPOTHESIS P-TRANCE).
- Data fixes found on the way: `FilterWithTagModel.inclusive = true` filters the tag *out* (INFERRED from Alluring Melody "Entrances non MOAB-Class Bloons"), and `LinkProjectileRadiusToTowerRangeModel` now resizes only the projectile it names (it resized every weapon, which made Mermonkey tridents range-sized rings).
- Aircraft Carrier / Flagship planes: sub-towers with `FighterMovementModel` flight (steer at maxSpeed, turn at most turningSpeed per frame; STAND-IN P-FIGHTER), at most `SubTowerFilterModel.maxNumberOfSubTowers` alive, gone with the ship.
- Carrier Flagship deck (`AddMakeshiftAreaModel`): its two deck polygons (ship-local, HYPOTHESIS P-DECK) take any tower but Banana Farm / Ace / Heli, which then stand at deck height; selling the ship sells them. The Flagship's hull keeps its facing while its guns aim (`RotateToTargetModel.rotateTower = false`).
- MOAB / ZOMG Takedown (`MoabTakedownModel` + `TargetGrapplableModel`): the strongest hookable MOAB-class bloons (one hook each, a ZOMG 3 for Pirate Lord's 6; not the excluded tags) are destroyed whole at once for their pop cash x2 (HYPOTHESIS P-TAKEDOWN; the reel-in isn't animated).
- Sub: First Strike Capability (instant marker, then a target-only 10,000 hit and a 350 blast via `CreateProjectileOnExpireModel`); Pre-emptive Strike fires the same kind of strike at every MOAB-class bloon the round sends as it appears (HYPOTHESIS P-PREEMPT); Sub Commander gives Subs in range (itself too) +4 pierce and x2 damage (HYPOTHESIS P-SUBCOMMANDER).
- `FilterAllExceptTargetModel` sub-projectiles hit only the target they inherit.
- Not yet: the Mermonkey's Heat, totems, Trance detonating damage over time, the sinking / surfacing animation, the hook reel-in.

**Hero abilities, as of now:**
- Generic pieces: `ChangeDamageTypeModel` / `MutateProjectileOnAbilityModel` boost the hero's projectiles for `lifespanFrames` (Armor Piercing Shells: pops Black, +9 to Ceramic and MOAB-class; Frostbite partly); `AbilityCancelLeakModel` (Quick Thinking) stops leaks costing lives.
- Captain Churchill: MOAB Barrage puts 200 damage every 0.5 s for 8.8 s on the 10 strongest MOAB-class bloons (HYPOTHESIS P-BARRAGE).
- Benjamin: Biohack gives the 4 nearest towers +1 damage for 6 s (STAND-IN P-BIOHACK, no shutdown after); Syphon Funding sends each round bloon one rank lower for 10 s (STAND-IN P-SYPHON, no extra cash).
- Adora: The Long Arm of Light: x2 range, x1.5 projectile radius, pops every type, for 10 s.
- Etienne: Drone Swarm (4 more drones for 22 s), UCAV (18 s), as timed sub-towers with their models.
- Admiral Brickell: Mega Mine is placed where you click and explodes for 4000 in a 100 radius when a bloon reaches it (any spot, not only water); Blast Chain speeds up her attacks x5 for 15 s (all attacks, STAND-IN).
- Sauda: Sword Charge sends its projectile (pierce 400, 30 damage) back along every track from its end at its `TravelAlongPathModel` speed (generic track travel for projectiles). Leaping Sword's impact lands on the lead bloon after `landDelay` (STAND-IN P-LEAP: the game's target choice isn't reversed).
- Paragon display levels: the model switches at the game's display degrees (degree / 20 for the 5 levels, `ParagonTower.GetDisplayDegreeIndex`), using `displayDegreePaths`.
- Not yet (each runs its own runtime logic, still to reverse): Silas's Frozen Cascade (`FindTowersToAffect`) / Frozen Burial (`GeneratePositions`), Striker's Artillery Command (damage mutator) / Target Focus, Rosalia's Flight Boost, Striker's Target Focus / Artillery Command, Adora's Blood Sacrifice, DanDMonke's Magic Barrier, Churchill's armor-piercing projectile art.

**Maps, as of now:**
- `tools/assets/export_all_maps.sh` exports every map scene in the game (81 of 88 export cleanly; a few fail on their scene model), each with its tracks, areas and 3D scene. N / Shift+N cycle through them before anything is placed.
- Each `entrySplitter` is exported as its own track (`map.json` "paths", with its active flag). A round's bloon spawns on track `emissionIndex % count`, from the game's code: `Spawner.Emit` calls `Splitter.SpawnRout(emissionIndex)`, which returns `activePaths[k % count]`. Same-frame emissions spawn in list order. Children stay on their parent's track. - Splitters (from the scene's Path components): a path with no entry splitter is a spawn path, except the boss path (STAND-IN P-BOSSPATH: by name). At a path's end, a bloon with an exit splitter goes on to that splitter's next path, round-robin with one counter per splitter, as `Splitter.Rout` does, and keeps its overflow distance. Targeting ranks bloons by total progress across the paths they've run (`Bloon::progress`). Balance, for example, now splits its two lanes into its middle paths in turn.
- Paths are exported in the scene hierarchy's depth-first order, the order `GetComponentsInChildren` gives (INFERRED as the order of the game's path list, which `Splitter.Init` builds its active paths from).
- Still HYPOTHESIS P-MULTIPATH: `GetSpawnPathsForRound` itself isn't reversed, Pushing a bloon back past a path's start routes it back through the entry splitter (Path.DistanceToPath, verified; the entry splitter's round-robin counter is an assumption).
- Fixed: maps whose export has no terrain extent (Balance, Alpine Run, Blons...) failed to load, and the game silently used Monkey Meadow's data instead. They load now (world bounds stand in), a bad `map.json` is reported, and `every_map_loads` checks every exported map. The game's own path building and split rules aren't reversed yet. Some maps have many path objects (Balance has 13) that may be branches of one route, so maps other than Monkey Meadow and Logs are experimental.
- Not yet: moving map parts (Geared, Sanctuary...), map-specific objects and removable obstacles, water on every map that has it (it works where the exporter finds the water areas).

**Paragons, as of now:**
- All 14 Paragons, from their TowerModels and "<Tower> Paragon" upgrades. When the towers of a kind together have a tier 5 on every path, a tier-5 path's upgrade button offers the Paragon. Buying it sacrifices every other tower of that kind.
- Degree, from the game's code: each sacrificed tower gives power. That's pops / 180; below tier 5 also its money spent / (paragon cost / 20000); and 6000 for each tier 5 from the fourth on. Each kind is capped (125,000 / 80,000 / 50,000; the tier-count kind is capped at 0 in this version). The degree is the number of `powerDegreeRequirements` met. The degree changes the stats as `PowerDegreeMutator.MutateTower` does: attack speed +round1(sqrt(50 x (d-1)))%, pierce +trunc((d-1)%) + 0.1 x (d-1), damage +(d-1)% + (d-1)/10, and one more step of each at degree 100. Ability cooldowns scale with the attack speed.
- STAND-IN P-PARAGONPOWER: the pops count stands in for damage dealt, cash earned (farms) isn't counted, and paying extra cash for power isn't offered. The paragon models are exported per tower (`tiers/index.json` "paragon").

**Difficulties, as of now:**
- Easy, Medium, Hard, Impoppable and CHIMPS from `data/modes.json`. Each applies its starting cash and lives, sell share, start and end round, round bonus, locked towers and selling rule. Impoppable and CHIMPS take Hard's mods underneath their own (HYPOTHESIS H-MODE-2).
- Bloon speed: `GlobalSpeedModModel` (speed x mult + add on every bloon, verified from the game's code) now applies in the playable build too. Medium is x1.1 and Hard x1.25; before this, the prototype ran Medium bloons at x1.0.
- Prices: the cost multipliers add up as `GlobalCostModModel.Mutate` does (Hard +0.08, Impoppable +0.08 +0.12). The price is then (1 - discounts) x cost, rounded the way `TowerManager.GetTowerCost` does: x / 5 rounded half to even, x5, in f32. That gives $170 / $215 / $240 for a Dart Monkey on Easy / Hard / Impoppable, like the game. Every price (towers, upgrades, Paragons) goes through it. The old rounding rounded halves away from zero.
- Easy's round-40 MOAB has (int)(200 x 2/3) = 133 health (`SetHealthForBloonModModel`, from the game's `SetMaxHealthOfBloonBehavior.OnBloonSpawned` and its mutator).
- Not yet: the other game modes (Primary Only, Deflation, Apopalypse, Half Cash...), Monkey Money.

**Paragon look:** the Paragon is drawn from its tower display (`paragon`) plus its attack display (`paragon_top`, from `ParagonTowerModel.displayDegreePaths`). Both models' outlines are drawn under both bodies. Untextured quads in models are glow planes, which are left out for now. Its projectiles come from the display plan like other tiers.


### Original menu scene extraction

Run `tools/assets/export_frontend.sh GAME` after the normal local extraction. It includes
read-only menu catalogs and GUI metadata; all outputs remain under `local_extracted/`.
Hero variants are indexed by their original Sprite GUID, and their full Sprite rectangles
are preserved. This matters for same-name round / rectangular icons and transparent padding.

Frontend capture names (`RRBTD6_FRONT`) now also include `knowledge`, `knowledge_tree`,
`social`, `powers`, `achievements`, `store`, `trophy_store`, `legends`, `mailbox`, `loading`.
Menu services and account progression are unbound; the extracted catalogs are visual shells.
Screens and textures load on demand. Off-screen catalog graphics do not load their textures.
Home-world motion is extracted by `export_home_animations.py`: continuous source clips
load on a worker thread, and the windmill uses its original rotation-script parameters.
Menu-only previews never unlock achievements or buy cosmetics. Catalog prices, ownership,
filters, online services, and the remaining world interactions are not yet fully bound.
Cloud motion retains the original cubic curves (about 13.5 KB for both long loops) and
updates cached scene geometry in place, preserving materials without a long vertex bake.

Menu regression capture options: `RRBTD6_LOADING_CAPTURE_DIR` records opening/opaque/closing
map-load phases; `RRBTD6_POWER_PREVIEW=GlueTrap` chooses a Power portrait;
`RRBTD6_ACHIEVEMENT_SEARCH=moab` previews filtered achievements. These only affect local
verification. `RRBTD6_TIMING=1` reports CPU draw time separately from frame pacing/presentation.
Animated Power portrait assets are extracted by `export_power_portraits.py` (included in the
frontend extraction recipe), and loaded only when selected.


### Live event/menu follow-up (2026-10-24)

- Home Race opens the current Race detail directly; its timer uses the live event name.
- Profile veteran rank replaces the normal rank display. Map tiles use native cumulative
  bronze/silver/gold backgrounds and the black-CHIMPS flag for the hematite background.
- Daily Play launches a local run using the downloaded map, mode, start/end rules,
  tower counts/upgrade caps and supported bloon speed/health/camo/regrow-rate modifiers.
  This reuses purchase limits without enabling Race concurrent-round scheduling.
  Co-op Daily is explicitly Solo Practice. Custom round sets and
  non-default obstacle multipliers remain rejected. No completion/reward upload.
- Original Social Season overview is linked from Home, Events and Social, with live
  season/theme/end time and native Teams/Perks navigation. Artwork comes from the local
  asset preparation script, not a network request per frame. Overview bindings update
  once a second; unknown aggregate/team scores use unavailable states, not sample scores.
- Friend highest-round overlays, Social Season Goals/details and live team scores still
  need their data/navigation wiring. The overview is a framework, not full social sync.
- Equal-depth bloon sprites and equal-progress MOAB models now tie-break by persistent
  bloon ID, avoiding reorder churn when the simulation swaps removed entities. This is
  not yet a complete audit of overlapping model depth behavior.

Prepare additional native season artwork with `export_social_season_art.py GAME EXTRACTED`
(or `export_event_ui.py`). Generated artwork remains ignored and local.


### Priority change: base game and co-op (2026-10-24)

Event/season-mode expansion is paused at the user's request. Private co-op now has
an initial **per-player-wallet gameplay prototype** over encrypted iroh connections with
automatic direct/relay transport. In Standard / Free For All, players ready up and
the host clicks Start. Two and four rendered clients passed relay-only placement,
upgrades, round start, combat and selling with matching diagnostic state. A shared
seed prevents local wall-clock initialization from choosing different freeplay rounds.
Tower commands use stable IDs and owner checks; each player has an independent pointer.
Pause/options continue pumping simulation and Home leaves the room.

Purchases, refunds, producer income and loan repayment use separate player wallets;
ordinary pop income is divided between players. Special income ownership still needs auditing.
This is not complete original co-op parity: individual heroes,
sacrifices/paragons, area restrictions, restart and reconnect remain
unfinished. Full cross-platform determinism and all-tower coverage are unverified.
See [the co-op notes](../coop/README.md) for limitations and reproducible tests.

The original co-op player panel now exposes peer balances, donations and cash
requests. Transfers are applied in the ordered command stream and cannot spend
another player's wallet or repay the recipient's loan. Panel motion uses the
exported Animator curves. Model prefetch retains at most four staged models;
loading advances a rolling window instead of staging the entire tower/bloon set.
Unused fallback base models are no longer loaded beside every warm tier model or
automatically between rounds. Towers in play/in hand still load their fallback
models when needed. Full-texture four-client co-op now passes the donation smoke
test; warmup private memory measured about 2.0 GB instead of 3.2 GB per client.

Identical animation payloads now share immutable frame storage. The key includes
the encoding and exact quantization bounds, so equal bytes with different bounds
stay separate. Hashing runs during staging; frame interpolation has no cache lookup
or hashing. Concurrent staging publishes one shared allocation. Five locally
exported Desperado tiers share 21.4 MiB rather than keeping five copies (85.8 MiB
avoided in that test). Savings depend on which duplicate tiers are loaded together.

UI sprite catalogs share uploads for byte-identical PNGs using the same linear,
non-mipmapped sampler. Each sprite retains its own borders and pixels-per-unit.
Weak cache entries preserve existing asset lifetimes. Preload scheduling, texture
resolution, sampling and simulation are unchanged by this sharing step.
`RRBTD6_SHARED_ASSET_CHECK=1` checks real UI upload sharing after GPU initialization;
`cargo test --profile game -p btd6-game shared_exported_animations -- --ignored --nocapture`
checks the local exported animation group.

Map-transition loading now continues drawing the original LoadingUI while CPU
jobs prepare the simulation, map images, UI trees, sprite indexes and font atlases.
The render thread polls workers instead of blocking on joins. Animated-model and
map uploads/mesh setup yield with a 3 ms work budget; fonts and HUD setup also
yield between stages. Large RGBA textures transfer in row spans of at most 1 MiB;
the completed texture is exposed only after all pixels are uploaded. The opt-in
shared-asset check verifies multi-chunk uploads by reading back every pixel.
Loading presentation is capped at 120 FPS (or the user's
lower cap) to leave CPU time for preparation. Preloaded assets and image quality
are unchanged. A single driver upload/compile can still exceed the work budget.
`RRBTD6_TIMING=1` reports loading draw intervals and unusually long work slices.
The baseline transition showed a 145 ms gap. The final single-player transition
measured a worst interval of 18.8 ms with no intervals over 33 ms; hardware/driver
load still affects these numbers.

Animated bodies and outlines share their immutable triangle-index arrays rather
than allocating/copying them for every pose. Static transformed-model draws also
borrow the scratch vertices and original indices. Submission keeps the same
triangles, materials and outline/body order. `RRBTD6_RENDER_CHECK=1` compares the
original mesh submission with the borrowed path on eight real Dart tier poses
(idle/attack, blinking, selection outline, rotation and tilt), requiring exact
GPU pixel equality. It also alternates warmed submission microbenchmarks. Local
results reduced that CPU submission cost by about 10–16%; this is not an overall
FPS or GPU-time measurement. Normal play performs no comparison/readback.

UI text nodes retain one wrapped/autosized layout each. Exact text, resolved font
identity/metrics, box dimensions, case, spacing and sizing settings invalidate it;
color, alignment and animation still apply each draw. No simulation scheduling or
render resolution changes. `RRBTD6_TEXT_CACHE_CHECK=1` runs 12 real-font GPU pixel
comparisons against fresh layouts, including changed text, fonts, fallback fonts,
resizing and visual styles. A local alternating layout-only benchmark measured
31.6 microseconds fresh versus 0.03 microseconds cached per unchanged wrapped label;
this is not an overall frame-rate measurement. Normal play performs no readbacks.

Shared rendering work: common billboard, flat-sprite and line quads submit stack
geometry rather than allocating vertex/index vectors per draw. MOAB ordering and
rank buffers retain their capacity between frames; comparisons and painter order
are unchanged. `RRBTD6_RENDER_CHECK=1` additionally compares six sprite submissions
pixel-for-pixel with the original owned-mesh path.

Particle definitions decode/upload a repeated texture path once, sharing the
texture among their independent systems. This applies to all effect prefabs;
particle counts, timing, random sequence, materials and draw order are unchanged.
No global cache retains effects after their previous lifetime. The opt-in
`RRBTD6_PARTICLE_TEXTURE_CHECK=1` checks GPU texture readbacks and filtered sampling
against per-system uploads. Its six-system fixture uses two uploads instead of
six, saving 36 MiB of texture storage. `--trace-loads` on the frame runner enables
`asset-stall` logs for loading operations taking at least 2 ms.

Animated poses transfer their computed vertex buffers directly into unsplit draw
chunks, instead of copying them from model-local scratch buffers. Released buffers
are reused through direct size buckets (32-vertex increments), limited to 4 MiB
and 256 entries. Live meshes own their buffers until dropped. Split meshes retain
the original vertex remapping and all pose arithmetic is unchanged.
`RRBTD6_RENDER_CHECK=1` also compares old/new pose vertex bits and rendered pixels.
`--suite --same-build-ab --ab-feature pose-reuse` compares original scratch/copy
behavior against reuse in the same executable, isolating parallel source changes.
The benchmark runner records Windows whole-process CPU seconds and peak private
commit/working set; these include startup and warm-up, and are not gameplay CPU
utilization percentages or dedicated GPU-memory measurements.

Startup sprite storage: decoded bloon/icon images are released after upload, and
fallback icon textures use the same content-addressed, linear-filtered uploads
as UI sprite catalogs. Both 2D and 3D paths retain their original pixels. The
local data set releases 25,235,484 bytes of CPU image data instead of keeping it
for the main loop's lifetime. `RRBTD6_SPRITE_STORAGE_CHECK=1` verifies all loaded
icon GPU pixels and shared handles. `--suite --same-build-ab --ab-feature
sprite-storage` compares this against the old retained-images/separate-upload
path in the same executable.

Animated-model shadows share mipmapped GPU uploads by dimensions and decoded RGBA
content. Weak ownership releases a shared upload when its last model unloads;
model shadow positions, sizes and draw order are unchanged. Ordinary loading
does no GPU readbacks. `RRBTD6_SHADOW_TEXTURE_CHECK=1` checks exact GPU pixels,
three mipmapped sampling sizes, sync/cooperative reuse, distinct-content isolation
and weak lifetime. The local base/final-path-tier shadow workload contains 74
images but only two distinct uploads: 25,864,776 -> 699,048 mipmapped RGBA bytes.
This is a texture-resource count, not a measurement of whole-process RAM or an
assumption that every exported tower tier is resident. Use `--suite
--same-build-ab --ab-feature shadow-textures` for repeated gameplay comparisons.

Renderer streaming storage: macroquad now reserves 5,000 vertices per draw-call
buffer instead of 10,000, keeping the existing 5,000-index capacity. Both static
and animated meshes use one triangle-preserving splitter/remapper, so large
exports fit without clamping. Sprite and particle batches already fit this size.
Consecutive compatible opaque map/menu parts keep their material bound; special
shader boundaries and transparent render queues retain their original order.
`RRBTD6_LEGACY_RENDER_STORAGE=1` restores the original capacities, static-array
duplication and material resets for same-build A/B measurements. `--suite
--same-build-ab --ab-feature render-storage` compares both paths. The narrower
`map-batching` feature isolates material-reset removal at the current capacity.

`RRBTD6_MAP_BATCH_CHECK=1` compares old/new submission pixels on Monkey Meadow,
Logs, Tinkerton and the main-menu world, including hidden-part variants, at a
fixed shader clock. Its fingerprint also matches across legacy/new capacities.
Sparse and multichunk index tests verify no triangle is dropped or reordered.
The local three-pair round suite measured about 47-90 MiB lower peak private
commit and 6-22% higher median average FPS; long frame-time spikes remain.
See `local_extracted/paragon-render-storage-ab/comparison.md` for conditions.

Immutable map geometry: the OpenGL world pass uploads plain opaque and
Gradient_3Color map meshes once and retains exact-sized GPU vertex/index buffers.
Consecutive compatible parts share a run, preserving primitive order and exact
gradient uniform bits. Animated water and other translated shaders retain their
original streaming path and transparent queue order. The main-menu world and
other mutable models continue streaming. Hidden parts fall back safely within
their affected run; viewport/DPI/render-scale changes need no geometry upload.
Buffers release with their map, and shader programs share weak ownership. Before
directly editing cached map vertices, indices or materials, call
`invalidate_map_geometry()`; `draw_map` requires an identity model transform and
an unclipped world pass. Unsupported backends/shaders retain normal rendering.

`RRBTD6_NO_STATIC_MAP_GPU=1` restores streaming for same-build comparisons;
`--ab-feature static-maps` isolates this change. `render-storage` now compares the
combined buffer/batching/cache changes against their original paths.
`RRBTD6_STATIC_MAP_CHECK=1` runs 20 exact GPU comparisons across Monkey Meadow,
Logs, Tinkerton and Adora's Temple, including special materials, partial hiding,
inset viewports, half-size targets, MSAA, and streaming geometry drawn afterward.
The audit checks that drawing allocates no replacement map buffers and unloading
releases them. No readbacks run during normal play. Monkey Meadow retains 391,748
bytes in eight immutable draws; the tested maps range up to 998,904 buffer bytes.

The benchmark runner freezes simulation JSON/map inputs and uses a private
preferences file, leaving live user settings untouched. Overrides are honored
only with both AUTOTEST and FRAME_BENCH enabled. Older executables without the
isolated-input marker are rejected rather than silently compared with live data.
On Windows it waits for a two-second compiler-free interval, polls for compiler
overlap during captures, and rejects/retries affected captures (five attempts).
Other desktop activity can still affect results. Frozen game data stays local
and git-ignored alongside raw captures.

Cloud reads remain enabled; uploads remain disabled. Ninja Kiwi's published flagging
policy includes third-party modifications/mods and restrictions on public co-op.
A correct serializer or accepted storage request does not establish anti-cheat safety.
Before enabling writes: preserve encrypted server backup, retain same-account identity,
produce a reviewable save delta, and verify download-after-upload. Never remove or
rewrite server account restrictions, fabricate earned rewards, or substitute another
account's save. No guarantee of an unflagged account can be made for this client.

Co-op asset preparation: `export_coop_ui.py GAME EXTRACTED`. Source lobby dropdowns
reset incompatible modes when difficulty changes; area previews remain configuration
only until area restrictions are implemented. Cloud uploads remain disabled.

Combat asset preparation: the renderer reads the current round's queued bloon
types and descendants without emitting anything or advancing RNG, then prepares
their complete model/damage-state families. Four staging jobs, including the
one cooperative model upload, are allowed at once. Each frame polls an upload
instead of joining an unfinished decode worker. An immediately required model
can take over its partially completed upload; fallback loading remains available.
The placed towers' current weapon projectiles, contact/exhaust children,
alternates, crit displays and orbits also prepare their art and particle effects.
Particle JSON/images decode off-thread, with cooperative texture uploads;
definition/material preparation creates no instances and consumes no particle RNG.
This moves first-use work into earlier available frames, without reducing texture
quality, geometry, particle counts or simulation steps. It cannot guarantee every
newly placed/upgraded tower or unexpected display has finished preparing in time.

`RRBTD6_PREFETCH_CHECK=1` compares real MOAB/BAD GPU pixels for background loads
and immediate upload takeover. `RRBTD6_PARTICLE_TEXTURE_CHECK=1` also checks the
cooperative effect-loading path against original texture pixels and sampling.
`--suite --same-build-ab --ab-feature background-assets` compares preparation
against the prior four-job startup prefetch in the same executable. Normal play
uses no verification readbacks. `RRBTD6_NO_BACKGROUND_ASSETS=1` disables this work
for those comparisons; cold first-use stalls remain in the reference captures.

Gameplay demand loads must not await a worker/cooperative upload directly in the
main draw loop: macroquad clears/presents each time the root future yields, even
when no complete scene was submitted. This reproduced three blank presentations
after late placement on Monkey Meadow. `load_work::complete_asset` finishes an
unexpected required asset inside the current frame; it is only for asset futures
that use CPU workers/cooperative budgets, never timers or `next_frame`. Regular
background preparation and loading-screen work remain incremental. A cold asset
can still cause a foreground pause, but cannot expose a partly drawn/cleared frame.

Tier art already loaded for the placed tower/placement preview also supplies its
shadow. Root base models/animations are loaded only when the tier display is
unavailable; `RRBTD6_KEEP_FALLBACK_MODELS=1` restores redundant base loads for
`--ab-feature fallback-models` memory comparisons. Tower-change detection reuses
its signature vector, and shadow/body passes reuse their resolved display keys.

`RRBTD6_PRESENTATION_CHECK=<local-directory>` audits physical presentations,
including those omitted by the logical frame benchmark. It writes up to eight
blank-frame screenshots and reports the total. With AUTOTEST, a
`RRBTD6_LATE_TOWER=BombShooter:45` fixture places a cold tower after the map is
already visible. `RRBTD6_LATE_PREVIEW=1` also exercises its in-hand model/shadow
before placement; `RRBTD6_PRESENTATION_STRICT=1` fails on any blank gameplay
presentation. This deliberately expensive GPU-readback audit is opt-in only;
do not enable it during frame-time benchmarks.

Upgrade prediction retires completed jobs for old selections/tiers, so obsolete
staged models cannot occupy every prefetch slot indefinitely. Unfinished workers
continue counting toward the four-model limit until they finish; retargeting does
not detach them. Prediction paths are reused until selection/tier/placement changes,
and only currently legal next upgrades are requested. This includes hero levels
above five. Discarding a staged texture releases its unuploaded decode claim, so a
later prediction can decode it again; uploaded/session-cached claims remain held.

The selected tower's next legal upgrade effect also prepares through the existing
single-effect cooperative loader. Normal/tier-five and hero/level-20 effects remain
separate, with no particle instances or random draws created during preparation.
Immediate purchases can still require foreground loading if preparation is not done.
`RRBTD6_NO_PREDICTIVE_RETIRE=1` restores the prior prediction path for A/B checks.
`tools/benchmark_frames.py --upgrade-case --frames 150 --warmup 10 --pairs 5
--ab-feature prediction-retirement` measures switching from Dart to Super, then
buying Super Range, including the demand-load frame. It uses private preferences
with a 120 FPS cap to give both variants the same preparation opportunity; its
average FPS is not an uncapped throughput comparison.

`RRBTD6_UPGRADE_PREP_CHECK=1` checks 80 exact particle-animation GPU frames and RNG
states across all four upgrade-effect families. `RRBTD6_UPGRADE_CASE=1:40:1:100`
is the AUTOTEST-only fixture (tower index, selection frame, path, purchase frame).
Verification readbacks are disabled during normal play and timing captures.


Capped frames in gameplay, the front end and pause menus now use an absolute
presentation schedule. Submission/swap time after the limiter counts toward the
next interval instead of being added to every frame budget. Slow frames reset the
schedule without a catch-up burst; changing caps or selecting Unlimited resets it
as well. Windows uses one private high-resolution waitable timer rather than a
busy spin. Unsupported systems retain the previous sleep/spin fallback. The timer
handle is released with the pacer; no global timer-resolution setting is changed.
Simulation steps and render content are unchanged. The loading-task loop keeps its
existing independent pacing.

`RRBTD6_LEGACY_FRAME_PACING=1` restores the prior limiter for comparisons.
`tools/benchmark_frames.py --upgrade-case --frames 480 --warmup 10 --pairs 3
--fps-cap 120 --ab-feature frame-pacing` compares the same executable using private
preferences. `--fps-cap 0` is the uncapped control. The independent, ignored
`frame_pacer::tests::benchmark_limiter` models a presentation delay and reports
limiter timings; it does not measure game rendering speed.

## Startup loading

The initial plane screen now runs eight real loading jobs before the title: sprite
decoding, map geometry, the initial tower mesh and animation, pop textures, the
two cursor textures, and home-world geometry. It draws the original MainText and
SubText nodes with the native English loading messages and `Step X of 8...`. CPU
sprite decoding runs on a worker; asset futures yield to the animated loading
screen. Existing lazy tower/round loading is preserved. The original open/close
animations play, with no fixed idle hold or minimum frame count for a finished
step. The title intro renders the native 3D Monkey Ace on its exported flight
path alongside the original bloon/pop timing. Its scripted propeller rotation
is baked into a small local animation loop and loaded during the final startup
job. Startup extraction regenerates these assets with output version 3.

`RRBTD6_STARTUP=1` also forces the front end for an autotest. With
`RRBTD6_TIMING=1`, each completed startup job is logged. Set
`RRBTD6_STARTUP_PROGRESS_OUT` to an existing directory to capture each stage's
first presentation; normal launches perform no screenshot readbacks.

Title world/control textures are now prepared during startup step 8, before the
short logo/monkey pop-in. Four CPU decode jobs at most run together, GPU uploads
yield in bounded slices, and prepared textures enter the existing shared cache.
This preserves the original pixels, animation curves and playback speed while
avoiding synchronous PNG decoding at the moment those sprites become visible.
`RRBTD6_NO_TITLE_PRELOAD=1` retains first-use loading for an A/B reference;
`RRBTD6_STARTUP_FRAME_TRACE=1` logs title-frame intervals, and
`RRBTD6_UI_MISS_TRACE=1` identifies remaining first-use sprite loads.

The title exit now composites its original transparent-sky fade over the home
world. Background input stays disabled during the exit, and the main-menu UI
opens on the same frame the title finishes instead of presenting an empty
closing frame. `RRBTD6_STARTUP_PRESS_START=1` automates Start after the intro
when `RRBTD6_AUTOTEST` is set. Pair it with `RRBTD6_PRESENTATION_CHECK` and
`RRBTD6_PRESENTATION_STRICT=1` to audit the physical handoff presentations.
`RRBTD6_STARTUP_TRANSITION_OUT` captures the beginning, middle and end of the
exit into an existing directory.

The title player label polls the same asynchronous cached profile as the home
menu. TMP font-color animation channels now apply the original name reveal after
the plane intro (4.5167�4.7667 seconds), including older exports with hashed bindings.
Clicking the signed-out prompt or current name opens the account panel.
Typing/paste and Sign in/Enter launch one explicit login on a worker, followed by
read-only profile downloads. The code travels over child stdin, never argv/logs;
sessions stay Windows-encrypted. New online login is enabled for that child only,
so opening the game does not enable background service polling or uploads.
Back/Escape closes the screen; an in-flight result still updates the cached UI.
Autotests cannot submit real credentials. `RRBTD6_STARTUP_ACCOUNT=1` with
`RRBTD6_AUTOTEST` opens the panel after the title animation for visual verification.

Settings > Account and the title account label now share one account panel and
background job. Signed-in state shows the username and Log out; signed-out state
shows one code field and Sign in. Back/Escape closes the panel without losing an
in-flight result. The Settings background is captured once rather than redrawn
under the modal. Logout removes this client's local sessions/pending code and
persists a signed-out marker, preventing saved snapshots from restoring login.
Downloaded progress stays encrypted and untouched. A successful new login clears
the marker; account-dependent menu trees are rebuilt after an account change.
`RRBTD6_SETTINGS_ACCOUNT=1` with `RRBTD6_FRONT=settings` and `RRBTD6_AUTOTEST`
opens the Settings account panel for offline visual checks.

The account card now presents local identity and optional Ninja Kiwi sign-in as
separate sections. The network notice identifies account downloads as well as
sign-in and GitHub client updates; it does not promise account safety.
Local profiles persist in `local_extracted/local-profile.json`, with an atomic
replacement and backup copy. Names (1�24 characters), installed avatar/banner
choices, highest completed round and win/loss totals are local-only. Round/end
saves run off the render thread, and downloaded Ninja Kiwi profiles remain
separate. Signing out restores the complete local profile instead of an empty
account object. Mid-game simulation snapshots/resume are not implemented.
`RRBTD6_LOCAL_PROFILE=1` with `RRBTD6_AUTOTEST` suppresses the encrypted-account
reader for offline local-profile visual checks without signing the user out.

Native menu motion now includes the map screen and its separate MapGroup card
sequence, map page/tab Next motion, difficulty/mode selection, Settings,
Advanced, Hotkeys and monkey-selection groups. Opening clips start their clock
after the first draw; a bounded group of active-content sprites is primed before that presentation
so first-use decode is not counted as time already elapsed in the entrance.
The same first-presentation rule applies to lazily opened pages such as the shop
and profile. Button feedback now accepts original GlobalButtonHoverAnimation,
GlobalTabAnimation and MapButton controllers as well as GlobalButtonAnimation.
Profile Public Stats toggles the selected-stats preview; local stat visibility
and cosmetics can be saved locally. This does not write cloud profile settings.

Menu entrance priming caps at 64 distinct sprites per screen and 16 per mask,
so opening a catalog does not decode its full offscreen inventory.

The profile gear opens the shared account editor. Very large imported veteran
ranks fit inside their original label bounds instead of overlapping the XP bar.

Remaining profile gaps include Legends-specific category rows, named-monkey context
and live social/accolade sections. This pass restores base profile data, local
identity/customization, the public-stats preview and native entrance motion; it
does not claim complete original-game profile coverage.


Daily chest/menu pass: the home chest has its exported native collider and opens
DailyRewardUI with the native DailyRewardsAnimation opening, spinning selected
reward and TickDropAC claim preview. LootThemes theme 2 uses MediumBaseLootItem.
The five-card calendar reads the server cache's reward cycle and reset offset,
and the account's dailyRewardIndex / nextDailyRewardDateTime. The date is on the
game's shifted daily clock (UTC minus the reset offset), not the PC's timezone.
Opening today's reward previews the next reset in this menu session only. No
currency, powers or Instas are awarded and no profile/cloud claim is written.
The Bloons TV mesh subtree and its click volume are removed by user request.

The profile dropdown uses its native TMP template on the root canvas, consumes
outside clicks/Escape and resets the stats scroll on selection. Main Game,
Rogue and Frontier show their separately imported basic counters. Legends-only
rows and category-specific public sharing remain incomplete.

Imported savedMaps metadata drives MapButton.Continue and ContinueGamePanel.
Starting fresh shows a CommonPopup and retains the imported official save;
full simulation restore and real local-save overwrite are not implemented.
Friends' map records have bindings for the native badge/panel and a hide option
under Advanced > UI, but the service adapter has not supplied actual records.
Missing records stay hidden rather than showing the prefab's sample scores.

The extracted Maps entrance has six staggered scale channels (MapGroup.Open,
0.7 s); it does not contain horizontal position channels for those six tiles.
This confirms the clip used, not complete parity with every native transition.

Animation disk storage: the current client reads verified lossless .bin.z files
as well as legacy unpacked animations. Decompression runs during model staging;
frame sampling is unchanged. tools/assets/pack_anims.py migrates existing models
and the launcher performs this at extraction finish step v3. See tools/EXTRACTION.md
for measured size savings, load-time costs and old-client compatibility.

The follow-up storage format also accepts .bin.dz temporal prediction. This is
expanded once in place during staging, with identical sampling data. Finish v5
uses the matching executable in --canonicalize-anims mode to store legacy float
animations at the exact precision the old runtime already used. Rebuild the
client before running the developer extraction script. Whole identical frames
can share RAM; frame offsets resolve once per pose. See tools/EXTRACTION.md for
measured sizes, CPU costs, validation and the still-open total-size target.


Audio storage: extracted audio now uses lossless PCM16 FLAC (audio step v2),
with legacy WAV reads retained. Music decoding and shared-buffer preparation
happen on workers; the existing mixer/resampler consumes the identical prepared
samples. The queue transfers an Arc instead of copying a whole track on the
mixer thread. Two decode jobs and one next-track buffer bound preparation.
Cold SFX can still wait on first use, and decoded playback PCM is unchanged in
size. Local audio: 1.990 -> 0.845 GB, every source sample verified before removing
the WAV. Build/install the new executable before migrating older local assets.
