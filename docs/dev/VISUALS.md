# Tower visuals status

What the clone draws for towers compared with the game, and what's left. Audits:
`tools/assets/audit_tier_displays.py` (every upgrade combination has a model folder with mesh,
textures, an idle and an attack clip) and `tools/assets/audit_visuals.py` (projectile displays,
effect names, model behaviours the data builder never reads).

## Done (2026-11-15)

- **Tier models**: every base tower has a model for all 64 upgrade combinations, with mesh, textures
  and clips (heroes: one model per stage). No missing or broken folders.
- **Ability effects**: `CreateEffectOnAbilityModel` (802 in the game data, 96 prefabs, all exported)
  now plays on the tower when an ability is used (ability `fx`, `Game::use_ability_effects` ->
  `place_fx`).
- **Tier upgrade effects**: a tier's own `CreateEffectOnUpgradeModel` effect (Etienne's radar pulse,
  Power towers) instead of the generic one where it differs.
- **Nested projectile displays**: `build_display_plan.py` now plans every display GUID a tower's
  game.json data uses (sub-projectiles, pets, sentries' death blasts), resolved by the catalog's prefab
  name. Newly exported: Skywarden's ice crystal / ice shard / lightning rod / Windlash arrow, Mermonkey's
  songs 2 and 3, the paragon sentry's blast, Magus Perfectus' unpopped phoenix bloon.
- **Extra tower displays** (`TowerModel` DisplayModels besides the body, tier `extras`): Mermonkey's
  bases (all tiers; meshless, played as effects), Ice Monkey Arctic Wind's snow, Druid's Ball Lightning
  base and Superstorm cloud, Tack 5xx's Hot Spot glow, Silas' snow. Exported by
  `export_pending_models.py` (`DisplayModel.display`), drawn by main.rs (mesh: `extra_models` with the
  idle clip; meshless: looping tagged effect).
- **Range-scaled displays** (`LinkDisplayScaleToTowerRangeModel`, tier `range_links`): those extras and
  the Inferno Ring loop are drawn at range / displayRadius (Tack 510 / 520's ring, Arctic Wind with
  more range).
- **Attack animations**: towers whose attack clips aren't named "...Attack" never animated their
  attacks. Now any non-idle "Attack" clip counts, and several of one family alternate per throw:
  Mortar's Artillery Battery (Attack1-3), Dartling's Hydra / MAD / Rocket Storm, Engineer's traps,
  Buccaneer (AttackL / R / Both), Druid's lightning, Beast Handler, Sauda, Pat Fusty, Admiral Brickell,
  Psi, D&D Monke, the Nautic Siege Core...
- **Attack clip by animation state** (2026-11-17): a throw plays the model's
  `MonkeyAnimationController.animationStatesByIndex[n]` (WriteHighQualityAnimationIndices @0x1815272b0:
  animationStates, then placementConfigs, attackIdleConfigs, IdleVariants; states.json,
  `export_anim_states.py`; Scene.PresentModels passes the pending state to UnityDisplayNode.set_AnimationState
  and clears it). n from Weapon.SetAnimation: `weaponModel.animation` (`anim_state`), CycleAnimation's current
  state at the throw's start (`cycle_anim`: + 1 per throw, Loop / Clamp past max, or the synced
  Next(min, max + 1) the clone already drew), or AlternateAnimation's `emissionCount % states + originState + 1`
  (`alt_anim`), a pending state replaced only by one at least as high (Tower.anim_state). Artillery Battery
  cycles 1-3: Attack3, Attack1, Attack2. export_animated.py now also exports every state's clip (role
  "state"; the Animator's clip list can lack them: Tack 204's OverDriveAttack, Druid's DruidSpearIdle /
  DruidSpearAttack, Silas, Desperado, Skywarden; those 72 tier models were re-exported locally; an existing
  install keeps its old folders, since export_tower_tiers skips exported ones). 2755 of 2778 weapon states
  resolve; the other 23 are past the end of their model's list (Village 5-x-x, some Buccaneer and Dartling
  crosspaths) and, as SetState, play nothing. STAND-IN: SetAnimation sets the state on the weapon's attack's
  display (mainAttack's with animateOnMainAttack); the clone gives the one state to the body and its attack
  displays alike. Pre-emptive Strike's launch sets its animationState
  (state 2, MonkeySub050MissileAttack; `preempt_anim`). Weapon.GetAnimationOverride changes nothing: the game
  data has no AnimationStateOverrideModel. LEFT (heroes only, not in the shop): EmissionClosestOffset
  (Admiral Brickell), SwapProjectile, ChangeProjectilePerEmit, DuplicateProjectilePerEmit; AttackBloonOnEntrance
  has no model in the data.

- **Paragon degree swaps** (`ParagonAssetSwapModel`, `ParagonAssetSwapPerDegreeModel`; paragon `swaps`,
  `per_degree`, `Game::paragon_display`): at its display level a Paragon's extras and shots use the
  swapped art from models/extra: Bomb's add-on and cluster / ISAB missiles, Sniper's platform, Glaive
  Dominus' glaives, Ascended Shadow's shurikens, the Tack Paragon's meteors. The Druid Paragon's cyclone
  levels (RootOfAllNatureCycloneT3-T5, Wrath T4) are exported as effects (export_particles.py; NoiseModule
  unsupported). The cyclone comes from Primordial Wrath's `ToggleEffectWhileAbilityActiveModel` (ability
  `toggle_fx`: the Wrath cyclone while active, the plain one otherwise): the plain one now loops on the
  Paragon, swapped by degree. STAND-IN: Primordial Wrath isn't simulated, so the Wrath cyclone never shows.

