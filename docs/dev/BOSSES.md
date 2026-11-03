# Bosses

What the game does, what's exported, and what the runtime needs. Nothing here runs yet: the runtime has
the rules screens and boss tags, but no boss fight.

## What's exported

`tools/assets/export_bosses.py`, in setup's `bloon_models` step:

- `game/bosses.json`: all 60 tiers, by bloon id (`Bloonarius1`, `BloonariusElite1`...): family, elite, tier,
  model, health, speed, radius and `behaviors`. Each behaviour keeps the game's type and field names; asset
  GUIDs become prefab and clip names.
- `models/bosses/<prefab>`: skinned models with their clips and particle attach points. `game.json`'s bloon
  `model` stays empty for bosses, because `model3d()` only accepts prefabs with "3D" in the name; use
  `bosses.json` instead. `attach.bin` is stored as `attach.bin.zst` because it's most of their size.
- `ui/prefabs/BossUI-RushUi.json`: the HUD, script `Races.BossUI`: health bar, armour bars, skulls, immunity
  icons and timers. Its methods show what it needs: `ShowSkulls`, `UpdateHealthAndShield`, `UpdateHeat`,
  `UpdateTail`, `UpdateLychRegrow`, `BossPenaltyTimerUpdate`.

## The manager

From `Track.BossBloonManager` (decompile with `tools/il2cpp/decompile.py`):

| Method | What it does |
|---|---|
| `Init` | `spawnRounds` comes from the simulation (`+0x1f8`), a per-tier list of round indices, set from `InGame.BossGameData` |
| `OnRoundStart(round)` | With no current boss, a tier whose spawn round is this round spawns: bloon id is the boss name, "Elite" if elite, then tier + 1 |
| `SpawnBoss` | The same id lookup, at `SpawnDistance` |
| `get_SpawnDistance` | The map's override (`+0xc4`) unless it's the unset value |
| `GetTierFromCurrentRound` | The first tier whose spawn round is at or after the current round, skipping `-1` |
| `GetBossMustBeDefeatedByRound` | The next tier's spawn round - 1; after the last tier, its spawn round + 19 |
| `BloonLeaked` | If the current boss leaks, lives go to 0 |
| `OnRoundEnd` | Stops the penalty timer, and checks the must-be-defeated round |
| `PauseCashAbilities`, `UnpauseCashAbilites` | Cash abilities stop during a fight |

## How a boss works

A boss is a bloon with behaviours that fire named actions. A trigger lists `actionIds`, and **each time it
fires, all of them run**: a percentage isn't paired with one action. So at each of Bloonarius's 75%, 50% and
25%, its spew animation and both bloon spawns run.

`HealthPercentTrigger.Damage` only acts on a new `lowestHealth`, so healing can't fire a threshold twice. A
threshold fires when it's below the previous lowest fraction and the new one is at or below it, and
`currentSkull` becomes its index + 1. With `repeatFirst` it fires once per whole step of the first value between
the health before and after the hit, as in Bloonarius's bleed every 1%. `Damage` doesn't read
`preventFallthrough`, so that's handled elsewhere.

`renderer/src/boss_logic.rs` has the schedule and `BossScript`, with tests, but `Game` doesn't call it yet. Hooking
it up: make a `BossScript` when a boss spawns, call `damage` on each hit and `tick` each frame, and carry out the
`Effect`s it returns. Timers fire every `interval x 60` frames, and `WaitForSeconds` holds its chained actions
until its delay passes.

## The families

Standard tier health and speed, with what's special beyond the shared behaviours:

| Family | Tiers 1 to 5 health | Speed | Needs |
|---|---|---|---|
| Bloonarius | 20k, 75k, 350k, 750k, 3M | 1.25 to 1.5 | `SpawnBloonsActionModel`, with spew and bleed curves |
| Lych | 14k, 52.5k, 220k, 525k, 2.1M | 2.0 to 2.2 | `LychBossSuperScriptModel`, `AbsorbTowerBuffsActionModel`, `HealSelfActionModel`, `LychTombstoneSpawnActionModel`, `SellTriggerModel`, `WorthReductionModel`, `RemoveAllMutatorsActionModel` |
| Vortex | 20k, 62.8k, 294k, 628k, 2.51M | 3.6 to 4.2 | `StunTowersInRadiusActionModel`, `DestroyProjectilesInRadiusActionModel`, `ReflectProjectilesInRadiusActionModel`, `BuffBloonSpeedModel` |
| Dreadbloon | 7.5k, 25k, 120k, 260k, 1M | 1.25 to 1.6 | `GenerateArmourActionModel`, `SpawnBloonsUntilArmourBreaksActionModel`, `DamageReductionModel`, `ApplyTowerSetImmunityModel` |
| Phayze | 9.5k, 35k, 160k, 350k, 1.45M | 0.8 | `PhayzeBehaviorModel`, `PhayzeTimerActionModel`, `DashForwardsActionModel`, `GenerateShieldActionModel`, `RedirectBloonSpawnActionModel`, `ThrowBloonsActionModel` |
| Blastapopoulos | 17.5k, 65k, 300k, 650k, 3M | 1.0 to 1.25 | `OverheatTriggerModel`, `AddHeatActionModel`, `FireballActionModel`, `CreatePropsOnBloonActionModel`, `RangeReductionZoneModel`, `AbilityCooldownZoneModel`, `RemoveAllDotActionModel` |

