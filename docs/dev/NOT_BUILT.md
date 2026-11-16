# What isn't built

A survey of game features with no working runtime, so the next job can be picked from a list. Taken from the game's
code and our exports on 2026-11-01; check against the code, because this goes stale.

## Bosses

Data, models, HUD and `boss_logic.rs` exist; nothing runs them. See [BOSSES](BOSSES.md).

## Powers

- **Have:** the Powers screen, and buying with trophies. Bought Powers sit in the profile.
- **Missing:** the in-game Powers menu, placing or activating them, and consuming one. `docs/dev/HANDOFF.md` also lists
  the sandbox's missing Powers button.
- **Game code to read:** `UnityToSimulation.ActivatePower`, `CheckAndConsumePower`, `CheckPowerLocation`,
  `GetPowerInventory`, `GetPowerTower`, `GetPowerProjectile`.
- `game/powers.json` (`tools/assets/export_powers.py`) has each Power's rules, and `ui/prefabs/PowersMenu.json` the
  in-game menu (script `RightMenu.Powers.PowersMenu`: `RebuildPowers`, `GetAvailablePowers`, `PowerUseSuccess`,
  `PowerUseFailed`). Neither is read yet. The shop's Powers button has no click handler.
- Three kinds: towers placed (Banana Farmer, Tech Bot, Portable Lake, Pontoon, Energising Totem, Cave Monkey...), items
  on the track (Road Spikes, Glue Trap, MOAB Mine, Camo Trap), and instant effects (Monkey Boost, Thrive, Super
  Monkey Storm, Dart Time, Cash Drop). `quantity` is how many one use gives (Road Spikes 5, Glue Trap 2), and
  `between_rounds` says when it can be used. Only `CashDrop` and `MoabMine` are named in the Rust code, and that may
  not be Power support.

## Monkey Knowledge

- **Have:** the tree, and buying with trophies.
- **Missing:** every effect. Of the 134 items, only `MoreCash` and `ManaShield` are named in the Rust code, and that may
  not be Knowledge support. Categories: Primary 32, Military 30, Magic 22, Support 22, Powers 15, Heroes 13.
- **Mind the recordings:** tower recordings compare only CHIMPS, where Knowledge is off. Effects have to stay off there.

## Map mechanics

- **Have:** paid obstacles, events on round end, round start, sim time, leaks, bloon spawn and removeable sold; spawner
  rules; paths and areas changing.
- **Missing triggers:** clicks (33 events), towers placed (6), bloons popped (5), projectile hits (5), repeating (4),
  tower in area (2), reverse mode (2), and three map-specific ones.
- **Missing actions:** projectiles from the map (10), path target zones (10), moving platforms (8), rotating and cycling
  areas (5), toggling splitters (2), creating towers (2).
- **Missing:** clickable items, prop animation (the clips and bindings are exported), RegenRemovable, area buffs,
  NestedPrefab placement, towers destroyed by a new area. `map-mechanics.md` lists these per map
  (`tools/assets/summarize_map_mechanics.py`).

## Tower visuals

`tools/assets/audit_visuals.py` lists behaviours the game has and our data doesn't read: 48 kinds. Most are effects on
sell, upgrade, expire and ability (seen on 40 to 77 towers each). Then `FireFromAirUnitModel` (16),
`LinkDisplayScaleToTowerRangeModel` (10) and `CreateTextEffectModel` (8). A name on this list doesn't always mean the
behaviour is missing, since some are handled by other code, so check before building.

### Art for unbuilt mechanics

The `pending_models` setup step (`tools/assets/export_pending_models.py`) exports what the game's models show and no
other step exports, so it's on disk before the mechanics are built. Models go to `models/extra/<name>` (a
TowerModel's to `models/subtowers/`), with `models/extra/index.json` mapping display GUIDs to folders; particle effects
and text pop-ups go to `effects/<name>`. That covers the Paragon degree parts, Beast Handler's tier 2-3 pets and their
projectiles, Rosalia's weapons, Buccaneer hooks and chains, Druid and Obyn vines, frozen platforms and more. Nothing
reads them yet.

### Paragons

`audit_visuals.py --all` lists every model behaviour the data builder never reads, visual or not; a Paragon's are
under the tier "Paragon". The ones only Paragons have show where they differ most from the game:

- **Magus Perfectus**: the clone fires all four attacks at once. The game's `MagusPerfectusGraveyardStateManagerModel`
  switches the attack set by graveyard state (`attackManagerStates`: Consuming, Generating, DrainingOverTime) as the
  graveyard fills and drains (`MagusPerfectusGraveyardModel` per attack: Arcane Spike -50, another +250; max 99999,
  rate bonus past half). Also missing: `ArcaneMetamorphosisModel`, `PhoenixRebirthModel`, `TowerCreateParagonTowerModel`
  (its own phoenix, `MagusPerfectusPhoenixLvl1`-`5`), `SpawnZombieOnBloonDestroyedModel`,
  `EmissionAtClosestPathSegmentModel`. Its art is exported: the graveyard displays `models/extra/MagusLvlFirst` /
  `MagusLvlLast`, and the particle effects `effects/MagusTextFloat` and `effects/MagusPerfectusPhoenixLvl1`-`5`. The upgrade panel already follows the states
  (`TSMThemeMagusPerfectus`; hud.rs, STAND-IN Consuming).