- **MOAB Takedown hooks** (`CreateRopeEffectModel`, takedown `rope`): the hooks weren't drawn at all;
  now chain links every 12 from 12 out along the tower-to-hook line and the hook at its end (Buccaneer
  x-4-x MonkeyPirateHook, x-5-x PirateLordHook). STAND-IN: from the tower's centre, not its eject point.

## Done (2026-11-17, animation clips)

- **Idle is state 0**: MonkeyAnimationController.SetState (@0x1815264c0) clamps to 0 and
  SelectIdleHighQuality (@0x181526070) picks between state 0 and the IdleVariants, so `AnimModel::idle_clip`
  now plays states.json[0]. The old "first clip ending in Idle" rule was wrong on 116 models: Druid 3xx /
  4xx bodies idled as Ball Lightning (BallLightningIdle instead of DruidSpearIdle), Desperado 030+ as "Idle"
  instead of IdleRifle / IdleShotgun / IdleRevolver, Chinook air units on their attack clip.
- **Missing state clips re-exported**: 136 models listed states whose clips were never exported (exported
  before export_animated.py added the "state" role, and export_tower_tiers skips existing folders). Now
  re-exported: every Heli air unit (HeliPilotIdle missing: it idled on HeliPilotAttack), Banana Farmer Pro
  (BFPro_Idle / Scoop / Throw: it played the plain Farmer's clips), Desperado (Idle), Druid
  (DruidAttackaLightning), Skywarden (SkywardenAttackAbilityShoot1), the Bomb Paragon, the SMS Pro beacon
  turret, Sheriff. `export_model.load_bundle` loads asset_references_assets_all from _bundle_cache on
  MemoryError (it aborted every batch). Audit: 10 left (Rosalia's weapon extras, Wererabbit, the fan Super
  Monkey's placement, the Heli / Farmer Pro base folders, which only hold the pad / base).
- **Attack restarts** (PlayableAnimator.Play @0x1815296e0): a new state interrupts the playing clip
  by the playing state's interruptibleBehaviour (0: equal or higher priority, 1: only higher, 2: any,
  else queued as the return state and played from its end) and restarts from frame 0. 1531 models'
  attack state is (priority 1, interruptible 0), so every throw restarts the attack clip at once; the
  clone queued it after the clip's end for all. export_anim_states.py writes state_flags.json
  ([priority, interruptibleBehaviour, looping] per state); `AnimModel::state_interrupts`. Scene.PresentModels
  (@0x18150e410) hands the node the frame's state then resets it to 0, and UnityDisplayNode.set_AnimationState
  (@0x181516280) acts only on a change, so a throw is a 0 -> n change. AnimationSpeed stays 1 (no sim
  caller found).
- **Attack displays follow the weapon's state** like the body (Rocket Storm's gun cycles
  RocketSwivelAttack01-04; by name it always played 01).
