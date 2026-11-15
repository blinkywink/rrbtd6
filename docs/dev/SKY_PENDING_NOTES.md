# Skywarden: decomp notes for the four two-tower recordings (2026-11-14)

Recordings (pending/sky, seed 699336078): sky_galv_dart (0-3-0 + Dart 0-0-2), sky_rod_super
(0-5-0 + Super 2-0-0), sky_ricochet_dart (0-4-0 ability at 120 + Dart 0-0-2), sky_chill_pair (two
0-0-3s). Chain lightning is done (5697ef1, sky_galv_dart exact). First diffs before that commit:
sky_rod_super rt 183 (Ceramics vs a MOAB; the Super Monkey throws 723 vs clone 947, its first
difference at rt 247 vs 239; a first position diff at rt 79), sky_ricochet_dart rt 1987 (same
Green/Black pattern as sky_galv_dart's rt 1927), sky_chill_pair rt 2649 (Green 298.80 vs 298.95).

## LightningRodManager.OnBloonDamaged (VERIFIED-DECOMP @0x180d32ea0)
On the Skywarden tower; called for every bloon damaged by any projectile.
- Returns while `cooldown` (+0x78) > 0.
- Returns when the damaging projectile's emitting tower (vfunc 0x7b8, its +0x48) is the manager's own
  tower: only OTHER towers' hits trigger it.
- Returns if the bloon is destroyed (+0x21).
- The projectile's DamageModels: any whose GetDamageTypeFromImmunities(immuneBloonProperties) is in
  the manager model's damage-type list (+0x28; Energy / Plasma) arms it.
- Then the manager's `lightningRods` (+0x70) in list order, skipping destroyed ones: the first whose
  2D distance from the bloon's position to the rod's position (vfunc 0x558) is <= LightningRod.GetRadius
  (@0x180cca1f0: radius, scaled by tower radius / towerBaseRadius when scaleWithTowerRadius) takes it.
- LightningRod.TryEmitLightning (@0x180cca610) with (projectile, bloon); when it emits, cooldown =
  rod model zapCooldown (+0x3c) x 60.
- TryEmitLightning: the rod model's projectile (+0x50; a Lightning, pierce 5, damage 40) through the
  rod's emission (+0x70) Emit(model, the bloon's position, Target(bloon), roundTime, the rod's tower
  (vfunc 0x7b8), ...). Rest of it (the bool it returns) not read yet.

## Chilled.Collide stage effects (VERIFIED-DECOMP @0x180c99c50)
With the bloon's existing stage mutator (value v = the mutator's +0x80 int): v > 1 -> SlowNearby; v > 2
and not sentShrapnel (+0x75) -> EmitIceShrapnel once per projectile; then the old stage mutators go
and mutators[min(v, 2)] is added.
- SlowNearby (@0x180c9b870): every bloon in the projectile's `zone` (+0xa0, its +0xa8 list; a
  collision zone, radius slowRadius 8 x radiusMultiplier 0.8?) that isn't Boss-tagged (0x1000), tag
  0x8000, or its model +0x93: non-MOABs get radiusSlowMutator (+0x88), MOABs (only with affectMoab)
  moabRadiusSlowMutator (+0x90), for (int)slowDuration x 60 frames (AddMutator vfunc 0x2a8, same flags
  as the hit's slow).
- EmitIceShrapnel (@0x180c9a170): shrapnelEmittingBloon = the bloon; BloonManager.GetTargets(count =
  shrapnelCount (+0x5c, 2), a static TargetType (+0x48..0x58 of a static class), from the bloon's
  Position, range shrapnelRange (+0x60, 300), BloonFilter(this)); for each target: the IceShard
  projectile (+0x68: pierce 3) through `emission` (+0xa8), aimed at Target.Position - the bloon's
  position (Normalize), its rotation from that, Projectile.SetStartPosition at the bloon.

## ActivateRicochetSupportZone (x-4-x Thunder Charge; sky_ricochet_dart)
Model (ActivateRicochetSupportZoneModel, read from the sim model): range 48 (x-4-x) / 58 (x-5-x) but
useTowerRange true (GetRange @0x180e04760: the tower's range when the support model's +0x70 flag
is set), lifespan 9 s, maxNumTowersModified 9999999, mutatorId ThunderAbilityRicochet, isUnique,
canEffectThisTower false, no filters; distance 100, minDistance 0, maxBounces 2, targetType Close,
delay 0.05, expireIfNoTargetFound false. It shares the support-zone helpers (GetLifespan /
MaxNumTowersModified / CanEffectThisTower are ActivateAttackCollisionSupportZone's), so it fits
build_game_data.py's timed `supports` table as a new kind.
RicochetMutator.Mutate (VERIFIED-DECOMP @0x180e26bf0): builds RetargetOnContactModel(mutatorId,
distance, maxBounces, delay, targetType, minDistance?, expireIfNoTargetFound) from the parent zone
model (+0x74..+0x8c), then for every weapon of the tower model, its projectile: if it has a
DamageModel and no RetargetOnContactModel yet, a clone of that model is appended to its behaviours
(so it comes after TravelStrait: the clone's `retarget` with after = true). Display +0x78 added to
the tower model. The clone already has RetargetOnContact (Proj.retarget).