Shared by all: `HealthPercentTriggerModel`, `PlayAnimTriggerActionModel`, `CreateSoundOnActionModel`,
`SpawnDeathAnimModel`, `AnimDamageStateModel`, `DistributeCashModel`, `BadImmunityModel`,
`SetSpeedPercentActionModel`, `SetInvulnerableActionModel`. Most also use `TimeTriggerModel` and
`WaitForSecondsActionModel`.

## How each family plays

From `game/bosses.json` (tier 1 numbers; higher tiers scale them) and the decompiled behaviours. Read a tier's
`behaviors` for exact values.

**Damage cap.** With `preventFallthrough` (Lych, Dreadbloon, Phayze, Blastapopoulos, Vortex),
`HealthPercentTrigger.GetModifiedDamage` limits a hit that would cross a threshold, so one big hit can't skip a
skull. Bloonarius doesn't have it, so its skulls can be skipped together, as `HealthTrigger::damage` allows.
`BossScript` doesn't apply this cap yet: `GetModifiedDamage` runs before damage lands, and its exact result needs
one more read.

**Bloonarius.** Skulls at 75, 50 and 25%: a spew animation (2.5 s pause) with two spawns, Ceramics ahead of it and
Greens along the track, becoming BADs and Rainbows by tier 5. A second trigger bleeds a few bloons every 1% of health.
`SpawnBloonsAction.SpawnBloon` picks positions from `spawnDistAhead`, or randomly between `spawnTrackMin` and
`spawnTrackMax` of the path, using the game's `BufferedRandom`, so it affects seeded replays.

**Vortex.** Skulls at 75, 50 and 25%: goes invulnerable for 0.5 s, stuns towers in radius 60 (16 s at tier 1, 27 s at
tier 5, and blocks selling them), knocks itself back 100 units, destroys and reflects projectiles. Every 5 s it
destroys projectiles again. Nearby bloons get a 2.5x speed boost (`BuffBloonSpeedModel`, radius 40).

**Dreadbloon.** Skulls at 75, 50 and 25%: regrows armour (3,750 at tier 1, 500,000 at tier 5) that slows it to 30% and
only some tags can break, changes which tower sets it's immune to (four phases), and spawns rock bloons every 4 s
until the armour breaks. Needs a separate armour health layer, which the HUD's armour bars show.

**Phayze.** Skulls at 75, 50 and 25%: stops, goes invulnerable, makes a shield of 2,375 and redirects bloon spawns to
a portal, then 2 s later dashes forward and drops invulnerability. Every 24 s it alternates timer effects for 12 s,
and throws bloons, adding camo. `PhayzeBehaviorModel` also removes camo detection from towers and speeds it up
while shielded.

**Blastapopoulos.** Skulls at 80, 60, 40 and 20%: pauses, goes invulnerable, throws a fireball (stuns towers, leaves a
magma pool) and adds 25% heat, then 2 s later makes a ring of 6 rocks and moves on. A heat meter (`OverheatTrigger`,
max 1,750 at tier 1) rises from damage and hits it's immune to, and falls over time, faster when idle or from ice.
Full heat burns up projectiles, stuns towers and adds ability cooldown for 20 s. Every 2 s it removes damage over time.
Towers near it lose 8% range and abilities cool down 10% slower.

**Lych.** No skulls from health. `LychBossSuperScriptModel` runs three timers: every 12 s it drains tower buffs in
radius 100 and freezes those towers for 5 s, every 9 s it regrows (heals 1.5%, can also raise its skull count), and
every 18 s it drops a tombstone. It also spawns Mini Lych souls, removes most mutators, slows and speeds itself,
reacts to towers being sold, and makes its pops worth 35%. It reaches into towers the most, so it should come last.

## Suggested order

1. A boss spawning at its round: model, spawn and death animations, damage states, health bar, leak = defeat,
   no children on pop, and the event's rules from the rules screen.
2. The trigger and action framework, with Bloonarius as the test: it only needs health triggers and bloon
   spawns. Spawned bloons need the game's positions (`spawnDistAhead`, `spawnTrackMin` and `spawnTrackMax`).
3. Dreadbloon, then Blastapopoulos, Vortex, Phayze and Lych, whose scripts reach furthest into towers.

Each needs its own recording before it counts as matching, as towers do.

## Not known yet

- `BossGameData.DefaultSpawnRounds` looks like `39, 59, 79, 99, 119` (rounds 40 to 120) and
  `DefaultCheckpointRounds` like `18, 38, 58, 78, 98, 118`: found in the game's metadata by shape, next to each
  other, matching the 5-entry array and the server's 6 `checkpointCosts`, not read through the field. A boss
  recording should confirm them. When an event overrides them is unknown; `BossGameData` also has `bossBloon`,
  `bossEliteMode`, `bossRankedMode`, `bossForcePreSpawnBoss` and `bossSpawnDistanceOverride`.
- The penalty timer and `GetBossTime` scoring.
- The exact damage `GetModifiedDamage` returns at a threshold, and the rest of `OverheatTrigger`.
- `BossTierRewardUI` (the scene shown between tiers), not exported yet.