- LEFT: several models have more than one clip with the same name (Admiral Brickell's three "Idle"
  states); `state_clip` looks clips up by name, so a state naming a duplicate gets the first one.

## Display audits (2026-11-17)

- `python -I tools/assets/audit_displays.py`: every projectile display GUID in game.json vs
  models/projectiles/index.json. 3769 checked: all resolve (4 named differently in the catalog, 3 missing:
  Final Strike's fallout, the Fan Club "projectiles").
- `python -I tools/assets/audit_dropped_displays.py [--tower X]`: every prefab GUID each tower tier's
  model reaches in the game (sub-towers included, other towers and upgrades not) vs what the tier's
  game.json / display plan carries, grouped by the holding behaviour. Note: catalog_prefabs names one
  prefab per GUID and a GUID can have several (the Bomb Shooter's body GUID is listed as BombBlitz), so
  compare GUIDs, not names.
- `tools/.venv/Scripts/python.exe tools/assets/display_sheet.py <dir> [Tower]`: contact sheets of each
  tower's projectile art with game / clone names.
- FIXED: projectile CreateEffectOnExpireModel / CreateEffectOnExhaustedModel holding the effect as their own
  assetId (no effectModel) were dropped by `fx_list`: Monkey Sub 030+'s ExplosionLarge, First Strike,
  Mortar's Shell Shock explosions, Spike Factory mines, Wall of Fire's exit, Buccaneer skulls (152 refs).
  The 29 effects behind them are exported (export_pending_models EFFECT_FIELDS; export_particles loads
  asset_references from _bundle_cache on MemoryError).
- DONE: CreateEffectProjectileAfterTime (`after_fx`, main.rs: Tack Paragon meteors, missile pods, Great
  White thrash splashes; STAND-IN: isBuffedByRate's rate scaling not applied); ScaleProjectileOverTime's
  effect (`scale_fx`: Riptide's aura ahead of the projectile, growing); ability attack displays (Glue Storm's
  hose, build_display_plan `attack: -1`, drawn while a `Tower.temp` attack runs). The Super Monkey arm
  entries in the audit are catalog-name noise (401 / 410 are mapped).
- DONE (display hooks in game.rs, `place_fx` only): LineProjectileEmission effectAtStart / effectAtEnd /
  effectAtCollision on the plain line (emission `line_fx`; Emit @0x180c42910: Desperado's rifle line start /
  end, the Sniper Paragon's muzzle flash, beam hits up to maxCollisionEffects). STAND-IN: a new effect each
  Emit, not the one kept and moved. Bloon Trap's BloonTrapBloonCollect on each eat that doesn't fill it
  (eat `fx`; EatBloon.PlayEffect @0x180cba040 at the trap). STAND-IN: the game skips it while the last plays.
- DONE (2026-11-18): Druid 030+'s Jungle Vines (JungleVineEffect.Collide / Process @0x180cc23b0 /
  @0x180cc3140): game.rs `vine_fx` (display only) at each grab; main.rs plays the back (JungleVineGrow*) and
  front (JungleVineWrap*) displays at the bloon's display height, a stand-in bloon stepping down the game's
  static list (Red, Blue, Green, Yellow, Pink, Black, Zebra, Rainbow, Ceramic; White / Purple as Black, Lead
  as Zebra; INFERRED order: the .cctor decompile shows only Red at 0) every degradeStep frames, then
  animation state 2 (End) on both. STAND-IN: the stand-in draws between the bloons and the effects, not
  between the back and front displays; the clip-toggled particle systems (SqueezeEffect, PopEffect) don't
  emit (inactive at load).
- Animator states in effects (export_particles `animator_clip`, particles.rs): every state's clip with
  GameObject active (attr 5) and SpriteRenderer sprite-swap (attr 6) tracks, exit-time transitions (to Exit:
  the Entry state), AnyState jumps on the int parameter (`Particles::set_anim_state`, as
  UnityDisplayNode.set_AnimationState). mecanim_clip: PPtr curves sit after curveCount in the streamed data
  and take no float slot (the old layout shifted every later transform track in clips with sprite swaps).
  Only the vines are re-exported so far. Autotest: `RRBTD6_SPAWNFX=a,b` shows effects mid-map,
  `RRBTD6_FXSTATE=frame:value` sets their state.
- LEFT: Sniper 030+'s BouncingBulletTracer (CreateEffectFromCollisionToCollision @0x180c9ea30: spawned along the
  segment, stretched by length / effectLength): its prefab has no particles, so the streak stand-in stays.
- DONE (2026-11-18): Boomerang hand switch (FlipFollowPath.Flip @0x180c81de0): flipTowerDisplayX mirrors
  the tower's model (`AnimModel::mirror_x`, culled parts swap sides) and effectOnFlip
  (AmbiRangTransformation) plays at the tower on a switch (weapon `flip_fx`; main.rs watches each tower's
  hand). Autotest: `RRBTD6_FLIP=<frame>` switches every tower's hand.
- DONE: Desperado 5xx Blazing Sun muzzle flashes (TheBlazingSun.ProjectilesCreated @0x180c961a0: the
  ejectEffect under stacksToBurn stacks, the overheat one from there; weapon `blazing_fx`).
- DONE: MOAB Takedown hooks' HookHit at the snap (the hook projectile's CreateEffectOnContactModel;
  takedown `hit_fx`, game.rs hooks_step, display only).
- DONE: Dark Shift / Spirit Walk effects (StartDarkshift @0x180e22bd0 disappearEffect at the old spot,
  EndDarkshift @0x180e214a0 reappearEffect at the new one; darkshift `out_fx` / `in_fx`).
- DONE: Turbo Charge / Perma Charge projectiles (TurboMutator.Mutate @0x180e7ac90: each weapon's projectile
  takes projectileDisplay while the ability runs; turbo `display`, main.rs `proj_look` by uid at creation).
  The six ability rang / glaive displays exported (sprites + particle trails; export_particles
  `proj:<prefab>` adds one projectile's particles to proj_index.json).
- DONE: Ice Monkey frozen platforms (FreezeNearbyWater.FilterTower / ZoneEnter / CreateMutator
  @0x180d24fd0 / @0x180d258d0 / @0x180d24dd0: a land tower in a water area inside the zone gets a mutator
  whose DisplayModel is freezeAsset, ShowDisplayForTerrainModel water): `Game::frozen_platform`, tier
  `freeze_fx` (FrozenPlatform, Herald of Everfrost's, Silas'), main.rs keeps it under the tower.
- DONE: Pre-emptive Strike's launch effect (PreEmptiveStrikeLauncherModel.ejectEffectModel at the launch
  point, tier `preempt_fx`); Desperado's Vigilante buff display while the buff runs (buffDisplay,
  `Game::vigilante_active`, tier `vigilante_fx`).