- **Degree displays** (`ParagonAssetSwapModel`, `ParagonAssetSwapPerDegreeModel`): Bomb, Boomerang, Ninja, Sniper,
  Tack and Druid swap parts of their display by degree (`BOMBParagonAddonLvl2`-`5`, `AscendedShadowShurikenLvl4`-`5`,
  `SniperParagonPlatformLvl1`...); they're exported to `models/extra/` (indexed by display GUID in its index.json).
- **Abilities and mechanics**: Ace (`CarpetBombAbilityModel`), Engineer (`CreateTowersInSequenceModel`), Ice
  (`AbilityTriggerEclipseModel`, `ActivateSlowZoneOnAbilityModel`), Sub (`FinalStrikeModel`,
  `MonkeySubParagonSupportModel`), Spike Factory (`SpikeaggedonModel`, `SpikeParagonDamageZoneModel`), Druid
  (`PrimordialWrathModel`), Boomerang (`OverchargeModel`), Tack (`EruptionModel`), Sniper (its ability and
  `TargetSniperParagonTargetingModel`), Buccaneer (`MultiHookManagerModel`).
- **Primordial Wrath** (Druid Paragon; research 2026-11-16, decompiles in research/dumps/decomp/PrimordialWrath/):
  a toggle (`PrimordialWrath.active` +0x88). CanUseAbility (@0x180e4f000): always while active, else when the
  owner's cash >= `cashPerSecondMin`. Process (@0x180e4f980), only while active and rounds are on: cash below
  `cashPerSecondMin` turns it off (vtable 0x8c8); else every 61 frames (`depletionTimer` reaches 0x3d) removes
  cash x `depletionPercent` clamped to [`cashPerSecondMin`, `cashPerSecondMax`] (Simulation.RemoveCash, type 2).
  Activate (@0x180e4e1b0) flips `active`, plays `wrathActiveSound` / `wrathInactiveSound`. Turning on: spawns
  `effectTowerModel` at the tower (Simulation.SpawnEffect), adds `buffMutator` to the tower
  (Tower.AddParagonMutator), creates the `cashModZone` behaviour (range delegate, upgrade hooks), resets
  `depletionTimer`, then each child tower (TowerManager.GetChildTowers: the trees, `treeSubtowerId`) gets
  `effectSubtowerModel` and `subTowerMutator`. Turning off: removes `buffMutator`, destroys the zone, spawns the
  `effectEnd*` models and removes the sub-tower mutators. The mutators carry `wrathProjectileDisplay` /
  `wrathSubtowerProjectileDisplay` and the buffs (`rangeMultiplier`, `attackSpreadMultiplier`,
  `damageMultiplierPerThreshold` per `buffRbeThreshold` RBE, + `extraRbePerBossSkull`, up to `maxMultiplier`);
  AddDebuff (@0x180e4ef30) gives towers in its zone (`InitializeZone` @0x180e4f680, `ZoneRange`) the
  `cashGenMutator` (a Paragon mutator on paragons). Display: its ToggleEffectWhileAbilityActiveModel already
  draws the inactive cyclone (docs/dev/VISUALS.md); the Wrath cyclone needs `active`.
- **Corvus' spellbook** (research 2026-11-16, decompiles in research/dumps/decomp/Malevolence/): the clone has
  no Corvus spells. Malevolence is a continuous spell (Simulation.Corvus.Spells.Continuous.Malevolence) run by
  the player's CorvusManager: state (+0x78) 1 = starting, 2 = paused, 3 = toggled off, 4 = running. ToggleOn
  (@0x180fbe6a0) has Corvus absorb a spirit (TriggerCorvusToAbsorbSpirit), cancels the other continuous spell
  (CancelContinuousSpell) and unpauses its attacks; ToggleOff (@0x180fbe610) releases the spirit; Process
  (@0x180fbe560) pauses the spell attacks (PauseAttacks @0x180fbe1c0) when CorvusManager.IsSpellActive says
  it isn't. Its display `MalevolenceFx` (exported) is a LinkDisplayScaleToTowerRange display with
  hideIfAttackPausedId "Malevolence": shown, range-scaled, while that attack runs.

## Trophy Store and small gaps

- Bought Trophy Store items don't change the game, apart from avatars and banners.
- The Defeat screen's Continue button shows a price but has no click handler.
- Tower recordings: `beast_tests::every_ability_runs`, `beast_tests::sub_commander` and `tests::necromancer_graveyard`
  already fail.
