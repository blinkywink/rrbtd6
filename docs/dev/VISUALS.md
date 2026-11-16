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
  Psi, D&D Monke, the Nautic Siege Core... STAND-IN: the animator's state-to-clip order (AlternateAnimationModel states,
  originState) isn't exported, so clips go in name order.

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

## Audit entries that aren't gaps

- `CreateEffectOnExpireModel`: read for projectiles (`expire_fx`) and sub-towers (`expire_fx`); the
  audit's text match misses the long type name.
- `CreateTextEffectModel`: banana and cash pick-up text is drawn (`cash_pops`, CashUp).
- Fan Club "projectiles" SuperMonkey / PlasmaBlasts: the fan monkeys' looks, drawn from `models/fans`.

## Left

- `MalevolenceFx` / `MalevolenceSpellCast` (Corvus) exported to effects/, not drawn: the clone doesn't
  simulate the Malevolence spell (its display shows while that attack isn't paused).
- Heli x-3-x+ keeps Downdraft's gust on its helicopter (`CreateEffectOnAirUnitModel`, tier `air_fx`).
- Beast Handler pets now play their tier's BeastUpgradeFx when the handler upgrades (sub-towers' own
  `upgrade_fx`). (`TranceGroundEffect`, Mermonkey x-x-3:
  a weapon's EjectEffectModel, played per shot; now at range / displayRadius too.)
- Paragon degree displays (`ParagonAssetSwapModel`), Beast Handler pet upgrade effects
  (`BeastUpgradeFx` on the pets), `ToggleEffectWhileAbilityActiveModel` (Druid), `MagicBarrierDisplayModel`
  (D&D Monke), `CreateRopeEffectModel` (Buccaneer hooks), `PointLineEffectModel` (Skywarden): exported art,
  not drawn. See docs/dev/NOT_BUILT.md.
- Model transforms (scale, offset) aren't compared with the game automatically: the FX capture covers
  particle systems only. Report anything that looks wrong in docs/dev/ISSUES.txt.