- Re-exported with Animator states: the 58 plain / projectile effects with an Animator (BeastMergeFx, Sun
  Temple / True Sun God transformations, Magus Perfectus' phoenix...) and the tier_* effects of the 12
  towers that have animated ones (Ice Monkey, Mermonkey, Geraldo, Tack, Bomb, Desperado, Sub, Silas, Wizard,
  Ace, Corvus, Druid).
- DONE: ability support zones' displayModel on each tower they buff while they run (support `display`,
  `Game::zone_displays`): Thunder's Decree (ThundersDecreeAbilityOnTowers), Naval Tactics
  (NavalTacticsBuff), Pat Fusty's Rallying Roar (RallyRoarBuffFX, exported).
- Audit entries already drawn: Paragon investment orbs, Acidic Mixture / Berserker Brew bubbles, Sub
  submerge pulses.
- DONE: Legend of the Night's immunity effect (Immunity.SpawnEffects / RemoveImmunity @0x180e36a20 /
  @0x180e36740: effectModel at each Map.GetTrackEndPoints point while leaks are forgiven; ability
  `immunity_fx`). Unverified on screen: Tutorial's track ends off the visible map.
- DONE: CritRoll / CritRollWithDistance's display (Desperado's CritText, Projectile.ChosenCritDisplay):
  "Crit!" where a rolled crit lands (weapon `crit_roll_text`, display only). STAND-IN: once, at the hit
  that rolls it.
- DONE: Spirit of the Forest's vines (SpiritOfTheForest.AddObjectAtPosition @0x180d92f10): spirit_grow makes
  each object's three draws in ring order (as before, only taken: the same sequence) and keeps the object
  (list by band from the tower: closeRange / middleRange / far; entry (int)(count x d), rotation and scale
  min + d x (max - min)) in `Tower::spirit_objs`; main.rs shows each as its effect (the nine
  SpiritOfTheForestVines* prefabs, exported).
  The Druid Paragon's RootOfAllNatureVines* and the Spike Paragon's carpet (MMMCarpetOfSpikes1-4;
  SpikeParagonDamageZone.AddObjectAtPosition @0x180d8dd10: one list for every band, second draw unused)
  likewise. STAND-IN: the Spike carpet's rotation is its position's own (none in the clone's positions: 0).
- DONE: merged Beast Handler pets' looks (ContributionMutator.Mutate @0x181208b50: the display step with the
  highest percent <= (power - basePower) / (maxPower - basePower) once above 0; pet power `steps`, models
  from models/extra). Unverified on screen (needs a merge). LEFT: the steps' projectileDisplay swap.
- DONE: the Great White / Orca / Megalodon's held bloon (GwEffect `kind` at its `pos` while not exited) drawn
  as a bloon, beside the vines' stand-ins; the shark itself (CreateGreatWhiteEffectModel.display, exported
  animated to models/extra by export_pending_models) playing its thrash clip, pitched -90 about x (INFERRED:
  the prefab lies along +y like the projectile prefabs; drawn upright it sat ~200 px off the bloon).
  STAND-IN: its animation states and the clip's start time (the sim clock, not createdAt) aren't the game's.
- FIXED: the Sun Temple played SunTempleTransformationFx for every tier; each MonkeyTempleModel's own
  effectModel / towerEffectModel now (temple `fx` / `tower_fx`: True Sun God 5xx, 420, 401, 402's
  transformations).
- DONE: particle NoiseModule (export_particles `noise`, particles.rs): per-axis strength (/ frequency with
  damping), octaves, scrollSpeed, positionAmount, as extra velocity. STAND-IN: smoothed value noise, not
  Unity's curl noise. Re-exported: the 117 effects that had it (or a stale SubModule) and the Corvus /
  Geraldo / Psi / Tack tier effects, and the 21 maps' ambient effects with it.
- DONE: Size / Rotation / Color by Speed (`size_speed`, `rot_speed`, `color_speed`: the curve at the speed
  placed in `range`); their 34 effects and 4 maps re-exported. LEFT: TrailModule (83 effects),
  ExternalForces (20), CustomData (9, shader streams).
- LEFT, rest: Metamorphosis / Phoenix Rebirth / Primordial Wrath switch effects (mechanics not simulated). Selection-only
  markers (target reticles, patrol points, Beast leash lines) are UI.
- Exporter fixes: export_particles.load_bundle uses _bundle_cache first, dependencies included (UnityPy
  reports a failed decompression as a parse error and loads nothing, so the MemoryError fallback never ran);
  file keys there can be ints (`str(k).lower()`: particle meshes and nine effects failed on it).

## Audit entries that aren't gaps

- `CreateEffectOnExpireModel`: read for projectiles (`expire_fx`) and sub-towers (`expire_fx`); the
  audit's text match misses the long type name.
- `CreateTextEffectModel`: banana and cash pick-up text is drawn (`cash_pops`, CashUp).
- Fan Club "projectiles" SuperMonkey / PlasmaBlasts: the fan monkeys' looks, drawn from `models/fans`.

## Left

- Need mechanics the clone doesn't simulate: D&D Monke's `MagicBarrierDisplayModel` (barrier near the track's
  end), Pat Fusty's `BigSqueezeFortifiedEffectModel` (MetalBandDropFX when a fortified MOAB's band drops),
  the Great White pet's `CreateGreatWhiteEffectModel` (grab / drag / RetreatFx), the Sheriff's
  `ControlledEffectZoneModel`, Silas' `DetonateEffectModel` (Frostbite detonation; damage, not only display).
- Cosmetic pets (`PetCreateEffectOnPlace / Sell`): the clone has no cosmetic pets.

- `MalevolenceFx` / `MalevolenceSpellCast` (Corvus) exported to effects/, not drawn: the clone doesn't
  simulate the Malevolence spell (its display shows while that attack isn't paused).
- Model transforms: every TowerModel's body DisplayModel has scale 1, no positionOffset and displayScale 1
  (checked 2026-11-17 over all 2508), so a body's size and offset are its prefab's transforms, which the
  export bakes in. Extras use their DisplayModel scale. The FX capture covers particle systems only.

## Done (2026-11-16)

- Projectile emit effects (`ProjectileCreateEffectOnEmitModel`, `emit_fx`): Herald of Everfrost's hailstorm,
  the Sub's airburst muzzle flash.
- Monkey Fan Club (Dart 040 / 050): the transformation cloud on each fan as it turns and as it turns back
  (effectModel, endDisplayModel), the pulse circles on each fan meanwhile (effectId), and the leader's own
  pulse on the tower that used it (effectLeaderId; `Game::fan_leaders`).
- Mermonkey x-x-4 / x-x-5's Trance Totem: as it attacks, its siren spiral plays on the Mermonkey
  (`CreateEffectOnParentOnAttackModel` on the totem's attack, tier `parent_attack_fx`; main.rs watches the
  totem's attack_t restart).
- Skywarden x-5-x's lightning-rod zaps draw its `PointLineEffectModel` line (LightningLarge3, a sprite strip
  tiled every 85.19 along the zap; tier `point_line`, pushed in `rod_zap_line`; the beam loader now takes
  sprite displays too).
- The Rare Quincy Action Figure plays its round-end effect (`CreateEffectOnRoundEndModel`, tier `round_end_fx`).
- Drones (Drone, Drone Swarm, Gyrfalcon) play their `CreateEffectOnAirUnitDestroyModel` effect (SellEffect)
  where the air unit is when they expire (sub-tower `air_destroy_fx`).
- Heli x-3-x+ keeps Downdraft's gust on its helicopter (`CreateEffectOnAirUnitModel`, tier `air_fx`).
- Beast Handler pets now play their tier's BeastUpgradeFx when the handler upgrades (sub-towers' own
  `upgrade_fx`). (`TranceGroundEffect`, Mermonkey x-x-3:
  a weapon's EjectEffectModel, played per shot; now at range / displayRadius too.)
