//! PROTOTYPE gameplay rules for the playable demo.
//!
//! Stats come verbatim from the decoded game data (every Dart Monkey and Tack
//! Shooter tier's TowerModel). Rules marked VERIFIED come from the engine session's
//! disassembly (research/04-native-analysis.md); the rest are placeholders
//! until moved into `btd6-sim`:
//! - VERIFIED: 60 frames per second; speedFrames = speed / 60; round group times
//!   (scale * t) * 60 frames, spaced span/(count-1).
//! - HYPOTHESIS P-LEAK: lives lost = leakDamage of the bloon plus all its descendants.
//! - HYPOTHESIS P-CASH: $1 per layer popped; end-of-round bonus 100 + round (Medium mod data).
//! - HYPOTHESIS P-TARGET: First/Last = furthest/least along the track, Close = nearest,
//!   Strong = highest RBE; camo bloons are only targetable by attacks without an active
//!   FilterInvisibleModel.
//! - HYPOTHESIS P-POP: children spawn at the parent's distance, spaced 4 units back,
//!   and cannot be hit by the projectile that popped their parent.
//! - HYPOTHESIS P-HIT: projectile hits when centres are within the sum of radii; a hit on an
//!   immune bloon (immuneBloonProperties & bloonProperties) uses pierce but deals no damage.
//! - HYPOTHESIS P-EMIT-ARC: Arc emission spreads `count` projectiles evenly over `angle`
//!   degrees centred on the target; Parallel offsets them sideways by `spread`.
//! - HYPOTHESIS P-NOROTATE: attacks without RotateToTargetModel fire along the tower's
//!   placement facing; a 360-degree arc starts there.
//! - HYPOTHESIS P-CONTACT: CreateProjectileOnContact spawns its projectiles at the hit bloon,
//!   along the parent's heading, on every hit (immune or not); passOnCollidedWith excludes
//!   that bloon.
//! - HYPOTHESIS P-RING: a LinkProjectileRadiusToTowerRange projectile (Ring of Fire) sits on
//!   the tower with radius = attack range for its AgeModel lifespan.
//! - HYPOTHESIS P-OFFSCREEN: towers only target bloons whose centre is on the map
//!   (Map.IsPointOnMap with no border, the test FilterOutOffscreen uses).
//! - HYPOTHESIS P-INSTANT: InstantModel projectiles (Sniper) sit on their target and hit it
//!   the frame they are fired; they only hit that bloon and expire if it is gone.
//! - HYPOTHESIS P-BOUNCE: RetargetOnContactModel jumps to the closest unhit bloon within
//!   `distance` of the last hit, `delay` seconds later, until pierce runs out.
//! - EmitOnDamageModel (Shrapnel Shot) emits like P-CONTACT, along the shot's heading, but only
//!   on hits that deal damage.
//! - HYPOTHESIS P-PATH: FollowPathModel paths (boomerangs) start at the eject point, +y along
//!   the throw, unscaled and never flipped.
//! - HYPOTHESIS P-BOUNCE-TRAVEL: travelling projectiles with RetargetOnContact (glaives) home in
//!   on the next bloon instead of jumping to it, and can hit others on the way.
//! - HYPOTHESIS P-ARRIVE: ArriveAtTargetModel flies straight to the target's position at fire
//!   time over timeToTake, then expires.
//! - HYPOTHESIS P-PUSH: PushBackModel moves a tagged bloon pushAmount * class multiplier back
//!   along the track on every hit.
//! - HYPOTHESIS P-SLOW: slow multipliers with different ids multiply; a freeze stops the bloon
//!   and adds BloonProperties.Frozen (16); a slow zone caps the result at its speedScale.
//! - HYPOTHESIS P-SLOWMOD: SlowModifierForTagModel replaces the multiplier when
//!   resetToUnmodified is set, otherwise multiplies it.
//! - HYPOTHESIS P-EFFECT-LAYERS: an effect with `layers` n passes to a popped bloon's children
//!   with n - 1 while n > 1; effects are applied before the hit's damage.
//! - HYPOTHESIS P-SEEK: TrackTargetModel shots turn at turnRate towards their target, then the
//!   closest unhit bloon within half of maxSeekAngle of their heading.
//! - HYPOTHESIS P-WIND: WindModel moves a bloon a uniform distanceMin..Max back along the track
//!   with probability `chance` per hit.
//! - HYPOTHESIS P-TRACKTARGET: TargetTrackModel aims at a uniformly random in-range track point.
//! - HYPOTHESIS P-ALT: AlternateProjectileModel replaces every interval-th shot.
//! - HYPOTHESIS P-ROUNDS: projectiles are cleared at round end unless AgeModel.rounds is left;
//!   useRoundTime lifespans only run during rounds.
//! - HYPOTHESIS P-ROUNDRATE: StartOfRoundRateBuffModel.modifier multiplies weapon intervals
//!   for its first `duration` seconds of a round.
//! - HYPOTHESIS P-RANDPOS: RandomPositionBasicModel aims at a uniform direction and distance
//!   in [minRadius, maxRadius], clamped inside the map.
//! - HYPOTHESIS P-BANK: SendToBankModel income goes into the bank (capped); interest is added
//!   at round end.
//! - HYPOTHESIS P-SUPPORT: support buffs reach towers whose footprint touches the radius;
//!   one instance per mutator id unless it stacks; range x(1 + m) + a, rate x m, pierce + a.
//! - HYPOTHESIS P-DISCOUNT: DiscountZone discounts sum (per stack limit) up to tierCap and
//!   discounted prices round to the nearest 5.
//! - Druid: P-EMIT-RANDOM (thorn spread), P-LIGHTNING (chain without branching), P-INTERVAL
//!   (Ball Lightning sub-shots at a random bloon in range), P-REMOVEMUT (tornado strips effects
//!   by mutator id), P-ATTARGET (Jungle Vines land on their target).
//! - Alchemist: P-ACIDPOOL (missed splashes become pools), P-BREW / P-BREWTARGET (brews on the
//!   closest attacking tower, by shots and seconds), P-LEADGOLD (cash for popping Lead).
//! - Desperado: P-BURST (repeated instant shots), P-LINE (line shots swept along the range).
//! - Mortar / Dartling: P-POINTER (point-aiming attacks fire while any reachable bloon exists,
//!   turning at RotateToPointerModel.rate), P-TARGETSPREAD (uniform disc around the aim point),
//!   P-POINTDEFAULT (STAND-IN: the set point starts on the nearest track point), P-SLOWDOWN
//!   (TravelStraitSlowdownModel as straight travel), P-EXHAUSTPIERCE (Rocket Storm blasts per hit).
//! - Engineer sentries: P-RANDLAND (random land spot, 20 draws), P-TYPEDTOWER (typed sentries
//!   take turns), P-SUBBLOCK (STAND-IN: sub-towers don't block placement). Sentries count as the
//!   Engineer for buffs (STAND-IN) and credit their pops to it (CreditPopsToParentTowerModel).
//!   Bloon Trap: P-EAT (bloons swallowed whole up to rbeCapacity, paid out x rbeCashMultiplier
//!   when collected).
//! - Abilities: ActivateAttackModel (the ability's attacks run alongside the tower's for their
//!   lifespan; one-shots fire one volley) and TurboModel (rate x multiplier, +damage).
//!   P-ABILITYCD (cooldowns tick only during rounds), P-ABILITYSTART (a new ability starts on
//!   full cooldown). Heli: P-HELI steering, P-HELIHEIGHT. Ace: P-ACEPATH flight patterns.
//! - Water towers: P-WATER (water towers go where the first area under the point is water),
//!   P-BROADSIDE (Buccaneer: the tower turns additionalRotation past its target, guns fire at
//!   facing - offsetRotation, a FireWhenAlternateWeaponIsReady gun fires with its paired gun if
//!   a bloon is in its FilterTargetAngleFilter arc), P-SUBMERGE (Sub "Submerge" mode swaps in
//!   submergeAttackModel; cooldown / hero XP scales to towers in range, not stacking), P-SHARED
//!   (Advanced Intel shares every other tower's range), P-TRADEEMPIRE, P-CASHBACK, P-FLAGSHIP
//!   (rate to every tower on water), P-WATERRANGE (Mermonkey range on water), P-SCALEDMG
//!   (damage x min(max, base + perSecond x age) / base), P-TRANCE (entranced bloons circle the
//!   tower at orbitRadius for `duration`, then go back to their track spot), P-FIGHTER (carrier
//!   planes), P-TAKEDOWN (MOAB / ZOMG Takedown: strongest hookable bloons destroyed whole for pop
//!   cash x multiplier), P-PREEMPT (a strike at each MOAB-class bloon a round sends),
//!   P-SUBCOMMANDER (x damageScale then +damageIncrease), P-DECK (Flagship deck areas in the
//!   ship's local x right / y forward frame; no size filter). Hero abilities: P-BARRAGE (MOAB
//!   Barrage as a damage-over-time on the strongest MOAB-class bloons), P-BIOHACK (nearest
//!   towers +damage; no shutdown), P-SYPHON (round bloons one rank lower; no extra cash).
//! - Maps: P-MULTIPATH (spawn paths in export order; the spawn index emissionIndex % count is
//!   the game's Splitter.SpawnRout, exits round-robin per splitter as Splitter.Rout; children
//!   stay on their parent's path), P-BOSSPATH (the boss path, by name, isn't a spawn path). Paragons: degree and degree stats from the game's
//!   code (see `paragon_degree` / `paragon_tier`); P-PARAGONPOWER for the inputs it lacks.
//! - NOT IMPLEMENTED: the other ability behaviours (Fan Club, Supply Drop, IMF Loan, Call to
//!   Arms, Overclock, support-zone abilities, Darkshift...), knockback, EmitOnPopModel, CarryProjectileModel, Spike Factory target modes (Close /
//!   Smart / Far / chosen point), GrowBlock, Monkey City income, free upgrades, Central
//!   Market / Banana Central buffs, Monkeyopolis.

#[path = "game/coop_state.rs"]
mod coop_state;
#[path = "game/coop_cash.rs"]
mod coop_cash;
pub(crate) mod coop_areas;

use crate::data::{AREA_LAND, AREA_SKIPPED, Area, BloonDef, Brew, Emission, GameData, Proj, TagMod, Tier, TowerDef, TowerTiers};
use std::collections::HashMap;

mod geraldo;
mod powers;

pub const FPS: f32 = 60.0;
/// `Constants.worldXMin/Max`, `worldZMin/Max` (the wide-screen flag that adds 28 to X is
/// compiled to `false`).
const WORLD_X: f32 = 150.0;
const WORLD_Z: f32 = 116.0;
/// `Map.IsPointOnMap(p, border)`: inside the world bounds grown by `border`. The bounds are
/// symmetric, so our z = -sim.y is fine.
fn on_map(p: [f32; 2], border: f32) -> bool {
    p[0].abs() <= WORLD_X + border && p[1].abs() <= WORLD_Z + border
}

/// Medium `SellMultiplierModModel.addition`.
pub const SELL_MULTIPLIER: f64 = 0.7;

/// `Simulation.trackDifficultyXpScale` by MapModel.mapDifficulty (recorded: tower_xp_rounds).
const TRACK_DIFFICULTY_XP_SCALE: [f32; 4] = [1.0, 1.1, 1.2, 1.3];

#[derive(Clone)]
pub struct Track {
    pts: Vec<[f32; 2]>,
    cum: Vec<f32>,
    /// PathModel point distances and bloonSpeedMultiplier (the game's, when imported).
    dists: Vec<f32>,
    speeds: Vec<f32>,
    /// Path.pointInfos: the point every CACHE_DISTANCE along the path, where a distance is
    /// looked up (empty: interpolate `pts`).
    quick: Vec<[f32; 2]>,
}

/// Path.CacheDistance.
const CACHE_DISTANCE: f32 = 0.01;

impl Track {
    pub fn new(pts: Vec<[f32; 2]>) -> Self {
        let mut cum = vec![0.0];
        for w in pts.windows(2) {
            let d = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
            cum.push(cum.last().unwrap() + d);
        }
        Self { pts, cum, dists: Vec::new(), speeds: Vec::new(), quick: Vec::new() }
    }

    /// With the game's own point distances and speed multipliers.
    pub fn with_info(pts: Vec<[f32; 2]>, dists: &[f32], speeds: &[f32], quick: &[f32]) -> Self {
        let mut t = Self::new(pts);
        t.quick = quick.chunks_exact(2).map(|w| [w[0], w[1]]).collect();
        if dists.len() == t.pts.len() && speeds.len() == t.pts.len() {
            t.dists = dists.to_vec();
            t.speeds = speeds.to_vec();
        }
        t
    }

    /// Path.DistanceToExtraPointInfoBloon (VERIFIED-DISASM @0xf8c750): the bloon speed
    /// multiplier at distance `d`, lerped in f32 between the points around it as
    /// (1 - t) a + t b (which isn't always exactly 1 between two 1s).
    pub fn speed_mult(&self, d: f32) -> f32 {
        if self.dists.len() < 2 {
            return 1.0;
        }
        let i = match self.dists.binary_search_by(|c| c.total_cmp(&d)) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        if i + 1 >= self.dists.len() {
            return self.speeds[self.speeds.len() - 1];
        }
        let span = self.dists[i + 1] - self.dists[i];
        if span == 0.0 {
            return self.speeds[i];
        }
        let t = (d - self.dists[i]) / span;
        (1.0 - t) * self.speeds[i] + t * self.speeds[i + 1]
    }
    pub fn length(&self) -> f32 {
        *self.cum.last().unwrap()
    }
    /// Path.InitEntryExits (VERIFIED-DISASM @0xf8d9b0): along the path's points every
    /// CacheDistance (0.01), the first and the last one on the map (Path.IsPointOnMap: x in
    /// Constants.worldXMin..worldXMax = -150..150, y in worldZMin..worldZMax = -116..116) give
    /// spawnDist and leakDist. Both are symmetric, so the clone's flipped y doesn't matter.
    pub fn entry_exit(&self) -> (Option<f32>, Option<f32>) {
        let on_map = |p: [f32; 2]| (-150.0..=150.0).contains(&p[0]) && (-116.0..=116.0).contains(&p[1]);
        let n = (self.length() / 0.01) as i64;
        let at = |i: i64| i as f32 * 0.01;
        let spawn = (0..=n).find(|&i| on_map(self.pos(at(i)))).map(at);
        let leak = (0..=n).rev().find(|&i| on_map(self.pos(at(i)))).map(at);
        (spawn, leak)
    }

    /// Path.Init / PathSegment..ctor (VERIFIED-DISASM @0xf8dd50, @0xf8bb60): the path in
    /// MaxSegmentSize (7) pieces from its start, the last one shorter; each segment's point is
    /// the path point halfway along it.
    pub fn segments(&self) -> Vec<[f32; 2]> {
        let len = self.length();
        let full = (len / 7.0) as usize;
        let mut v: Vec<[f32; 2]> = (0..full).map(|i| self.pos(i as f32 * 7.0 + 3.5)).collect();
        if (len / 7.0).ceil() as usize > full {
            let min = full as f32 * 7.0;
            v.push(self.pos(min + (len - min) * 0.5));
        }
        v
    }

    /// Path.DistanceToPointInfoBloon (VERIFIED-DISASM @0xf8cb40): a bloon sits 4 below the path
    /// point in the game's y (so 4 above in the clone's flipped y).
    pub fn bloon_pos(&self, d: f32) -> [f32; 2] {
        let p = self.pos(d);
        [p[0], p[1] + 4.0]
    }

    pub fn pos(&self, d: f32) -> [f32; 2] {
        // Path.DistanceToPointInfo (VERIFIED-DISASM @0xf8cbe0): pointInfos[(uint)(d / CacheDistance)],
        // the last one past the end, the first below 0.
        if !self.quick.is_empty() {
            return self.quick[((d / CACHE_DISTANCE) as u32 as usize).min(self.quick.len() - 1)];
        }
        let d = d.clamp(0.0, self.length());
        let i = match self.cum.binary_search_by(|c| c.total_cmp(&d)) {
            Ok(i) => i.min(self.pts.len() - 2),
            Err(i) => i.saturating_sub(1).min(self.pts.len() - 2),
        };
        let seg = self.cum[i + 1] - self.cum[i];
        let t = if seg > 0.0 { (d - self.cum[i]) / seg } else { 0.0 };
        let (a, b) = (self.pts[i], self.pts[i + 1]);
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
    }
}

pub struct Bloon {
    pub id: u64,
    pub kind: usize,
    pub dist: f32,
    /// Cached `track.pos(dist)`: refreshed wherever `dist` changes (spawn, movement, push-back,
    /// wind), so targeting and collision don't repeat the path lookup per tower / projectile.
    pub pos: [f32; 2],
    pub hp: i32,
    pub effects: Vec<Effect>,
    /// Camo removed (RemoveBloonModifiersModel); children inherit it.
    pub decamo: bool,
    /// Index into `Game::tracks`.
    pub path: u8,
    /// Length of the tracks it has already run along (splitter routing).
    pub base: f32,
    /// Grow (regrow bloons): frames until it next grows, and the layer it grows back up to
    /// (growCeilingLayerNumber: its own when spawned, its parent's when popped out of one).
    pub grow_t: u32,
    pub grow_ceiling: u32,
    /// Grow.growToChildColor / growToParentColor: set by a model with overrideChildWithColor
    /// (White: Pink -> White) and kept by the object through its degrades.
    pub grow_colors: Option<(String, String)>,
    /// Its Grow hasn't had its first Process on this model yet (Grow.init).
    pub grow_fresh: bool,
    /// Emitted this frame: it doesn't move until the next (the recordings show a new bloon at
    /// its spawn distance on its first frame).
    pub fresh: bool,
    /// Where it was when this frame began (collisions use these; INFINITY for a bloon emitted
    /// this frame, which nothing can hit yet).
    pub prev: [f32; 2],
    /// Its distance when this frame began.
    pub prev_dist: f32,
    /// This frame's speed multiplier (slows, MOAB shove...).
    pub mult: f32,
    /// Where the bloon grid has it: its cell is updated as it processes, before it moves, so a
    /// frame's collisions see the cell from a frame before `prev`.
    pub cell_pos: [f32; 2],
    /// Bloon.trackSpeedMultiplier: the path's speed multiplier where it was last processed.
    pub track_mult: f32,
    /// Its place in the bloon list a collision grid bucket fills from (creation order; a
    /// degraded bloon is the same object, so its first child keeps it).
    pub order: u64,
    /// Bloon.lineage: the layer ids it came from. A projectile that hit any of them can't hit
    /// it (Projectile.HaveAlreadyCollidedWithBloonInLineage, VERIFIED-DISASM @0xcabdc0).
    pub lineage: Option<std::sync::Arc<[u64]>>,
    /// Bloon.layerId: new for every bloon made, the popped one's object carrying on as its first
    /// child included; what Projectile.CollideBloon puts in collidedWith (VERIFIED-DISASM
    /// @0xcaa480; recording dart_500: one ball pops both Reds a Black became).
    pub layer: u64,
    /// Bloon.rotation (= prevRot): the path's heading where it was put on the path (SetPath),
    /// then, for a rotateToFollowPath model, lerped towards the heading as it moves
    /// (UpdatePositionAlongTrack). Its collision circles turn with it (SetRotation).
    pub rot: f32,
    /// Its place in the process list: Simulation.time x 4 + the step's phase it was made in
    /// (see `Game::phase`).
    pub born: u64,
    /// Its Position.z: 0 when made, then on each Bloon.Move 120 for a MOAB-class model and 20
    /// otherwise (recording boomerang_paragon_r90_t, logged under the Orbit). A popped bloon's
    /// first child is the same object and keeps it until it moves.
    pub height: f32,
    /// Emitted by the Spawner and not moved yet: not in the CollisionChecker's bloon grid
    /// (HYPOTHESIS P-SPAWNGRID, recording ice_050: a Black spawned at 2547 isn't tried by a
    /// pierce-999 pulse covering the map on 2548).
    pub unplaced: bool,
    /// Made its first Bloon.Move this frame (it was `unplaced` until then).
    pub first_move: bool,
    /// MoveWithWind on it (offTrackCount > 0): carried at this x its speedFrames toward its
    /// track spot instead of moving along the track.
    pub wind: Option<f32>,
    /// MoveWithWind attached this frame: its first Process is the next frame's.
    pub wind_new: bool,
    /// While carried: the distance it had when blown back, which keeps it filed on that path
    /// segment (Bloon.Move, which refiles it, doesn't run; recording ninja_020 rt 421: a Ninja
    /// targets a Red carried at 173.6 from its old segment in range).
    pub wind_seg: Option<f32>,
    /// In a Gyrfalcon's claws (MoveWithAirUnit, offTrackCount): not filed on the path's segments.
    pub held: bool,
    /// The wind was attached to it by a Wind hit (offTrackCount), not taken over from a parent.
    pub wind_direct: bool,
    /// The MoveWithWindModel carrying it (a child that copies the mutator draws with it).
    pub wind_model: Option<Box<crate::data::Wind>>,
    /// NecromancerZone: (inside the zone, subscribed to its OnBloonDegrade). HYPOTHESIS
    /// P-GRAVESUB: BloonEnter subscribes; a degrade the zone hears uses the subscription up, and
    /// every bloon a pop makes (the first child too) enters afresh on the zone's next Process
    /// (recordings wizard_004 rt 2384: a Pink whose last pop the zone heard isn't counted again
    /// that frame; wizard_005 needs the first child's fresh Enter).
    pub grave_zone: (bool, bool),
    /// The factory object it lives in (see `Game::fact`): a degraded bloon's first child keeps
    /// its parent's.
    pub obj: u32,
}

/// List<T>.Sort(Comparison<T>) as IL2CPP's mscorlib does it (ArraySortHelper.IntrospectiveSort):
/// unstable, so equal keys land where the game's do.
pub(crate) fn dotnet_sort<T>(v: &mut [T], cmp: &impl Fn(&T, &T) -> i32) {
    fn swap_if_greater<T>(v: &mut [T], cmp: &impl Fn(&T, &T) -> i32, a: usize, b: usize) {
        if a != b && cmp(&v[a], &v[b]) > 0 {
            v.swap(a, b);
        }
    }
    fn insertion<T>(v: &mut [T], cmp: &impl Fn(&T, &T) -> i32, lo: usize, hi: usize) {
        for i in lo..hi {
            let mut j = i as isize;
            while j >= lo as isize && cmp(&v[j as usize + 1], &v[j as usize]) < 0 {
                v.swap(j as usize, j as usize + 1);
                j -= 1;
            }
        }
    }
    fn down_heap<T>(v: &mut [T], cmp: &impl Fn(&T, &T) -> i32, mut i: usize, n: usize, lo: usize) {
        while i <= n / 2 {
            let mut child = 2 * i;
            if child < n && cmp(&v[lo + child - 1], &v[lo + child]) < 0 {
                child += 1;
            }
            if cmp(&v[lo + i - 1], &v[lo + child - 1]) >= 0 {
                break;
            }
            v.swap(lo + i - 1, lo + child - 1);
            i = child;
        }
    }
    fn intro<T>(v: &mut [T], cmp: &impl Fn(&T, &T) -> i32, lo: usize, mut hi: usize, mut depth: u32) {
        while hi > lo {
            let size = hi - lo + 1;
            if size <= 16 {
                match size {
                    2 => swap_if_greater(v, cmp, lo, hi),
                    3 => {
                        swap_if_greater(v, cmp, lo, hi - 1);
                        swap_if_greater(v, cmp, lo, hi);
                        swap_if_greater(v, cmp, hi - 1, hi);
                    }
                    _ => insertion(v, cmp, lo, hi),
                }
                return;
            }
            if depth == 0 {
                let n = size;
                for i in (1..=n / 2).rev() {
                    down_heap(v, cmp, i, n, lo);
                }
                for i in (2..=n).rev() {
                    v.swap(lo, lo + i - 1);
                    down_heap(v, cmp, 1, i - 1, lo);
                }
                return;
            }
            depth -= 1;
            let mid = lo + (hi - lo) / 2;
            swap_if_greater(v, cmp, lo, mid);
            swap_if_greater(v, cmp, lo, hi);
            swap_if_greater(v, cmp, mid, hi);
            v.swap(mid, hi - 1);
            let (mut l, mut r) = (lo, hi - 1);
            while l < r {
                l += 1;
                while cmp(&v[l], &v[hi - 1]) < 0 {
                    l += 1;
                }
                r -= 1;
                while cmp(&v[hi - 1], &v[r]) < 0 {
                    r -= 1;
                }
                if l >= r {
                    break;
                }
                v.swap(l, r);
            }
            v.swap(l, hi - 1);
            if l + 1 < hi {
                intro(v, cmp, l + 1, hi, depth);
            }
            if l == 0 {
                return;
            }
            hi = l - 1;
        }
    }
    if v.len() > 1 {
        let depth = 2 * (usize::BITS - 1 - v.len().leading_zeros() + 1);
        let hi = v.len() - 1;
        intro(v, cmp, 0, hi, depth);
    }
}

/// sinf / cosf as the game's MSVC C runtime computes them: its FMA3 path (VERIFIED-DISASM sinf
/// @0x4cbe30, cosf @0x4cb8a0), double-precision polynomials with fused steps, not always the
/// correctly rounded float (recording boomerang_500: Glaive Lord's orbit, cos one float step
/// above; Rust's f32 sin / cos are further off, recordings boomerang_*). Arguments past the
/// Cody-Waite range (Payne-Hanek in the runtime) fall back to double sin / cos.
pub(crate) fn crt_sin_cos(a: f32) -> (f32, f32) {
    (ucrt_sinf(a), ucrt_cosf(a))
}

// AMD libm (UCRT) sinf / cosf coefficients.
const UCRT_S: [f64; 4] = [-0.16666666666666666, 0.008333333333333333, -0.0001984126984126984, 2.7557319223985893e-06];
const UCRT_C: [f64; 4] = [0.041666666666666664, -0.0013888888888888887, 2.4801587301587298e-05, -2.755731922398589e-07];
const UCRT_TWO_BY_PI: f64 = 0.6366197723675814;
const UCRT_PIBY2_1: f64 = 1.5707963267341256;
const UCRT_PIBY2_1TAIL: f64 = 6.077100506506192e-11;

/// r + r^3 x S(r^2), Horner with FMA.
fn ucrt_sin_poly(r: f64) -> f64 {
    let x2 = r * r;
    let p = x2.mul_add(UCRT_S[3], UCRT_S[2]).mul_add(x2, UCRT_S[1]).mul_add(x2, UCRT_S[0]);
    p.mul_add(r * x2, r)
}

/// `head` + r^4 x C(r^2), Horner with FMA (`head` is 1 - r^2 / 2, fused or not by caller).
fn ucrt_cos_poly(x2: f64, head: f64) -> f64 {
    let p = x2.mul_add(UCRT_C[3], UCRT_C[2]).mul_add(x2, UCRT_C[1]).mul_add(x2, UCRT_C[0]);
    p.mul_add(x2 * x2, head)
}

/// Cody-Waite reduction by pi/2: (n, r) with |x| = n x pi/2 + r.
fn ucrt_reduce(ax: f64) -> (i64, f64) {
    let n = UCRT_TWO_BY_PI.mul_add(ax, 0.5) as i32;
    let nd = n as f64;
    let r2 = (-nd).mul_add(UCRT_PIBY2_1, ax);
    (n as i64 & 3, r2 - nd * UCRT_PIBY2_1TAIL)
}

fn ucrt_sinf(a: f32) -> f32 {
    let bits = a.to_bits() & 0x7fff_ffff;
    let x = a as f64;
    if bits >= 0x7f80_0000 {
        return f32::NAN;
    }
    if bits <= 0x3f49_0fdb {
        if bits < 0x3c00_0000 {
            if bits < 0x3900_0000 {
                return a;
            }
            return ((x * x) * x).mul_add(-0.16666666666666666, x) as f32;
        }
        return ucrt_sin_poly(x) as f32;
    }
    if bits >= 0x4b80_0456 {
        return x.sin() as f32;
    }
    let (region, r) = ucrt_reduce(x.abs());
    let v = if region & 1 == 1 { ucrt_cos_poly(r * r, (r * r).mul_add(-0.5, 1.0)) } else { ucrt_sin_poly(r) };
    (if (region >= 2) != (x < 0.0) { -v } else { v }) as f32
}

fn ucrt_cosf(a: f32) -> f32 {
    let bits = a.to_bits() & 0x7fff_ffff;
    let x = a as f64;
    if bits >= 0x7f80_0000 {
        return f32::NAN;
    }
    if bits <= 0x3f49_0fdb {
        if bits < 0x3c00_0000 {
            if bits < 0x3900_0000 {
                return 1.0;
            }
            return (-(x * 0.5)).mul_add(x, 1.0) as f32;
        }
        let x2 = x * x;
        return ucrt_cos_poly(x2, 1.0 - x2 * 0.5) as f32;
    }
    if bits >= 0x4f49_0fdb {
        return x.cos() as f32;
    }
    let (region, r) = ucrt_reduce(x.abs());
    let v = if region & 1 == 1 { ucrt_sin_poly(r) } else { let x2 = r * r; ucrt_cos_poly(x2, 1.0 - x2 * 0.5) };
    (if (region + 1) >> 1 & 1 == 1 { -v } else { v }) as f32
}

/// atan2f(y, x) as the game's C runtime gives it (Vector2.get_Rotation: atan2f(x, y) x Rad2Deg;
/// recording boomerang_000: Rust's f32 atan2 two float steps off).
pub(crate) fn crt_atan2(y: f32, x: f32) -> f32 {
    (y as f64).atan2(x as f64) as f32
}

/// Math.MoveTowardsAngle (Unity's): by at most `max` the short way round, landing on `target`.
fn move_towards_angle(current: f32, target: f32, max: f32) -> f32 {
    let d = target - current;
    let mut delta = (d - (d / 360.0).floor() * 360.0).clamp(0.0, 360.0);
    if delta > 180.0 {
        delta -= 360.0;
    }
    if -max < delta && delta < max {
        return target;
    }
    let t = current + delta;
    if (t - current).abs() <= max {
        t
    } else {
        current + (t - current).signum() * max
    }
}

/// Math.LerpAngle (VERIFIED-DISASM @0xea7760): Unity's, Repeat(b - a, 360) taken the short way
/// round, t clamped to 0..1.
fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let d = b - a;
    let mut n = (d - (d / 360.0).floor() * 360.0).clamp(0.0, 360.0);
    if n > 180.0 {
        n -= 360.0;
    }
    a + n * t.clamp(0.0, 1.0)
}

/// BloonProperties.Frozen: set while a freeze effect is active.
pub const FROZEN: u32 = 16;

/// An active bloon effect (built from an `EffectSpec` when a projectile hits).
#[derive(Clone)]
pub struct Effect {
    pub kind: EffectKind,
    pub id: String,
    pub frames_left: f32,
    /// Layers left including this one; children inherit it while this is above 1.
    pub layers: u32,
    pub owner: u64,
    /// Its damage's (distributeToChildren, overrideDistributeBlocker) (damage over time).
    pub distribute: (bool, bool),
    /// BehaviorMutator.dontRemoveOnBloonDegrade.
    pub keep: bool,
    /// Its lifespan when it went on (Mutable.AddMutator: what a refresh resets it to).
    pub total: f32,
    /// The frame (sim_elapsed) its behaviour was made: not processed until the next.
    pub added: u64,
    /// Its mutator's overlayType (display only; empty: none).
    pub overlay: std::sync::Arc<str>,
    /// When its behaviour was made (`Game::effect_seq`): the bloon's behaviour order. A bloon
    /// keeps the behaviours it has when its model changes and adds new ones last, so this differs
    /// from the mutator list's order (the Vec's) once the middle insert moves a mutator.
    pub seq: u64,
}

/// FxHash (rustc's): a fast hasher for the sim's bloon-id maps, looked up thousands of times a
/// frame. Lookups give the same values with any hasher, and nothing depends on these maps'
/// iteration order (the default hasher already randomises it on every run).
#[derive(Default, Clone, Copy)]
struct FxHasher(u64);

impl FxHasher {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

impl std::hash::Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut w = [0u8; 8];
            w[..chunk.len()].copy_from_slice(chunk);
            self.add(u64::from_le_bytes(w));
        }
    }
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }
    fn write_u32(&mut self, i: u32) {
        self.add(i as u64);
    }
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

type FastMap<K, V> = HashMap<K, V, std::hash::BuildHasherDefault<FxHasher>>;
type FastSet<K> = std::collections::HashSet<K, std::hash::BuildHasherDefault<FxHasher>>;

/// The tower phase's target search, made once while no bloon moves (`Game::target_index_mut`)
/// instead of over every bloon for every attack: each track segment's bloons in storage order, and
/// each path's lead bloons per attack filter. Speed only: the targets are the same.
#[derive(Default)]
struct TargetIndex {
    /// [path][segment (distance / 7)] -> bloon indices, ascending.
    segs: Vec<Vec<Vec<u32>>>,
    /// Each bloon's segment numbered across paths (`base[path] + segment`; u32::MAX: none).
    seg_of: Vec<u32>,
    base: Vec<usize>,
    /// (last, camo, no MOABs) -> `Game::path_leads`.
    lead: HashMap<(bool, bool, bool), Vec<Option<(usize, f32, u32)>>>,
    /// (camo, no MOABs) -> `Game::strong_leads`.
    strong: HashMap<(bool, bool), Vec<Option<(usize, f32, f32)>>>,
}

/// BloonManager's mutator ids that Bloon.RemoveAllMutators leaves (IsMutatorIgnoredForRemoval,
/// the set its .cctor @0xfea720 fills).
const KEPT_ON_MOAB_POP: [&str; 19] = [
    "KineticChargeDisplay", "KineticChargeDisplayTinkerfairy", "CarpetSpikes", "ReanimationBlocker", "RemoveBloonProperty",
    "TimeStopBloons", "Gyrfalcon", "Trance", "Haunt", "bombSlow", "DistributeToChildrenSet", "SpiritOfTheForestClose",
    "SpiritOfTheForestFar", "SpiritOfTheForestMedium", "BlackResist", "BossBloonBuffInRadius", "BossBloonBuffSpeed",
    "Stun:BigSqueeze", "Wind",
];

#[derive(Clone)]
pub enum EffectKind {
    Slow { mult: f32, glue: u32 },
    Freeze,
    /// DamageOverTime: `interval` in frames; `tick` is lastTick on the round clock (whole frames).
    /// `mods`: its DamageModifierForTag models (DamageOverTime.DealDamage, per tick on what the
    /// bloon is then).
    /// `start`: Initialise's lastTick offset from the clock (initialDelayFrames; triggerImmediate:
    /// None, lastTick -999999999), used again when a degrade makes its behaviour anew.
    Dot { damage: f32, interval: f32, tick: f32, immune: u32, mods: std::sync::Arc<[TagMod]>, start: Option<f32> },
    Brittle { damage: f32 },
    Emit { interval: f32, tick: f32, proj: Box<Proj> },
    /// EmitOnPopModel: fired where the bloon pops.
    /// EmitOnPop: `ignore` its model's ignoreSameFrameDegrade, `skip` the behaviour's
    /// ignoringSameFrameDegrade (set by Initialise, cleared by Process and the first PreDegrade).
    PopEmit { proj: Box<Proj>, emission: Box<Emission>, ignore: bool, skip: bool },
    /// CarryProjectile: the projectiles it made (uids; empty until Attatched emits them), which
    /// sit where the bloon is and go with the mutator.
    Carry { proj: Box<Proj>, emission: Box<Emission>, uids: Vec<u64> },
    /// EmitOnDestroy: its projectile where the bloon is destroyed; `dont_copy`: not passed to a
    /// degrade's other children.
    DestroyEmit { proj: Box<Proj>, emission: Box<Emission>, dont_copy: bool },
    /// DesperadoMarkModel: x `damage_mult` damage from Desperados, x `cash_mult` pop cash.
    Mark { damage_mult: f32, cash_mult: f32 },
    /// StripChildrenModel: pops without releasing children.
    Strip,
    /// RemoveDamageTypeModifierModel: these bloonProperties don't count while it lasts.
    Unarmor { bits: u32 },
    /// KnockbackMutator (VERIFIED-DISASM @0x12067c0): the bloon model's speed becomes
    /// speed - |m x speed|.
    Knock { m: f32 },
    /// IncreaseBloonWorthWithTierModel: its pop pays `per_tier` x its layer number more.
    Worth { per_tier: f32 },
    /// TranceBloonModel: off the track, circling `centre` at `radius`; `angle` in radians,
    /// `rate` in radians per frame.
    Trance { centre: [f32; 2], radius: f32, angle: f32, rate: f32 },
    /// No effect: blocks the effect id `id` minus "Cooldown" until it runs out.
    Cooldown,
    /// CreateEffectOnPopModel: its effect (name, scale) where the bloon pops (visual only).
    PopFx { fx: std::sync::Arc<str>, scale: f32 },
}

/// RandomPosition's state (VERIFIED-DISASM @0x180dd2fa0 ff.): its shuffled offsets (game
/// frame), the shuffle's draw buffer length, the four lists CheckPositions fills, the index.
pub struct RandPosState {
    pub model: Option<crate::data::RandomPos>,
    pub offsets: Vec<[f32; 2]>,
    pub buf: usize,
    pub lists: [Vec<[f32; 2]>; 4],
    pub index: i32,
    pub dirty: bool,
}

/// RandomPosition.CalculatePossibleOffsets (VERIFIED-DISASM @0x180dd23b0), game frame: rings
/// k x pointDistance + minDistance for k < (int)((max - min) / pointDistance); each from (0, r),
/// then every 2 x asin(pointDistance x 0.5 / r) degrees (90 past that domain) while k x step <
/// (360 - step) + 0.01, (0, 1) Vector2.Rotate-d by it, x r.
fn rand_pos_offsets(m: &crate::data::RandomPos) -> Vec<[f32; 2]> {
    let mut out = Vec::new();
    let rings = ((m.max - m.min) / m.point) as i32;
    for i in 0..rings {
        let r = i as f32 * m.point + m.min;
        out.push([0.0, r]);
        let x = (m.point * 0.5) / r;
        let mut step = 90.0f32;
        if (0.0..=1.0).contains(&x) {
            let a = (x as f64).asin() as f32;
            step = RAD2DEG * a + RAD2DEG * a;
        }
        let limit = (360.0 - step) + 0.01;
        let mut k = 1;
        while (k as f32 * step) < limit {
            let deg = k as f32 * step;
            let (sa, ca) = crt_sin_cos(DEG2RAD * -deg);
            let v = [ca * 0.0 - sa * 1.0, ca * 1.0 + sa * 0.0];
            out.push([v[0] * r, v[1] * r]);
            k += 1;
        }
    }
    out
}

/// Where Mutable.AddMutator (VERIFIED-DISASM @0x180f34f20) puts a new mutator: its
/// SortedListLite insert (@0x181ffcdb0; binary search @0x18014ca90 on Mutable.SortMutators'
/// priority, highest first, an equal one at the probed middle) puts one of equal priority at
/// index count / 2. Slow and AddBehaviorToBloon mutators are all priority 1 (BehaviorMutator
/// ctor @0x180f310c0 from SlowMutator @0x1811ff250 / AddBehaviorToBloonModel.Mutator
/// @0x1811cbd20).
fn mutator_slot(effects: &[Effect]) -> usize {
    effects.len() / 2
}

/// Bloon.CopyMutatorsFromBloon -> Mutable.CopyMutatorsFrom (VERIFIED-DISASM @0x180f35ac0): the
/// source's mutators in its order, each through AddMutator (`mutator_slot`) on the new bloon.
fn copy_order(effects: Vec<Effect>) -> Vec<Effect> {
    let mut out: Vec<Effect> = Vec::new();
    for e in effects {
        let at = out.len() / 2;
        out.insert(at, e);
    }
    out
}

impl Bloon {
    /// Mutable.IsMutatedBy(string) (VERIFIED-DISASM @0x180f36670): a mutator whose id, or (an id
    /// split by ':', BehaviorMutator.Initialise) its idMajor or idMajorMinor, is `s`.
    pub fn mutated_by(&self, s: &str) -> bool {
        self.effects.iter().any(|e| {
            let id = e.id.as_str();
            if id == s {
                return true;
            }
            let mut parts = id.split(':');
            let major = parts.next().unwrap_or("");
            match parts.next() {
                Some(minor) => major == s || (s.len() == major.len() + 1 + minor.len() && s.starts_with(major) && s[major.len()..].starts_with(':') && s.ends_with(minor)),
                None => false,
            }
        })
    }

    /// Whether a projectile with this hit list already hit it or a bloon it came from.
    pub fn hit_by(&self, hit: &HitSet) -> bool {
        !hit.is_empty() && (hit.contains(&self.layer) || self.lineage.as_ref().is_some_and(|l| l.iter().any(|x| hit.contains(x))))
    }
    /// How far along its route it is: earlier tracks plus the distance on this one.
    pub fn progress(&self) -> f32 {
        self.base + self.dist
    }

    pub fn camo(&self, def: &BloonDef) -> bool {
        def.is_camo && !self.decamo
    }

    /// DamageModifierForBloonStateModel states this prototype tracks. INFERRED names: "Ice"
    /// (frozen), "Glue" (glued), "Stun" (a slow with multiplier 0 that isn't glue).
    pub fn in_state(&self, state: &str) -> bool {
        match state {
            "Ice" | "Frozen" => self.frozen(),
            "Glue" => self.glue() > 0,
            // DamageModifierForBloonState.IsCheckPassed (VERIFIED-DISASM @0xcb3000) asks
            // Mutable.GetMutatorsByKey (@0xf36040): mutators whose id contains the state
            // (recording ninja_005 rt 112: "StickyBomb" is Master Bomber's StickyBombSplash, a
            // shuriken does 1 + 19 + 19 to a stunned, stuck MOAB).
            _ => self.effects.iter().any(|e| e.id.contains(state)),
        }
    }

    pub fn has_effect(&self, id: &str) -> bool {
        self.effects.iter().any(|e| e.id == id)
    }

    /// Mutable.IsMutatedBy(string) (VERIFIED-DISASM @0xf55180): a mutator's id, or the parts
    /// BehaviorMutator.SetId splits off it at ':' (idMajor "Ice", idMajorMinor "Ice:Debuff").
    pub fn is_mutated_by(&self, id: &str) -> bool {
        self.effects.iter().any(|e| {
            let mut parts = e.id.split(':');
            let major = parts.next().unwrap_or("");
            e.id == id || major == id || parts.next().is_some_and(|minor| id.strip_prefix(major).and_then(|r| r.strip_prefix(':')) == Some(minor))
        })
    }

    pub fn frozen(&self) -> bool {
        self.effects.iter().any(|e| matches!(e.kind, EffectKind::Freeze))
    }

    pub fn glue(&self) -> u32 {
        self.effects
            .iter()
            .enumerate()
            .filter(|(k, e)| !self.effects[..*k].iter().any(|x| x.id == e.id))
            .map(|(_, e)| if let EffectKind::Slow { glue, .. } = e.kind { glue } else { 0 })
            .max()
            .unwrap_or(0)
    }

    /// HYPOTHESIS P-SLOW: slows with different ids multiply; a freeze stops the bloon (a
    /// MOAB-class one instead gets the speed `frozen_speed` gives it).
    pub fn speed_mult(&self, def: &BloonDef) -> f32 {
        if self.frozen() && !def.tags.iter().any(|t| t == "Moabs") {
            return 0.0;
        }
        // (Mutable.ApplyMutation: only the first mutator with an id applies.)
        self.effects
            .iter()
            .enumerate()
            .filter(|(k, e)| !self.effects[..*k].iter().any(|x| x.id == e.id))
            .map(|(_, e)| if let EffectKind::Slow { mult, .. } = e.kind { mult } else { 1.0 })
            .product()
    }

    /// FreezeModel.Mutator.Mutate (VERIFIED-DISASM @0x1208f10) on a MOAB-class bloon:
    /// BloonModel.set_Speed to 5 for a Moab, Bfb or Ddt and 2.25 for a Zomg, Bad or Miniboss
    /// (when the freeze's speed is below that), half its speed for a Boss under an "IceParagon"
    /// freeze (recording ice_005: a frozen MOAB moves 5 / 60 a frame). INFERRED: other slows
    /// still multiply it.
    pub fn frozen_speed(&self, def: &BloonDef, speed: f32) -> f32 {
        let has = |t: &str| def.tags.iter().any(|x| x == t);
        if !has("Moabs") {
            return speed;
        }
        let Some(e) = self.effects.iter().find(|e| matches!(e.kind, EffectKind::Freeze)) else { return speed };
        if has("Boss") && e.id.starts_with("IceParagon") {
            speed * 0.5
        } else if has("Moab") || has("Bfb") || has("Ddt") {
            5.0
        } else if has("Zomg") || has("Bad") || has("Miniboss") {
            2.25
        } else {
            speed
        }
    }

    /// The bloon model speed after KnockbackMutator.Mutate.
    pub fn model_speed(&self, speed: f32) -> f32 {
        match self.effects.iter().find_map(|e| if let EffectKind::Knock { m } = e.kind { Some(m) } else { None }) {
            Some(m) => speed - (m * speed).abs(),
            None => speed,
        }
    }

    pub fn bonus_damage(&self) -> f32 {
        self.effects.iter().map(|e| if let EffectKind::Brittle { damage } = e.kind { damage } else { 0.0 }).sum()
    }

    pub fn properties(&self, def: &BloonDef) -> u32 {
        let removed = self.effects.iter().fold(0, |m, e| if let EffectKind::Unarmor { bits } = e.kind { m | bits } else { m });
        (def.bloon_properties | if self.frozen() { FROZEN } else { 0 }) & !removed
    }
}

/// CollisionGrid cells (CollisionChecker): 10 x 10 from (worldXMin - 52, worldZMin - 52).
const GAME_GRID_CELL: f32 = 10.0;

/// The bloon-grid cell of a clone-space point (the game's y is the clone's -y).
fn grid_cell(p: [f32; 2]) -> (i32, i32) {
    (((p[0] - (-150.0 - 52.0)) / GAME_GRID_CELL) as i32, ((-p[1] - (-116.0 - 52.0)) / GAME_GRID_CELL) as i32)
}

/// Math.FindIntersectionLineLine (VERIFIED-DISASM @0xea6f20): where segment p1-p2 crosses
/// segment a-b, in f32 as the game has it.
fn segment_cross(p1: [f32; 2], p2: [f32; 2], a: [f32; 2], b: [f32; 2]) -> Option<[f32; 2]> {
    let (dx, dy) = (p2[0] - p1[0], p2[1] - p1[1]);
    let den = (b[1] - a[1]) * dx - (b[0] - a[0]) * dy;
    if den == 0.0 {
        return None;
    }
    let inv = 1.0 / den;
    let s = inv * ((a[0] - p1[0]) * dy - dx * (a[1] - p1[1]));
    let t = inv * ((a[0] - p1[0]) * (b[1] - a[1]) - (b[0] - a[0]) * (a[1] - p1[1]));
    if t < 0.0 || t > 1.0 || s < 0.0 || s > 1.0 {
        return None;
    }
    Some([t * dx + p1[0], t * dy + p1[1]])
}

/// Vector2.Normalize: zero when shorter than the smallest float.
fn normalize2(v: [f32; 2]) -> [f32; 2] {
    let m = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if m < f32::from_bits(1) { [0.0, 0.0] } else { [v[0] / m, v[1] / m] }
}

/// ProjectileBlockerCollisionRebound against one area: Area.Intersects (the outline's edges in
/// order, then the holes'; Polygon.Intersects @0xeaada0) for a move from outside the area;
/// the new position and direction.
fn blocker_bounce(area: &Area, last: [f32; 2], pos: [f32; 2]) -> Option<([f32; 2], [f32; 2])> {
    let edge = std::iter::once(&area.points).chain(area.holes.iter()).find_map(|poly| {
        (0..poly.len()).find_map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            segment_cross(last, pos, a, b).map(|h| (a, b, h))
        })
    });
    let (a, b, hit) = edge?;
    if Game::in_area(last, area) {
        return None;
    }
    let v = [pos[0] - last[0], pos[1] - last[1]];
    let mag = (v[0] * v[0] + v[1] * v[1]).sqrt();
    let n = normalize2([b[0] - a[0], b[1] - a[1]]);
    let dot = n[1] * v[1] + n[0] * v[0];
    let t = [n[0] * 2.0 * dot, n[1] * 2.0 * dot];
    let dir = normalize2([t[0] - v[0], t[1] - v[1]]);
    let back = [last[0] - hit[0], last[1] - hit[1]];
    let k = mag - (back[0] * back[0] + back[1] * back[1]).sqrt();
    Some(([hit[0] + dir[0] * k, hit[1] + dir[1] * k], dir))
}

/// The rest of the CollisionGrid enumerator's test (VERIFIED-DISASM @0x1825db980): a bloon with
/// AdditionalCollisions (MOAB-class circles: offset x, offset y, radius) is only hit through one
/// of them, centred on its position plus the offset turned by its rotation (precalculatedOffset,
/// as Targetting.CanHitMoab), within the circle's radius + the query's (recording dart_050_club: a
/// dart passing a MOAB's body inside its 21 radius but outside both circles flies on).
fn circles_hit(q: [f32; 2], qr: f32, b: [f32; 2], rot: f32, circles: &[[f32; 3]]) -> bool {
    if circles.is_empty() {
        return true;
    }
    let a = DEG2RAD * -rot;
    let (sa, ca) = crt_sin_cos(a);
    circles.iter().any(|c| {
        let (ox, oy) = (ca * c[0] - sa * c[1], ca * c[1] + sa * c[0]);
        let (dx, dy) = (b[0] + ox - q[0], -(b[1] - oy) + q[1]);
        let r = c[2] + qr;
        dx * dx + dy * dy <= r * r
    })
}

/// (int)((double)(r / 10) + 0.5): a radius in whole grid cells (CollisionState.radiusCells,
/// Bucket.largestCellSpan).
fn cell_span(r: f32) -> i32 {
    ((r / GAME_GRID_CELL) as f64 + 0.5) as i32
}

/// CollisionGrid query enumerator (VERIFIED-DISASM @0x1825db980): every bloon in the query's
/// own cell counts, and every bloon in a cell whose circumscribed square lies inside the query's
/// box; elsewhere distance <= query radius + bloon radius.
fn grid_hit(q: [f32; 2], qr: f32, b: [f32; 2], b_cell: [f32; 2], br: f32) -> bool {
    let rr = qr + br;
    let dd = (b[0] - q[0]).powi(2) + (b[1] - q[1]).powi(2);
    if dd <= rr * rr {
        return true;
    }
    // Beyond reach: only a shared cell (at most a cell diagonal apart, give or take the frame's
    // move between `b` and `b_cell`) or a cell inside a big query's box can still count.
    let far = (b_cell[0] - q[0]).powi(2) + (b_cell[1] - q[1]).powi(2);
    if far > (GAME_GRID_CELL * std::f32::consts::SQRT_2 + 0.01).powi(2) && qr < 200f32.sqrt() {
        return false;
    }
    let (qc, bc) = (grid_cell(q), grid_cell(b_cell));
    if qc == bc {
        return true;
    }
    // Cell centre in game space; extent sqrt(200) (Constants).
    let ext = 200f32.sqrt();
    let cx = -202.0 + bc.0 as f32 * GAME_GRID_CELL + GAME_GRID_CELL * 0.5;
    let cy = -168.0 + bc.1 as f32 * GAME_GRID_CELL + GAME_GRID_CELL * 0.5;
    let (gx, gy) = (q[0], -q[1]);
    if gx - qr < cx - ext && cx + ext < gx + qr && gy - qr < cy - ext && cy + ext < gy + qr {
        return true;
    }
    false
}

/// After bloon `i` is swap-removed from a list of `len0`, the old last bloon is at `i`: later
/// candidates (after position `k`) that named it follow it there.
fn remap_moved(cand: &mut [u32], k: usize, i: usize, len0: usize) {
    #[cfg(test)] let _remap_timer=sim_profile::Timer::new(2);
    if i + 1 == len0 {
        return;
    }
    // A bloon is in a candidate list once (it sits in one grid cell), so the first match is the only one.
    if let Some(j) = cand.iter().skip(k + 1).position(|&c| c as usize == len0 - 1) {
        cand[k + 1 + j] = i as u32;
        #[cfg(test)]
        assert!(!cand[k + 2 + j..].iter().any(|&c| c as usize == len0 - 1), "a bloon listed twice in collision candidates");
    }
}

/// BloonModel.set_Speed (VERIFIED-DISASM @0x1155860): speedFrames = speed x (1/60 in f32), a
/// multiply (not speed / 60: the last bit differs, and it shows over many frames).
fn speed_frames(speed: f32) -> f32 {
    speed * (1.0f32 / 60.0)
}

/// Vector3.AddDisplaySpaceHeight: a height h is h / tan 60 on the map (the camera's angle).
/// (The game caches tanf(Deg2Rad * 60) in f32: 1.7320509, not the nearest f32 to tan 60; VERIFIED by
/// recording, eject points.)
const TAN60: f32 = 1.7320509;
/// Math.Rad2Deg, Math.Deg2Rad, and TravelStraitModel's 1/60 (speedFrames = speed x it).
const RAD2DEG: f32 = 57.29578;
const DEG2RAD: f32 = 0.017453292;
const FRAME_TIME: f32 = 0.016666668;

/// Vector2.Rotate((0, 1), deg) in the clone's frame (the game's y flipped): a projectile's
/// direction for a rotation in the game's degrees.
fn game_dir(deg: f32) -> [f32; 2] {
    let a = DEG2RAD * -deg;
    // (UCRT sinf / cosf, as Vector2.Rotate: recordings bomb_032 / 502, a blast's frags.)
    let (s, c) = crt_sin_cos(a);
    [0.0 * c - s, -(c + s * 0.0)]
}

/// RotateToTarget.ApplyRotation: towers aim from 10 above their position.
const DISPLAY_TOWER_HEIGHT: f32 = 10.0;

/// lastEmission of a weapon that never fired (int.MinValue: ready at once).
const NEVER: i64 = i32::MIN as i64;

/// Simulation.Towers.Weapons.Weapon state.
#[derive(Clone, Debug)]
pub struct WeaponState {
    /// Vagrant's bloon zone (Zoned<Bloon>.inZone) as of its last Process: bloon ids.
    pub zone_in: Option<Vec<u64>>,
    /// Its index in the game's weapon list (the tower's attacks' weapons, then every ability's,
    /// in ability order): what the recorder's W lines number it by.
    pub slot: usize,
    /// Weapon name and occurrence (the k-th of that name): how a model change finds the same
    /// weapon.
    pub key: (String, u32),
    /// lastEmission: when the last throw started, on the weapon's clock (roundTime, or
    /// Simulation.time for fireBetweenRounds: `sim_elapsed`, not `steps`; recording desperado_005,
    /// the shotgun's 18 at rt 73 = time 88 + 12 - 82).
    pub last: i64,
    /// animationStartedAt (Simulation.time) of a throw whose projectile hasn't left yet; -1.
    pub anim: i64,
    /// Where that throw was aimed, and at which bloon (0: a point): the throw stays on the
    /// target it started with (recorded: the tower keeps turning to it as it leaves range).
    pub aim_at: [f32; 2],
    pub aim_id: u64,
    /// LimitProjectile.limitEmissionUntilTime (Simulation.time).
    pub limit_until: i64,
    /// Target's offset from its bloon (Targetting.CanHitMoab: the collision circle a MOAB was
    /// seen by); the throw leaves towards the bloon's position plus it.
    pub aim_off: [f32; 2],
    /// Which of the MOAB's collision circles that was (its shots steer at it).
    pub aim_circle: Option<u8>,
    /// The attack's Target's factory object.
    pub aim_obj: Option<u32>,
    pub rate_frames: i64,
    pub missed: f32,
    /// Weapon.bonusDamageCarryover (ApplyExtraDamagePrecision): what a weapon faster than 1/120 s
    /// is owed in whole extra shots' damage; 0 when the weapon is made.
    pub dmg_carry: f32,
    /// What the rate was last calculated from: the tower's buffs (its mutated model), and the
    /// weapon behaviours as of the last shot (distance bonus, leak danger, necromancer stacks).
    buff_rate: f32,
    /// WeaponModel.rate the rate was last calculated from (an upgrade is a model change too).
    model_rate: f32,
    dist_k: f32,
    /// Standoff.GetRate's divisor as of the last CalcRateFrames.
    standoff_k: f32,
    /// VagrantWeaponBehavior's towers and bloons in its zones, as of the last shot.
    vagrant_n: (u32, u32),
    /// TheBlazingSun: currentStacks and lastEmissionAt (roundTime).
    sun_stacks: i32,
    sun_last: i64,
    danger: f32,
    necro: f32,
    /// EmissionsPerRoundFilterModel: throws this round.
    pub round_shots: u32,
    /// BurstWeaponBehavior.currentCount: shots left in this burst (Initialise: the model's count).
    pub burst_left: u32,
    /// BurstWeaponBehavior.Process's overrideTarget (Weapon+0xdc) with alwaysEmptyClip: the point
    /// its attack range ahead along the tower's rotation as this weapon's frame ended.
    pub empty_pt: Option<[f32; 2]>,
    /// The range its Vagrant bloon zone was last processed with.
    pub zone_r: f32,
    /// Its throws (EmissionWithOffsets takes its markers in turn per weapon: recording sub_052,
    /// a Sub's twin guns keep their order through its Ballistic Missile's launches).
    pub emits: u32,
    /// CritMultiplier: (current, nextCritAt); (0, 0) until the weapon has a crit.
    pub crit: (u32, u32),
    /// Weapon.fractionalPierce: the projectiles' fractional pierce adds up here.
    pub frac_pierce: f32,
}

impl WeaponState {
    fn new(key: (String, u32), w: &crate::data::Weapon, clock: i64) -> Self {
        let mut s = WeaponState {
            slot: 0,
            zone_in: None,
            key,
            last: NEVER,
            anim: -1,
            aim_at: [0.0; 2],
            aim_id: 0,
            limit_until: 0,
            aim_off: [0.0; 2],
            aim_circle: None,
            aim_obj: None,
            emits: 0,
            rate_frames: 1,
            missed: 0.0,
            dmg_carry: 0.0,
            buff_rate: f32::NAN,
            model_rate: f32::NAN,
            dist_k: 1.0,
            standoff_k: 1.0,
            vagrant_n: (0, 0),
            sun_stacks: 0,
            sun_last: 0,
            danger: 0.0,
            necro: 1.0,
            round_shots: 0,
            burst_left: w.burst.as_ref().map_or(0, |b| b.count),
            empty_pt: None,
            zone_r: 0.0,
            crit: (0, 0),
            frac_pierce: 0.0,
        };
        // Weapon.ApplyStartingCooldown (VERIFIED-DISASM @0xc6d7f0): startInCooldown waits a full
        // interval; customStartCooldown that long instead (against the model's rateFrames).
        if w.start_cooldown {
            s.last = clock;
            let custom = (w.custom_start * 60.0) as i32 as i64;
            if custom > 0 {
                s.last = clock + custom - (w.rate * 60.0) as i32 as i64;
            }
        }
        s
    }

    fn ready(&self, clock: i64) -> bool {
        self.last + self.rate_frames <= clock
    }

    /// Weapon.CalcRateFrames' rate: the (mutated) model rate through the weapon behaviours,
    /// then WeaponRateMin.
    fn rate(&self, w: &crate::data::Weapon) -> f32 {
        let mut r = w.rate * self.buff_rate;
        // VagrantWeaponBehavior.GetRate (VERIFIED-DISASM @0xcb69e0): rate / (max(0, noTowersRateBonus
        // - towers x towerRangeBonusReduction) + 1), then / (min(bloons x bloonInRangeAttackSpeedBuff,
        // maxBloonAttackSpeedBuff) + 1), each only when its two values aren't both 0.
        if let Some([a, b, c, d]) = w.vagrant {
            if a != 0.0 || b != 0.0 {
                r /= (a - self.vagrant_n.0 as f32 * b).max(0.0) + 1.0;
            }
            if c != 0.0 || d != 0.0 {
                r /= (self.vagrant_n.1 as f32 * c).min(d) + 1.0;
            }
        }
        // TheBlazingSun.GetRate (VERIFIED-DISASM @0x180c95dd0), after Standoff in the behaviour
        // list (recording desperado_500 with SimRecorder's Q lines): rate / (currentStacks x
        // attackSpeedPerStack + 1), attackSpeedPerStack -0.01 (slower as it heats up).
        let sun = w.blazing_sun.map_or(1.0, |[_, _, per, _]| if self.sun_stacks != 0 { self.sun_stacks as f32 * per + 1.0 } else { 1.0 });
        let r = r / self.dist_k / self.standoff_k / sun / (1.0 + self.danger * w.leak_danger) * self.necro;
        if w.rate_min > r {
            w.rate_min
        } else {
            r
        }
    }
}

/// Weapon.CalcRateFrames (VERIFIED-DISASM @0xc6d860): at least 1/120 s; up to 0.1 s rounds the
/// frames up, longer rounds half to even (Mathf.RoundToInt); at least one frame.
fn rate_frames(rate: f32) -> i64 {
    let rate = rate.max(1.0 / 120.0);
    let x = rate * 60.0;
    let f = if rate <= 0.1 { (x as f64).ceil() as i64 } else { x.round_ties_even() as i64 };
    f.max(1)
}

/// Weapon.UpdateMissedFrames (VERIFIED-DISASM @0xc70ab0): what rounding up a fast weapon's
/// frames loses, as a fraction of a shot.
fn missed_rate(rate: f32) -> f32 {
    let rate = rate.max(1.0 / 120.0);
    if rate > 0.1 {
        return 0.0;
    }
    let x = rate * 60.0;
    (x.ceil() - x) / x
}

/// Simulation.Towers.Behaviors.Orbit state (VERIFIED-DISASM @0xd6da00): the angle the first
/// glaive is at, the turn speed (degrees a frame), the bloon it saw in range (bloonInRange) and
/// the glaives' positions (projectiles of their own in the game, display only) and how many frames
/// they have existed (their RotateModel spin, display only).
#[derive(Clone, Debug)]
pub struct OrbitState {
    pub angle: f32,
    pub speed: f32,
    pub seen: Option<u64>,
    pub glaives: Vec<[f32; 2]>,
    pub age: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TargetMode {
    First,
    Last,
    Close,
    Strong,
    /// TargetPointerModel: aims at the cursor (Dartling "Normal").
    Pointer,
    /// TargetSelectedPointModel: aims at a point the player sets (Mortar, Dartling "Locked").
    Point,
    /// PursuitSettingModel (Heli Pilot): the air unit chases the first bloon.
    Pursuit,
    /// Monkey Ace flight patterns.
    Circle,
    FigureEight,
    Infinite,
    /// CenterElipsePatternModel: an ellipse round a point the player sets.
    Centered,
    /// Spike Factory track modes: TargetTrackModel (random spot in range), CloseTargetTrackModel
    /// (nearest spot), SmartTargetTrackModel, AutoTargetTrackModel.
    Track,
    CloseTrack,
    SmartTrack,
    AutoTrack,
    /// SubmergeModel (Sub): the tower submerges and uses its submerge attack instead.
    Submerge,
    /// TargetEliteTargettingModel (Elite Sniper): bloons past percentageThroughMap first, then
    /// the most dangerous.
    Elite,
}

impl TargetMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "First" => Some(Self::First),
            "Last" => Some(Self::Last),
            "Close" => Some(Self::Close),
            "Strong" => Some(Self::Strong),
            "Pointer" => Some(Self::Pointer),
            "Point" => Some(Self::Point),
            "Pursuit" => Some(Self::Pursuit),
            "Circle" => Some(Self::Circle),
            "FigureEight" => Some(Self::FigureEight),
            "Infinite" => Some(Self::Infinite),
            "Centered" => Some(Self::Centered),
            "Track" => Some(Self::Track),
            "CloseTrack" => Some(Self::CloseTrack),
            "SmartTrack" => Some(Self::SmartTrack),
            "AutoTrack" => Some(Self::AutoTrack),
            "Submerge" => Some(Self::Submerge),
            "Elite" => Some(Self::Elite),
            _ => None,
        }
    }

    /// Aims at a point rather than at a bloon.
    pub fn aims_at_point(self) -> bool {
        matches!(self, Self::Pointer | Self::Point)
    }

    /// Not a bloon-targeting mode (aims at a point, or steers a Heli).
    pub fn steers(self) -> bool {
        !matches!(self, Self::First | Self::Last | Self::Close | Self::Strong | Self::Submerge | Self::Elite)
    }

    /// Targeting button text (the Dartling's customName "Normal" / "Locked").
    pub fn label(self) -> &'static str {
        match self {
            Self::First => "First",
            Self::Last => "Last",
            Self::Close => "Close",
            Self::Strong => "Strong",
            Self::Pointer => "Normal",
            Self::Point => "Locked",
            Self::Pursuit => "Pursuit",
            Self::Circle => "Circle",
            Self::FigureEight => "Figure Eight",
            Self::Infinite => "Infinite",
            Self::Centered => "Centered Path",
            // TT_Track / TT_CloseTrack / TT_SmartTrack / TT_AutoTrack.
            Self::Track => "Normal",
            Self::CloseTrack => "Close",
            Self::SmartTrack => "Smart",
            Self::AutoTrack => "Automatic",
            Self::Submerge => "Submerge",
            Self::Elite => "Elite",
        }
    }

    /// Button text for a Heli's movement mode.
    pub fn air_label(self) -> &'static str {
        match self {
            Self::Pointer => "Follow Touch",
            Self::Point => "Lock In Place",
            s => s.label(),
        }
    }
}

/// AnalyticsGameSessionInfo as SinglePlayerStatsPanel shows it: towersPlaced (the player's
/// placements), bloonsPopped and damageDealt (every layer and point of damage the player's towers
/// are credited with), ecoEarned (what the towers' cash drops pay when collected), MOAB-class bloons
/// popped, totalCashSpent (what the player paid for towers, upgrades and Paragons) and abilities
/// the player activated. Real games: three Darts on rounds 1-2 (recording end_victory_towers)
/// showed 3 / 53 / 53; rounds 39-41 on Medium (end_victory_stats: a 0-0-3 Farm and a 0-4-0 Dart
/// upgraded, two towers placed free, abilities fired by the recorder, not the player) showed
/// 4 placed, 1,260 eco, 1 MOAB, 0 abilities, 11,290 spent (just the upgrades).
#[derive(Clone, Copy, Debug, Default)]
pub struct SessionStats {
    pub towers_placed: u32,
    pub pops: u64,
    pub damage: u64,
    pub eco: f64,
    pub cash_spent: f64,
    pub abilities: u32,
    /// Bloon.UpdateTracking, once per hit that pops layers, by the hit bloon's own type: layers
    /// popped off camo / regrow bloons, and one per Lead, Ceramic, MOAB, BFB, ZOMG, DDT, BAD and
    /// boss popped (a ZOMG destroyed outright counts once, not its children).
    pub camo_pops: u64,
    pub regrow_pops: u64,
    pub lead: u32,
    pub ceramic: u32,
    pub moab: u32,
    pub bfb: u32,
    pub zomg: u32,
    pub ddt: u32,
    pub bad: u32,
    pub boss: u32,
    /// towersPlacedByTowerSet: placements per set (Primary, Military, Magic, Support, Hero).
    pub sets: [u32; 5],
}

impl SessionStats {
    /// MOAB-class bloons popped (the end screens' MOABs Popped): moabs + bfbs + zomgs + ddts +
    /// bads + bosses.
    pub fn moabs(&self) -> u64 {
        (self.moab + self.bfb + self.zomg + self.ddt + self.bad + self.boss) as u64
    }

    /// CalloutWellRounded.DataToGet: with Primary, Military, Magic and Support all placed, the
    /// fewest placements of any set placed; else 0.
    pub fn well_rounded(&self) -> u32 {
        if self.sets[..4].iter().any(|&n| n == 0) {
            return 0;
        }
        self.sets.iter().copied().filter(|&n| n > 0).min().unwrap_or(0)
    }
}

/// A Paragon purchase, for its effects (ParagonTower.StartSacrifice / Process / Finish).
#[derive(Clone, Debug)]
pub struct Sacrifice {
    pub tower: u64,
    pub at: [f32; 2],
    /// Each sacrificed tower: position, cash invested, a tier 5.
    pub orbs: Vec<([f32; 2], f64, bool)>,
    /// Sim frames: the sacrifice's first (orbs start moving) and when the Paragon activates.
    pub start: u64,
    pub active_at: u64,
    pub degree: u32,
}

pub struct Tower {
    pub id: u64,
    /// RandomPosition per attack (by model name).
    pub randpos: Vec<(String, RandPosState)>,
    /// Spin's angle per spinning weapon (by name): (name, angle, angle before this frame's step).
    pub spin: Vec<(String, f32, f32)>,
    /// Layers popped by this tower's projectiles.
    pub pops: u64,
    /// Health its projectiles took off bloons (the panel's Damage: hits on a MOAB count before
    /// it pops).
    pub damage: u64,
    /// The info panel's advanced stats: Round Placed, Cash Earnt, Lives Gained.
    pub placed_round: usize,
    pub cash_earned: f64,
    pub lives_gained: f64,
    pub def: usize,
    pub pos: [f32; 2],
    pub tiers: [u8; 3],
    pub invested: f64,
    pub target: TargetMode,
    /// The point a TargetSelectedPointModel attack aims at (Mortar target, Dartling lock).
    pub point: Option<[f32; 2]>,
    /// The point's height: PathSegment's pointHeight (PointInfo z + 2, VERIFIED-DISASM ctor
    /// @0xf6d050) when it came from a segment, else 0.
    pub point_z: f32,
    /// Per (attack, weapon) Weapon state (timing, per-round count, crit count).
    pub weapons: Vec<WeaponState>,
    pub shots: u32,
    pub angle: f32,
    /// Direction of the last aimed shot (display only): what a turret that turns without its
    /// body (Engineer sentries) faces.
    pub aim: f32,
    /// Its rotation in the game's frame (degrees) as of its last aimed shot: what emissions it
    /// owns start from (ArcEmission.GetDirection reads the tower's rotation).
    pub game_rot: Option<f32>,
    /// ChurchillBaseRotation (Apex Plasma Master): Tower.Rotation, the turret's angle on it (set
    /// by RotateToTarget.ApplyRotation: aim - Tower.Rotation; the turret turns with the base), and
    /// (pausedRotation, elapsedPaused, rotateToTarget, rotationTargetAngle).
    pub base_rot: f32,
    /// A projectile-made sub-tower's rotation (CreateTowerModel.useProjectileRotation).
    pub sub_base: Option<f32>,
    pub turret_rel: f32,
    pub base_state: (bool, u64, bool, f32),
    /// Per attack: its last aim (radians) and target (display only): each of Robo Monkey's
    /// arms aims on its own (attack displays), the body at the middle of their targets.
    pub aims: Vec<(f32, Option<[f32; 2]>)>,
    /// Seconds since the last attack started (display only).
    pub attack_t: f32,
    /// Seconds spent idle (display only; drives the idle loop).
    pub idle_t: f32,
    /// Monkey Bank balance.
    pub bank: f64,
    /// Alchemist brews on this tower.
    pub brews: Vec<ActiveBrew>,
    /// Set for sub-towers (Engineer sentries): not selectable, upgradable or sellable.
    pub sub: Option<SubTower>,
    /// The Heli's air unit; its attacks aim and fire from here.
    pub air: Option<AirUnit>,
    /// Frames until each ability (by name) is ready again.
    pub ability_cd: HashMap<String, f32>,
    /// Ability.activationsThisRound, by ability name (reset by OnRoundStartSim).
    pub ability_round: HashMap<String, u32>,
    /// Attacks switched on by abilities (ActivateAttackModel), after the tier's own.
    pub temp: Vec<TempAttack>,
    pub turbo: Option<ActiveTurbo>,
    /// Monkey Fan Club: the boost and frames left.
    pub fan: Option<(crate::data::FanClub, f32)>,
    /// Total Transformation: the tier and model it's turned into, and frames left.
    pub morph: Option<(std::sync::Arc<Tier>, Option<String>, f32)>,
    /// ActivateAbilityAfterIntervalModel: frames since its ability last went off by itself.
    pub auto_t: f32,
    /// SwitchDisplayModel: model name and frames left.
    pub look: Option<(String, f32)>,
    /// Frames left with the tower's own attacks off (MutateRemoveAllAttacks...).
    pub mute: f32,
    /// Overclock / Take Aim on this tower: (interval multiplier, Village range multiplier, range
    /// multiplier, frames left).
    pub overclock: Option<(f32, f32, f32, f32)>,
    /// IncreaseRangeModel: (multiplier, added range, frames left).
    pub range_up: Option<(f32, f32, f32)>,
    /// Ultraboost stacks: (count, interval multiplier and Village range multiplier per stack).
    pub ultra: (u32, f32, f32),
    /// Skywarden Focus stance (else Spread).
    pub focus: bool,
    /// DamageBasedAttackSpeedModel: pops seen, damage towards the next stack, stacks, idle frames.
    pub rage: (u64, f32, u32, u32),
    /// Support zones switched on by abilities, with frames left.
    pub temp_supports: Vec<(crate::data::Support, f32, u64)>,
    /// Sniper ammo crate: (frames until it applies, shots left, the boost).
    pub ammo: Option<(f32, u32, crate::data::AmmoSwap)>,
    /// Heroes: XP towards the next level (`Hero.xp`, reset by each level-up's cost).
    pub xp: f32,
    /// A Paragon: its degree and its tier with the degree's stat changes applied.
    pub paragon: Option<(u32, std::sync::Arc<Tier>)>,
    /// ParagonTower.activeAt (Simulation.time): its attacks stay paused until after it.
    pub paragon_active_at: u64,
    /// The first frame it is a Paragon: its attacks still act that frame (the pause starts after).
    pub paragon_first: u64,
    /// The attack model names it had before it became a Paragon (those Attacks carry on).
    pub paragon_kept: Vec<String>,
    /// When each of its Attacks (by AttackModel name) was made, as Bloon.born: its place in the
    /// process list. An upgrade keeps an Attack of the same name; a new one is made then.
    pub attack_born: HashMap<String, u64>,
    /// Orbit (Glaive Lord): its circling glaives.
    pub orbit: Option<OrbitState>,
    /// Running ability boosts to its projectiles, with frames left.
    pub boosts: Vec<(crate::data::ProjBoost, f32)>,
    /// Standing on a ship's deck (AddMakeshiftAreaModel): the ship's id and the deck height.
    pub deck: Option<(u64, f32)>,
    /// Geraldo items on it: (item index, rounds left).
    pub items: Vec<(usize, u32)>,
    /// Highest tier any path may reach (Worn Hero's Cape Super Monkeys: 3).
    pub tier_cap: u8,
    /// A one-off animation the game triggered (the temple opening: Animator state
    /// SunTempleUpgrade / SunTempleUpgradeTier5) and seconds since (display only).
    pub show: Option<(String, f32)>,
    /// CreateEffectWhileAttacking: the attack has a target (the loop effect plays).
    pub attacking_fx: bool,
    /// Attack.hadValidTarget by attack model name: what its last FindTarget (when its weapon was
    /// ready) found.
    pub had_target: HashMap<String, bool>,
    /// A running Marked To Pop.
    pub mtp: Option<MtpState>,
    /// Comanche Defense: frames until it can call in again, call-ins this round.
    pub comanche: (f32, u32),
    /// VigilanteTowerBehavior.lastActivatedAt (roundTime); None before the first.
    pub vigilante_at: Option<i64>,
    /// VagrantWeaponBehavior's zone counts (towers, bloons) as last updated (once a frame, as the
    /// tower is processed).
    pub vagrant_zone: Option<(u32, u32)>,
    /// Necromancer graveyard: RBE gathered per round (round, rbe); spent on undead bloons.
    pub grave: Vec<(usize, f32)>,
    /// NecromancerTargetTrackWithinRange.pathsInRange is filled by GeneratePoints, which runs
    /// only from TargetTrack.OnRoundSet (vtable slot 135), on the round events Initialise
    /// subscribes it to: a Necromancer has no track to target, and so never fires, until a
    /// round ends or is set (recording wizard_004: placed as round 40 starts, first undead when
    /// it ends; a round start alone doesn't do it).
    pub necro_points: bool,
    /// The clocks (steps, round clock) when the tower was placed or last upgraded: input comes
    /// before Simulation.Simulate advances them, so weapons the new model adds start their
    /// cooldown (Weapon.ApplyStartingCooldown) from these, one frame before the tower phase
    /// that builds them (recording wizard_020: Wall of Fire lastEmission -360 = 0 + 30 - 390).
    pub model_clock: Option<(i64, i64)>,
    /// `steps` when it was placed: support mutators reach it (and go out from it) from the next
    /// frame (recording ninja_032: its rt 1 Caltrops are made with 6 pierce, later ones 6.16).
    pub placed_step: u64,
    /// The support set its TargetTrack points were made under: a change remakes them.
    pub track_buffs: u64,
    /// The TargetTrack attack's supplier inputs (centre, range, no MOAB, through walls).
    pub track_args: Option<([f32; 2], f32, bool, bool)>,
    /// RBE spent on undead this second (maxRbeSpawnedPerSecond) and the step that second began.
    pub grave_spent: (f32, u64),
    /// NecromancerTargetTrackWithinRange: currentPathIndex, pathIndexToSet (copied over at
    /// SetUpAttack), the player's selectedPathSegments (track, segment), and the segment its
    /// target is on now.
    pub necro_cur: usize,
    pub necro_next: usize,
    pub necro_sel: Option<(u8, usize)>,
    pub necro_seg: Option<(u8, usize)>,
    /// NecromancerEmission.path / pathSegment: the last target's (kept when a target has none).
    pub necro_emit: Option<(u8, usize)>,
    /// NecromancerEmission.currentDamageBonus: the DamageMutator on the tower (necro_rate_update).
    pub necro_dmg: f32,
    /// TrackTargetSupplier.targetPathSegments (shuffled) and TargetTrack's cursor into it, with
    /// the tiers it was made for (a new tier's Attack makes a new list).
    pub track_supply: Option<(Vec<[f32; 2]>, usize, [u8; 3])>,
    /// The off hand's target mode (TargetLeftHandModel: Robo Monkey's second arm).
    pub left_target: TargetMode,
    /// TargetCamo.prioritiseCamo: the upgrade panel's camo priority toggle is on.
    pub camo_prio: bool,
    /// Submerged (TSMThemeMonkeySub's submerge button): the SubmergeModel attack replaces its own.
    pub submerged: bool,
    /// TSMThemeMonkeyAce reverse: flies its pattern backwards; `reverse_at` the step it last
    /// reversed (the button waits minFramesSinceReverse, 300 frames, between presses).
    pub air_reverse: bool,
    pub reverse_at: u64,
    /// FlipFollowPath.isFlipped: throws with the left hand (its path mirrored, the eject point
    /// moved; recording boomerang_000_left); the right hand is the path as authored.
    pub flip: bool,
    /// Sun Temple / True Sun God: the temple groups its sacrifices earned, and its tier with
    /// them applied.
    pub temple: Option<(Vec<crate::data::TempleGroup>, std::sync::Arc<Tier>)>,
}

/// MarkedToPop's state (Activate @0x180e3b6d0 / Process @0x180e3c0b0).
#[derive(Clone, Default)]
pub struct MtpState {
    /// 1 marking, 2 executing.
    pub phase: u8,
    pub act_at: i64,
    pub exec_at: i64,
    /// [markingTimeFrames, executionTimeMaxFrames].
    pub frames: [f32; 2],
    /// FilterMarkedToPop.lineageToFilter of the marking and the execution attack.
    pub lin: [Vec<u64>; 2],
}

pub struct TempAttack {
    pub attack: crate::data::Attack,
    pub frames_left: f32,
    /// Removed after its first volley (isOneShot).
    pub one_shot: bool,
    /// The tower's own attacks pause while it runs (turnOffExisting).
    pub pauses: bool,
    /// Frames before it starts.
    pub delay: f32,
    /// ActivateAttack.Activate runs it at the activation (processOnActivate or isOneShot).
    pub now: bool,
    /// The round clock its lifespan was last counted at.
    pub counted: Option<i64>,
    /// Its first weapon's place among the ability weapons as the game lists them: every ability's
    /// ActivateAttack weapons are dependants from the start, in the abilities' order (recording
    /// sniper_050: Tier5AmmoSwap's never-firing one before the Supply Drop's).
    pub slot: usize,
}

/// One ability button: every tower's copy of the ability `name`.
pub struct AbilityGroup {
    pub name: String,
    /// A tower that has it (for the icon).
    pub tower: usize,
    pub ready: usize,
    pub total: usize,
    /// Cooldown left on the soonest copy, as a fraction and in seconds.
    pub frac_left: f32,
    pub secs_left: f32,
}

pub struct ActiveTurbo {
    pub mult: f32,
    pub extra_damage: f32,
    pub frames_left: f32,
}

/// GyrfalconPattern's Target.
#[derive(Clone, Copy, Default)]
pub enum GyrTarget {
    #[default]
    None,
    Bloon(u64, u64),
    Seg(u8, usize),
    Tower,
    Point([f32; 2]),
}

/// GyrfalconPattern's state.
#[derive(Clone, Default)]
pub struct GyrState {
    pub init: bool,
    pub hover: bool,
    pub target: GyrTarget,
    pub cd_start: i64,
    pub last_grapple: i64,
    pub next_idle: i64,
    pub idle: Option<[f32; 2]>,
    /// Each held bloon's distance (SetPath: the segment's centre less 2 per bloon before it).
    pub dists: Vec<(u64, f32, [f32; 2])>,
    /// Where the bird was at the end of last frame: where a bloon it lets go stays.
    pub last_pos: [f32; 2],
    /// When it was made: Simulation.Simulate runs the process list in creation order, so a bloon
    /// made since hasn't moved yet when it looks (recording beast_001: its second Black).
    pub stamp: u64,
}

pub struct AirUnit {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    /// Facing (radians); the Heli turns to its target while attacking, else to its heading.
    pub angle: f32,
    /// Position along an Ace's flight pattern (curve parameter, radians).
    pub phase: f32,
    /// Display height above the ground.
    pub height: f32,
    /// Beast Handler bird: the bloon it's diving at, the bloons it holds, frames until it may
    /// dive again, frames until the next grapple hit.
    pub dive: Option<u64>,
    pub grab: Vec<u64>,
    pub gy: GyrState,
    pub wait: f32,
    pub tick: f32,
    /// PathMovement: riding its pattern (else turning back onto it), bank and take-off pitch
    /// (degrees), take-off display scale, take-off progress (0..1) and seconds flown.
    pub on_path: bool,
    pub bank: f32,
    pub pitch: f32,
    pub scale: f32,
    pub tt: f32,
    pub age: f32,
}

/// A point on flight pattern `mode` round `c` at parameter `th`. STAND-IN P-ACEPATH: Circle and
/// Centered are ellipses with the pattern's radii; FigureEight (vertical) and Infinite
/// (horizontal, `rotated`) are lemniscates of Gerono with loops of the pattern's radius that cross
/// at the centre (the runway).
fn pattern_point(mode: TargetMode, rx: f32, ry: f32, c: [f32; 2], th: f32) -> [f32; 2] {
    let (s, s2, co) = (th.sin(), (2.0 * th).sin(), th.cos());
    match mode {
        TargetMode::FigureEight => [c[0] + rx * s2, c[1] + 2.0 * ry * s],
        TargetMode::Infinite => [c[0] + 2.0 * rx * s, c[1] + ry * s2],
        _ => [c[0] + rx * co, c[1] + ry * s],
    }
}

pub struct SubTower {
    pub spec: std::sync::Arc<crate::data::SubTowerSpec>,
    /// A Beast Handler pet: permanent, replaced when the handler upgrades, removed on sell.
    pub pet: bool,
    /// Id of the tower that made it (pop credit with CreditPopsToParentTowerModel).
    pub parent: u64,
    pub frames_left: f32,
    /// Frames since it was made.
    pub age: f32,
    /// TowerExpireModel.rounds: round ends left (0: no limit).
    pub rounds_left: u32,
    /// PathMovementFromScreenCenter.pathIndex: the circle point it is heading for (None until
    /// its first Process picks the closest).
    pub path_index: Option<usize>,
    /// Its height: PathMovementFromScreenCenter moves it in 3D toward z 250. HYPOTHESIS
    /// P-PHOENIXZ0: made at 250 (recording wizard_050: 0, 240, 249.9 or 260 are off at rt 31).
    pub z: f32,
    /// The round time it was made at (Tower +0x17c, TowerExpire.Process's start).
    pub born: i64,
}

#[derive(Clone)]
pub struct ActiveBrew {
    pub brew: Brew,
    pub shots_left: u32,
    /// Frames left (infinite without a time limit).
    pub frames_left: f32,
}

impl Tower {
    /// TheBlazingSun.OnBloonDestroy (VERIFIED-DISASM @0x180c96080): each bloon this tower destroys
    /// (removed for good: a degrade keeps the game's bloon object) takes a stack off, not below 0
    /// (recording desperado_500's Q lines: stacks rise to 11 on the MOAB and stay there once every
    /// throw kills). Only Blazing Sun weapons ever have stacks.
    pub(crate) fn sun_destroyed(&mut self, n: u64) {
        if n == 0 {
            return;
        }
        for st in &mut self.weapons {
            if st.sun_stacks > 0 {
                st.sun_stacks = (st.sun_stacks - n as i32).max(0);
            }
        }
    }

    /// Records attack `ai`'s aim at `target` and turns the body: to the middle of every
    /// attack's last target (`middle`, RotateToMiddleOfTargetsModel), else with this aim when
    /// the attack turns the body.
    fn set_attack_aim(&mut self, ai: usize, aim: f32, target: [f32; 2], pos: [f32; 2], middle: bool, turn_body: bool) {
        if self.aims.len() <= ai {
            self.aims.resize(ai + 1, (aim, None));
        }
        self.aims[ai] = (aim, Some(target));
        if middle {
            let ts: Vec<[f32; 2]> = self.aims.iter().filter_map(|a| a.1).collect();
            let m = [ts.iter().map(|t| t[0]).sum::<f32>() / ts.len() as f32, ts.iter().map(|t| t[1]).sum::<f32>() / ts.len() as f32];
            self.angle = (m[1] - pos[1]).atan2(m[0] - pos[0]);
        } else if turn_body {
            self.angle = aim;
        }
    }

    pub fn tier_key(&self) -> String {
        if self.paragon.is_some() {
            return "paragon".into();
        }
        format!("{}{}{}", self.tiers[0], self.tiers[1], self.tiers[2])
    }
}

/// Stack-only spelling of the existing model key, including multi-digit hero levels.
/// Called throughout the simulation; formatting a String here allocates on every lookup.
fn tier_model_key(tiers:[u8;3],buf:&mut [u8;9])->&str{
    let mut len=0;
    for n in tiers{
        if n>=100{buf[len]=b'0'+n/100;len+=1;}
        if n>=10{buf[len]=b'0'+(n/10)%10;len+=1;}
        buf[len]=b'0'+n%10;len+=1;
    }
    std::str::from_utf8(&buf[..len]).expect("decimal tier key")
}

/// Uniform grid over bloon positions: the projectile broad phase (the game has its own
/// CollisionGrid). Built when the projectile phase starts and kept in step with swap-removes and
/// spawns until it ends; bloon positions don't change inside that phase.
#[derive(Default)]
pub struct BloonGrid {
    origin: [f32; 2],
    w: usize,
    h: usize,
    cells: Vec<Vec<u32>>,
    /// Cell of each bloon and its slot in that cell, by storage index.
    of: Vec<u32>,
    slot: Vec<u32>,
}

const GRID_CELL: f32 = 24.0;

impl BloonGrid {
    fn xy(&self, p: [f32; 2]) -> (usize, usize) {
        let x = ((p[0] - self.origin[0]) / GRID_CELL).floor().clamp(0.0, (self.w - 1) as f32) as usize;
        let y = ((p[1] - self.origin[1]) / GRID_CELL).floor().clamp(0.0, (self.h - 1) as f32) as usize;
        (x, y)
    }

    /// Cells cover the map plus a wide margin; anything further out lands in the edge cells,
    /// which queries near the edge also include.
    fn build(&mut self, bloons: &[Bloon], lo: [f32; 2], hi: [f32; 2]) {
        let m = 300.0;
        self.origin = [lo[0] - m, lo[1] - m];
        self.w = (((hi[0] - lo[0] + 2.0 * m) / GRID_CELL).ceil() as usize).max(1);
        self.h = (((hi[1] - lo[1] + 2.0 * m) / GRID_CELL).ceil() as usize).max(1);
        self.cells.resize_with(self.w * self.h, Vec::new);
        for c in &mut self.cells {
            c.clear();
        }
        self.of.clear();
        self.slot.clear();
        for (i, b) in bloons.iter().enumerate() {
            self.add(i, b.pos);
        }
    }

    fn add(&mut self, i: usize, p: [f32; 2]) {
        debug_assert_eq!(self.of.len(), i);
        let (x, y) = self.xy(p);
        let c = y * self.w + x;
        self.slot.push(self.cells[c].len() as u32);
        self.cells[c].push(i as u32);
        self.of.push(c as u32);
    }

    /// Mirrors `bloons.swap(i, j)`.
    fn swap(&mut self, i: usize, j: usize) {
        let (ci, ki, cj, kj) = (self.of[i] as usize, self.slot[i] as usize, self.of[j] as usize, self.slot[j] as usize);
        self.cells[ci][ki] = j as u32;
        self.cells[cj][kj] = i as u32;
        self.of.swap(i, j);
        self.slot.swap(i, j);
    }

    /// Mirrors `bloons.swap_remove(i)` on storage of length `len`.
    fn swap_remove(&mut self, i: usize, len: usize) {
        // Out of bloon i's cell (the cell's last entry takes its slot)...
        let (c, k) = (self.of[i] as usize, self.slot[i] as usize);
        self.cells[c].swap_remove(k);
        if let Some(&moved) = self.cells[c].get(k) {
            self.slot[moved as usize] = k as u32;
        }
        // ...then the last bloon takes index i.
        let last = len - 1;
        if i != last {
            let (cl, kl) = (self.of[last] as usize, self.slot[last] as usize);
            self.cells[cl][kl] = i as u32;
        }
        self.of.swap_remove(i);
        self.slot.swap_remove(i);
    }

    /// Indices of bloons in cells within `r` of `c`, ascending.
    /// When that's most of the bloons anyway, it's simply every index (no sort).
    fn query(&self, c: [f32; 2], r: f32, out: &mut Vec<u32>) {
        out.clear();
        let (x0, y0) = self.xy([c[0] - r, c[1] - r]);
        let (x1, y1) = self.xy([c[0] + r, c[1] + r]);
        let n: usize = (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))).map(|(x, y)| self.cells[y * self.w + x].len()).sum();
        if n * 4 > self.of.len() {
            out.extend(0..self.of.len() as u32);
            return;
        }
        for y in y0..=y1 {
            for x in x0..=x1 {
                out.extend_from_slice(&self.cells[y * self.w + x]);
            }
        }
        out.sort_unstable();
    }
}

/// The game's own bloon CollisionGrid (40 x 33 cells of 10 from (-202, -168) in game space):
/// each cell lists its bloons in list order, by the cell of their `cell_pos`. Built when the
/// projectile phase starts and kept in step with swap-removes and new bloons; slots don't move
/// (a removed bloon leaves an empty slot, which a degraded bloon's first child takes over, as
/// the game's bloon stays the same object).
#[derive(Default)]
pub struct GameGrid {
    w: i32,
    h: i32,
    cells: Vec<Vec<u32>>,
    /// Cell and slot of each bloon, by storage index (u32::MAX: none).
    of: Vec<u32>,
    slot: Vec<u32>,
    /// Places of dead bloons whose objects the factory has pooled since the grid was filled
    /// (by object, with the cell position): a bloon made from one takes its place.
    pub ghosts: std::collections::HashMap<u32, ((u32, u32), [f32; 2])>,
    /// Bucket.largestCellSpan of each cell: (int)(largest radius / 10 + 0.5) of what it was
    /// filled with (Bucket.Add, VERIFIED-DISASM @0x25a6f20).
    span: Vec<i32>,
}

const EMPTY: u32 = u32::MAX;

impl GameGrid {
    fn cell(&self, p: [f32; 2]) -> usize {
        let (x, y) = grid_cell(p);
        (y.clamp(0, self.h - 1) * self.w + x.clamp(0, self.w - 1)) as usize
    }

    /// CollisionGrid.Fill: the bloons in the factory's list order (`key`), all but those made
    /// this frame (ids from `new_from`: the spawner and pops run after the grid is filled).
    fn build(&mut self, bloons: &[Bloon], key: impl Fn(&Bloon) -> u64, radius: impl Fn(&Bloon) -> f32, new_from: u64, ghosts: &[(u32, u64, [f32; 2], f32)]) {
        self.w = 40;
        self.h = 33;
        self.cells.resize_with((self.w * self.h) as usize, Vec::new);
        for c in &mut self.cells {
            c.clear();
        }
        self.of.clear();
        self.slot.clear();
        self.of.resize(bloons.len(), EMPTY);
        self.slot.resize(bloons.len(), EMPTY);
        self.ghosts.clear();
        self.span.clear();
        self.span.resize(self.cells.len(), 0);
        let mut largest = vec![0.0f32; self.cells.len()];
        // Ghosts are numbered past the bloons.
        let n = bloons.len() as u32;
        let mut idx: Vec<u32> = (0..n).filter(|&i| bloons[i as usize].id < new_from && !bloons[i as usize].unplaced).collect();
        idx.extend((0..ghosts.len() as u32).map(|g| n + g));
        let k = |i: u32| if i < n { key(&bloons[i as usize]) } else { ghosts[(i - n) as usize].1 };
        idx.sort_by_cached_key(|&i| k(i));
        for i in idx {
            let (c, r) = if i >= n {
                let (obj, _, at, r) = ghosts[(i - n) as usize];
                let c = self.cell(at);
                self.ghosts.insert(obj, ((c as u32, self.cells[c].len() as u32), at));
                self.cells[c].push(EMPTY);
                (c, r)
            } else {
                (self.cell(bloons[i as usize].cell_pos), radius(&bloons[i as usize]))
            };
            if r > largest[c] {
                largest[c] = r;
                self.span[c] = (r / GAME_GRID_CELL + 0.5) as i32;
            }
            if i >= n {
                continue;
            }
            self.of[i as usize] = c as u32;
            self.slot[i as usize] = self.cells[c].len() as u32;
            self.cells[c].push(i);
        }
    }

    /// CollisionGrid MoveNext (VERIFIED-DISASM @0x25db980) visits a bucket only if it isn't
    /// empty (largestCellSpan > 0) and max(dx, dy) of its cell offset from the query's cell,
    /// less the query's (int)(radius / 10 + 0.5), is at most its largestCellSpan. The max is
    /// signed: only buckets right of or below the query are cut (recording glue_205_close: a
    /// glue shot passes a Pink two cells below it, then hits it a cell away).
    ///
    /// The bucket is the one bloon `i` was filed in by the last Fill (where it was then), not
    /// where it is now (recording wizard_020: a Wall of Fire misses a Blue that moved into the
    /// next cell, then hits it once the next Fill files it there). A bloon in no bucket is
    /// taken where `at` is.
    fn visits(&self, q: [f32; 2], radius: f32, i: usize, at: [f32; 2]) -> bool {
        if self.span.is_empty() {
            return true;
        }
        let c = match self.place_of(i).0 {
            EMPTY => self.cell(at),
            c => c as usize,
        };
        let (qx, qy) = grid_cell(q);
        let (bx, by) = ((c as i32) % self.w, (c as i32) / self.w);
        let span = self.span[c];
        let over = (bx - qx).max(by - qy) - ((radius / GAME_GRID_CELL) as f64 + 0.5) as i32;
        span > 0 && (over <= 0 || over <= span)
    }

    /// Where bloon `i` sits.
    fn place_of(&self, i: usize) -> (u32, u32) {
        (self.of.get(i).copied().unwrap_or(EMPTY), self.slot.get(i).copied().unwrap_or(EMPTY))
    }

    fn vacate(&mut self, i: usize) {
        let (c, k) = (self.of[i], self.slot[i]);
        if c != EMPTY {
            self.cells[c as usize][k as usize] = EMPTY;
        }
    }

    /// A bloon appended at storage index `i`: not in any cell until the grid is filled again.
    fn push(&mut self, i: usize) {
        debug_assert_eq!(self.of.len(), i);
        self.of.push(EMPTY);
        self.slot.push(EMPTY);
    }

    /// Bloon `i` takes over the (emptied) place `at` of the bloon it degraded from.
    fn take_place(&mut self, i: usize, at: (u32, u32)) {
        if at.0 == EMPTY {
            return;
        }
        self.vacate(i);
        self.cells[at.0 as usize][at.1 as usize] = i as u32;
        self.of[i] = at.0;
        self.slot[i] = at.1;
    }

    /// Mirrors `bloons.swap(i, j)`.
    fn swap(&mut self, i: usize, j: usize) {
        let (ci, ki, cj, kj) = (self.of[i], self.slot[i], self.of[j], self.slot[j]);
        if ci != EMPTY {
            self.cells[ci as usize][ki as usize] = j as u32;
        }
        if cj != EMPTY {
            self.cells[cj as usize][kj as usize] = i as u32;
        }
        self.of.swap(i, j);
        self.slot.swap(i, j);
    }

    /// Mirrors `bloons.swap_remove(i)` on storage of length `len`.
    fn swap_remove(&mut self, i: usize, len: usize) {
        self.vacate(i);
        let last = len - 1;
        if i != last {
            let (cl, kl) = (self.of[last], self.slot[last]);
            if cl != EMPTY {
                self.cells[cl as usize][kl as usize] = i as u32;
            }
        }
        self.of.swap_remove(i);
        self.slot.swap_remove(i);
    }

    /// CollisionGrid.GetInRange's visiting order: the (2n+1)^2 cells around `q` with
    /// n = (int)((margin + 5 + radius) / 10), in orderedNeighborOffsets order; bloons per cell in
    /// list order.
    fn query(&self, q: [f32; 2], radius: f32, margin: f32, offsets: &[i32], out: &mut Vec<u32>) {
        out.clear();
        let (qx, qy) = grid_cell(q);
        let side = ((margin + 5.0 + radius) / GAME_GRID_CELL) as i32 * 2 + 1;
        let n = ((side * side) as usize).min(offsets.len() / 2);
        for w in offsets.chunks(2).take(n) {
            let (x, y) = (qx + w[0], qy + w[1]);
            if x < 0 || y < 0 || x >= self.w || y >= self.h {
                continue;
            }
            out.extend(self.cells[(y * self.w + x) as usize].iter().copied().filter(|&b| b != EMPTY));
        }
    }
}

/// Bloon ids a projectile has hit: a short list, switching to a hash set past 16 entries so
/// high-pierce blasts don't rescan a long list for every bloon they touch.
#[derive(Clone, Default)]
pub struct HitSet {
    small: Vec<u64>,
    big: FastSet<u64>,
}

impl HitSet {
    pub fn contains(&self, id: &u64) -> bool {
        if self.big.is_empty() { self.small.contains(id) } else { self.big.contains(id) }
    }
    pub fn push(&mut self, id: u64) {
        if self.big.is_empty() {
            self.small.push(id);
            if self.small.len() > 16 {
                self.big.extend(self.small.drain(..));
            }
        } else {
            self.big.insert(id);
        }
    }
    pub fn extend_from_slice(&mut self, ids: &[u64]) {
        for &id in ids {
            self.push(id);
        }
    }
    pub fn clear(&mut self) {
        self.small.clear();
        self.big.clear();
    }
    pub fn is_empty(&self) -> bool {
        self.small.is_empty() && self.big.is_empty()
    }
    pub fn retain(&mut self, mut keep: impl FnMut(&u64) -> bool) {
        self.small.retain(|x| keep(x));
        self.big.retain(|x| keep(x));
    }
}

pub struct Projectile {
    /// Unique per projectile (display: its particle effects follow it).
    pub uid: u64,
    /// timeSinceLastCollisionCheck: the frame it last checked for bloons (0 after a hit).
    pub check_t: u64,
    /// TravelStrait's direction (game frame, as emitted): what TrackTarget turns.
    pub dir: Option<[f32; 2]>,
    /// It has moved once (TrackTarget turns it from then on).
    pub seek_moved: bool,
    /// TrackTarget.lastTargetSearchElapsed.
    pub seek_t: u64,
    /// Its Position.z: a tower's shot at the weapon's ejectZ, one made on a bloon at that bloon's
    /// height, an exhaust's at its parent's (recording bomb_502: frags from a blast on a MOAB, 120
    /// up, fly over the 100-high trees).
    pub z: f32,
    pub pos: [f32; 2],
    /// Where it was when this frame began (collisions use it).
    pub prev: [f32; 2],
    /// Created this frame: it neither moves nor hits until the next.
    pub fresh: bool,
    pub vel: [f32; 2],
    pub frames_left: f32,
    pub pierce: f32,
    pub spec: Proj,
    pub crit: Option<f32>,
    /// CritRoll (Projectile.AddCritRoll): [chance, damageMultiplier, damageAdditive, rolled, crit].
    pub crit_roll: Option<[f32; 5]>,
    /// Its FollowPath is mirrored (thrown by the other hand: FlipFollowPath).
    pub mirror: bool,
    pub hit: HitSet,
    /// A bounce (clearCollidedWith) empties `hit` once this frame's collisions are done.
    pub clear_hits_next: bool,
    /// RefreshPierce came due this frame: applied after the collision pass.
    pub refresh_due: bool,
    /// A carried projectile's bloon (CarryProjectile), which it never hits; 0: none.
    pub carrier: u64,
    /// Tiers of the tower that fired it (display only).
    pub tiers: [u8; 3],
    /// Id of the tower that fired it (pop credit).
    pub owner: u64,
    /// Instant projectiles: the only bloon they can hit. Bouncing glaives: the bloon they home in on.
    pub target: Option<u64>,
    /// CollideOnlyWithTarget's Target for a travelling shot (SetTarget at emission and when its
    /// TrackTarget takes a new one), and its ignoreFirstCheck.
    pub cot: Option<u64>,
    pub ignore_first: bool,
    /// Where the shot came from (instant), or the start of its path (follow-path).
    origin: [f32; 2],
    /// Frames left before a retargeted shot hits; bounces so far; can bounce to camo bloons.
    wait: f32,
    bounces: u32,
    /// RetargetOnContact.startFrame while a delayed bounce is pending.
    retarget_at: Option<u64>,
    /// As Bloon.born.
    born: u64,
    camo: bool,
    /// Its tower sees camo by a support buff (which turns the FilterInvisibles off).
    buff_camo: bool,
    /// CreateTypedTower's towerModelToMake (index into create_tower), chosen when fired.
    typed: Option<usize>,
    /// Follow-path projectiles: throw direction (unit) and distance travelled along the path.
    /// Its rotation in the game's frame (degrees), when made the game's way (Vector2.Rotate):
    /// what its own emissions (exhaust) start from. None once something turns it.
    /// MapBorderRebound.lastPosition (none until its first Process).
    border_last: Option<[f32; 2]>,
    grot: Option<f32>,
    fwd: [f32; 2],
    path_d: f32,
    /// TravelAlongPathModel: the track it runs along and its distance on it.
    on_track: Option<(u8, f32)>,
    /// TravelAlongPath.offset (hasOffsetPosition): added to its track point each frame (the
    /// Necromancer's undead bloons, spread off the track line).
    track_off: [f32; 2],
    /// ClearHitBloons.lastClear: Simulation.time the hit list was last cleared (made at).
    clear_t: f32,
    /// Frames since fired (display spin).
    pub age: f32,
    /// Seeking projectiles: the bloon they steer towards.
    seek: Option<u64>,
    /// Its Target's collision-circle offset (a MOAB seen by Targetting.CanHitMoab), as it was
    /// when targeted: Target.Position is the bloon's plus it (recording sub_004: a dart steers
    /// at a MOAB's front circle as the MOAB was turned when the Sub took aim).
    seek_off: Option<[f32; 2]>,
    /// An instant shot's offset from its target bloon (the collision circle it was aimed at).
    inst_off: [f32; 2],
    /// Instant.positionSet.
    instant_set: bool,
    /// An instant shot's Target when it is a spot, not a bloon (Wall of Fire's track point).
    instant_pos: Option<[f32; 2]>,
    /// A plain Line emission's bloons (CollisionGrid.GetInLine, nearest the start first), collided
    /// in the instant pass the frame it is made.
    line_hits: Option<Vec<u64>>,
    /// LineProjectileEmission.Emit ends with canCollideWithBloons = false: no later collisions.
    line_spent: bool,
    /// Target's cached position: where its bloon was when the Target was made (Target.GetPosition
    /// gives it once the bloon is destroyed, the projectile's copy never being updated).
    target_cache: Option<[f32; 2]>,
    /// The factory object its Target's bloon lives in: pooled and reused, the Target follows
    /// the new bloon's position though no longer valid (recording sub_050: a blast on the Pink
    /// that took the object over).
    target_obj: Option<u32>,
    /// Its target had moved this frame when it was made (its Attack is later in the process
    /// list: recording beast_010, a pet's Splash where the Black is after its move).
    target_moved: bool,
    /// Where it goes back to after CreateEffectProjectileAfterTime moved it for a frame.
    effect_back: Option<[f32; 2]>,
    /// Where its last lightning jump ended (visual).
    bolt_from: Option<[f32; 2]>,
    /// Arrive-and-stay projectiles (caltrops): frames until they reach their spot and stop.
    moving: f32,
    /// ArriveAtTarget: (startFrame, origPos, targetPos).
    arrive_at: Option<(u64, [f32; 2], [f32; 2])>,
    /// ArriveAtTarget's timeToTake (seconds) for this shot.
    arrive_time: f32,
    /// Round ends it still survives (AgeModel.rounds).
    rounds_left: u32,
    /// Cash projectiles (bananas): what collecting it pays.
    pub value: f64,
    pub cash_base: f64,
    /// Frames since the pierce was last refreshed (RefreshPierceModel) / since the last
    /// interval sub-projectile (CreateProjectileOnIntervalModel).
    refresh_t: f32,
    interval_t: f32,
    /// CreateProjectileOnExhaustFraction.isDone, a bit per `spec.on_exhaust` entry.
    exhaust_done: u32,
    /// Projectile.Target's bloon (the game's): what the weapon aimed at, or a glaive's latest
    /// bounce; sub-projectiles made on exhaust take it (CollideOnlyWithTarget).
    aim: Option<u64>,
}

/// A side attack aimed only at a set point (Cleansing Foam): it has its own button, not a mode.
fn side_point(a: &crate::data::Attack) -> bool {
    a.targets.len() == 1 && a.targets[0] == "Point"
}

/// ProgressiveDifficultyManager.roundBasedHealthIncrease / roundBasedSpeedIncrease: from each
/// listed round, this much more per round until the next entry; and bonusSpeedIncreaseRounds:
/// a one-off amount once reached (bonusHealthIncreaseRounds is empty).
const HEALTH_RAMP: [(i32, f32); 9] = [(0, 0.0), (81, 0.02), (101, 0.05), (125, 0.15), (151, 0.35), (251, 1.0), (301, 1.5), (401, 2.5), (501, 5.0)];
const SPEED_RAMP: [(i32, f32); 2] = [(0, 0.0), (81, 0.02)];
const SPEED_BONUS: [(i32, f32); 4] = [(101, 0.18), (151, 0.4), (201, 0.5), (252, 0.48)];

/// GetHealthMultiplier / GetSpeedMultiplier (VERIFIED-DISASM @0xfc5540 / @0xfc58c0): 1 + the
/// ramp's rounds so far (each entry's count capped where the next begins) + bonuses reached,
/// the sum capped at 99999.
fn ramp(round: i32, table: &[(i32, f32)], bonus: &[(i32, f32)]) -> f32 {
    let mut sum = 0.0f32;
    for (i, &(from, inc)) in table.iter().enumerate() {
        if inc == 0.0 || round < from {
            continue;
        }
        let n = match table.get(i + 1) {
            Some(&(next, _)) => (next - from - 1).min(round - from) + 1,
            None => round - from + 1,
        };
        sum += n as f32 * inc;
    }
    sum += bonus.iter().filter(|b| round >= b.0).map(|b| b.1).sum::<f32>();
    sum.min(99999.0) + 1.0
}

fn progressive_health(round: i32) -> f32 {
    ramp(round, &HEALTH_RAMP, &[])
}

fn progressive_speed(round: i32) -> f32 {
    ramp(round, &SPEED_RAMP, &SPEED_BONUS)
}

/// Mathf.Round: to the nearest integer, halves to even.
fn round_half_even(x: f32) -> f32 {
    let r = x.round();
    if (x - x.trunc()).abs() == 0.5 && r % 2.0 != 0.0 { r - x.signum() } else { r }
}

/// FilterBloonIfDamageType.FilterBloon (VERIFIED-DISASM @0xc3d240): a bloon is filtered out
/// when it has one of `mask`'s properties that the projectile's damage is immune to (any of
/// them when it does no damage).
fn cant_hit(mask: u32, p: &Proj, props: u32) -> bool {
    mask != 0 && props & mask & if p.damage > 0.0 { p.immune } else { u32::MAX } != 0
}

/// FilterOveridingMutatedTarget.FilterBloon (VERIFIED-DISASM @0xc5eaf0): `f` is
/// (mutationOverideOrder, highestPriorityMutationId, defaultMutationId). A bloon is filtered out
/// when it carries a mutation ordered before the highest-priority one, or that one; with no
/// highest (or it equal to the default) when it carries the default; past the list, the default.
fn overriding_mutated(b: &Bloon, f: &(Vec<String>, String, String)) -> bool {
    let (order, highest, default) = f;
    if highest != default || !highest.is_empty() {
        for m in order {
            if m == highest {
                return b.is_mutated_by(highest);
            }
            if b.is_mutated_by(m) {
                return true;
            }
        }
    }
    b.is_mutated_by(default)
}

/// NinjaKiwi.Common.SeededRandom (VERIFIED-DISASM): Lehmer, seed = |seed x 16807 mod (2^31 - 1)|.
struct NkRandom(i64);

impl NkRandom {
    fn next(&mut self) -> i64 {
        self.0 = ((self.0 * 16807) % 2147483647).abs();
        self.0
    }

    /// NextFloat: the new seed (as an int) x 4.656613e-10.
    fn next_float(&mut self) -> f32 {
        self.next() as i32 as f32 * 4.656_613e-10
    }

    /// Range(min, max) for ints: min + seed mod (max - min); no draw when they're equal.
    fn range(&mut self, a: i32, b: i32) -> i32 {
        let (lo, hi) = (a.min(b), a.max(b));
        if lo == hi {
            return lo;
        }
        lo + (self.next() as i32) % (hi - lo)
    }
}

/// NinjaKiwi.Common.BufferedRandom (research/04-native-analysis.md): System.Random(seed) with
/// one sample read ahead; NextDouble is the old sample x (1 / int.MaxValue).
#[derive(Clone, Debug)]
struct BufferedRandom {
    rng: crate::callouts::DotNetRandom,
    next: i32,
}

impl BufferedRandom {
    fn new(seed: i32) -> Self {
        let mut rng = crate::callouts::DotNetRandom::new(seed);
        let next = rng.next();
        Self { rng, next }
    }

    fn next_double(&mut self) -> f64 {
        let v = self.next;
        self.next = self.rng.next();
        v as f64 * (1.0 / i32::MAX as f64)
    }
}

/// `a - b` wrapped to [-pi, pi].
fn angle_diff(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(std::f32::consts::TAU);
    if d > std::f32::consts::PI { d - std::f32::consts::TAU } else { d }
}

/// Point `d` along a polyline, and whether `d` is past its end.
/// PathImpl.DistanceToPoint (VERIFIED-DISASM @0xf877e0; .ctor @0xf88f00): the points carry their
/// running f32 sum of Vector3.Distance; at `d`, the segment with prev <= d < its sum gives
/// prev + (cur - prev) x ((d - prevSum) / |cur - prev|); past the end, the last point (recordings
/// boomerang_*: a float step otherwise).
fn along(pts: &[[f32; 2]], d: f32) -> ([f32; 2], bool) {
    let mut sum = 0.0f32;
    for w in pts.windows(2) {
        let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
        let mag = (dx * dx + dy * dy).sqrt();
        let next = sum + mag;
        if sum <= d && d < next {
            let t = (d - sum) / mag;
            return ([w[0][0] + dx * t, w[0][1] + dy * t], false);
        }
        sum = next;
    }
    (pts.last().copied().unwrap_or([0.0, 0.0]), true)
}

/// Curve.Evaluate (VERIFIED-DISASM @0xcd01f0): the first sample at 0 or below, the last at 1 or
/// above, else linear between samples floor and ceil of (count - 1) x p.
fn curve_eval(s: &[f32], p: f32) -> f32 {
    if p <= 0.0 {
        return s[0];
    }
    if p >= 1.0 {
        return s[s.len() - 1];
    }
    let x = (s.len() - 1) as f32 * p;
    let i = x as usize;
    let j = (x as f64).ceil() as usize;
    (s[j] - s[i]) * (x - i as f32) + s[i]
}

/// PathImpl.get_Length: the last point's running f32 sum of segment lengths.
fn path_len(pts: &[[f32; 2]]) -> f32 {
    pts.windows(2).fold(0.0f32, |sum, w| {
        let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
        sum + (dx * dx + dy * dy).sqrt()
    })
}

/// MOAB-class multiplier key of a bloon id (PushBackModel.multiplierMOAB/BFB/DDT/ZOMG).
fn moab_class(name: &str) -> Option<&'static str> {
    ["Moab", "Bfb", "Ddt", "Zomg", "Bad"]
        .into_iter()
        .zip(["MOAB", "BFB", "DDT", "ZOMG", "BAD"])
        .find(|(p, _)| name.starts_with(p))
        .map(|(_, k)| k)
}

/// Combined support buffs on one tower (see `data::Support`).
#[derive(Clone)]
pub struct Buff {
    /// Damage multiplier (Sub Commander), applied before `damage_add`.
    pub damage_mult: f32,
    /// Pierce added to every projectile, sub-projectiles too (SubCommanderSupport).
    pub pierce_all: f32,
    /// Projectile radius multiplier (Long Arm of Light).
    pub radius_mult: f32,
    pub range_mult: f32,
    pub range_add: f32,
    /// VigilanteMutator's loseLifeRangeBuff: added to the tower's range and to its attacks'
    /// ranges, except attacks with fixedRange (VERIFIED-DISASM @0x126e830, AttackModel+0x66).
    pub vig_range: f32,
    pub rate_mult: f32,
    pub pierce_add: f32,
    /// Shinobi Tactics: (multiplier, stacks); each stack adds (1 - multiplier) x the original
    /// pierce to every projectile.
    pub pierce_pct: Option<(f32, u32)>,
    pub speed_mult: f32,
    pub camo: bool,
    pub immune: Option<u32>,
    pub cash_mult: f32,
    /// MonkeyCityIncomeSupportModel: the tower's cash generation x this.
    pub income_mult: f32,
    /// AbilityCooldownScaleSupportModel: its ability cooldowns run this much faster.
    pub ability_cd_mult: f32,
    /// SupportRemoveFilterOutTagModel: its tag filters (FilterOutTag) are off.
    pub unfilter: bool,
    /// Brews: extra damage on every damaging projectile, and extra per-tag damage.
    pub damage_add: f32,
    pub tag_mods: Vec<TagMod>,
    /// Under MonkeyFanClub's Mutator (VERIFIED-DISASM @0xe61060): range at least its range
    /// (attacks without fixedRange too), every ArcEmission a single shot, projectile radius
    /// projectileRadius, weapons ejecting from its point, damage immunity (where there is
    /// some) its immuneBloonProperties when set; rate, pierce and damage go through the fields
    /// above.
    pub fan: Option<crate::data::FanClub>,
}

impl Default for Buff {
    fn default() -> Self {
        Self {
            damage_mult: 1.0,
            pierce_all: 0.0,
            radius_mult: 1.0,
            range_mult: 0.0,
            range_add: 0.0,
            vig_range: 0.0,
            rate_mult: 1.0,
            pierce_add: 0.0,
            pierce_pct: None,
            speed_mult: 0.0,
            camo: false,
            immune: None,
            cash_mult: 1.0,
            income_mult: 1.0,
            ability_cd_mult: 1.0,
            unfilter: false,
            damage_add: 0.0,
            tag_mods: Vec::new(),
            fan: None,
        }
    }
}

/// Grow's first Process (VERIFIED-DISASM @0xff1b70) on a model with overrideChildWithColor:
/// growToChildColor = it, growToParentColor = the model's id without Regrow / Camo / Fortified.
/// (A model without one leaves the colours as they were.)
fn grow_init(b: &mut Bloon, defs: &[crate::data::BloonDef], names: &[String]) {
    if let Some(c) = defs[b.kind].grow.as_ref().and_then(|g| g.child_color.clone()) {
        b.grow_colors = Some((c, names[b.kind].replace("Regrow", "").replace("Camo", "").replace("Fortified", "")));
    }
}

/// Whether bloon tags `tags` match a DamageModifierForTagModel's tag list.
impl Projectile {
    /// Vector3Boxed.get_Rotation of its direction (game degrees, as `grot`): the direction it
    /// steers by, else where it moves.
    fn dir_rot(&self) -> f32 {
        match self.dir {
            Some(d) => RAD2DEG * crt_atan2(d[0], d[1]),
            None => RAD2DEG * crt_atan2(self.vel[0], -self.vel[1]),
        }
    }
}

/// A due DamageOverTime tick: (bloon id, damage, immunities, owner, distribute, mutation id,
/// its lastTick before this tick).
type DotTick = (u64, f32, u32, u64, (bool, bool), String, f32);

/// DamageOverTime.DealDamage (VERIFIED-DISASM @0x1003cb0): damage plus each modifier's
/// additive for the bloon's tags, times every modifier's multiplier (recording glue_500: Bloon
/// Solver's 1 + 4 on a MOAB).
fn dot_damage(damage: f32, mods: &[TagMod], tags: &[String]) -> f32 {
    let add: f32 = mods.iter().filter(|m| tag_match(tags, m)).map(|m| m.add).sum();
    let mult: f32 = mods.iter().map(|m| m.mult).product();
    mult * (damage + add)
}

pub fn tag_match(tags: &[String], m: &TagMod) -> bool {
    let mut want = m.tag.split(',').map(str::trim).filter(|s| !s.is_empty());
    if m.all {
        want.all(|w| tags.iter().any(|t| t == w))
    } else {
        want.any(|w| tags.iter().any(|t| t == w))
    }
}

impl Buff {
    pub fn range(&self, r: f32) -> f32 {
        self.range_with(r, self.vig_range)
    }

    /// An attack's range: fixedRange attacks don't take the Vigilante buff (recording
    /// desperado_032 rt 1402: the 0-3-x Vagrant attack, fixed at 40, doesn't count a Black 40.48
    /// away during the buff; desperado_002 rt 533: the 0-0-2 one, 60, does count out to 76).
    pub fn attack_range(&self, a: &crate::data::Attack) -> f32 {
        self.range_with(a.range, if a.fixed_range { 0.0 } else { self.vig_range })
    }

    fn range_with(&self, r: f32, vig: f32) -> f32 {
        if r >= 1e5 {
            return r;
        }
        let r = r * (1.0 + self.range_mult) + self.range_add + vig;
        match &self.fan {
            Some(fc) => r.max(fc.range),
            None => r,
        }
    }

    /// The projectile as fired under these buffs (None if unchanged). Damage-type support reaches
    /// sub-projectiles too (explosions); pierce and speed only the shot itself.
    fn projectile(&self, p: &Proj) -> Option<Proj> {
        if self.pierce_add == 0.0 && self.pierce_all == 0.0 && self.pierce_pct.is_none() && self.speed_mult == 0.0 && self.immune.is_none() && self.damage_add == 0.0 && self.damage_mult == 1.0 && self.radius_mult == 1.0 && self.tag_mods.is_empty() && self.fan.is_none() {
            return None;
        }
        let mut q = p.clone();
        // SupportShinobiTactics.MutatorTower.Mutate (VERIFIED-DISASM @0xd92090): per stack,
        // pierce = (1 - multiplier) x the original's pierce + pierce, on every projectile
        // (recording ninja_040: 2 -> 2.16).
        if let Some((m, n)) = self.pierce_pct {
            fn pct(p: &mut Proj, m: f32, n: u32) {
                let orig = p.shinobi_base.unwrap_or(p.pierce);
                for _ in 0..n {
                    p.pierce = (1.0 - m) * orig + p.pierce;
                }
                for c in &mut p.on_contact {
                    pct(&mut c.projectile, m, n);
                }
                for e in &mut p.on_exhaust {
                    pct(&mut e.projectile, m, n);
                }
            }
            pct(&mut q, m, n);
        }
        q.pierce += self.pierce_add;
        if self.pierce_all != 0.0 {
            fn all(p: &mut Proj, add: f32) {
                p.pierce += add;
                for c in &mut p.on_contact {
                    all(&mut c.projectile, add);
                }
                for e in &mut p.on_exhaust {
                    all(&mut e.projectile, add);
                }
            }
            all(&mut q, self.pierce_all);
        }
        q.radius *= self.radius_mult;
        if let Some(fc) = &self.fan {
            q.radius = fc.projectile_radius;
            fn fan(p: &mut Proj, im: u32) {
                if im != 0 && p.immune != 0 {
                    p.immune = im;
                }
                for c in &mut p.on_contact {
                    fan(&mut c.projectile, im);
                    if c.emission.kind == "Arc" {
                        c.emission.count = 1;
                    }
                }
                for e in &mut p.on_exhaust {
                    fan(&mut e.projectile, im);
                    if e.emission.kind == "Arc" {
                        e.emission.count = 1;
                    }
                }
            }
            fan(&mut q, fc.immune);
        }
        q.speed *= 1.0 + self.speed_mult;
        // HYPOTHESIS P-BREW: brew damage reaches every damaging (sub-)projectile the tower fires.
        if self.damage_add != 0.0 || self.damage_mult != 1.0 || !self.tag_mods.is_empty() {
            // SubCommanderSupport.MutatorTower.Mutate (VERIFIED-DISASM @0x180d910d0): a
            // DamageModel not named FirstStrike gets damage x damageScale + damageIncrease, and
            // every DamageModifierForTag / ForBloonType's additive x damageScale (recording sub_005:
            // a dart on a MOAB (2 + 2) x 2).
            fn boost(p: &mut Proj, mult: f32, add: f32, mods: &[TagMod]) {
                if p.damage > 0.0 {
                    p.damage = if p.first_strike_dm { p.damage + add } else { p.damage * mult + add };
                    if mult != 1.0 {
                        for m in &mut p.tag_mods {
                            m.add *= mult;
                        }
                        for m in &mut p.type_mods {
                            m.add *= mult;
                        }
                    }
                    p.tag_mods.extend(mods.iter().cloned());
                }
                for c in &mut p.on_contact {
                    boost(&mut c.projectile, mult, add, mods);
                }
                for e in &mut p.on_exhaust {
                    boost(&mut e.projectile, mult, add, mods);
                }
            }
            boost(&mut q, self.damage_mult, self.damage_add, &self.tag_mods);
        }
        if let Some(im) = self.immune {
            fn strip(p: &mut Proj, im: u32) {
                p.immune &= im;
                for c in &mut p.on_contact {
                    strip(&mut c.projectile, im);
                }
                for e in &mut p.on_exhaust {
                    strip(&mut e.projectile, im);
                }
            }
            strip(&mut q, im);
        }
        Some(q)
    }
}

/// What a tower or upgrade of base cost `cost` costs with the difficulty's cost multiplier and
/// discount fraction `d` (research: GlobalCostModModel.Mutate adds (mult - 1) x cost to the
/// model's cost; TowerManager.GetTowerCost rounds (1 - discounts) x cost with the helper at
/// 0x18054EEC0: x / 5 rounded half to even, times 5, truncated; all in f32).
pub fn game_cost(cost: f64, mult: f32, d: f64) -> f64 {
    let c = cost as f32;
    let scaled = c + (mult - 1.0) * c;
    let q = (1.0 - d as f32) * scaled / 5.0;
    let (t, fr) = ((q as f64).trunc(), (q as f64).fract());
    let r = if fr == 0.5 || fr == -0.5 {
        (t + if (t as i64) & 1 == 1 { fr.signum() } else { 0.0 }) as f32
    } else if q >= 0.0 {
        (q + 0.5).floor()
    } else {
        (q - 0.5).ceil()
    };
    (5.0 * r) as i32 as f64
}

/// CashModel payout for a uniform roll `r`: min..max, times (1 + bonusMultiplier).
fn cash_value(c: &crate::data::Cash, r: f32) -> f64 {
    ((c.min + (c.max - c.min) * r) * (1.0 + c.bonus)) as f64
}

/// A stationary area hit to draw: bomb explosion, a Ring of Fire when `fire`, an Ice Monkey
/// freeze when `ice`.
#[derive(Clone, Copy)]
pub struct Burst {
    pub pos: [f32; 2],
    pub radius: f32,
    pub fire: bool,
    pub ice: bool,
}

#[path = "map_logic.rs"]
pub mod map_logic;

/// Blockers of `areas` by grid cell, as Game::blocker_grid.
fn blocker_grid(areas: &[Area]) -> Vec<(i32, Vec<[f32; 4]>)> {
    let mut g: Vec<(i32, Vec<[f32; 4]>)> = vec![(0, Vec::new()); 40 * 33];
    for a in areas.iter().filter(|a| !a.is_disabled) {
        for c in &a.blockers {
            let (x, y) = grid_cell([c[0], c[1]]);
            let cell = &mut g[(y.clamp(0, 32) * 40 + x.clamp(0, 39)) as usize];
            cell.1.push([c[0], -c[1], c[2], a.height]);
            // Bucket.Add: largestCellSpan = (int)(largestRadius / 10 + 0.5) (VERIFIED by dump).
            let largest = cell.1.iter().map(|b| b[2]).fold(0.0f32, f32::max);
            cell.0 = cell_span(largest);
        }
    }
    g
}

/// Map.InsertArea: before the first area not above it.
fn area_insert_index(areas: &[Area], height: f32) -> usize {
    areas.iter().position(|a| a.height <= height).unwrap_or(areas.len())
}

pub struct Game {
    pub mod_runtime:crate::mod_runtime::Runtime,
    pub defs: Vec<BloonDef>,
    /// Each bloon's speed and max health before ProgressiveDifficultyManager's late-round
    /// scaling (bloonModelsOriginal).
    base_defs: Vec<(f32, i32)>,
    /// Bloon ids, indexed like `defs`.
    names: Vec<String>,
    by_name: HashMap<String, usize>,
    /// Each bloon type's children as type indices (BloonModel.children resolved once).
    child_kinds: Vec<Vec<usize>>,
    /// GrowModel: (bloon kind it grows into, frames between growths) per bloon kind.
    grow_kinds: Vec<Option<(usize, u32)>>,
    /// Pops a bloon of each type gives when destroyed down to nothing (itself and every layer
    /// inside it).
    pops_in: Vec<u64>,
    /// Largest bloon radius (broad-phase margin).
    max_bloon_radius: f32,
    /// Health removed from bloons so far (towers are credited with the change around a hit).
    damage_tally: u64,
    /// Bloons removed for good (popped with no children) so far, and by the last `damage` call:
    /// TheBlazingSun.OnBloonDestroy counts these.
    destroyed_tally: u64,
    last_destroyed: u64,
    /// Rank of each cell offset (dx + 64, dy + 64 in a 129 x 129 table) in the collision
    /// grid's query order; u16::MAX where unknown.
    grid_rank: Vec<u16>,
    has_grid_rank: bool,
    /// CollisionGrid.orderedNeighborOffsets, and the game's bloon grid built from it.
    grid_offsets: Vec<i32>,
    ggrid: GameGrid,
    /// The furthest any bloon moves in a frame (the broad phase looks at frame-start positions).
    max_bloon_step: f32,
    /// Projectile broad phase, live (`grid_on`) only during the projectile phase.
    grid: BloonGrid,
    grid_on: bool,
    grid_cand: Vec<u32>,
    /// The firing tower's rotation (the game's degrees) for the shots `emit` is making, when the
    /// game's own direction math applies.
    emit_rot: Option<f32>,
    /// The emitting projectile's direction as a rotation (EmissionRotationOffProjectileDirection).
    emit_dir_rot: Option<f32>,
    /// Height for the projectiles `emit` makes (Projectile.z).
    emit_z: f32,
    /// The Target an emission is given, when it is a bloon (see `emit_height`).
    emit_target: Option<u64>,
    /// In InstantDamageEmission.Process's collision pass (after the towers act).
    in_instant: bool,
    /// Path.bloonsList per path: bloon ids and whether they're still alive, in the game's order
    /// (appended when made; popped ones stay until the flush every 60 frames), and where each
    /// id sits.
    path_lists: Vec<Vec<(u64, bool)>>,
    path_rank: FastMap<u64, (u8, u32)>,
    /// Each path list's length when this frame's tower phase began: where the Spawner's bloons
    /// go in it (the Spawner comes before the towers and bloons in the process list).
    plist_mark: Vec<usize>,
    /// The bloon factory (Factory<Bloon>): `fact` lists every object made (creation order; a
    /// dead one until the next flush), `fact_at` an object's place in it, `pool` the flushed
    /// objects a new bloon takes, last flushed first.
    fact: Vec<(u32, bool)>,
    fact_at: Vec<u32>,
    pool: Vec<u32>,
    fact_mark: usize,
    /// Set by a pop for its first child: the parent's object, carried on.
    spawn_parent_obj: Option<u32>,
    /// Bloons the collision grid was filled with that died later that frame (object, id, cell
    /// position, order), and the first id made after the fill.
    died_after_fill: Vec<(u32, u64, [f32; 2], u64, f32)>,
    ghosts_src: Vec<(u32, u64, [f32; 2], u64, f32)>,
    fill_first_id: Option<u64>,
    /// The cell position of the grid place a new bloon took over from its object's dead bloon.
    ghost_cell: Option<[f32; 2]>,
    /// On a flush frame, the list places from before it: the collision grid is filled in that
    /// order (recording dart_052: a Blue ahead of a Yellow the flush moved in front of it).
    grid_rank_pre: Option<FastMap<u64, u32>>,
    /// The projectile factory's list: every projectile made (by uid), the dead ones until the
    /// next flush. `projectiles` keeps the live ones in this order.
    proj_list: Vec<u64>,
    /// A pop in progress: how deep its layers go, whether a child popped too, and the first id
    /// it made (the game works out a hit's whole result first, so in-between layers never exist).
    pop_depth: u32,
    pop_nested: bool,
    pop_first_id: u64,
    /// next_id when this step began: bloons from here on were made this frame.
    step_first_id: u64,
    /// Damage being dealt from inside a bloon's Process after its Move (DamageOverTime ticks).
    pop_after_move: bool,
    /// The projectile loop's position (those up to it have moved this frame); usize::MAX
    /// outside it.
    proj_cursor: usize,
    /// The damage being dealt: (distributeToChildren, overrideDistributeBlocker).
    distribute: (bool, bool),
    /// The step's phase (process-list order of things made in it): 0 collisions, 1 towers,
    /// 2 movement and later (the Spawner's emissions count as 0: it runs before the towers).
    phase: u8,
    /// Inside `step` (else between frames, where input such as an ability activation lands).
    in_step: bool,
    /// instant_pass's outputs (children, sub-towers, contact spawns, exhaust spawns) for the step.
    instant_out: (Vec<u64>, Vec<(std::sync::Arc<crate::data::SubTowerSpec>, [f32; 2], u64, Option<f32>, bool)>, Vec<([f32; 2], f32, crate::data::Contact, [u8; 3], u64, Option<u64>, f32)>, Vec<([f32; 2], f32, crate::data::Exhaust, [u8; 3], u64, Option<u64>, (Option<f32>, f32, f32))>),
    /// Simulation.time.elapsed: frames simulated (the factory flush runs when it is a multiple
    /// of 60).
    pub sim_elapsed: u64,
    rbe: Vec<f32>,
    /// GameData.bloonMutatorData (BadImmunity.FilterMutation's lists).
    mutator_data: crate::data::BloonMutatorData,
    /// BloonModel.layerCountOffset per bloon kind: layers credited past the first when it pops.
    layer_offset: Vec<u64>,
    /// DistributeCashModel.additionalCash per bloon kind (cash past the $1 a layer).
    extra_cash: Vec<f64>,
    /// Cash a bloon of each kind pays when destroyed whole (every layer's $1 + additionalCash).
    cash_in: Vec<f64>,
    /// The bloon models as they were before ChangeCeramicsAndChildren, while it is applied
    /// (bloonModelsOriginal): children, leak damage, max health and tags per kind.
    pre_super: Option<Vec<(Vec<String>, f32, i32, Vec<String>)>>,
    pub tower_defs: Vec<(String, TowerDef)>,
    pub tower_tiers: Vec<Option<TowerTiers>>,
    pub shop: Vec<crate::data::ShopItem>,
    /// Every hero (the shop holds the chosen one).
    pub heroes: Vec<crate::data::ShopItem>,
    /// The first track (single-track helpers and tests).
    pub track: Track,
    /// Every active track. Round bloons spawn on tracks[emissionIndex % count]
    /// (Splitter.SpawnRout); children stay on their parent's. HYPOTHESIS P-MULTIPATH: every
    /// active exported path is a spawn path (splitter routing isn't modelled).
    pub tracks: Vec<Track>,
    /// Per track: the tracks its exit splitter routes on to, that splitter's index (into
    /// `routs`), and whether round bloons spawn on it.
    track_exit: Vec<(Vec<usize>, usize)>,
    /// Per track: Path.spawnDist and leakDist (None with an entry / exit splitter).
    track_ends: Vec<(Option<f32>, Option<f32>)>,
    /// Per track: each PathSegment's point (the path point at its centre).
    track_segs: Vec<Vec<[f32; 2]>>,
    /// Per track, per segment: PathSegment.canTargetSegment (GatherPathInfoData, VERIFIED-DISASM
    /// @0xf8b930: not disablePathTargeting at the segment's centre). Without the imported flags
    /// (other maps) the old stand-in: the segment's point is on the map.
    track_seg_ok: Vec<Vec<bool>>,
    /// Per track: Path.pointInfos rotations (every CacheDistance), when imported.
    track_rot: Vec<Vec<f32>>,
    /// Per track: the tracks its entry splitter routes back to, and that splitter's index.
    track_entry: Vec<(Vec<usize>, usize)>,
    spawn_tracks: Vec<usize>,
    /// The map's spawner when it isn't a plain splitter (map_routes.rs).
    spawn_route: Option<crate::map_routes::SpawnRoute>,
    /// Removeables and map events (map_logic.rs).
    pub logic: map_logic::LogicState,
    /// Splitter.Rout counters, one per exit splitter.
    routs: Vec<usize>,
    /// `map.areas` in the game's order: the base land area (GatherMapBaseArea runs before
    /// GatherMapAreasAndBlockers), then the scene's areas.
    areas: Vec<Area>,
    /// BlockerModels: (x, z, radius, isDisabled).
    blockers: Vec<([f32; 3], bool)>,
    /// CollisionChecker.MapBlockerGrid: per 10-unit cell (40 x 33, like the bloon grid) its
    /// Bucket.largestCellSpan and the MapBlockers centred in it (game x, game y, radius, height).
    blocker_grid: Vec<(i32, Vec<[f32; 4]>)>,
    pub bounds: ([f32; 2], [f32; 2]),
    /// The bounds come from the map's terrain mesh (else they're the game's world bounds, which
    /// say where things may go, not where the scene ends).
    pub has_terrain: bool,
    /// ImmunityModel (Legend of the Night): frames left with leaks costing no lives.
    pub leak_immune: f32,
    rounds: Vec<Vec<crate::data::Group>>,
    /// GameModel.freeplayGroups (rounds past the round set) and FreeplayRoundManager's seed.
    freeplay_groups: Vec<crate::data::FreeplayGroup>,
    pub freeplay_seed: i32,
    /// Geraldo's shop items and their stock (filled when Geraldo is first placed).
    geraldo_items: Vec<crate::data::GeraldoItem>,
    /// GameData.power_projectiles (powers.rs).
    power_projectiles: HashMap<String, Proj>,
    /// Power id -> the tower def it places (powers.rs), for the ones the clone simulates.
    power_tower_defs: HashMap<String, usize>,
    /// Geraldo's shop stock per player (Simulation.GetGeraldoShopInventory(inputId)); one in single player.
    geraldo_stock: Vec<Vec<geraldo::ItemStock>>,
    /// Apopalypse (GameModel.isApopalypse): every round comes from the freeplay groups
    /// (UI.ApplyExtraNotInSimModelSettings installs FreeplayRoundManager from round 1) and the
    /// next one is sent as soon as the last has emitted everything.
    pub apopalypse: bool,
    round_scale: f32,
    pub bloons: Vec<Bloon>,
    pub towers: Vec<Tower>,
    pub projectiles: Vec<Projectile>,
    /// Round emissions not yet sent: (due frame, bloon type, emission index in the round).
    pending: Vec<(u32, usize, u32)>,
    pub round: usize,
    pub round_active: bool,
    round_frame: u32,
    /// Sandbox spawns waiting: (step they come out on, bloon type), by `steps`.
    sandbox_queue: Vec<(u64, usize)>,
    /// Spawner.bloonsOutsideOfTrack: MOAB-class bloons the round sent that aren't on the map yet
    /// (Tower.OnBloonEnterTrack when they are).
    outside_track: Vec<u64>,
    /// Simulation steps since the game began.
    steps: u64,
    /// Simulation.roundTime: frames that ran while a round was on or bloons were out (normal
    /// weapons' clock; Simulation.time is `steps`).
    round_clock: i64,
    /// Last projectile uid given out.
    next_uid: u64,
    /// BloonManager.GetTotalRbeOfActiveBloons, refreshed each step (Avatar of Wrath).
    pub active_rbe: f32,
    pub cash: f64,
    /// IMF Loan still owed; `earn` sends `debt_rate` of income to it.
    pub debt: f64,
    /// MonkeyBoost mutators from Powers (powers.rs): tower id, round_clock at, durationFrames,
    /// rateScale.
    pub power_boosts: Vec<(u64, i64, i64, f32)>,
    /// Thrives running (powers.rs): rounds left, cashScale.
    pub thrives: Vec<(i32, f32)>,
    /// The Spawner's emissions due this frame (bloon kind, path), made after the movement.
    frame_spawns: Vec<(usize, u8)>,
    /// Dart Time running (powers.rs): its endFrame (round_clock), bloonSpeed, towerAttackSpeed.
    pub dart_time: Option<(i64, f32, f32)>,
    debt_rate: f64,
    pub lives: f64,
    /// The mode's starting lives (BonusLivesPerRoundModel's cap is relative to it).
    start_lives: f64,
    /// How far along its track the furthest bloon is, 0..1 (LeakDangerAttackSpeedModel).
    danger: f32,
    /// Lives were lost this frame (AbilityReduceCooldownOnLivesLostModel).
    leaked: bool,
    pub popped: u64,
    /// Positions of bloons popped since last drained (display only).
    pub pops: Vec<[f32; 2]>,
    /// Explosions and fire rings since last drained (display only).
    pub bursts: Vec<Burst>,
    /// Instant hits since last drained, from muzzle or previous bloon to the bloon hit (display only).
    pub tracers: Vec<([f32; 2], [f32; 2])>,
    /// Line shots with a LineEffectModel this frame (visual): from, to, the beam.
    pub beams: Vec<([f32; 2], [f32; 2], crate::data::LineFx)>,
    /// Lightning jumps this frame (visual): from, to, the bolts, the shot's uid.
    pub bolts: Vec<([f32; 2], [f32; 2], crate::data::Bolts, u64)>,
    /// Cash collected since last drained: where and how much (display only).
    pub cash_pops: Vec<([f32; 2], f64)>,
    /// ShowTextOnHit texts this frame (the renderer draws "Crit!" there).
    pub hit_texts: Vec<[f32; 2]>,
    /// Upgrades bought this frame, for the upgrade effect: (tower position, effect prefab).
    pub upgrade_fx: Vec<([f32; 2], &'static str)>,
    /// Paragon purchases for the renderer's sacrifice effects (display only).
    pub sacrifices: Vec<Sacrifice>,
    /// Towers placed this frame: (position, placement effect prefab, scale).
    pub place_fx: Vec<([f32; 2], String, f32)>,
    /// The next `Effect::seq`.
    pub effect_seq: u64,
    /// The round Monkey City's free Dart Monkey was taken.
    free_tower_round: Option<usize>,
    /// MOAB classes a hero has already warned about this round
    /// (CreateSoundOnBloonEnterTrack.ResetPlayedBooleans on round start).
    voice_warned: Vec<String>,
    /// EmitOnPopModel shots from bloons that popped: (where, projectile, emission, owner).
    /// (where, projectile, emission, owner, the bloon's rotation in degrees, its layer, its
    /// height).
    pop_emits: Vec<([f32; 2], Box<Proj>, Box<Emission>, u64, f32, u64, f32)>,
    pub paragon_degree: Option<crate::data::ParagonDegree>,
    /// Difficulty: cost multiplier, sell share, round bonus (base, per round), locked towers,
    /// selling disabled (CHIMPS), and its name.
    pub cost_mult: f32,
    pub sell_mult: f64,
    pub round_bonus: Option<[f64; 2]>,
    pub locked: Vec<String>,
    pub no_sell: bool,
    pub health_mods: Vec<crate::data::HealthMod>,
    /// ModifyAllCashModModel multiplier on every income.
    pub cash_mult: f64,
    /// Sandbox mode (the sandbox panel's tools are available).
    pub sandbox: bool,
    /// Simulation.sandBoxResetHealthTimer: frames until sandbox health refills (-1 off).
    /// OnSandboxHealthChanged starts it at 300 when health reaches 0.
    pub sandbox_health_timer: i32,
    /// InstantDamageEmission projectiles made this frame: they hit their target after the towers act.
    instant_hits: Vec<u64>,
    /// Simulation.lastSetHealth: what sandbox health refills to (Set Health sets it).
    pub last_set_health: f64,
    /// Health at the start of this step, to notice it changing (Simulation's health events).
    lives_at_step: f64,
    pub mode_name: String,
    /// The mode's difficulty (Easy / Medium / Hard): ReadonlyInGameData.selectedDifficulty.
    pub difficulty: String,
    /// What the end screens' player stats show (AnalyticsGameSessionInfo).
    pub stats: SessionStats,
    /// BloonCauseOfDefeat.bloonCoDDataList: recent leaks (bloon id, its total leak damage, the
    /// frame), at most 30, in time order.
    pub leaks: Vec<(String, f32, u64)>,
    /// The round index the difficulty starts at (StartingRoundModModel - 1).
    pub start_round: usize,
    /// Frames of Syphon Funding left (P-SYPHON).
    pub syphon: f32,
    /// Seconds spent per step phase (effects + movement, towers, projectiles), for profiling.
    pub prof: [f64; 3],
    pub prof_max: [f64; 3],
    #[cfg(test)]
    perf_detail: [f64; 3],
    #[cfg(test)]
    reference_targeting: bool,
    /// The tower phase's target search index (`TargetIndex`); None when bloons moved since it was made.
    target_index: Option<TargetIndex>,
    /// Support buffs on each tower, refreshed every step.
    pub buffs: Vec<Buff>,
    /// Sounds since last drained (display only).
    pub sfx: Vec<Sfx>,
    next_id: u64,
    /// Bloon.layerId counter.
    next_layer: u64,
    pub game_over: bool,
    /// The mode's last round was beaten (the Victory screen); `freeplay` then continues.
    pub won: bool,
    pub freeplay: bool,
    /// The mode's last round (EndRoundModel); the round set goes on to 140 for freeplay.
    pub end_round: usize,
    /// The map's difficulty (`GameData::map_difficulty`); None earns no XP.
    pub map_difficulty: Option<usize>,
    /// Bridge.Xp: what this game has earned since the last drain (InGame.RoundEnd ->
    /// Btd6Player.DrainXP: `account` to the player's rank XP, `towers` to each tower's Monkey XP
    /// by base id, in the order first earned).
    pub xp_account: f32,
    pub xp_towers: Vec<(String, f32)>,
    /// STAND-IN random source (xorshift), not the game's seeded RNG: Distraction's blow-back
    /// chance and distance, shuriken spread.
    rng: u64,
    /// Picks sound variants, kept apart from `rng` so sounds don't change the simulation.
    sfx_rng: u64,
    /// Simulation.syncedRandom (Simulation.Initialise: BufferedRandom(GameModel.randomSeed),
    /// the seed being `freeplay_seed`), made at its first draw. Only the draws that are
    /// transcribed use it.
    synced: Option<BufferedRandom>,
    /// The cursor on the map, for TargetPointerModel attacks. Kept when the cursor leaves the
    /// map (DartlingMaintainLastPosModel).
    pub pointer: Option<[f32; 2]>,
    pub(crate) coop_pointers: HashMap<u64, Option<[f32; 2]>>,
    pub(crate) coop_wallets: Option<rrbtd6_coop::economy::Wallets>,
    /// Co-op: the local player, for what the HUD shows (their Geraldo's shop); never simulation.
    pub(crate) coop_view: Option<u8>,
    /// Co-op: the division's placement areas (None: Free For All).
    pub(crate) coop_areas: Option<coop_areas::CoopAreas>,
    /// Co-op: who bought each Geraldo item on a tower ((tower id, item) -> player), for its level.
    pub(crate) item_buyers: std::collections::BTreeMap<(u64, usize), u8>,
    /// Towers removed during `step` (expired sub-towers), in removal order, so the caller can fix
    /// indices it holds: (index, where the tower now at that index came from; usize::MAX when
    /// the later ones shifted down instead).
    pub removed: Vec<(usize, usize)>,
    /// Typed sub-towers made so far (they take turns).
    typed_made: usize,
}

/// A clip to play. `group`: the PopEffectModel sound group (id, limit, frames, frames at fast forward).
/// GameData.bloonImmuneSounds.immuneSounds (Assets/Data/GameData.asset), in the order the
/// damage path prefers them: Diamond (64), Frozen (16), Lead (1), Purple (8). Its
/// towerSetImmuneSounds (Primary HitMetal, Military HitSteel, Magic HitPurple, Support HitBrick)
/// are for event bloons immune to a tower set, which we don't model yet.
const IMMUNE_SOUNDS: [(u32, &str, [&str; 4]); 4] = [
    (64, "DiamondImmune", ["HitGolden01", "HitGolden02", "HitGolden03", "HitGolden04"]),
    (16, "FrozenImmune", ["HitIce01", "HitIce02", "HitIce03", "HitIce04"]),
    (1, "LeadImmune", ["HitMetal01", "HitMetal02", "HitMetal03", "HitMetal04"]),
    (8, "PurpleImmune", ["HitPurple01", "HitPurple02", "HitPurple03", "HitPurple04"]),
];

/// Hero voice lines' sound group: one at a time, at least VOICE_GAP frames apart
/// (PlayHeroUpgradeSound passes 20). Leak lines: their own group, 2400 frames apart
/// (PlayLeakSound).
const VOICE: &str = "HeroVoice";
const VOICE_GAP: u32 = 20;

pub struct Sfx {
    pub name: String,
    pub group: Option<(String, u32, u32, u32)>,
}

// The collision pass (CollisionChecker.Process), as a macro so InstantDamageEmission and plain Line
// emissions can run it for their own projectiles as each tower's attacks act.
macro_rules! collide_pass {
    ($only:expr, $s:ident, $p:ident, $kids:ident, $exhausted:ident, $subs:ident, $contacts:ident, $cand:ident, $sim_now:ident) => {
    while $p < $s.projectiles.len() {
        $s.proj_cursor = $p;
        // (The instant pass: only this frame's InstantDamageEmission projectiles, made after the
    // towers acted, hit now; nothing else is processed again.)
    if let Some(only) = $only {
        if !only.contains(&$s.projectiles[$p].uid) {
            $p += 1;
            continue;
        }
    }
        // Simulation.Simulate checks collisions before the process list moves anything
        // (VERIFIED by recording: a dart is created, waits a frame, then hits where it and
        // the bloon were when the frame began).
        if $s.projectiles[$p].fresh && $only.is_none() {
            $s.projectiles[$p].fresh = false;
            $p += 1;
            continue;
        }
        $s.projectiles[$p].prev = $s.projectiles[$p].pos;
        if std::mem::take(&mut $s.projectiles[$p].clear_hits_next) {
            $s.projectiles[$p].hit.clear();
        }
        // TrackTarget.GetNewTarget, when it will look below (from where it is now).
        {
            let pr = &mut $s.projectiles[$p];
            if let Some(path) = &pr.spec.path {
                // HYPOTHESIS P-PATH: the path is rotated so its +y points along the throw
                // (at the target), unscaled, starting at the eject point; mirrored when the
                // tower throws with its other hand (FlipFollowPath).
                pr.path_d += path.speed / FPS;
                // FollowPath.ProcessFrame (VERIFIED-DISASM @0xcdc450): with an easePath the
                // distance runs along it, and the point is the path's at path length x (ease
                // point's x / FindPathMax: the largest x, from 0); either way it ends once
                // the walked path's length is passed.
                let ([lx, ly], end) = match &path.ease {
                    Some(e) => {
                        let ([ex, _], _) = along(e, pr.path_d);
                        let max = e.iter().fold(0.0f32, |m, q| if m < q[0] { q[0] } else { m });
                        let (q, _) = along(&path.points, path_len(&path.points) * (ex / max));
                        (q, path_len(e) < pr.path_d)
                    }
                    None => (along(&path.points, pr.path_d).0, path_len(&path.points) < pr.path_d),
                };
                let lx = if pr.mirror { -lx } else { lx };
                let [c, s] = pr.fwd;
                // FollowPath.ProcessFrame (VERIFIED-DISASM @0xcdc450): (x x flip, y) turned by
                // Vector2.Rotate(projectile rotation), added to the start point.
                let next = match pr.grot {
                    Some(r) => {
                        let a = DEG2RAD * -r;
                        let (sa, ca) = crt_sin_cos(a);
                        // (The exported points' x is the clone's mirror of the game's.)
                        let gx = -lx;
                        let (ox, oy) = (ca * gx - sa * ly, ca * ly + sa * gx);
                        [pr.origin[0] + ox, pr.origin[1] - oy]
                    }
                    None => [pr.origin[0] + c * ly + s * lx, pr.origin[1] + s * ly - c * lx],
                };
                pr.vel = [next[0] - pr.pos[0], next[1] - pr.pos[1]];
                pr.pos = next;
                if end && path.destroy_at_end {
                    pr.frames_left = 0.0;
                }
            } else {
                // AccelerateModel: faster each frame up to maxSpeed.
                if let Some([a, max]) = pr.spec.accel {
                    let sp = pr.vel[0].hypot(pr.vel[1]);
                    if sp > 0.0 {
                        let k = (sp + a / FPS / FPS).min(max / FPS) / sp;
                        pr.vel = [pr.vel[0] * k, pr.vel[1] * k];
                    }
                }
                // TravelStrait.Process (VERIFIED-DISASM @0xd155a0): it moves while its age is under
                // the lifespan, else it expires where it is (recording dart_500: a Juggernaut's
                // burst comes from its last position).
                let last_frame = pr.spec.speed > 0.0 && pr.spec.seek.is_none() && pr.frames_left <= 1.0;
                if let (Some((t0, o, tg)), Some(ar)) = (pr.arrive_at, pr.spec.arrive.as_ref()) {
                    let perc = ((($sim_now - t0) as f32) * (1.0f32 / 60.0) / pr.arrive_time).min(1.0);
                    let e = curve_eval(&ar.samples, perc);
                    let np = [o[0] + (tg[0] - o[0]) * e, o[1] + (tg[1] - o[1]) * e];
                    pr.vel = [np[0] - pr.pos[0], np[1] - pr.pos[1]];
                    pr.pos = np;
                } else if !last_frame {
                    pr.pos[0] += pr.vel[0];
                    pr.pos[1] += pr.vel[1];
                    pr.seek_moved = true;
                }
                if pr.moving > 0.0 {
                    pr.moving -= 1.0;
                    if pr.moving <= 0.0 {
                        pr.vel = [0.0, 0.0];
                    }
                }
            }
            // (Not in the instant pass: a shot made this frame isn't processed until the next, so
            // its Age doesn't count yet; recording sub_030, Ballistic Missile's 0.6 s blast
            // waits 36 frames.)
            if (!pr.spec.round_time || $s.round_active) && $only.is_none() {
                pr.frames_left -= 1.0;
            }
            pr.age += 1.0;
            // ExpireProjectileAtScreenEdge.Process (VERIFIED-DISASM @0xcdaf10): Map.IsPointOnMap
            // (position, 0), after TravelStrait has moved it (every model lists it after the
            // travel behaviour; recording dart_500: the Ultra-Juggernaut expires at x -150.4).
            if pr.spec.expire_at_edge && !on_map(pr.pos, 0.0) {
                pr.frames_left = 0.0;
            }
            // ClearHitBloons.Process (VERIFIED-DISASM @0xcbb310): lastClear starts at the
            // creation time; once lastClear + intervalFrames (Math.Round(interval x 60)) <= now
            // the hit list is emptied, after this frame's collisions (recording boomerang_003:
            // a Kylie Boomerang on a Blue it hit at 1971 hits it again at 1975).
            if let Some(iv) = pr.spec.clear_hits {
                let frames = ((iv * FPS) as f64).round_ties_even() as f32;
                let now = $sim_now as f32;
                if frames + pr.clear_t <= now {
                    pr.clear_t = now;
                    pr.clear_hits_next = true;
                }
            }
            if let Some(iv) = pr.spec.refresh_pierce {
                pr.refresh_t += 1.0;
                if pr.refresh_t >= iv * FPS {
                    pr.refresh_t = 0.0;
                    // RefreshPierce.Process runs after the CollisionChecker: the pierce is
                    // back for the next frame's hits (recording wizard_040: a spent Wall of
                    // Fire refreshed at 234 hits again on 235).
                    pr.refresh_due = true;
                }
            }
            // ProjectileBlockerCollisionRebound.Process (VERIFIED-DISASM @0xcefc20): after it
            // moves, the first blocker area (in the map's order, not below it) whose edge the
            // move crossed from outside turns it: mirrored across that edge, the rest of the
            // move carried on from the crossing.
            // (Not once ExpireProjectileAtScreenEdge, listed before it, has expired it.)
            let edge_expired = pr.spec.expire_at_edge && pr.frames_left <= 0.0;
            if let (Some(rb), false, false) = (&pr.spec.blocker_rebound, pr.spec.ignore_blockers, edge_expired) {
                if let Some((at, dir)) = $s.areas.iter().filter(|a| a.is_blocker && a.height >= 0.0).find_map(|a| blocker_bounce(a, pr.prev, pr.pos)) {
                    pr.pos = at;
                    let sf = pr.spec.speed * FRAME_TIME;
                    pr.vel = [dir[0] * sf, dir[1] * sf];
                    pr.grot = None;
                    // (The game's Process runs after the frame's collisions.)
                    pr.clear_hits_next |= rb.clear;
                }
            }
            // MapBorderRebound.Process (VERIFIED-DISASM @0xcead80): when the last position was on
            // the map and this one isn't, the direction becomes normalize(2 (d.n) n - d), d =
            // last - now, n = (1, 0) past the left / right edge, else (0, 1); the position stays.
            // Started off the map, it never bounces (recording dart_500's twelve balls).
            if pr.spec.rebound && !edge_expired {
                if let Some(last) = pr.border_last {
                    if on_map(last, 0.0) && !on_map(pr.pos, 0.0) {
                        let n = if pr.pos[0] < -WORLD_X || pr.pos[0] > WORLD_X { [1.0f32, 0.0] } else { [0.0, 1.0] };
                        let d = [last[0] - pr.pos[0], last[1] - pr.pos[1]];
                        let dn = d[0] * n[0] + d[1] * n[1];
                        let r = [n[0] * 2.0 * dn - d[0], n[1] * 2.0 * dn - d[1]];
                        // Vector2.Normalize (x / |v|, |v| = sqrt(x x + y y) in f32), then TravelStrait's
                        // speedFrames (recording dart_500: a ball off the top edge).
                        let l = (r[0] * r[0] + r[1] * r[1]).sqrt();
                        if l > 0.0 {
                            let sf = pr.spec.speed * FRAME_TIME;
                            pr.vel = [r[0] / l * sf, r[1] / l * sf];
                            pr.grot = None;
                        }
                    }
                }
                pr.border_last = Some(pr.pos);
            }
            // Instant shots stay on their target (InstantModel.dontFollowTarget = false) and
            // expire if it is gone (HYPOTHESIS P-INSTANT).
            // TravelAlongPathModel: along the track at `speed`, backwards when `reverse`
            // (HYPOTHESIS P-TRACKTRAVEL: units per second like other speeds); it ends at the
            // track's ends.
            if let (Some((k, d)), Some(tt)) = (pr.on_track, pr.spec.track_travel.as_ref()) {
                let tr = &$s.tracks[k as usize];
                let nd = d + if tt.reverse { -tt.speed } else { tt.speed } / FPS;
                let np = tr.pos(nd.clamp(0.0, tr.length()));
                let np = [np[0] + pr.track_off[0], np[1] + pr.track_off[1]];
                pr.vel = [np[0] - pr.pos[0], np[1] - pr.pos[1]];
                pr.pos = np;
                pr.on_track = Some((k, nd));
                if nd < 0.0 || nd > tr.length() {
                    pr.frames_left = 0.0;
                }
            }
            // Frames before it can hit (burst shots, bounce delays, Mega Mine's arming).
            pr.wait = (pr.wait - 1.0).max(0.0);
            // (CollideOnlyWithTarget shots keep their Target as it is: no homing.)
            if let Some(id) = pr.target.filter(|_| pr.spec.instant || !pr.spec.only_target) {
                let at = $s.bloons.iter().find(|b| b.id == id).map(|b| b.pos);
                match (at, pr.spec.instant) {
                    // (Instant: its target's position once the bloons have moved, below.)
                    (Some(_), true) => {}
                    // Instant.Process (VERIFIED-DISASM @0xce0640): its target invalid, it stays
                    // at the bloon's last position, and expires there only with
                    // destroyIfInvalid (recording sub_050: Pre-emptive Strike's missile on a
                    // MOAB a Ballistic Missile popped lives out its lifespan).
                    (None, true) => {
                        if pr.spec.instant_destroy {
                            pr.frames_left = 0.0;
                        }
                        // CreateEffectProjectileAfterTime.SpawnEffect (VERIFIED-DISASM @0xca0540),
                        // useTargetPosition: a new Position at GetPositionOfTarget, after this
                        // frame's Instant.Process (which puts it back next frame). With the
                        // target gone that is the Target's cached first spot (recording sub_030:
                        // a Ballistic Missile 18 frames on, for one frame).
                        if let Some(back) = pr.effect_back.take() {
                            pr.pos = back;
                        }
                        if let (Some(f), Some(c)) = (pr.spec.effect_after_target, pr.target_cache) {
                            if pr.born / 4 + f as u64 == $s.sim_elapsed {
                                pr.effect_back = Some(pr.pos);
                                pr.pos = pr.target_obj.and_then(|o| $s.bloons.iter().find(|b| b.obj == o)).map_or(c, |b| [b.pos[0] + pr.inst_off[0], b.pos[1] + pr.inst_off[1]]);
                            }
                        }
                    }
                    // Bounced glaives (HYPOTHESIS P-BOUNCE-TRAVEL) home in on their next bloon
                    // at their own speed and fly on straight if it is gone.
                    (Some(q), false) => {
                        let (dx, dy) = (q[0] - pr.pos[0], q[1] - pr.pos[1]);
                        let l = (dx * dx + dy * dy).sqrt().max(1e-3);
                        let v = pr.spec.speed / FPS;
                        pr.vel = [dx / l * v, dy / l * v];
                    }
                    (None, false) => pr.target = None,
                }
            }
        }
        // Cash never hits bloons. Without a PickupModel (Marketplace) it pays out at once.
        if let Some(c) = &$s.projectiles[$p].spec.cash {
            if c.pickup.is_none() {
                let pr = $s.projectiles.remove($p);
                $s.collect_projectile_cash(&pr);
                $s.sound(&pr.spec.pickup_sounds, None);
                $s.cash_pops.push((pr.pos, pr.value));
            } else if $s.projectiles[$p].frames_left <= 0.0 {
                $s.projectiles.remove($p);
            } else {
                $p += 1;
            }
            continue;
        }
        // CollisionChecker.Process (VERIFIED-DISASM @0x180f31590) skips a projectile whose
        // model has dontUseCollisionChecker (recording bomb_005: Bomb Blitz's Cluster
        // fragments, made inside a MOAB, never hit it). Not the instant pass.
        let flying = ($s.projectiles[$p].line_spent && $only.is_none()) || $s.projectiles[$p].spec.no_hit
            // (Lightning hits through Lightning.Process, emulated by this pass: P-LIGHTNING.)
            || ($s.projectiles[$p].spec.no_checker && !$s.projectiles[$p].spec.lightning && $only.is_none())
            || ($s.projectiles[$p].moving > 0.0 && $s.projectiles[$p].spec.arrive.as_ref().is_some_and(|a| a.filter_moving));
        // Broad phase: the bloons in grid cells within reach, in storage order (the order
        // a full scan would meet them in).
        $cand.clear();
        // Projectile.HasValidBloonCollision (VERIFIED-DISASM @0xcabcf0): with
        // checkCollisionIntervalFrames, bloons are only checked once that many frames have
        // passed since the last check (recording glue_400: a Liquefier linger, every 19).
        let checks = {
            let pr = &mut $s.projectiles[$p];
            let f = pr.spec.check_frames;
            if f > 0 && ($s.sim_elapsed as i64 - pr.check_t as i64) < f as i64 {
                false
            } else {
                if f > 0 {
                    pr.check_t = $s.sim_elapsed;
                }
                true
            }
        };
        if checks && !flying && $s.projectiles[$p].pierce > 0.0 && $s.projectiles[$p].wait <= 0.0 {
            let pr = &$s.projectiles[$p];
            // Positions from the frame's start: allow for this frame's movement.
            let at = if pr.spec.instant || (pr.target.is_some() && !pr.spec.only_target) { pr.pos } else { pr.prev };
            if $s.has_grid_rank && !(pr.spec.instant || (pr.target.is_some() && !pr.spec.only_target)) {
                // CollisionChecker.Process: gridMargin 40 (0 for shots that can't hit MOABs).
                let margin = if pr.spec.exclude_tags.iter().any(|t| t == "Moabs") { 0.0 } else { 40.0 };
                $s.ggrid.query(at, pr.spec.radius, margin, &$s.grid_offsets, &mut $cand);
            } else {
                // A bloon in the projectile's own 10-unit cell can be up to ~14 away.
                $s.grid.query(at, (pr.spec.radius + $s.max_bloon_radius).max(GAME_GRID_CELL * std::f32::consts::SQRT_2) + $s.max_bloon_step, &mut $cand);
            }
        }
        // CollisionGrid query order: cells by orderedNeighborOffsets around the projectile's
        // cell, bloons within a cell in list order. Only bloons it can reach are kept (the hit
        // test below rejects the rest anyway), so the sort stays small.
        if $cand.len() > 1 && $s.has_grid_rank && ($s.projectiles[$p].spec.instant || $s.projectiles[$p].target.is_some()) {
            let pr = &$s.projectiles[$p];
            let direct = pr.spec.instant || (pr.target.is_some() && !pr.spec.only_target);
            let at = if direct { pr.pos } else { pr.prev };
            let (bl, defs) = (&$s.bloons, &$s.defs);
            $cand.retain(|&i| {
                let b = &bl[i as usize];
                // (An instant shot on a bloon: the bloon's grid cell, as the hit test below.)
                let (bp, bc) = if direct && !(pr.spec.instant && pr.target.is_some()) { (b.pos, b.pos) } else if direct { (b.pos, b.cell_pos) } else { (b.prev, b.cell_pos) };
                grid_hit(at, pr.spec.radius, bp, bc, defs[b.kind].radius)
            });
            // With pierce for all of them, the order can't change who gets hit.
            if $cand.len() > 1 && ($cand.len() as f32) > pr.pierce {
                let pc = grid_cell(at);
                let rank = &$s.grid_rank;
                $cand.sort_by_key(|&i| {
                    let b = &bl[i as usize];
                    let bc = grid_cell(if direct { b.pos } else { b.cell_pos });
                    let (dx, dy) = (bc.0 - pc.0, bc.1 - pc.1);
                    let r = if dx.abs() <= 64 && dy.abs() <= 64 { rank[((dx + 64) * 129 + (dy + 64)) as usize] } else { u16::MAX };
                    (r, b.order)
                });
            }
        }
        // Projectile.CollideBloon from InstantDamageEmission.Process: the target itself, with
        // no distance test (only the collision filters).
        if $only.is_some() {
            $cand.clear();
            // (A plain Line: its GetInLine list, in that order, from its emission.)
            if let Some(ids) = $s.projectiles[$p].line_hits.take() {
                for id in ids {
                    if let Some(i) = $s.bloons.iter().position(|b| b.id == id) {
                        $cand.push(i as _);
                    }
                }
            } else {
                let t = $s.projectiles[$p].target;
                if let Some(i) = $s.bloons.iter().position(|b| Some(b.id) == t) {
                    $cand.push(i as _);
                }
            }
        }
        let mut k = 0;
        while !flying && k < $cand.len() && $s.projectiles[$p].pierce > 0.0 && $s.projectiles[$p].wait <= 0.0 {
            let i = $cand[k] as usize;
            let len0 = $s.bloons.len();
            let b = &$s.bloons[i];
            let d = &$s.defs[b.kind];
            let pr = &$s.projectiles[$p];
            // Straight-flying shots meet bloons where both were when the frame began.
            let (bp, pp) = if pr.spec.instant || pr.target_moved || (pr.target.is_some() && !pr.spec.only_target) { (b.pos, pr.pos) } else { (b.prev, pr.prev) };
            // (Where an instant shot is when it meets its own Target: on it; see below.)
            let inst_pp = if pr.spec.instant && pr.target == Some(b.id) { [bp[0] + pr.inst_off[0], bp[1] + pr.inst_off[1]] } else { pp };
            let rr = pr.spec.radius + d.radius;
            // (An instant shot on a bloon meets the bloons where the BloonGrid has them, as the others do:
            // the CollisionChecker goes before the bloons move. Recording sniper_030 rt 2876: a bullet
            // bounced onto a Blue in the next cell doesn't hit it till the grid has the Blue in its own.
            // Wall of Fire, on a track spot, keeps the bloon's spot: recording wizard_025.)
            let bcell = if (pr.spec.instant && pr.target.is_none()) || (pr.target.is_some() && !pr.spec.only_target && !pr.spec.instant) { b.pos } else { b.cell_pos };
            let direct = $only.is_some();
            if (!direct && !grid_hit(pp, pr.spec.radius, bp, bcell, d.radius))
                || (!direct && $s.has_grid_rank && !$s.ggrid.visits(pp, pr.spec.radius, i, bcell))
                || (!direct && !circles_hit(pp, pr.spec.radius, bp, b.rot, &d.circles))
                || (pr.spec.instant && pr.spec.only_target && pr.target.is_some_and(|t| t != b.id))
                // CollideOnlyWithTarget.FilterCollision (VERIFIED-DISASM @0xcbb6e0): only its
                // Target while that is valid; none once it is gone (recording boomerang_500: a
                // MoabDot whose glaive's target popped sits out its life, its pierce unused).
                // VERIFIED-DISASM @0x180c9cbd0: with no valid Target it takes only its first
                // collision, when ignoreFirstCheck was set (SetTarget with no bloon and
                // collideWithFirstIfInvalidTarget); else nothing (recording tack_520: Inferno
                // Ring's meteor whose MOAB popped flies past a Black).
                || (pr.spec.only_target && !pr.spec.instant && match pr.target.or(pr.cot) {
                    Some(t) => t != b.id,
                    None => !pr.ignore_first,
                })
                || b.hit_by(&pr.hit)
                || pr.carrier == b.id
                // Projectile.IsCollisionValid (VERIFIED-DISASM @0xcac350): with
                // canCollisionBeBlockedByMapLos, Map.IsTargetLineClear from the shot to the
                // bloon (recording bomb_100: a blast misses a Yellow behind the entrance trees).
                || (pr.spec.los_blocked && !$s.line_clear(pp, 0.0, bp))
                || pr.spec.exclude_bloons.iter().any(|x| $s.names[b.kind].starts_with(x.as_str()))
                // FilterMutatedTarget.FilterBloon (VERIFIED-DISASM @0x180c20240): recording
                // ice_005, icicles never hit a bloon carrying Ice:Icicles.
                || pr.spec.skip_mutated.iter().any(|m| b.mutated_by(m))
                || (!pr.spec.need_mutated.is_empty() && !pr.spec.need_mutated.iter().all(|m| b.mutated_by(m)))
                || pr.spec.exclude_tags.iter().any(|t| d.tags.contains(t))
                || (pr.spec.glue_filter > 0 && b.glue() >= pr.spec.glue_filter)
                || (pr.spec.skip_frozen && b.frozen())
                // FilterInvisible: a shot from a tower without camo detection passes
                // camo bloons (recording wizard_050: a Wall of Fire beside a Camo
                // Rainbow keeps its pierce).
                // FilterInvisible.FilterBloon (VERIFIED-DISASM @0xc1fb70): an active one
                // passes no camo bloon, whatever the attack sees (recording engineer_025:
                // the XXXL Trap's collision filter skips a Camo Rainbow its attack can aim at).
                || (!pr.buff_camo && b.camo(d) && pr.spec.filters.iter().any(|f| f == "FilterInvisibleModel"))
                || cant_hit(pr.spec.cant_hit, &pr.spec, b.properties(d))
                || pr.spec.override_filter.as_ref().is_some_and(|f| overriding_mutated(b, f))
            {
                k += 1;
                continue;
            }
            // HYPOTHESIS P-EAT: a trap swallows every bloon it touches whole (no pops, no pop
            // cash) while it holds less than rbeCapacity RBE; then it closes and waits to be
            // collected for the eaten RBE x rbeCashMultiplier.
            if let Some(eat) = pr.spec.eat.clone() {
                let rbe = $s.rbe[b.kind] as f64;
                let len0 = $s.bloons.len();
                $s.remove_bloon(i);
                // The old last bloon now sits in slot i.
                remap_moved(&mut $cand, k, i, len0);
                k += 1;
                let pr = &mut $s.projectiles[$p];
                pr.value += rbe;
                if pr.value >= eat.capacity as f64 {
                    pr.value *= eat.cash_mult as f64;
                    pr.cash_base=pr.value;
                    // STAND-IN pickup reach (a quarter of this: 15 units), like bananas.
                    pr.spec.cash = Some(crate::data::Cash { min: 0.0, max: 0.0, bonus: 0.0, pickup: Some(60.0),bonus_shared:false });
                    pr.spec.no_hit = true;
                    break;
                }
                continue;
            }
            let id = b.id;
            let layer = b.layer;
            let immune = pr.spec.immune & b.properties(d) != 0;
            // HYPOTHESIS P-CRITTEXT: ShowTextOnHit's "Crit!" shows on crit shots only.
            let crit_text = (pr.crit.is_some() && pr.spec.hit_text).then_some(b.pos);
            let hit_z = b.height;
            let mut popped_now = 0u64;
            let mut dmg = pr.crit.unwrap_or(pr.spec.damage);
            for m in &pr.spec.state_mods {
                if m.states.iter().any(|s| b.in_state(s)) {
                    dmg = dmg * m.mult + m.add;
                }
            }
            // DesperadoMarkModel: marked bloons take extra damage from Desperados.
            if let Some(m) = b.effects.iter().find_map(|e| match e.kind { EffectKind::Mark { damage_mult, .. } => Some(damage_mult), _ => None }) {
                if $s.towers.iter().find(|t| t.id == pr.owner).is_some_and(|t| $s.tower_defs[t.def].0 == "Desperado") {
                    dmg *= m;
                }
            }
            if let Some([mult, add, at]) = pr.spec.cash_damage {
                if $s.cash_for_tower(pr.owner) >= at as f64 {
                    dmg = dmg * mult + add;
                }
            }
            // HYPOTHESIS P-SCALEDMG: ScaleDamageWithTimeModel multiplies the damage by
            // min(maxDamage, baseDamage + scalePerSecond x seconds flown) / baseDamage.
            if let Some([base, per, max]) = pr.spec.scale_damage.filter(|s| s[0] > 0.0) {
                dmg *= (base + per * pr.age / FPS).min(max) / base;
            }
            if pr.spec.damage > 0.0 {
                dmg += b.bonus_damage();
            }
            for m in &pr.spec.tag_mods {
                if tag_match(&d.tags, m) {
                    dmg = dmg * m.mult + m.add;
                }
            }
            for m in &pr.spec.type_mods {
                let name = &$s.names[b.kind];
                if name == &m.id || (m.children && name.starts_with(m.id.as_str())) {
                    dmg = dmg * m.mult + m.add;
                }
            }
            if let Some(w) = &pr.spec.wrath {
                dmg += ($s.active_rbe / w.threshold * w.damage).clamp(0.0, w.max);
            }
            // EmitOnDamage.Collide (VERIFIED-DISASM @0xcbb8c0) asks Bloon.WillDamageBloon rather
            // than looking at a damage done: its fragments are made before the hit's pops
            // (recording desperado_005h rt 1338: a pellet's three Shrapnel get ids before the
            // Desert Phantom explosion of the Yellow it popped, and hit first).
            // (Damage.Collide's crit roll comes after: recording desperado_025g rt 75, a pellet's
            // three Shrapnel draw 18 synced randoms, then its crit roll.)
            if !immune && $s.projectiles[$p].spec.on_contact.iter().any(|c| c.on_damage && !c.on_pop) {
                let pr = &$s.projectiles[$p];
                let heading = if pr.spec.instant && pr.target.is_some() { (bp[1] - pr.origin[1]).atan2(bp[0] - pr.origin[0]) } else { pr.vel[1].atan2(pr.vel[0]) };
                let (tiers, owner) = (pr.tiers, pr.owner);
                let grot = pr.grot.or_else(|| $s.towers.iter().find(|t| t.id == owner).and_then(|t| t.game_rot).map(|r| { let d = game_dir(r); RAD2DEG * crt_atan2(d[0], -d[1]) }));
                let early: Vec<_> = pr.spec.on_contact.iter().filter(|c| c.on_damage && !c.on_pop).cloned().collect();
                // (They carry the shot's crit roll as it is then, rolled or not: recording
                // desperado_025g, Shrapnel from a pellet's first damaging hit (made before its roll)
                // each roll at their own first damaging hits (rt 80: the second crits); those from
                // its later hits never roll (rt 81).)
                let parent_crit = pr.crit_roll;
                for c in early {
                    let start = $s.projectiles.len();
                    $s.emit_z = hit_z;
                    $s.emit_rot = grot;
                    $s.emit(bp, heading, &c.emission, &c.projectile, None, tiers, owner);
                    $s.emit_rot = None;
                    $s.emit_z = 0.0;
                    for pr in &mut $s.projectiles[start..] {
                        pr.hit.push(layer);
                        if parent_crit.is_some() {
                            pr.crit_roll = parent_crit;
                        }
                    }
                }
            }
            // Projectile.RollForCrits (VERIFIED-DISASM @0xc8df40), from Damage.Collide after the
            // damage is worked out: the first hit rolls syncedRandom.NextDouble <= chance; a
            // crit is damage x damageMultiplier + damageAdditive on every hit after.
            if let Some(mut cr) = $s.projectiles[$p].crit_roll {
                if cr[3] == 0.0 {
                    cr[3] = 1.0;
                    if $s.synced_double() <= cr[0] as f64 {
                        cr[4] = 1.0;
                    }
                    $s.projectiles[$p].crit_roll = Some(cr);
                }
                if cr[4] != 0.0 {
                    dmg = cr[1] * dmg + cr[2];
                }
            }
            $kids.clear();
            // CreateProjectileOnContact.Collide runs in the collision pass, before the hit's
            // damage and its pops (recording glue_025: a glob's Glue Splatter splat comes
            // before the splat its popped Pink leaves, and pops the Reds first).
            if $s.projectiles[$p].spec.on_contact.iter().any(|c| !c.on_damage && !c.on_pop && !c.after_damage) {
                let pr = &$s.projectiles[$p];
                let heading = if pr.spec.instant && pr.target.is_some() { (bp[1] - pr.origin[1]).atan2(bp[0] - pr.origin[0]) } else { pr.vel[1].atan2(pr.vel[0]) };
                let now: Vec<_> = pr
                    .spec
                    .on_contact
                    .iter()
                    .filter(|c| !c.on_damage && !c.on_pop && !c.after_damage)
                    .map(|c| (if c.at_shot { pp } else { bp }, c.clone(), c.pass_on_collided.then_some(layer)))
                    .collect();
                let (tiers, owner, grot) = (pr.tiers, pr.owner, pr.grot);
                for (at, c, skip) in now {
                    let start = $s.projectiles.len();
                    $s.emit_z = hit_z;
                    // CreateProjectileOnContact.Collide (@0xcc1570) emits from the projectile's
                    // own rotation (game frame), turned by Vector2.Rotate (recording bomb_003:
                    // a frag a float step off with the radians heading).
                    // ArcEmission.GetDirection (VERIFIED-DISASM @0x180c37590): the rotation is
                    // the owner tower's at that moment (its emission behaviours may change it),
                    // not the projectile's (recording bomb_032: a blast's frags from 34.864883,
                    // the tower turned on since the bomb's 34.864876). Other emissions keep the
                    // projectile's (Sniper's EmitOnDamage fans).
                    $s.emit_rot = if c.emission.kind == "Arc" && c.emission.dir_off.is_none() && !c.on_damage {
                        $s.towers.iter().find(|t| t.id == owner).and_then(|t| t.game_rot).or(grot)
                    } else {
                        grot
                    };
                    $s.emit(at, heading, &c.emission, &c.projectile, None, tiers, owner);
                    $s.emit_rot = None;
                    $s.emit_z = 0.0;
                    if let Some(id) = skip {
                        for pr in &mut $s.projectiles[start..] {
                            pr.hit.push(id);
                        }
                    }
                }
            }
            if let Some(at) = crit_text {
                $s.hit_texts.push(at);
            }
            let mut immune_destroyed = false;
            if immune {
                let props = $s.bloons[i].properties(&$s.defs[$s.bloons[i].kind]);
                $s.immune_sound(props);
                // Bloon.Damage with destroyProjectileIfImmune (VERIFIED-DISASM Damage.Collide
                // @0xcd5e63; recording dart_upgrades_r30: a dart on a Lead is gone), only when
                // BloonPropertyHelper.DestroysProjectilesIfImmune (@0x51c3a0): the immunity it
                // met is Lead, Purple or Frozen, or White for Frigid damage (recording bomb_100:
                // a blast on a Black lives on). GetImmunitiesFromDamageType("Frigid") is 12
                // (BloonPropertyHelper.cctor @0x51d2d6; recording ice_001: a base blast on a
                // White lives on).
                let spec = &$s.projectiles[$p].spec;
                let mask = if spec.immune == 12 { 0x1d } else { 0x19 };
                if !spec.immune_keep && spec.damage > 0.0 && spec.immune & mask & props != 0 {
                    $s.projectiles[$p].pierce = 0.0;
                    $s.projectiles[$p].frames_left = 0.0;
                    immune_destroyed = true;
                }
                $s.pre_damage_effects(i, $p, dmg);
            }
            if !immune {
                let spec = &$s.projectiles[$p].spec;
                let cleanse = spec.decamo || spec.cleanse_regrow || spec.cleanse_fortified || spec.cleanse_lead;
                if cleanse && (dmg >= 0.5 || !spec.decamo_needs_damage) {
                    if spec.decamo {
                        $s.bloons[i].decamo = true;
                    }
                    // RemoveBloonModifiers.Collide (@0xcd5820): Mutable.UpdateRootModel to
                    // GetCleansed's model (@0xcd5d90): the id without "Camo" / "Regrow" /
                    // "Fortified" as the model asks (RainbowCamo -> Rainbow; recording
                    // wizard_502). A Reanimated bloon is left as it is.
                    let d0 = &$s.defs[$s.bloons[i].kind];
                    if !d0.tags.iter().any(|t| t == "Reanimated") {
                        let mut name = $s.names[$s.bloons[i].kind].clone();
                        if spec.decamo {
                            name = name.replace("Camo", "");
                        }
                        if spec.cleanse_regrow {
                            name = name.replace("Regrow", "");
                        }
                        if spec.cleanse_fortified {
                            name = name.replace("Fortified", "");
                        }
                        if let Some(k) = $s.names.iter().position(|n| *n == name) {
                            $s.bloons[i].kind = k;
                        }
                    }
                    // cleanseLead: a Lead takes its health (Bloon.Damage, ebp = health):
                    // its layer goes (recording engineer_030: Cleansing Foam pops a Lead
                    // into two Blacks).
                    if spec.cleanse_lead && $s.defs[$s.bloons[i].kind].tags.iter().any(|t| t == "Lead") {
                        let hp = $s.bloons[i].hp;
                        $s.damage(i, hp, &mut $kids);
                    }
                }
                $s.pre_damage_effects(i, $p, dmg);
                // HYPOTHESIS P-LEADGOLD: Lead to Gold pays its flat amount when a hit pops a
                // bloon with the tag (the game attaches it to the bloon for its next pop).
                let gold = $s.projectiles[$p].spec.cash_on_pop.as_ref().filter(|c| $s.defs[$s.bloons[i].kind].tags.contains(&c.tag)).map(|c| c.amount as f64);
                let tally0 = $s.damage_tally;
                $s.distribute = ($s.projectiles[$p].spec.distribute, $s.projectiles[$p].spec.distribute_override);
                let n = $s.damage(i, dmg.round() as i32, &mut $kids);
                $s.distribute = (true, false);
                // FreezeModel.applyAfterDamage: Projectile.CollideBloon runs the behaviours by
                // collision pass, the freeze (pass 1) after the damage, on what the hit bloon
                // is now and (cascadeMutators) the children it made (recording ice_052: a Pink
                // hit for its last layer by the Re-Freeze pulse leaves a frozen Yellow; ice_000,
                // a Black's second Pink frozen too). INFERRED: every child with the full layers.
                if $s.projectiles[$p].spec.effects.iter().any(|e| e.after_damage) {
                    let (spec, owner) = ($s.projectiles[$p].spec.clone(), $s.projectiles[$p].owner);
                    for kid in std::iter::once(id).chain($kids.iter().copied()).collect::<Vec<_>>() {
                        if let Some(j) = $s.bloons.iter().position(|b| b.id == kid) {
                            $s.apply_effects(j, &spec, owner, dmg, true);
                        }
                    }
                }
                let self_tally_dealt = $s.damage_tally - tally0;
                // Knockback.Collide (VERIFIED-DISASM @0xce56e0), after the damage: on what the
                // hit bloon is now and what it split into (their timer starts with them: they
                // don't move this frame; recording dart_500), unless it
                // isn't valid for pushback (IsBloonValidForPushback: on the
                // map; its near-the-exit checks aren't done). It turns at once: a first child
                // that already moved this frame moves at the new speed.
                if let Some(kb) = $s.projectiles[$p].spec.knockback.clone() {
                    let mut valid = None;
                    for kid in std::iter::once(id).chain($kids.iter().copied()) {
                        let Some(j) = $s.bloons.iter().position(|b| b.id == kid) else { continue };
                        let tags = &$s.defs[$s.bloons[j].kind].tags;
                        let has = |t: &str| tags.iter().any(|x| x == t);
                        let m = if has("Moabs") { kb.moab } else if has("Lead") || has("Ceramic") || has("Fortified") { kb.heavy } else { kb.light };
                        // Bloon.IsBloonValidForPushback (VERIFIED-DISASM @0xfd2d50): on the map, not
                        // behind spawnDist - 1.25 x radius, and at least 10 past the centre of the
                        // first segment whose point is on the map (distance as the frame began).
                        let path = $s.bloons[j].path as usize;
                        let at = if $s.bloons[j].dist != $s.bloons[j].prev_dist { $s.bloons[j].prev_dist } else { $s.bloons[j].dist };
                        let spawn_ok = $s.track_ends.get(path).and_then(|e| e.0).is_none_or(|sd| at >= sd - $s.defs[$s.bloons[j].kind].radius * 1.25);
                        let first_seen = $s.track_segs.get(path).and_then(|sg| sg.iter().position(|q| on_map(*q, 0.0)));
                        let past_entry = first_seen.is_none_or(|k| at >= k as f32 * 7.0 + 3.5 + 10.0);
                        let b = &mut $s.bloons[j];
                        // The hit bloon decides for its children (they carry its mutator on;
                        // recording dart_520: a Black's second Red, made behind the first
                        // on-map segment, goes back with it).
                        // (A bloon already knocked back takes it again: Knockback.Collide's
                        // IsMutatedBy check isn't for its own mutator; another Knockback mutator goes on
                        // with a full lifespan. INFERRED from recording desperado_004 rt 261: a third
                        // pellet a frame after the first two keeps the MOAB going back 12 frames, not 11.)
                        let ok = *valid.get_or_insert(on_map(b.pos, 0.0) && spawn_ok && past_entry);
                        let immune = $s.defs[b.kind].bad_immunity
                            && ($s.mutator_data.bad_immunity_ids.iter().any(|$p| "Knockback".starts_with($p.as_str())) || $s.mutator_data.bad_immunity_tags.iter().any(|t| "Knockback".contains(t.as_str())));
                        if immune {
                            continue;
                        }
                        if !ok {
                            continue;
                        }
                        b.effects.push(Effect { overlay: Default::default(), seq: 0, kind: EffectKind::Knock { m }, id: "Knockback".into(), frames_left: (kb.lifespan * FPS).round(), total: (kb.lifespan * FPS).round(), added: $s.sim_elapsed, layers: u32::MAX, owner: 0, distribute: (true, false), keep: false });
                        if b.dist != b.prev_dist {
                            let s = speed_frames(b.model_speed(b.frozen_speed(&$s.defs[b.kind], $s.defs[b.kind].speed)));
                            b.dist = (b.prev_dist + s * b.track_mult * 1.0 * b.mult).max(0.0);
                            b.pos = $s.tracks[b.path as usize].bloon_pos(b.dist);
                        }
                    }
                }
                popped_now = n;
                if let (Some(v), true) = (gold, n > 0) {
                    $s.earn(v);
                    $s.cash_pops.push((bp, v));
                }
                let owner = $s.projectiles[$p].owner;
                if let Some(ti) = $s.towers.iter().position(|t| t.id == owner) {
                    $s.towers[ti].pops += n;
                    $s.towers[ti].sun_destroyed($s.last_destroyed);
                    $s.towers[ti].damage += self_tally_dealt;
                    $s.stats.pops += n;
                    $s.stats.damage += self_tally_dealt;
                    // Monkey Town (CashIncreaseModel): pop cash x multiplier.
                    let m = $s.buffs.get(ti).map_or(1.0, |b| b.cash_mult as f64);
                    $s.earn(n as f64 * (m - 1.0));
                }
            }
            // WindModel (Distraction): Wind.Collide runs after the DamageModel (recording
            // ninja_010 rt 2069: no roll for a Red the hit destroyed), on what the bloon is now.
            // (A bloon this hit destroyed isn't IsBloonValidForPushback (isDestroyed); a
            // degraded one is the same object, found by its id.)
            if let (Some(w), Some(i)) = ($s.projectiles[$p].spec.wind.clone(), $s.bloons.iter().position(|b| b.id == id)) {
                let is_moab = $s.defs[$s.bloons[i].kind].tags.iter().any(|t| t == "Moabs");
                // Wind.Collide (VERIFIED-DISASM @0xcf7fd0): syncedRandom.NextDouble <= chance;
                // MoveWithWind.Attatched (@0xffec70): NextDouble x (max - min) + min back.
                // Not on a bloon already carried (offTrackCount) or not
                // IsBloonValidForPushback (@0xfb4240: on the map, past spawnDist - radius
                // x 1.25); tagFilter: skip when (a tag with presence == tagInclusive) ==
                // tagInclusive.
                let b = &$s.bloons[i];
                let d = &$s.defs[b.kind];
                let vd = b.dist;
                let behind = $s.track_ends.get(b.path as usize).and_then(|e| e.0).is_some_and(|sd| vd < sd - d.radius * 1.25);
                let filtered = !w.tag_filter.is_empty() && w.tag_filter.iter().any(|t| d.tags.contains(t) == w.tag_inclusive) == w.tag_inclusive;
                // (… nor within 10 past the path's first on-map segment: recording ninja_010,
                // the game's first wind roll is on rt 49's hit, not the entering bloon's on 15.)
                // (Path.segments: the first whose point is on the map, its centre.)
                let pi = b.path as usize;
                let first = $s.track_segs.get(pi).and_then(|sg| sg.iter().position(|&q| on_map(q, 0.0))).and_then(|k| $s.seg_span(pi, k)).map(|x| x.0);
                let entering = first.is_some_and(|f| vd < f + 10.0);
                // (isDestroyed: a bloon this hit popped outright is still listed here at 0 hp.)
                // (MoveWithWind.Attatched / OnDestroy (VERIFIED-DISASM @0xffec70 / @0xffee80) keep
                // offTrackCount: a carried bloon, even one blown back earlier this frame, isn't
                // rolled for; recording ninja_320 rt 2005.)
                let valid = b.hp > 0 && b.wind.is_none() && on_map(b.pos, 0.0) && !behind && !filtered && !entering;
                // (Not after an immune hit: the shot bounces off before its collide behaviours;
                // recording ninja_030, no roll for the Leads until rt 2582.)
                let roll = (valid && !immune && (w.moabs || !is_moab)).then(|| $s.synced_double());
                if roll.is_some_and(|r| r <= w.chance as f64) {
                    let mut back = ($s.synced_double() as f32) * (w.max - w.min) + w.min;
                    if w.tag_scale > 0.0 && w.tags.iter().any(|t| $s.defs[$s.bloons[i].kind].tags.contains(t)) {
                        back *= w.tag_scale;
                    }
                    // MoveWithWind.Attatched: distanceTraveled less it (not behind spawnDist -
                    // radius x 1.25), then it is carried there (offTrackCount).
                    let d0 = $s.bloons[i].dist;
                    $s.push_back(i, back);
                    let kind = $s.bloons[i].kind;
                    let r = $s.defs[kind].radius;
                    let sd = $s.track_ends.get($s.bloons[i].path as usize).and_then(|e| e.0);
                    let b = &mut $s.bloons[i];
                    if let Some(sd) = sd {
                        if b.dist <= sd - r * 1.25 {
                            b.dist = sd - r * 1.25;
                        }
                    }
                    b.wind = Some(w.speed_mult);
                    b.wind_new = true;
                    b.wind_seg = Some(d0);
                    b.wind_direct = true;
                    b.wind_model = Some(Box::new(w.clone()));
                    // MoveWithWind is a mutator the hit's other children take over, each
                    // Attatched with its own distance draw from where it was placed
                    // (recordings ninja_030 rt 2616 / ninja_020 rt 3029: a White's two Pinks
                    // both held at 173.6, a third synced draw that frame).
                        // (Each new object once: a child the pop took further, the same object, is
                    // listed again; recording ninja_520 rt 247.)
                    let mut seen = std::collections::HashSet::new();
                    for kid in $kids.clone().into_iter().filter(|&k| k != id && seen.insert(k)) {
                        if let Some(k) = $s.bloons.iter().position(|b| b.id == kid) {
                            let mut back = ($s.synced_double() as f32) * (w.max - w.min) + w.min;
                            if w.tag_scale > 0.0 && w.tags.iter().any(|t| $s.defs[$s.bloons[k].kind].tags.contains(t)) {
                                back *= w.tag_scale;
                            }
                            let k0 = $s.bloons[k].dist;
                            $s.push_back(k, back);
                            let kr = $s.defs[$s.bloons[k].kind].radius;
                            let b = &mut $s.bloons[k];
                            if let Some(sd) = sd {
                                if b.dist <= sd - kr * 1.25 {
                                    b.dist = sd - kr * 1.25;
                                }
                            }
                            b.wind = Some(w.speed_mult);
                            b.wind_new = true;
                            b.wind_seg = Some(k0);
                            b.wind_direct = false;
                            b.wind_model = Some(Box::new(w.clone()));
                        }
                    }
                }
            }
            // PushBack.Collide (VERIFIED-DISASM @0xcf29a0): pushAmount x the class multiplier,
            // through Bloon.Move(-amount), which keeps it from trailing the entrance by more than
            // 1.25 x its radius (recording boomerang_005: a MOAB pressed back to 157.35 moves on
            // from there once its stun is gone). The model lists it after the DamageModel
            // (same collision pass), so it acts on what the hit bloon has become (recording
            // boomerang_paragon_r98: a pressed BFB that pops goes back 1, a MOAB's push).
            // (Not on a boss or a bloon with BadImmunityModel: recording
            // boomerang_paragon_r98, the press leaves a BAD where it is.)
            if let Some(pb) = $s.projectiles[$p].spec.push.clone().filter(|pb| !(pb.only_if_damaged && (immune || dmg < 1.0))) {
                if let Some(i) = $s.bloons.iter().position(|b| b.id == id) {
                    let d = &$s.defs[$s.bloons[i].kind];
                    if !d.bad_immunity && !d.tags.iter().any(|t| t == "Boss") && (pb.tag.is_empty() || d.tags.contains(&pb.tag)) {
                        let m = moab_class(&$s.names[$s.bloons[i].kind]).and_then(|k| pb.mult.get(k)).copied().unwrap_or(1.0);
                        let back = pb.amount * m;
                        let r = d.radius;
                        $s.push_back(i, back);
                        if let Some(sd) = $s.track_ends.get($s.bloons[i].path as usize).and_then(|e| e.0) {
                            let lim = sd - r * 1.25;
                            let b = &mut $s.bloons[i];
                            if b.dist <= lim && lim != b.dist {
                                b.dist = lim;
                            }
                        }
                    }
                }
            }
            if !$s.projectiles[$p].spec.hit_sounds.is_empty() {
                let names = $s.projectiles[$p].spec.hit_sounds.clone();
                $s.sound(&names, None);
            }
            // CreateLightningEffect: a bolt from where the last jump ended (the shot's origin
            // first) to the bloon it hit (visual).
            if let Some(bl) = $s.projectiles[$p].spec.bolts.clone() {
                let to = $s.bloons.iter().find(|b| b.id == id).map_or($s.projectiles[$p].prev, |b| b.pos);
                let pr = &mut $s.projectiles[$p];
                let from = pr.bolt_from.unwrap_or(pr.origin);
                pr.bolt_from = Some(to);
                let uid = pr.uid;
                $s.bolts.push((from, to, bl, uid));
            }
            // CreateEffectOnContact: its effect where it hit (visual).
            if !$s.projectiles[$p].spec.contact_fx.is_empty() {
                let at = $s.projectiles[$p].prev;
                for (fx, k) in $s.projectiles[$p].spec.contact_fx.clone() {
                    $s.place_fx.push((at, fx, k));
                }
            }
            let pr = &mut $s.projectiles[$p];
            // CollideBloon adds the layer it hit; what that layer became carries it in its
            // lineage.
            pr.hit.push(layer);
            pr.ignore_first = false;
            // CollideBloon: timeSinceLastCollisionCheck = 0.
            pr.check_t = 0;
            // TrackTarget.Collide (VERIFIED-DISASM @0xd128d0): hitting its own target, it
            // looks again at once (lastTargetSearchElapsed = 0) and, with trackNewTargets,
            // drops it (recording sub_000_close: a torpedo turns to the next bloon after
            // hitting its first).
            // TrackTarget.Collide: lastTargetSearchElapsed = 0, so its Process this frame looks
            // (`track_targets`).
            if pr.seek == Some(id) {
                pr.seek_t = 0;
            }
            pr.pierce -= pr.spec.pierce_cost;
            // Projectile.CollideBloon (VERIFIED-DISASM @0xc8b970): Exhausted once pierce is
            // under 1 (not ignorePierceExhaustion): a fractional rest is spent (recording
            // ninja_040: a Shinobi-buffed shuriken goes with 0.16 left).
            if pr.pierce > 0.0 && pr.pierce < 1.0 && !pr.spec.keep_spent {
                pr.pierce = 0.0;
            }
            let heading = if pr.spec.instant && pr.target.is_some() {
                $s.tracers.push((pr.origin, bp));
                let h = (bp[1] - pr.origin[1]).atan2(bp[0] - pr.origin[0]);
                pr.origin = bp;
                h
            } else {
                pr.vel[1].atan2(pr.vel[0])
            };
            let mut now_after = Vec::new();
            for c in &pr.spec.on_contact { // P-CONTACT
                if (c.on_damage && immune) || (c.on_pop && popped_now == 0) || (!c.on_damage && !c.on_pop && !c.after_damage) {
                    continue;
                }
                let skip = if c.pass_on_collided { Some(layer) } else { None };
                // EmitOnDamage.Collide (VERIFIED-DISASM @0xcbb8c0): emits at once from the
                // bloon, along the shot's own rotation, and puts the bloon in each new
                // projectile's collidedWith (recording sniper_025: Shrapnel fragments fan
                // round the shot's -68.67 and fly through their MOAB at pierce 2).
                if c.on_damage && !c.on_pop {
                    // (Made before the damage, above.)
                    continue;
                }
                if c.on_damage {
                    now_after.push((bp, c.clone(), Some(layer)));
                    continue;
                }
                if c.after_damage {
                    // Emission.BaseEmit (VERIFIED-DISASM @0xc3dce0) makes nothing once its
                    // entity is destroyed: an immune hit that destroyed the shot leaves no
                    // after-damage spawn (recording wizard_030: Fireballs on a Purple).
                    if immune_destroyed {
                        continue;
                    }
                    // CreateProjectileOnContact.Collide (VERIFIED-DISASM @0xca2a60) after the
                    // damage: on the bloon, or where the shot is with dontCreateAtBloon or
                    // once the hit destroyed it (isDestroyed; recording wizard_020: a
                    // Fireball popping a Red explodes where the fireball was).
                    // (An instant shot is on its Target: Instant.Process put it there this frame, before the
                    // collision; recording sniper_030 rt 2876, Shrapnel from a bounced bullet whose Blue
                    // it popped starts on the Blue's spot this frame, not the last.)
                    let at = if c.at_shot || !$s.bloons.iter().any(|b| b.id == id) { inst_pp } else { bp };
                    now_after.push((at, c.clone(), skip));
                    continue;
                }
                // (dontCreateAtBloon: where the shot is; recording tack_500, a meteor's blast.)
                let at = if c.at_shot { pp } else { bp };
                $contacts.push((at, heading, c.clone(), pr.tiers, pr.owner, skip, hit_z));
            }
            if !now_after.is_empty() {
                // (Projectile.EjectRotation: an instant shot's is its tower's rotation as
                // it fired.)
                let (tiers, owner) = (pr.tiers, pr.owner);
                let grot = pr.grot.or_else(|| $s.towers.iter().find(|t| t.id == owner).and_then(|t| t.game_rot).map(|r| { let d = game_dir(r); RAD2DEG * crt_atan2(d[0], -d[1]) }));
                for (at, c, skip) in now_after {
                    let start = $s.projectiles.len();
                    $s.emit_z = hit_z;
                    // ArcEmission.GetDirection (VERIFIED-DISASM @0x180c37590): the rotation is
                    // the owner tower's at that moment (its emission behaviours may change it),
                    // not the projectile's (recording bomb_032: a blast's frags from 34.864883,
                    // the tower turned on since the bomb's 34.864876). Other emissions keep the
                    // projectile's (Sniper's EmitOnDamage fans).
                    $s.emit_rot = if c.emission.kind == "Arc" && c.emission.dir_off.is_none() && !c.on_damage {
                        $s.towers.iter().find(|t| t.id == owner).and_then(|t| t.game_rot).or(grot)
                    } else {
                        grot
                    };
                    $s.emit(at, heading, &c.emission, &c.projectile, None, tiers, owner);
                    $s.emit_rot = None;
                    $s.emit_z = 0.0;
                    if let Some(id) = skip {
                        for pr in &mut $s.projectiles[start..] {
                            pr.hit.push(id);
                        }
                    }
                }
            }
            let pr = &mut $s.projectiles[$p];
            // RetargetOnContact.Collide (VERIFIED-DISASM @0xcf6ae0): with bounces left, a delayed
            // one notes the frame (Process retargets delayFrames later), else it retargets now.
            if let (Some(rt), true) = (pr.spec.retarget.clone(), pr.pierce > 0.0) {
                if pr.bounces < rt.max_bounces {
                    if ((rt.delay * FPS) as f64).round_ties_even() > 0.0 {
                        pr.retarget_at = Some($sim_now);
                    } else {
                        $s.retarget($p, &rt);
                    }
                }
            }
            // Storage only changes at or after `i`: a popped bloon is swap-removed (the last
            // bloon moves into slot i) and children are appended. So re-check slot i if its
            // occupant changed, else move on, instead of rescanning from 0 (quadratic in big
            // pop cascades). Each bloon is still checked once per projectile per frame.
            if !$s.bloons.get(i).is_some_and(|b| b.id == id) {
                // Popped: the old last bloon moved into slot i (children are appended after
                // it, already in this projectile's hit list).
                remap_moved(&mut $cand, k, i, len0);
            }
            k += 1;
        }
        // CollisionChecker.Process, after bloons: a projectile touching a MapBlocker taller
        // than it (VsBlockerRadius, from where it began the frame) is destroyed, unless its
        // model ignores blockers (VERIFIED by recording: darts flying into Monkey Meadow's
        // trees), when taller than it. A target-only shot is checked too
        // (recordings boomerang_500 / 510 / 520: Glaive Lord's MoabDot dropped by a tree's
        // edge lives 1 frame instead of 3).
        // An instant shot with pierce left too (recording sniper_205 rt 955, SimRecorder XS: a double
        // shot whose target its sibling popped sits on a blocker and goes; at a clear spot it lives
        // out its 6 frames).
        // Not the frame it is made: the next (sniper_205's 2633 at age 1; sniper_250 rt 2509, a
        // shot on a blocker after its emission's collision is still there that frame).
        let pr = &$s.projectiles[$p];
        // (A dontUseCollisionChecker projectile skips this too: recording bomb_005, a Bomb
        // Blitz Cluster fragment flies on past a tree.)
        if !pr.spec.ignore_blockers && !(pr.spec.no_checker && $only.is_none()) && pr.spec.blocker_rebound.is_none() && !(pr.spec.instant && pr.born >= $s.sim_elapsed * 4) && (pr.target.is_none() || pr.spec.only_target) && pr.pierce > 0.0 && pr.frames_left > 0.0 && $s.blocker_hit(pr.prev, pr.spec.vs_blocker_radius, pr.z) {
            // Projectile.CollideMap first: CreateProjectileOnContact.CollideMap (VERIFIED-DISASM
            // @0xcc1150) makes its projectile where the shot is (recording glue_500: a Bloon
            // Solver glob into the map's edge leaves its splat there).
            let pr = &$s.projectiles[$p];
            let heading = pr.vel[1].atan2(pr.vel[0]);
            for c in pr.spec.on_contact.iter().filter(|c| !c.on_damage && !c.on_pop) {
                $contacts.push((pr.prev, heading, c.clone(), pr.tiers, pr.owner, None, pr.z));
            }
            // CollisionChecker.Process (VERIFIED-DISASM @0xf31590): CollideMap, then
            // RootObject.Destroy: no Expire, so no expire projectiles (recording sub_005: an
            // Airburst dart into a log makes no splits, and no ids go to them).
            $s.projectiles.remove($p);
            continue;
        }
        // HYPOTHESIS P-ACIDPOOL: a splash that expires without hitting anything becomes a pool
        // (lifespanIfMisses, radiusIfMisses, pierce).
        let pr = &mut $s.projectiles[$p];
        if pr.frames_left <= 0.0 && pr.hit.is_empty() {
            if let Some(ap) = pr.spec.acid_pool.take() {
                pr.frames_left = ap.lifespan * FPS;
                pr.spec.radius = ap.radius;
                pr.pierce = ap.pierce;
            }
        }
        // CreateProjectileOnExhaustFraction.Process (VERIFIED-DISASM @0xcc2a00), after this
        // frame's collisions and its move: once (isDone) the pierce used (1 - pierce / the
        // model's) reaches `fraction`, or the age (INFERRED: frames since made) over the
        // lifespan frames reaches a `duration_fraction` >= 0, it makes its projectiles where
        // the shot is (recording boomerang_500: Glaive Lord's MoabDot on a glaive's first
        // hit). With no round on and canCreateInBetweenRounds off it is done with nothing.
        {
            let pr = &mut $s.projectiles[$p];
            for (k, ex) in pr.spec.on_exhaust.iter().enumerate() {
                if pr.exhaust_done & (1 << k) != 0 || ex.expire {
                    continue;
                }
                let used = 1.0 - pr.pierce / pr.spec.pierce;
                // Projectile.lifespan: the AgeModel's lifespanFrames (Age.UpdatedModel), else
                // int.MaxValue (Projectile.Initialise).
                let life = match pr.spec.age_lifespan.map(|l| l * FPS) { Some(f) if f.is_finite() => (f as f64).round_ties_even() as f32, _ => i32::MAX as f32 };
                let aged = pr.age / life;
                if used < ex.fraction && (aged < ex.duration_fraction || ex.duration_fraction < 0.0) {
                    continue;
                }
                pr.exhaust_done |= 1 << k;
                if !($s.sandbox || $s.round_active || !$s.bloons.is_empty()) && !ex.between_rounds {
                    continue;
                }
                let mut ex = ex.clone();
                if let Some([interval, pierce, radius]) = ex.per_distance.filter(|d| d[0] > 0.0) {
                    let n = ((pr.pos[0] - pr.origin[0]).hypot(pr.pos[1] - pr.origin[1]) / interval).floor();
                    ex.projectile.pierce += n * pierce;
                    ex.projectile.radius += n * radius;
                }
                // (Out of pierce, it was exhausted in Projectile.CollideBloon, before this
                // frame's move: recording boomerang_paragon_r63, a glaive on a Lead leaves
                // its MoabDot where it was.)
                let at = if pr.pierce <= 0.0 { pr.prev } else { pr.pos };
                $exhausted.push((at, pr.vel[1].atan2(pr.vel[0]), ex, pr.tiers, pr.owner, pr.aim.or(pr.target), (pr.grot, pr.z, pr.dir_rot()), pr.pierce <= 0.0));
            }
        }
        if std::mem::take(&mut $s.projectiles[$p].refresh_due) {
            let full = $s.projectiles[$p].spec.pierce;
            $s.projectiles[$p].pierce = full;
        }
        let pr = &$s.projectiles[$p];
        // An instant shot whose expire projectile goes where its living target is expires
        // after the bloons have moved (below).
        let later = pr.spec.instant && pr.pierce > 0.0 && pr.spec.on_exhaust.iter().any(|e| e.use_target) && pr.target.is_some_and(|id| $s.bloons.iter().any(|b| b.id == id));
        // (ignorePierceExhaustion: spent, it stays; recording wizard_040, a Wall of Fire at 0
        // from rt 231 refreshes at 234.)
        if (pr.pierce <= 0.0 && !pr.spec.keep_spent) || (pr.frames_left <= 0.0 && !later) {
            let pr = $s.projectiles.remove($p);
            let (mut ex, sub) = $s.retire_projectile(&pr);
            $exhausted.append(&mut ex);
            $subs.extend(sub);
        } else {
            $p += 1;
        }
    }
    };
}

impl Game {
    pub fn new(mut d: GameData) -> Self {
        // Tracks: the active exported paths (or the single "path"), their splitter routing,
        // and the spawn paths: no entry splitter, not the boss path (STAND-IN P-BOSSPATH: by
        // name).
        let mut tracks: Vec<crate::data::MapPath> = d.map.paths.iter().filter(|p| p.active && p.points.len() > 1).cloned().collect();
        if tracks.is_empty() {
            tracks.push(crate::data::MapPath { name: String::new(), active: true, id: 0, entry: Vec::new(), exit: Vec::new(), exit_splitter: 0, entry_splitter: 0, points: d.map.path.clone(), dists: Vec::new(), speeds: Vec::new(), quick: Vec::new(), quick_rot: Vec::new(), no_target: Vec::new() });
        }
        // Reverse mode (ReverseModel): bloons run every track end to start, so each track is
        // flipped and its splitters swap roles; the old end tracks become the spawn tracks.
        if d.modes.get(&d.mode).is_some_and(|m| m.reverse) {
            for t in &mut tracks {
                t.points.reverse();
                std::mem::swap(&mut t.entry, &mut t.exit);
                std::mem::swap(&mut t.entry_splitter, &mut t.exit_splitter);
            }
        }
        let mut splitter_ids: Vec<i64> = Vec::new();
        // Splitters by id (one Splitter.Rout counter each), and each track's routes through them.
        let mut route = |ids: &[i64], splitter: i64| -> (Vec<usize>, usize) {
            let to: Vec<usize> = ids.iter().filter_map(|id| tracks.iter().position(|q| q.id == *id)).collect();
            let s = match splitter_ids.iter().position(|x| *x == splitter) {
                Some(i) => i,
                None => {
                    splitter_ids.push(splitter);
                    splitter_ids.len() - 1
                }
            };
            (to, s)
        };
        let track_exit: Vec<(Vec<usize>, usize)> = tracks.iter().map(|p| route(&p.exit, p.exit_splitter)).collect();
        let track_entry: Vec<(Vec<usize>, usize)> = tracks.iter().map(|p| route(&p.entry, p.entry_splitter)).collect();
        let n_splitters = splitter_ids.len().max(1);
        let mut spawn_tracks: Vec<usize> = (0..tracks.len()).filter(|&i| tracks[i].entry.is_empty() && !tracks[i].name.contains("Boss")).collect();
        if spawn_tracks.is_empty() {
            spawn_tracks.push(0);
        }
        // Reverse mode uses the spawner's reverse splitter, which isn't exported.
        let reversed = d.modes.get(&d.mode).is_some_and(|m| m.reverse);
        let spawn_route = d.map.spawner.as_ref().filter(|_| !reversed).and_then(|s| {
            crate::map_routes::SpawnRoute::new(s, |id| tracks.iter().position(|t| t.id == id))
        });
        let logic = map_logic::LogicState::new(&d.map, tracks.clone(), splitter_ids.clone(), reversed);
        // The chosen difficulty, else Medium, else the old single-mode start values.
        let mode = d.modes.get(&d.mode).or(d.modes.get("Medium")).cloned().unwrap_or(crate::data::Mode {
            cash: d.start.cash,
            lives: d.start.lives,
            sell: SELL_MULTIPLIER,
            cost_mult: 1.0,
            speed_mult: 1.0,
            speed_add: 0.0,
            start_round: 1,
            end_round: d.end_round.unwrap_or(usize::MAX),
            bonus: Some([100.0, 1.0]),
            locked: Vec::new(),
            no_sell: false,
            health: Vec::new(),
            difficulty: None,
            sub: None,
            locked_sets: Vec::new(),
            cash_mult: 1.0,
            moab_health: 1.0,
            round_set: None,
            sandbox: false,
            reverse: false,
            apopalypse: false,
            deflation: false,
        });
        // BloonHealthModel (Double HP MOABs): maxHealth x healthMod on every "Moabs" bloon.
        if mode.moab_health != 1.0 {
            for b in d.bloons.values_mut() {
                if b.tags.iter().any(|t| t == "Moabs") {
                    b.max_health = (b.max_health as f32 * mode.moab_health) as i32;
                }
            }
        }
        // RoundSetModModel (Alternate Bloons Rounds).
        if let Some(r) = mode.round_set.as_ref().and_then(|n| d.round_sets.remove(n)) {
            d.rounds = r;
        }
        // GlobalSpeedModModel.ModBloon: speed x multiplier + addition on every bloon (verified).
        for b in d.bloons.values_mut() {
            b.speed = b.speed * mode.speed_mult + mode.speed_add;
        }
        let mut names: Vec<String> = d.bloons.keys().cloned().collect();
        names.sort();
        let by_name: HashMap<String, usize> = names.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect();
        let defs: Vec<BloonDef> = names.iter().map(|n| d.bloons[n].clone()).collect();
        let max_bloon_radius = defs.iter().map(|d| d.radius).fold(0.0, f32::max);
        let max_bloon_step = defs.iter().map(|d| d.speed).fold(0.0, f32::max) * 2.0 / FPS + 1.0;
        let n_defs = defs.len();
        let mut pops_in = vec![0u64; defs.len()];
        fn count(k: usize, defs: &[BloonDef], by: &HashMap<String, usize>, memo: &mut Vec<u64>) -> u64 {
            if memo[k] == 0 {
                memo[k] = 1 + defs[k].children.iter().filter_map(|c| by.get(c)).map(|&c| count(c, defs, by, memo)).sum::<u64>();
            }
            memo[k]
        }
        for k in 0..defs.len() {
            count(k, &defs, &by_name, &mut pops_in);
        }
        let cash_in: Vec<f64> = pops_in.iter().map(|&n| n as f64).collect();
        let child_kinds: Vec<Vec<usize>> = defs.iter().map(|d| d.children.iter().filter_map(|c| by_name.get(c).copied()).collect()).collect();
        let grow_kinds: Vec<Option<(usize, u32)>> =
            defs.iter().map(|d| d.grow.as_ref().and_then(|g| Some((*by_name.get(&g.to)?, (g.rate * 60.0) as u32)))).collect();
        let mut rbe = vec![-1.0; defs.len()];
        fn rbe_of(i: usize, defs: &[BloonDef], by: &HashMap<String, usize>, memo: &mut Vec<f32>) -> f32 {
            if memo[i] >= 0.0 {
                return memo[i];
            }
            let mut r = defs[i].leak_damage;
            for c in &defs[i].children {
                if let Some(&k) = by.get(c) {
                    r += rbe_of(k, defs, by, memo);
                }
            }
            memo[i] = r;
            r
        }
        for i in 0..defs.len() {
            rbe_of(i, &defs, &by_name, &mut rbe);
        }
        // Power towers the clone simulates (powers.rs): extra defs after the shop's, never in it.
        let mut power_tower_names: HashMap<String, String> = HashMap::new();
        for (pid, pt) in std::mem::take(&mut d.power_towers) {
            if !pt.unmodelled.is_empty() {
                continue;
            }
            let name = pt.name.clone();
            d.tower_tiers.entry(name.clone()).or_insert(pt.tiers);
            d.towers.entry(name.clone()).or_insert(pt.tower);
            power_tower_names.insert(pid, name);
        }
        let mut tower_defs: Vec<(String, TowerDef)> = d.towers.into_iter().collect();
        let shop_pos = |n: &str| d.shop.iter().position(|s| s.id == n).unwrap_or(usize::MAX);
        tower_defs.sort_by(|a, b| shop_pos(&a.0).cmp(&shop_pos(&b.0)).then(a.0.cmp(&b.0)));
        let tower_tiers = tower_defs.iter().map(|(n, _)| d.tower_tiers.get(n).cloned()).collect();
        let mut game=Self {
            mod_runtime:crate::mod_runtime::Runtime::new(std::mem::take(&mut d.mod_scripts)),
            base_defs: defs.iter().map(|d| (d.speed, d.max_health)).collect(),
            defs,
            names,
            max_bloon_radius,
            damage_tally: 0,
            destroyed_tally: 0,
            last_destroyed: 0,
            has_grid_rank: !d.grid_offsets.is_empty(),
            grid_offsets: d.grid_offsets.clone(),
            mutator_data: d.bloon_mutator_data.clone(),
            ggrid: GameGrid::default(),
            grid_rank: {
                let mut r = vec![u16::MAX; 129 * 129];
                for (k, w) in d.grid_offsets.chunks(2).enumerate() {
                    if let [dx, dy] = w {
                        if dx.abs() <= 64 && dy.abs() <= 64 {
                            r[((dx + 64) * 129 + (dy + 64)) as usize] = k.min(u16::MAX as usize - 1) as u16;
                        }
                    }
                }
                r
            },
            max_bloon_step,
            pops_in,
            child_kinds,
            grow_kinds,
            grid: BloonGrid::default(),
            grid_on: false,
            grid_cand: Vec::new(),
            emit_rot: None,
            emit_dir_rot: None,
            emit_z: 0.0,
            emit_target: None,
            in_instant: false,
            path_lists: Vec::new(),
            path_rank: Default::default(),
            plist_mark: Vec::new(),
            fact: Vec::new(),
            fact_at: Vec::new(),
            pool: Vec::new(),
            fact_mark: 0,
            spawn_parent_obj: None,
            died_after_fill: Vec::new(),
            ghosts_src: Vec::new(),
            fill_first_id: None,
            ghost_cell: None,
            grid_rank_pre: None,
            proj_list: Vec::new(),
            pop_depth: 0,
            pop_nested: false,
            pop_first_id: 0,
            step_first_id: 0,
            phase: 0,
            in_step: false,
            instant_out: Default::default(),
            sim_elapsed: 0,
            by_name,
            rbe,
            layer_offset: vec![0; n_defs],
            extra_cash: vec![0.0; n_defs],
            cash_in,
            pre_super: None,
            tower_defs,
            tower_tiers,
            track_ends: tracks
                .iter()
                .map(|p| {
                    let t = Track::new(p.points.clone());
                    let (spawn, leak) = t.entry_exit();
                    (spawn.filter(|_| p.entry.is_empty()), leak.filter(|_| p.exit.is_empty()))
                })
                .collect(),
            track_segs: tracks.iter().map(|p| Track::with_info(p.points.clone(), &p.dists, &p.speeds, &p.quick).segments()).collect(),
            track_rot: tracks.iter().map(|p| p.quick_rot.clone()).collect(),
            track_seg_ok: tracks
                .iter()
                .map(|p| {
                    let tr = Track::with_info(p.points.clone(), &p.dists, &p.speeds, &p.quick);
                    let len = tr.length();
                    let n = tr.segments().len();
                    (0..n)
                        .map(|i| {
                            let lo = i as f32 * 7.0;
                            let centre = lo + ((len.min(lo + 7.0)) - lo) * 0.5;
                            if p.no_target.len() == p.dists.len() && !p.dists.is_empty() {
                                // The point the centre falls on (last point at or before it).
                                let k = p.dists.iter().rposition(|&d| d <= centre).unwrap_or(0);
                                !p.no_target[k]
                            } else {
                                on_map(tr.pos(centre), 0.0)
                            }
                        })
                        .collect()
                })
                .collect(),
            tracks: tracks.iter().map(|p| Track::with_info(p.points.clone(), &p.dists, &p.speeds, &p.quick)).collect(),
            track_exit,
            track_entry,
            spawn_tracks,
            spawn_route,
            logic,
            routs: vec![0; n_splitters],
            track: match tracks.first() {
                Some(p) if p.points == d.map.path => Track::with_info(p.points.clone(), &p.dists, &p.speeds, &p.quick),
                _ => Track::new(d.map.path),
            },
            blockers: d.map.areas.iter().flat_map(|a| a.blockers.iter().map(move |&c| (c, a.is_disabled))).collect(),
            blocker_grid: blocker_grid(&d.map.areas),
            areas: {
                let base = Area {
                    id: 0,
                    kind: AREA_LAND,
                    points: vec![[-WORLD_X, -WORLD_Z], [WORLD_X, -WORLD_Z], [WORLD_X, WORLD_Z], [-WORLD_X, WORLD_Z]],
                    height: 0.0,
                    blockers: Vec::new(),
                    is_disabled: false,
                    is_blocker: false,
                    holes: Vec::new(),
                };
                // A disabled area isn't on the map until a CreateAreaAction makes it.
                std::iter::once(base).chain(d.map.areas.into_iter().filter(|a| !a.is_disabled)).collect()
            },
            // Without an exported terrain extent: the game's world bounds.
            bounds: d.map.terrain.as_ref().map_or(([-WORLD_X, -WORLD_Z], [WORLD_X, WORLD_Z]), |t| (t.min, t.max)),
            has_terrain: d.map.terrain.is_some(),
            leak_immune: 0.0,
            end_round: mode.end_round.min(d.rounds.len()),
            map_difficulty: d.map_difficulty,
            xp_account: 0.0,
            xp_towers: Vec::new(),
            won: false,
            freeplay: false,
            rounds: d.rounds,
            apopalypse: mode.apopalypse && !d.freeplay.is_empty(),
            geraldo_items: d.geraldo,
            power_projectiles: d.power_projectiles,
            power_tower_defs: HashMap::new(),
            geraldo_stock: Vec::new(),
            freeplay_groups: d.freeplay,
            // FreeplayRoundManager.initialSeed: the game's per-game random seed.
            freeplay_seed: (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64) % 2147483646) as i32 + 1,
            // LockTowerSetModModel (Primary / Military / Magic Only) and LockTowerModModel
            // (CHIMPS / Deflation: Banana Farm): those towers aren't in the shop at all.
            shop: d
                .shop
                .iter()
                .filter(|it| it.hero || (!mode.locked_sets.contains(&it.set) && !mode.locked.contains(&it.id)))
                .cloned()
                .collect(),
            heroes: d.heroes,
            round_scale: d.round_scale,
            bloons: Vec::with_capacity(4096),
            towers: Vec::new(),
            projectiles: Vec::with_capacity(1024),
            pending: Vec::new(),
            round: mode.start_round.saturating_sub(1),
            round_active: false,
            round_frame: 0,
            sandbox_queue: Vec::new(),
            outside_track: Vec::new(),
            steps: 0,
            round_clock: 0,
            pop_after_move: false,
            proj_cursor: usize::MAX,
            distribute: (true, false),
            next_uid: 0,
            active_rbe: 0.0,
            cash: mode.cash,
            debt: 0.0,
            power_boosts: Vec::new(),
            thrives: Vec::new(),
            frame_spawns: Vec::new(),
            dart_time: None,
            debt_rate: 0.0,
            lives: mode.lives,
            start_lives: mode.lives,
            danger: 0.0,
            leaked: false,
            popped: 0,
            pops: Vec::new(),
            bursts: Vec::new(),
            tracers: Vec::new(),
            beams: Vec::new(),
            bolts: Vec::new(),
            hit_texts: Vec::new(),
            cash_pops: Vec::new(),
            upgrade_fx: Vec::new(),
            sacrifices: Vec::new(),
            place_fx: Vec::new(),
            effect_seq: 0,
            voice_warned: Vec::new(),
            free_tower_round: None,
            pop_emits: Vec::new(),
            prof: [0.0; 3],
            syphon: 0.0,
            paragon_degree: d.paragon_degree.clone(),
            cost_mult: mode.cost_mult,
            sell_mult: mode.sell,
            round_bonus: mode.bonus,
            // LockTowerModModel ids, plus every tower of a LockTowerSetModModel set.
            locked: {
                let mut l = mode.locked.clone();
                for (name, tt) in &d.tower_tiers {
                    let set = tt.tiers.get("000").map_or(0, |t| t.set);
                    if mode.locked_sets.contains(&set) {
                        l.push(name.clone());
                    }
                }
                l
            },
            cash_mult: mode.cash_mult,
            sandbox: mode.sandbox,
            sandbox_health_timer: -1,
            instant_hits: Vec::new(),
            last_set_health: mode.lives,
            lives_at_step: 0.0,
            no_sell: mode.no_sell,
            health_mods: mode.health.clone(),
            mode_name: if d.modes.contains_key(&d.mode) { d.mode.clone() } else { "Medium".into() },
            difficulty: mode.difficulty.clone().unwrap_or_else(|| "Medium".into()),
            stats: SessionStats::default(),
            leaks: Vec::new(),
            start_round: mode.start_round.saturating_sub(1),
            prof_max: [0.0; 3],
            #[cfg(test)]
            perf_detail: [0.0; 3],
            #[cfg(test)]
            reference_targeting: false,
            target_index: None,
            buffs: Vec::new(),
            sfx: Vec::new(),
            sfx_rng: 0x2545_F491_4F6C_DD1D,
            synced: None,
            pointer: None,
            coop_pointers: HashMap::new(),
            coop_wallets: None,
            coop_view: None,
            coop_areas: None,
            item_buyers: std::collections::BTreeMap::new(),
            removed: Vec::new(),
            typed_made: 0,
            rng: 0x9E37_79B9_7F4A_7C15,
            next_id: 1,
            next_layer: 0,
            game_over: false,
        };
        game.power_tower_defs = power_tower_names.into_iter().filter_map(|(pid, name)| Some((pid, game.tower_defs.iter().position(|(n, _)| *n == name)?))).collect();
        game.logic_start();
        game.mod_event("on_new_game",None);
        game
    }

    /// Rounds in this game: the mode's end round, or the whole round set in freeplay.
    pub fn total_rounds(&self) -> usize {
        match (self.freeplay, self.freeplay_groups.is_empty()) {
            (true, false) => usize::MAX,
            (true, true) => self.rounds.len(),
            _ => self.end_round,
        }
    }

    /// ProgressiveDifficultyManager.ChangeSpeedAndHealth (VERIFIED-DISASM @0xfc4c50, tables
    /// from its .cctor), applied to the original bloon models as each round starts:
    /// every non-Boss bloon's speed x GetSpeedMultiplier(round) and every MOAB-class bloon's
    /// max health x GetHealthMultiplier(round) (Mathf.Round). HYPOTHESIS P-FIRSTSPAWN: the
    /// first-spawn multiplier it takes the larger of isn't modelled (taken as 1).
    fn progressive_difficulty(&mut self, round: i32) {
        // CheckRound / UpdateDifficulty (VERIFIED-DISASM @0xfc5170 / @0xfc5ca0): from the 81st
        // round (Spawner.spawnedRound 80) the Ceramics and children change once; before it the
        // original models are put back (sandbox round changes).
        if round >= 81 && self.pre_super.is_none() {
            self.change_ceramics_and_children();
        } else if round < 81 && self.pre_super.is_some() {
            self.revert_ceramics_and_children();
        }
        let (speed, health) = (progressive_speed(round), progressive_health(round));
        for (d, &(s0, h0)) in self.defs.iter_mut().zip(&self.base_defs) {
            if !d.tags.iter().any(|t| t == "Boss") {
                d.speed = s0 * speed;
            }
            if d.tags.iter().any(|t| t == "Moabs") {
                d.max_health = round_half_even(h0 as f32 * health) as i32;
            }
        }
    }

    /// BadImmunity.FilterMutation (VERIFIED-DISASM @0xffcf40): a BAD (BadImmunityModel) takes no
    /// mutator whose id starts with one of badImmunityIds or contains one of badImmunityTags
    /// (Slow, Stun, Knockback, Glue, ...; recording boomerang_paragon_r98: the paragon's MOAB
    /// Press doesn't stun a BAD).
    fn bad_immune_to(&self, kind: usize, id: &str) -> bool {
        self.defs[kind].bad_immunity
            && (self.mutator_data.bad_immunity_ids.iter().any(|p| id.starts_with(p.as_str())) || self.mutator_data.bad_immunity_tags.iter().any(|t| id.contains(t.as_str())))
    }

    /// BloonModel.GetTotalLeakDamage over the current models: its leak damage + its children's.
    fn total_leak(&self, k: usize) -> f32 {
        self.defs[k].leak_damage + self.child_kinds[k].iter().map(|&c| self.total_leak(c)).sum::<f32>()
    }

    /// ProgressiveDifficultyManager.ChangeCeramicsAndChildren (VERIFIED-DISASM @0xfc43d0). Over
    /// GameModel.bloons in order, every bloon that isn't MOAB-class or isFixedMaxHealth:
    /// - with two or more children keeps only the first; its layerCountOffset gains each
    ///   dropped child's layer count (BloonDegradeStepper.GetNonRegrowLayers, counted on the
    ///   models before any change), so its pops still count them;
    /// - a Ceramic also gets the original maxHealth x 6, the Super tag, DistributeCashModel
    ///   additionalCash = cash x 86 + additionalCash, and leakDamage + (total leak damage before
    ///   - after). GetTotalLeakDamage (@0x1154740) caches each model's total (totalLeakDamage,
    ///   a nullable) and only the Ceramic's is reset (ResetTotalLeakDamage) before "after", so
    ///   both use the children's totals from before any change: leakDamage 10 + 47. Every cache
    ///   is reset afterwards (BloonsChangedForModel), and with its Rainbow down to 8 a Super
    ///   Ceramic leaks 65 (fortified 75). INFERRED: the caches are already filled by then (the
    ///   wiki's 65 / 75 agree; an empty cache would take the Black, changed earlier, as 6).
    fn change_ceramics_and_children(&mut self) {
        let n = self.defs.len();
        self.pre_super = Some(self.defs.iter().map(|d| (d.children.clone(), d.leak_damage, d.max_health, d.tags.clone())).collect());
        let mut layers = vec![0u64; n];
        fn count(k: usize, kids: &[Vec<usize>], memo: &mut [u64]) -> u64 {
            if memo[k] == 0 {
                memo[k] = 1 + kids[k].iter().map(|&c| count(c, kids, memo)).sum::<u64>();
            }
            memo[k]
        }
        for k in 0..n {
            count(k, &self.child_kinds, &mut layers);
        }
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&k| self.defs[k].index);
        for k in order {
            let tags = &self.defs[k].tags;
            if tags.iter().any(|t| t == "Moabs") || self.defs[k].is_fixed_max_health {
                continue;
            }
            let ceramic = tags.iter().any(|t| t == "Ceramic");
            // (self.rbe still holds the totals from before the change: the caches.)
            let before = self.rbe[k];
            if self.child_kinds[k].len() >= 2 {
                self.layer_offset[k] += self.child_kinds[k][1..].iter().map(|&c| layers[c]).sum::<u64>();
                self.child_kinds[k].truncate(1);
                self.defs[k].children.truncate(1);
            }
            if ceramic {
                self.defs[k].max_health = self.base_defs[k].1 * 6;
                self.defs[k].tags.push("Super".into());
                // (Every Ceramic's DistributeCashModel is cash 1, additionalCash 0.)
                self.extra_cash[k] = 86.0;
                let after = self.defs[k].leak_damage + self.child_kinds[k].iter().map(|&c| self.rbe[c]).sum::<f32>();
                self.defs[k].leak_damage += before - after;
            }
        }
        self.refresh_bloon_totals();
    }

    /// Revert / CheckRound's re-enable: the original bloon models again.
    fn revert_ceramics_and_children(&mut self) {
        let Some(pre) = self.pre_super.take() else { return };
        for (k, (children, leak, health, tags)) in pre.into_iter().enumerate() {
            self.child_kinds[k] = children.iter().filter_map(|c| self.by_name.get(c).copied()).collect();
            let d = &mut self.defs[k];
            (d.children, d.leak_damage, d.max_health, d.tags) = (children, leak, health, tags);
        }
        self.layer_offset.iter_mut().for_each(|o| *o = 0);
        self.extra_cash.iter_mut().for_each(|c| *c = 0.0);
        self.refresh_bloon_totals();
    }

    /// Totals worked out from the bloon models: total leak damage, pops and cash when destroyed
    /// whole (each layer counts 1 + layerCountOffset and pays $1 + additionalCash).
    fn refresh_bloon_totals(&mut self) {
        let n = self.defs.len();
        self.rbe = (0..n).map(|k| self.total_leak(k)).collect();
        fn sum<T: Copy + std::iter::Sum + std::ops::Add<Output = T>>(k: usize, kids: &[Vec<usize>], own: &dyn Fn(usize) -> T) -> T {
            own(k) + kids[k].iter().map(|&c| sum(c, kids, own)).sum::<T>()
        }
        self.pops_in = (0..n).map(|k| sum(k, &self.child_kinds, &|j| 1 + self.layer_offset[j])).collect();
        self.cash_in = (0..n).map(|k| sum(k, &self.child_kinds, &|j| 1.0 + self.extra_cash[j])).collect();
    }

    /// Round `round` (1-based) past the round set: FreeplayRoundManager.GetRoundEmissions
    /// (VERIFIED-DISASM @0xf841a0). Seeded with initialSeed + round, the group list is shuffled
    /// (LinqExtensions.ShuffleInPlace: i from 0, swap with Range(i, n)), then a budget is drawn:
    /// rounds 1-50  (5e-11 r^7.7 + r^1.75 + 20) / 160 (400 - 3r)(1 + r/100) 0.6,
    /// rounds 51-100  5e-11 r^7.7 + r^1.75 + 20,  past 100  4000 r - 225000,
    /// times (1.5 - NextFloat). Each shuffled group allowed this round (inside one of its
    /// bounds) whose score fits what's left is added, starting where the last one ended, and
    /// its score spent.
    pub fn freeplay_round(&self, round: i32) -> Vec<crate::data::Group> {
        let mut rng = NkRandom((self.freeplay_seed.wrapping_add(round)) as i64);
        let n = self.freeplay_groups.len() as i32;
        let mut order: Vec<usize> = (0..n as usize).collect();
        for i in 0..n {
            let j = rng.range(i, n);
            order.swap(i as usize, j as usize);
        }
        let mult = rng.next_float();
        let r = round as f64;
        let curve = || 5e-11 * (round as f32).powf(7.7) as f64 + (round as f32).powf(1.75) as f64 + 20.0;
        let mut budget = if round > 100 {
            round as f64 * 4000.0 - 225000.0
        } else if round > 50 {
            curve()
        } else {
            curve() / 160.0 * (400 - 3 * round) as f64 * (r * 0.01 + 1.0) * 0.6
        };
        if round > 1 {
            budget -= (mult as f64 - 0.5) * budget;
        }
        let mut out = Vec::new();
        let mut offset = 0.0f32;
        for &k in &order {
            let g = &self.freeplay_groups[k];
            if !g.bounds.iter().any(|b| b[0] <= round && round <= b[1]) || g.score > budget {
                continue;
            }
            out.push(crate::data::Group { bloon: g.bloon.clone(), count: g.count, start: g.start + offset, duration: g.duration });
            // The next group starts at this one's last emission.
            offset += g.start + if g.count == 1 { g.duration * 0.5 } else { g.duration };
            budget -= g.score;
        }
        out
    }

    /// Victory screen's Freeplay: keep playing past the mode's last round.
    pub fn start_freeplay(&mut self) {
        self.freeplay = true;
        self.won = false;
    }

    /// Read-only render preparation: includes descendants and every damage state,
    /// without emitting bloons, drawing RNG values or generating freeplay rounds.
    pub fn upcoming_bloon_models(&self)->Vec<String>{
        let mut seen=vec![false;self.defs.len()];
        let mut todo:Vec<usize>=self.pending.iter().map(|p|p.1).chain(self.bloons.iter().map(|b|b.kind)).collect();
        if !self.round_active{
            todo.extend(self.rounds.get(self.round).into_iter().flatten().filter_map(|g|self.by_name.get(&g.bloon).copied()));
        }
        let mut models=Vec::new();
        while let Some(kind)=todo.pop(){
            if seen[kind]{continue;}seen[kind]=true;
            let def=&self.defs[kind];
            models.extend(def.model.iter().cloned());
            models.extend(def.damage_models.iter().map(|d|d.model.clone()));
            todo.extend(self.child_kinds[kind].iter().copied());
            if let Some(kind)=def.grow.as_ref().and_then(|g|self.by_name.get(&g.to)){todo.push(*kind);}
        }
        models.sort();models.dedup();models
    }

    pub fn start_round(&mut self) {
        if self.round_active || self.game_over || self.won || self.round >= self.total_rounds() {
            return;
        }
        self.voice_warned.clear();
        self.thrive_round_start();
        // Ability.OnRoundStartSim (VERIFIED-DISASM @0x180dfacd0): activationsThisRound = 0.
        for tw in &mut self.towers {
            tw.ability_round.clear();
        }
        for tw in &mut self.towers {
            tw.comanche.1 = 0;
        }
        self.pending.clear();
        self.progressive_difficulty(self.round as i32 + 1);
        let groups = match self.rounds.get(self.round).filter(|_| !self.apopalypse) {
            Some(r) => r.clone(),
            None => self.freeplay_round(self.round as i32 + 1),
        };
        for g in &groups {
            let Some(&k) = self.by_name.get(&g.bloon) else { continue };
            // VERIFIED-DISASM (RoundData.GetRoundDef + BloonGroupModel.GetEmissions).
            let end_s = g.duration + g.start;
            let (start, end) = ((self.round_scale * g.start) * FPS, (self.round_scale * end_s) * FPS);
            let span = end - start;
            let (step, offset) = if g.count == 1 { (span, span * 0.5) } else { (span / (g.count.max(2) - 1) as f32, 0.0) };
            for i in 0..g.count {
                let t = (i as f32 * step + start) + offset;
                // Spawner.AddEmissions numbers the emissions in list order (emissionIndex).
                let idx = self.pending.len() as u32;
                self.pending.push((t.ceil().max(0.0) as u32, k, idx));
            }
        }
        // Popped from the end: earliest frame first, and within a frame in list order (as
        // Spawner.Process scans its list).
        self.pending.sort_by_key(|p| std::cmp::Reverse((p.0, p.2)));
        self.round_frame = 0;
        self.round_active = true;
        // Weapon.RoundStart (VERIFIED-DISASM @0xc703a0): missedRate = 0 (recording dart_040_club:
        // a Fan Club weapon's fraction gone as round 20 starts).
        for tw in &mut self.towers {
            tw.weapons.iter_mut().for_each(|w| {
                w.round_shots = 0;
                w.missed = 0.0;
            });
        }
        self.logic_round_start(self.start_round as i32);
        self.mod_event("on_round_start",None);
    }

    /// Damage Avatar of Wrath's bolts add now (its tier's first Wrath modifier), and the tier's
    /// DruidVengeanceEffectModel level for it.
    pub fn wrath_level(&self, t: usize) -> Option<(usize, &crate::data::Vengeance)> {
        let tier = self.tier(t)?;
        let v = tier.vengeance.as_ref()?;
        let w = tier.attacks.iter().flat_map(|a| &a.weapons).find_map(|w| w.projectile.wrath.as_ref())?;
        Some((v.level((self.active_rbe / w.threshold * w.damage).clamp(0.0, w.max)), v))
    }

    /// The icon sprite ("Bloons/Red", "UiBloonIcons/MoabIcon") of the bloon type called `name`.
    pub fn custom_bloons(&self)->Vec<String>{
        self.names.iter().zip(&self.defs).filter(|(_,def)|def.modded).map(|(name,_)|name.clone()).collect()
    }
    pub(crate) fn mod_has_bloon(&self,name:&str)->bool{self.by_name.contains_key(name)}
    pub fn bloon_sprite(&self, name: &str) -> Option<&str> {
        self.by_name.get(name).and_then(|&k| self.defs[k].sprite.as_deref())
    }

    /// Sandbox bloon menu: `count` bloons of type `name` (with the camo / regrow / fortified
    /// variant if it exists, else the closest one that does), `spacing` seconds apart.
    pub fn sandbox_spawn(&mut self, name: &str, camo: bool, regrow: bool, fortified: bool, count: u32, spacing: f32) -> bool {
        // There is no plain DDT: it's always camo, in the sandbox's menu too.
        let camo = camo || name == "Ddt";
        // Variant names: <Base>[Regrow][Fortified][Camo] (CeramicRegrowFortifiedCamo).
        let mut tries = Vec::new();
        for (r, f, c) in [(regrow, fortified, camo), (false, fortified, camo), (regrow, false, camo), (regrow, fortified, false),
            (false, false, camo), (false, fortified, false), (regrow, false, false), (false, false, false)] {
            tries.push(format!("{name}{}{}{}", if r { "Regrow" } else { "" }, if f { "Fortified" } else { "" }, if c { "Camo" } else { "" }));
        }
        let Some(&k) = tries.iter().find_map(|n| self.by_name.get(n)) else { return false };
        let gap = (spacing.max(0.0) * FPS).round() as u64;
        for i in 0..count as u64 {
            self.sandbox_queue.push((self.steps + i * gap, k));
        }
        self.sandbox_queue.sort_by_key(|q| std::cmp::Reverse(q.0));
        true
    }

    /// Path.bloonsList.Add.
    fn plist_add(&mut self, id: u64, path: u8) {
        let p = path as usize;
        if self.path_lists.len() <= p {
            self.path_lists.resize_with(p + 1, Vec::new);
        }
        self.path_rank.insert(id, (path, self.path_lists[p].len() as u32));
        self.path_lists[p].push((id, true));
    }

    /// Moves just-added bloon `id` from the end of its path's list to the frame's mark.
    fn plist_move_to_mark(&mut self, id: u64, path: u8) {
        let p = path as usize;
        let Some(&mark) = self.plist_mark.get(p) else { return };
        let list = &mut self.path_lists[p];
        if list.last().map(|e| e.0) != Some(id) || mark >= list.len() - 1 {
            if let Some(m) = self.plist_mark.get_mut(p) {
                *m = (*m).max(list.len());
            }
            return;
        }
        let e = list.pop().unwrap();
        list.insert(mark, e);
        for (k, &(bid, _)) in list.iter().enumerate().skip(mark) {
            self.path_rank.insert(bid, (path, k as u32));
        }
        self.plist_mark[p] = mark + 1;
    }

    /// A bloon destroyed: it stays in its path's list, dead, until the next flush. A layer made
    /// and popped by the same hit was never in it.
    fn plist_kill(&mut self, id: u64) {
        if self.pop_depth > 0 && id >= self.pop_first_id {
            if let Some((p, k)) = self.path_rank.remove(&id) {
                let l = &mut self.path_lists[p as usize];
                l.remove(k as usize);
                for (n, e) in l.iter().enumerate().skip(k as usize) {
                    self.path_rank.insert(e.0, (p, n as u32));
                }
            }
            return;
        }
        if let Some(&(p, k)) = self.path_rank.get(&id) {
            if let Some(e) = self.path_lists.get_mut(p as usize).and_then(|l| l.get_mut(k as usize)) {
                e.1 = false;
            }
        }
    }

    /// Factory<Bloon>.Create: the object a new bloon lives in, the last one flushed into the pool
    /// if any (INFERRED from recording glue_400: a pop on a flush frame makes its third Green in
    /// a Red's object that died the frame before, and a linger in that Red's grid cell hits it).
    fn fact_create(&mut self) -> u32 {
        if let Some(o) = self.spawn_parent_obj.take() {
            if let Some(&k) = self.fact_at.get(o as usize).filter(|&&k| k != u32::MAX) {
                self.fact[k as usize].1 = true;
            }
            self.died_after_fill.retain(|d| d.0 != o);
            return o;
        }
        let o = self.pool.pop().unwrap_or_else(|| {
            self.fact_at.push(u32::MAX);
            (self.fact_at.len() - 1) as u32
        });
        self.fact_at[o as usize] = self.fact.len() as u32;
        self.fact.push((o, true));
        o
    }

    /// A bloon destroyed: its object stays listed, dead, until the next flush.
    fn fact_kill(&mut self, i: usize) {
        let b = &self.bloons[i];
        if let Some(&k) = self.fact_at.get(b.obj as usize).filter(|&&k| k != u32::MAX) {
            // HYPOTHESIS P-POOLBACK: a bloon made this frame and popped for good that frame (no children
            // left) gives its object back at once, unlisted (the recordings' H lines show id 0) (recordings sub_250 / sub_052: the
            // object a new bloon takes after the flush, seen through a Target that still holds it).
            if b.fresh && self.defs[b.kind].children.is_empty() {
                let o = b.obj;
                self.fact.remove(k as usize);
                for e in k as usize..self.fact.len() {
                    self.fact_at[self.fact[e].0 as usize] = e as u32;
                }
                self.fact_at[o as usize] = u32::MAX;
                self.pool.push(o);
                return;
            }
            self.fact[k as usize].1 = false;
        }
        // The collision grid is filled as the bloons start processing: one that was in it and
        // dies after stays in it.
        if !self.grid_on && self.fill_first_id.is_some_and(|f| b.id < f) {
            let at = if b.fresh { b.cell_pos } else { b.prev };
            self.died_after_fill.push((b.obj, b.id, at, b.order, self.defs[b.kind].radius));
        }
    }

    /// Factory.Flush: as the path lists (front to back, each hole taking the last entry), the
    /// dead objects going into the pool.
    fn fact_flush(&mut self) {
        let mut i = 0;
        while i < self.fact.len() {
            if self.fact[i].1 {
                i += 1;
                continue;
            }
            let (o, _) = self.fact.swap_remove(i);
            self.fact_at[o as usize] = u32::MAX;
            self.pool.push(o);
            if let Some(&(m, _)) = self.fact.get(i) {
                self.fact_at[m as usize] = i as u32;
            }
        }
    }

    /// The Spawner's bloon goes in the factory's list where the frame's process began.
    fn fact_move_to_mark(&mut self, obj: u32) {
        let mark = self.fact_mark;
        if self.fact.last().map(|e| e.0) != Some(obj) || mark >= self.fact.len() - 1 {
            self.fact_mark = mark.max(self.fact.len());
            return;
        }
        let e = self.fact.pop().unwrap();
        self.fact.insert(mark, e);
        for k in mark..self.fact.len() {
            self.fact_at[self.fact[k].0 as usize] = k as u32;
        }
        self.fact_mark = mark + 1;
    }

    /// FactoryFactory.Flush (Simulation.Simulate, every 60 frames; VERIFIED by recording): dead
    /// entries leave the lists, front to back, each hole taking the list's last entry.
    fn plist_flush(&mut self) {
        for (p, l) in self.path_lists.iter_mut().enumerate() {
            let mut i = 0;
            while i < l.len() {
                if l[i].1 {
                    i += 1;
                    continue;
                }
                self.path_rank.remove(&l[i].0);
                l.swap_remove(i);
                if let Some(e) = l.get(i) {
                    self.path_rank.insert(e.0, (p as u8, i as u32));
                }
            }
        }
    }

    /// The projectile list's flush (as `plist_flush`); live projectiles are then kept in its
    /// order, which is the order the collision checker and the process list go through them.
    fn proj_flush(&mut self) {
        let alive: FastSet<u64> = self.projectiles.iter().map(|p| p.uid).collect();
        let l = &mut self.proj_list;
        let mut i = 0;
        while i < l.len() {
            if alive.contains(&l[i]) {
                i += 1;
            } else {
                l.swap_remove(i);
            }
        }
        let at: FastMap<u64, usize> = l.iter().enumerate().map(|(k, &u)| (u, k)).collect();
        self.projectiles.sort_by_key(|p| at.get(&p.uid).copied().unwrap_or(usize::MAX));
    }

    /// Sandbox: every bloon gone (and the round's / sandbox's queued ones).
    pub fn clear_bloons(&mut self) {
        for e in &mut self.fact {
            e.1 = false;
        }
        self.bloons.clear();
        self.path_lists.clear();
        self.path_rank.clear();
        self.pending.clear();
        self.sandbox_queue.clear();
        self.outside_track.clear();
        if self.grid_on {
            self.grid = Default::default();
        }
    }

    /// Sandbox: every tower gone (no refund).
    pub fn clear_towers(&mut self) {
        while let Some(t) = self.towers.len().checked_sub(1) {
            self.towers.swap_remove(t);
            self.removed.push((t, t));
        }
    }

    /// Display heading of a TravelAlongPath projectile (the Necromancer's undead): the track's
    /// direction where it is, the way it runs. Its `vel` swings between frames (about -0.42 and
    /// +0.01 for an undead bloon), which flipped the sprite every frame.
    /// With disableRotateWithPathDirection (the undead) it keeps the rotation it was emitted
    /// with: the Necromancer's emits at game rotation 0, i.e. heading -90 degrees here (HYPOTHESIS
    /// P-UNDEADROT0; owner: the real undead look like upright bloons).
    pub fn track_heading(&self, p: &Projectile) -> Option<f32> {
        let (k, d) = p.on_track?;
        let tt = p.spec.track_travel.as_ref()?;
        if tt.fixed_rotation {
            return Some(-std::f32::consts::FRAC_PI_2);
        }
        let tr = self.tracks.get(k as usize)?;
        let s = if tt.reverse { -0.5 } else { 0.5 };
        let len = tr.length();
        let (a, b) = (tr.pos((d - s).clamp(0.0, len)), tr.pos((d + s).clamp(0.0, len)));
        Some((b[1] - a[1]).atan2(b[0] - a[0]))
    }

    /// Sandbox: every ability ready (each entry at 0, not removed: a missing entry is a newly
    /// gained ability, which Ability.Initialise starts at 33% cooldown).
    pub fn reset_cooldowns(&mut self) {
        for t in 0..self.towers.len() {
            let names: Vec<String> = self.tier(t).map(|tr| tr.abilities.iter().map(|a| a.name.clone()).collect()).unwrap_or_default();
            let cd = &mut self.towers[t].ability_cd;
            for v in cd.values_mut() {
                *v = 0.0;
            }
            for n in names {
                cd.insert(n, 0.0);
            }
        }
    }

    /// Sandbox: both displayed tower counters back to 0.
    pub fn reset_damage(&mut self) {
        for tw in &mut self.towers {
            tw.pops = 0;
            tw.damage = 0;
        }
    }

    /// Sandbox "Set Round": round `n` (1-based) starts now.
    pub fn set_round(&mut self, n: usize) {
        self.pending.clear();
        self.round_active = false;
        // TargetTrack.OnRoundSet: GeneratePoints.
        for tw in &mut self.towers {
            tw.necro_points = true;
        }
        // Generated freeplay rounds are not bounded by the exported fixed round set.
        // The generator and progressive difficulty take a one-based signed round.
        let last = if self.freeplay_groups.is_empty() { self.rounds.len().max(1) } else { i32::MAX as usize };
        let round=n.clamp(1,last);
        self.round = round - 1;
        if self.sandbox || round > self.end_round { self.start_freeplay(); }
        self.start_round();
    }

    /// Test/benchmark helper: spawn `count` bloons of type `name` spread along the first `spread` units.
    pub fn debug_spawn(&mut self, name: &str, count: usize, spread: f32) -> bool {
        let Some(&k) = self.by_name.get(name) else { return false };
        for i in 0..count {
            self.spawn(k, spread * i as f32 / count.max(1) as f32);
        }
        true
    }

    fn spawn(&mut self, kind: usize, dist: f32) -> u64 {
        self.spawn_on(kind, dist, 0)
    }

    /// A round / sandbox emission onto a spawn path.
    pub(crate) fn spawn_emitted(&mut self, kind: usize, path: u8) -> u64 {
        let id = self.spawn_on(kind, 0.0, path);
        if let Some(b) = self.bloons.last_mut() {
            b.fresh = true;
            b.prev = [f32::INFINITY; 2];
        }
        id
    }

    fn spawn_on(&mut self, kind: usize, dist: f32, path: u8) -> u64 {
        let dist = self.held_at_spawn(kind, path, dist);
        self.spawn_at(kind, dist, path)
    }

    /// Path.DistanceToPointInfoBloon's rotation: the quick point at (int)(dist / 0.01); None
    /// for a path without them.
    pub(crate) fn path_rot(&self, path: u8, dist: f32) -> Option<f32> {
        self.track_rot.get(path as usize).filter(|v| !v.is_empty()).map(|v| v[((dist / 0.01).max(0.0) as usize).min(v.len() - 1)])
    }

    /// A bloon at exactly `dist`. Children made by a pop (SpawnChildren) sit where they're spaced,
    /// even behind the entrance, until their first Bloon.Move (recording dart_400).
    fn spawn_at(&mut self, kind: usize, dist: f32, path: u8) -> u64 {
        #[cfg(test)] let _spawn_timer=sim_profile::Timer::new(4);
        let id = self.next_id;
        self.next_id += 1;
        let pos = self.tracks[path as usize].bloon_pos(dist);
        let track_mult = self.tracks[path as usize].speed_mult(dist);
        let grow_t = self.grow_kinds[kind].map_or(0, |g| g.1);
        let grow_ceiling = self.defs[kind].layer_number;
        let obj = self.fact_create();
        self.bloons.push(Bloon { held: false, id, kind, dist, pos, hp: self.defs[kind].max_health, effects: Vec::new(), decamo: false, path, base: 0.0, grow_t, grow_ceiling, grow_colors: None, grow_fresh: true, fresh: false, prev: pos, prev_dist: dist, mult: 1.0, cell_pos: pos, track_mult, order: id, lineage: None, layer: { self.next_layer += 1; self.next_layer }, rot: self.path_rot(path, dist).unwrap_or(0.0), born: self.sim_elapsed * 4 + self.phase as u64, height: 0.0, unplaced: false, first_move: false, wind: None, wind_new: false, wind_seg: None, wind_direct: false, wind_model: None, grave_zone: (false, false), obj });
        self.plist_add(id, path);
        self.dart_time_spawned();
        if self.grid_on {
            self.grid.add(self.bloons.len() - 1, pos);
            if self.has_grid_rank {
                self.ggrid.push(self.bloons.len() - 1);
                // An object whose dead bloon the grid still lists takes its place there.
                if let Some((at, cell)) = self.ggrid.ghosts.remove(&obj) {
                    self.ggrid.take_place(self.bloons.len() - 1, at);
                    self.ghost_cell = Some(cell);
                }
            }
        }
        id
    }

    /// Bloon.Move / SetDistanceTravelled (VERIFIED-DISASM @0xfd3b90, @0xfd4fa0): a bloon is
    /// never behind spawnDist - 1.25 x its radius (it waits just off the map's edge).
    fn held_at_spawn(&self, kind: usize, path: u8, dist: f32) -> f32 {
        match self.track_ends.get(path as usize).and_then(|e| e.0) {
            Some(sd) => {
                let m = sd - self.defs[kind].radius * 1.25;
                if dist <= m { m } else { dist }
            }
            None => dist,
        }
    }

    fn point_in_poly(p: [f32; 2], poly: &[[f32; 2]]) -> bool {
        let mut inside = false;
        let mut j = poly.len() - 1;
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[j]);
            if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
                inside = !inside;
            }
            j = i;
        }
        inside
    }

    /// HYPOTHESIS: `Area.Intersects(Circle)` is not transcribed; standard circle-vs-polygon.
    fn circle_hits_poly(c: [f32; 2], r: f32, poly: &[[f32; 2]]) -> bool {
        if Self::point_in_poly(c, poly) {
            return true;
        }
        let mut j = poly.len() - 1;
        for i in 0..poly.len() {
            let (a, b) = (poly[j], poly[i]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 { (((c[0] - a[0]) * dx + (c[1] - a[1]) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
            let (qx, qy) = (a[0] + dx * t - c[0], a[1] + dy * t - c[1]);
            if qx * qx + qy * qy < r * r {
                return true;
            }
            j = i;
        }
        false
    }

    /// `Map.CanPlace` for circle-footprint land towers (research/04 §11, VERIFIED-DISASM except
    /// the polygon geometry helpers and the base area's exact shape).
    /// The ship (tower index) and deck height if `p` is on a deck tower `def` may stand on.
    /// HYPOTHESIS P-DECK: deck points are (x right, y forward) of the ship's facing, like
    /// weapon eject points; the size filter (small / medium) isn't applied.
    pub fn deck_at(&self, def: usize, p: [f32; 2]) -> Option<(usize, f32)> {
        let name = &self.tower_defs[def].0;
        for (s, tw) in self.towers.iter().enumerate() {
            let Some(tr) = self.tier_of(tw) else { continue };
            for d in &tr.decks {
                if d.exclude.iter().any(|x| name == x || name.ends_with(x.as_str())) {
                    continue;
                }
                let (sn, cs) = tw.angle.sin_cos();
                let poly: Vec<[f32; 2]> = d.points.iter().map(|q| [tw.pos[0] + cs * q[1] + sn * q[0], tw.pos[1] + sn * q[1] - cs * q[0]]).collect();
                if Self::point_in_poly(p, &poly) {
                    return Some((s, d.height));
                }
            }
        }
        None
    }

    /// AreaModel.IsPointInside: inside the outline and in none of the holes.
    fn in_area(p: [f32; 2], a: &Area) -> bool {
        Self::point_in_poly(p, &a.points) && !a.holes.iter().any(|h| Self::point_in_poly(p, h))
    }

    /// A circle touches an area: its centre is inside it, or it crosses the outline or a hole's
    /// edge (STAND-IN for Area.Intersects with holes).
    fn circle_hits_area(c: [f32; 2], r: f32, a: &Area) -> bool {
        if Self::in_area(c, a) {
            return true;
        }
        let crosses = |poly: &[[f32; 2]]| {
            (0..poly.len()).any(|i| {
                let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
                let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
                let l2 = (dx * dx + dy * dy).max(1e-9);
                let t = (((c[0] - p[0]) * dx + (c[1] - p[1]) * dy) / l2).clamp(0.0, 1.0);
                let (x, y) = (p[0] + t * dx - c[0], p[1] + t * dy - c[1]);
                x * x + y * y <= r * r
            })
        };
        crosses(&a.points) || a.holes.iter().any(|h| crosses(h))
    }

    pub fn can_place(&self, def: usize, p: [f32; 2]) -> bool {
        let r = self.tower_defs[def].1.radius;
        let areas = &self.tower_defs[def].1.areas;
        let deck = self.deck_at(def, p).map(|(s, _)| s);
        // HYPOTHESIS P-WATER: a water tower (areaTypes has 1) goes where the first area under the
        // point (above the base land) is water; land-only towers keep Map.CanPlace's land rule.
        let water = areas.contains(&1) && self.water_at(p);
        let land_tower = areas.is_empty() || areas.contains(&AREA_LAND);
        let land = land_tower && (self.land_clear(p, r) || (self.water_at(p) && self.frozen_water_at(p)));
        if !water && !land && deck.is_none() {
            return false;
        }
        // OverlapsOtherEntity: circle-circle, strict. STAND-IN P-SUBBLOCK: sub-towers (sentries)
        // don't block placement.
        for (i, t) in self.towers.iter().enumerate().filter(|(_, t)| t.sub.is_none()) {
            if Some(i) == deck {
                continue;
            }
            let rr = r + self.tower_defs[t.def].1.radius;
            if rr * rr > (t.pos[0] - p[0]).powi(2) + (t.pos[1] - p[1]).powi(2) {
                return false;
            }
        }
        true
    }

    /// FreezeNearbyWaterModel: `p` is within a water-freezing tower's reach. HYPOTHESIS
    /// P-FROZENWATER: the point inside the reach is enough.
    fn frozen_water_at(&self, p: [f32; 2]) -> bool {
        (0..self.towers.len()).any(|o| {
            let Some(r) = self.tier(o).and_then(|tr| tr.freeze_water) else { return false };
            let r = if r > 0.0 { r } else { self.range(o) };
            let q = self.towers[o].pos;
            (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) <= r * r
        })
    }

    /// `p` lies in a water area (areaType 1) and on the map.
    fn water_at(&self, p: [f32; 2]) -> bool {
        on_map(p, 0.0) && self.areas.iter().any(|a| a.kind == 1 && Self::in_area(p, a))
    }

    /// Map.IsTargetLineClear (VERIFIED-DISASM @0xf7aec0, Area.Intersects @0xf67e40,
    /// Polygon.Intersects @0xeaada0): the line from `from` (height `from_z`) to `to` crosses no
    /// edge of an active blocker area taller than `from_z` (outline, then holes).
    /// CollisionGroupData.precalculatedOffset of bloon `b`'s collision circle `ci`, as the
    /// attack targeting has it: turned by the bloon's rotation (game frame, y flipped here), or
    /// along the track where the path gives none.
    fn circle_off(&self, b: &Bloon, ci: usize) -> [f32; 2] {
        let c = self.defs[b.kind].circles[ci];
        match self.path_rot(b.path, 0.0).map(|_| b.rot) {
            Some(r) => {
                let (sa, ca) = crt_sin_cos(DEG2RAD * -r);
                let (ox, oy) = (ca * c[0] - sa * c[1], ca * c[1] + sa * c[0]);
                [ox, -oy]
            }
            None => {
                let tr = &self.tracks[b.path as usize];
                let (a, z) = (tr.pos((b.dist - 0.5).max(0.0)), tr.pos(b.dist + 0.5));
                let l = (z[0] - a[0]).hypot(z[1] - a[1]).max(1e-6);
                let fwd = [(z[0] - a[0]) / l, (z[1] - a[1]) / l];
                [fwd[0] * c[1] - fwd[1] * c[0], fwd[1] * c[1] + fwd[0] * c[0]]
            }
        }
    }

    /// TrackTarget.Process (VERIFIED-DISASM @0xd13290) for every seeking projectile made before
    /// this frame, after it and the bloons have moved (the process list has the bloons before
    /// the projectiles, and TravelStrait before TrackTarget): the direction it sets is the next
    /// frame's move (recording sub_010: a dart turns towards a Red another dart pops in the next
    /// frame's collisions). On a look (5 frames after the last, or at once after it hits its
    /// target: TrackTarget.Collide) it drops a target that is no longer IsBloonValid (recording
    /// sub_001: a dart's Black goes behind a log), then with trackNewTargets finds a new one if
    /// it has none (or always, with constantlyAquireNewTarget); with a target it turns towards
    /// it (CalculateDirection). Between looks a gone target means flying straight (recording
    /// tack_500: a meteor whose MOAB popped).
    fn track_targets(&mut self) {
        let now = self.sim_elapsed * 4;
        for p in 0..self.projectiles.len() {
            let pr = &self.projectiles[p];
            let Some(sk) = pr.spec.seek.as_ref() else { continue };
            if pr.spec.path.is_some() || pr.born >= now {
                continue;
            }
            // (The look clock starts when it is made; a shot thrown with a target keeps it until
            // then: recording ninja_002, a shuriken still turns for a Black behind the entrance
            // trees on its first frame.)
            let look = (self.sim_elapsed.saturating_sub(pr.seek_t) >= 5 || (pr.dir.is_none() && pr.seek.is_none())).then(|| {
                let keep = pr.seek.filter(|&id| self.bloons.iter().find(|b| b.id == id).is_some_and(|b| self.seek_valid(pr, sk, b, None)));
                let pick = (sk.new_targets && (keep.is_none() || sk.constantly)).then(|| self.seek_pick(pr, sk, None, 0.0));
                (keep, pick)
            });
            let pr = &mut self.projectiles[p];
            let sk = pr.spec.seek.clone().unwrap();
            if let Some((keep, pick)) = look {
                pr.seek_t = self.sim_elapsed;
                if keep.is_none() {
                    pr.seek = None;
                    pr.seek_off = None;
                }
                if let Some(best) = pick {
                    pr.seek = best.map(|x| x.0);
                    if pr.spec.only_target && best.is_some() {
                        pr.cot = pr.seek;
                    }
                    let off = best.and_then(|(id, ci)| ci.zip(self.bloons.iter().find(|b| b.id == id))).map(|(ci, b)| self.circle_off(b, ci as usize));
                    self.projectiles[p].seek_off = off;
                }
            }
            // Target.Position: the bloon's, plus its collision circle's offset.
            let off = self.projectiles[p].seek_off.unwrap_or([0.0; 2]);
            // Simulation.Simulate runs the process list in creation order: a bloon made after
            // the projectile hasn't moved yet when its TrackTarget looks (recording wizard_205,
            // rt 1858-1860: Guided Magic heads for a newer Black's previous-frame spot each frame;
            // sub_*: older bloons, their moved spot).
            let born = self.projectiles[p].born;
            let q = self.projectiles[p].seek.and_then(|id| self.bloons.iter().find(|b| b.id == id)).map(|b| {
                let at = if b.born > born { b.prev } else { b.pos };
                [at[0] + off[0], at[1] + off[1]]
            });
            let pr = &mut self.projectiles[p];
            let heading = pr.vel[1].atan2(pr.vel[0]);
            if let (Some(q), Some(d), false, true) = (q, pr.dir, pr.spec.speed == 0.0, pr.seek_moved) {
                // TrackTarget.CalculateDirection (VERIFIED-DISASM @0xd127b0), game frame:
                // Vector2.AngleBetween(direction, target - position) (@0xf64060: the
                // atan2f difference wrapped into -pi..pi, negated, in degrees), clamped
                // to turnRatePerFrame, and the direction turned by it
                // (Vector3Boxed.Rotate @0xf654a0); TravelStrait then moves it at its
                // speed (recording tack_500: Inferno Ring's meteor).
                // (Not on its first move: recording tack_500, the meteor's first step
                // is straight along its launch direction.)
                let t = [q[0] - pr.pos[0], -(q[1] - pr.pos[1])];
                let mut a = crt_atan2(t[1], t[0]) - crt_atan2(d[1], d[0]);
                if a > std::f32::consts::PI {
                    a += -2.0 * std::f32::consts::PI;
                } else if a < -std::f32::consts::PI {
                    a += 2.0 * std::f32::consts::PI;
                }
                // turnRatePerFrame = turnRate x (1 / 60) in f32 (360 -> 6.0000005: recording
                // sub_000_close, a torpedo's y one float step off at 6.0).
                let max = sk.turn_rate * FRAME_TIME;
                let mut ang = (RAD2DEG * -a).clamp(-max, max);
                // TrackTargetWithinTime.CalculateDirection (VERIFIED-DISASM @0xcf3560): the
                // clamped turn / (timeInFrames / elapsed), clamped again; not finite: no turn
                // (recording ninja_002: Seeking Shuriken turns 1 / 3.48 of the way each frame).
                if let Some(t) = sk.within {
                    ang /= t / 1.0;
                    if !ang.is_finite() {
                        continue;
                    }
                    ang = ang.clamp(-max, max);
                }
                let r = DEG2RAD * -ang;
                let (sn, cs) = crt_sin_cos(r);
                let nd = [cs * d[0] - sn * d[1], cs * d[1] + sn * d[0]];
                pr.dir = Some(nd);
                let sf = pr.spec.speed * FRAME_TIME;
                pr.vel = [nd[0] * sf, -nd[1] * sf];
                pr.grot = None;
            } else if let (Some(q), None) = (q, pr.dir) {
                let max = (sk.turn_rate / FPS).to_radians();
                let h = heading + angle_diff((q[1] - pr.pos[1]).atan2(q[0] - pr.pos[0]), heading).clamp(-max, max);
                let sp = pr.vel[0].hypot(pr.vel[1]);
                pr.vel = [h.cos() * sp, h.sin() * sp];
                pr.grot = None;
            }
        }
    }

    /// TrackTarget.GetNewTarget's range: `distance`, or with useLifetimeAsDistance the way it has
    /// left to fly ((lifespanFrames - age) / lifespanFrames x lifespanFrames x speedFrames).
    /// `ahead`: frames of the frame's age not counted in `pr.age` yet (the age counts the frame
    /// being processed; recordings sub_010, a dart with 6 frames to fly passes up a Pink 16.8
    /// away, and one with 9 takes a Green 24.8 away as it hits its target).
    fn seek_range(&self, pr: &Projectile, sk: &crate::data::Seek, ahead: f32) -> f32 {
        if sk.lifetime {
            let lf = (pr.spec.lifespan * FPS).round_ties_even();
            let age = pr.age + ahead;
            (lf - age) / lf * (lf * (pr.spec.speed * FRAME_TIME))
        } else {
            sk.distance
        }
    }

    /// TrackTarget.IsBloonValid (VERIFIED-DISASM @0xd12ff0): not hit (Projectile.FilterBloon),
    /// within maxSeekAngle of its heading unless that is 360 or more (recording sub_000_last, a
    /// torpedo turns 100 degrees), and the line to it clear unless the shot ignores blockers.
    /// `also`: a layer it has just hit, not in its hits yet.
    fn seek_valid(&self, pr: &Projectile, sk: &crate::data::Seek, b: &Bloon, also: Option<u64>) -> bool {
        let d = &self.defs[b.kind];
        if b.hit_by(&pr.hit) || also.is_some_and(|l| b.layer == l || b.lineage.as_ref().is_some_and(|x| x.contains(&l))) || (!pr.camo && b.camo(d)) {
            return false;
        }
        // HYPOTHESIS P-SEEKOFFTRACK: a bloon the wind carries (offTrackCount) isn't a new target
        // (recording ninja_032 rt 3175: a shuriken passes over a Black blown back from its spot).
        if b.wind.is_some() || b.held {
            return false;
        }
        // (A bloon made after the projectile hasn't moved yet when its TrackTarget looks: its
        // previous spot; recording ninja_502_last rt 482, a White 142.7 degrees off, not 144.02.)
        let bp = if b.born > pr.born { b.prev } else { b.pos };
        let (dx, dy) = (bp[0] - pr.pos[0], bp[1] - pr.pos[1]);
        if sk.max_angle < 360.0 && angle_diff(dy.atan2(dx), pr.vel[1].atan2(pr.vel[0])).abs().to_degrees() > sk.max_angle {
            return false;
        }
        pr.spec.ignore_blockers || self.line_clear(pr.pos, 0.0, bp)
    }

    /// TrackTarget.GetNewTarget (VERIFIED-DISASM @0xd12be0): BloonManager.GetTarget with
    /// TargetType Close, i.e. BloonManager.GetClosestTarget (VERIFIED-DISASM @0xfe8700), over
    /// `seek_range`. Of the targetable path segments
    /// within range (+ 40 unless it can't hit MOABs; paths in order, segments ascending), the
    /// first strictly nearer than the best so far that holds a bloon it may take gives the first
    /// such bloon in Path.bloonsList order (recording sub_001: a Black on a nearer segment over a
    /// nearer Black on a further one), of those `seek_valid`.
    fn seek_pick(&self, pr: &Projectile, sk: &crate::data::Seek, also: Option<u64>, ahead: f32) -> Option<(u64, Option<u8>)> {
        let range = self.seek_range(pr, sk, ahead);
        let reach = range + if pr.spec.exclude_tags.iter().any(|t| t == "Moabs") { 0.0 } else { 40.0 };
        // A segment is in range when its point is within range + 2 (PathSegment.IsInRange): its
        // bloons can be taken as they are; on one only within the reach, MOAB-class ones by
        // Targetting.CanHitMoab: the first collision circle within range + its radius, which the
        // Target then carries (STAND-IN: without CanHitMoab's line-of-sight test).
        let valid = |b: &Bloon, near: bool| -> Option<Option<u8>> {
            let d = &self.defs[b.kind];
            let circle = if near {
                None
            } else if d.is_moab {
                Some((0..d.circles.len()).find(|&ci| {
                    let o = self.circle_off(b, ci);
                    let c = [b.pos[0] + o[0] - pr.pos[0], b.pos[1] + o[1] - pr.pos[1]];
                    c[0] * c[0] + c[1] * c[1] < (range + d.circles[ci][2]) * (range + d.circles[ci][2])
                })? as u8)
            } else {
                return None;
            };
            self.seek_valid(pr, sk, b, also).then_some(circle)
        };
        let mut best: Option<(f32, u64, Option<u8>)> = None;
        for (pi, sg) in self.track_segs.iter().enumerate() {
            for (k, q) in sg.iter().enumerate() {
                if !self.track_seg_ok.get(pi).and_then(|v| v.get(k)).copied().unwrap_or(false) {
                    continue;
                }
                let dd = (q[0] - pr.pos[0]) * (q[0] - pr.pos[0]) + (q[1] - pr.pos[1]) * (q[1] - pr.pos[1]);
                if dd > reach * reach || best.is_some_and(|b| dd.sqrt() >= b.0) {
                    continue;
                }
                let near = dd <= (range + 2.0) * (range + 2.0);
                // (Path.Process files the bloons on their segments before they move: at the
                // frame's end the lists still have them where the frame began, recording sub_010,
                // a dart passing up a Black that has just moved onto a nearer segment.)
                // (A segment lists its bloons in Path.bloonsList order: recording ninja_502_close,
                // a shuriken re-targets a White ahead of a Yellow made later on the same segment.)
                let rank = |b: &Bloon| self.path_rank.get(&b.id).map_or(u32::MAX, |r| r.1);
                if let Some((b, circle)) = self
                    .bloons
                    .iter()
                    .filter(|b| b.path as usize == pi && (b.wind_seg.unwrap_or(b.prev_dist) / 7.0).max(0.0) as usize == k)
                    .filter_map(|b| valid(b, near).map(|c| (b, c)))
                    .min_by_key(|(b, _)| rank(b))
                {
                    best = Some((dd.sqrt(), b.id, circle));
                }
            }
        }
        best.map(|b| (b.1, b.2))
    }

    pub fn line_clear(&self, from: [f32; 2], from_z: f32, to: [f32; 2]) -> bool {
        for a in self.areas.iter().filter(|a| a.is_blocker && !a.is_disabled && a.height > from_z) {
            for poly in std::iter::once(&a.points).chain(a.holes.iter()) {
                let n = poly.len();
                for i in 0..n {
                    if segment_cross(from, to, poly[i], poly[(i + 1) % n]).is_some() {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// RandomPosition's target for tower `t`'s attack `key` (clone frame). UpdateValidPositions
    /// (VERIFIED-DISASM @0x180dd3470) when the tower gets the model: the offsets, shuffled
    /// (@0x181921d10: syncedRandom.Next(count) for each slot of its reused buffer, then from the
    /// end each entry swapped with the drawn one), index 0. CheckPositions (@0x180dd27f0) when
    /// due (made, or a tower created since): tower position + offset, TruncateVector2(2), on the
    /// map, then in / out of idealDistanceWithinTrack (a path segment's point within it) x
    /// placeable clear of towers / placeable at all. GetTargets (MoveNext @0x180dd9350): the
    /// first non-empty list, index + 1 (wrapping to 0) taken.
    /// RandomPosition.UpdatedModel / Initialise: its offsets, shuffled, for the tower's model now
    /// (at the upgrade itself: recording engineer_upgrades_r30, a reshuffle at each of two upgrades).
    fn random_pos_update(&mut self, t: usize, key: &str, m: &crate::data::RandomPos) -> usize {
        let k = match self.towers[t].randpos.iter().position(|x| x.0 == key) {
            Some(k) => k,
            None => {
                self.towers[t].randpos.push((key.to_string(), RandPosState { model: None, offsets: Vec::new(), buf: 0, lists: Default::default(), index: 0, dirty: true }));
                self.towers[t].randpos.len() - 1
            }
        };
        if self.towers[t].randpos[k].1.model != Some(*m) {
            let mut offs = rand_pos_offsets(m);
            let n = offs.len();
            // (The buffer starts at 200: recording engineer_upgrades_r30's Next(102) x 200, then
            // Next(239) x 239; a model with the same fields isn't shuffled again.)
            let buf = self.towers[t].randpos[k].1.buf.max(n).max(200);
            let rnd: Vec<usize> = (0..buf).map(|_| (self.synced_double() * n as f64) as usize).collect();
            let mut i = n;
            while i >= 2 {
                i -= 1;
                offs.swap(rnd[i % buf], i);
            }
            let st = &mut self.towers[t].randpos[k].1;
            (st.model, st.offsets, st.buf, st.index, st.dirty) = (Some(*m), offs, buf, 0, true);
        }
        k
    }

    fn random_pos_init(&mut self, t: usize) {
        if let Some(tr) = self.tier(t) {
            let rps: Vec<_> = tr.attacks.iter().filter_map(|a| a.random_pos.map(|m| (a.model_name.clone(), m))).collect();
            for (key, m) in rps {
                self.random_pos_update(t, &key, &m);
            }
        }
    }

    fn random_pos_target(&mut self, t: usize, key: &str, m: crate::data::RandomPos) -> Option<[f32; 2]> {
        let k = self.random_pos_update(t, key, &m);
        if self.towers[t].randpos[k].1.dirty {
            let tp = [self.towers[t].pos[0], -self.towers[t].pos[1]];
            let trunc = |v: f32| (((v * 100.0) as f64).trunc() / 100.0f32 as f64) as f32;
            let mut lists: [Vec<[f32; 2]>; 4] = Default::default();
            for off in self.towers[t].randpos[k].1.offsets.clone() {
                let pc = [trunc(tp[0] + off[0]), -trunc(tp[1] + off[1])];
                if !on_map(pc, 0.0) {
                    continue;
                }
                let near = m.ideal <= 0.0
                    || self.track_segs.iter().enumerate().any(|(pi, sg)| sg.iter().enumerate().any(|(k, q)| self.track_seg_ok.get(pi).and_then(|v| v.get(k)).copied().unwrap_or(false) && (q[0] - pc[0]).powi(2) + (q[1] - pc[1]).powi(2) < m.ideal * m.ideal));
                let placeable = if m.water { on_map(pc, -m.radius) && self.areas.iter().any(|a| a.kind == 1 && Self::in_area(pc, a)) } else { self.land_clear(pc, m.radius) };
                if !placeable {
                    continue;
                }
                // (Towers whose footprint doesntBlockTowerPlacement, the Sentries, don't count.)
                let clear = self.towers.iter().filter(|o| o.sub.is_none()).all(|o| {
                    let rr = m.radius + self.tower_defs[o.def].1.radius;
                    rr * rr <= (o.pos[0] - pc[0]).powi(2) + (o.pos[1] - pc[1]).powi(2)
                });
                lists[if near { 0 } else { 2 } + if clear { 0 } else { 1 }].push(pc);
            }
            let st = &mut self.towers[t].randpos[k].1;
            st.lists = lists;
            st.dirty = false;
        }
        let st = &mut self.towers[t].randpos[k].1;
        let l = st.lists.iter().find(|l| !l.is_empty())?;
        let mut idx = st.index + 1;
        if idx as usize >= l.len() {
            idx = 0;
        }
        let q = l[idx as usize];
        st.index = idx;
        Some(q)
    }

    /// The map part of `can_place`: placeable land for a circle of radius `r` at `p`.
    fn land_clear(&self, p: [f32; 2], r: f32) -> bool {
        if !on_map(p, -r) {
            return false;
        }
        // The first area containing the point decides; the base land area is first.
        let Some(area) = self.areas.iter().find(|a| a.kind != AREA_SKIPPED && Self::in_area(p, a)) else {
            return true;
        };
        if area.kind != AREA_LAND {
            return false;
        }
        // Other unplaceable areas within 0..=5 above it must not touch the 0.75 * radius circle.
        for o in &self.areas {
            if std::ptr::eq(o, area) || o.kind == AREA_SKIPPED || o.kind == AREA_LAND {
                continue;
            }
            let dh = o.height - area.height;
            if (0.0..=5.0).contains(&dh) && Self::circle_hits_area(p, r * 0.75, o) {
                return false;
            }
        }
        // BlockedByBlockerZone: full radius, non-strict. The flag that inverts the test sits at
        // BlockerModel.isDisabled's offset (INFERRED); it is false on Monkey Meadow.
        for &(c, disabled) in &self.blockers {
            let d2 = (c[0] - p[0]).powi(2) + (c[1] - p[1]).powi(2);
            let blocked = if disabled { d2 >= (c[2] - r) * (c[2] - r) } else { (c[2] + r) * (c[2] + r) >= d2 };
            if blocked {
                return false;
            }
        }
        true
    }

    /// CollisionChecker.Process's MapBlockerGrid.GetInRange(position, VsBlockerRadius, margin 35,
    /// no radius override) (VERIFIED-DISASM @0x1825db740 / @0x1825db980, and by recording): the
    /// (int)((35 + 5 + r) / 10) x 2 + 1 square of cells in orderedNeighborOffsets order; a cell
    /// counts only if its largestCellSpan is above 0 (so blockers under radius 5 never stop
    /// anything) and max(dx, dy) - radiusCells <= 0 or <= that span; then its blockers count in
    /// the query's own cell, else within radius + VsBlockerRadius; only blockers taller than the
    /// projectile (`z`, its Position.z) count.
    fn blocker_hit(&self, q: [f32; 2], vsr: f32, z: f32) -> bool {
        if self.grid_offsets.is_empty() {
            return false;
        }
        let r = vsr.min(99999.0);
        let (qx, qy) = grid_cell(q);
        let reach = cell_span(r);
        let side = ((35.0 + 5.0 + r) / GAME_GRID_CELL) as i32 * 2 + 1;
        let n = ((side * side) as usize).min(self.grid_offsets.len() / 2);
        let (gx, gy) = (q[0], -q[1]);
        for (k, w) in self.grid_offsets.chunks(2).take(n).enumerate() {
            let (x, y) = (qx + w[0], qy + w[1]);
            if x < 0 || y < 0 || x >= 40 || y >= 33 {
                continue;
            }
            let (span, list) = &self.blocker_grid[(y * 40 + x) as usize];
            let m = w[0].max(w[1]) - reach;
            if *span <= 0 || !(m as f32 <= 0.0 || m <= *span) {
                continue;
            }
            for b in list {
                if b[3] > z && (k == 0 || (b[0] - gx).powi(2) + (b[1] - gy).powi(2) <= (b[2] + r) * (b[2] + r)) {
                    return true;
                }
            }
        }
        false
    }

    /// Price of tower `def` placed at `p`, after discounts. HYPOTHESIS P-DISCOUNT: discounted
    /// prices round to the nearest 5 like the game's other price scaling.
    pub fn place_cost(&self, def: usize, p: [f32; 2]) -> f64 {
        let cost = game_cost(self.tower_defs[def].1.cost, self.cost_mult, self.discount(def, p, None, 0));
        match self.free_tower(def) {
            Some(m) => cost * m,
            None => cost,
        }
    }

    /// MonkeyCityModel: this round's free tower of kind `def` isn't used yet; its cost multiplier.
    /// HYPOTHESIS P-FREEDART: one a round, placed anywhere.
    fn free_tower(&self, def: usize) -> Option<f64> {
        if self.free_tower_round == Some(self.round) {
            return None;
        }
        let name = &self.tower_defs[def].0;
        self.towers.iter().find_map(|t| self.tier_of(t)?.free_tower.as_ref().filter(|f| f.tower == *name).map(|f| f.mult))
    }

    /// The hero definition if tower type `def` is a hero.
    pub fn hero_def(&self, def: usize) -> Option<&crate::data::HeroDef> {
        self.tower_tiers.get(def)?.as_ref()?.hero.as_ref()
    }

    /// `Simulation.GetXpForRound` (VERIFIED-DISASM @0xa76720) for 0-based round index `r`:
    /// 20 * (r + 2 + max(r - 19, 0)) + 50 * max(r - 49, 0), times
    /// `GameModel.selectedDifficultyRating` (1, recorded on Medium) and, by the caller, the map
    /// difficulty's `TRACK_DIFFICULTY_XP_SCALE`.
    pub fn xp_for_round(r: i32) -> f32 {
        let a = r + 2 + (r - 19).max(0);
        (20 * a + 50 * (r - 49).max(0)) as f32
    }

    /// The towers' part of `Simulation.DistributeXp` (round index `r`, its XP `xp`) when the game
    /// can earn XP (InGame.CanEarnPlayerAndTowerXP: not Sandbox, not a creation-mode map): past
    /// the mode's last round (Spawner.endRound) it's 0.3 of that, from round index 100 0.1 of it
    /// up to 3000. InputManager.DistributeXp adds it all to the account XP and to each standard
    /// tower (TowerModel.IsStandardTower: no hero, sub-tower, Geraldo item or power) its worth's
    /// share of every standard tower's (TowerManager.GetTotalSpentOnTowers), by its base id
    /// (Xp.AddTowerXp; Tower.TransformXp changes nothing without TransformTowerXp behaviors,
    /// which only Monkey Knowledge adds: Monkey Education's 8%, off here). VERIFIED by
    /// recording (tower_xp_rounds: DrainXP per round).
    fn distribute_tower_xp(&mut self, r: usize, xp: f32) {
        if self.sandbox || self.map_difficulty.is_none() {
            return;
        }
        let xp = if r + 1 <= self.end_round {
            xp
        } else if r < 100 {
            xp * 0.3
        } else {
            (xp * 0.1).min(3000.0)
        };
        self.xp_account += xp;
        let standard = |g: &Self, t: &Tower| t.sub.is_none() && g.hero_def(t.def).is_none();
        let total = self.towers.iter().filter(|t| standard(self, t)).map(|t| t.invested as f32).sum::<f32>();
        if total <= 0.0 {
            return;
        }
        for t in 0..self.towers.len() {
            if !standard(self, &self.towers[t]) {
                continue;
            }
            let share = self.towers[t].invested as f32 / total * xp;
            let id = self.tower_defs[self.towers[t].def].0.clone();
            match self.xp_towers.iter_mut().find(|(k, _)| *k == id) {
                Some(e) => e.1 += share,
                None => self.xp_towers.push((id, share)),
            }
        }
    }

    /// `Hero.AddXp` + `CheckForLevelUp` (VERIFIED-DISASM): xp * xpScale (xpBonus 1), then level
    /// up while xp >= the next level's xpCost, subtracting it.
    pub fn add_hero_xp(&mut self, t: usize, x: f32) {
        let Some(h) = self.hero_def(self.towers[t].def).cloned() else { return };
        if x <= 0.0 {
            return;
        }
        self.towers[t].xp += x * h.xp_scale;
        let mut leveled = false;
        while let Some(next) = h.levels.iter().find(|l| l.level == self.towers[t].tiers[0] + 1) {
            if self.towers[t].xp < next.xp {
                break;
            }
            self.towers[t].xp -= next.xp;
            self.towers[t].tiers[0] += 1;
            leveled = true;
        }
        if leveled {
            self.towers[t].show = Some((crate::anim::UPGRADE_CLIP.into(), 0.0));
            self.init_point(t);
            self.init_air(t);
            self.sync_abilities(t);
            self.sync_pets(t);
            let s = self.tier_sounds(t);
            self.sound(&s.upgrade, None);
            self.voice(&s.hero_upgrade, VOICE, VOICE_GAP);
        }
    }

    /// XP still needed for tower `t`'s next level, and that level's xpCost.
    pub fn hero_progress(&self, t: usize) -> Option<(f32, f32)> {
        let h = self.hero_def(self.towers[t].def)?;
        let next = h.levels.iter().find(|l| l.level == self.towers[t].tiers[0] + 1)?;
        Some((self.towers[t].xp, next.xp))
    }

    pub fn place(&mut self, def: usize, p: [f32; 2]) -> bool {
        if def>=self.tower_defs.len(){return false;}
        if self.mod_runtime.intercepts & crate::mod_runtime::PLACE!=0 {
            let event=serde_json::json!({"tower":self.tower_defs[def].0,"position":p,"cost":self.place_cost(def,p)});
            if self.mod_intercept("before_place",event).cancel{return false;}
        }
        let placed_id=self.next_id;
        let placed=self.place_native(def,p);
        if placed&&self.mod_runtime.listens("on_tower_placed"){
            if let Some(i)=self.towers.iter().position(|t|t.id==placed_id){
                let event=crate::mod_entities::tower(self,i);self.mod_event_payload("on_tower_placed",None,event);}
        }
        placed
    }
    fn place_native(&mut self, def: usize, p: [f32; 2]) -> bool {
        self.place_at(def, p, true)
    }

    /// `place` without the land / overlap test (`checked` false): InputManager.CreateTowerAt as
    /// SimRecorder calls it puts the tower down regardless (recording dart_paragon_deg_r40: two
    /// Darts 6.6 apart).
    pub(crate) fn place_at(&mut self, def: usize, p: [f32; 2], checked: bool) -> bool {
        // LockTowerModModel: this difficulty can't buy it.
        if self.locked.iter().any(|x| *x == self.tower_defs[def].0) {
            return false;
        }
        // One hero per game (co-op: one per player, their own, checked by coop_game).
        if self.coop_wallets.is_none() && self.hero_def(def).is_some() && self.towers.iter().any(|t| self.hero_def(t.def).is_some()) {
            return false;
        }
        let cost = self.place_cost(def, p);
        // Co-op: Map.CheckCoopPlacement, the acting player's areas.
        if checked && self.coop_wallets.as_ref().and_then(|w| w.actor).is_some_and(|me| !self.coop_area_allows(me, p)) {
            return false;
        }
        if self.cash < cost || (checked && !self.can_place(def, p)) {
            return false;
        }
        if self.free_tower(def).is_some() {
            self.free_tower_round = Some(self.round);
        }
        self.cash -= cost;
        self.stats.towers_placed += 1;
        if let Some(set) = self.shop.iter().find(|s| s.id == self.tower_defs[def].0).map(|s| s.set) {
            if let Some(k) = [1, 2, 4, 8, 16].iter().position(|&f| set & f != 0) {
                self.stats.sets[k] += 1;
            }
        }
        // InputManager.CreateTowerAt as SimRecorder calls it (`checked` false) is free.
        if checked {
            self.stats.cash_spent += cost;
        }
        let id = self.next_id;
        self.next_id += 1;
        // CreateEffectOnPlaceModel: PlacementEffect / HeroPlacementEffect where it lands.
        if let Some((fx, scale)) = self.tower_defs[def].1.place_fx.clone() {
            self.place_fx.push((p, fx, scale));
        }
        let deck = self.deck_at(def, p).map(|(sh, h)| (self.towers[sh].id, h));
        // RandomPosition's OnTowerCreated: calcPositionsNextFrame.
        for tw in &mut self.towers {
            for (_, rp) in &mut tw.randpos {
                rp.dirty = true;
            }
        }
        self.towers.push(Tower {
            id,
            pops: 0,
            damage: 0,
            placed_round: self.round + 1,
            cash_earned: 0.0,
            lives_gained: 0.0,
            def,
            pos: p,
            tiers: [0; 3],
            invested: cost,
            target: TargetMode::First,
            point: None,
            point_z: 0.0,
            weapons: Vec::new(),
            shots: 0,
            angle: -std::f32::consts::FRAC_PI_2,
            aim: -std::f32::consts::FRAC_PI_2,
            game_rot: None,
            spin: Vec::new(),
            randpos: Vec::new(),
            base_rot: 0.0,
            sub_base: None,
            turret_rel: 0.0,
            base_state: (false, 0, false, 0.0),
            aims: Vec::new(),
            attack_t: f32::MAX,
            idle_t: 0.0,
            bank: 0.0,
            brews: Vec::new(),
            sub: None,
            air: None,
            ability_cd: HashMap::new(),
            ability_round: HashMap::new(),
            temp: Vec::new(),
            turbo: None,
            fan: None,
            morph: None,
            auto_t: 0.0,
            focus: false,
            look: None,
            overclock: None,
            range_up: None,
            ultra: (0, 1.0, 1.0),
            mute: 0.0,
            rage: (0, 0.0, 0, 0),
            temp_supports: Vec::new(),
            ammo: None,
            xp: 0.0,
            deck,
            items: Vec::new(),
            tier_cap: 5,
            boosts: Vec::new(),
            paragon: None,
            paragon_active_at: 0,
            paragon_first: 0,
            paragon_kept: Vec::new(),
            attack_born: HashMap::new(),
            orbit: None,
            temple: None,
            // MonkeyAnimationController.placementConfigs: the monkey drops in (anim.rs).
            show: Some((crate::anim::PLACE_CLIP.into(), 0.0)),
            attacking_fx: false,
            had_target: HashMap::new(),
            mtp: None,
            grave: Vec::new(),
            necro_points: false,
            model_clock: Some((self.sim_elapsed as i64, self.round_clock)),
            placed_step: self.steps as u64,
            track_buffs: 0,
            track_args: None,
            grave_spent: (0.0, 0),
            necro_cur: 0,
            necro_next: 0,
            necro_sel: None,
            necro_seg: None,
            necro_emit: None,
            necro_dmg: 0.0,
            track_supply: None,
            comanche: (0.0, 0),
            vigilante_at: None,
            vagrant_zone: None,
            left_target: TargetMode::First,
            camo_prio: false,
            submerged: false,
            air_reverse: false,
            reverse_at: 0,
            flip: false,
        });
        let t = self.towers.len() - 1;
        if self.hero_def(def).is_some() {
            self.towers[t].tiers = [1, 0, 0];
        }
        self.init_air(t);
        self.sync_abilities(t);
        // RandomPosition.Initialise: every spawner attack shuffles when the tower is made.
        self.random_pos_init(t);
        self.sync_pets(t);
        // Point-aiming towers start in their first mode (Dartling: follow the cursor). STAND-IN
        // P-POINTDEFAULT: the set point starts on the track point nearest the tower.
        if let Some(&m) = self.target_modes(t).first().filter(|m| m.steers()) {
            self.towers[t].target = m;
        }
        self.init_point(t);
        let s = self.tier_sounds(t);
        self.sound(&s.place, None);
        self.voice(&s.hero_place, VOICE, VOICE_GAP);
        true
    }

    /// Tower `t` has an attack aimed at a point the player sets (Mortar, Cleansing Foam).
    pub fn has_point_attack(&self, t: usize) -> bool {
        self.tier(t).is_some_and(|tr| tr.attacks.iter().any(|a| a.targets.iter().any(|s| s == "Point")))
    }

    /// Tower `t` currently fires at its set point: in Point mode, or through a side attack.
    /// TSMThemeMonkeyAce: the reverse button is ready (300 frames since the last reverse).
    pub fn can_reverse(&self, t: usize) -> bool {
        self.towers[t].reverse_at == 0 || self.steps >= self.towers[t].reverse_at + 300
    }

    /// Reverses tower `t`'s flight pattern.
    pub fn reverse_air(&mut self, t: usize) {
        if self.can_reverse(t) {
            let steps = self.steps;
            let tw = &mut self.towers[t];
            tw.air_reverse = !tw.air_reverse;
            tw.reverse_at = steps.max(1);
        }
    }

    pub fn uses_point(&self, t: usize) -> bool {
        matches!(self.towers[t].target, TargetMode::Point | TargetMode::Centered) || self.tier(t).is_some_and(|tr| tr.attacks.iter().any(side_point))
    }

    /// TargetSelectedPointModel.startWithClosestTrackPoint: a point attack with no point yet
    /// aims at the track point nearest the tower.
    fn init_point(&mut self, t: usize) {
        if self.towers[t].point.is_none() && self.has_point_attack(t) {
            let p = self.towers[t].pos;
            let start = self.tier(t).is_some_and(|tr| tr.attacks.iter().any(|a| a.point_start_track));
            if !start {
                self.towers[t].point = self.closest_track_point(p, f32::MAX, 0.0).map(|(_, q, _)| q);
            }
        }
    }

    /// TargetSelectedPoint.Process (VERIFIED-DISASM @0xdecc20) with startWithClosestTrackPoint:
    /// until it has a point, each frame it takes Attack.GetVisibleInRangePathSegment's, the
    /// visible in-range segment point nearest the tower, no offset (recording wizard_320: every
    /// Wall of Fire at (-113.1, -18.9)); CanHaveTarget is false until then.
    fn start_track_point(&self, t: usize, a: &crate::data::Attack, range: f32) -> Option<[f32; 2]> {
        let p = self.towers[t].pos;
        let no_moab = a.exclude_tags.iter().any(|x| x == "Moabs");
        let d2 = |q: [f32; 2]| (q[0] - p[0]) * (q[0] - p[0]) + (q[1] - p[1]) * (q[1] - p[1]);
        let mut best: Option<([f32; 2], f32)> = None;
        for (pi, k) in self.visible_segments(self.attack_pos(t), range, no_moab, a.through_walls) {
            let q = self.track_segs[pi as usize][k];
            if best.is_none_or(|(_, b)| d2(q) < b) {
                best = Some((q, d2(q)));
            }
        }
        best.map(|x| x.0)
    }

    /// Launches tower `t`'s air unit from its pad if its tier has one.
    fn init_air(&mut self, t: usize) {
        if self.towers[t].air.is_none() && self.tier(t).is_some_and(|tr| tr.air.is_some()) {
            let tw = &mut self.towers[t];
            tw.air = Some(AirUnit {
                pos: tw.pos,
                vel: [0.0; 2],
                angle: tw.angle,
                phase: 0.0,
                height: 40.0,
                dive: None,
                grab: Vec::new(),
                gy: GyrState::default(),
                wait: 0.0,
                tick: 0.0,
                on_path: false,
                bank: 0.0,
                pitch: 0.0,
                scale: 1.0,
                tt: 0.0,
                age: 0.0,
            });
        }
    }

    /// Where tower `t`'s attacks come from and its range is centred: the Heli, else the tower.
    pub fn attack_pos(&self, t: usize) -> [f32; 2] {
        let tw = &self.towers[t];
        tw.air.as_ref().map_or(tw.pos, |a| a.pos)
    }

    /// STAND-IN P-HELI: steers each air unit toward its goal (the cursor in Follow Touch, the set
    /// point in Lock In Place, the first bloon in Pursuit) at HeliMovementModel.maxSpeed, with
    /// acceleration movementForceStart, slowing linearly within slowdownRadiusMax.
    fn move_air(&mut self) {
        let helis: Vec<(usize, [f32; 2])> = (0..self.towers.len())
            .filter(|&t| self.tier(t).and_then(|tr| tr.air.as_ref()).is_some_and(|a| a.patterns.is_empty()))
            .filter_map(|t| self.towers[t].air.as_ref().map(|a| (t, a.pos)))
            .collect();
        for t in 0..self.towers.len() {
            let Some(spec) = self.tier(t).and_then(|tr| tr.air.clone()) else { continue };
            if self.tier(t).is_some_and(|tr| tr.grapple.is_some()) {
                continue;
            }
            if !spec.patterns.is_empty() {
                self.fly_pattern(t, &spec);
                continue;
            }
            let tw = &mut self.towers[t];
            // Lock In Place without a set point holds where the Heli is.
            if tw.target == TargetMode::Point && tw.point.is_none() {
                tw.point = tw.air.as_ref().map(|a| a.pos);
            }
            let tw = &self.towers[t];
            let Some(a) = &tw.air else { continue };
            let goal = match tw.target {
                TargetMode::Pointer => self.coop_pointer(tw.id).unwrap_or(a.pos),
                TargetMode::Point => tw.point.unwrap_or(tw.pos),
                TargetMode::Pursuit => self
                    .bloons
                    .iter()
                    .filter(|b| on_map(b.pos, 0.0))
                    .max_by(|x, y| x.progress().total_cmp(&y.progress()))
                    .map_or(a.pos, |b| b.pos),
                _ => a.pos,
            };
            let a = self.towers[t].air.as_mut().unwrap();
            let d = [goal[0] - a.pos[0], goal[1] - a.pos[1]];
            let len = d[0].hypot(d[1]);
            let want = spec.speed * (len / spec.slowdown.max(1.0)).min(1.0);
            let mut desired = if len > 1e-3 { [d[0] / len * want, d[1] / len * want] } else { [0.0; 2] };
            // HeliMovementModel.otherHeliRepulsionRange (30): Helis closer than that push apart,
            // harder the closer they are (STAND-IN strength).
            for &(o, q) in &helis {
                let e = [a.pos[0] - q[0], a.pos[1] - q[1]];
                let el = e[0].hypot(e[1]);
                if o != t && el < 30.0 {
                    let (ux, uy) = if el > 1e-3 { (e[0] / el, e[1] / el) } else { (((t * 7) as f32).cos(), ((t * 7) as f32).sin()) };
                    let k = spec.speed * (1.0 - el / 30.0);
                    desired = [desired[0] + ux * k, desired[1] + uy * k];
                }
            }
            let dv = [desired[0] - a.vel[0], desired[1] - a.vel[1]];
            let dl = dv[0].hypot(dv[1]);
            let max_dv = spec.accel / FPS;
            let k = if dl > max_dv { max_dv / dl } else { 1.0 };
            a.vel = [a.vel[0] + dv[0] * k, a.vel[1] + dv[1] * k];
            a.pos = [a.pos[0] + a.vel[0] / FPS, a.pos[1] + a.vel[1] / FPS];
            if a.vel[0].hypot(a.vel[1]) > 5.0 {
                a.angle = a.vel[1].atan2(a.vel[0]);
            }
        }
    }

    /// GyrfalconPattern (VERIFIED-DISASM Process @0xdc69a0, UpdateTargeting @0xdc93c0,
    /// UpdateMovement @0xdc8130, ApplyMovement @0xdc2d60, SnatchBloon @0xdc7620, ReleaseBloons
    /// @0xdc6f20, UpdateGrappleDamage @0xdc7b70): the bird starts 30 below its handler, flies to
    /// the best bloon in the handler's range at maxSpeed x bloonTargetSpeedMultiplier; within
    /// snapDistance it snatches it and others around it (SetPath to the destination segment's
    /// centre less 2 per bloon held, a stun, the initial damage), carries them at maxSpeed to the
    /// segment in range furthest from the exit, hitting them every grapple period; there it lets
    /// go and cools down. Held bloons sit on the bird (MoveWithAirUnit).
    fn fly_birds(&mut self) {
        for t in 0..self.towers.len() {
            let Some(g) = self.tier(t).and_then(|tr| tr.grapple.clone()) else { continue };
            self.init_air(t);
            let rt = self.round_clock;
            let id = self.towers[t].id;
            let (parent, owner) = match &self.towers[t].sub {
                Some(s) if s.spec.credit_parent => (s.parent, s.parent),
                Some(s) => (s.parent, id),
                None => (id, id),
            };
            let pi = self.towers.iter().position(|o| o.id == parent).unwrap_or(t);
            let home = self.towers[pi].pos;
            let mode = self.towers[pi].target;
            let range = self.range(pi);
            let cd = (g.cooldown * FPS) as i64;
            let rate = (g.rate * FPS) as i64;
            let snap = g.snap;
            let mut st = std::mem::take(&mut self.towers[t].air.as_mut().unwrap().gy);
            let mut pos = self.towers[t].air.as_ref().unwrap().pos;
            let mut held = std::mem::take(&mut self.towers[t].air.as_mut().unwrap().grab);
            // SetInitialPosition: initialYOffset below the handler (up the clone's y), on the map.
            if !st.init {
                st.init = true;
                st.stamp = self.sim_elapsed * 4 + 1;
                pos = [home[0], home[1] + g.y_offset];
                st.idle = Some(pos);
                st.hover = true;
                st.cd_start = -1;
                st.next_idle = -1;
                // (The pet's made here, and processes from the next frame: recording beast_003, its
                // first idle plan a frame after this.)
                let a = self.towers[t].air.as_mut().unwrap();
                a.pos = pos;
                a.gy = st;
                continue;
            }
            held.retain(|b| self.bloons.iter().any(|x| x.id == *b));
            // UpdateGrappleDamage.
            if !held.is_empty() && rt >= st.last_grapple + rate {
                let ids = held.clone();
                self.bird_hit(&ids, &g, g.initial, g.grapple_moab, owner);
                held.retain(|b| self.bloons.iter().any(|x| x.id == *b));
                st.last_grapple = rt;
            }
            // UpdateTargeting.
            if rt >= st.cd_start + cd {
                // (Carrying nothing any more, it lets go.)
                if held.is_empty() && matches!(st.target, GyrTarget::Seg(..)) {
                    self.bird_release(&mut held, &mut st, rt);
                    st.target = GyrTarget::None;
                } else if !self.gyr_valid(&st) {
                    st.target = match self.bird_target(home, range, mode, &g, pos, st.stamp, None, &[], &[]) {
                        Some(b) => {
                            // handlerSendAnimationState.
                            if st.hover {
                                self.towers[pi].show = Some(("IdleBirdGo".into(), 0.0));
                            }
                            GyrTarget::Bloon(b.0, b.1)
                        }
                        None if !st.hover => GyrTarget::Tower,
                        None => GyrTarget::None,
                    };
                }
            }
            // Process: with nothing to do it plans an idle move.
            if !self.gyr_valid(&st) {
                if st.next_idle < 0 {
                    let d = self.synced_double();
                    st.next_idle = g.idle_min as i64 + (d * (g.idle_max - g.idle_min) as f64) as i64 + rt;
                }
                // The idle move: a random spot within the hovered tower's range, on the map.
                if st.next_idle <= rt && st.hover {
                    let (d1, d2) = (self.synced_double() as f32, self.synced_double() as f32);
                    let (sa, ca) = crt_sin_cos(DEG2RAD * -(d1 * 360.0));
                    let r = d2 * range;
                    let q = [home[0] - sa * r, home[1] - ca * r];
                    // Map.GetPointWithinMap @0xf5a0c0: Constants' world bounds.
                    let q = [q[0].clamp(-WORLD_X, WORLD_X), q[1].clamp(-WORLD_Z, WORLD_Z)];
                    st.target = GyrTarget::Point(q);
                    st.next_idle = -1;
                }
            } else {
                st.next_idle = -1;
            }
            // UpdateMovement.
            let dest = match st.target {
                GyrTarget::Bloon(b, _) => self.bloons.iter().find(|x| x.id == b).map_or(home, |x| Self::gyr_at(x, st.stamp)),
                GyrTarget::Seg(p, k) => self.track_segs[p as usize][k],
                GyrTarget::Point(q) => q,
                GyrTarget::Tower | GyrTarget::None => if st.hover { st.idle.unwrap_or(home) } else { home },
            };
            let in_cooldown = st.cd_start >= 0 && rt < st.cd_start + cd;
            st.last_pos = pos;
            // (UpdateMovement returns while cooling down: it waits where it let go; invalid target
            // and hovering, it stays too.)
            let mut stay = in_cooldown || (!self.gyr_valid(&st) && st.hover);
            if !matches!(st.target, GyrTarget::Point(_)) && (st.hover || st.cd_start >= 0) && (dest[0] - pos[0]).hypot(dest[1] - pos[1]) <= snap {
                stay = true;
            }
            if st.cd_start >= 0 && !in_cooldown {
                st.cd_start = -1;
            }
            if !stay {
                if !matches!(st.target, GyrTarget::Point(_)) {
                    st.hover = false;
                }
                // ApplyMovement: maxSpeed, x bloonTargetSpeedMultiplier flying empty at a bloon,
                // x moabSpeedScale carrying a MOAB-class bloon; direction, speed x dt.
                let mut speed = g.speed;
                if matches!(st.target, GyrTarget::Bloon(..)) && self.gyr_valid(&st) && held.is_empty() {
                    speed *= g.dive_mult;
                }
                if g.moab_scale != 1.0 && held.iter().any(|b| self.bloons.iter().any(|x| x.id == *b && self.defs[x.kind].is_moab)) {
                    speed *= g.moab_scale;
                }
                let d = [dest[0] - pos[0], dest[1] - pos[1]];
                let m = (d[0] * d[0] + d[1] * d[1]).sqrt();
                if m > 0.0 {
                    let step = speed * (1.0 / 60.0f32);
                    pos = [pos[0] + d[0] / m * step, pos[1] + d[1] / m * step];
                }
            }
            let close = (dest[0] - pos[0]).powi(2) + (dest[1] - pos[1]).powi(2) <= snap * snap;
            if close && !in_cooldown {
                match st.target {
                    GyrTarget::Tower => {
                        st.hover = true;
                        st.target = GyrTarget::None;
                        // handlerHoverAnimationState.
                        self.towers[pi].show = Some(("IdleBirdCome".into(), 0.0));
                    }
                    GyrTarget::Point(q) => {
                        st.hover = true;
                        st.idle = Some(q);
                        st.target = GyrTarget::None;
                    }
                    GyrTarget::Seg(..) => {
                        self.bird_release(&mut held, &mut st, rt);
                        st.target = GyrTarget::None;
                    }
                    GyrTarget::None if !st.hover => {
                        st.hover = true;
                        st.idle = None;
                    }
                    GyrTarget::Bloon(b, _) if self.bloons.iter().any(|x| x.id == b) => {
                        self.towers[t].attack_t = 0.0;
                        // The segment in range furthest from the exit. (UpdateMovement @0xdc8130: the
                        // target is snatched, or only hit if it has passed that segment or can't be,
                        // and so with what's around it, Targetting.GetTargets with Close around it,
                        // while pierce is left: amount less each held bloon's cost.)
                        let dseg = self.bird_dest_segment(pi);
                        let mut tagged: Vec<u64> = Vec::new();
                        let can_all = {
                            let x = &self.bloons[self.bloons.iter().position(|o| o.id == b).unwrap()];
                            dseg.is_some_and(|(p, k)| self.seg_span(p as usize, k).is_some_and(|(_, max)| max <= x.dist))
                                && !self.defs[x.kind].tags.iter().any(|t| t == "Bad")
                        };
                        let mut left = g.amount as f32;
                        for x in std::iter::once(b).chain(self.bird_grab(dest, g.radius, &g, st.stamp, &held, b)) {
                            let Some(i) = self.bloons.iter().position(|o| o.id == x) else { continue };
                            let use_ = self.grab_cost(&self.bloons[i], &g) as f32;
                            if x != b && left < use_ {
                                continue;
                            }
                            left -= use_;
                            if x == b {
                                for h in &held {
                                    if let Some(o) = self.bloons.iter().find(|o| o.id == *h) {
                                        left -= self.grab_cost(o, &g) as f32;
                                    }
                                }
                            }
                            // SnatchBloon @0xdc7620.
                            let can = can_all && !self.bird_will_destroy(i, g.initial);
                            if can {
                                let (p, k) = dseg.unwrap();
                                let centre = self.seg_span(p as usize, k).unwrap().0;
                                let n = held.len() as f32;
                                self.bloons[i].path = p;
                                self.bloons[i].dist = centre - (n + n);
                                self.bloons[i].held = true;
                                // All but the first sit 1 off the bird, (0, 1) turned by a NextDouble
                                // x 360 (recording beast_001 rt 404).
                                let off = if n > 0.0 {
                                    let d = self.synced_double() as f32;
                                    let (sa, ca) = crt_sin_cos(DEG2RAD * -(d * 360.0));
                                    [-sa, -ca]
                                } else {
                                    [0.0, 0.0]
                                };
                                st.dists.push((x, centre - (n + n), off));
                                held.push(x);
                                st.target = GyrTarget::Seg(p, k);
                            }
                            if can_all {
                                st.last_grapple = rt;
                            }
                            self.bird_hit(&[x], &g, g.initial, g.initial_moab, owner);
                            tagged.push(x);
                            if left <= 0.0 {
                                break;
                            }
                        }
                        held.retain(|o| self.bloons.iter().any(|x| x.id == *o));
                        // What's next: keepPickingUpBloons flies on to a bloon near the bird that
                        // hasn't passed the segment; with none it carries what it has.
                        let next = if can_all && g.keep && left > 0.0 {
                            self.bird_target(home, range, mode, &g, pos, st.stamp, Some((g.keep_range, dseg)), &held, &tagged)
                        } else {
                            None
                        };
                        if let Some(n) = next {
                            st.target = GyrTarget::Bloon(n.0, n.1);
                        } else if held.is_empty() {
                            self.bird_release(&mut held, &mut st, rt);
                            st.target = GyrTarget::None;
                        } else if let Some((p, k)) = dseg {
                            st.target = GyrTarget::Seg(p, k);
                        }
                    }
                    _ => {}
                }
            }
            let a = self.towers[t].air.as_mut().unwrap();
            a.pos = pos;
            a.height = 40.0;
            a.grab = held;
            a.gy = st;
        }
    }

    /// The attack's AttackFilterModel: FilterOutTag, FilterOutBloon, FilterInvisible and
    /// FilterBloonIfDamageType (ifCantHitBloonProperties, the damage's immunities).
    fn bird_filter(&self, b: &Bloon, g: &crate::data::Grapple) -> bool {
        let d = &self.defs[b.kind];
        !g.exclude_tags.iter().any(|t| d.tags.iter().any(|x| x == t))
            && !g.exclude_bloons.iter().any(|x| *x == d.base_id)
            && !(g.camo_filter && b.camo(d))
            && (g.immune & 1 == 0 || b.properties(d) & 1 == 0)
    }

    fn gyr_at(b: &Bloon, stamp: u64) -> [f32; 2] {
        if b.born > stamp && b.prev[0].is_finite() { b.prev } else { b.pos }
    }

    fn gyr_valid(&self, st: &GyrState) -> bool {
        match st.target {
            GyrTarget::Bloon(b, _) => self.bloons.iter().any(|x| x.id == b),
            GyrTarget::None => false,
            _ => true,
        }
    }

    /// Held bloons sit on the bird and don't move (after this frame's bloon moves).
    fn hold_bird_bloons(&mut self) {
        for t in 0..self.towers.len() {
            let Some(a) = self.towers[t].air.as_ref() else { continue };
            if a.grab.is_empty() {
                continue;
            }
            let (pos, held, dists) = (a.pos, a.grab.clone(), a.gy.dists.clone());
            for h in &held {
                if let Some(b) = self.bloons.iter_mut().find(|b| b.id == *h) {
                    let off = dists.iter().find(|d| d.0 == *h).map_or([0.0; 2], |d| d.2);
                    b.pos = [pos[0] + off[0], pos[1] + off[1]];
                    b.prev = b.pos;
                    if let Some(d) = dists.iter().find(|d| d.0 == *h) {
                        b.dist = d.1;
                    }
                    b.held = true;
                    if !b.effects.iter().any(|e| e.id == "BeastGrab") {
                        b.effects.push(Effect { overlay: Default::default(), seq: 0, kind: EffectKind::Slow { mult: 0.0, glue: 0 }, id: "BeastGrab".into(), frames_left: 2.0, total: 2.0, added: 0, layers: 1, owner: 0, distribute: (true, false), keep: false });
                    }
                }
            }
        }
    }

    fn bird_release(&mut self, held: &mut Vec<u64>, st: &mut GyrState, rt: i64) {
        for h in held.drain(..) {
            if let Some(b) = self.bloons.iter_mut().find(|b| b.id == h) {
                b.effects.retain(|e| e.id != "BeastGrab");
                b.held = false;
                let off = st.dists.iter().find(|d| d.0 == h).map_or([0.0; 2], |d| d.2);
                b.pos = [st.last_pos[0] + off[0], st.last_pos[1] + off[1]];
                b.prev = b.pos;
                if let Some(d) = st.dists.iter().find(|d| d.0 == h) {
                    b.dist = d.1;
                }
                // (A bloon made after the bird is processed after it: its move is this frame.)
                if b.born > st.stamp {
                    let (kind, path, dist) = (b.kind, b.path, b.dist);
                    let step = speed_frames(self.defs[kind].speed) * b.speed_mult(&self.defs[kind]);
                    let d = self.held_at_spawn(kind, path, dist + step);
                    let q = self.tracks[path as usize].bloon_pos(d);
                    let b = self.bloons.iter_mut().find(|b| b.id == h).unwrap();
                    b.dist = d;
                    b.pos = q;
                    b.prev = q;
                }
            }
        }
        st.dists.clear();
        if st.cd_start < 0 {
            st.cd_start = rt;
        }
    }

    /// GetDestinationPathSegment: of Attack.GetPathSegmentsInRange, the largest distanceUntilLeak.
    fn bird_dest_segment(&self, pi: usize) -> Option<(u8, usize)> {
        let range = self.range(pi);
        let p = self.towers[pi].pos;
        let mut best: Option<((u8, usize), f32)> = None;
        let all = self.track_segs.iter().enumerate().flat_map(|(ti, sg)| sg.iter().enumerate().map(move |(k, q)| (ti as u8, k, *q)));
        for (ti, k, q) in all.filter(|e| (e.2[0] - p[0]).powi(2) + (e.2[1] - p[1]).powi(2) < range * range) {
            let _ = q;
            let v = self.seg_until_leak(ti, k);
            if v > best.map_or(0.0, |b| b.1) {
                best = Some(((ti, k), v));
            }
        }
        best.map(|b| b.0)
    }

    /// GetBloonTarget: a bloon the handler's range covers, best by its target type.
    fn bird_target(&self, home: [f32; 2], range: f32, mode: TargetMode, g: &crate::data::Grapple, pos: [f32; 2], stamp: u64, near: Option<(f32, Option<(u8, usize)>)>, held: &[u64], tagged: &[u64]) -> Option<(u64, u64)> {
        let rr = range * range;
        // Targetting.GetTargets' order for the handler's target type (parent tower, its position):
        // by path segment, the first of a segment's bloons in the path's list (recording beast_502).
        let seg_leak = |b: &Bloon| {
            let k = (b.wind_seg.unwrap_or(b.prev_dist) / 7.0).max(0.0) as usize;
            self.seg_until_leak(b.path, k)
        };
        // Targetting.GetGlobalBest @0xffa100: Path.bestFirst / bestLast (FindBestBloon, as they were
        // filed: the distance at the frame's start) before the segments.
        let best = match mode {
            TargetMode::First => self.bloons.iter().filter(|b| b.wind.is_none() && !b.held).max_by(|x, y| x.prev_dist.total_cmp(&y.prev_dist).then_with(|| self.path_rank.get(&y.id).map(|r| r.1).cmp(&self.path_rank.get(&x.id).map(|r| r.1)))).map(|b| b.id),
            TargetMode::Last => self.bloons.iter().filter(|b| b.wind.is_none() && !b.held).min_by(|x, y| x.prev_dist.total_cmp(&y.prev_dist).then_with(|| self.path_rank.get(&x.id).map(|r| r.1).cmp(&self.path_rank.get(&y.id).map(|r| r.1)))).map(|b| b.id),
            _ => None,
        };
        let key = |b: &Bloon| if Some(b.id) == best { f32::MIN } else { match mode {
            TargetMode::Last => -seg_leak(b),
            // (BloonManager.CompareBracketClose @0xfc9650: its segment's point.)
            TargetMode::Close => {
                let k = (b.wind_seg.unwrap_or(b.prev_dist) / 7.0).max(0.0) as usize;
                self.track_segs.get(b.path as usize).and_then(|s| s.get(k)).map_or(f32::MAX, |q| (q[0] - home[0]).powi(2) + (q[1] - home[1]).powi(2))
            }
            TargetMode::Strong => -(self.rbe[b.kind] as f32 * 1e4 + b.progress()),
            _ => seg_leak(b),
        } };
        self.bloons
            .iter()
            .filter(|b| { let q = Self::gyr_at(b, stamp); (q[0] - home[0]).powi(2) + (q[1] - home[1]).powi(2) <= rr })
            .filter(|b| !self.towers.iter().any(|tw| tw.air.as_ref().is_some_and(|a| a.grab.contains(&b.id))))
            .filter(|b| self.grab_cost(b, g) <= g.amount)
            // (A bloon made after the bird isn't a target until it has moved: recording beast_502.)
            .filter(|b| b.born <= stamp || b.born / 4 + 1 <= self.sim_elapsed || (b.born % 4 == 0 && b.lineage.is_some()))
            .filter(|b| self.bird_filter(b, g))
            .filter(|b| !held.contains(&b.id) && !tagged.contains(&b.id))
            // TargetCheck @0xdc7a30: not before the segment, within the range of the bird.
            .filter(|b| near.is_none_or(|(r, seg)| {
                seg.is_none_or(|(p, k)| self.seg_span(p as usize, k).is_none_or(|(_, max)| max <= b.dist))
                    && (b.pos[0] - pos[0]).powi(2) + (b.pos[1] - pos[1]).powi(2) <= r * r
            }))
            // (On one segment, the first in the path's list.)
            .min_by(|x, y| key(x).total_cmp(&key(y)).then_with(|| self.path_rank.get(&x.id).map(|r| r.1).cmp(&self.path_rank.get(&y.id).map(|r| r.1))))
            .map(|b| (b.id, b.layer))
    }

    fn bird_will_destroy(&self, i: usize, dmg: f32) -> bool {
        // Bloon.WillDestroyBloon: the damage takes all its layers.
        let mut k = self.bloons[i].kind;
        let mut total = self.bloons[i].hp as f32;
        while let Some(&c) = self.child_kinds[k].first() {
            total += self.defs[c].max_health as f32;
            k = c;
        }
        total - dmg < 1.0
    }

    /// DamageBasedAttackSpeedModel. HYPOTHESIS P-RAGE: damage dealt = layers the tower popped.
    fn update_rage(&mut self) {
        for t in 0..self.towers.len() {
            let Some(ds) = self.tier(t).and_then(|tr| tr.damage_speed.clone()) else { continue };
            let tw = &mut self.towers[t];
            let (last, acc, stacks, idle) = &mut tw.rage;
            let dealt = tw.pops.saturating_sub(*last);
            *last = tw.pops;
            if dealt == 0 {
                *idle += 1;
                if *idle > ds.idle_frames {
                    *stacks = 0;
                    *acc = 0.0;
                }
                continue;
            }
            *idle = 0;
            *acc += dealt as f32;
            while *acc >= ds.threshold.max(1.0) && *stacks < ds.max_stacks {
                *acc -= ds.threshold.max(1.0);
                *stacks += 1;
            }
        }
    }

    /// Toggles a Skywarden between Spread and Focus stance.
    pub fn toggle_stance(&mut self, t: usize) -> bool {
        if self.tier(t).is_some_and(|tr| tr.stance.is_some()) {
            self.towers[t].focus = !self.towers[t].focus;
            return true;
        }
        false
    }

    /// Grab cost of bloon `b` for a bird: 1 + the penalty of its type. HYPOTHESIS P-BIRDBAD:
    /// BADs (no penalty field) can't be grabbed.
    fn grab_cost(&self, b: &Bloon, g: &crate::data::Grapple) -> u32 {
        // GetPierceUsage (VERIFIED-DISASM @0xdc4220): 1, plus each penalty whose name its id has.
        let id = &self.defs[b.kind].base_id;
        1 + ["Lead", "Ceramic", "Moab", "Bfb", "Zomg", "Ddt"].iter().filter(|k| id.contains(**k)).filter_map(|k| g.penalty.get(*k)).sum::<u32>()
    }

    /// The bloons a bird takes along (Targetting.GetTargets, VERIFIED-DISASM @0xffa980): those on
    /// the path segments whose point is within radius + 2 of `p` (recording beast_002: a Yellow
    /// on such a segment 26 away, not one 17 away on another), that the attack's filters pass.
    fn bird_grab(&self, p: [f32; 2], radius: f32, g: &crate::data::Grapple, stamp: u64, held: &[u64], skip: u64) -> Vec<u64> {
        let reach = radius + 2.0;
        let mut out: Vec<(u64, (f32, u64))> = Vec::new();
        for b in &self.bloons {
            // (One made after the bird this frame isn't there yet: as bird_target.)
            if !self.bird_filter(b, g) || b.id == skip || held.contains(&b.id) || !(b.born <= stamp || b.born / 4 + 1 <= self.sim_elapsed || (b.born % 4 == 0 && b.lineage.is_some())) {
                continue;
            }
            let k = (b.wind_seg.unwrap_or(b.prev_dist) / 7.0).max(0.0) as usize;
            let Some(q) = self.track_segs.get(b.path as usize).and_then(|s| s.get(k)) else { continue };
            if (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) <= reach * reach {
                out.push((b.id, ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2), self.path_rank.get(&b.id).map_or(u64::MAX, |r| r.1 as u64))));
            }
        }
        out.sort_by(|x, y| x.1.0.total_cmp(&y.1.0).then(x.1.1.cmp(&y.1.1)));
        out.into_iter().map(|e| e.0).collect()
    }

    /// A child SpawnChildren was told not to make (Bloon.blockSpawnChildrenCount; damage result
    /// VERIFIED-DISASM BloonDegradeStepper.UpdateDamageResult @0xfc8a50: the first step only, the
    /// others only added up): no object, no place in the lists.
    fn unmake_bloon(&mut self, j: usize) {
        let (id, o) = (self.bloons[j].id, self.bloons[j].obj);
        if let Some(&k) = self.fact_at.get(o as usize).filter(|&&k| k != u32::MAX) {
            self.fact.remove(k as usize);
            for e in k as usize..self.fact.len() {
                self.fact_at[self.fact[e].0 as usize] = e as u32;
            }
            self.fact_at[o as usize] = u32::MAX;
            self.pool.push(o);
        }
        if let Some((p, k)) = self.path_rank.remove(&id) {
            let l = &mut self.path_lists[p as usize];
            l.remove(k as usize);
            for (n, e) in l.iter().enumerate().skip(k as usize) {
                self.path_rank.insert(e.0, (p, n as u32));
            }
        }
        if self.grid_on {
            self.grid.swap_remove(j, self.bloons.len());
            if self.has_grid_rank {
                self.ggrid.swap_remove(j, self.bloons.len());
            }
        }
        self.bloons.swap_remove(j);
    }

    /// Hits the grabbed bloons for `dmg` (+`moab` on MOAB-class, +regrowDamageModifier on regrow).
    fn bird_hit(&mut self, ids: &[u64], g: &crate::data::Grapple, dmg: f32, moab: f32, owner: u64) {
        let mut kids = Vec::new();
        for id in ids {
            kids.clear();
            let Some(i) = self.bloons.iter().position(|b| b.id == *id) else { continue };
            let tags = &self.defs[self.bloons[i].kind].tags;
            let extra = if tags.iter().any(|t| t == "Moabs") { moab } else { 0.0 } + if tags.iter().any(|t| t == "Grow") { g.regrow_add } else { 0.0 };
            let tally0 = self.damage_tally;
            let n = self.damage(i, (dmg + extra).round() as i32, &mut kids);
            let self_tally_dealt = self.damage_tally - tally0;
            if let Some(tw) = self.towers.iter_mut().find(|t| t.id == owner) {
                tw.pops += n;
                tw.sun_destroyed(self.last_destroyed);
                tw.damage += self_tally_dealt;
                self.stats.pops += n;
                self.stats.damage += self_tally_dealt;
            }
        }
    }

    /// PathMovement.Process (VERIFIED-DISASM @0xd777c0): the path parameter advances at
    /// speed / path length a frame. Off its path (just made, or the mode changed) the plane turns
    /// toward its path point at most `rotation` degrees a second and flies forward at speed x
    /// catchUpSpeed; it rejoins once the point is within one such step and under 25 degrees
    /// off its nose. On the path it sits on the point, facing the way it moved. Bank: eases
    /// (LerpAngle 0.07) toward the turn x bankRotationMul x bankRotation, capped at
    /// bankRotation. Take-off (TakeOffAnimation @0xd78870): over takeOffAnimTime, with
    /// x = tt^exponent, scale = takeOffScale + (1 - takeOffScale)(sin((x - 1/4)pi) + 1)/2 and
    /// pitch = takeOffPitch (sin((x - 1/4)2pi) + 1)/2. STAND-IN P-ACEPATH: the pattern shapes;
    /// P-TAKEOFF: the height climbs over takeOffTime as 1 - (1 - s)^takeOffExponent.
    fn fly_pattern(&mut self, t: usize, spec: &crate::data::AirSpec) {
        let centre = [(self.bounds.0[0] + self.bounds.1[0]) / 2.0, (self.bounds.0[1] + self.bounds.1[1]) / 2.0];
        let tw = &mut self.towers[t];
        let pat = spec.patterns.iter().find(|p| TargetMode::parse(&p.mode) == Some(tw.target)).unwrap_or(&spec.patterns[0]);
        let mode = TargetMode::parse(&pat.mode).unwrap_or(TargetMode::Circle);
        let c = if mode == TargetMode::Centered { *tw.point.get_or_insert(centre) } else { tw.pos };
        let a = tw.air.as_mut().unwrap();
        let dt = 1.0 / FPS;
        let p0 = pattern_point(mode, pat.rx, pat.ry, c, a.phase);
        let p1 = pattern_point(mode, pat.rx, pat.ry, c, a.phase + 1e-3);
        let dl = (p1[0] - p0[0]).hypot(p1[1] - p0[1]) / 1e-3;
        // TSMThemeMonkeyAce's reverse button flies the pattern the other way round.
        let step = spec.speed * dt / dl.max(1e-3);
        a.phase = (a.phase + if tw.air_reverse { -step } else { step }).rem_euclid(std::f32::consts::TAU);
        let goal = pattern_point(mode, pat.rx, pat.ry, c, a.phase);
        let d = [goal[0] - a.pos[0], goal[1] - a.pos[1]];
        let want = d[1].atan2(d[0]).to_degrees();
        let rot = a.angle.to_degrees();
        let old = a.pos;
        let new_rot = if a.on_path {
            a.pos = goal;
            if d[0].hypot(d[1]) > 1e-4 { want } else { rot }
        } else {
            let max = spec.turn * dt;
            let r = rot + angle_diff(want.to_radians(), rot.to_radians()).to_degrees().clamp(-max, max);
            let step = spec.speed * spec.catch_up.max(1.0) * dt;
            let (s, co) = r.to_radians().sin_cos();
            a.pos = [a.pos[0] + co * step, a.pos[1] + s * step];
            if step * step >= d[0] * d[0] + d[1] * d[1] && angle_diff(want.to_radians(), r.to_radians()).to_degrees().abs() < 25.0 {
                a.on_path = true;
            }
            r
        };
        let goal_bank = (angle_diff(new_rot.to_radians(), rot.to_radians()).to_degrees() * spec.bank_mul * spec.bank).clamp(-spec.bank, spec.bank);
        a.bank += angle_diff(goal_bank.to_radians(), a.bank.to_radians()).to_degrees() * 0.07;
        a.angle = new_rot.to_radians();
        a.vel = [(a.pos[0] - old[0]) * FPS, (a.pos[1] - old[1]) * FPS];
        a.age += dt;
        if let Some(to) = &spec.takeoff {
            if to.anim > 0.0 && a.tt < 1.0 {
                a.tt = (a.tt + dt / to.anim).min(1.0);
                let pi = std::f32::consts::PI;
                let x = a.tt.powf(to.scale_exp);
                a.scale = (((x - 0.25) * pi).sin() + 1.0) * 0.5 * (1.0 - to.scale) + to.scale;
                let y = a.tt.powf(to.pitch_exp);
                a.pitch = (((y - 0.25) * 2.0 * pi).sin() + 1.0) * 0.5 * to.pitch;
            }
            if to.time > 0.0 {
                let s = (a.age / to.time).min(1.0);
                a.height = 40.0 * (1.0 - (1.0 - s).powf(to.exp.max(1.0)));
            }
        }
        tw.angle = a.angle;
    }

    /// A sub-tower (sentry) made by tower `parent` at `p`. It counts as the parent's tower type
    /// for buffs (STAND-IN).
    fn spawn_sub(&mut self, spec: std::sync::Arc<crate::data::SubTowerSpec>, p: [f32; 2], parent: u64, pet: bool) {
        let Some(def) = self.towers.iter().find(|t| t.id == parent).map(|t| t.def) else { return };
        let id = self.next_id;
        self.next_id += 1;
        let place = spec.tier.sounds.place.clone();
        if let Some((Some(fx), k)) = spec.place_fx.clone() {
            self.place_fx.push((p, fx, k));
        }
        // RandomPosition's OnTowerCreated: calcPositionsNextFrame.
        for tw in &mut self.towers {
            for (_, rp) in &mut tw.randpos {
                rp.dirty = true;
            }
        }
        self.towers.push(Tower {
            id,
            pops: 0,
            damage: 0,
            placed_round: self.round + 1,
            cash_earned: 0.0,
            lives_gained: 0.0,
            def,
            pos: p,
            tiers: [0; 3],
            invested: 0.0,
            target: TargetMode::First,
            point: None,
            point_z: 0.0,
            weapons: Vec::new(),
            shots: 0,
            angle: -std::f32::consts::FRAC_PI_2,
            aim: -std::f32::consts::FRAC_PI_2,
            game_rot: None,
            spin: Vec::new(),
            randpos: Vec::new(),
            base_rot: 0.0,
            sub_base: None,
            turret_rel: 0.0,
            base_state: (false, 0, false, 0.0),
            aims: Vec::new(),
            attack_t: f32::MAX,
            idle_t: 0.0,
            bank: 0.0,
            brews: Vec::new(),
            sub: Some(SubTower { frames_left: if spec.lifespan > 0.0 { spec.lifespan * FPS } else { f32::INFINITY }, age: 0.0, rounds_left: spec.rounds, pet, spec, parent, path_index: None, z: 250.0, born: self.round_clock }),
            air: None,
            ability_cd: HashMap::new(),
            ability_round: HashMap::new(),
            temp: Vec::new(),
            turbo: None,
            fan: None,
            morph: None,
            auto_t: 0.0,
            focus: false,
            look: None,
            overclock: None,
            range_up: None,
            ultra: (0, 1.0, 1.0),
            mute: 0.0,
            rage: (0, 0.0, 0, 0),
            temp_supports: Vec::new(),
            ammo: None,
            xp: 0.0,
            deck: None,
            items: Vec::new(),
            tier_cap: 5,
            boosts: Vec::new(),
            paragon: None,
            paragon_active_at: 0,
            paragon_first: 0,
            paragon_kept: Vec::new(),
            attack_born: HashMap::new(),
            orbit: None,
            temple: None,
            show: None,
            attacking_fx: false,
            had_target: HashMap::new(),
            mtp: None,
            grave: Vec::new(),
            necro_points: false,
            model_clock: Some((self.sim_elapsed as i64, self.round_clock)),
            placed_step: self.steps as u64,
            track_buffs: 0,
            track_args: None,
            grave_spent: (0.0, 0),
            necro_cur: 0,
            necro_next: 0,
            necro_sel: None,
            necro_seg: None,
            necro_emit: None,
            necro_dmg: 0.0,
            track_supply: None,
            comanche: (0.0, 0),
            vigilante_at: None,
            vagrant_zone: None,
            left_target: TargetMode::First,
            camo_prio: false,
            submerged: false,
            air_reverse: false,
            reverse_at: 0,
            flip: false,
        });
        let n = self.towers.len() - 1;
        // Its Attacks are in the process list from when it is made, mid-frame (recording
        // wizard_040: the Phoenix; engineer_100: a Sentry aims where its Pink is after the move).
        {
            // (A pet: BeastHandlerLeash.Process makes it on the handler's next frame, after that
            // frame's Spawner: recording beast_010, its Splash on a Black where it is after its move.)
            let is_pet = self.towers[n].sub.as_ref().is_some_and(|s| s.pet);
            let stamp = self.sim_elapsed * 4 + self.phase as u64;
            let names: Vec<String> = self.towers[n].sub.as_ref().map(|s| s.spec.tier.attacks.iter().map(|a| a.model_name.clone()).collect()).unwrap_or_default();
            for nm in names.into_iter().filter(|_| !is_pet) {
                self.towers[n].attack_born.insert(nm, stamp);
            }
        }
        // A free-flying sub-tower without movement modes (Etienne's drones) chases bloons.
        // STAND-IN P-DRONE: Pursuit (the game's drones follow Etienne's targeting).
        if self.towers[n].sub.as_ref().is_some_and(|s| s.spec.tier.air.as_ref().is_some_and(|a| a.patterns.is_empty()) && s.spec.tier.air_modes.is_empty() && s.spec.tier.grapple.is_none()) {
            self.towers[n].target = TargetMode::Pursuit;
        }
        self.init_air(n);
        self.sound(&place, None);
    }

    /// The map has an area of this type (water = 1).
    /// Some track passes within `r` of `c`.
    fn track_within(&self, c: [f32; 2], r: f32) -> bool {
        self.tracks.iter().any(|tr| {
            tr.pts.windows(2).any(|w| {
                let (a, b) = (w[0], w[1]);
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let len2 = dx * dx + dy * dy;
                let k = if len2 > 0.0 { (((c[0] - a[0]) * dx + (c[1] - a[1]) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
                let (qx, qy) = (a[0] + dx * k - c[0], a[1] + dy * k - c[1]);
                qx * qx + qy * qy <= r * r
            })
        })
    }

    /// Where a pet goes. HYPOTHESIS P-PETSPOT: inside the handler's leash range (the tier's
    /// range); a land pet takes the valid land spot nearest the track, a water pet the nearest
    /// water; a pet with no display of its own (Gyrfalcon) stays at the handler.
    fn pet_spot(&self, t: usize, pet: &crate::data::SubTowerSpec) -> Option<[f32; 2]> {
        let h = self.towers[t].pos;
        let r = self.tier(t).map_or(40.0, |x| x.range);
        // (TowerCreateTowerModel's Phoenix starts on its tower: recording wizard_050.)
        if (pet.base.is_none() && pet.model.is_none()) || pet.circle.is_some() {
            return Some(h);
        }
        let water = pet.areas.contains(&1) && !pet.areas.contains(&2);
        let ok = |q: [f32; 2]| {
            if !on_map(q, -6.0) {
                return false;
            }
            let inside = |kind: i32| self.areas.iter().any(|a| a.kind == kind && Self::in_area(q, a));
            if water {
                inside(1)
            } else {
                !self.areas.iter().any(|a| a.kind != AREA_LAND && a.kind != AREA_SKIPPED && Self::in_area(q, a))
            }
        };
        let track_d = |q: [f32; 2]| {
            let mut best = f32::MAX;
            for tr in &self.tracks {
                let mut d = 0.0;
                while d < tr.length() {
                    let p = tr.pos(d);
                    best = best.min((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2));
                    d += 8.0;
                }
            }
            best
        };
        let mut best: Option<([f32; 2], f32)> = None;
        for ring in 1..=8 {
            let rr = r * ring as f32 / 8.0;
            for k in 0..24 {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                let q = [h[0] + rr * a.cos(), h[1] + rr * a.sin()];
                if ok(q) {
                    let d = track_d(q);
                    if best.is_none_or(|(_, b)| d < b) {
                        best = Some((q, d));
                    }
                }
            }
        }
        best.map(|(q, _)| q)
    }

    /// Replaces tower `t`'s pets with its current tier's (Beast Handler).
    fn sync_pets(&mut self, t: usize) {
        let id = self.towers[t].id;
        let pets = self.tier(t).map(|x| x.pets.clone()).unwrap_or_default();
        let had = self.towers.iter().any(|o| o.sub.as_ref().is_some_and(|s| s.pet && s.parent == id));
        if pets.is_empty() && !had {
            return;
        }
        // Pets are spawned after their handler, so removing them never shifts `t`.
        for i in (0..self.towers.len()).rev() {
            if self.towers[i].sub.as_ref().is_some_and(|s| s.pet && s.parent == id) {
                self.towers.remove(i);
                self.removed.push((i, usize::MAX));
            }
        }
        for pet in pets {
            // BeastHandlerLeash.ProcessLeashedTower (VERIFIED-DISASM @0xcffd80): the pet is made at
            // its spawner attack's Target, a RandomPosition (recording beast_010: at (-61.9, -54.22)).
            let spawner = self.tier(t).and_then(|x| x.attacks.iter().find(|a| a.model_name.contains("PetSpawnerAttack") && a.model_name.contains(pet.name.as_str())).and_then(|a| a.random_pos.map(|m| (a.model_name.clone(), m))));
            let spot = match spawner {
                Some((key, m)) => self.random_pos_target(t, &key, m),
                None => self.pet_spot(t, &pet),
            };
            if let Some(q) = spot {
                let circling = pet.circle.is_some();
                self.spawn_sub(pet, q, id, true);
                if circling {
                    if let Some(s) = self.towers.last_mut().and_then(|tw| tw.sub.as_mut()) {
                        s.age = -1.0;
                    }
                }
            }
        }
    }

    /// PathMovementFromScreenCenter.Process (VERIFIED-DISASM @0xd578d0; the path from
    /// PathMovementFromScreenCenterPattern.GetPath @0xdcd640): 360 points round the screen
    /// centre (0, 0), one a degree (Math.PointOnCircleCircumference). It starts heading for the
    /// closest (GetClosestPathIndex @0xd57650); its squared distance under speedFrames x 5, the
    /// next one;
    /// Vector3.MoveTowards it by speedFrames; rotation = Math.LerpAngle(rotation, the move's
    /// Vector2.Rotation, 0.1) (recording wizard_050: the Phoenix leaves the tower 0.833 a frame,
    /// rotation 7.42, 14.10, 20.11 ...).
    /// HYPOTHESIS P-PHOENIXCIRCLE: point i is at i degrees (cos, sin) in game coordinates.
    fn circle_subs(&mut self) {
        for tw in &mut self.towers {
            let Some(s) = &mut tw.sub else { continue };
            let Some(ci) = s.spec.circle.clone() else { continue };
            // (Made this frame: its behaviours run from the next one. HYPOTHESIS P-PETFRAME: a
            // TowerCreateTowerModel tower is made in its parent's first frame, age -1 here;
            // recording wizard_050: the Phoenix still on the tower at rt 1, the summoned one
            // turning on the frame after the ability.)
            if s.age < 1.0 {
                continue;
            }
            // Math.PointOnCircleCircumference (VERIFIED-DISASM @0xf20770): (cos, sin) of
            // angle x Deg2Rad, times the radius.
            let point = |i: usize| {
                let (sa, ca) = crt_sin_cos(i as f32 * DEG2RAD);
                [ca * ci.radius, -(sa * ci.radius)]
            };
            let step = ci.speed * FRAME_TIME;
            let d = |p: [f32; 2], q: [f32; 2]| ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt();
            let mut idx = match s.path_index {
                Some(i) => i,
                None => (0..360).fold((0, f32::MAX), |(bi, bd), i| {
                    let dd = d(tw.pos, point(i));
                    if dd < bd { (i, dd) } else { (bi, bd) }
                }).0,
            };
            // (Vector2.DistanceSquared (@0xf45660, unfused) against speedFrames x 5: recording wizard_050 keeps the
            // point at 2.76 away and moves on at 1.97.)
            let (ex, ey) = (tw.pos[0] - point(idx)[0], tw.pos[1] - point(idx)[1]);
            if ex * ex + ey * ey < step * 5.0 {
                idx = (idx + 1) % 360;
            }
            s.path_index = Some(idx);
            let to = point(idx);
            // Vector3.MoveTowards (VERIFIED-DISASM @0xf47cf0) to (point, 250) by speedFrames
            // (speed x 1/60 in f32, the model's .ctor @0x1225980): dx² + dy² + dz² unfused, sqrt
            // in double, (d / length) x maxDelta + current per component.
            let (dx, dy, dz) = (to[0] - tw.pos[0], to[1] - tw.pos[1], 250.0 - s.z);
            let sq = dx * dx + dy * dy + dz * dz;
            let next = if sq == 0.0 || step * step >= sq {
                s.z = 250.0;
                to
            } else {
                let len = (sq as f64).sqrt() as f32;
                s.z = dz / len * step + s.z;
                [dx / len * step + tw.pos[0], dy / len * step + tw.pos[1]]
            };
            let (dx, dy) = (next[0] - tw.pos[0], next[1] - tw.pos[1]);
            let want = RAD2DEG * crt_atan2(dx, -dy);
            let cur = tw.game_rot.unwrap_or(0.0);
            // Mathf.LerpAngle: num = Repeat(b - a, 360) (t - Floor(t / 360) x 360, clamped to
            // [0, 360]), less 360 past 180, then a + num x t, in f32.
            let t0 = want - cur;
            let mut num = (t0 - (t0 / 360.0).floor() * 360.0).clamp(0.0, 360.0);
            if num > 180.0 {
                num -= 360.0;
            }
            let rot = cur + num * 0.1;
            tw.game_rot = Some(rot);
            // (Its facing is the rotation: game degrees from +y, our y flipped.)
            tw.angle = (rot - 90.0).to_radians();
            tw.pos = next;
        }
        // OrbitingTowerModel: round the parent, siblings of one kind spread evenly.
        // HYPOTHESIS P-ORBITTOWER: counter-clockwise from angle 0 when made.
        let parents: HashMap<u64, [f32; 2]> = self.towers.iter().map(|t| (t.id, t.pos)).collect();
        let mut seen: HashMap<(u64, *const crate::data::SubTowerSpec), u32> = HashMap::new();
        let counts = {
            let mut c: HashMap<(u64, *const crate::data::SubTowerSpec), u32> = HashMap::new();
            for tw in &self.towers {
                if let Some(s) = tw.sub.as_ref().filter(|s| s.spec.orbit_tower.is_some()) {
                    *c.entry((s.parent, std::sync::Arc::as_ptr(&s.spec))).or_default() += 1;
                }
            }
            c
        };
        for tw in &mut self.towers {
            let Some(s) = &tw.sub else { continue };
            let Some(ob) = &s.spec.orbit_tower else { continue };
            let Some(&c) = parents.get(&s.parent) else { continue };
            let key = (s.parent, std::sync::Arc::as_ptr(&s.spec));
            let k = seen.entry(key).or_default();
            let n = counts.get(&key).copied().unwrap_or(1).max(1);
            let age = s.age;
            let a = (age * ob.rate).to_radians() + *k as f32 * std::f32::consts::TAU / n as f32;
            *k += 1;
            let next = [c[0] + ob.radius * a.cos(), c[1] + ob.radius * a.sin()];
            tw.angle = a + std::f32::consts::FRAC_PI_2;
            tw.pos = next;
        }
    }

    /// Whether attack `a`'s first weapon (state `wi`) could throw now: off cooldown and not held
    /// by LimitProjectile (its count of the tower's projectiles, or limitEmissionUntilTime).
    fn can_throw(&self, t: usize, wi: usize, a: &crate::data::Attack) -> bool {
        let (Some(w), Some(st)) = (a.weapons.first(), self.towers[t].weapons.get(wi)) else { return true };
        let clock = if w.between_rounds { self.sim_elapsed as i64 } else { self.round_clock };
        if !st.ready(clock) {
            return false;
        }
        let Some(n) = w.limit else { return true };
        let id = self.towers[t].id;
        let out = self.projectiles.iter().filter(|pr| pr.owner == id && (if w.limit_id.is_empty() { pr.spec.eat.is_some() } else { pr.spec.pid == w.limit_id }) && !(pr.spec.eat.is_some() && pr.spec.cash.is_some())).count();
        out < n as usize && (self.sim_elapsed as i64) >= st.limit_until
    }

    /// CreateTypedTower.CalculateTowerModel (VERIFIED-DISASM @0xcb06a0, scores @0xcb00d0 /
    /// @0xcafba0 / @0xcafde0 / @0xcb0290): from the share of each bloon baseId among all bloons
    /// (BloonManager.GetProportionOfTotalBloonCountByBaseId) and the Sentries of each type out,
    /// the highest score (strictly, from -99999) of crushing, boom, cold, energy; each score the
    /// float Average of its terms (Enumerable.Average: a double sum) - 0.01 per own Sentry.
    fn typed_tower_choice(&self) -> usize {
        let total = self.bloons.len();
        let count = |id: &str| self.bloons.iter().filter(|b| self.defs[b.kind].base_id == id).count();
        let prop = |id: &str| if total > 0 { count(id) as f32 / total as f32 } else { 0.0 };
        let sentries = |id: &str| self.towers.iter().filter(|t| t.sub.as_ref().is_some_and(|s| s.spec.id == id)).count() as i32;
        let (pk, bk, wh, ld, pu, zb, rb, ce) = (prop("Pink"), prop("Black"), prop("White"), prop("Lead"), prop("Purple"), prop("Zebra"), prop("Rainbow"), prop("Ceramic"));
        let (mo, bfb, zomg, ddt, bad) = (prop("Moab"), prop("Bfb"), prop("Zomg"), prop("Ddt"), prop("Bad"));
        let (n_pink, n_cer, n_moab) = (count("Pink") as i32, count("Ceramic") as i32, count("Moab") as i32);
        let (crush, boom, cold, energy) = (sentries("SentryCrushing"), sentries("SentryBoom"), sentries("SentryCold"), sentries("SentryEnergy"));
        let avg = |v: &[f32]| (v.iter().map(|&x| x as f64).sum::<f64>() / v.len() as f64) as f32;
        let k = -0.01f32;
        let mut l = vec![ce, pu];
        if crush == 0 && n_cer > 14 {
            l.push(1.5);
        }
        let s_crush = avg(&l) + crush as f32 * k;
        let mut l = vec![ld, bk * -1.0, mo * -0.5];
        if boom < cold && n_pink > 19 {
            l.push(1.5);
        }
        let s_boom = avg(&l) + boom as f32 * k;
        let mut l = vec![pk, pu, ce, ld * -1.0, wh * -1.0];
        if cold < boom && n_pink > 19 {
            l.push(1.5);
        }
        let s_cold = avg(&l) + cold as f32 * k;
        let mut l = vec![zb, rb, mo, bfb + bfb, zomg * 4.5, ddt * 2.5, bad * 10.0, pu * -1.0];
        if n_moab > 9 && (cold == 0 || crush == 0) {
            l.push(-4.0);
        }
        let s_energy = avg(&l) + energy as f32 * k;
        let mut best = (0, -99999.0f32);
        for (i, sc) in [s_crush, s_boom, s_cold, s_energy].into_iter().enumerate() {
            if best.1 < sc {
                best = (i, sc);
            }
        }
        best.0
    }

    /// CreateProjectileOnTowerDestroy.OnExpire / CreateProjectile (VERIFIED-DISASM @0xd15050 /
    /// @0xd14910): its emission fires at the tower, owned by its parent; with useTowerRotation
    /// each shot takes the tower's rotation (recording engineer_520: a Sentry Champion sentry's
    /// BeforeExplosion at its base rotation -114.71555). `sold`: setAgeZeroOnSell, so the fuse
    /// (BeforeExplosion's AgeModel) is already out and it blows up at once.
    fn fire_on_destroy(&mut self, t: usize, sold: bool) {
        let Some(sub) = self.towers[t].sub.as_ref() else { return };
        let (spec, parent) = (sub.spec.clone(), sub.parent);
        let (pos, rot, tiers, id) = (self.towers[t].pos, self.towers[t].base_rot, self.towers[t].tiers, self.towers[t].id);
        for od in &spec.on_destroy {
            let start = self.projectiles.len();
            self.emit_rot = Some(rot);
            self.emit(pos, (rot - 90.0) * DEG2RAD, &od.emission, &od.projectile, None, tiers, if parent != 0 { parent } else { id });
            self.emit_rot = None;
            for pr in &mut self.projectiles[start..] {
                if od.use_rot {
                    pr.grot = Some(rot);
                }
                if sold {
                    pr.frames_left = 0.0;
                }
            }
        }
    }

    /// Removes expired sub-towers (indices go to `removed`).
    fn expire_subs(&mut self, round_end: bool) {
        let mut gone = Vec::new();
        for t in 0..self.towers.len() {
            let Some(s) = &mut self.towers[t].sub else { continue };
            if !round_end {
                s.frames_left -= 1.0;
                s.age += 1.0;
            }
            let mut rounds_up = false;
            if round_end && s.rounds_left > 0 {
                s.rounds_left -= 1;
                rounds_up = s.rounds_left == 0;
            }
            // TowerExpire.Process (VERIFIED-DISASM @0xda7f50): gone once lifespanFrames
            // ((int)(lifespan x 60)) <= round time - the round time it was made at (recording
            // engineer_520: a Sentry made at 10 goes, firing its BeforeExplosion, at 1510, though
            // a frame between rounds came in between).
            let timed_out = s.spec.lifespan > 0.0 && ((s.spec.lifespan * 60.0) as i64) <= self.round_clock - s.born;
            if timed_out || (round_end && s.spec.round_end) || rounds_up {
                if let Some((Some(fx), k)) = s.spec.expire_fx.clone() {
                    let p = self.towers[t].pos;
                    self.place_fx.push((p, fx, k));
                }
                gone.push(t);
            }
        }
        for &t in &gone {
            self.fire_on_destroy(t, false);
        }
        for t in gone.into_iter().rev() {
            {
                // RootObject.Destroy (VERIFIED-DISASM @0xf414c0) takes it off the Simulation's
                // process list with an ordered RemoveAt (@0x1d996a0), so the towers keep processing
                // in creation order (recording engineer_300 rt 2771: Sentry 1708 fires before
                // 1741). (The factory list a recording walks swap-removes it ~30 frames later.)
                self.towers.remove(t);
                self.removed.push((t, usize::MAX));
            }
        }
    }

    /// Targeting modes tower `t` can switch between, in Target*Model order over its attacks.
    /// The largest SubmergeModel scale over submerged Subs: `pick` gives (in range, map-wide).
    /// STAND-IN P-SUBMERGE: in range means within the Sub's range of `p`, scales don't stack.
    fn submerged_scale(&self, p: [f32; 2], pick: impl Fn(&crate::data::Submerge) -> (f32, f32)) -> f32 {
        let mut best = 1.0f32;
        for (s, tw) in self.towers.iter().enumerate() {
            if !tw.submerged {
                continue;
            }
            let Some(sm) = self.tier_of(tw).and_then(|tr| tr.submerge.as_ref()) else { continue };
            let (near, global) = pick(sm);
            let r = self.range(s);
            let in_range = (tw.pos[0] - p[0]).powi(2) + (tw.pos[1] - p[1]).powi(2) <= r * r;
            best = best.max(global);
            if in_range {
                best = best.max(near);
            }
        }
        best
    }

    pub fn target_modes(&self, t: usize) -> Vec<TargetMode> {
        let mut out = Vec::new();
        if let Some(tr) = self.tier(t).filter(|tr| tr.air.is_some()) {
            return tr.air_modes.iter().filter_map(|s| TargetMode::parse(s)).collect();
        }
        let attacks = self.tier(t).map(|tr| tr.attacks.as_slice()).unwrap_or_default();
        let all_point = attacks.iter().all(|a| a.targets.is_empty() || side_point(a));
        for a in attacks.iter().filter(|a| all_point || !side_point(a)) {
            for m in a.targets.iter().filter_map(|s| TargetMode::parse(s)) {
                if !out.contains(&m) {
                    out.push(m);
                }
            }
        }
        out
    }

    /// Tower `t` can submerge (SubmergeModel): the Sub's upgrade panel has its button.
    pub fn can_submerge(&self, t: usize) -> bool {
        self.tier(t).is_some_and(|tr| tr.submerge.is_some())
    }

    /// Tower `t` has an off-hand attack (TargetLeftHandModel): the panel shows a target row
    /// per hand.
    pub fn has_hands(&self, t: usize) -> bool {
        self.tier(t).is_some_and(|tr| tr.attacks.iter().any(|a| a.hand.as_deref() == Some("left")))
    }

    /// The off hand's mode after (or before) its current one, from the main hand's modes.
    pub fn cycle_left_target(&self, t: usize, back: bool) -> TargetMode {
        let modes = self.target_modes(t);
        let cur = self.towers[t].left_target;
        let n = modes.len();
        match modes.iter().position(|&m| m == cur) {
            Some(i) if n > 0 => modes[if back { (i + n - 1) % n } else { (i + 1) % n }],
            _ => modes.first().copied().unwrap_or(cur),
        }
    }

    /// The mode after (or before, `back`) tower `t`'s current one.
    pub fn cycle_target(&self, t: usize, back: bool) -> TargetMode {
        let modes = self.target_modes(t);
        let cur = self.towers[t].target;
        let n = modes.len();
        match modes.iter().position(|&m| m == cur) {
            Some(i) if n > 0 => modes[if back { (i + n - 1) % n } else { (i + 1) % n }],
            _ => modes.first().copied().unwrap_or(cur),
        }
    }

    /// Gives tower `t` a cooldown entry for each ability its tier has, as Ability.Initialise
    /// starts it (below, in the tower tick): cooldownFrames x 0.33, all of it with
    /// resetCooldownOnTierUpgrade, none with startOffCooldown.
    fn sync_abilities(&mut self, t: usize) {
        let Some(tier) = self.tier(t) else { return };
        let fresh: Vec<(String, f32)> = tier
            .abilities
            .iter()
            .filter(|a| !self.towers[t].ability_cd.contains_key(&a.name))
            .map(|a| {
                let frames = (a.cooldown * FPS) as i32 as f32;
                (a.name.clone(), if a.ready { 0.0 } else if a.full_start { frames } else { frames * 0.33 })
            })
            .collect();
        self.towers[t].ability_cd.extend(fresh);
    }

    /// Abilities on the map grouped by name, in first-gained order, for the ability buttons.
    pub fn ability_groups(&self) -> Vec<AbilityGroup> {
        let mut out: Vec<AbilityGroup> = Vec::new();
        for t in 0..self.towers.len() {
            let Some(tier) = self.tier(t) else { continue };
            for a in &tier.abilities {
                let left = self.towers[t].ability_cd.get(&a.name).copied().unwrap_or(0.0);
                let frac = if a.cooldown > 0.0 { left / (a.cooldown * FPS) } else { 0.0 };
                match out.iter_mut().find(|g| g.name == a.name) {
                    Some(g) => {
                        g.total += 1;
                        g.ready += (left <= 0.0) as usize;
                        if frac < g.frac_left {
                            (g.frac_left, g.secs_left) = (frac, left / FPS);
                        }
                    }
                    None => out.push(AbilityGroup {
                        name: a.name.clone(),
                        tower: t,
                        ready: (left <= 0.0) as usize,
                        total: 1,
                        frac_left: frac,
                        secs_left: left / FPS,
                    }),
                }
            }
        }
        out
    }

    /// Uses one ready `name` ability (the first tower that has it). Abilities only work during
    /// rounds (AbilityModel.canActivateBetweenRounds is false for the towers here).
    /// The ability is used on a spot the player picks (Darkshift).
    pub fn ability_targets_point(&self, name: &str) -> bool {
        (0..self.towers.len()).any(|t| self.tier(t).is_some_and(|tr| tr.abilities.iter().any(|a| a.name == name && (a.darkshift.is_some() || a.place_at.is_some()))))
    }

    /// Darkshift: the first ready tower with ability `name` that can reach `p` (within the
    /// radius, a valid spot for it) moves there.
    pub fn activate_ability_at(&mut self, name: &str, p: [f32; 2]) -> bool {
        if !self.rounds_active() {
            return false;
        }
        for t in 0..self.towers.len() {
            let Some(a) = self.tier(t).and_then(|tr| tr.abilities.iter().find(|a| a.name == name)).cloned() else { continue };
            if self.towers[t].ability_cd.get(name).copied().unwrap_or(0.0) > 0.0 {
                continue;
            }
            // Mega Mine: the projectile appears at the point after `delay` (STAND-IN: any
            // point; the game wants water).
            if let Some(pa) = &a.place_at {
                self.towers[t].ability_cd.insert(name.to_string(), a.cooldown * FPS);
                let (tiers, owner) = (self.towers[t].tiers, self.towers[t].id);
                let single = Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None};
                let start = self.projectiles.len();
                self.emit(p, 0.0, &single, &pa.projectile, None, tiers, owner);
                for pr in &mut self.projectiles[start..] {
                    pr.pos = p;
                    pr.vel = [0.0; 2];
                    pr.wait = pa.delay * FPS;
                }
                self.sound(&a.sounds, None);
                return true;
            }
            let Some(ds) = &a.darkshift else { continue };
            let from = self.towers[t].pos;
            if ds.restrict && (p[0] - from[0]).hypot(p[1] - from[1]) > ds.radius {
                continue;
            }
            // Checked as if the tower weren't standing where it is now.
            self.towers[t].pos = [f32::MAX, f32::MAX];
            let ok = self.can_place(self.towers[t].def, p);
            self.towers[t].pos = from;
            if !ok {
                continue;
            }
            self.towers[t].ability_cd.insert(name.to_string(), a.cooldown * FPS);
            self.towers[t].pos = p;
            self.sound(&a.sounds, None);
            return true;
        }
        false
    }

    /// The ability is used on a tower the player picks (Overclock).
    pub fn ability_targets_tower(&self, name: &str) -> bool {
        (0..self.towers.len()).any(|t| self.tier(t).is_some_and(|tr| tr.abilities.iter().any(|a| a.name == name && a.overclock.is_some())))
    }

    /// Uses a ready copy of tower-targeted ability `name` on tower `target`. HYPOTHESIS
    /// P-OCTARGET: any tower but the user itself and sub-towers; the duration uses the target's
    /// highest upgrade tier.
    pub fn activate_ability_on(&mut self, name: &str, target: usize) -> bool {
        if !self.rounds_active() || target >= self.towers.len() || self.towers[target].sub.is_some() {
            return false;
        }
        for t in 0..self.towers.len() {
            if t == target {
                continue;
            }
            let Some(a) = self.tier(t).and_then(|tr| tr.abilities.iter().find(|a| a.name == name)).cloned() else { continue };
            let Some(oc) = &a.overclock else { continue };
            if self.towers[t].ability_cd.get(name).copied().unwrap_or(0.0) > 0.0 {
                continue;
            }
            self.towers[t].ability_cd.insert(name.to_string(), a.cooldown * FPS);
            let top = *self.towers[target].tiers.iter().max().unwrap_or(&0) as usize;
            let mult = oc.tier_mult.get(top).copied().unwrap_or(1.0);
            let tw = &mut self.towers[target];
            tw.overclock = Some((oc.rate, oc.village_range, oc.range, oc.frames * mult));
            if let Some(st) = &oc.permanent {
                tw.ultra = ((tw.ultra.0 + 1).min(st.max_stacks), st.rate, st.village_range);
            }
            self.sound(&a.sounds, None);
            return true;
        }
        false
    }

    /// One of the Spawner's emissions for this frame (see step_native).
    fn emit_frame_spawn(&mut self, k: usize, path: u8) {
        let id = self.spawn_emitted(k, path);
        if let Some(b) = self.bloons.last_mut() {
            b.fresh = false;
            b.unplaced = true;
        }
        // In the path's list it goes before what the towers and the bloons' own processes
        // added this frame (recording glue_250: a round's Black listed before a Pink a
        // Corrosive Glue tick popped out the same frame).
        self.plist_move_to_mark(id, path);
        if let Some(o) = self.bloons.last().map(|b| b.obj) {
            self.fact_move_to_mark(o);
        }
        // SetMaxHealthOfBloonBehavior.OnBloonSpawned: a bloon of the mod's id spawned in
        // its round gets maxHealth = (int)(maxHealth x healthMultiplier), that layer only.
        if let Some(h) = self.health_mods.iter().find(|h| h.round == self.round + 1 && h.bloon == self.names[k]) {
            let hp = (self.defs[k].max_health as f32 * h.mult) as i32;
            if let Some(b) = self.bloons.last_mut() {
                b.hp = hp;
            }
        }
        if self.defs[k].tags.iter().any(|t| t == "Moabs") {
            self.outside_track.push(id);
        }
    }

    /// Ability.OnPreLivesLost (VERIFIED-DISASM @0x180dfa8e0) for every tower: an activateOnPreLeak
    /// ability Activates when IsReady (@0x180df9...: cooldown done, under maxActivationsPerRound,
    /// rounds running); Activate starts the cooldown and counts the activation. Whether any went.
    fn pre_leak_abilities(&mut self) -> bool {
        if !self.rounds_active() {
            return false;
        }
        let mut any = false;
        for t in 0..self.towers.len() {
            let Some(abilities) = self.tier(t).map(|tr| tr.abilities.iter().filter(|a| a.pre_leak).cloned().collect::<Vec<_>>()) else { continue };
            for a in abilities {
                let tw = &self.towers[t];
                if tw.ability_cd.get(&a.name).copied().unwrap_or(0.0) > 0.0 {
                    continue;
                }
                if a.per_round > 0 && tw.ability_round.get(&a.name).copied().unwrap_or(0) as i32 >= a.per_round {
                    continue;
                }
                // The Spawner ran before the bloons in the process list: this frame's emissions are
                // already in the factory list (recording bomb_005: the Black entering at 1190 is
                // among the 52 the Bomb Storm pops).
                for (k, path) in std::mem::take(&mut self.frame_spawns) {
                    self.emit_frame_spawn(k, path);
                }
                let tw = &mut self.towers[t];
                tw.ability_cd.insert(a.name.clone(), (a.cooldown * FPS) as i32 as f32);
                *tw.ability_round.entry(a.name.clone()).or_insert(0) += 1;
                self.use_ability(t, &a);
                any = true;
            }
        }
        any
    }

    pub fn activate_ability(&mut self, name: &str) -> bool {
        if !self.rounds_active() {
            return false;
        }
        for t in 0..self.towers.len() {
            let Some(a) = self.tier(t).and_then(|tr| tr.abilities.iter().find(|a| a.name == name)).cloned() else { continue };
            if self.towers[t].ability_cd.get(name).copied().unwrap_or(0.0) > 0.0 {
                continue;
            }
            // (Ability.IsReady, VERIFIED-DISASM @0xdf9e80: Simulation.AreRoundsActive, so any time
            // in sandbox or while bloons are out; the clone's round_active alone left sandbox
            // abilities dead.)
            // Ability.CanUseAbility (VERIFIED-DISASM @0x180df8d...): every behaviour's check, a
            // toggled-off ToggleableAbilityBehavior's fails (recording sniper_040: the scenario's
            // Activate(true) on both abilities drops only Supply Drop's crate). HYPOTHESIS
            // P-TOGGLE: the toggle stays as it started (switching it isn't modelled).
            if a.toggled_off {
                continue;
            }
            // IsReady: at most maxActivationsPerRound a round; Activate counts it.
            if a.per_round > 0 && self.towers[t].ability_round.get(name).copied().unwrap_or(0) as i32 >= a.per_round {
                continue;
            }
            *self.towers[t].ability_round.entry(name.to_string()).or_insert(0) += 1;
            self.towers[t].ability_cd.insert(name.to_string(), a.cooldown * FPS);
            self.use_ability(t, &a);
            return true;
        }
        false
    }

    /// ClearHitBloonsWhenNoLongerCollidingModel: projectiles forget bloons (gone, or) no longer
    /// within reach, so those can be hit again when they come back into contact.
    fn forget_untouched(&mut self) {
        if !self.projectiles.iter().any(|p| p.spec.clear_off && !p.hit.is_empty()) {
            return;
        }
        let at: FastMap<u64, ([f32; 2], f32)> = self.bloons.iter().map(|b| (b.id, (b.pos, self.defs[b.kind].radius))).collect();
        for pr in self.projectiles.iter_mut().filter(|p| p.spec.clear_off) {
            let (p, r) = (pr.pos, pr.spec.radius);
            pr.hit.retain(|id| at.get(id).is_some_and(|(q, br)| (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) <= (r + br) * (r + br)));
        }
    }

    /// ActivateAbilityAfterIntervalModel (Carpet of Spikes): during rounds the tower's first
    /// ability goes off every `interval` seconds by itself (its cooldown untouched).
    /// HYPOTHESIS P-AUTOABILITY: the timer only runs during rounds and starts at 0.
    fn auto_abilities(&mut self) {
        if !self.round_active {
            return;
        }
        for t in 0..self.towers.len() {
            let Some((every, a)) = self.tier(t).and_then(|tr| Some((tr.auto_ability?, tr.abilities.first()?.clone()))) else { continue };
            self.towers[t].auto_t += 1.0;
            if self.towers[t].auto_t >= every * FPS {
                self.towers[t].auto_t = 0.0;
                self.use_ability(t, &a);
            }
        }
    }

    /// What using ability `a` of tower `t` does (its cooldown is the caller's business).
    fn use_ability(&mut self, t: usize, a: &crate::data::Ability) {
        self.use_ability_effects(t, a);
        // Ability.Activate between frames (input): its ActivateAttackModel attacks act at once,
        // stamped with the frame just simulated (recording ice_050: Absolute Zero's blast is out
        // with the weapon at 120, the frame the ability went on after, and freezes on 121).
        if !self.in_step && self.towers[t].temp.iter().any(|x| x.now) {
            // (Towers placed since the last frame have no buffs yet.)
            if self.buffs.len() != self.towers.len() {
                self.buffs = self.compute_buffs();
            }
            let phase = std::mem::replace(&mut self.phase, 1);
            let cd_speed = vec![1.0; self.towers.len()];
            let (mut dots, mut emits, mut leak_voice) = (Vec::new(), Vec::new(), false);
            let round_clock = self.round_clock;
            self.towers_phase(Some(t), &cd_speed, &mut Default::default(), round_clock, &mut dots, &mut emits, &[], &[], &mut leak_voice);
            self.phase = phase;
        }
    }

    fn use_ability_effects(&mut self, t: usize, a: &crate::data::Ability) {
        // AbilityDamageAll.Activate (VERIFIED-DISASM @0x180df6800), each model in turn: every bloon in
        // the factory list (not children made on the way) whose having one of the tags equals
        // `inclusive` takes Bloon.Damage(damage), distributed with the blocker overridden, for the
        // tower (recording bomb_005: Bomb Blitz at 120 wipes the MOAB).
        for da in &a.damage_all {
            let ids: Vec<u64> = self.bloons.iter()
                .filter(|b| self.defs[b.kind].tags.iter().any(|g| da.tags.contains(g)) == da.inclusive)
                .map(|b| b.id).collect();
            let owner = self.towers[t].id;
            for id in ids {
                let Some(j) = self.bloons.iter().position(|b| b.id == id) else { continue };
                let (tally0, saved) = (self.damage_tally, self.distribute);
                self.distribute = (true, true);
                let n = self.damage(j, da.damage, &mut Vec::new());
                self.distribute = saved;
                let dealt = self.damage_tally - tally0;
                if let Some(tw) = self.towers.iter_mut().find(|x| x.id == owner) {
                    tw.pops += n;
                tw.sun_destroyed(self.last_destroyed);
                    tw.damage += dealt;
                    self.stats.pops += n;
                    self.stats.damage += dealt;
                }
            }
        }
        let mut slot = self.tier(t).map_or(0, |tr| {
            tr.abilities.iter().take_while(|o| o.name != a.name).flat_map(|o| o.activate.iter()).flat_map(|x| x.attacks.iter()).map(|at| at.weapons.len()).sum()
        });
        let now_rc = self.round_clock;
        let tw = &mut self.towers[t];
        for x in &a.activate {
            // MarkedToPop.Activate: phase 1, the marking attack on (the execution attack waits),
            // the others paused, the lineages cleared.
            if let Some(fr) = x.mtp {
                tw.temp.retain(|o| !(o.attack.model_name.contains("MarkingAttack") || o.attack.model_name.contains("ExecutionAttack")));
                tw.mtp = Some(MtpState { phase: 1, act_at: now_rc, exec_at: -1, frames: fr, lin: [Vec::new(), Vec::new()] });
            }
            for (k, at) in x.attacks.iter().enumerate() {
                let here = slot;
                slot += at.weapons.len();
                tw.temp.push(TempAttack {
                    slot: here,
                    attack: at.clone(),
                    frames_left: if x.mtp.is_some() { f32::INFINITY } else { x.lifespan * FPS },
                    one_shot: x.one_shot,
                    pauses: x.turn_off_existing,
                    delay: if x.mtp.is_some() && k > 0 { f32::INFINITY } else { x.delay * FPS },
                    now: x.one_shot || x.process_on_activate,
                    counted: None,
                });
            }
        }
        // (Ability.Activate runs MarkedToPop.Process at once: the marking attack has no target yet
        // (hadValidTarget), so the execution starts; its attack acts in the next frame (recordings
        // desperado_050, 052, 250: the marking weapon never fires, the execution at rt 120).)
        if a.activate.iter().any(|x| x.mtp.is_some()) {
            let had = self.towers[t].had_target.get("AttackModel_MarkingAttack_").copied().unwrap_or(false);
            let now = self.round_clock;
            let tw = &mut self.towers[t];
            if !had {
                if let Some(mm) = tw.mtp.as_mut() {
                    mm.phase = 2;
                    mm.exec_at = now;
                }
                for o in tw.temp.iter_mut() {
                    if o.attack.model_name.contains("MarkingAttack") {
                        o.delay = f32::INFINITY;
                    } else if o.attack.model_name.contains("ExecutionAttack") {
                        o.delay = 0.0;
                    }
                }
            }
        }
        let tw = &mut self.towers[t];
        if let Some(tb) = &a.turbo {
            tw.turbo = Some(ActiveTurbo { mult: tb.mult, extra_damage: tb.extra_damage, frames_left: tb.lifespan * FPS });
        }
        for sp in &a.supports {
            tw.temp_supports.retain(|(o, _, _)| o.id != sp.id);
            // (Its mutator reaches the towers from the next frame: recording ninja_052, Sabotage
            // at rt 120, the Caltrops' points remade at 122.)
            tw.temp_supports.push((sp.clone(), sp.lifespan * FPS, self.steps as u64));
        }
        if let Some(am) = &a.ammo {
            tw.ammo = Some((am.delay * FPS, am.shots, am.clone()));
        }
        tw.attack_t = 0.0;
        {
            let tw = &mut self.towers[t];
            if let Some(sw) = &a.switch {
                tw.look = sw.display.clone().map(|d| (d, sw.lifespan * FPS));
            }
            // HYPOTHESIS P-MUTEFRAME: MutateRemoveAllAttacksOnAbilityActivate's timed mutator
            // lets the attacks back a frame after lifespan x 60 (recording wizard_050: Summon
            // Phoenix at 120, the Wall of Fire again at 1320); `mute` counts down before attacks.
            tw.mute = tw.mute.max(a.mute * FPS + if a.mute > 0.0 { 1.0 } else { 0.0 });
            self.leak_immune = self.leak_immune.max(a.immunity * FPS);
            let tw = &mut self.towers[t];
            if let Some(r) = &a.range_up {
                tw.range_up = Some((r.mult, r.add, r.frames));
            }
        }
        for b in &a.boosts {
            let tw = &mut self.towers[t];
            tw.boosts.push((b.clone(), b.frames));
        }
        if let Some(td) = &a.takedown {
            self.takedown(t, td);
        }
        // STAND-IN P-BIOHACK: the `count` towers nearest the hero (not sub-towers, not
        // heroes) get +damage for the lifespan; the shutdown after it isn't modelled.
        if let Some(bh) = &a.biohack {
            let p = self.towers[t].pos;
            let mut near: Vec<(f32, usize)> = (0..self.towers.len())
                .filter(|&o| o != t && self.towers[o].sub.is_none() && self.hero_def(self.towers[o].def).is_none())
                .map(|o| ((self.towers[o].pos[0] - p[0]).hypot(self.towers[o].pos[1] - p[1]), o))
                .collect();
            near.sort_by(|a, b| a.0.total_cmp(&b.0));
            for &(_, o) in near.iter().take(bh.count as usize) {
                let boost = crate::data::ProjBoost { frames: bh.lifespan * FPS, immune: None, damage: bh.damage, tag_mods: Vec::new(), radius_mult: 1.0 };
                self.towers[o].boosts.push((boost, bh.lifespan * FPS));
            }
        }
        // HYPOTHESIS P-BARRAGE: MOAB Barrage puts a damage-over-time on the `targets`
        // strongest MOAB-class bloons on the map: `damage` every `interval` for its duration.
        if let Some(br) = &a.barrage {
            let owner = self.towers[t].id;
            let mut moabs: Vec<(f32, usize)> = (0..self.bloons.len())
                .filter(|&i| self.defs[self.bloons[i].kind].tags.iter().any(|x| x == "Moabs") && on_map(self.bloons[i].pos, 0.0))
                .map(|i| (self.rbe[self.bloons[i].kind] * 1e4 + self.bloons[i].progress(), i))
                .collect();
            moabs.sort_by(|a, b| b.0.total_cmp(&a.0));
            for &(_, i) in moabs.iter().take(br.targets as usize) {
                self.bloons[i].effects.push(Effect {
                    overlay: Default::default(), seq: 0,
                    kind: EffectKind::Dot { damage: br.damage, interval: br.interval * FPS, tick: self.round_clock as f32, immune: br.immune, mods: Default::default(), start: Some(0.0) },
                    id: "MoabBarrage".into(),
                    frames_left: br.lifespan * FPS,
                    total: br.lifespan * FPS,
                    added: self.sim_elapsed,
                    layers: 1,
                    owner,
                    distribute: (true, false),
                    keep: false,
                });
            }
        }
        // Sword Charge: the projectile starts at the end of every track (the start if it
        // doesn't run in reverse).
        if let Some(proj) = &a.track_charge {
            let (tiers, owner) = (self.towers[t].tiers, self.towers[t].id);
            let single = Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None};
            for k in 0..self.tracks.len() {
                let reverse = proj.track_travel.as_ref().is_some_and(|x| x.reverse);
                let d0 = if reverse { self.tracks[k].length() } else { 0.0 };
                let p0 = self.tracks[k].pos(d0);
                let start = self.projectiles.len();
                self.emit(p0, 0.0, &single, proj, None, tiers, owner);
                for pr in &mut self.projectiles[start..] {
                    pr.pos = p0;
                    pr.vel = [0.0; 2];
                    pr.on_track = Some((k as u8, d0));
                    if let Some(tt) = &proj.track_travel {
                        pr.frames_left = tt.lifespan * FPS;
                    }
                }
            }
        }
        // STAND-IN P-LEAP: Leaping Sword lands on the bloon furthest along the track (on
        // the map) after landDelay.
        if let Some(lp) = &a.leap {
            let target = self.bloons.iter().filter(|b| on_map(b.pos, 0.0)).max_by(|x, y| x.progress().total_cmp(&y.progress())).map(|b| b.pos);
            if let Some(p) = target {
                let (tiers, owner) = (self.towers[t].tiers, self.towers[t].id);
                let single = Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None};
                let start = self.projectiles.len();
                self.emit(p, 0.0, &single, &lp.projectile, None, tiers, owner);
                for pr in &mut self.projectiles[start..] {
                    pr.pos = p;
                    pr.vel = [0.0; 2];
                    pr.wait = lp.delay * FPS;
                    pr.frames_left += lp.delay * FPS;
                }
            }
        }
        if a.syphon > 0.0 {
            self.syphon = self.syphon.max(a.syphon * FPS);
        }
        let (pos, id) = (self.towers[t].pos, self.towers[t].id);
        for spec in &a.create_tower {
            self.spawn_sub(spec.clone(), pos, id, false);
        }
        // CreateProjectileOnAbilityActivateModel.
        let tiers = self.towers[t].tiers;
        let face = self.towers[t].angle;
        for c in &a.on_activate {
            self.emit(pos, face, &c.emission, &c.projectile, None, tiers, id);
        }
        // ActivateAbilitiesOnAbilityModel: the named ability goes too (on its own cooldown).
        if let Some(other) = a.also.as_ref().filter(|o| **o != a.name) {
            self.activate_ability(other);
        }
        if let Some(fc) = &a.fan_club {
            // MonkeyFanClub.Activate (VERIFIED-DISASM @0xe5bf10): the activating tower, then every
            // tower nearest first (TowerManager.GetClosestTowers within 99999, List.Sort) of the
            // same base tower, top and bottom paths at most maxTier and middle at most 4 (recording
            // dart_040_club: a 0-3-0 joins), until `count` in all. A tower already a fan is
            // re-fanned; a Plasma fan (immuneBloonProperties has Purple) isn't taken by the plain
            // Fan Club.
            let plasma = |f: &crate::data::FanClub| f.immune & 8 != 0;
            let (def, p) = (self.towers[t].def, self.towers[t].pos);
            let mut near: Vec<(f32, usize)> = (0..self.towers.len())
                .map(|o| {
                    let q = self.towers[o].pos;
                    ((q[0] - p[0]) * (q[0] - p[0]) + (q[1] - p[1]) * (q[1] - p[1]), o)
                })
                .collect();
            dotnet_sort(&mut near, &|a: &(f32, usize), b: &(f32, usize)| if a.0 < b.0 { -1 } else { (b.0 < a.0) as i32 });
            let mut fans = vec![t];
            for (_, o) in near {
                if o == t || fans.len() >= fc.count.max(1) as usize {
                    continue;
                }
                let x = &self.towers[o];
                let tiers_ok = x.tiers[0] as u32 <= fc.max_tier as u32 && x.tiers[1] <= 4 && x.tiers[2] as u32 <= fc.max_tier as u32;
                if x.def != def || x.sub.is_some() || !tiers_ok || x.morph.is_some() {
                    continue;
                }
                if !plasma(fc) && x.fan.as_ref().is_some_and(|(f, _)| plasma(f)) {
                    continue;
                }
                fans.push(o);
            }
            for o in fans {
                self.towers[o].fan = Some((fc.clone(), fc.lifespan * FPS));
            }
        }
        if let Some(m) = &a.morph {
            // HYPOTHESIS P-MORPH: the nearest eligible towers within the Alchemist's range.
            let (p, r) = (self.towers[t].pos, self.range(t));
            let mut near: Vec<(f32, usize)> = (0..self.towers.len())
                .filter(|&o| o != t)
                .filter(|&o| {
                    let x = &self.towers[o];
                    let name = &self.tower_defs[x.def].0;
                    x.sub.is_none() && x.paragon.is_none() && x.morph.is_none() && x.fan.is_none()
                        && x.tiers.iter().all(|&n| n <= m.max_tier)
                        && m.affect.iter().any(|a| name == a || name.starts_with(a.as_str()))
                })
                .map(|o| { let q = self.towers[o].pos; ((q[0] - p[0]).hypot(q[1] - p[1]), o) })
                .filter(|(d, _)| *d <= r)
                .collect();
            near.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (_, o) in near.into_iter().take(m.max as usize) {
                self.towers[o].morph = Some((m.tier.clone(), m.display.clone(), m.lifespan * FPS));
            }
        }
        if let Some(l) = &a.loan {
            if self.coop_wallets.is_some(){self.coop_loan(self.towers[t].id,l.amount,l.rate);}else{
            self.cash += l.amount;
            self.debt += l.amount;
            self.debt_rate = l.rate;
            }
        }
        self.sound(&a.sounds, None);
    }

    /// Income (pops, pickups, banks, round bonus): ImfLoanModel.incomeRecoveryRate of it repays
    /// any loan first.
    fn earn(&mut self, v: f64) {
        if self.coop_wallets.is_some(){self.coop_credit(None,v);return;}
        // ModifyAllCashModModel (Half Cash, Deflation).
        let v = v * self.cash_mult;
        let pay = (v * self.debt_rate).min(self.debt).max(0.0);
        self.debt -= pay;
        self.cash += v - pay;
    }

    /// Empties tower `t`'s bank into the player's cash.
    pub fn collect_bank(&mut self, t: usize) {
        let b = std::mem::take(&mut self.towers[t].bank).floor();
        if b > 0.0 {
            self.earn_for(self.towers[t].id,b);
            self.cash_pops.push((self.towers[t].pos, b));
        }
    }

    /// Collects hand-picked cash (bananas) that has landed within reach of `p`. STAND-IN reach:
    /// a quarter of PickupModel.collectRadius (50 = 12.5 units), the real cursor test is unknown.
    pub fn collect_cash(&mut self, p: [f32; 2]) {
        let mut i = 0;
        while i < self.projectiles.len() {
            let pr = &self.projectiles[i];
            let reach = pr.spec.cash.as_ref().and_then(|c| c.pickup).map(|r| r * 0.25);
            if let Some(r) = reach.filter(|_| pr.moving <= 0.0) {
                if (pr.pos[0] - p[0]).powi(2) + (pr.pos[1] - p[1]).powi(2) <= r * r {
                    let pr = self.projectiles.remove(i);
                    self.collect_projectile_cash(&pr);
                    self.sound(&pr.spec.pickup_sounds, None);
                    self.cash_pops.push((pr.pos, pr.value));
                    continue;
                }
            }
            i += 1;
        }
    }

    /// CollectCashZoneModel: towers that collect landed cash in reach. HYPOTHESIS P-COLLECT:
    /// collected the moment it has landed (the attract lerp isn't shown).
    fn collect_zones(&mut self) {
        let zones: Vec<([f32; 2], f32)> = (0..self.towers.len())
            .filter_map(|t| {
                let r = self.tier(t)?.collect_zone?;
                Some((self.towers[t].pos, if r > 0.0 { r } else { self.range(t) }))
            })
            .collect();
        if zones.is_empty() {
            return;
        }
        let mut i = 0;
        while i < self.projectiles.len() {
            let pr = &self.projectiles[i];
            let landed = pr.spec.cash.as_ref().is_some_and(|c| c.pickup.is_some()) && pr.moving <= 0.0;
            if landed && zones.iter().any(|(c, r)| (pr.pos[0] - c[0]).powi(2) + (pr.pos[1] - c[1]).powi(2) <= r * r) {
                let pr = self.projectiles.remove(i);
                self.collect_projectile_cash(&pr);
                    self.sound(&pr.spec.pickup_sounds, None);
                self.cash_pops.push((pr.pos, pr.value));
                continue;
            }
            i += 1;
        }
    }

    /// Current tier stats for tower `t`.
    pub fn tier(&self, t: usize) -> Option<&Tier> {
        self.tier_of(&self.towers[t])
    }

    pub fn tier_of<'a>(&'a self, tw: &'a Tower) -> Option<&'a Tier> {
        if let Some(s) = &tw.sub {
            return Some(&s.spec.tier);
        }
        if let Some((_, t)) = &tw.paragon {
            return Some(t);
        }
        if let Some((t, _, _)) = &tw.morph {
            return Some(t);
        }
        if let Some((_, t)) = &tw.temple {
            return Some(t);
        }
        let tiers=&self.tower_tiers[tw.def].as_ref()?.tiers;
        #[cfg(test)] if self.reference_targeting{return tiers.get(&tw.tier_key());}
        tiers.get(tier_model_key(tw.tiers,&mut [0;9]))
    }

    /// The tower attacks anywhere on the map (no range circle).
    pub fn is_global(&self, t: usize) -> bool {
        self.tier(t).map_or(self.tower_defs[self.towers[t].def].1.global, |tier| tier.global)
    }

    /// Range circle radius. Map-wide towers show their TowerModel.range (Sniper: 20), not their
    /// attacks' 9999999.
    pub fn range(&self, t: usize) -> f32 {
        if self.is_global(t) {
            return self.tier(t).map_or(self.tower_defs[self.towers[t].def].1.range, |tier| tier.range);
        }
        let r = match self.tier(t) {
            // Map-wide side attacks (Master Bomber's sticky bombs: 9999999) don't widen the circle.
            Some(tier) => tier
                .attacks
                .iter()
                .filter(|a| !a.fire_without_target && a.range < 1e5)
                .map(|a| a.range)
                .fold(tier.range, f32::max),
            None => self.tower_defs[self.towers[t].def].1.range,
        };
        self.buffs.get(t).map_or(r, |b| b.range(r))
    }

    /// Next upgrade on `path`: Some((name, cost)) if one exists and crosspathing allows it.
    pub fn next_upgrade(&self, t: usize, path: usize) -> Option<(String, f64)> {
        // A Paragon has no more upgrades; a sub-tower has none.
        if self.towers[t].paragon.is_some() || self.towers[t].sub.is_some() {
            return None;
        }
        if path < 3 && self.towers[t].tiers[path] == 5 && self.paragon_ready(t) {
            let p = self.tower_tiers[self.towers[t].def].as_ref()?.paragon.as_ref()?;
            return Some((p.name.clone(), game_cost(p.cost, self.cost_mult, 0.0)));
        }
        let tw = &self.towers[t];
        if tw.sub.is_some() {
            return None;
        }
        let tt = self.tower_tiers[tw.def].as_ref()?;
        let cur = tw.tiers[path] as usize;
        let mut next = tw.tiers;
        next[path] += 1;
        if next[path] > tw.tier_cap {
            return None;
        }
        // BTD6 crosspath rule: at most two paths used, at most one above tier 2.
        let used = next.iter().filter(|&&x| x > 0).count();
        let high = next.iter().filter(|&&x| x > 2).count();
        if used > 2 || high > 1 || !tt.tiers.contains_key(&format!("{}{}{}", next[0], next[1], next[2])) {
            return None;
        }
        // BeastHandlerUpgradeLock.IsUpgradeBlocked (VERIFIED-DISASM @0xd220e0): buying a path's
        // first tier needs somewhere for its pet: the water pet (Piranha) water within the
        // handler's range (BeastHandlerLeash.IsWithinRangeOfWater), the land pet (Microraptor)
        // track within range (IsPathSegmentInRange). HYPOTHESIS P-PETLOCK: "within range" is a
        // water area / track segment touching the current range circle.
        if cur == 0 {
            if let Some(nt) = tt.tiers.get(&format!("{}{}{}", next[0], next[1], next[2])) {
                // (IsWithinRangeOfWater / IsPathSegmentInRange ask the PetSpawnerAttack, range 60, not the
                // handler's own 20.)
                let (c, r) = (tw.pos, self.tier(t).map_or(0.0, |x| x.attacks.iter().find(|a| a.model_name.contains("PetSpawnerAttack")).map_or(x.range, |a| a.range)));
                let water_pet = nt.pets.iter().any(|p| p.areas.contains(&1) && !p.areas.contains(&2));
                let land_pet = nt.pets.iter().any(|p| p.areas.contains(&2) && p.base.is_some());
                if water_pet && !self.areas.iter().any(|a| a.kind == 1 && Self::circle_hits_poly(c, r, &a.points)) {
                    return None;
                }
                if land_pet && !self.track_within(c, r) {
                    return None;
                }
            }
        }
        // Only one of each tier 5 per tower type (outside sandbox).
        if next[path] == 5 && self.towers.iter().any(|o| o.def == tw.def && o.tiers[path] == 5) {
            return None;
        }
        if let Some(h) = &tt.hero {
            // Buying a level: Hero.GetCostToLevelUp (VERIFIED-DISASM @0xd47da0):
            // max((next.xpCost - xp) * costPerXpToLevel, 11).
            if path != 0 {
                return None;
            }
            let lvl = h.levels.iter().find(|l| l.level == tw.tiers[0] + 1)?;
            return Some((lvl.name.clone(), (((lvl.xp - tw.xp) * h.cost_per_xp).max(11.0)) as f64));
        }
        let d = self.discount(tw.def, tw.pos, Some(t), next[path] as u32);
        tt.upgrades.iter().find(|u| u.path == path && u.tier == cur).map(|u| (u.name.clone(), game_cost(u.cost, self.cost_mult, d)))
    }

    /// A Paragon's display level, 1-based (research: ParagonTower.GetDisplayDegreeIndex):
    /// degree / (degree count / levels), at most the last level.
    pub fn paragon_level(&self, tw: &Tower) -> Option<usize> {
        let (degree, _) = tw.paragon.as_ref()?;
        let levels = self.tower_tiers[tw.def].as_ref()?.paragon.as_ref()?.levels.max(1);
        let count = self.paragon_degree.as_ref().map_or(100, |p| p.requirements.len()).max(1);
        let per = (count / levels).max(1);
        Some((*degree as usize / per).min(levels - 1) + 1)
    }

    /// Tower `t` can become its Paragon: it has one, `t` has a tier 5, and the towers of its
    /// kind together have a tier 5 on every path.
    pub fn paragon_ready(&self, t: usize) -> bool {
        let tw = &self.towers[t];
        if tw.paragon.is_some() || tw.sub.is_some() || !tw.tiers.contains(&5) {
            return false;
        }
        if self.tower_tiers[tw.def].as_ref().and_then(|x| x.paragon.as_ref()).is_none() {
            return false;
        }
        (0..3).all(|p| self.towers.iter().any(|o| o.def == tw.def && o.sub.is_none() && o.paragon.is_none() && o.tiers[p] == 5))
    }

    /// The Paragon's degree from the towers sacrificed for it (research: ParagonTower.
    /// GetTowerInvestment, CombineInvestments, GetDegreeFromInvestment). Per tower: pops power
    /// = damage dealt / popsOverX + cashEarnedContributionModifier x cash earned / popsOverX;
    /// below tier 5 also tier sum x nonTier5TowersMultByX and money power = worth /
    /// (upgrade cost / moneySpentOverX); a tier 5 from the fourth on tier5TowersMultByX. Each
    /// kind is capped, then the degree is the number of powerDegreeRequirements met (>= 1).
    /// STAND-IN P-PARAGONPOWER: damage dealt is the tower's pop count, cash earned is 0, tier 5s
    /// are counted in placement order, and paying extra cash for power isn't offered.
    /// Paragon purchase for tower `t`: (cost, the most extra cash that still adds power: up to
    /// maxPowerFromMoneySpent, and the towers it absorbs). None if it can't become one.
    pub fn paragon_quote(&self, t: usize) -> Option<(f64, f64, Vec<usize>)> {
        if !self.paragon_ready(t) || self.towers[t].paragon.is_some() {
            return None;
        }
        let def = self.towers[t].def;
        let pdef = self.tower_tiers[def].as_ref().and_then(|x| x.paragon.clone())?;
        let cost = game_cost(pdef.cost, self.cost_mult, 0.0);
        let all: Vec<usize> = (0..self.towers.len())
            .filter(|&o| self.towers[o].def == def && self.towers[o].sub.is_none() && self.towers[o].paragon.is_none())
            .collect();
        let max = self.paragon_degree.as_ref().map_or(0.0, |pd| {
            let (_, money) = self.paragon_power(&all, pdef.cost, 0.0);
            let room = (pd.max_power_from_money_spent - money.min(pd.max_power_from_money_spent)).max(0.0);
            (room * pdef.cost as f32 / pd.money_spent_over_x) as f64
        });
        Some((cost, max.max(0.0).floor(), all))
    }

    /// The degree tower `t`'s Paragon would have with `extra` cash invested.
    pub fn paragon_degree_with(&self, t: usize, extra: f64) -> u32 {
        let def = self.towers[t].def;
        let Some((_, _, all)) = self.paragon_quote(t) else { return 1 };
        let Some(pdef) = self.tower_tiers[def].as_ref().and_then(|x| x.paragon.clone()) else { return 1 };
        self.paragon_degree_ex(&all, pdef.cost, extra)
    }

    pub fn paragon_degree(&self, sacrificed: &[usize], cost: f64) -> u32 {
        self.paragon_degree_ex(sacrificed, cost, 0.0)
    }

    /// (total power, power from money) of the sacrificed towers plus `extra` invested cash.
    fn paragon_power(&self, sacrificed: &[usize], cost: f64, extra: f64) -> (f32, f32) {
        let Some(pd) = &self.paragon_degree else { return (0.0, 0.0) };
        let (mut pops, mut money, mut non5, mut t5) = (0f32, 0f32, 0f32, 0f32);
        let mut fives = 0;
        let mut order: Vec<usize> = sacrificed.to_vec();
        order.sort_by_key(|&o| self.towers[o].id);
        for o in order {
            let tw = &self.towers[o];
            pops += tw.pops as f32 / pd.pops_over_x;
            if tw.tiers.iter().all(|&x| x < 5) {
                non5 += tw.tiers.iter().map(|&x| x as f32).sum::<f32>() * pd.non_tier5_towers_mult_by_x;
                money += tw.invested as f32 / (cost as f32 / pd.money_spent_over_x);
            } else {
                if fives >= 3 {
                    t5 += pd.tier5_towers_mult_by_x;
                }
                fives += 1;
            }
        }
        // ParagonConfirmationPopup: extra cash counts as money spent.
        money += extra as f32 / (cost as f32 / pd.money_spent_over_x);
        let total = pops.min(pd.max_power_from_pops) + money.min(pd.max_power_from_money_spent)
            + non5.min(pd.max_power_from_non_tier5_count) + t5.min(pd.max_power_from_tier5_count);
        (total, money)
    }

    fn paragon_degree_ex(&self, sacrificed: &[usize], cost: f64, extra: f64) -> u32 {
        let Some(pd) = &self.paragon_degree else { return 1 };
        let (total, _) = self.paragon_power(sacrificed, cost, extra);
        (pd.requirements.iter().take_while(|&&r| r <= total).count() as u32).max(1)
    }

    /// The Paragon tier with degree `d`'s changes (research: ParagonTower.GetDegreeMutator,
    /// PowerDegreeMutator.MutateTower): weapon rates and ability cooldowns / (1 + 0.01 x
    /// round1(sqrt((d-1) x attackCooldownReductionX))); pierce + trunc(pierce x (d-1) x
    /// piercePercentPerDegree %) + (d-1) x pierceIncreasePerDegree; damage x (1 + (d-1) x
    /// damagePercentPerDegree %) + (d-1) / damageIncreaseForDegrees x damageIncreasePerDegree;
    /// one more step of each at degreeCount. Applied to every (sub-)projectile with damage.
    pub fn paragon_tier(&self, base: &Tier, d: u32) -> Tier {
        let Some(pd) = &self.paragon_degree else { return base.clone() };
        let e = d.saturating_sub(1) as f32;
        let x = (e * pd.attack_cooldown_reduction_x).max(0.0).sqrt() * 10.0;
        // Math.Round away from zero, to 0.1.
        let cdr = (if x.fract() == 0.5 { x.trunc() + 1.0 } else { (x + 0.5).floor() }) / 10.0;
        let (mut pp, mut pa, mut dp) = (e * pd.pierce_percent_per_degree, e * pd.pierce_increase_per_degree, e * pd.damage_percent_per_degree);
        let mut da = e / pd.damage_increase_for_degrees * pd.damage_increase_per_degree;
        if d >= pd.degree_count {
            pp += pd.pierce_percent_per_degree;
            pa += pd.pierce_increase_per_degree;
            dp += pd.damage_percent_per_degree;
            da += pd.damage_increase_per_degree;
        }
        fn proj(p: &mut Proj, pp: f32, pa: f32, dp: f32, da: f32) {
            p.pierce += (p.pierce * pp * 0.01).trunc() + pa;
            if p.damage > 0.0 {
                p.damage += p.damage * dp * 0.01 + da;
            }
            for c in &mut p.on_contact {
                proj(&mut c.projectile, pp, pa, dp, da);
            }
            for c in &mut p.on_exhaust {
                proj(&mut c.projectile, pp, pa, dp, da);
            }
        }
        let mut t = base.clone();
        let speed = 1.0 / (1.0 + 0.01 * cdr);
        for a in &mut t.attacks {
            for w in &mut a.weapons {
                w.rate *= speed;
                proj(&mut w.projectile, pp, pa, dp, da);
            }
        }
        for ab in &mut t.abilities {
            ab.cooldown *= speed;
        }
        t
    }

    pub fn upgrade(&mut self, t: usize, path: usize) -> bool {
        if self.towers[t].paragon.is_some() {
            return false;
        }
        // Upgrading a tier-5 path when a Paragon is possible buys the Paragon (no extra cash).
        if path < 3 && self.towers[t].tiers[path] == 5 && self.paragon_ready(t) {
            return self.buy_paragon(t, 0.0);
        }
        let Some((_, cost)) = self.next_upgrade(t, path) else { return false };
        if self.cash < cost {
            return false;
        }
        self.upgrade_bought(t, path, cost)
    }

    /// Buys tower `t`'s Paragon with `extra` cash invested: every other tower of its kind is
    /// sacrificed (no refund) for its degree.
    pub fn buy_paragon(&mut self, t: usize, extra: f64) -> bool {
        if self.mod_runtime.intercepts & crate::mod_runtime::UPGRADE!=0 {
            let event=serde_json::json!({"tower":crate::mod_entities::tower(self,t),"extra":extra,"paragon":true});
            if self.mod_intercept("before_upgrade",event).cancel{return false;}
        }
        {
            let def = self.towers[t].def;
            let Some(pdef) = self.tower_tiers[def].as_ref().and_then(|x| x.paragon.clone()) else { return false };
            let cost = game_cost(pdef.cost, self.cost_mult, 0.0);
            let extra = extra.max(0.0);
            if self.cash < cost + extra || self.paragon_quote(t).is_none() {
                return false;
            }
            let id = self.towers[t].id;
            let all: Vec<usize> = (0..self.towers.len()).filter(|&o| self.towers[o].def == def && self.towers[o].sub.is_none() && self.towers[o].paragon.is_none()).collect();
            let degree = self.paragon_degree_ex(&all, pdef.cost, extra);
            let cost = cost + extra;
            self.cash -= cost;
            self.stats.cash_spent += cost;
            let gone: Vec<u64> = all.iter().map(|&o| self.towers[o].id).filter(|&o| o != id).collect();
            // ParagonTower.MakeNewInvestmentEffect: an orb per sacrificed tower, sized by what it
            // cost (Tier5 orb for a tier-5 tower).
            let orbs = all.iter().filter(|&&o| self.towers[o].id != id)
                .map(|&o| (self.towers[o].pos, self.towers[o].invested, self.towers[o].tiers.iter().any(|&x| x >= 5))).collect();
            self.sacrifices.push(Sacrifice { tower: id, at: self.towers[t].pos, orbs, start: self.sim_elapsed + 1,
                                             active_at: self.sim_elapsed + 1 + pdef.inactive_frames, degree: 0 });
            self.towers.retain(|o| !gone.contains(&o.id) && !o.sub.as_ref().is_some_and(|s| gone.contains(&s.parent)));
            let t = self.towers.iter().position(|o| o.id == id).unwrap();
            let kept: Vec<String> = self.tier(t).map(|tr| tr.attacks.iter().map(|a| a.model_name.clone()).collect()).unwrap_or_default();
            let tier = std::sync::Arc::new(self.paragon_tier(&pdef.tier, degree));
            if let Some(s) = self.sacrifices.last_mut() {
                s.degree = degree;
            }
            let tw = &mut self.towers[t];
            tw.paragon = Some((degree, tier));
            tw.paragon_kept = kept;
            // ParagonTower.StartSacrifice (VERIFIED-DISASM @0xd73560): paused, activeAt = time +
            // inactiveDurationFrames on the frame the purchase is processed (the next); Finish
            // un-pauses once time >= activeAt, after that frame's weapons (recording
            // dart_paragon_r40: bought before t 76, first throw at t 197).
            tw.paragon_active_at = self.sim_elapsed + 1 + pdef.inactive_frames;
            tw.paragon_first = self.sim_elapsed + 1;
            tw.invested += cost;
            let s = self.tier_sounds(t);
            self.sound(&s.upgrade, None);
            self.sync_abilities(t);
            true
        }
    }

    fn upgrade_bought(&mut self, t: usize, path: usize, cost: f64) -> bool {
        if self.mod_runtime.intercepts & crate::mod_runtime::UPGRADE!=0 {
            let event=serde_json::json!({"tower":crate::mod_entities::tower(self,t),"path":path,"cost":cost,"paragon":false});
            if self.mod_intercept("before_upgrade",event).cancel{return false;}
        }
        // Was a Sun Temple (Some(false)) / True Sun God (Some(true)) before.
        let was_temple = self.tier(t).and_then(|x| x.temple.as_ref()).map(|tp| tp.groups.iter().any(|g| g.two));
        let had_necro = self.tier(t).is_some_and(|x| x.attacks.iter().any(|a| a.necro.is_some()));
        let had_switch = self.tier(t).is_some_and(|x| x.switch_target.is_some());
        let had_submerge = self.tier(t).is_some_and(|x| x.submerge.is_some());
        self.cash -= cost;
        self.stats.cash_spent += cost;
        let tw = &mut self.towers[t];
        tw.tiers[path] += 1;
        tw.invested += cost;
        tw.xp = 0.0; // a bought hero level spends the XP towards it
        // The new model's upgrade state (AnimationBakerStateConfig.isUpgradeAnimation), if any.
        tw.show = Some((crate::anim::UPGRADE_CLIP.into(), 0.0));
        tw.model_clock = Some((self.sim_elapsed as i64, self.round_clock));
        // A new NecromancerTargetTrackWithinRange (Initialise): no track points until the next
        // round starts.
        if !had_necro && self.tier(t).is_some_and(|x| x.attacks.iter().any(|a| a.necro.is_some())) {
            self.towers[t].necro_points = false;
        }
        // SwitchTargetSupplierOnUpgrade (Ice x-x-3: First, Sniper x-5-x: Elite). HYPOTHESIS
        // P-SWITCHTARGET: only when the tier before didn't already have it.
        if !had_switch {
            if let Some(m) = self.tier(t).and_then(|x| x.switch_target.as_deref()).and_then(TargetMode::parse) {
                self.towers[t].target = m;
            }
        }
        self.random_pos_init(t);
        // Submerge.Attatched (VERIFIED-DISASM @0xd99ed0): a new Submerge sets the target type
        // "Submerge" (recording sub_upgrades_r30: the pulse from the frame after the x-x-3 upgrade).
        if !had_submerge && self.tier(t).is_some_and(|x| x.submerge.is_some()) {
            self.towers[t].submerged = true;
            self.towers[t].target = TargetMode::Submerge;
        }
        self.init_point(t);
        self.init_air(t);
        self.sync_abilities(t);
        self.sync_pets(t);
        let id = self.towers[t].id;
        self.temple_upgraded(t, was_temple);
        let Some(t) = self.towers.iter().position(|o| o.id == id) else { return true };
        let s = self.tier_sounds(t);
        self.sound(&s.upgrade, None);
        self.sound(&s.attached, None);
        if self.hero_def(self.towers[t].def).is_some() {
            self.voice(&s.hero_upgrade, VOICE, VOICE_GAP);
        }
        // UpgradeEffect / UpgradeEffectTier5; heroes HeroUpgradeEffect (Lv20 at level 20).
        let level = self.towers[t].tiers[path];
        let fx = match (self.hero_def(self.towers[t].def).is_some(), level) {
            (true, 20) => "HeroUpgradeEffectLv20",
            (true, _) => "HeroUpgradeEffect",
            (false, 5) => "UpgradeEffectTier5",
            _ => "UpgradeEffect",
        };
        self.upgrade_fx.push((self.towers[t].pos, fx));
        true
    }

    /// Buying `path` on tower `t` makes a new temple (Sun Temple: Some(false), True Sun God:
    /// Some(true)): the game asks first (the "Sun Temple" / "True Sun Temple" popup).
    pub fn temple_prompt(&self, t: usize, path: usize) -> Option<bool> {
        self.next_upgrade(t, path)?;
        let tw = &self.towers[t];
        if tw.sub.is_some() || tw.paragon.is_some() || path > 2 || tw.tiers[path] >= 5 {
            return None;
        }
        let mut next = tw.tiers;
        next[path] += 1;
        let tt = self.tower_tiers[tw.def].as_ref()?;
        let two = tt.tiers.get(&format!("{}{}{}", next[0], next[1], next[2]))?.temple.as_ref()?.groups.iter().any(|g| g.two);
        let was = self.tier(t).and_then(|x| x.temple.as_ref()).map(|tp| tp.groups.iter().any(|g| g.two));
        (was != Some(two)).then_some(two)
    }

    /// MonkeyTemple.StartSacrifice / ActOnSacrifice (VERIFIED-DISASM @0xd62xxx / @0xd5cd30): on
    /// becoming a new temple (Sun Temple, then True Sun God) the towers in its range are
    /// destroyed without refund, their worth (Tower.worth: all spent on them) summed per tower
    /// set, and each of the new tier's groups whose set reached its cost applies. Heroes, sub
    /// towers, pets and Paragons are skipped. HYPOTHESIS P-TEMPLE: other Super Monkeys are
    /// skipped too (the special Vengeful / Tech Bot / Anti-Bloon sacrifices aren't modelled) and
    /// "in range" is the tower's centre within the temple's range. Earned groups stay through
    /// later upgrades (the game's tower mutators never expire).
    fn temple_upgraded(&mut self, t: usize, was: Option<bool>) {
        let (id, def, key) = (self.towers[t].id, self.towers[t].def, self.towers[t].tier_key());
        let Some(base) = self.tower_tiers[def].as_ref().and_then(|tt| tt.tiers.get(&key)).cloned() else { return };
        let mut groups = self.towers[t].temple.take().map(|(g, _)| g).unwrap_or_default();
        if let Some(tp) = &base.temple {
            let two = tp.groups.iter().any(|g| g.two);
            if was != Some(two) {
                let (c, r) = (self.towers[t].pos, base.range);
                let mut worth: HashMap<u32, f64> = HashMap::new();
                let mut gone = Vec::new();
                for tw in &self.towers {
                    if tw.id == id || tw.sub.is_some() || tw.paragon.is_some() || tw.deck.is_some() || self.hero_def(tw.def).is_some() {
                        continue;
                    }
                    if self.tower_defs[tw.def].0 == self.tower_defs[def].0 {
                        continue;
                    }
                    if (tw.pos[0] - c[0]).powi(2) + (tw.pos[1] - c[1]).powi(2) > r * r {
                        continue;
                    }
                    let set = self.tower_tiers[tw.def].as_ref().and_then(|tt| tt.tiers.get("000")).map_or(0, |x| x.set);
                    *worth.entry(set).or_default() += tw.invested;
                    gone.push((tw.id, tw.pos));
                }
                // MonkeyTempleModel.towerEffectModel on each tower consumed, effectModel on the temple.
                for &(_, p) in &gone {
                    self.place_fx.push((p, "ConsumeMonkeyEffect".into(), 1.0));
                }
                self.place_fx.push((c, "SunTempleTransformationFx".into(), 1.0));
                // CheckTempleCanFireModel / MonkeyTemple.CanFire: no shots for weaponDelay.
                self.towers[t].mute = self.towers[t].mute.max(tp.delay * FPS);
                // MonkeyTempleModel.animation: the SunTemple controller's SunTempleUpgrade state
                // (TempleBaseUpgrade: the temple opens), SunTempleUpgradeTier5 for True Sun God
                // (TrueSunGodUpgrade: it opens and the god rises out).
                let clip = if two { "TrueSunGodUpgrade" } else { "TempleBaseUpgrade" };
                self.towers[t].show = Some((clip.into(), 0.0));
                let gone: Vec<u64> = gone.into_iter().map(|(i, _)| i).collect();
                self.towers.retain(|o| !gone.contains(&o.id) && !o.sub.as_ref().is_some_and(|s| gone.contains(&s.parent)));
                for g in &tp.groups {
                    if worth.get(&g.set).copied().unwrap_or(0.0) >= g.cost as f64 {
                        groups.push(g.clone());
                    }
                }
            }
        }
        if base.temple.is_some() || !groups.is_empty() {
            let tier = std::sync::Arc::new(Self::temple_tier(&base, &groups));
            if let Some(tw) = self.towers.iter_mut().find(|o| o.id == id) {
                tw.temple = Some((groups, tier));
                }
        }
    }

    /// `base` with the temple groups' mutators: their attacks and supports added, then every
    /// attack's damage / pierce / projectile speed and size / range changed. HYPOTHESIS
    /// P-TEMPLESTAT: ReloadTimeTowerMutator scales the interval by (1 - multiplier), each
    /// group's on top of the others (all mutation ids differ, so all stack).
    fn temple_tier(base: &Tier, groups: &[crate::data::TempleGroup]) -> Tier {
        let mut t = base.clone();
        let (mut dmg, mut pierce, mut speed, mut size, mut range, mut reload, mut wind) = (0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0);
        for g in groups {
            t.attacks.extend(g.attacks.iter().map(|(_, a)| a.clone()));
            t.supports.extend(g.supports.iter().cloned());
            t.round_cash += g.round_cash as f64;
            dmg += g.damage;
            pierce += g.pierce;
            speed += g.speed;
            size += g.size;
            range += g.range;
            reload *= 1.0 - g.reload;
            wind += g.wind;
        }
        fn proj(p: &mut Proj, dmg: f32, pierce: f32, speed: f32, size: f32, wind: f32) {
            if p.damage > 0.0 {
                p.damage += dmg;
            }
            // HYPOTHESIS P-TEMPLEWIND: WindChanceTowerMutator raises the chance of the WindModels
            // already on its projectiles (the temple's main blast carries one at chance 0).
            if let Some(w) = p.wind.as_mut() {
                w.chance = (w.chance + wind).min(1.0);
            }
            p.pierce += pierce;
            p.speed *= 1.0 + speed;
            p.radius *= 1.0 + size;
            for c in &mut p.on_contact {
                proj(&mut c.projectile, dmg, pierce, speed, size, wind);
            }
            for c in &mut p.on_exhaust {
                proj(&mut c.projectile, dmg, pierce, speed, size, wind);
            }
        }
        t.range += range;
        for a in &mut t.attacks {
            if a.range < 1e5 {
                a.range += range;
            }
            for w in &mut a.weapons {
                w.rate *= reload;
                proj(&mut w.projectile, dmg, pierce, speed, size, wind);
            }
        }
        t
    }

    /// Share of the money put into tower `t` that selling it returns. STAND-IN P-CASHBACK:
    /// each other Favored Trades Buccaneer in range (up to maxStacks) adds
    /// cashbackZoneMultiplier, capped at cashbackMaxPercent.
    pub fn sell_fraction(&self, t: usize) -> f64 {
        let p = self.towers[t].pos;
        let mut zones: Vec<&crate::data::Cashback> = Vec::new();
        for (o, tw) in self.towers.iter().enumerate() {
            if o == t {
                continue;
            }
            let Some(cb) = self.tier_of(tw).and_then(|tr| tr.cashback.as_ref()) else { continue };
            let r = self.range(o);
            if (tw.pos[0] - p[0]).powi(2) + (tw.pos[1] - p[1]).powi(2) <= r * r {
                zones.push(cb);
            }
        }
        let Some(first) = zones.first() else { return self.sell_mult };
        let n = zones.len().min(first.stacks.max(1) as usize) as f64;
        (self.sell_mult + first.mult * n).min(first.max)
    }

    /// Nothing done yet this game (map, hero and difficulty can still change).
    pub fn untouched(&self) -> bool {
        self.towers.is_empty() && !self.round_active && self.round == self.start_round
    }

    pub fn sell(&mut self, t: usize) {
        if t>=self.towers.len(){return;}
        if self.mod_runtime.intercepts & crate::mod_runtime::SELL!=0 {
            let event=crate::mod_entities::tower(self,t);
            if self.mod_intercept("before_sell",event).cancel{return;}
        }
        let event=self.mod_runtime.listens("on_tower_sold").then(||crate::mod_entities::tower(self,t));
        let count=self.towers.len();
        self.sell_native(t);
        if self.towers.len()!=count{if let Some(event)=event{self.mod_event_payload("on_tower_sold",None,event);}}
    }
    fn sell_native(&mut self, t: usize) {
        // DisableSellTowerModModel (CHIMPS); deck towers still go with their ship.
        if self.no_sell && self.towers[t].deck.is_none() && self.towers.iter().all(|o| o.deck.is_none_or(|(s, _)| s != self.towers[t].id)) {
            return;
        }
        let s = self.tier_sounds(t);
        self.sound(&s.sell, None);
        // CreateEffectOnSellModel: every tower's is SellEffect.
        if self.towers[t].sub.is_none() {
            self.place_fx.push((self.towers[t].pos, "SellEffect".into(), 1.0));
        }
        // destroyTowersOnAreaWhenSold: towers on its deck go too (STAND-IN: sold, not lost).
        let id = self.towers[t].id;
        while let Some(o) = self.towers.iter().position(|x| x.deck.is_some_and(|(s, _)| s == id)) {
            self.sell_native(o);
        }
        let Some(t) = self.towers.iter().position(|x| x.id == id) else { return };
        let frac = self.sell_fraction(t);
        self.fire_on_destroy(t, true);
        // (Off the process list by an ordered RemoveAt, as an expired one: RootObject.Destroy.)
        let tw = self.towers.remove(t);
        self.cash += (tw.invested * frac).floor() + tw.bank.floor();
        self.towers.retain(|o| !o.sub.as_ref().is_some_and(|s| (s.pet || s.spec.with_parent) && s.parent == tw.id));
    }

    /// Supports reaching a tower of type `def` at `p` (`skip`: the tower itself), grouped by
    /// mutator id and cut to each id's stack limit. HYPOTHESIS P-SUPPORT: a tower is in range
    /// when its footprint touches the support radius (custom radius, else the source's range);
    /// a non-stacking id keeps its strongest instance.
    fn supports_at(&self, def: usize, p: [f32; 2], skip: Option<usize>) -> Vec<(crate::data::Support, u32)> {
        let name = &self.tower_defs[def].0;
        let set = self.tower_tiers[def].as_ref().and_then(|tt| tt.tiers.get("000")).map_or(0, |t| t.set);
        let reach = self.tower_defs[def].1.radius;
        let mut by: HashMap<String, Vec<crate::data::Support>> = HashMap::new();
        for (s, src) in self.towers.iter().enumerate() {
            let Some(tier) = self.tier(s) else { continue };
            if src.placed_step + 1 >= self.steps as u64 {
                continue;
            }
            let now = self.steps as u64;
            for sp in tier.supports.iter().chain(src.temp_supports.iter().filter(|x| x.2 + 1 < now && x.1 > -1.0).map(|(sp, _, _)| sp)) {
                if Some(s) == skip && !sp.on_self {
                    continue;
                }
                if !sp.sets.is_empty() && !sp.sets.iter().any(|&m| m & set != 0) {
                    continue;
                }
                if !sp.bases.is_empty() && !sp.bases.iter().any(|b| b == name) {
                    continue;
                }
                let r = sp.radius.unwrap_or(tier.range) + reach;
                if !sp.global && (src.pos[0] - p[0]).powi(2) + (src.pos[1] - p[1]).powi(2) > r * r {
                    continue;
                }
                by.entry(sp.id.clone()).or_default().push(sp.clone());
            }
        }
        by.into_values()
            .map(|mut v| {
                // Strongest first: lowest interval multiplier, otherwise the largest value.
                v.sort_by(|a, b| {
                    if a.kind == "rate" { a.mult.total_cmp(&b.mult) } else { (b.mult + b.add).total_cmp(&(a.mult + a.add)) }
                });
                let n = (v.len() as u32).min(v[0].stacks.max(1));
                (v.swap_remove(0), n)
            })
            .collect()
    }

    /// Buff icons over tower `t` (BuffIndicatorUi): each reaching support with a buff icon,
    /// with how many of it stack (shown as "x2" ...). Sorted by icon name for a stable layout.
    pub fn buff_icons(&self, t: usize) -> Vec<(String, u32)> {
        let mut v: Vec<(String, u32)> = self
            .supports_at(self.towers[t].def, self.towers[t].pos, Some(t))
            .into_iter()
            .filter_map(|(sp, n)| Some((sp.icon?, n)))
            .collect();
        v.sort();
        v.dedup_by(|a, b| {
            if a.0 == b.0 {
                b.1 += a.1;
                true
            } else {
                false
            }
        });
        v
    }

    fn compute_buffs(&self) -> Vec<Buff> {
        let trade_empire = (0..self.towers.len()).find_map(|t| self.tier(t).and_then(|tr| tr.trade_empire.clone()));
        let flagship = (0..self.towers.len()).filter_map(|t| self.tier(t).and_then(|tr| tr.flagship)).reduce(f32::min);
        (0..self.towers.len())
            .map(|t| {
                let mut b = Buff::default();
                // VigilanteMutator.Mutate (VERIFIED-DISASM @0x126e830): every weapon's rate x
                // loseLifeAttackSpeedBuff, range + loseLifeRangeBuff, while it runs.
                if let (Some(at), Some([rate, range, duration, _])) = (self.towers[t].vigilante_at, self.tier(t).and_then(|tr| tr.vigilante)) {
                    // (TimedMutator: gone once durationFrames have passed; recording desperado_002,
                    // on at 533, the rate back at 1433.)
                    if self.round_clock - at < duration as i64 {
                        b.rate_mult *= rate;
                        b.vig_range += range;
                    }
                }
                // MonkeyBoost (a Power, powers.rs): the rate x rateScale while it lasts.
                b.rate_mult *= self.power_boost(t);
                // Dart Time's TowerTimeSlowMutator: the rate x 1 / towerAttackSpeed.
                b.rate_mult *= self.dart_time_rate();
                // OverrideCamoDetection (VERIFIED-DISASM @0xd4fbd0): a camo-detection mutator on the
                // tower and its sub-towers (recording wizard_052: Monkey Sense's Phoenix takes a
                // Rainbow Camo at rt 3316).
                let parent = self.towers[t].sub.as_ref().and_then(|s| self.towers.iter().position(|o| o.id == s.parent));
                if std::iter::once(t).chain(parent).any(|o| self.tier(o).is_some_and(|tr| tr.detect_camo)) {
                    b.camo = true;
                }
                let sups = if self.towers[t].placed_step + 1 >= self.steps as u64 { Vec::new() } else { self.supports_at(self.towers[t].def, self.towers[t].pos, Some(t)) };
                for (sp, n) in sups {
                    let k = n as f32;
                    match sp.kind.as_str() {
                        "range" => {
                            b.range_mult += sp.mult * k;
                            b.range_add += sp.add * k;
                        }
                        "rate" => b.rate_mult *= sp.mult.powi(n as i32),
                        "pierce" => b.pierce_add += sp.add * k,
                        "pierce_pct" => b.pierce_pct = Some((sp.mult, n)),
                        "speed" => b.speed_mult += sp.mult * k,
                        "camo" => b.camo = true,
                        "immune" => b.immune = Some(b.immune.unwrap_or(u32::MAX) & sp.add as u32),
                        "cash" => b.cash_mult *= sp.mult.powi(n as i32),
                        "income" => b.income_mult *= sp.mult.powi(n as i32),
                        "ability_cd" => b.ability_cd_mult *= sp.mult.powi(n as i32),
                        "unfilter" => b.unfilter = true,
                        "damage" => b.damage_add += sp.add * k,
                        // SubCommanderSupport (VERIFIED-DISASM @0x180d910d0): +pierceIncrease on every
                        // ProjectileModel of the tower, its sub-projectiles too (recording sub_005:
                        // Airburst's splits at 9).
                        "pierce_all" => b.pierce_all += sp.add * k,
                        "damage_mult" => {
                            b.damage_mult *= sp.mult.powi(n as i32);
                            b.damage_add += sp.add * k;
                        }
                        _ => {}
                    }
                }
                self.item_buffs(t, &mut b);
                // HYPOTHESIS P-BREW: rateUp / rangeUp are fractions (+10% attack speed divides
                // the interval by 1.1; +10% range).
                for ab in &self.towers[t].brews {
                    let br = &ab.brew;
                    b.damage_add += br.damage;
                    b.pierce_add += br.pierce;
                    b.range_mult += br.range;
                    if br.rate > 0.0 {
                        b.rate_mult /= 1.0 + br.rate;
                    }
                    b.tag_mods.extend(br.tag_mods.iter().cloned());
                }
                // TurboModel (Turbo Charge): interval x multiplier, +extraDamage.
                if let Some(tb) = &self.towers[t].turbo {
                    b.rate_mult *= tb.mult;
                    b.damage_add += tb.extra_damage;
                }
                if let (true, Some(st)) = (self.towers[t].focus, self.tier(t).and_then(|tr| tr.stance.as_ref())) {
                    b.range_mult += st.range - 1.0;
                    b.rate_mult *= st.rate;
                }
                if let Some(ds) = self.tier(t).and_then(|tr| tr.damage_speed.as_ref()) {
                    b.rate_mult /= 1.0 + self.towers[t].rage.2 as f32 * ds.increase;
                }
                // Overclock / Ultraboost (P-OVERCLOCK: Village range for the Village only).
                let village = self.tower_defs[self.towers[t].def].0 == "MonkeyVillage";
                if let Some((rate, vr, range, _)) = self.towers[t].overclock {
                    b.rate_mult *= rate;
                    b.range_mult += range - 1.0;
                    if village {
                        b.range_mult += vr - 1.0;
                    }
                }
                if let Some((mult, add, _)) = self.towers[t].range_up {
                    b.range_mult += mult - 1.0;
                    b.range_add += add;
                }
                let (n, rate, vr) = self.towers[t].ultra;
                if n > 0 {
                    b.rate_mult *= rate.powi(n as i32);
                    if village {
                        b.range_mult += vr.powi(n as i32) - 1.0;
                    }
                }
                // STAND-IN P-TRADEEMPIRE: Trade Empire's damage buffs go to every merchant ship;
                // STAND-IN P-FLAGSHIP: the Flagship's attack speed to every tower on water.
                if self.tier(t).is_some_and(|tr| tr.merchant) {
                    if let Some(te) = trade_empire.as_ref() {
                        b.damage_add += te.damage;
                        b.tag_mods.push(crate::data::TagMod { tag: "Ceramic".into(), mult: 1.0, add: te.ceramic, all: false });
                        b.tag_mods.push(crate::data::TagMod { tag: "Moabs".into(), mult: 1.0, add: te.moab, all: false });
                    }
                }
                if let Some(f) = flagship.filter(|_| self.water_at(self.towers[t].pos)) {
                    b.rate_mult *= f;
                }
                // Ability projectile boosts (Armor Piercing Shells, Frostbite).
                for (pb, _) in &self.towers[t].boosts {
                    b.damage_add += pb.damage;
                    b.radius_mult *= pb.radius_mult;
                    b.tag_mods.extend(pb.tag_mods.iter().cloned());
                    if let Some(im) = pb.immune {
                        b.immune = Some(b.immune.unwrap_or(u32::MAX) & im);
                    }
                }
                // HYPOTHESIS P-WATERRANGE: PlacementAreaTypeRangeBuffModel adds to the range
                // multiplier like other range buffs when the tower stands on water.
                if let Some(m) = self.tier(t).map(|tr| tr.water_range).filter(|m| *m != 1.0) {
                    if self.water_at(self.towers[t].pos) {
                        b.range_mult += m - 1.0;
                    }
                }
                b.damage_add += self.towers[t].necro_dmg;
                if let Some((fc, _)) = &self.towers[t].fan {
                    b.rate_mult *= fc.rate;
                    b.pierce_add += fc.pierce;
                    b.damage_add += fc.damage;
                    b.fan = Some(fc.clone());
                }
                b
            })
            .collect()
    }

    /// Discount (fraction off) for buying tier `tier` (0 = the tower itself) of `def` at `p`.
    pub fn discount(&self, def: usize, p: [f32; 2], skip: Option<usize>, tier: u32) -> f64 {
        let d: f32 = self
            .supports_at(def, p, skip)
            .iter()
            .filter(|(sp, _)| sp.kind == "discount" && tier <= sp.tier_cap)
            .map(|(sp, n)| sp.mult * *n as f32)
            .sum();
        d.min(1.0) as f64
    }

    pub fn tower_at(&self, p: [f32; 2]) -> Option<usize> {
        self.towers.iter().position(|t| {
            // A sub-tower is picked only when its model allows it (Sentries yes, Phoenixes no).
            if t.sub.as_ref().is_some_and(|s| !s.spec.selectable) {
                return false;
            }
            let r = self.tower_defs[t.def].1.radius + 4.0;
            (t.pos[0] - p[0]).powi(2) + (t.pos[1] - p[1]).powi(2) <= r * r
        })
    }

    pub fn bloon_pos(&self, b: &Bloon) -> [f32; 2] {
        b.pos
    }

    /// Damage bloon `i`; ids of children created go to `out_children` (for P-POP immunity).
    /// Returns the number of layers popped.
    pub(crate) fn mod_bloon_name(&self,kind:usize)->&str{&self.names[kind]}
    pub(crate) fn mod_damage_bloon(&mut self,id:u64,layer:Option<u64>,amount:i32){
        if let Some(i)=self.bloons.iter().position(|b|b.id==id&&layer.is_none_or(|l|l==b.layer)){
            let saved=self.distribute;self.distribute=(true,false);
            self.damage(i,amount,&mut Vec::new());self.distribute=saved;
        }
    }
    /// Bloon.Damage. (A bloon in a Gyrfalcon's claws has blockSpawnChildrenCount: of what it pops
    /// into only the one carrying on is made, whatever hit it; recordings beast_001, beast_032.)
    fn damage(&mut self, i: usize, dmg: i32, out_children: &mut Vec<u64>) -> u64 {
        let held = self.pop_depth == 0 && self.bloons[i].held;
        let (id, start) = (self.bloons[i].id, out_children.len());
        let n = self.damage_inner(i, dmg, out_children);
        // TheBlazingSun.OnBloonDestroy(projectile, bloon, poppedBy, damageResult): the hit bloon's
        // own object gone for good. A degrade keeps the game's bloon object (its first child), so
        // a MOAB popping into Ceramics destroys nothing (recording desperado_502: bloon 426 hit at
        // rt 98 lives on as a Ceramic, stacks unchanged); one hit destroys at most its own bloon
        // (desperado_500: one a throw once every throw kills).
        self.last_destroyed = (n > 0 && !self.bloons.iter().any(|b| b.id == id)) as u64;
        if held {
            for k in out_children[start..].iter().rev().copied().filter(|k| *k != id).collect::<Vec<_>>() {
                if let Some(j) = self.bloons.iter().position(|b| b.id == k) {
                    self.unmake_bloon(j);
                }
            }
            let mut at = 0;
            out_children.retain(|k| {
                at += 1;
                at <= start || *k == id
            });
        }
        n
    }

    fn damage_inner(&mut self, i: usize, mut dmg: i32, out_children: &mut Vec<u64>) -> u64 {
        #[cfg(test)] let _damage_timer=(self.pop_depth==0).then(||sim_profile::Timer::new(0));
        if self.pop_depth==0&&self.mod_runtime.intercepts & crate::mod_runtime::DAMAGE!=0 {
            let event=serde_json::json!({"bloon":crate::mod_entities::bloon(self,i),"damage":dmg});
            let decision=self.mod_intercept("before_damage",event);
            if decision.cancel{return 0;}
            if let Some(damage)=decision.damage{dmg=damage.round() as i32;if dmg==0{return 0;}}
        }
        let hit_camo = self.pop_depth == 0 && self.bloons[i].camo(&self.defs[self.bloons[i].kind]);
        let b = &mut self.bloons[i];
        self.damage_tally += dmg.clamp(0, b.hp.max(0)) as u64;
        b.hp -= dmg;
        let kind = b.kind;
        if b.hp > 0 {
            if dmg > 0 && !self.defs[kind].hit_sounds.is_empty() && self.sound_room(&self.defs[kind].hit_sounds[0], None) {
                let names = self.defs[kind].hit_sounds.clone();
                self.sound(&names, None);
            }
            return 0;
        }
        if let Some(ps) = self.defs[kind].pop_sound.as_ref().filter(|ps| !ps.names.is_empty() && self.sound_room(&ps.names[0], Some((&ps.group, ps.limit)))) {
            let ps = ps.clone();
            self.sound(&ps.names, Some((ps.group, ps.limit, ps.frames, ps.frames_ff)));
        }
        // CreateSoundOnBloonDestroyed (tag Moabs): the hero remarks on a popped MOAB-class bloon.
        if self.defs[kind].tags.iter().any(|t| t == "Moabs") {
            self.hero_voice(|s| &s.moab_pop, VOICE, VOICE_GAP);
        }
        // Bloon.Damage works the whole result out first (BloonDegradeStepper: what's left of the
        // damage goes to every child, layer by layer), then SpawnChildren makes only the bloons
        // that are left: here the layers pop one by one and the survivors are placed after.
        let top = self.pop_depth == 0;
        // (HYPOTHESIS P-GRAVET5: with a Prince of Darkness (GraveyardManager.towerNecroTier5)
        // every degrade is heard, subscribed or not; recording wizard_025: its pool fills as if
        // so, and an undead goes at rt 1853.)
        let grave_t5 = self.towers.iter().any(|tw| tw.tiers[2] >= 5 && self.tower_defs[tw.def].0 == "WizardMonkey");
        let grave_heard = top && (grave_t5 || self.bloons[i].grave_zone.1);
        let grave_id = self.bloons[i].id;
        if top {
            self.pop_nested = false;
            self.pop_first_id = self.next_id;
        } else {
            self.pop_nested = true;
        }
        self.pop_depth += 1;
        let (oc0, len_start) = (out_children.len(), self.bloons.len());
        let b = &mut self.bloons[i];
        let mut popped = 1;
        let overflow = -b.hp;
        let (dist, decamo, path, base, ceiling) = (b.dist, b.decamo, b.path, b.base, b.grow_ceiling);
        let parent_colors = b.grow_colors.clone();
        let regrow = self.grow_kinds[kind].is_some();
        // Grow keeps its timer through a degrade (the game's bloon is the same object): running
        // if the parent was below its ceiling, else it starts now.
        let parent_grow = (self.defs[kind].layer_number < ceiling).then_some(b.grow_t);
        let parent_cell = b.cell_pos;
        let parent_obj = b.obj;
        let parent_order = b.order;
        let parent_born = b.born;
        let parent_height = b.height;
        let (parent_wind, parent_wind_new, parent_wind_seg, parent_wind_direct) = (b.wind, b.wind_new, b.wind_seg, b.wind_direct);
        let parent_wind_model = b.wind_model.clone();
        let parent_id = b.id;
        let lineage: std::sync::Arc<[u64]> = b.lineage.iter().flat_map(|l| l.iter().copied()).chain(std::iter::once(b.layer)).collect();
        let parent_place = if self.grid_on && self.has_grid_rank { self.ggrid.place_of(i) } else { (EMPTY, EMPTY) };
        let pp0 = b.pos;
        // EmitOnPop.PreDegrade (VERIFIED-DISASM @0x100bcd0): ignoringSameFrameDegrade skips this
        // pop and clears (recording ice_320: a Black frozen and popped by the same blast leaves
        // no shards).
        // (Once per Bloon.Damage: the layers it goes through in one hit are worked out, not
        // degraded one by one; recording ice_500, a Black hit for 5 leaves frozen Reds and no
        // shards.)
        // It emits along the bloon's rotation: 0 for one that doesn't turn with the path
        // (recording ice_520: six shards at -150, -90 ... 150, ArcEmission from 0).
        let rot = if self.defs[kind].rotate_to_follow_path { self.bloons[i].rot } else { 0.0 };
        let layer = self.bloons[i].layer;
        let bz = self.bloons[i].height;
        // HYPOTHESIS P-DESTROYEMIT: a child made and popped through by this same hit has no
        // mutators yet (they are copied once the hit is worked out), so it emits nothing; the bloon
        // hit and older ones do (recording desperado_205 rt 89: two Desert Phantom explosions where
        // the two Ceramics went, none from their popped-through children; rt 90: the surviving
        // children of a Rainbow explode later).
        let fresh_child = !top && self.bloons[i].id >= self.pop_first_id;
        let destroy: Vec<_> = if fresh_child { Vec::new() } else { self.bloons[i]
            .effects
            .iter()
            .filter_map(|e| match &e.kind {
                EffectKind::DestroyEmit { proj, emission, .. } => Some((pp0, proj.clone(), emission.clone(), e.owner)),
                _ => None,
            })
            .collect() };
        // CreateEffectOnPop: its effect where the bloon popped (visual).
        if top {
            for e in &self.bloons[i].effects {
                if let EffectKind::PopFx { fx, scale } = &e.kind {
                    self.place_fx.push(([pp0[0], pp0[1]], fx.to_string(), *scale));
                }
            }
        }
        let b = &mut self.bloons[i];
        let mut pop_emits = Vec::new();
        for e in b.effects.iter_mut().filter(|_| top) {
            if let EffectKind::PopEmit { proj, emission, skip, .. } = &mut e.kind {
                if !std::mem::take(skip) {
                    pop_emits.push((pp0, proj.clone(), emission.clone(), e.owner, rot, layer, bz));
                }
            }
        }
        // In the collision pass they come out at once and take part in it (recording ice_520:
        // shards from Reds popped on 2526 hit that frame's new Reds, which emit nothing).
        // So do a DamageOverTime tick's, in the bloon's Process (recording glue_005: a Super
        // Glued MOAB's tick pops it and the splat is out that frame).
        if (self.phase == 0 && self.grid_on) || self.pop_after_move {
            for (pos, proj, emission, owner, rot, layer, z) in pop_emits {
                self.emit_pop(pos, &proj, &emission, owner, rot, layer, z);
            }
        } else {
            self.pop_emits.extend(pop_emits);
        }
        let b = &mut self.bloons[i];
        // Mark / Trojan: pop cash x the mark, + worth per layer; Strip: no children.
        // DistributeCashModel: cash + additionalCash for the layer (a Super Ceramic's 86).
        let mut pop_cash = 1.0f64 + self.extra_cash[kind];
        let mut strip = false;
        for e in &b.effects {
            match e.kind {
                EffectKind::Mark { cash_mult, .. } => pop_cash *= cash_mult as f64,
                EffectKind::Worth { per_tier } => pop_cash += (per_tier * self.defs[kind].layer_number as f32) as f64,
                EffectKind::Strip => strip = true,
                _ => {}
            }
        }
        // The pop happened before this frame's movement (Simulation.Simulate checks collisions
        // first): children start from where the parent was then and move at their own speed.
        // (Moved: this frame's movement is done. Before it, a distance change is a push, not a
        // move; recording boomerang_024: a MOAB Press shot pushes a MOAB back and pops it, and its
        // first Ceramic moves one step that frame, not two.)
        // In the collision pass SpawnChildren.CreatedChildren reads its distanceTraveled now:
        // a push earlier in the pass counts (recording boomerang_paragon_r98: a MOAB pressed back
        // 1, then popped by a glaive, leaves its Ceramics from 1 further back).
        let start = if self.phase == 0 { b.dist } else { b.prev_dist };
        let (mut start_dist, mut moved, mult, parent_tm, parent_fresh) = (start, self.phase >= 2 && b.dist != b.prev_dist, b.mult, b.track_mult, b.fresh);
        // A pop in the bloon's own Process after its Move (DamageOverTime): children start from
        // where it is now, the first (the bloon itself) having moved already (recording
        // boomerang_500: the MOAB's DoT pop leaves Ceramics at 213.08 back by 6.675 each).
        if self.pop_after_move {
            (start_dist, moved) = (b.dist, false);
        }
        // SpawnChildren.CreatedChildren (VERIFIED-DISASM @0x1027440): a popped isMoab bloon first
        // loses every mutator (Bloon.RemoveAllMutators @0xfd4b00) but those marked
        // dontRemoveOnBloonDegrade and BloonManager's ignored ids (recording
        // boomerang_paragon_r98: ZOMGs on fire pop into BFBs that never burn); the children then
        // copy the rest (CopyMutatorsFromBloon) and DamageOverTime.PostDegrade (@0x1003fe0) gives
        // their DamageOverTime the parent's lastTick.
        let moab_pop = self.defs[kind].is_moab;
        // DamageOverTime.PostDegrade (VERIFIED-DISASM @0x180fe54d0), for each of the parent's
        // DamageOverTime behaviours in order, gives the child's first DamageOverTime its own
        // lastTick: the first ends with the last one's.
        // (ApplyMutation applies only the first mutator with an id: a later copy has no behaviour.)
        let parent_tick = b
            .effects
            .iter()
            .enumerate()
            .filter(|(k, e)| !b.effects[..*k].iter().any(|x| x.id == e.id))
            .filter_map(|(_, e)| if let EffectKind::Dot { tick, .. } = e.kind { Some((e.seq, tick)) } else { None })
            .max_by_key(|x| x.0)
            .map(|x| x.1);
        let inherited: Vec<Effect> = b
            .effects
            .iter()
            .filter(|e| e.layers > 1 && (!moab_pop || e.keep || KEPT_ON_MOAB_POP.contains(&e.id.as_str())))
            .map(|e| Effect { layers: e.layers - 1, ..e.clone() })
            .collect();
        // DamageOverTime.OnDestroy (VERIFIED-DISASM @0x1003f90): a DamageOverTime that goes with
        // its mutator still runs CheckTick, so one due now ticks on what the bloon has become
        // (recording boomerang_paragon_r98: a burning BAD popped on its tick frame leaves its
        // first ZOMG 500 down).
        let clock = self.round_clock;
        let destroy_ticks: Vec<(f32, u32, u64, (bool, bool))> = if moab_pop {
            b.effects
                .iter()
                .enumerate()
                .filter(|(k, e)| !(e.keep || KEPT_ON_MOAB_POP.contains(&e.id.as_str())) && !b.effects[..*k].iter().any(|x| x.id == e.id))
                .filter_map(|(_, e)| match e.kind {
                    EffectKind::Dot { damage, interval, tick, immune, ref mods, .. } if clock - tick as i64 >= (interval as f64).round_ties_even() as i64 => {
                        Some((dot_damage(damage, mods, &self.defs[kind].tags), immune, e.owner, e.distribute))
                    }
                    _ => None,
                })
                .collect()
        } else {
            Vec::new()
        };
        self.remove_bloon(i);
        // DistributeCash.PostDegrade (@0x180fea0e0): the cash x Simulation.GetCashModifierFromSimBehaviors
        // (Thrive). HYPOTHESIS P-THRIVEORDER: after Mark / Worth.
        pop_cash *= self.sim_cash_modifier();
        self.earn(pop_cash); // P-CASH
        // GetNonRegrowLayersForLayers: a layer counts 1 + its layerCountOffset.
        popped += self.layer_offset[kind];
        self.popped += 1 + self.layer_offset[kind];
        let pp = self.tracks[path as usize].pos(dist);
        self.pops.push(pp);
        let n_kids = if strip { 0 } else { self.child_kinds[kind].len() };
        if n_kids == 0 {
            self.destroyed_tally += 1;
            self.bloon_destroyed(parent_id, pp0);
            // EmitOnDestroy (recording glue_400: a Liquefied Red the DoT pops leaves a linger).
            // (At the bloon's Position, its height included: recording glue_400, a linger
            // beside the entrance trees at z 20 isn't blocked.)
            for (pos, proj, emission, owner) in destroy {
                let tiers = self.towers.iter().find(|t| t.id == owner).map_or([0; 3], |t| t.tiers);
                self.emit_z = bz;
                self.emit(pos, 0.0, &emission, &proj, None, tiers, owner);
                self.emit_z = 0.0;
            }
        }
        // SpawnChildren.CreatedChildren (VERIFIED-DISASM @0x1027440): the first child is the
        // parent itself (it moves on, at its new speed); child k sits k x min(26.7 / count, 8.6)
        // behind (not below 0) and, being new, waits for the next frame to move.
        let spacing = (26.7f32 / n_kids.max(1) as f32).min(8.6);
        for n in 0..n_kids {
            let c = self.child_kinds[kind][n];
            let d0 = (start_dist - n as f32 * spacing).max(0.0);
            let d1 = if moved && n == 0 { (d0 + speed_frames(self.defs[c].speed) * parent_tm * 1.0 * mult).max(0.0) } else { d0 };
            if n == 0 {
                self.spawn_parent_obj = Some(parent_obj);
            }
            self.ghost_cell = None;
            let mut id = self.spawn_at(c, d1, path);
            if let Some(ch) = self.bloons.last_mut() {
                // The first child is the parent object: projectiles that hit it already, and
                // throws locked onto it, still know it.
                if n == 0 {
                    // (spawn_on listed it under a new id: it is the parent, still listed.)
                    let fresh = ch.id;
                    ch.id = parent_id;
                    id = parent_id;
                    if let Some(&(p, k)) = self.path_rank.get(&parent_id) {
                        // The parent's entry carries on; drop the one just made (wherever it is:
                        // in a nested pop it isn't always the last).
                        self.path_lists[p as usize][k as usize].1 = true;
                        if let Some((p, k)) = self.path_rank.remove(&fresh) {
                            let l = &mut self.path_lists[p as usize];
                            l.remove(k as usize);
                            for (n, e) in l.iter().enumerate().skip(k as usize) {
                                self.path_rank.insert(e.0, (p, n as u32));
                            }
                        }
                    } else if let Some((p, k)) = self.path_rank.remove(&fresh) {
                        // A parent made and popped by this same hit has no entry: its object
                        // carries on as this child, listed where the child was added
                        // (recording dart_400: a Lead's four Pinks are all listed).
                        self.path_lists[p as usize][k as usize].0 = parent_id;
                        self.path_rank.insert(parent_id, (p, k));
                    }
                }
                let ch = self.bloons.last_mut().unwrap();
                ch.lineage = Some(lineage.clone());
                ch.prev = self.tracks[path as usize].bloon_pos(d0);
                ch.prev_dist = d0;
                // (The others are made after this frame's movement, so they wait already.)
                if n == 0 {
                    ch.track_mult = parent_tm;
                    ch.order = parent_order;
                    // The parent's object, so its place in the process list (recording
                    // boomerang_400: a Black's Pink moves before a later glaive aims at it).
                    ch.born = parent_born;
                    ch.height = parent_height;
                }
                let j = self.bloons.len() - 1;
                out_children.push(id);
                if self.grid_on && self.has_grid_rank {
                    // (The others only join the grid when it's next filled, unless their object's
                    // dead bloon is still in it: see spawn_at.)
                    if n == 0 {
                        self.ggrid.take_place(j, parent_place);
                    }
                }
            }
            let child_tags = &self.defs[c].tags;
            let keep: Vec<Effect> = inherited
                .iter()
                .filter(|e| !matches!(e.kind, EffectKind::Freeze) || !child_tags.iter().any(|t| t == "Moabs"))
                .filter(|e| n == 0 || !matches!(e.kind, EffectKind::DestroyEmit { dont_copy: true, .. }))
                .cloned()
                .map(|mut e| {
                    // The other children's mutators are new (CopyMutatorsFromBloon): their
                    // EmitOnPop is initialised again. The first is the parent object, its
                    // behaviour as it was (recording ice_520: a Black's copy made and degraded
                    // again by one hit leaves Reds that don't shatter the next frame).
                    if let EffectKind::PopEmit { ignore, skip, .. } = &mut e.kind {
                        if n > 0 {
                            *skip = *ignore;
                        }
                    }
                    // Mutable.CopyMutatorsFrom (@0x180f35ac0) passes the source TimedMutator's removeAt
                    // (0x18) as the copy's duration, which becomes its totalDuration (0x28); a refresh
                    // (AddMutator: removeAt < new duration -> totalDuration) gives it that back
                    // (recording ice_025: a re-frozen copy thaws when its parent would have).
                    // INFERRED, freezes only: glue_005 / glue_025 need a copy's glue mutators to get
                    // their full lifespan back (see HANDOFF, Ice notes).
                    if n > 0 && matches!(e.kind, EffectKind::Freeze) {
                        e.total = e.frames_left;
                    }
                    // A copy's CarryProjectile is new: Attatched makes its own projectiles
                    // (recording ice_005: an Icicles Rainbow's Blacks and Whites each carry one).
                    if let EffectKind::Carry { uids, .. } = &mut e.kind {
                        if n > 0 {
                            uids.clear();
                        }
                    }
                    e
                })
                .collect();
            // The first child is the parent object, its mutator list as it was; the others copy it.
            let mut keep = if n == 0 { keep } else { copy_order(keep) };
            // Each child's DamageOverTime behaviours are made anew for its model (Initialise:
            // lastTick = now + initialDelay, or -999999999 with triggerImmediate), then
            // PostDegrade gives the first the parent's last lastTick (recording
            // glue_upgrades_r30: a Pink's Dissolver gets Corrosive Glue's old lastTick on every
            // pop and pops a layer a frame, 968..971).
            // (The first child is the parent object: it keeps its behaviours, their lastTicks
            // and order; a new one makes them in its mutator order.)
            let now = self.round_clock as f32;
            if n > 0 {
                for e in &mut keep {
                    e.seq = self.effect_seq;
                    self.effect_seq += 1;
                    if let EffectKind::Dot { tick, start, .. } = &mut e.kind {
                        *tick = start.map_or(-999999999.0, |s| now + s);
                    }
                }
            }
            let first = (0..keep.len())
                .filter(|&k| !keep[..k].iter().any(|x| x.id == keep[k].id) && matches!(keep[k].kind, EffectKind::Dot { .. }))
                .min_by_key(|&k| keep[k].seq);
            if let (Some(k), Some(t)) = (first, parent_tick) {
                if let EffectKind::Dot { tick, .. } = &mut keep[k].kind {
                    *tick = t;
                }
            }
            let child = self.bloons.last_mut().unwrap();
            // Made in the collision pass, before this frame's movement: new objects, they move
            // from the next frame (the first child is the parent object and moves on, unless that
            // was made this frame too: recording dart_400, a Lead's second Black's Pinks).
            if self.grid_on {
                child.fresh = n > 0 || parent_fresh;
            }
            // HYPOTHESIS P-INSTANTCHILD: new objects made by an instant shot's pop (after the
            // towers act) aren't in the next frame's CollisionGrid until they have moved
            // (recording sniper_500: a MOAB's three new Ceramics are hit on 170, not 169).
            if self.in_instant && n > 0 {
                child.unplaced = true;
            }
            // (The first child is the parent object: a MoveWithWind on it goes on carrying it.
            // The others copy the mutator (CopyMutatorsFromBloon @0xfb0f40 -> Mutable.
            // CopyMutatorsFrom @0xf35ac0: AddMutator, ApplyMutation), so each gets its own
            // MoveWithWind.Attatched: a distance draw and its push from the parent's spot and
            // distance; recordings ninja_320 rt 3149/3150, ninja_025 rt 279: one synced draw per
            // new object a carried bloon's pop makes.)
            let mut copy_draw = None;
            if parent_wind.is_some() {
                child.wind = parent_wind;
                child.wind_new = if n == 0 { parent_wind_new } else { true };
                // (A degrade files it on its path again at its distanceTraveled now: recording
                // ninja_420_last rt 3018, a carried Black popped to a Red at 173.6 is no longer a
                // target from the segment it was blown off; ninja_020 rt 421, one not popped is.)
                child.wind_seg = parent_wind_seg.map(|_| dist);
                child.wind_direct = n == 0 && parent_wind_direct;
                child.wind_model = parent_wind_model.clone();
                child.pos = pp0;
                child.prev = pp0;
                if n > 0 {
                    child.dist = dist;
                    copy_draw = parent_wind_model.clone();
                }
            }
            if let Some(w) = copy_draw {
                let ck = self.bloons.last().unwrap().kind;
                let mut back = (self.synced_double() as f32) * (w.max - w.min) + w.min;
                if w.tag_scale > 0.0 && w.tags.iter().any(|t| self.defs[ck].tags.contains(t)) {
                    back *= w.tag_scale;
                }
                let r = self.defs[ck].radius;
                let sd = self.track_ends.get(self.bloons.last().unwrap().path as usize).and_then(|e| e.0);
                let child = self.bloons.last_mut().unwrap();
                child.wind_seg = Some(child.dist);
                child.dist -= back;
                if let Some(sd) = sd {
                    if child.dist <= sd - r * 1.25 {
                        child.dist = sd - r * 1.25;
                    }
                }
            }
            let child = self.bloons.last_mut().unwrap();
            child.effects = keep;
            child.decamo = decamo;
            child.base = base;
            // (Every child keeps the parent's colours: recordings, Pinks born of a WhiteRegrow
            // grow back into Whites, those of a Black into Blacks.)
            child.grow_colors = parent_colors.clone();
            child.grow_fresh = true;
            // HYPOTHESIS P-GROWCHILD: children of a regrow bloon grow back up to its ceiling
            // (starting a fresh timer).
            if regrow {
                child.grow_ceiling = child.grow_ceiling.max(ceiling);
                if let Some(t) = parent_grow {
                    child.grow_t = t;
                }
            }
            child.cell_pos = self.ghost_cell.take().unwrap_or(parent_cell);
            // Path.bloonsList (VERIFIED by recording): popped bloons leave by swapping in the last
            // one, new ones are appended, and the first child is the parent object, still in its
            // place.
            let mut j = self.bloons.len() - 1;
            if n == 0 && i < j {
                self.swap_bloons(i, j);
                j = i;
            }
            // Bloon.RecieveDamage / BloonDegradeStepper.UpdateDamageResult (VERIFIED-DISASM
            // @0xfd4810 / @0xfe7560): damage past the layer goes on only when the damage's
            // distributeToChildren is set and the bloon's distributeDamageToChildren (or the
            // damage's overrideDistributeBlocker); else it is cut to the layer's health
            // (recording dart_005: a crit pops a MOAB, its four Ceramics come out whole;
            // boomerang_500: Glaive Lord's orbit, distributeToChildren off, pops a Ceramic into
            // two whole Rainbows).
            let (dist_on, dist_override) = self.distribute;
            if overflow > 0 && dist_on && (self.defs[kind].distribute_damage_to_children || dist_override) {
                popped += self.damage(j, overflow, out_children);
            }
        }
        self.pop_depth -= 1;
        if top {
            self.track_pops(kind, hit_camo, popped);
            // Bloon.Degrade raises OnBloonDegrade once per Damage, with the DamageResult.
            if grave_heard {
                self.necro_gather(popped as i64, pp0);
                if let Some(b) = self.bloons.iter_mut().find(|b| b.id == grave_id) {
                    b.grave_zone.1 = false;
                }
            }
        }
        if top && self.pop_nested {
            // SpawnChildren.CreatedChildren over the final list: the first is the parent object
            // (it moves on at its new speed), the k-th sits k x min(26.7 / count, 8.6) behind.
            let mut seen = std::collections::HashSet::new();
            let ids: Vec<u64> = out_children[oc0..].iter().copied().filter(|id| seen.insert(*id)).collect();
            // What this pop made sits in the parent's slot or past where storage ended.
            let at: std::collections::HashMap<u64, usize> = std::iter::once(i)
                .chain(len_start.saturating_sub(1)..self.bloons.len())
                .filter(|&k| k < self.bloons.len() && seen.contains(&self.bloons[k].id))
                .map(|k| (self.bloons[k].id, k))
                .collect();
            let finals: Vec<usize> = ids.iter().filter_map(|id| at.get(id).copied()).collect();
            let sp = (26.7f32 / finals.len().max(1) as f32).min(8.6);
            for (k, &j) in finals.iter().enumerate() {
                let c = self.bloons[j].kind;
                let d0 = (start_dist - k as f32 * sp).max(0.0);
                let d1 = if moved && k == 0 { (d0 + speed_frames(self.defs[c].speed) * parent_tm * 1.0 * mult).max(0.0) } else { d0 };
                // Not held at the entrance: that waits for the child's first Bloon.Move
                // (recording dart_400: a Lead's fourth Pink at 169.2, then 173.6).
                let dist = d1;
                let tr = &self.tracks[path as usize];
                let (pos, prev, tm) = (tr.bloon_pos(dist), tr.bloon_pos(d0), if k == 0 { parent_tm } else { tr.speed_mult(dist) });
                let b = &mut self.bloons[j];
                // (A bloon MoveWithWind carries keeps its place and distance; it walks to its
                // track spot.)
                if b.wind.is_none() {
                    b.dist = dist;
                    b.pos = pos;
                    b.prev = prev;
                }
                b.prev_dist = d0;
                b.track_mult = tm;
            }
        }
        // The copies' carried projectiles, where the degrade put them.
        if top {
            let made: Vec<usize> = (0..self.bloons.len())
                .filter(|&k| self.bloons[k].effects.iter().any(|e| matches!(&e.kind, EffectKind::Carry { uids, .. } if uids.is_empty())))
                .collect();
            for k in made {
                self.attach_carries_as(k, Some(layer));
            }
        }
        // (They come once the degrade has placed its children, each a pop of its own: recording
        // glue_205, a Corrosive MOAB's tick takes its first Ceramic and the other three stay
        // spaced as four.)
        for (damage, immune, owner, distribute) in destroy_ticks {
            let Some(j) = self.bloons.iter().position(|b| b.id == parent_id) else { break };
            if immune & self.bloons[j].properties(&self.defs[self.bloons[j].kind]) != 0 {
                continue;
            }
            let (tally0, saved) = (self.damage_tally, self.distribute);
            self.distribute = distribute;
            let n = self.damage(j, damage.round() as i32, out_children);
            self.distribute = saved;
            let dealt = self.damage_tally - tally0;
            if let Some(tw) = self.towers.iter_mut().find(|t| t.id == owner) {
                tw.pops += n;
                tw.sun_destroyed(self.last_destroyed);
                tw.damage += dealt;
                self.stats.pops += n;
                self.stats.damage += dealt;
            }
        }
        popped
    }

    /// SpiritOfTheForestModel. HYPOTHESIS P-SPIRIT: every bloon on the map takes the zone for
    /// its distance from the Druid (closeRange / middleRange / anywhere else on the track) each
    /// zone interval; vines aren't drawn yet. Several Druids each deal theirs.
    fn spirit_of_the_forest(&mut self) {
        let frame = self.round_frame as f32;
        let druids: Vec<([f32; 2], u64, crate::data::Spirit)> = (0..self.towers.len())
            .filter_map(|t| Some((self.towers[t].pos, self.towers[t].id, self.tier(t)?.spirit.clone()?)))
            .collect();
        if druids.is_empty() {
            return;
        }
        let mut kids = Vec::new();
        // Highest index first: a pop swaps the last bloon (or a new child) into the hole.
        for i in (0..self.bloons.len()).rev() {
            let Some(b) = self.bloons.get(i) else { continue };
            if !on_map(b.pos, 0.0) {
                continue;
            }
            let def = &self.defs[b.kind];
            let (mut total, mut owner) = (0i32, 0u64);
            for (c, id, sp) in &druids {
                let d = (b.pos[0] - c[0]).hypot(b.pos[1] - c[1]);
                let Some(z) = sp.zones.get(if d <= sp.close { 0 } else if d <= sp.middle { 1 } else { 2 }) else { continue };
                if frame % (z.interval * FPS).max(1.0) >= 1.0 || z.immune & b.properties(def) != 0 {
                    continue;
                }
                let dmg = if z.tags.iter().any(|tg| def.tags.contains(tg)) { z.damage * z.mult + z.add } else { z.damage };
                total += dmg.round() as i32;
                if owner == 0 {
                    owner = *id;
                }
            }
            if total > 0 {
                let tally0 = self.damage_tally;
                let n = self.damage(i, total, &mut kids);
                let self_tally_dealt = self.damage_tally - tally0;
                if let Some(tw) = self.towers.iter_mut().find(|t| t.id == owner) {
                    tw.pops += n;
                tw.sun_destroyed(self.last_destroyed);
                    tw.damage += self_tally_dealt;
                    self.stats.pops += n;
                    self.stats.damage += self_tally_dealt;
                }
            }
        }
    }

    /// Grow.Process (VERIFIED-DISASM @0x1010790): unless a GrowBlock is on it (freezes carry
    /// one: FreezeModel.growBlockModel), every rate x 60 frames a regrow bloon below its ceiling
    /// layer becomes its growToId bloon at full health (Regenerate: UpdateRootModel +
    /// ResetHealth). Its effects and decamo stay.
    fn regrow(&mut self) {
        // Grow Blocker zones (AddBehaviorToBloonInZoneModel + GrowBlockModel).
        let blocks: Vec<([f32; 2], f32)> = (0..self.towers.len())
            .filter_map(|t| {
                let r = self.tier(t)?.grow_block?;
                Some((self.towers[t].pos, if r > 0.0 { r } else { self.range(t) }))
            })
            .collect();
        // (Bloons made this step, by the collisions or the towers' instant hits after them, aren't in
        // the process list yet: no Grow tick this frame; recording desperado_300 rt 2806, a Zebra's
        // Pinks popped by the pistol regrow 180 frames later, at 2986; dart_400: Pinks made on frame 3181
        // regrow on 3361.)
        let born = self.step_first_id..self.next_id;
        for i in 0..self.bloons.len() {
            let b = &self.bloons[i];
            let Some((to, every)) = self.grow_kinds[b.kind] else { continue };
            if born.contains(&b.id) {
                continue;
            }
            // (Its first Process on a model sets the colours, growBlock or not: a bloon a hit
            // takes through a White to its Pinks never had the White's; recording dart_400 rt
            // 3361, a Zebra's four Pinks all grow into Blacks.)
            if b.grow_fresh {
                let b = &mut self.bloons[i];
                b.grow_fresh = false;
                grow_init(b, &self.defs, &self.names);
            }
            let b = &self.bloons[i];
            if b.frozen() || blocks.iter().any(|(c, r)| (b.pos[0] - c[0]).powi(2) + (b.pos[1] - c[1]).powi(2) <= r * r) {
                continue;
            }
            let b = &mut self.bloons[i];
            // Grow (recorded): no timer at the ceiling; below it every `rate` (180 frames at
            // 3 s), counted from the first degrade and restarted by each growth.
            if self.defs[b.kind].layer_number >= b.grow_ceiling {
                b.grow_t = every;
                continue;
            }
            b.grow_t = b.grow_t.saturating_sub(1);
            if b.grow_t > 0 {
                continue;
            }
            b.grow_t = every;
            // Grow.Process (VERIFIED-DISASM @0xff1b70): growTo is growToId, or with the colours
            // of an overrideChildWithColor model and a bloon whose baseId has the child colour,
            // growToId with its bloon's baseId replaced by the parent colour (recording
            // ninja_003 rt 1976: a WhiteRegrow's Pink grows back into a White, not a Black).
            let base = |n: &str| n.replace("Regrow", "").replace("Camo", "").replace("Fortified", "");
            let to = match &b.grow_colors {
                Some((c, p)) if base(&self.names[b.kind]).contains(c.as_str()) => {
                    let name = self.names[to].replace(&base(&self.names[to]), p);
                    self.by_name.get(&name).copied().unwrap_or(to)
                }
                _ => to,
            };
            b.kind = to;
            b.hp = self.defs[to].max_health;
            b.grow_fresh = true;
        }
    }

    /// HYPOTHESIS P-TRACKTARGET: TargetTrackModel picks a uniformly random track distance whose
    /// point is in range, then offsets it by up to maxOffset.
    /// HYPOTHESIS P-BREWTARGET: a brew goes to the closest attacking tower in range that isn't
    /// on the ignore list and doesn't already have it (or a brew that replaces it).
    fn brew_target(&self, t: usize, brew: &Brew, range: f32) -> Option<usize> {
        let pos = self.towers[t].pos;
        (0..self.towers.len())
            .filter(|&o| o != t)
            .filter_map(|o| {
                let ot = &self.towers[o];
                let (name, def) = &self.tower_defs[ot.def];
                let dd = (ot.pos[0] - pos[0]).powi(2) + (ot.pos[1] - pos[1]).powi(2);
                let r = range + def.radius;
                let has = ot.brews.iter().any(|a| a.brew.id == brew.id || a.brew.replaces.contains(&brew.id));
                let attacks = self.tier(o).is_some_and(|ti| ti.attacks.iter().any(|a| a.brew.is_none() && !a.weapons.is_empty()));
                (dd <= r * r && !has && attacks && !brew.ignore.contains(name)).then_some((dd, o))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, o)| o)
    }

    /// TargetTrack (GetTargets <d__8>.MoveNext VERIFIED-DISASM @0xe0b7d0; TrackTargetSupplier
    /// .GeneratePoints @0xe09ae0): on its first Process the attack lists the points of its
    /// visible in-range path segments and shuffles them with syncedRandom (forward
    /// Fisher-Yates, Next(i, count), @0x18bce60); each target is the next point (wrapping), off
    /// by NextDouble x maxOffset - maxOffset x 0.5 in x, then y (recording ninja_002: the
    /// first Caltrops at rt 1).
    /// `base`: the attack's own range (the tower's buffs at the time apply).
    fn supplied_track_point(&mut self, t: usize, c: [f32; 2], base: f32, off: f32, no_moab: bool, through_walls: bool) -> Option<(f32, [f32; 2], u64)> {
        // (A model update remakes the supplier's points: the tower's support mutators changing,
        // recording ninja_032, a second shuffle at rt 2 once Shinobi Tactics is on.)
        self.towers[t].track_args = Some((c, base, no_moab, through_walls));
        self.track_supply_refresh(t);
        let (pts, cur, _) = self.towers[t].track_supply.as_ref().unwrap();
        if pts.is_empty() {
            return None;
        }
        let i = if *cur >= pts.len() { 0 } else { *cur };
        let p = pts[i];
        let x = self.synced_double() as f32 * off - off * 0.5;
        let y = self.synced_double() as f32 * off - off * 0.5;
        if let Some(s) = self.towers[t].track_supply.as_mut() {
            s.1 = i + 1;
        }
        // (The game's y is ours negated.)
        Some((0.0, [p[0] + x, p[1] - y], 0))
    }

    /// TrackTargetSupplier.GeneratePoints when the tower's model is new (placed, upgraded, or its
    /// support mutators changed): the Close-ordered visible segment points, shuffled.
    fn track_supply_refresh(&mut self, t: usize) {
        let Some((c, base, no_moab, through_walls)) = self.towers[t].track_args else { return };
        // (Under the model's range now: recording ninja_052, Sabotage's range buff gives 18 points.)
        let range = self.buffs.get(t).map_or(base, |b| b.range(base));
        let tiers = self.towers[t].tiers;
        let sig = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            if let Some(b) = self.buffs.get(t) {
                (b.rate_mult.to_bits(), b.pierce_add.to_bits(), b.pierce_pct.map(|(m, n)| (m.to_bits(), n)), b.range_mult.to_bits(), b.range_add.to_bits(), b.vig_range.to_bits(), b.damage_add.to_bits(), b.speed_mult.to_bits()).hash(&mut h);
            }
            h.finish()
        };
        if self.towers[t].track_supply.as_ref().is_none_or(|s| s.2 != tiers) || self.towers[t].track_buffs != sig {
            self.towers[t].track_buffs = sig;
            let mut pts: Vec<[f32; 2]> = self.visible_segments(c, range, no_moab, through_walls).into_iter().map(|(pi, k)| self.track_segs[pi as usize][k]).collect();
            // (The list is the Close one: squared distance from the tower, a stable OrderBy;
            // recording ninja_002: with it the shuffle gives the game's -134.1, -99.1, -113.1,
            // -92.1, -106.1 Caltrops in turn.)
            let tp = self.towers[t].pos;
            pts.sort_by(|a, b| ((a[0] - tp[0]) * (a[0] - tp[0]) + (a[1] - tp[1]) * (a[1] - tp[1])).total_cmp(&((b[0] - tp[0]) * (b[0] - tp[0]) + (b[1] - tp[1]) * (b[1] - tp[1]))));
            let n = pts.len();
            for i in 0..n {
                let j = i + (self.synced_double() * (n - i) as f64) as usize;
                pts.swap(i, j.min(n - 1));
            }
            // HYPOTHESIS P-TRACKCURSOR: the cursor carries on into the new points (recording
            // ninja_032 rt 216: the second Caltrops go to the remade list's second point).
            let cur = self.towers[t].track_supply.as_ref().map_or(0, |s| s.1);
            self.towers[t].track_supply = Some((pts, cur, tiers));
        }
    }

    fn track_point(&mut self, c: [f32; 2], range: f32, off: f32) -> Option<(f32, [f32; 2], u64)> {
        let total: f32 = self.tracks.iter().map(Track::length).sum();
        for _ in 0..32 {
            // A uniform spot over all the tracks' combined length.
            let mut d = self.rand() * total;
            let mut k = 0;
            while k + 1 < self.tracks.len() && d > self.tracks[k].length() {
                d -= self.tracks[k].length();
                k += 1;
            }
            let p = self.tracks[k].pos(d);
            if (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) <= range * range && on_map(p, 0.0) {
                let (a, r) = (self.rand() * std::f32::consts::TAU, self.rand() * off);
                return Some((0.0, [p[0] + r * a.cos(), p[1] + r * a.sin()], 0));
            }
        }
        None
    }

    /// TargetTrackOrDefault with useClosestTrack (VERIFIED-DISASM GeneratePoints @0xe07640,
    /// <GetTrackTargets>d__12.MoveNext @0xe0bc60): closestPoint is the point of the first of
    /// Attack.GetVisibleInRangePathSegments for the Close key, whose list SetVisiblePathSegments
    /// (@0xdafaa0, b__48_1) orders by squared distance from the tower (a stable OrderBy); none
    /// found: no target. Offset per axis by NextDouble * maxTrackOffset - maxTrackOffset * 0.5
    /// (x, then y).
    fn first_visible_track_point(&mut self, tower: [f32; 2], c: [f32; 2], range: f32, off: f32, no_moab: bool, through_walls: bool) -> Option<(f32, [f32; 2], u64)> {
        let d2 = |q: [f32; 2]| (q[0] - tower[0]) * (q[0] - tower[0]) + (q[1] - tower[1]) * (q[1] - tower[1]);
        let mut best: Option<([f32; 2], f32)> = None;
        for (pi, k) in self.visible_segments(c, range, no_moab, through_walls) {
            let q = self.track_segs[pi as usize][k];
            if best.is_none_or(|(_, b)| d2(q) < b) {
                best = Some((q, d2(q)));
            }
        }
        let (p, _) = best?;
        let x = self.synced_double() as f32 * off - off * 0.5;
        let y = self.synced_double() as f32 * off - off * 0.5;
        // (The game's y is ours negated.)
        Some((0.0, [p[0] + x, p[1] - y], 0))
    }

    /// STAND-IN P-CLOSETRACK: CloseTargetTrackModel: the in-range track point nearest the tower
    /// (sampled every 2 units), offset by up to maxOffset.
    fn closest_track_point(&mut self, c: [f32; 2], range: f32, off: f32) -> Option<(f32, [f32; 2], u64)> {
        let mut best: Option<([f32; 2], f32)> = None;
        for tr in &self.tracks {
            let len = tr.length();
            let mut d = 0.0;
            while d <= len {
                let p = tr.pos(d);
                let dd = (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2);
                if dd <= range * range && best.is_none_or(|(_, b)| dd < b) {
                    best = Some((p, dd));
                }
                d += 2.0;
            }
        }
        let (p, _) = best?;
        let (a, r) = (self.rand() * std::f32::consts::TAU, self.rand() * off);
        Some((0.0, [p[0] + r * a.cos(), p[1] + r * a.sin()], 0))
    }

    /// Whether one more sound of this group (or family, from `name`) could still start when
    /// the audio drains the queue: it admits at most `limit` per group, one per family (their
    /// gap is above zero) and TOTAL_LIMIT overall, so the rest needn't be queued at all.
    fn sound_room(&self, name: &str, group: Option<(&str, u32)>) -> bool {
        if self.sfx.len() >= crate::audio::TOTAL_LIMIT {
            return false;
        }
        match group {
            Some((g, limit)) => {
                self.sfx.iter().filter(|s| s.group.as_ref().is_some_and(|x| x.0 == g)).count() < limit.max(1) as usize
            }
            None => {
                let fam = name.trim_end_matches(|c: char| c.is_ascii_digit());
                !self.sfx.iter().any(|s| s.group.is_none() && s.name.trim_end_matches(|c: char| c.is_ascii_digit()) == fam)
            }
        }
    }

    /// BloonCauseOfDefeat.UpdateBloonLeaks (InGame's leak handler: BloonModel.GetTotalLeakDamage,
    /// Simulation.time.elapsed): kept while under 30, else in place of the oldest once that is
    /// more than 300 frames old; sorted by frame.
    fn record_leak(&mut self, kind: usize) {
        let Some(id) = self.by_name.iter().find(|(_, k)| **k == kind).map(|(n, _)| n.clone()) else { return };
        let entry = (id, self.total_leak(kind), self.sim_elapsed);
        if self.leaks.len() < 30 {
            self.leaks.push(entry);
        } else if self.sim_elapsed.saturating_sub(self.leaks[0].2) > 300 {
            self.leaks[0] = entry;
        }
        self.leaks.sort_by_key(|l| l.2);
    }

    /// BloonCauseOfDefeat.FinalizeDeath at the defeat: leaks of the last 300 frames, summed by
    /// bloon, by damage ascending, the three largest (the defeat screen's Bloons Leaked).
    pub fn cause_of_defeat(&self) -> Vec<(String, f32)> {
        let mut sums: Vec<(String, f32)> = Vec::new();
        for (id, dmg, at) in &self.leaks {
            if self.sim_elapsed.saturating_sub(*at) > 300 {
                continue;
            }
            match sums.iter_mut().find(|(n, _)| n == id) {
                Some(e) => e.1 += dmg,
                None => sums.push((id.clone(), *dmg)),
            }
        }
        sums.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let cut = sums.len().saturating_sub(3);
        sums.drain(..cut);
        sums
    }

    /// Bloon.UpdateTracking for a hit on a bloon of `kind` that popped `n` layers.
    fn track_pops(&mut self, kind: usize, camo: bool, n: u64) {
        if n == 0 {
            return;
        }
        let tags = &self.defs[kind].tags;
        let has = |t: &str| tags.iter().any(|x| x == t);
        let st = &mut self.stats;
        if camo {
            st.camo_pops += n;
        }
        if has("Grow") {
            st.regrow_pops += n;
        }
        let boss = has("Boss");
        for (tag, c) in [("Lead", &mut st.lead), ("Ceramic", &mut st.ceramic), ("Moab", &mut st.moab), ("Bfb", &mut st.bfb), ("Zomg", &mut st.zomg), ("Ddt", &mut st.ddt)] {
            if has(tag) {
                *c += 1;
            }
        }
        if has("Bad") && !boss {
            st.bad += 1;
        }
        if boss {
            st.boss += 1;
        }
    }

    /// Pops `bloons[i]` in one go: it and every layer inside it count as popped (no children
    /// spawn). Returns the number of pops.
    fn destroy_whole(&mut self, i: usize) -> u64 {
        let n = self.pops_in[self.bloons[i].kind];
        let camo = self.bloons[i].camo(&self.defs[self.bloons[i].kind]);
        self.track_pops(self.bloons[i].kind, camo, n);
        let p = self.bloons[i].pos;
        self.damage_tally += n;
        self.remove_bloon(i);
        self.popped += n;
        self.pops.push(p);
        n
    }

    /// HYPOTHESIS P-TAKEDOWN: MOAB / ZOMG Takedown hooks the strongest MOAB-class bloons on the
    /// map (not camo unless the tower sees camo, not the excluded tags), one hook each, a ZOMG
    /// `zomg` hooks and a BAD `bad`, until the hooks run out, and destroys them whole at once
    /// (the game reels them in first). Pays their pop cash x multiplier + additive.
    fn takedown(&mut self, t: usize, td: &crate::data::Takedown) {
        let camo = self.buffs.get(t).is_some_and(|b| b.camo) || self.tier(t).is_some_and(|tr| tr.attacks.iter().any(|a| a.camo));
        let mut cands: Vec<(f32, u64, u32)> = self
            .bloons
            .iter()
            .filter(|b| {
                let d = &self.defs[b.kind];
                d.tags.iter().any(|x| x == "Moabs") && !td.exclude.iter().any(|x| d.tags.contains(x)) && (camo || !b.camo(d)) && on_map(b.pos, 0.0)
            })
            .map(|b| {
                let d = &self.defs[b.kind];
                let cost = if d.tags.iter().any(|x| x == "Bad") { td.bad } else if d.tags.iter().any(|x| x == "Zomg") { td.zomg } else { 1 };
                (self.rbe[b.kind] * 1e4 + b.progress(), b.id, cost)
            })
            .collect();
        cands.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut left = td.hooks;
        let owner = self.towers[t].id;
        for (_, id, cost) in cands {
            if cost > left {
                continue;
            }
            left -= cost;
            let Some(i) = self.bloons.iter().position(|b| b.id == id) else { continue };
            let tally0 = self.damage_tally;
            let cash = self.cash_in[self.bloons[i].kind];
            let n = self.destroy_whole(i);
            let self_tally_dealt = self.damage_tally - tally0;
            self.earn(cash * td.mult + td.add);
            if let Some(tw) = self.towers.iter_mut().find(|x| x.id == owner) {
                tw.pops += n;
                tw.sun_destroyed(self.last_destroyed);
                tw.damage += self_tally_dealt;
                self.stats.pops += n;
                self.stats.damage += self_tally_dealt;
            }
            if left == 0 {
                break;
            }
        }
    }

    /// PreEmptiveStrikeLauncher.Launch (VERIFIED-DISASM @0xd7cca0) for each Pre-emptive Strike
    /// Sub, as MOAB-class bloon `target` enters the track: its strike made at the throw offset
    /// turned by the tower's rotation (Vector2.Rotate) and moved down by z / tan 60
    /// (AddDisplaySpaceHeight), aimed at the bloon, whose position it takes from then (recording
    /// sub_050). Children of popped bloons don't count. (throwOffsetSecond, used on every other
    /// launch when set, is zero on every Sub.)
    fn preemptive_strikes(&mut self, target: u64) {
        if !self.bloons.iter().any(|b| b.id == target) {
            return;
        }
        let single = Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None};
        for t in 0..self.towers.len() {
            let Some(proj) = self.tier(t).and_then(|tr| tr.preempt.clone()) else { continue };
            let [ex, ey, ez] = self.tier(t).map_or([0.0; 3], |tr| tr.preempt_offset);
            let (sa, ca) = crt_sin_cos(DEG2RAD * self.towers[t].game_rot.unwrap_or(0.0));
            let (ox, oy) = (ca * ex - sa * ey, ca * ey + sa * ex);
            let tp = self.towers[t].pos;
            let at = [tp[0] + ox, tp[1] - (oy - ez / TAN60)];
            let (tiers, owner) = (self.towers[t].tiers, self.towers[t].id);
            let start = self.projectiles.len();
            self.emit(at, 0.0, &single, &proj, None, tiers, owner);
            for pr in &mut self.projectiles[start..] {
                pr.target = Some(target);
                pr.pos = at;
            }
        }
    }

    /// Moves bloon `i` back `back` along its route (push-back, wind). Path.DistanceToPath: while
    /// the distance is below 0, a path with an entry router hands the bloon to the routed path
    /// (Splitter.Rout, round-robin) and adds that path's length; otherwise it stops at 0.
    fn push_back(&mut self, i: usize, back: f32) {
        let b = &mut self.bloons[i];
        b.dist -= back;
        while b.dist < 0.0 {
            let (to, s) = &self.track_entry[b.path as usize];
            if to.is_empty() {
                b.dist = 0.0;
                break;
            }
            let r = &mut self.routs[*s];
            if *r >= to.len() {
                *r = 0;
            }
            let prev = to[*r];
            *r += 1;
            let len = self.tracks[prev].length();
            b.dist += len;
            b.base -= len;
            b.path = prev as u8;
        }
    }

    /// Orbit.Process (VERIFIED-DISASM @0xd6da00) for each tower with an OrbitModel: it keeps
    /// the bloon it saw while that is alive and within 40 (centre to centre, in 3D), else asks
    /// BloonManager.GetTarget (closest, range 40); the speed eases to 15 with one (+0.5 a frame)
    /// or 3 without (-0.2 a frame), the angle grows by it (less 360 past 360), and glaive k sits
    /// at the tower + Vector2.Rotate((0, range), 360 / count x k + angle). Orbit.ctor starts the
    /// speed at 3; AddProjectiles makes the glaives when it attaches.
    fn orbit_step(&mut self, paragons: bool) {
        for t in 0..self.towers.len() {
            if self.towers[t].paragon.is_some() != paragons {
                continue;
            }
            // A Paragon's Orbit (recordings boomerang_paragon_*, its state logged): on the
            // purchase frame it turns once more as before, then is inactive (frozen, its glaives
            // gone) until activeAt; that frame it is active again and AddProjectiles puts the
            // glaives at the tower + (0, range) without turning; it turns from the next.
            let Some(o) = self.tier(t).and_then(|tr| tr.orbit.clone()) else {
                self.towers[t].orbit = None;
                continue;
            };
            let c = self.towers[t].pos;
            let mut drop_glaives = false;
            if self.towers[t].paragon.is_some() {
                let (first, active_at) = (self.towers[t].paragon_first, self.towers[t].paragon_active_at);
                if self.sim_elapsed > first && self.sim_elapsed < active_at {
                    if let Some(st) = &mut self.towers[t].orbit {
                        st.glaives.clear();
                    }
                    continue;
                }
                if self.sim_elapsed == active_at {
                    if let Some(st) = &mut self.towers[t].orbit {
                        st.glaives = vec![[c[0], c[1] - o.range]; o.count as usize];
                        st.age = 0;
                    }
                    continue;
                }
                drop_glaives = self.sim_elapsed == first;
            }
            let mut st = self.towers[t].orbit.take().unwrap_or(OrbitState { angle: 0.0, speed: 3.0, seen: None, glaives: Vec::new(), age: 0 });
            // Rotate.Process (VERIFIED-DISASM @0xcd88a0) turns each glaive rotationFrames a frame from
            // when AddProjectiles makes it (a new set starts again).
            st.age = if st.glaives.is_empty() { 0 } else { st.age + 1 };
            // (Its 3D distance: the tower at height 0, the bloon at its Position.z; recording
            // boomerang_paragon_r90_t, a MOAB popped into a Ceramic still 120 up is let go, and
            // a Ceramic is no MOAB to GetClosestTarget now.)
            let kept = st.seen.and_then(|id| self.bloons.iter().find(|b| b.id == id)).is_some_and(|b| {
                let (dx, dy) = (b.pos[0] - c[0], b.pos[1] - c[1]);
                !(1600.0 < dx * dx + dy * dy + b.height * b.height)
            });
            if !kept {
                // (No filter: camo bloons count too; recording boomerang_500, a camo Rainbow.)
                st.seen = self.closest_target(c, 40.0, |_| true, |b| b.pos).map(|i| self.bloons[i].id);
            }
            let top = if st.seen.is_some() { 15.0 } else { 3.0 };
            if st.speed <= top {
                st.speed += 0.5;
                if top <= st.speed {
                    st.speed = top;
                }
            } else {
                st.speed -= 0.2;
            }
            st.angle += st.speed;
            if 360.0 < st.angle {
                st.angle -= 360.0;
            }
            st.glaives = (0..o.count)
                .map(|k| {
                    // Vector2.Rotate (game frame), the clone's y flipped.
                    let a = DEG2RAD * -((360.0 / o.count as f32) * k as f32 + st.angle);
                    let (sa, ca) = crt_sin_cos(a);
                    let (gx, gy) = (ca * 0.0 - sa * o.range, ca * o.range + sa * 0.0);
                    [c[0] + gx, c[1] - gy]
                })
                .collect();
            if drop_glaives {
                st.glaives.clear();
            }
            self.towers[t].orbit = Some(st);
        }
    }

    /// The bloon `id` is destroyed (popped with nothing left, or leaked) at `at`.
    /// ArriveAtTarget.OnBloonDestroyEvent (VERIFIED-DISASM @0xca5870): a shot flying at it takes
    /// the spot as its Target (SetTarget: from its start, the progress kept, so one already there
    /// jumps to it; recording boomerang_paragon_r63, a MOAB Press after its Ceramic is gone).
    fn bloon_destroyed(&mut self, id: u64, at: [f32; 2]) {
        #[cfg(test)] let _destroy_timer=sim_profile::Timer::new(5);
        let (now, cursor) = (self.sim_elapsed, self.proj_cursor);
        for (q, pr) in self.projectiles.iter_mut().enumerate() {
            if pr.aim != Some(id) {
                continue;
            }
            let Some((t0, o, target)) = pr.arrive_at.as_mut() else { continue };
            *target = at;
            pr.aim = None;
            // (One that has already moved this frame is placed again from the new Target, as
            // its Process would: recording boomerang_paragon_r63, the same frame.)
            if let (true, Some(ar)) = (cursor == usize::MAX || q <= cursor, pr.spec.arrive.as_ref()) {
                let perc = (((now - *t0) as f32) * (1.0f32 / 60.0) / pr.arrive_time).min(1.0);
                let e = curve_eval(&ar.samples, perc);
                let np = [o[0] + (at[0] - o[0]) * e, o[1] + (at[1] - o[1]) * e];
                pr.vel = [np[0] - pr.pos[0], np[1] - pr.pos[1]];
                pr.pos = np;
            }
        }
    }

    /// The towers' attacks for a frame (Simulation.Simulate's tower processes), or with
    /// `only_temp` the ability attacks of that tower just activated between frames.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    fn towers_phase(&mut self, only_temp: Option<usize>, cd_speed: &[f32], early: &mut FastSet<u64>, round_clock: i64,
        dots: &mut Vec<DotTick>, emits: &mut Vec<([f32; 2], Box<Proj>, u64, u64, u64)>,
        zones: &[([f32; 2], f32, crate::data::Zone)], shoves: &[([f32; 2], crate::data::MoabShove)], leak_voice: &mut bool) {
        // Bloons moved, popped and spawned since the last tower phase.
        self.target_index = None;
        let rounds_on = self.rounds_active();
        for t in only_temp.map_or_else(|| (0..self.towers.len()).collect::<Vec<_>>(), |t| vec![t]) {
            // (A pet is made in its handler's Process: its own start with the next frame; recording
            // beast_upgrades_r30, a pet upgraded in at rt 400 aims at rt 402.)
            if self.towers[t].sub.as_ref().is_some_and(|s| s.pet && s.age < 2.0) {
                continue;
            }
            // SyncTargetPriorityWithSubTowers (@0xd9f090): the handler's target type is its pets'.
            if let Some(p) = self.towers[t].sub.as_ref().filter(|s| s.pet).map(|s| s.parent) {
                if let Some(m) = self.towers.iter().find(|o| o.id == p).map(|o| o.target) {
                    self.towers[t].target = m;
                }
            }
            // (Its TargetTrack attack's Process remakes the points under a new model at once,
            // firing or not.)
            if only_temp.is_none() {
                self.track_supply_refresh(t);
            }
            let buff = self.buffs[t].clone();
            // ParagonTower.Finish (@0xd6f480): when the sacrifice ends it sets the tower's and its
            // attack displays' animation state to endAnimationState. That is the index after the
            // controller's animationStates (Apex Plasma Master and Glaive Dominus: 2 states, end 2;
            // Ascended Shadow: 3, end 3), where the high-quality animator puts Placement: the
            // Paragon drops in again.
            if only_temp.is_none() && self.towers[t].paragon.is_some() && self.sim_elapsed == self.towers[t].paragon_active_at
                && self.tower_tiers[self.towers[t].def].as_ref().and_then(|x| x.paragon.as_ref()).is_some_and(|p| p.end_anim >= 0) {
                self.towers[t].show = Some((crate::anim::PLACE_CLIP.into(), 0.0));
            }
            if only_temp.is_none() {
                // SpinModel weapons: (weapon name, rotationPerSecond x 1/60).
                let spins: Vec<(String, f32)> = self.tier(t).map(|tr| tr.attacks.iter()
                    .chain(tr.abilities.iter().flat_map(|a| a.activate.iter().flat_map(|x| x.attacks.iter())))
                    .flat_map(|a| a.weapons.iter())
                    .filter(|w| w.spin != 0.0)
                    .map(|w| (w.name.clone(), w.spin * FRAME_TIME)).collect()).unwrap_or_default();
                let abilities_now: Vec<(String, f32, bool, bool)> = self.tier(t).map(|tr| tr.abilities.iter()
                    .filter(|a| !self.towers[t].ability_cd.contains_key(&a.name))
                    .map(|a| (a.name.clone(), a.cooldown, a.ready, a.full_start)).collect()).unwrap_or_default();
                let tw = &mut self.towers[t];
                if tw.attack_t < f32::MAX {
                    tw.attack_t += 1.0 / FPS;
                }
                tw.idle_t += 1.0 / FPS;
                if let Some(s) = tw.show.as_mut() {
                    s.1 += 1.0 / FPS;
                }
                // Spin.Process (VERIFIED-DISASM @0x180c92af0): angle + rotationPerFrame, less 360
                // once at or past 360 (never wrapped below 0: recording tack_040, -720.00006 when
                // Blade Maelstrom first fires at 121).
                // Every spinning weapon of its tier, an ability's attack's too: those weapons exist
                // (and spin) from when the tower has the model, not from the activation.
                // (The shot reads the angle as it was before this frame's step: Weapon.Process
                // fires before Spin.Process; recording tack_050.)
                for (name, step) in &spins {
                    let x = match tw.spin.iter().position(|x| &x.0 == name) {
                        Some(k) => &mut tw.spin[k],
                        None => {
                            tw.spin.push((name.clone(), 0.0, 0.0));
                            tw.spin.last_mut().unwrap()
                        }
                    };
                    x.2 = x.1;
                    let v = x.1 + step;
                    x.1 = if 360.0 <= v { v - 360.0 } else { v };
                }
                for ab in &mut tw.brews {
                    ab.frames_left -= 1.0;
                }
                tw.brews.retain(|a| a.frames_left > 0.0 && a.shots_left > 0);
                // Ability.Initialise (VERIFIED-DISASM @0x180df9... Initialise): cooldownTimeRemaining
                // starts at cooldownFrames x 0.33 (all of it with resetCooldownOnTierUpgrade, none
                // with startOffCooldown) (recording bomb_005: Bomb Blitz placed at 0 goes off on a
                // leak at 1190, 3600 x 0.33 = 1188 frames on). HYPOTHESIS P-ABILITYINIT: set the
                // first tick the tower has the ability.
                for (name, cooldown, ready, full) in abilities_now {
                    tw.ability_cd.entry(name).or_insert_with(|| {
                        let frames = (cooldown * FPS) as i32 as f32;
                        if ready { 0.0 } else if full { frames } else { frames * 0.33 }
                    });
                }
                // Ability.Process (VERIFIED-DISASM @0xdfafc0): the cooldown runs while AreRoundsActive.
                if rounds_on {
                    let speed = cd_speed[t] * self.buffs.get(t).map_or(1.0, |b| b.ability_cd_mult);
                    for cd in tw.ability_cd.values_mut() {
                        *cd = (*cd - speed).max(0.0);
                    }
                }
                for (_, f, _) in &mut tw.temp_supports {
                    *f -= 1.0;
                }
                // (Its removal reaches the towers a frame late too: recording ninja_052, the points
                // remade at 1922 when Sabotage ends.)
                tw.temp_supports.retain(|(_, f, _)| *f > -1.0);
                if let Some((d, _, _)) = &mut tw.ammo {
                    *d -= 1.0;
                }
                if let Some(tb) = &mut tw.turbo {
                    tb.frames_left -= 1.0;
                    if tb.frames_left <= 0.0 {
                        tw.turbo = None;
                    }
                }
                tw.mute = (tw.mute - 1.0).max(0.0);
                for b in &mut tw.boosts {
                    b.1 -= 1.0;
                }
                tw.boosts.retain(|b| b.1 > 0.0);
                if let Some((_, _, _, f)) = &mut tw.overclock {
                    *f -= 1.0;
                    if *f <= 0.0 {
                        tw.overclock = None;
                    }
                }
                if let Some((_, _, f)) = &mut tw.range_up {
                    *f -= 1.0;
                    if *f <= 0.0 {
                        tw.range_up = None;
                    }
                }
                if let Some((_, f)) = &mut tw.look {
                    *f -= 1.0;
                    if *f <= 0.0 {
                        tw.look = None;
                    }
                }
                if let Some((_, f)) = &mut tw.fan {
                    *f -= 1.0;
                    if *f <= 0.0 {
                        tw.fan = None;
                    }
                }
                if let Some((_, _, f)) = &mut tw.morph {
                    *f -= 1.0;
                    if *f <= 0.0 {
                        tw.morph = None;
                                }
                }
            }
            #[cfg(test)] let model_start=std::time::Instant::now();
            let Some(mut tier) = self.tier(t).cloned() else { continue };
            #[cfg(test)] {self.perf_detail[0]+=model_start.elapsed().as_secs_f64();}
            // Submerge.UpdateAttackStates (VERIFIED-DISASM @0xdb9de0): the tower's attacks whose
            // name has "Submerge" in it are paused while it is surfaced, the others while it is
            // submerged (recording sub_300: Submerge and Support's pulse never fires on First).
            // Without one of its own, the submerge attack replaces them.
            let sub_state = self.towers[t].submerged;
            let mut sub_paused: Vec<bool> = Vec::new();
            match &tier.submerge {
                Some(_) if tier.attacks.iter().any(|a| a.model_name.contains("Submerge")) => {
                    sub_paused = tier.attacks.iter().map(|a| a.model_name.contains("Submerge") != sub_state).collect();
                }
                Some(sm) if sub_state => tier.attacks = vec![sm.attack.clone()],
                Some(_) => {}
                None => self.towers[t].submerged = false,
            }
            if self.in_step && self.towers[t].mtp.is_some() {
                self.mtp_step(t);
            }
            let extra: Vec<crate::data::Attack> = self.towers[t].temp.iter().map(|x| x.attack.clone()).collect();
            let extra_slots: Vec<usize> = self.towers[t].temp.iter().map(|x| x.slot).collect();
            let paused = self.towers[t].temp.iter().any(|x| x.pauses && x.delay <= 0.0) || self.towers[t].mute > 0.0 
                // ParagonTower's pause starts after its first frame: an attack with a target in range
                // then throws once (recordings boomerang_paragon_r63 / r90: at the purchase, then
                // 120 frames later).
                || (self.sim_elapsed > self.towers[t].paragon_first && self.sim_elapsed <= self.towers[t].paragon_active_at);
            let waiting: Vec<bool> = self.towers[t].temp.iter().map(|x| x.delay > 0.0).collect();
            let base_weapons: usize = tier.attacks.iter().map(|a| a.weapons.len()).sum();
            let nweap: usize = base_weapons + extra.iter().map(|a| a.weapons.len()).sum::<usize>();
            // Weapons keep their state through model changes (upgrades, abilities, morphs) by
            // name (Weapon.UpdatedModel); a new one starts ready (lastEmission = int.MinValue)
            // unless it starts in cooldown. Ability attacks come and go after the tier's own.
            let all = || tier.attacks.iter().chain(extra.iter()).flat_map(|a| a.weapons.iter());
            // (name, occurrence): the k-th weapon of that name.
            let key = |i: usize, w: &crate::data::Weapon| (w.name.clone(), all().take(i).filter(|o| o.name == w.name).count() as u32 + 1);
            let same = self.towers[t].weapons.len() == nweap
                && all().zip(&self.towers[t].weapons).enumerate().all(|(i, (w, st))| st.key.0 == w.name && (st.key.1 == 1 || st.key == key(i, w)));
            if !same {
                let mut old = std::mem::take(&mut self.towers[t].weapons);
                let (steps, rc) = self.towers[t].model_clock.unwrap_or((self.sim_elapsed as i64, self.round_clock));
                self.towers[t].weapons = all()
                    .enumerate()
                    .map(|(i, w)| {
                        let k = key(i, w);
                        match old.iter().position(|o| o.key == k) {
                            Some(j) => old.swap_remove(j),
                            None => WeaponState::new(k, w, if w.between_rounds { steps } else { rc }),
                        }
                    })
                    .collect();
            }
            {
                let tw = &mut self.towers[t];
                let slots: Vec<usize> = (0..base_weapons).chain(extra.iter().zip(&extra_slots).flat_map(|(x, &sl)| (0..x.weapons.len()).map(move |k| base_weapons + sl + k))).collect();
                for (i, st) in tw.weapons.iter_mut().enumerate() {
                    st.slot = slots.get(i).copied().unwrap_or(i);
                }
            }
            let model_changed = self.towers[t].model_clock.is_some();
            self.towers[t].model_clock = None;
            let pos = self.attack_pos(t);
            // HYPOTHESIS P-ROUNDRATE: StartOfRoundRateBuffModel.modifier multiplies the weapon
            // interval for the first `duration` seconds of each round (0.25 = 4x as fast).
            let start_mult = match &tier.round_start_rate {
                Some(r) if self.round_active && (self.round_frame as f32) < r.duration * FPS && r.mult > 0.0 => r.mult,
                _ => 1.0,
            };
            // A buff change is a model change: Weapon.UpdatedModel recalculates rateFrames.
            // VagrantWeaponBehavior.GetRate reads its zones as they are then (recording
            // desperado_002: the Vigilante buff ending at 1433 gives 42 frames from that frame's
            // bloons).
            // (Zones as last updated: a model change before this frame's update reads last frame's;
            // recording desperado_002, the Vigilante buff ending at 1433 counts the bloons within
            // its buffed range.)
            let has_vagrant = all().any(|w| w.vagrant.is_some());
            let vagrant_now = has_vagrant.then(|| self.towers[t].vagrant_zone.unwrap_or_else(|| { let p = self.towers[t].pos; self.vagrant_counts(t, p) }));
            // Weapon.UpdatedModel on a new tower model (placing, upgrading) runs CalcRateFrames,
            // whose BloonDistanceRateBonus and Standoff look at the bloons now (Attack.GetVisibleTarget
            // Close / GetVisibleTargets; recording desperado_upgrades_r30: x-0-0 to 1-0-0 at 201 turns
            // 72 frames into 66).
            let buff_changed = self.towers[t].weapons.iter().any(|st| st.buff_rate != buff.rate_mult * start_mult);
            // A mutator ending (the Vigilante buff) comes after this frame's Zoned<Bloon>.Process, which
            // still had the old range: the rate is worked out from the zone refreshed with this frame's
            // bloons at that range (recording desperado_052 rt 2598: 6 Zebras within 96, one new this
            // frame, 117 frames; desperado_002 rt 1433).
            if buff_changed && !model_changed {
                let p = self.attack_pos(t);
                let ids_in = |r: f32, bl: &[Bloon]| bl.iter().filter(|b| { let q = b.pos; (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) <= r * r }).map(|b| b.id).collect::<Vec<u64>>();
                let bl = std::mem::take(&mut self.bloons);
                for st in &mut self.towers[t].weapons {
                    if st.zone_in.is_some() {
                        st.zone_in = Some(ids_in(st.zone_r, &bl));
                    }
                }
                self.bloons = bl;
            }
            if model_changed {
                let mut i = 0;
                for attack in tier.attacks.iter().chain(extra.iter()) {
                    let (camo, range) = (attack.camo || buff.camo, buff.attack_range(attack));
                    let no_moab = attack.exclude_tags.iter().any(|x| x == "Moabs");
                    let ap = self.attack_pos(t);
                    for w in &attack.weapons {
                        let dk = w.dist_rate.map(|(per, max)| {
                            let d = self.visible_close(ap, range, camo, no_moab, attack).map_or(max, |q| (q[0] - ap[0]).hypot(q[1] - ap[1]).min(max));
                            1.0 + max * per * (d / max.max(1.0))
                        });
                        let sk = w.standoff.map(|[max, per]| {
                            let n = self.visible_count(ap, range, camo, no_moab, attack);
                            let mut x = max;
                            for _ in 1..n {
                                x -= per;
                                if x <= 0.0 {
                                    x = 0.0;
                                    break;
                                }
                            }
                            x + 1.0
                        });
                        if let Some(st) = self.towers[t].weapons.get_mut(i) {
                            if let Some(dk) = dk {
                                st.dist_k = dk;
                            }
                            if let Some(sk) = sk {
                                st.standoff_k = sk;
                            }
                        }
                        i += 1;
                    }
                }
            }
            for (st, w) in self.towers[t].weapons.iter_mut().zip(all()) {
                let br = buff.rate_mult * start_mult;
                // (And so is an upgrade: recording dart_upgrades_r30, Quick Shots bought at 600
                // throws on 601, 48 frames after the last.)
                if st.buff_rate != br || st.model_rate != w.rate || model_changed {
                    st.buff_rate = br;
                    st.model_rate = w.rate;
                    if let (Some(n), Some(_)) = (vagrant_now, w.vagrant) {
                        st.vagrant_n = (n.0, st.zone_in.as_ref().map_or(n.1, |z| z.len() as u32));
                    }
                    st.rate_frames = rate_frames(st.rate(w));
                    if std::env::var_os("DBGB").is_some() { eprintln!("DBGBT chg {} {} dk {} sk {} vag {:?} br {} rf {}", self.round_clock, w.name, st.dist_k, st.standoff_k, st.vagrant_n, st.buff_rate, st.rate_frames); } // DBGX
                }
            }
            if has_vagrant {
                let p = self.towers[t].pos;
                self.towers[t].vagrant_zone = Some(self.vagrant_counts(t, p));
            }
            let mut wi = 0;
            let mut turned = false;
            // RotateToMiddleOfTargetsModel on any attack: the body faces its attacks' targets'
            // midpoint instead of turning with each throw.
            let middle = tier.attacks.iter().any(|a| a.face_middle);
            // CloseTargetTrack's draws come with its Attack's Process, after the RandomPosition's
            // and ahead of the tower's Weapons' shots (recording engineer_205: the Bloon Trap's two
            // before the Spawner's shot).
            let mut track_draws: Option<(f32, f32)> = None;
            for (ai, attack) in tier.attacks.iter().chain(extra.iter()).enumerate() {
                // (LineProjectileEmission.Emit / InstantDamageEmission.Process act as their weapon
                // fires, before the next attack looks for a target: recording desperado_050 rt 73, the
                // pistol's hit marks the MOAB and the rifle's line pops it before the mark attack aims.)
                if self.in_step && !self.instant_hits.is_empty() {
                    self.instant_pass();
                }
                // At an activation only the ability's attacks act, those ActivateAttack.Activate
                // (VERIFIED-DISASM @0xe20900) processes then: processOnActivate or isOneShot
                // (recording glue_040: Glue Strike, neither, throws on the next frame).
                if only_temp.is_some() && (ai < tier.attacks.len() || !self.towers[t].temp[ai - tier.attacks.len()].now) {
                    wi += attack.weapons.len();
                    continue;
                }
                // (A Paragon's new Attacks are made paused from its first frame and wake with its
                // Orbit, a frame before the kept ones: recordings boomerang_paragon_r40 (its MOAB
                // Press waits) and r98 (it throws at 181, the glaives at 182).)
                // Simulation.Simulate runs the process list in creation order: an Attack made after
                // some bloons (a tower placed or upgraded mid-round) acts after they have been
                // processed this frame (recording boomerang_paragon_r98: the Paragon's new MOAB
                // Press aims where a BFB is after this frame's move). Made when it first runs here.
                if ai < tier.attacks.len() {
                    let pet_first = self.towers[t].sub.as_ref().is_some_and(|s| s.pet) as u64;
                    let stamp = *self.towers[t].attack_born.entry(attack.model_name.clone()).or_insert(self.sim_elapsed * 4 + pet_first);
                    self.process_bloons_before(stamp, early, round_clock, dots, emits, zones, shoves, leak_voice);
                }
                let fresh_attack = self.towers[t].paragon.is_some() && self.sim_elapsed < self.towers[t].paragon_active_at
                    && !self.towers[t].paragon_kept.contains(&attack.model_name);
                let paused = paused && (self.towers[t].paragon.is_none() || self.towers[t].paragon_kept.contains(&attack.model_name)
                    || self.towers[t].temp.iter().any(|x| x.pauses && x.delay <= 0.0) || self.towers[t].mute > 0.0);
                // At zero Sandbox health, bloons keep moving but neither base nor temporary
                // ability attacks may start or release shots. Timers above still advance.
                if (self.sandbox && self.lives <= 0.0) || ((paused || fresh_attack || sub_paused.get(ai) == Some(&true)) && ai < tier.attacks.len()) || ai.checked_sub(tier.attacks.len()).is_some_and(|k| waiting.get(k) == Some(&true)) {
                    wi += attack.weapons.len();
                    continue;
                }
                if let Some(brew) = &attack.brew {
                    for weapon in &attack.weapons {
                        let i = wi;
                        wi += 1;
                        let clock = if weapon.between_rounds { self.sim_elapsed as i64 } else { self.round_clock };
                        if !self.towers[t].weapons[i].ready(clock) || !(self.sandbox || self.round_active || !self.bloons.is_empty()) {
                            continue;
                        }
                        match self.brew_target(t, brew, buff.attack_range(attack)) {
                            Some(o) => {
                                self.towers[t].weapons[i].last = clock;
                                let to = self.towers[o].pos;
                                let tw = &mut self.towers[t];
                                tw.angle = (to[1] - pos[1]).atan2(to[0] - pos[0]);
                                tw.attack_t = 0.0;
                                tw.idle_t = 0.0;
                                let ot = &mut self.towers[o];
                                ot.brews.retain(|a| a.brew.id != brew.id && !brew.replaces.contains(&a.brew.id));
                                ot.brews.push(ActiveBrew {
                                    brew: brew.clone(),
                                    shots_left: brew.shots.max(1),
                                    frames_left: if brew.lifespan > 0.0 { brew.lifespan * FPS } else { f32::INFINITY },
                                });
                                self.bursts.push(Burst { pos: to, radius: 8.0, fire: brew.kind == "berserk", ice: false });
                            }
                            None => {}
                        }
                    }
                    continue;
                }
                // Fast path: while every weapon stays on cooldown and outside its animation lead
                // this frame, the attack only counts down (no target search). Not for turning
                // or stance / paired-gun attacks, which act every frame.
                if attack.turn_rate.is_none() && attack.weapons.iter().all(|w| w.name != "LongWeapon" && w.with_main.is_none()) {
                    let n = attack.weapons.len();
                    let (now_t, rc) = (self.sim_elapsed as i64, self.round_clock);
                    if attack.weapons.iter().zip(&self.towers[t].weapons[wi..wi + n]).all(|(w, st)| st.anim < 0 && !st.ready(if w.between_rounds { now_t } else { rc })) {
                        wi += n;
                        continue;
                    }
                }
                // Attacks with a single Target*Model (Tack Shooter: TargetCloseModel) ignore the
                // tower's selectable mode.
                // The off hand (TargetLeftHandModel) uses the tower's left-hand mode.
                let tower_mode = if attack.hand.as_deref() == Some("left") { self.towers[t].left_target } else { self.towers[t].target };
                let modes: Vec<TargetMode> = attack.targets.iter().filter_map(|s| TargetMode::parse(s)).collect();
                let mode = match modes.as_slice() {
                    [only] => *only,
                    _ if modes.contains(&tower_mode) => tower_mode,
                    // Point modes don't carry over to bloon-targeting attacks (and back).
                    [first, ..] if tower_mode.steers() || first.aims_at_point() => *first,
                    _ if tower_mode.steers() => TargetMode::First,
                    _ => tower_mode,
                };
                #[cfg(test)] let target_start=std::time::Instant::now();
                // Geometry is constant during this attack's search. Reuse each segment's
                // range/visibility result without reordering bloons or caching between attacks.
                let use_segment_cache=true;
                #[cfg(test)] let use_segment_cache=!self.reference_targeting;
                let mut segment_cache:Vec<Vec<Option<(bool,bool,bool)>>> = if use_segment_cache {
                    self.track_segs.iter().map(|sg|vec![None;sg.len()]).collect()
                }else{Vec::new()};
                // Target selection for this attack.
                let mut best: Option<(f32, [f32; 2], u64)> = None;
                // The chosen target's offset from its bloon (CanHitMoab's precalculatedOffset), kept
                // exact rather than recomputed from the point.
                let mut best_off = [0.0f32; 2];
                let mut best_circle: Option<u8> = None;
                let mut any_target = false;
                let (camo, range) = (attack.camo || buff.camo, buff.attack_range(attack));
                // STAND-IN P-SHARED: Advanced Intel shares the ranges of all other placed towers
                // (not sub-towers), each at its own buffed range.
                let shared: Vec<([f32; 2], f32)> = if attack.shared_range {
                    (0..self.towers.len())
                        .filter(|&o| o != t && self.towers[o].sub.is_none())
                        .map(|o| (self.attack_pos(o), self.range(o)))
                        .collect()
                } else {
                    Vec::new()
                };
                // Targetting.GetTargets (VERIFIED-DISASM @0xffa980, and by recording) for First:
                // the path's leading bloon (Path.bestFirst*, the camo / MOAB category the attack
                // can see) if its segment is in range; else the in-range segments nearest the exit
                // first (Attack.CompareBracketFirst: distanceUntilLeak), each one's bloons in
                // Path.bloonsList order (our storage order).
                let no_moab = attack.exclude_tags.iter().any(|t| t == "Moabs");
                // Close: the in-range segments (range + 40 when MOABs can be targeted; paths in
                // order, segments ascending: PathManager.GetPathSegmentsInRange) sorted by their
                // point's squared distance to the tower (Attack.CompareBracketClose,
                // VERIFIED-DISASM @0xdcb4c0, List.Sort), each one's bloons in Path.bloonsList order;
                // the first that can be targeted (recording dart_000_close: a Blue ahead of a
                // closer Red on the same segment).
                let mut close_rank: HashMap<(u8, usize), usize> = HashMap::new();
                if mode == TargetMode::Close {
                    let reach = range + if no_moab { 0.0 } else { 40.0 };
                    let mut segs: Vec<(u8, usize, f32)> = Vec::new();
                    for (pi, sg) in self.track_segs.iter().enumerate() {
                        for (k, q) in sg.iter().enumerate() {
                            let ok = self.track_seg_ok.get(pi).and_then(|v| v.get(k)).copied().unwrap_or(false);
                            let d2 = (q[0] - pos[0]) * (q[0] - pos[0]) + (q[1] - pos[1]) * (q[1] - pos[1]);
                            if ok && (q[0] - pos[0]).powi(2) + (q[1] - pos[1]).powi(2) <= reach * reach {
                                segs.push((pi as u8, k, d2));
                            }
                        }
                    }
                    dotnet_sort(&mut segs, &|a: &(u8, usize, f32), b: &(u8, usize, f32)| if a.2 < b.2 { -1 } else { (b.2 < a.2) as i32 });
                    close_rank = segs.iter().enumerate().map(|(r, s)| ((s.0, s.1), r)).collect();
                }
                // (Last: the same with the last bloon and Attack.CompareBracketLast, VERIFIED-DISASM
                // @0xdcb6f0: segments furthest from the exit first; recording dart_420_last, the
                // first in list order of two on one segment, not the one further back.)
                // (Path.Process -> Path.AddBloon goes through Path.bloonsList, strictly better
                // only: of two as far along, the first listed leads; recording glue_400, twin
                // Greens out of one pop, stored the other way round.)
                let mut lead: Vec<Option<(usize, f32, u32)>> = vec![None; self.tracks.len()];
                if mode == TargetMode::First || mode == TargetMode::Last || mode == TargetMode::Strong {
                    let key = (mode == TargetMode::Last, camo, no_moab);
                    lead = if use_segment_cache {
                        if !self.target_index_mut().lead.contains_key(&key) {
                            let l = self.path_leads(key.0, camo, no_moab);
                            self.target_index_mut().lead.insert(key, l);
                        }
                        self.target_index_mut().lead[&key].clone()
                    } else {
                        self.path_leads(key.0, camo, no_moab)
                    };
                }
                // Path.FindBestBloon (VERIFIED-DISASM @0xf8d160) for Strong: per path, in its list
                // order, the highest BloonModel.danger, ties to the one further along (strictly);
                // the bloons the attack can see. Targetting.GetTargets takes it if its segment is
                // in range, else the in-range segments in First's order (recording
                // dart_004_camo_strong_r36: the second of a line of Pinks). INFERRED: only with camo
                // priority; otherwise the visible targets in First's order (recording
                // mix_dart_boomerang_r40: the first of a White and a Black on one segment, not the
                // Black further along; every Strong recording agrees).
                let mut strong_lead: Vec<Option<(usize, f32, f32)>> = vec![None; self.tracks.len()];
                if mode == TargetMode::Strong {
                    strong_lead = if use_segment_cache {
                        if !self.target_index_mut().strong.contains_key(&(camo, no_moab)) {
                            let l = self.strong_leads(camo, no_moab);
                            self.target_index_mut().strong.insert((camo, no_moab), l);
                        }
                        self.target_index_mut().strong[&(camo, no_moab)].clone()
                    } else {
                        self.strong_leads(camo, no_moab)
                    };
                }
                let mut best_key = f64::NEG_INFINITY;
                let mut best_tie = (false, i64::MIN, i64::MIN);
                if use_segment_cache && shared.is_empty() {
                    self.target_index_mut();
                }
                // A segment's (plain, line clear, near enough for MOAB circles), as check_segment below.
                let seg_check = |pi: usize, k: usize| {
                    let seg_ok = self.track_seg_ok.get(pi).and_then(|v| v.get(k)).copied().unwrap_or(false);
                    let seg_q = self.track_segs.get(pi).and_then(|sg| sg.get(k)).copied();
                    let near = |q: [f32; 2], reach: f32| seg_ok && (q[0] - pos[0]).powi(2) + (q[1] - pos[1]).powi(2) <= reach * reach;
                    let clear = seg_q.is_some_and(|q| attack.through_walls || self.line_clear(pos, 0.0, q));
                    let plain = seg_q.is_some_and(|q| near(q, if no_moab { range } else { range + 2.0 })) && clear;
                    (plain, clear, seg_q.is_some_and(|q| near(q, range + 40.0)))
                };
                // Without shared ranges only a bloon on a segment that is plain or near enough can be
                // a target (`!seg_in && shared.is_empty()` below), so only those are visited, still in
                // storage order (ties go to the first). Same targets as checking every bloon.
                let order: Vec<usize> = if use_segment_cache && shared.is_empty() {
                    let index = self.target_index.as_ref().expect("target index");
                    // The segments that can hold targets, then the bloons on them in storage order.
                    let mut wanted = vec![false; index.base.last().map_or(0, |&b| b + index.segs.last().map_or(0, Vec::len))];
                    for (pi, segs) in index.segs.iter().enumerate() {
                        for (k, ids) in segs.iter().enumerate() {
                            if ids.is_empty() {
                                continue;
                            }
                            let r = match segment_cache.get_mut(pi).and_then(|sg| sg.get_mut(k)) {
                                Some(slot) => *slot.get_or_insert_with(|| seg_check(pi, k)),
                                None => seg_check(pi, k),
                            };
                            wanted[index.base[pi] + k] = r.0 || r.2;
                        }
                    }
                    index.seg_of.iter().enumerate().filter(|&(_, &k)| k != u32::MAX && wanted[k as usize]).map(|(i, _)| i).collect()
                } else {
                    (0..self.bloons.len()).collect()
                };
                for &bi in &order {
                    let b = &self.bloons[bi];
                    // Path.Process (VERIFIED-DISASM @0xf6fe60) files on its segments only the bloons
                    // with offTrackCount 0; the wind's are taken after them (below).
                    if b.wind.is_some() || b.held {
                        continue;
                    }
                    let d = &self.defs[b.kind];
                    let p = b.pos;
                    let dd = (p[0] - pos[0]).powi(2) + (p[1] - pos[1]).powi(2);
                    // Attack targets the bloons on its in-range path segments (PathSegment.IsInRange:
                    // the segment's point within range; Path.AddBloon: a bloon is on segment
                    // (int)(distance / 7)), not by the bloon's own distance.
                    // (Filed by the distance it had when the frame began: see target_index_mut.)
                    let seg_idx = (b.wind_seg.unwrap_or(b.prev_dist) / 7.0).max(0.0) as usize;
                    let seg_ok = self.track_seg_ok.get(b.path as usize).and_then(|v| v.get(seg_idx)).copied().unwrap_or(false);
                    // Attack.GetPathSegmentsInRangeFast (VERIFIED-DISASM @0xdcbbb0) lists the targetable
                    // segments (PathSegment.canTargetSegment) within range, + 40 when the attack can
                    // hit MOABs, each with whether its point is within range + 2 and whether the line
                    // from the tower to it is clear (Map.IsTargetLineClear, unless attackThroughWalls).
                    // Targetting.GetTargets / GetGlobalBest: a bloon on a segment that is both is a
                    // plain target; otherwise only a MOAB-class bloon can be, by its collision circles
                    // (Targetting.CanHitMoab), aimed at the circle that passed (recordings dart_500:
                    // the first throw at the front circle, later ones at the MOAB itself).
                    let seg_q = self.track_segs.get(b.path as usize).and_then(|sg| sg.get(seg_idx)).copied();
                    let seg_near = |q: [f32; 2], reach: f32| seg_ok && (q[0] - pos[0]).powi(2) + (q[1] - pos[1]).powi(2) <= reach * reach;
                    let check_segment=||{
                        let clear=seg_q.is_some_and(|q|attack.through_walls||self.line_clear(pos,0.0,q));
                        let plain=seg_q.is_some_and(|q|seg_near(q,if no_moab{range}else{range+2.0}))&&clear;
                        let moab_near=seg_q.is_some_and(|q|seg_near(q,range+40.0));
                        (plain,clear,moab_near)
                    };
                    let (plain,seg_clear,moab_near)=match segment_cache.get_mut(b.path as usize).and_then(|sg|sg.get_mut(seg_idx)){
                        Some(slot)=>*slot.get_or_insert_with(check_segment),
                        None=>check_segment(),
                    };
                    // Attack.CalcCantTargetMoab (VERIFIED-DISASM @0xdcb0e0): an AttackFilter whose
                    // FilterOutTag is "Moabs" (@0xc5e950) lists segments within the range only, so
                    // the range + 2 test never widens it (recording ice_040: a Black on a segment
                    // 30.7 from a range-30 Ice Monkey isn't a target). INFERRED: FilterMoab flipped
                    // the same (both are exclude_tags "Moabs" here).

                    let mut aim_at = b.pos;
                    let mut aim_off = [0.0f32; 2];
                    let mut aim_circle: Option<u8> = None;
                    // HYPOTHESIS P-SHAREDGRID: with isSharedRangeEnabled the attack targets the
                    // segments TargetSupplierSharedRange.PressTheGridsTogether (@0xe0ee30) keeps,
                    // the plain ones only: no MOAB reaches in by its collision circles (recording
                    // sub_400: a MOAB is first a target when its segment's point is in range).
                    let seg_in = plain || (!attack.shared_range && !d.circles.is_empty() && moab_near && {
                        // CollisionGroupData.precalculatedOffset: the offset turned by the bloon's
                        // rotation (Bloon.SetRotation, VERIFIED-DISASM @0xfd5260; Vector2.Rotate as
                        // game_dir), game frame (y flipped here). Recording dart_502: on a curve the
                        // MOAB's lagging rotation puts its front circle in range 4 frames sooner.
                        let rot = self.path_rot(b.path, 0.0).map(|_| b.rot);
                        let tr = &self.tracks[b.path as usize];
                        let (a, z) = (tr.pos((b.dist - 0.5).max(0.0)), tr.pos(b.dist + 0.5));
                        let l = (z[0] - a[0]).hypot(z[1] - a[1]).max(1e-6);
                        let fwd = [(z[0] - a[0]) / l, (z[1] - a[1]) / l];
                        d.circles
                            .iter()
                            .enumerate()
                            .map(|(ci, c)| match rot {
                                Some(r) => {
                                    let a = DEG2RAD * -r;
                                    let (sa, ca) = crt_sin_cos(a);
                                    let (ox, oy) = (ca * c[0] - sa * c[1], ca * c[1] + sa * c[0]);
                                    [b.pos[0] + ox, b.pos[1] - oy, c[2], ox, -oy, ci as f32]
                                }
                                None => {
                                    let (ox, oy) = (fwd[0] * c[1] - fwd[1] * c[0], fwd[1] * c[1] + fwd[0] * c[0]);
                                    [b.pos[0] + ox, b.pos[1] + oy, c[2], ox, oy, ci as f32]
                                }
                            })
                            // In range + its radius, and seen: the segment's line is clear, or (map
                            // given unless attackThroughWalls) the line to the circle itself.
                            .find(|c| {
                                (c[0] - pos[0]).powi(2) + (c[1] - pos[1]).powi(2) < (range + c[2]) * (range + c[2])
                                    && (seg_clear || self.line_clear(pos, 0.0, [c[0], c[1]]))
                            })
                            .map(|c| {
                                aim_at = [c[0], c[1]];
                                aim_off = [c[3], c[4]];
                                aim_circle = Some(c[5] as u8);
                            })
                            .is_some()
                    });
                    // Targetting.GetTargets (VERIFIED-DISASM @0x180fdbe70), which Elite sorts: a MOAB
                    // reached by a collision circle (CanHitMoab) becomes Target(bloon), its centre; only
                    // GetTarget (First / Last / Close / Strong) keeps the circle (recording sniper_250: an Elite shot on
                    // a MOAB at its centre, sniper_032's First 6.3 ahead).
                    if mode == TargetMode::Elite {
                        aim_at = b.pos;
                        aim_off = [0.0; 2];
                        aim_circle = None;
                    }
                    if !seg_in && shared.is_empty() {
                        continue;
                    }
                    if b.camo(d) && !camo {
                        continue;
                    }
                    // FilterOnlyCamoIn.FilterBloon (@0xc20590): bloon.bloonModel.isCamo.
                    if attack.only_camo && !b.camo(d) {
                        continue;
                    }
                    if !attack.require_tags.iter().all(|t| d.tags.contains(t)) || (!buff.unfilter && attack.exclude_tags.iter().any(|t| d.tags.contains(t))) {
                        continue;
                    }
                    if (attack.glue_filter > 0 && b.glue() >= attack.glue_filter)
                        || (attack.skip_frozen && b.frozen())
                        || attack.weapons.first().is_some_and(|w| cant_hit(attack.cant_hit, &w.projectile, b.properties(d)))
                        || attack.skip_effects.iter().any(|id| b.has_effect(id))
                        || (!attack.need_effects.is_empty() && !attack.need_effects.iter().all(|id| b.has_effect(id)))
                        || (attack.mtp_filter && self.mtp_filtered(t, &attack.model_name, b))
                    {
                        continue;
                    }
                    let p = b.pos;
                    // Off-map bloons sit on off-map segments, which can't be targeted (above);
                    // map-wide attacks with shared ranges still skip off-map bloons.
                    if !seg_in && !on_map(p, 0.0) {
                        continue;
                    }
                    let dd = (p[0] - pos[0]).powi(2) + (p[1] - pos[1]).powi(2);
                    let shared_in = || shared.iter().any(|&(q, rr)| (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) <= (rr + d.radius).powi(2));
                    if (!seg_in && !shared_in()) || dd < attack.min_range * attack.min_range {
                        continue;
                    }
                    any_target = true;
                    let mut score = match mode {
                        TargetMode::First => b.progress(),
                        TargetMode::Last => -b.progress(),
                        TargetMode::Close => -dd,
                        TargetMode::Strong => self.rbe[b.kind] * 1e4 + b.progress(),
                        // TargetEliteTargetting.Sort (VERIFIED-DISASM @0x180de19d0; buffers from its .ctor
                        // @0x180de1c50, percentageThroughMap 0.75 from the model): camo priority on and a camo
                        // bloon: danger + 1e9; past 75%: perc x 0.12 + 1e6; a MOAB: perc x 0.12 + danger + 10;
                        // with SpawnChildren and more than one child: children x 1e4 + perc x 0.12 + danger;
                        // else danger. (The f32 sum, as the game adds it.) Equal keys: see below.
                        TargetMode::Elite => {
                            let perc = self.perc_through_map(b);
                            let pt = perc * 1.2f32 * 0.1f32;
                            let key: f32 = if self.towers[t].camo_prio && b.camo(d) {
                                d.danger + 1e9
                            } else if perc > 0.75 {
                                pt + 1e6
                            } else if d.tags.iter().any(|t| t == "Moabs") {
                                pt + d.danger + 10.0
                            } else if d.children.len() > 1 {
                                d.children.len() as f32 * 1e4 + (pt + d.danger)
                            } else {
                                d.danger
                            };
                            key
                        }
                        _ => 0.0,
                    };
                    // The camo priority toggle (TargetCamo.prioritiseCamo). HYPOTHESIS
                    // P-CAMOPRIO: camo bloons first, then the mode, as prioritiseCamo upgrades do.
                    let base = score;
                    let mut prio = 0.0f64;
                    if (attack.prio_camo || self.towers[t].camo_prio) && b.camo(d) {
                        score += 1e12;
                        prio += 1.0;
                    }
                    if attack.prio_moab && d.tags.iter().any(|t| t == "Moabs") {
                        score += 1e13;
                        prio += 10.0;
                    }
                    let key = if mode == TargetMode::Close && seg_in {
                        let rank = close_rank.get(&(b.path, seg_idx)).map_or(1e6, |&r| r as f64);
                        prio * 1e12 - rank * 1e3 - self.path_rank.get(&b.id).map_or(999.0, |r| r.1 as f64).min(999.0)
                    } else if mode == TargetMode::Strong && seg_in {
                        let len = self.tracks[b.path as usize].length() as f64;
                        let until_leak = len - (seg_idx as f64 * 7.0 + 3.5);
                        // TargetStrong.GetTarget (VERIFIED-DISASM @0xe0dfa0): over Attack
                        // .GetVisibleTargets, the first with a strictly higher danger (recording
                        // boomerang_000_strong: an in-range Blue over a Red further along). It
                        // asks for them as TargetType.First (statics +0x18), so the path's
                        // leading bloon comes first when in range, then First's order (recordings
                        // glue_205_strong: of three Ceramics on a segment the one ahead, second
                        // listed; mix_dart_boomerang_r40: the first listed of a White and a Black).
                        prio * 1e12
                            + if (attack.prio_camo || self.towers[t].camo_prio) && strong_lead[b.path as usize].is_some_and(|(l, _, _)| l == bi) { 1e10 } else if lead[b.path as usize].is_some_and(|(l, _, _)| l == bi) { d.danger as f64 * 1e8 + 1e4 } else { d.danger as f64 * 1e8 - until_leak * 1e4 - self.path_rank.get(&b.id).map_or(1e6, |r| r.1 as f64) * 1e-3 }
                    } else if mode == TargetMode::First && seg_in {
                        // (Segments in the order of the ones the bloons were filed on at frame
                        // start: recording engineer_320 rt 2452, a Sentry's First is a Red on segment
                        // 63, not a Blue at 441.3 still filed on 62.)
                        // (prio is the camo / MOAB priority count: the f32 score near 1e12 can't
                        // order bloons; recording dart_002_camo_r37.)
                        let len = self.tracks[b.path as usize].length() as f64;
                        let until_leak = len - (seg_idx as f64 * 7.0 + 3.5);
                        prio * 1e12 + if lead[b.path as usize].is_some_and(|(l, _, _)| l == bi) { 1e9 } else { -until_leak * 1e4 - self.path_rank.get(&b.id).map_or(1e6, |r| r.1 as f64) * 1e-3 }
                    } else if mode == TargetMode::Last && seg_in {
                        let len = self.tracks[b.path as usize].length() as f64;
                        let until_leak = len - (seg_idx as f64 * 7.0 + 3.5);
                        prio * 1e12 + if lead[b.path as usize].is_some_and(|(l, _, _)| l == bi) { 1e9 } else { until_leak * 1e4 - self.path_rank.get(&b.id).map_or(1e6, |r| r.1 as f64) * 1e-3 }
                    } else if mode == TargetMode::Elite {
                        prio * 1e12 + base as f64
                    } else {
                        prio * 1e12 + base as f64
                    };
                    // (Equal Elite keys: the first in Targetting.GetTargets' list, which LINQ's stable
                    // sort keeps: GetGlobalBest's (TargetType First) leader, then the visible segments
                    // front to back, each in its own list order. Recorded with SimRecorder's ES lines,
                    // sniper_052 rt 3295: 15108, Yellow 15249 on segment 28, Pinks 15369, 15361, 15353 on 27.)
                    let tie = if mode == TargetMode::Elite {
                        (lead[b.path as usize].is_some_and(|(l, _, _)| l == bi), seg_idx as i64, -self.path_rank.get(&b.id).map_or(i64::MAX / 2, |r| r.1 as i64))
                    } else {
                        (false, 0, 0)
                    };
                    if best.is_none() || key > best_key || (key == best_key && tie > best_tie) {
                        best_tie = tie;
                        best_key = key;
                        best = Some((score, aim_at, b.id));
                        best_off = aim_off;
                        best_circle = aim_circle;
                    }
                }
                #[cfg(test)] {self.perf_detail[2]+=target_start.elapsed().as_secs_f64();}
                // Targetting.GetTargets (VERIFIED-DISASM @0xffa980): after the segments' targets,
                // the bloons off the track (offTrackCount >= 1, not destroyed) the attack's filter
                // takes and it can see, in BloonManager's order (HYPOTHESIS P-OFFTRACKORDER: the
                // factory's, newest object first; recording ninja_025 rt 295: a carried Blue over an
                // older carried Ceramic), the first the attack accepts (recordings ninja_020 rt 421,
                // ninja_520 rt 334: a carried bloon is a target when nothing on the segments is;
                // ninja_420_last: not over one).
                if best.is_none() && mode != TargetMode::Point {
                    let mut off: Vec<usize> = (0..self.bloons.len()).filter(|&i| (self.bloons[i].wind.is_some() || self.bloons[i].held) && self.bloons[i].hp > 0).collect();
                    off.sort_by_key(|&i| std::cmp::Reverse(self.bloons[i].obj));
                    for i in off {
                        let b = &self.bloons[i];
                        let d = &self.defs[b.kind];
                        if (b.camo(d) && !camo) || (attack.only_camo && !b.camo(d)) || (no_moab && d.tags.iter().any(|t| t == "Moabs")) {
                            continue;
                        }
                        if !attack.require_tags.iter().all(|t| d.tags.contains(t)) || (!buff.unfilter && attack.exclude_tags.iter().any(|t| d.tags.contains(t))) {
                            continue;
                        }
                        if (attack.glue_filter > 0 && b.glue() >= attack.glue_filter)
                            || (attack.skip_frozen && b.frozen())
                            || attack.weapons.first().is_some_and(|w| cant_hit(attack.cant_hit, &w.projectile, b.properties(d)))
                            || attack.skip_effects.iter().any(|id| b.has_effect(id))
                            || (!attack.need_effects.is_empty() && !attack.need_effects.iter().all(|id| b.has_effect(id)))
                            || (attack.mtp_filter && self.mtp_filtered(t, &attack.model_name, b))
                        {
                            continue;
                        }
                        // HYPOTHESIS P-OFFTRACKRANGE: within range + its radius (recording ninja_520: a
                        // Blue 56.9 from a range-50 Ninja is taken at rt 334, one 77.2 away at 307 isn't).
                        let rr = range + d.radius;
                        if (b.pos[0] - pos[0]).powi(2) + (b.pos[1] - pos[1]).powi(2) > rr * rr {
                            continue;
                        }
                        if !attack.through_walls && !self.line_clear(pos, 0.0, b.pos) {
                            continue;
                        }
                        any_target = true;
                        best = Some((0.0, b.pos, b.id));
                        best_off = [0.0; 2];
                        best_circle = None;
                        break;
                    }
                }
                // FilterIfAttackHasTarget (VERIFIED-DISASM @0x180c1ec30): nothing while the named
                // attack's hadValidTarget is set (recording desperado_050: the pistol stays quiet while
                // the rifle has a target). Attack.hadValidTarget is FindTarget's result, set when the
                // weapon is ready (Weapon.Process: ClearTarget then FindTarget, @0x180c50670).
                if let Some(other) = &attack.if_target_attack {
                    if self.towers[t].had_target.get(other).copied().unwrap_or(false) {
                        best = None;
                    }
                }
                {
                    let clock = if attack.weapons.first().is_some_and(|w| w.between_rounds) { self.sim_elapsed as i64 } else { self.round_clock };
                    if self.towers[t].weapons.get(wi).is_some_and(|st| st.ready(clock)) {
                        let found = best.is_some();
                        self.towers[t].had_target.insert(attack.model_name.clone(), found);
                    }
                }
                // CreateEffectWhileAttacking.Process (@0xdb5e30): on while the attack has a target.
                if attack.while_fx.is_some() {
                    self.towers[t].attacking_fx = best.is_some();
                }
                if attack.pool_fallback && best.is_none() && (self.sandbox || self.round_active || !self.bloons.is_empty()) {
                    best = self.track_point(pos, range, 0.0);
                }
                // The target's height (Target.Position z) for ArriveAtTarget's altSpeed timing.
                let mut aim_z = 0.0f32;
                if let Some(off) = attack.track_target.filter(|_| mode != TargetMode::Point) {
                    // STAND-IN P-SMARTTRACK: Smart aims at the leading bloon's track spot in range
                    // (Normal when none); Automatic does that, else the closest spot.
                    let lead = || {
                        let rr = range * range;
                        self.bloons
                            .iter()
                            .filter(|b| (b.pos[0] - pos[0]).powi(2) + (b.pos[1] - pos[1]).powi(2) <= rr && on_map(b.pos, 0.0))
                            .max_by(|x, y| x.progress().total_cmp(&y.progress()))
                            .map(|b| (0.0, b.pos, 0))
                    };
                    best = if !(self.sandbox || self.round_active || !self.bloons.is_empty()) {
                        None
                    } else {
                        match mode {
                            TargetMode::CloseTrack => self.closest_track_point(pos, range, off),
                            TargetMode::SmartTrack => match lead() {
                                Some(b) => Some(b),
                                None => self.track_point(pos, range, off),
                            },
                            TargetMode::AutoTrack => match lead() {
                                Some(b) => Some(b),
                                None => self.closest_track_point(pos, range, off),
                            },
                            _ if attack.track_closest && attack.track_default => self.first_visible_track_point(self.towers[t].pos, pos, range, off, no_moab, attack.through_walls),
                            // CloseTargetTrack (VERIFIED-DISASM GeneratePoints @0xdb55f0, GetTargets
                            // MoveNext @0xdbf7b0): Attack.GetVisibleInRangePathSegment's point (2 up),
                            // moved by (d x maxOffset - maxOffset x 0.5) on x then y, two
                            // syncedRandom draws (recording engineer_004: the Bloon Trap's point).
                            _ if attack.track_closest => match self.start_track_point(t, &attack, range) {
                                // (Its targets are asked for only when a weapon can throw: ready
                                // and not held by LimitProjectile; recording engineer_205, two
                                // draws a trap, none while it is out.)
                                Some(q) if self.can_throw(t, wi, &attack) => {
                                    let (d1, d2) = match track_draws.take() {
                                        Some(d) => d,
                                        None => (self.synced_double() as f32, self.synced_double() as f32),
                                    };
                                    let h = off * 0.5;
                                    aim_z = 2.0;
                                    Some((0.0, [q[0] + (d1 * off - h), -((d2 * off - h) + -q[1])], 0))
                                }
                                Some(q) => {
                                    aim_z = 2.0;
                                    Some((0.0, q, 0))
                                }
                                None => None,
                            },
                            TargetMode::First | TargetMode::Last | TargetMode::Close | TargetMode::Strong | TargetMode::Track if !attack.track_closest && !attack.track_default => self.supplied_track_point(t, pos, attack.range, off, no_moab, attack.through_walls),
                            _ => self.track_point(pos, range, off),
                        }
                    };
                }
                // NecromancerTargetTrackWithinRange: a track segment, not a bloon (necro_target).
                if attack.necro.is_some() {
                    // No track points yet (Tower::necro_points): no target. HYPOTHESIS
                    // P-SANDBOXNECRO: sandbox always has them (owner, 2026-11-02: the real
                    // sandbox Necromancer raises undead from pops of sent bloons, no round
                    // played; which round event the sandbox fires isn't found yet).
                    let seg = if self.towers[t].necro_points || self.sandbox { self.necro_target(t, pos, range, no_moab, attack.through_walls) } else { None };
                    self.towers[t].necro_seg = seg;
                    best = seg.map(|(pi, k)| (0.0, self.track_segs[pi as usize][k], 0));
                }
                // HYPOTHESIS P-RANDPOS: RandomPositionBasicModel picks a uniform direction and a
                // distance in [minRadius, maxRadius], kept inside the map bounds.
                if let Some(rp) = attack.random_pos.filter(|_| !attack.weapons.is_empty() && (self.sandbox || self.round_active || !self.bloons.is_empty())) {
                    let key = attack.model_name.clone();
                    best = self.random_pos_target(t, &key, rp).map(|q| (0.0, q, 0));
                    if track_draws.is_none() && only_temp.is_none() {
                        let mut k = 0;
                        for a in tier.attacks.iter() {
                            if a.track_closest && self.can_throw(t, k, a) && self.start_track_point(t, a, a.range).is_some() {
                                track_draws = Some((self.synced_double() as f32, self.synced_double() as f32));
                                break;
                            }
                            k += a.weapons.len();
                        }
                    }
                } else if let Some([lo, hi]) = attack.random_position {
                    best = if (self.sandbox || self.round_active || !self.bloons.is_empty()) {
                        // HYPOTHESIS P-RANDLAND: RandomPositionModel (areaType Land) redraws
                        // until the spot is placeable land clear by targetRadius; no shot if 20
                        // draws all fail.
                        let mut found = None;
                        // RandomPositionBasic.GetTargets (VERIFIED-DISASM @0x180dd9e70, MoveNext):
                        // r = (float)(synced NextDouble x (max - min) + min), angle = (float)(synced
                        // NextDouble x 2 pi) radians, the tower's position + (cos, sin) x r (game
                        // frame), drawn again until Map.IsPointOnMap(p, -mapBorder) (recording
                        // sniper_040: Supply Drop's crate lands 38.15 out at 101.6 degrees).
                        if let Some(border) = attack.random_basic {
                            for _ in 0..1000 {
                                let r = (self.synced_double() * (hi - lo) as f64 + lo as f64) as f32;
                                let a = (self.synced_double() * (std::f32::consts::TAU as f64)) as f32; // (2 pi as a float, widened: VERIFIED-DISASM const @0x183e50188; recording sniper_052)
                                let (sa, ca) = crt_sin_cos(a);
                                let q = [pos[0] + ca * r, pos[1] - sa * r];
                                if on_map(q, -border) {
                                    found = Some((0.0, q, 0));
                                    break;
                                }
                            }
                        }
                        for _ in 0..if attack.random_basic.is_some() { 0 } else if attack.random_land.is_some() { 20 } else { 1 } {
                            let (a, r) = (self.rand() * std::f32::consts::TAU, lo + (hi - lo) * self.rand());
                            let (b0, b1) = self.bounds;
                            let q = [(pos[0] + a.cos() * r).clamp(b0[0] + 7.0, b1[0] - 7.0), (pos[1] + a.sin() * r).clamp(b0[1] + 7.0, b1[1] - 7.0)];
                            if attack.random_land.map_or(true, |rr| self.land_clear(q, rr)) {
                                found = Some((0.0, q, 0));                                break;
                            }
                        }
                        found
                    } else {
                        None
                    };
                }
                // Point-aiming attacks fire at their point (below) and turn towards it at
                // RotateToPointerModel.rate (HYPOTHESIS P-POINTER: turning shots fire along the barrel).
                let mut turning = false;
                if mode.aims_at_point() {
                    if attack.point_start_track && self.towers[t].point.is_none() {
                        self.towers[t].point = self.start_track_point(t, &attack, range);
                        self.towers[t].point_z = 2.0;
                    }
                    aim_z = self.towers[t].point_z;
                    let tw = &self.towers[t];
                    let fwd = [pos[0] + tw.angle.cos() * 40.0, pos[1] + tw.angle.sin() * 40.0];
                    let aim = match mode {
                        TargetMode::Pointer => self.coop_pointer(tw.id).unwrap_or(fwd),
                        _ => tw.point.unwrap_or(fwd),
                    };
                    // TargetPointer.GetTarget (VERIFIED-DISASM @0xe06e20): the pointer is always a
                    // valid Target; TargetSelectedPoint.CanHaveTarget (@0xe0aa00): its point once
                    // set. So they fire whenever rounds are active, bloons or not (sandbox: a
                    // Dartling shoots at the cursor as soon as it is placed).
                    let has_point = mode == TargetMode::Pointer || self.towers[t].point.is_some();
                    best = (has_point || any_target).then_some((0.0, aim, 0));
                    if let Some(rate) = attack.turn_rate.filter(|r| *r < 1e5) {
                        if !turned {
                            let want = (aim[1] - pos[1]).atan2(aim[0] - pos[0]);
                            let step = (rate / FPS).to_radians();
                            let tw = &mut self.towers[t];
                            tw.angle += angle_diff(want, tw.angle).clamp(-step, step);
                            turned = true;
                        }
                        turning = true;
                    }
                }
                // ToggleFocusStanceModel: with a LongWeapon, Spread fires the others and Focus
                // only it (HYPOTHESIS P-STANCE: the swap list decodes empty; matched by name).
                let has_long = attack.weapons.iter().any(|w| w.name == "LongWeapon");
                let focus = self.towers[t].focus;
                // FireAlternateWeaponModel ids of the guns that fired this frame.
                let mut fired_ids: Vec<i32> = Vec::new();
                // The facing the attack aims with (the hull's, unless it doesn't turn).
                let mut aim_face = self.towers[t].angle;
                let extra_rot = attack.extra_rotation.to_radians();
                for weapon in &attack.weapons {
                    if has_long && (weapon.name == "LongWeapon") != focus {
                        wi += 1;
                        continue;
                    }
                    // HYPOTHESIS P-BROADSIDE: a FireWhenAlternateWeaponIsReadyModel gun fires
                    // whenever its paired gun does (its own 0.05 s rate only gates repeats), and
                    // with a FilterTargetAngleFilterModel only if a bloon it could target is in
                    // range inside its arc of the tower's facing.
                    if let Some(id) = weapon.with_main {
                        let face = aim_face;
                        let ok = fired_ids.contains(&id)
                            && weapon.fov.map_or(true, |[fov, base]| {
                                let centre = face - base.to_radians();
                                self.bloons.iter().any(|b| {
                                    let d = &self.defs[b.kind];
                                    let r = range + d.radius;
                                    let (dx, dy) = (b.pos[0] - pos[0], b.pos[1] - pos[1]);
                                    (camo || !b.camo(d))
                                        && on_map(b.pos, 0.0)
                                        && dx * dx + dy * dy <= r * r
                                        && angle_diff(dy.atan2(dx), centre).abs() <= (fov * 0.5).to_radians()
                                })
                            });
                        if !ok {
                            wi += 1;
                            continue;
                        }
                        self.towers[t].weapons[wi].last = NEVER;
                    }
                    // LimitProjectile.FilterEmission (VERIFIED-DISASM @0xc7c530): the tower's
                    // projectiles of model id projectileId (a full trap has paid out: EatBloon
                    // .EmitCash); at the limit each try holds it to now + delayInFrames
                    // (IsEmissionLimitedByTime @0xc7ced0; recording engineer_004: no new Bloon
                    // Trap 167 frames after the full one goes).
                    let at_limit = weapon.limit.is_some_and(|n| {
                        let id = self.towers[t].id;
                        let out = self.projectiles.iter().filter(|pr| pr.owner == id && (if weapon.limit_id.is_empty() { pr.spec.eat.is_some() } else { pr.spec.pid == weapon.limit_id }) && !(pr.spec.eat.is_some() && pr.spec.cash.is_some())).count();
                        let now = self.sim_elapsed as i64;
                        if out >= n as usize && !weapon.limit_destroyed {
                            self.towers[t].weapons[wi].limit_until = now + weapon.limit_delay;
                            true
                        } else {
                            out >= n as usize || now < self.towers[t].weapons[wi].limit_until
                        }
                    });
                    // SubTowerFilterModel (Aircraft Carrier): at most `max` planes alive.
                    let at_limit = at_limit
                        || weapon.max_subs.as_ref().is_some_and(|m| {
                            let id = self.towers[t].id;
                            let live = self.towers.iter().filter(|o| o.sub.as_ref().is_some_and(|s| s.parent == id && m.ids.contains(&s.spec.id))).count();
                            live >= m.max as usize
                        });
                    let w = wi;
                    wi += 1;
                    let lead = weapon.animation_offset;
                    let ambient = attack.fire_without_target;
                    // HYPOTHESIS P-SIMTIME: fireBetweenRounds weapons keep lastEmission on Simulation.time
                    // (sim_elapsed; recordings ninja_040/050/250: Sabotage's weapon at 135 for rt 120).
                    let now_t = self.sim_elapsed as i64;
                    let clock = if weapon.between_rounds { now_t } else { self.round_clock };
                    let tw = &mut self.towers[t];
                    let quota_left = weapon.per_round.map_or(true, |n| tw.weapons[w].round_shots < n) && !at_limit;
                    // Weapon.Process (VERIFIED-DISASM @0xc6f800): without fireBetweenRounds it does
                    // nothing unless Simulation.AreRoundsActive: a Dartling, Mortar or Spike
                    // Factory between rounds with nothing out stays quiet (not in sandbox).
                    let rounds_on = weapon.between_rounds || (self.sandbox || self.round_active || !self.bloons.is_empty());
                    // NecroEmissionFilter.FilterEmission (VERIFIED-DISASM @0xc7f340): a graveyard
                    // weapon fires only with minRbeNeeded in the graveyard (the Necromancer's
                    // minBloonsSpawnedPerWave, the Prince's minPiercePerBloon); until then it stays
                    // ready (recording wizard_005: nothing until the round's first pop, at 1852).
                    let grave_ok = attack.necro.as_ref().is_none_or(|nc| {
                        let need = if weapon.prince { nc.prince_pierce } else { nc.min_per_wave as f32 };
                        tw.grave.iter().map(|g| g.1).sum::<f32>().floor() >= need
                    });
                    // BurstWeaponBehavior.Process (VERIFIED-DISASM @0x180c6c250), alwaysEmptyClip: once the
                    // weapon is ready (IsReloadReady) with its clip part used, overrideTargetPosition is the
                    // point the attack's range ahead along the tower's rotation, and Weapon.Process aims there
                    // whatever FindTarget found (recording desperado_205 rt 139: the shotgun's second shell
                    // with no bloon out).
                    let empty_clip = weapon.burst.as_ref().filter(|b| b.always_empty && !b.over_time).and_then(|b| {
                        let st = &tw.weapons[w];
                        // (BurstWeaponBehavior.Process sets it after the weapon's own Process: it is aimed
                        // with from the frame after the weapon became ready; rt 139, lastEmission 156.)
                        (st.burst_left != b.count && st.ready(clock - 1)).then(|| {
                            // (Set as last frame's weapon processing ended, with the tower's rotation
                            // then: recording desperado_003 rt 952, the pistol turning the tower to a
                            // Lead first leaves the shotgun's point, and the tower, at 67.03.)
                            if let Some(q) = st.empty_pt {
                                return (0.0, q, 0u64);
                            }
                            let d = game_dir(tw.game_rot.unwrap_or(0.0));
                            let r = buff.attack_range(attack);
                            // (From the tower's position raised 10 in display space, RotateToTarget's
                            // useThrowMarkerHeight off: the point the tower already faces, so it doesn't
                            // turn; recording desperado_005 rt 171, the shell from -16.44.)
                            let from = [pos[0], pos[1] + DISPLAY_TOWER_HEIGHT / TAN60];
                            (0.0, [from[0] + d[0] * r, from[1] + d[1] * r], 0u64)
                        })
                    });
                    let decide = empty_clip.or(best).or(ambient.then_some((0.0, pos, 0))).filter(|_| quota_left && rounds_on && grave_ok);
                    // Weapon.Process (VERIFIED-DISASM @0xc6f800): once lastEmission + rateFrames
                    // <= its clock and the attack has a target, the throw animation starts and
                    // the cooldown restarts there; the projectile leaves animationOffsetFrames
                    // later (WeaponModel: (int)(animationOffset x 60)), at the attack's target
                    // then (SpawnDart). A throw still pending when the next starts leaves first.
                    let off = (weapon.animation_offset * 60.0) as i32 as i64;
                    let mut spawns = 0;
                    // TheBlazingSun.Process (VERIFIED-DISASM @0x180c96150): with
                    // debuffExpiresAfterFrames > 0, stacks go back to 0 once lastEmissionAt + that
                    // <= roundTime.
                    if let Some([_, _, _, exp]) = weapon.blazing_sun {
                        let st = &mut tw.weapons[w];
                        if exp > 0.0 && st.sun_last + exp as i64 <= self.round_clock {
                            st.sun_stacks = 0;
                        }
                    }
                    let st = &mut tw.weapons[w];
                    let started_id = decide.filter(|_| st.ready(clock)).map(|(_, _, id)| id);
                    let started = decide.filter(|_| st.ready(clock)).map(|(_, p, _)| p);
                    if let Some(id) = started_id {
                        st.aim_id = id;
                        st.aim_off = best_off;
                        st.aim_circle = best_circle;
                    }
                    if let Some(p) = started {
                        if st.anim >= 0 {
                            spawns += 1;
                        }
                        st.anim = now_t;
                        st.last = clock;
                        st.aim_at = p;
                        st.round_shots += 1;
                    }
                    // (Between rounds Process does nothing, so a throw in progress waits too: recording
                    // sub_005, a throw due at the round's end leaves on the next round's first frame.)
                    if rounds_on && st.anim >= 0 && st.anim + off <= now_t {
                        st.anim = -1;
                        spawns += 1;
                    }
                    // CycleAnimation.GetAnimation (@0xc72140, the weapon's behaviours as the throw starts):
                    // randomize takes a synced Next(min, max + 1), a draw whatever it's used for
                    // (recording beast_052: Next(2, 5) at each of the pet's throws).
                    if started.is_some() && weapon.cycle_random {
                        let seed = self.freeplay_seed;
                        let d = self.synced.get_or_insert_with(|| BufferedRandom::new(seed)).next_double();
                        if std::env::var_os("RRBTD6_RNG_LOG").is_some() {
                            eprintln!("RNG {} {d:?}", self.round_clock);
                        }
                    }
                    // The Target's factory object, while its bloon lives. (Its cached spot is where
                    // the bloon was when the throw started: readers get copies of the struct;
                    // recording sub_052, a missile whose target died in the throw.)
                    if st.anim >= 0 && st.aim_id != 0 {
                        if let Some(b) = self.bloons.iter().find(|b| b.id == st.aim_id) {
                            st.aim_obj = Some(b.obj);
                        }
                    }
                    let (aimed, aim_id, aim_off, aim_circle) = (st.aim_at, st.aim_id, st.aim_off, st.aim_circle);
                    // RotateToTarget.Process (VERIFIED-DISASM @0xdf6be0) with onlyRotateDuringThrow:
                    // while a weapon is in its throw (Weapon.IsInThrow: animationStartedAt >= 0),
                    // ApplyRotation turns the tower to the attack's target each frame, the frame the
                    // throw starts included (recording dart_520: -66.28, -65.64, -64.96, -64.24, then
                    // the release's -63.48; an emission in between starts from it).
                    if st.anim >= 0 && aim_id != 0 && attack.rotate && !turning && weapon.aim_offset.is_none() && extra_rot == 0.0 {
                        if let Some(b) = self.bloons.iter().find(|b| b.id == aim_id) {
                            let target = [b.pos[0] + aim_off[0], b.pos[1] + aim_off[1]];
                            let aim_from = [pos[0], pos[1] + DISPLAY_TOWER_HEIGHT / TAN60];
                            tw.game_rot = Some(RAD2DEG * crt_atan2(target[0] - aim_from[0], -(target[1] - aim_from[1])));
                            tw.turret_rel = tw.game_rot.unwrap() - tw.base_rot;
                            if std::env::var_os("DBGE").is_some() { eprintln!("DBGAIM turn rt {} w {} aim {} rot {:?}", self.round_clock, w, aim_id, tw.game_rot); } // DBGX
                        }
                    }
                    if let (Some(p), false) = (started, ambient) {
                        if attack.rotate && !turning {
                            tw.aim = (p[1] - pos[1]).atan2(p[0] - pos[0]) - extra_rot;
                            tw.set_attack_aim(ai, tw.aim, p, pos, middle, attack.turn_body);
                        }
                        tw.attack_t = 0.0;
                        tw.idle_t = 0.0;
                        // AnimateOnBeastHandler.SetUpAttack (@0xdab0f0): the pet's attack sets its
                        // handler's animation state, cycling animationStateMinParent..MaxParent.
                        if let Some(parent) = tw.sub.as_ref().filter(|s| s.pet).map(|s| s.parent) {
                            if let Some(h) = self.towers.iter_mut().find(|o| o.id == parent) {
                                h.show = Some(("AttackParent03".into(), 0.0));
                            }
                        }
                    }
                    if spawns == 0 && weapon.burst.as_ref().is_some_and(|b| b.always_empty && !b.over_time) {
                        let tw = &mut self.towers[t];
                        let d = game_dir(tw.game_rot.unwrap_or(0.0));
                        let r = buff.attack_range(attack);
                        let from = [pos[0], pos[1] + DISPLAY_TOWER_HEIGHT / TAN60];
                        tw.weapons[w].empty_pt = Some([from[0] + d[0] * r, from[1] + d[1] * r]);
                    }
                    let tw = &mut self.towers[t];
                    let st = &mut tw.weapons[w];
                    if spawns == 0 {
                        continue;
                    }
                    fired_ids.extend(weapon.alt_id);
                    // The shots to emit, in order: each spawned throw, then the missed-frame
                    // shots its rate leaves (doubleShot).
                    let mut queue = vec![false; spawns];
                    let mut qi = 0;
                    while qi < queue.len() {
                        let double = queue[qi];
                        qi += 1;
                        // TheBlazingSun.EmissionStarted (VERIFIED-DISASM @0x180c95d90): stacks + 1 up to
                        // maxStacks, lastEmissionAt = roundTime; not for the missed-frame extra shots
                        // (recording desperado_502's Q lines: 7 stacks at rt 91 after a doubled throw).
                        if let (Some([max, ..]), false) = (weapon.blazing_sun, double) {
                            let st = &mut self.towers[t].weapons[w];
                            st.sun_stacks = (st.sun_stacks + 1).min(max as i32);
                            st.sun_last = self.round_clock;
                        }
                        // A burst's ProjectilesCreated, applied after the shot's CalcRateFrames.
                        let mut burst_after: Option<crate::data::BurstFire> = None;
                        // With no target any more the throw still leaves: Target.IsValid (VERIFIED-DISASM
                        // @0xb9bb90) fails for a destroyed bloon, so RotateToTarget no longer turns the
                        // tower and the shot goes along its current facing (recording dart_040_club:
                        // three towers on a Green the first one's darts popped that frame).
                        let locked = (aim_id != 0).then(|| self.bloons.iter().find(|b| b.id == aim_id).map(|b| ([b.pos[0] + aim_off[0], b.pos[1] + aim_off[1]], b.id))).flatten();
                        // (The attack only looks for a new target when a throw starts: Weapon.Process
                        // calls Attack.FindTarget once lastEmission + rateFrames is reached; recording
                        // dart_040_club, rt 2378.)
                        let lost = locked.is_none() && !ambient && aim_id != 0;
                        let lost_obj = self.towers[t].weapons[w].aim_obj.filter(|_| lost);
                        let (mut target, target_id) = match (locked, best) {
                            (Some((p, id)), _) => (p, id),
                            (None, _) if lost && (weapon.projectile.instant || (weapon.projectile.only_target && weapon.emission.kind == "InstantDamage")) => (aimed, 0),
                            (None, Some((_, p, id))) => (p, id),
                            (None, None) if ambient => (pos, 0),
                            (None, None) => (aimed, 0),
                        };
                        // BloonDistanceRateBonus.GetRate (VERIFIED-DISASM @0xc742f0): rate / (1 +
                        // rangeMax x rateBonusPerRange x min(distance, rangeMax) / rangeMax).
                        let dist_k = weapon.dist_rate.map_or(1.0, |(per, max)| {
                            let d = (target[0] - pos[0]).hypot(target[1] - pos[1]).min(max);
                            1.0 + max * per * (d / max.max(1.0))
                        });
                        // (Its own attack's range: recording desperado_005 rt 73 with Q lines, the pistol
                        // (range 60) counts the MOAB 42 away, the shotgun (36) doesn't.)
                        let vagrant_n = weapon.vagrant.map(|_| self.vagrant_counts_in(t, pos, range));
                        let vagrant_n = vagrant_n.map(|(tw, b)| (tw, self.towers[t].weapons[w].zone_in.as_ref().map_or(b, |z| z.len() as u32)));
                        if !double {
                            if let Some(fx) = &attack.fire_fx {
                                self.place_fx.push((pos, fx.clone(), 1.0));
                            }
                        }
                        'shot: {
                            // Necromancer: undead bloons from the graveyard instead of a shot.
                            if let Some(nc) = &attack.necro {
                                let nc = nc.clone();
                                let (proj, tiers) = (weapon.projectile.clone(), self.towers[t].tiers);
                                let seg = self.towers[t].necro_seg;
                                self.necro_fire(t, &nc, &proj, seg, tiers, weapon.prince);
                                break 'shot;
                            }
                            // SendToBankModel: the shot's cash goes straight into the bank (up to capacity).
                            if weapon.to_bank {
                                if let (Some(c), Some(bank)) = (&weapon.projectile.cash, &tier.bank) {
                                    let v = cash_value(c, self.rand());
                                    let tw = &mut self.towers[t];
                                    tw.bank = (tw.bank + v).min(bank.capacity as f64);
                                }
                                break 'shot;
                            }
                            let tw = &mut self.towers[t];
                            // Towers without RotateToTargetModel keep their placement facing and fire
                            // along it (HYPOTHESIS P-NOROTATE: full-circle arcs start at that facing).
                            // RotateToTarget.ApplyRotation (VERIFIED-DISASM @0xdf6440): the tower turns to its
                            // target from its own position raised 10 in display space (10 / tan 60 down the
                            // map in the game's y, up in the clone's), each frame of the throw; the shot
                            // leaves along that facing.
                            let aim_from = [pos[0], pos[1] + DISPLAY_TOWER_HEIGHT / TAN60];
                            let mut dir = if attack.rotate && !turning { (target[1] - aim_from[1]).atan2(target[0] - aim_from[0]) } else { tw.angle };
                            // The game's own rotation: Rad2Deg x atan2f(x, y) of target - aim point in its
                            // frame (Vector2.get_Rotation); the eject point and the shots' directions are
                            // turned from it (Vector2.Rotate by -rotation).
                            let mut grot = (attack.rotate && !turning && weapon.aim_offset.is_none() && extra_rot == 0.0).then(|| RAD2DEG * crt_atan2(target[0] - aim_from[0], -(target[1] - aim_from[1])));
                            // (A projectile-made sub-tower's turret: relative to its rotation, back.)
                            if let (Some(b), true) = (tw.sub_base, target_id != 0) {
                                grot = grot.map(|g| b + (g - b));
                            }
                            if let (true, Some(_), Some(g)) = (lost, grot, tw.game_rot) {
                                grot = Some(g);
                                dir = (g - 90.0) * DEG2RAD;
                            }
                            // A tower that doesn't turn shoots from its own rotation (0 unless it has
                            // one): ArcEmission works in degrees from it (recording tack_000: a tack at
                            // -135 flies (-1.8856201, -1.8856163), not the radian arc's rounding).
                            // (So does the Phoenix, whose rotation its flight sets: recording wizard_050.)
                            let circling = tw.sub.as_ref().is_some_and(|s| s.spec.circle.is_some());
                            // (A tower turned by its other attacks keeps that rotation, in degrees, for
                            // the eject: recordings wizard_500 / 520, Shimmer's spark an ulp off from the
                            // radian facing.)
                            if grot.is_none() && !attack.rotate && weapon.aim_offset.is_none() && extra_rot == 0.0 && ((tw.angle + std::f32::consts::FRAC_PI_2).abs() < 1e-6 || circling || tw.game_rot.is_some()) {
                                grot = Some(tw.game_rot.unwrap_or(0.0));
                            }
                            // RotateToTargetModel.rotateTower off: the shot aims, the tower (and
                            // so Weapon.GetEjectPosition's rotation) stays as it was (recording
                            // wizard_025: a 0-2-5's first Wall of Fire ejects at rotation 0, aimed
                            // at -6.6).
                            // (A turret on a turning base still turns: recording dart_paragon_r40.)
                            let body_turns = attack.turn_body || tier.base_rotation;
                            // HYPOTHESIS P-SUBEJECT: a sub-tower's turret aimed at a bloon ejects along its aim
                            // (recording engineer_100: a Sentry's dart from its gun turned to -58.45).
                            let body_rot = (!body_turns && (target_id == 0 || tw.sub.is_none())).then(|| tw.game_rot.unwrap_or(0.0));
                            if grot.is_some() && body_turns {
                                tw.game_rot = grot;
                                if std::env::var_os("DBGE").is_some() { eprintln!("DBGAIM emitrot rt {} w {} target {} rot {:?}", self.round_clock, w, target_id, grot); } // DBGX
                                tw.turret_rel = grot.unwrap() - tw.base_rot;
                                // On a turning base the turret's angle is its base's plus its own
                                // (TransformBehavior), as the shot reads it (recording
                                // dart_paragon_r40, rt 276: a float step off the aim itself).
                                if tier.base_rotation {
                                    grot = Some(tw.base_rot + tw.turret_rel);
                                    tw.game_rot = grot;
                                }
                            }
                            // Broadside guns: the tower turns `extra_rotation` past the target (the
                            // paired gun keeps the facing its main gun set) and fires at facing - offset
                            // (Unity's clockwise degrees; map angles run anticlockwise).
                            let mut face = if attack.rotate && !turning && weapon.with_main.is_none() { dir - extra_rot } else { aim_face };
                            if !attack.rotate || turning {
                                face = tw.angle;
                            }
                            aim_face = face;
                            if let Some(off) = weapon.aim_offset {
                                dir = face - off.to_radians();
                            }
                            // Fire-without-target attacks (orbiting glaives) run silently in the background.
                            if !ambient {
                                tw.aim = face;
                                tw.set_attack_aim(ai, face, target, pos, middle, attack.turn_body);
                                if tw.attack_t > lead + 1.5 / FPS {
                                    tw.attack_t = 0.0;
                                }
                                tw.idle_t = 0.0;
                            }
                            if !double {
                                tw.shots += 1;
                                tw.weapons[w].emits += 1;
                            }
                            for ab in &mut tw.brews {
                                ab.shots_left = ab.shots_left.saturating_sub(1);
                            }
                            // Ammo crate: boosts the tower's own shots once its drop delay has passed.
                            let ammo = match &mut tw.ammo {
                                Some((d, n, am)) if *d <= 0.0 && ai < tier.attacks.len() => {
                                    *n -= 1;
                                    Some(am.clone())
                                }
                                _ => None,
                            };
                            if tw.ammo.as_ref().is_some_and(|(_, n, _)| *n == 0) {
                                tw.ammo = None;
                            }
                            // CritMultiplier.GetProjectile: every shot counts up; past nextCritAt the
                            // count restarts and nextCritAt is rolled again (SetNextCrit: a whole number
                            // in [lower, upper]); the shot that reaches nextCritAt crits. Initialise
                            // (the tier that adds the crit) starts the count; UpdatedModel (later
                            // upgrades) keeps it.
                            // A missed-frame shot (doubleShot) doesn't count but crits with its shot.
                            let crit = match weapon.crit.as_ref() {
                                Some(c) => {
                                    let roll = |r: f32| c.lower + ((r * (c.upper.saturating_sub(c.lower) + 1) as f32) as u32).min(c.upper.saturating_sub(c.lower));
                                    if self.towers[t].weapons[w].crit.1 == 0 {
                                        let r = self.rand();
                                        self.towers[t].weapons[w].crit = (0, roll(r).max(1));
                                    }
                                    let (cur, next) = self.towers[t].weapons[w].crit;
                                    let (cur, next) = if double {
                                        (cur, next)
                                    } else if cur + 1 > next {
                                        let r = self.rand();
                                        (1, roll(r).max(1))
                                    } else {
                                        (cur + 1, next)
                                    };
                                    self.towers[t].weapons[w].crit = (cur, next);
                                    (cur >= next).then_some(c.damage)
                                }
                                None => {
                                    self.towers[t].weapons[w].crit = (0, 0);
                                    None
                                }
                            };
                            let tw = &mut self.towers[t];
                            let owner = match &tw.sub {
                                Some(s) if s.spec.credit_parent => s.parent,
                                _ => tw.id,
                            };
                            let (tiers, emits) = (tw.tiers, tw.weapons[w].emits);
                            // AlternateProjectile (VERIFIED-DISASM GetAnimation @0xc690c0, GetProjectile
                            // @0xc69510): the weapon's own count + 1 each throw, back to 1 past interval;
                            // the throw where it equals interval is the alternate (recording ninja_003:
                            // Flash Bomb every 2nd shuriken of that weapon, not of the tower).
                            let (proj, emission) = match weapon.alternate.as_ref().filter(|a| a.interval > 0 && emits % a.interval == 0) {
                                Some(a) => (&a.projectile, &a.emission),
                                None => (&weapon.projectile, &weapon.emission),
                            };
                            // MonkeyFanClub's Mutator: every ArcEmission a single shot, the weapon
                            // ejecting from the fan club's point.
                            let fan_emission = buff.fan.as_ref().filter(|_| emission.kind == "Arc").map(|_| crate::data::Emission { count: 1, ..emission.clone() });
                            let emission = fan_emission.as_ref().unwrap_or(emission);
                            let weapon_eject = buff.fan.as_ref().map_or(weapon.eject, |fc| fc.eject);
                            let mut buffed = buff.projectile(proj);
                            // A crit shot looks like the crit's displayModel (Crossbow Master's crit arrow).
                            if crit.is_some() {
                                if let Some(d) = weapon.crit.as_ref().and_then(|c| c.display.clone()) {
                                    let mut q = buffed.take().unwrap_or_else(|| proj.clone());
                                    q.display = Some(d);
                                    buffed = Some(q);
                                }
                            }
                            if let Some(am) = &ammo {
                                let mut q = buffed.take().unwrap_or_else(|| proj.clone());
                                q.pierce += am.pierce;
                                q.damage *= am.damage_mult;
                                q.lifespan *= am.lifespan_mult;
                                if let Some(im) = am.immune {
                                    q.immune = im;
                                }
                                buffed = Some(q);
                            }
                            // Weapon.Emit: a weapon model faster than 1/120 s goes through
                            // ApplyExtraDamagePrecision (VERIFIED-DISASM @0xc6cc60) each emission:
                            // carry += 1/120 - max(rate, 0.001); past the rate, the whole rates in it
                            // (n) come off and every DamageModel in the shot does (1 + n) x damage
                            // (recording dart_050_club: a Plasma fan's dart every ~4 frames hits for 4).
                            {
                                let mr = weapon.rate * self.towers[t].weapons[w].buff_rate;
                                if 1.0f32 / 120.0 > mr {
                                    let r = mr.max(0.001);
                                    let st = &mut self.towers[t].weapons[w];
                                    let mut c = (1.0f32 / 120.0 - r) + st.dmg_carry;
                                    let mut n = 0u32;
                                    if c > r {
                                        while r < c {
                                            n += 1;
                                            c -= r;
                                        }
                                    }
                                    st.dmg_carry = c;
                                    if n > 0 {
                                        fn mult(p: &mut Proj, k: f32) {
                                            if p.damage > 0.0 {
                                                p.damage = k * p.damage + p.damage;
                                            }
                                            for c in &mut p.on_contact {
                                                mult(&mut c.projectile, k);
                                            }
                                            for e in &mut p.on_exhaust {
                                                mult(&mut e.projectile, k);
                                            }
                                        }
                                        let mut q = buffed.take().unwrap_or_else(|| proj.clone());
                                        mult(&mut q, n as f32);
                                        buffed = Some(q);
                                    }
                                }
                            }
                            let proj = buffed.as_ref().unwrap_or(proj);
                            // RandomArcEmissionModel.randomAngle (the weapon's RandomAngleOffset turns the
                            // shots once they are made: below).
                            let spread = emission.random_angle;
                            if spread > 0.0 {
                                dir += ((self.rand() * 2.0 - 1.0) * spread).to_radians();
                                grot = None;
                            }
                            // RandomTargetSpread.Emit -> Vector3.RandomOffsetFromRadius (VERIFIED-DISASM
                            // @0x180f47ff0) on Simulation.syncedRandom: d = NextDouble, then the angle
                            // NextDouble x 360; the point + (d x r, 0) turned by -angle (UCRT sinf / cosf),
                            // game frame (recording engineer_040: Bloon Trap lands 0.01 off its point).
                            if emission.target_spread > 0.0 {
                                let d = self.synced_double() as f32;
                                let deg = self.synced_double() as f32 * 360.0;
                                let (sn, cs) = crt_sin_cos(DEG2RAD * -deg);
                                let r = emission.target_spread;
                                let gy = -target[1];
                                let gx = (cs * d * r - sn * 0.0) + target[0];
                                let gy = (sn * d * r + cs * 0.0) + gy;
                                target = [gx, -gy];
                                // (Then a new Target at Map.GetTerrainHeight there, @0xc4b720:
                                // recording engineer_040, a spread foam timed to z 0. STAND-IN:
                                // no area heights, 0 on the track.)
                                aim_z = 0.0;
                            }
                            // Launch from the weapon's eject point. HYPOTHESIS P-EJECT: x is to the
                            // tower's right and y forward, as for sprites facing +y.
                            let (s, c) = if weapon.aim_offset.is_some() { face.sin_cos() } else { dir.sin_cos() };
                            // EmissionWithOffsetsModel: each shot from the next throw marker in turn.
                            let n = emission.markers.len();
                            let [ex, ey, ez] = if n > 0 { emission.markers[(self.towers[t].weapons[w].emits as usize).saturating_sub(1) % n] } else { weapon_eject };
                            // Weapon.GetEjectPosition (VERIFIED-DISASM @0xc6e520 + recording): the eject offset
                            // turned with the tower (game: Vector2.Rotate by -rotation; the clone's y is flipped,
                            // so its x mirrors), then Vector3.AddDisplaySpaceHeight moves it ejectZ / tan 60 down
                            // the game's map (up the clone's).
                            // The offset is built whole, then added to the tower's position.
                            let from = match grot.map(|r| body_rot.unwrap_or(r)) {
                                // Weapon.GetEjectPosition: Vector2.Rotate by -rotation, AddDisplaySpaceHeight
                                // (y - z / tan 60), then onto the tower's position.
                                Some(r) => {
                                    let a = DEG2RAD * -r;
                                    let (sa, ca) = crt_sin_cos(a);
                                    let (ox, oy) = (ca * ex - sa * ey, ca * ey + sa * ex);
                                    // FlipFollowPath.EmitPosition (VERIFIED-DISASM @0xc81b60): thrown
                                    // with the left hand, the base point moves by (0 - 2 ejectX, 0 -
                                    // 2 ejectY) on the model's flipped axes, turned with the tower
                                    // (recording boomerang_000_left: the throw starts 6.94 across).
                                    let base = match (self.towers[t].flip, weapon.flip_eject) {
                                        (true, Some([fx, fy])) => {
                                            let [wx, wy, _] = weapon.eject;
                                            let (dx, dy) = (if fx { 0.0 - (wx + wx) } else { 0.0 }, if fy { 0.0 - (wy + wy) } else { 0.0 });
                                            let (gx, gy) = (ca * dx - sa * dy, ca * dy + sa * dx);
                                            [pos[0] + gx, pos[1] - gy]
                                        }
                                        _ => pos,
                                    };
                                    // (A Line emission's start, dontUseTowerPosition: the turned offset onto the
                                    // position first, then the eject height (recordings desperado_030 / 050 / 250:
                                    // the rifle's barrel exact; the other way off by 1 ulp in y). HYPOTHESIS
                                    // P-LINEEJECT: the order of the two adds.)
                                    if emission.kind == "Line" { [base[0] + ox, (base[1] + -oy) + ez / TAN60] } else { [base[0] + ox, base[1] + -(oy - ez / TAN60)] }
                                }
                                None => [pos[0] + (c * ey - s * ex), pos[1] + ((s * ey + c * ex) + ez / TAN60)],
                            };
                            let start = self.projectiles.len();
                            if proj.link_radius {
                                let mut spec = proj.clone();
                                // LinkProjectileRadiusToTowerRange: the tower's range +
                                // projectileRadiusOffset (recording tack_400: 23 + 2).
                                spec.radius = range + proj.link_offset;
                                // (From the eject point too: recording tack_400, the ring sits
                                // ejectZ / tan 60 below the tower.)
                                self.emit_z = ez + self.tower_height(t);
                                self.emit_target = (target_id != 0).then_some(target_id);
                                self.emit(from, dir, emission, &spec, crit, tiers, owner);
                                // EjectEffectModel: the muzzle effect at the eject point (visual).
                                for (fx, k) in &weapon.eject_fx {
                                    self.place_fx.push((from, fx.clone(), *k));
                                }
                                // MultiEmissionModel: its secondary projectile, at the same eject
                                // point (recording ice_250: Absolute Zero's pierce-999 pulse with
                                // the blast, both at z 7).
                                if let Some(sec) = &emission.secondary {
                                    let single = Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None};
                                    self.emit(from, dir, &single, sec, None, tiers, owner);
                                }
                                self.emit_z = 0.0;
                                self.emit_target = None;
                            } else {
                                let bursts = self.bursts.len();
                                // A seeking shot from a tower that doesn't turn leaves towards its
                                // target from the eject point (recording tack_500: Inferno Ring's
                                // meteor at -75.73624, the line to its MOAB).
                                // EmissionRotationOffBloonDirection.OnEmit (VERIFIED-DISASM @0xc3b510):
                                // the shot turned to (bloon - eject point); with dontSetAfterEmit only
                                // OnGetDirection's (bloon - tower) (recording wizard_005: Prince of
                                // Darkness's bolts fly at their MOAB while the tower stays at 0).
                                let bloon_from = match weapon.bloon_dir.as_deref() { Some("tower") => Some(pos), Some("eject") => Some(from), _ => None };
                                // (Target.bloon's own position: no collision-circle offset.)
                                let bloon_at = self.bloons.iter().find(|b| b.id == target_id).map(|b| b.pos);
                                let seek_launch = bloon_from.is_none() && proj.seek.is_some() && !attack.rotate && target_id != 0;
                                let grot = if let (Some(f), Some(b)) = (bloon_from, bloon_at) {
                                    let r = RAD2DEG * crt_atan2(b[0] - f[0], -(b[1] - f[1]));
                                    dir = (r - 90.0) * DEG2RAD;
                                    Some(r)
                                } else if seek_launch {
                                    Some(RAD2DEG * crt_atan2(target[0] - from[0], -(target[1] - from[1])))
                                } else {
                                    grot
                                };
                                self.emit_rot = grot;
                                self.emit_z = ez + self.tower_height(t);
                                self.emit_target = (target_id != 0).then_some(target_id);
                                let s0 = self.projectiles.len();
                                self.emit(from, dir, emission, proj, crit, tiers, owner);
                                // (Its direction is the normalized line itself, each component / its
                                // length in f32, not one rebuilt from the angle: recording tack_500 with
                                // SimRecorder's D lines, the meteor's launch x -0.9462439.)
                                if seek_launch {
                                    let v = [target[0] - from[0], -(target[1] - from[1])];
                                    let n = (v[0] * v[0] + v[1] * v[1]).sqrt();
                                    if n > 0.0 {
                                        let nd = [v[0] / n, v[1] / n];
                                        for pr in &mut self.projectiles[s0..] {
                                            if pr.dir.is_some() {
                                                let sf = pr.spec.speed * FRAME_TIME;
                                                pr.dir = Some(nd);
                                                pr.vel = [nd[0] * sf, -nd[1] * sf];
                                            }
                                        }
                                    }
                                }
                                self.emit_rot = None;
                                self.emit_z = 0.0;
                                self.emit_target = None;
                                // MultiEmissionModel: its secondary projectile from the tower too.
                                if let Some(sec) = &emission.secondary {
                                    let single = Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None};
                                    self.emit(pos, dir, &single, sec, None, tiers, owner);
                                }
                                if ambient {
                                    self.bursts.truncate(bursts);
                                }
                            }
                            for pr in &mut self.projectiles[start..] {
                                pr.aim = (target_id != 0).then_some(target_id);
                                // CollideOnlyWithTarget.SetTarget (VERIFIED-DISASM @0x180c9ce60).
                                if pr.spec.only_target && !pr.spec.instant {
                                    pr.cot = (target_id != 0).then_some(target_id);
                                    pr.ignore_first = target_id == 0 && pr.spec.first_if_invalid;
                                }
                            }
                            // Spin.ProjectilesCreated (VERIFIED-DISASM @0x180c92b30): each new shot's
                            // direction turned by the angle (Vector3Boxed.Rotate) and its rotation + it
                            // (recording tack_040: Blade Maelstrom at -810.00006 / -630.00006).
                            if weapon.spin != 0.0 {
                                let ang = self.towers[t].spin.iter().find(|x| x.0 == weapon.name).map_or(0.0, |x| x.2);
                                let (sn, cs) = crt_sin_cos(DEG2RAD * -ang);
                                for pr in &mut self.projectiles[start..] {
                                    if let Some(d) = pr.dir {
                                        let nd = [cs * d[0] - sn * d[1], cs * d[1] + sn * d[0]];
                                        let sf = pr.spec.speed * FRAME_TIME;
                                        pr.dir = Some(nd);
                                        pr.vel = [nd[0] * sf, -nd[1] * sf];
                                    }
                                    pr.grot = pr.grot.map(|g| g + ang);
                                }
                            }
                            if !ambient {
                                self.sound(&weapon.fire_sounds, None);
                            }
                            // Instant shots at a track point (Wall of Fire) sit there instead of on a bloon.
                            // InstantDamageEmission.Emit (VERIFIED-DISASM @0xc61020) puts its projectile on
                            // the target's position and keeps the target; its Process (@0xc61370) then
                            // collides the two (Projectile.CollideBloon) the same frame (recording
                            // desperado_000: a Blue shot at 14 is a Red at 14).
                            // HYPOTHESIS P-ATTARGET: the Wererabbit's emission does the first part.
                            if proj.instant && target_id == 0 && !lost && emission.kind != "AtTarget" && emission.kind != "InstantDamage" {
                                // Instant.Process (VERIFIED-DISASM @0xcc1b30) puts it on its Target's
                                // position; a new projectile isn't processed the frame it is made, so
                                // that frame it is at the eject point (recording wizard_020: Wall of
                                // Fire at the eject point at 30, on its track spot from 31).
                                for pr in &mut self.projectiles[start..] {
                                    pr.instant_pos = Some(target);
                                }
                            } else if proj.instant || emission.kind == "AtTarget" || emission.kind == "InstantDamage" {
                                let bfound = self.bloons.iter().find(|b| b.id == target_id).map(|b| (b.pos, b.obj));
                                let bz = self.bloons.iter().find(|b| b.id == target_id).map(|b| b.height);
                                let (bpos, bobj) = (bfound.map(|f| f.0), bfound.map(|f| f.1).or(lost_obj));
                                for pr in &mut self.projectiles[start..] {
                                    pr.target = (target_id != 0).then_some(target_id);
                                    // Its target already dead (recording sub_052): made on the Target's
                                    // cached spot, then Instant.Process puts it on lastBloonPosition,
                                    // never set: the eject point.
                                    if lost && proj.instant {
                                        pr.target = Some(aim_id);
                                        pr.effect_back = Some(pr.pos);
                                    }
                                    // InstantDamageEmission with its target already dead (recording beast_025,
                                    // the Microraptor's shot): no collision, so its pierce is never used; with
                                    // no AgeModel it stays where it was aimed till the round's end, hitting
                                    // nothing (FilterAllExceptTarget).
                                    if lost && !proj.instant && emission.kind == "InstantDamage" && proj.only_target {
                                        pr.target = Some(aim_id);
                                        pr.frames_left = f32::INFINITY;
                                    }
                                    // An Instant shot from any other emission stays on its eject point the
                                    // frame it's made: Instant.Process (not run yet) puts it on its Target
                                    // (recording desperado_400: at the barrel at 73, on the MOAB at 74).
                                    if !(proj.instant && !emission.markers.is_empty()) {
                                        pr.pos = target;
                                        // (Its Position is the Target's, z too: the bloon's height; recording sniper_250, a
                                        // shot on a MOAB at z 120.)
                                        if let (Some(z), true) = (bz, proj.instant) {
                                            pr.z = z;
                                        }
                                    } else if pr.frames_left <= 0.0 {
                                        pr.frames_left = f32::INFINITY;
                                    }
                                    pr.target_cache = Some(target);
                                    pr.target_obj = bobj;
                                    pr.target_moved = early.contains(&target_id);
                                    if let Some(bp) = bpos {
                                        pr.inst_off = if locked.is_some() { aim_off } else { [target[0] - bp[0], target[1] - bp[1]] };
                                    }
                                }
                            }
                            // HYPOTHESIS P-INSTANTDOUBLE: a missed-frame extra shot isn't in this frame's
                            // InstantDamageEmission.Process; it hits in the next collision pass
                            // (recording sniper_005: two hits on 132 after the double shot on 131).
                            // CritRoll.ProjectilesCreated (VERIFIED-DISASM @0xc71dd0): each shot gets a roll
                            // of chance + CritChanceOffset; CritRollWithDistance's (@0xc71790) is the
                            // tower-to-Target distance, rounded half to even, x bonusPerDistance.
                            if let Some([add, mult, chance, per]) = weapon.crit_roll {
                                let d = ((target[0] - pos[0]).hypot(target[1] - pos[1])).round_ties_even();
                                for pr in &mut self.projectiles[start..] {
                                    pr.crit_roll = Some([chance + d * per, mult, add, 0.0, 0.0]);
                                }
                            }
                            // MarkedToPopProjectileBehavior.SetTarget (VERIFIED-DISASM @0x180ccc970): the
                            // target's lineage goes into its attack's FilterMarkedToPop.
                            if attack.mtp_filter && target_id != 0 {
                                if let Some(b) = self.bloons.iter().find(|b| b.id == target_id) {
                                    let mut ids = vec![b.layer];
                                    ids.extend(b.lineage.iter().flat_map(|l| l.iter().copied()));
                                    let k = if attack.model_name.contains("Marking") { 0 } else { 1 };
                                    if let Some(m) = self.towers[t].mtp.as_mut() {
                                        m.lin[k].extend(ids);
                                    }
                                }
                            }
                            if emission.kind == "InstantDamage" && target_id != 0 && !double {
                                let uids: Vec<u64> = self.projectiles[start..].iter().map(|pr| pr.uid).collect();
                                self.instant_hits.extend(uids);
                            }
                            // HYPOTHESIS P-OVERTIME: EmissionOverTimeModel repeats the shot count-1 more
                            // times, `frames` apart, at the same target. Only instant shots are
                            // delayed; travelling ones leave together.
                            if let Some(bf) = weapon.burst.as_ref().filter(|b| b.over_time && b.count > 1) {
                                for k in 1..bf.count {
                                    let s0 = self.projectiles.len();
                                    self.emit_rot = grot;
                                    self.emit_z = ez + self.tower_height(t);
                                self.emit_target = (target_id != 0).then_some(target_id);
                                    self.emit(from, dir, emission, proj, crit, tiers, owner);
                                    self.emit_rot = None;
                                    self.emit_z = 0.0;
                                    self.emit_target = None;
                                    if proj.instant || emission.kind == "AtTarget" || emission.kind == "InstantDamage" {
                                        for pr in &mut self.projectiles[s0..] {
                                            pr.target = (target_id != 0).then_some(target_id);
                                            pr.pos = target;
                                            pr.wait = k as f32 * bf.frames;
                                            pr.frames_left = pr.frames_left.max(pr.wait + 2.0);
                                        }
                                    }
                                }
                            }
                            // (Once per SpawnDart: Weapon.SpawnDart @0x180c51420 runs the behaviours'
                            // ProjectilesCreated after its missed-frame extra Emits, so those don't count
                            // as burst shots; recording desperado_502, a burst through rt 106.)
                            burst_after = weapon.burst.clone().filter(|b| !b.over_time && !double);
                            // HYPOTHESIS P-LINE: LineProjectileEmissionModel sweeps the shot along the
                            // range over its lifespan, wide enough that no bloon on the line is skipped.
                            if emission.kind == "Line" && emission.line_plain {
                                // LineProjectileEmission.Emit (VERIFIED-DISASM @0x180c42910): the line runs
                                // 400 from the emit point towards the Target (useTowerRotation: the tower's
                                // facing); CollisionGrid.GetInLine (MoveNext @0x18281ef30) takes every
                                // bloon GetInRange finds around the points thickness, 3 x thickness, ...
                                // along it (thickness = the projectile's radius), the list is ordered by
                                // distance from the start and Projectile.CollideBloons hits them at once,
                                // up to its pierce. The projectile itself stays at the start for its
                                // lifespan (recording desperado_050 rt 73: the rifle pops the MOAB the frame
                                // it fires).
                                let a = if emission.line_tower_rot { dir } else { (target[1] - from[1]).atan2(target[0] - from[0]) };
                                let (ls, lc) = a.sin_cos();
                                let r = proj.radius;
                                // (emitFromTarget, VERIFIED-DISASM: from the Target's position, emitFromTargetLength
                                // long; recording desperado_040 rt 73, the rifle's shot stays at the barrel.)
                                let len = emission.line_from_target.unwrap_or(400.0);
                                let line_start = if emission.line_from_target.is_some() { target } else { from };
                                let mut found: Vec<(f32, u64)> = Vec::new();
                                if r > 0.0 {
                                    let mut t = r;
                                    while t <= len {
                                        let c = [line_start[0] + lc * t, line_start[1] + ls * t];
                                        for (i, b) in self.bloons.iter().enumerate() {
                                            if found.iter().any(|f| f.1 == b.id) {
                                                continue;
                                            }
                                            let d = &self.defs[b.kind];
                                            if grid_hit(c, r, b.pos, b.cell_pos, d.radius)
                                                && (!self.has_grid_rank || self.ggrid.visits(c, r, i, b.cell_pos))
                                                && circles_hit(c, r, b.pos, b.rot, &d.circles)
                                            {
                                                found.push(((b.pos[0] - line_start[0]).hypot(b.pos[1] - line_start[1]), b.id));
                                            }
                                        }
                                        t = r + r + t;
                                    }
                                }
                                found.sort_by(|x, y| x.0.total_cmp(&y.0));
                                if std::env::var_os("DBGE").is_some() { eprintln!("DBGAIM line rt {} from {:?} a {} r {} found {:?} bloons {:?}", self.round_clock, from, a, r, found, self.bloons.iter().map(|b| (b.id, b.pos, b.cell_pos)).collect::<Vec<_>>()); } // DBGX
                                let ids: Vec<u64> = found.into_iter().map(|f| f.1).collect();
                                let uids: Vec<u64> = self.projectiles[start..].iter().map(|pr| pr.uid).collect();
                                for pr in &mut self.projectiles[start..] {
                                    pr.pos = from;
                                    pr.prev = from;
                                    pr.vel = [0.0, 0.0];
                                    pr.line_hits = Some(ids.clone());
                                    // (Emit ends with canCollideWithBloons = false, VERIFIED-DISASM: the
                                    // shot stays its lifespan without colliding again; recording
                                    // desperado_050 rt 1602, a Zebra 13 from the rifle's start untouched.)
                                    pr.line_spent = true;
                                }
                                self.instant_hits.extend(uids);
                                // (Emit collides them at once, before SpawnDart's CalcRateFrames: recording
                                // desperado_250 rt 73, the rifle's next rate counts no MOAB, 80 frames not 88.)
                                if self.in_step {
                                    self.instant_pass();
                                }
                            } else if emission.kind == "Line" {
                                let (ls, lc) = dir.sin_cos();
                                // Map-wide lines (Ray of Doom) reach across the map, not 9999999.
                                let (b0, b1) = self.bounds;
                                let range = range.min((b1[0] - b0[0]).hypot(b1[1] - b0[1]));
                                let step = range / (proj.lifespan * FPS).max(1.0);
                                for pr in &mut self.projectiles[start..] {
                                    pr.vel = [lc * step, ls * step];
                                    pr.spec.radius = pr.spec.radius.max(step * 0.5);
                                    pr.frames_left = pr.frames_left.max(1.0);
                                }
                                let to = [from[0] + lc * range, from[1] + ls * range];
                                match &weapon.line_fx {
                                    // LineEffectModel: its beam and, every few frames while it
                                    // fires, its start / end effects (visual).
                                    Some(lf) => {
                                        if self.steps % 12 == 0 {
                                            for (fx, at) in [(&lf.start, from), (&lf.end, to)] {
                                                if let Some(fx) = fx {
                                                    self.place_fx.push((at, fx.clone(), 1.0));
                                                }
                                            }
                                        }
                                        self.beams.push((from, to, lf.clone()));
                                    }
                                    None => self.tracers.push((from, to)),
                                }
                            }
                            if proj.seek.is_some() && !ambient {
                                for pr in &mut self.projectiles[start..] {
                                    // (A throw whose target died keeps that Target, invalid, until its first
                                    // look 5 frames on: recording sub_001, a dart flies straight past a live Red.)
                                    pr.seek = (!lost).then_some(target_id);
                                    pr.seek_off = aim_circle.map(|_| aim_off);
                                }
                            }
                            // HYPOTHESIS P-ARRIVE: ArriveAtTargetModel flies straight to where the target
                            // was when fired (its Curve only eases the motion), then expires.
                            if let Some(ar) = proj.arrive.as_ref().filter(|a| a.time > 0.0 || a.alt_speed > 0.0) {
                                let now = self.sim_elapsed;
                                for pr in &mut self.projectiles[start..] {
                                    // ArriveAtTarget.Process (VERIFIED-DISASM @0xca5c00): perc =
                                    // (time - startFrame) x 1/60 / timeToTake (at most 1), the position
                                    // origPos + (targetPos - origPos) x Curve.Evaluate(perc); it
                                    // arrives on the first frame perc reaches 1.
                                    // With timeToTake 0 it takes altSpeed: the 3D distance from the
                                    // eject point to the target on the ground / altSpeed (recording
                                    // engineer_040: Bloon Trap eases in over 3 frames, 0.584 and
                                    // 0.916 of the way).
                                    let t_take = if ar.time > 0.0 {
                                        ar.time
                                    } else {
                                        // (A segment point stands 2 up: recording engineer_032, Cleansing
                                        // Foam eases in 0.662 / 0.974 of the way from 10.7 to 2.)
                                        let (dx, dy, dz) = (target[0] - pr.pos[0], target[1] - pr.pos[1], pr.z - aim_z);
                                        (dx * dx + dy * dy + dz * dz).sqrt() / ar.alt_speed
                                    };
                                    if t_take > 0.0 && !ar.samples.is_empty() {
                                        pr.arrive_at = Some((now, pr.pos, target));
                                        pr.arrive_time = t_take;
                                        let mut k = (t_take * FPS).floor().max(1.0);
                                        while (k * (1.0f32 / 60.0)) / t_take < 1.0 {
                                            k += 1.0;
                                        }
                                        pr.vel = [0.0, 0.0];
                                        // canCollideWithBloons is set in the Process that arrives,
                                        // which runs after that frame's collisions: it hits from the
                                        // next frame (recording ninja_002: caltrops, rt 1441 vs 1442).
                                        pr.moving = k + 1.0;
                                        if ar.expire {
                                            pr.frames_left = k;
                                        }
                                        continue;
                                    }
                                    let n = if ar.time > 0.0 {
                                        ar.time * FPS
                                    } else {
                                        ((target[0] - pr.pos[0]).hypot(target[1] - pr.pos[1]) / (ar.alt_speed / FPS)).max(1.0)
                                    };
                                    pr.vel = [(target[0] - pr.pos[0]) / n, (target[1] - pr.pos[1]) / n];
                                    // Expiring ones still count as moving so filterCollisionWhileMoving
                                    // holds for the whole flight (a sentry thrown over the track).
                                    pr.moving = n;
                                    if ar.expire {
                                        pr.frames_left = n;
                                    }
                                }
                            }
                            // CreateTypedTowerCurrentIndex.ProjectilesCreated (VERIFIED-DISASM @0xc70db0):
                            // each new shot's CreateTypedTower picks its sentry now.
                            let typed = (proj.create_tower.len() == 4).then(|| self.typed_tower_choice());
                            for pr in &mut self.projectiles[start..] {
                                pr.camo = camo;
                                if pr.spec.create_tower.len() == 4 {
                                    pr.typed = typed;
                                }
                            }
                            // RandomAngleOffset.ProjectilesCreated (VERIFIED-DISASM @0xc8fe40): per
                            // shot, in order, a syncedRandom NextDouble; g = (float)d x (max - min) -
                            // (max - min) x 0.5; its direction turned by g (Vector3Boxed.Rotate), its
                            // rotation and EjectRotation + g. After the eject point (recording
                            // ninja_300: Double Shot's pair start from one spot).
                            // Weapon.fractionalPierce (as EmitOnDamage.Collide, VERIFIED-DISASM
                            // @0xcbb8c0): each shot's pierce - (int)pierce adds up; at 1 that shot
                            // gets one more (recording ninja_030: a Shinobi Ninja's 2.16 shurikens,
                            // every 6 or 7 throws one of 3.16).
                            for p in start..self.projectiles.len() {
                                let pr = &mut self.projectiles[p];
                                let st = &mut self.towers[t].weapons[w];
                                if pr.pierce.is_finite() && pr.pierce < 1e6 {
                                    let f = (pr.pierce - (pr.pierce as i32) as f32) + st.frac_pierce;
                                    st.frac_pierce = f;
                                    if 1.0 <= f {
                                        st.frac_pierce = f - 1.0;
                                        pr.pierce += 1.0;
                                    }
                                }
                            }
                            if weapon.spread > 0.0 {
                                let range = weapon.spread * 2.0;
                                for p in start..self.projectiles.len() {
                                    let d = self.synced_double();
                                    let g = d as f32 * range - range * 0.5;
                                    let pr = &mut self.projectiles[p];
                                    if let Some(dv) = pr.dir {
                                        let (sn, cs) = crt_sin_cos(DEG2RAD * -g);
                                        let nd = [cs * dv[0] - sn * dv[1], cs * dv[1] + sn * dv[0]];
                                        pr.dir = Some(nd);
                                        let sf = pr.spec.speed * FRAME_TIME;
                                        pr.vel = [nd[0] * sf, -nd[1] * sf];
                                    }
                                    pr.grot = pr.grot.map(|r| r + g);
                                }
                            }
                        }
                        // Weapon.SpawnDart after Emit: CalcRateFrames and UpdateMissedFrames;
                        // each whole missed frame is another shot now.
                        if !double {
                            let danger = self.danger;
                            // BloonDistanceRateBonus.GetRate (VERIFIED-DISASM @0xc6b080), from CalcRateFrames
                            // after the Emit: Attack.GetVisibleTarget with TargetType Close; none: rangeMax.
                            let dist_k = weapon.dist_rate.map_or(dist_k, |(per, max)| {
                                let d = self.visible_close(pos, range, camo, no_moab, attack).map_or(max, |q| (q[0] - pos[0]).hypot(q[1] - pos[1]).min(max));
                                1.0 + max * per * (d / max.max(1.0))
                            });
                            // Standoff.GetRate (VERIFIED-DISASM @0xc92df0): rate / (x + 1), x = maxRateIncrease
                            // less rateDecreasePerBloon for each of Attack.GetVisibleTargets after the
                            // first, at least 0 (0: the rate as it is).
                            let standoff_k = weapon.standoff.map_or(1.0, |[max, per]| {
                                let n = self.visible_count(pos, range, camo, no_moab, attack);
                                let mut x = max;
                                for _ in 1..n {
                                    x -= per;
                                    if x <= 0.0 {
                                        x = 0.0;
                                        break;
                                    }
                                }
                                x + 1.0
                            });
                            if std::env::var_os("DBGB").is_some() { eprintln!("DBGBT {} w{w} k {dist_k} s {standoff_k} vag {:?} rate {} rf {} sun {}", self.round_clock, vagrant_n, weapon.rate, self.towers[t].weapons[w].rate_frames, self.towers[t].weapons[w].sun_stacks); } // DBGX
                            let st = &mut self.towers[t].weapons[w];
                            st.standoff_k = standoff_k;
                            st.dist_k = dist_k;
                            if let Some(n) = vagrant_n {
                                st.vagrant_n = n;
                            }
                            st.danger = danger;
                            let r = st.rate(weapon);
                            st.rate_frames = rate_frames(r);
                            st.missed += missed_rate(r);
                            while st.missed > 1.0 {
                                st.missed -= 1.0;
                                queue.insert(qi, true);
                            }
                        }
                        // BurstWeaponBehavior.ProjectilesCreated (VERIFIED-DISASM @0xc75760):
                        // currentCount - 1; while shots are left, lastEmission = now +
                        // intervalFrames - rateFrames on the weapon's clock, so the next (a whole
                        // new throw, targeting again) comes intervalFrames later; the last one
                        // refills it (recording desperado_000: throws at 14 and 20, 92 and 98).
                        // The rateFrames is the one just calculated (recording desperado_002: a
                        // shot at 11 with the rate going 50 -> 49 sets -32).
                        if let Some(bf) = burst_after {
                            let st = &mut self.towers[t].weapons[w];
                            st.burst_left = st.burst_left.saturating_sub(1);
                            if st.burst_left < 1 {
                                st.burst_left = bf.count;
                            } else {
                                st.last = clock + bf.frames as i64 - st.rate_frames;                                if std::env::var_os("DBGB").is_some() { eprintln!("DBGBT burst w{w} clock {clock} frames {} rf {} -> {}", bf.frames, st.rate_frames, st.last); } // DBGX
                            }
                        }
                    if weapon.burst.as_ref().is_some_and(|b| b.always_empty && !b.over_time) {
                        let tw = &mut self.towers[t];
                        let d = game_dir(tw.game_rot.unwrap_or(0.0));
                        let r = buff.attack_range(attack);
                        let from = [pos[0], pos[1] + DISPLAY_TOWER_HEIGHT / TAN60];
                        tw.weapons[w].empty_pt = Some([from[0] + d[0] * r, from[1] + d[1] * r]);
                    }
                    }
                }
            }
            // Zoned<Bloon>.Process (@0x18239f600; addresses from SimRecorder's zone_methods probe), made
            // after the weapon so processed after it fires: Vagrant's bloon zone keeps the bloons within
            // its attack range (2D distance^2 <= range^2) as they are now; a shot counts the list as last
            // kept, bloons destroyed since included (recording desperado_002 rt 1570: a Green 61.57 away
            // at the frame's end isn't counted, a Pink 60.72 away is).
            {
                let mut wi = 0;
                for attack in tier.attacks.iter().chain(extra.iter()) {
                    let r = buff.attack_range(attack);
                    for w in &attack.weapons {
                        if w.vagrant.is_some() {
                            let p = self.attack_pos(t);
                            let ids: Vec<u64> = self.bloons.iter().filter(|b| { let q = b.pos; (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) <= r * r }).map(|b| b.id).collect();
                            if let Some(st) = self.towers[t].weapons.get_mut(wi) {
                                st.zone_in = Some(ids);
                                st.zone_r = r;
                            }
                        }
                        wi += 1;
                    }
                }
            }
            if only_temp.is_some() {
                continue;
            }
            // Ability attacks: one-shots go after their first volley (a weapon's cooldown
            // turned positive) or 1 s without one; the others when their lifespan runs out.
            let tw = &mut self.towers[t];
            let mut i0 = base_weapons;
            let mut k = 0;
            while k < tw.temp.len() {
                let n = tw.temp[k].attack.weapons.len();
                let x = &mut tw.temp[k];
                if x.delay > 0.0 {
                    x.delay -= 1.0;
                    i0 += n;
                    k += 1;
                    continue;
                }
                // (An ability used between frames runs only its own new attacks at once
                // (ActivateAttack.Activate): the others' lifespans aren't counted again.)
                if only_temp.is_some() && !x.now {
                    i0 += n;
                    k += 1;
                    continue;
                }
                // The lifespan runs on roundTime: a frame the round clock doesn't move isn't counted
                // (recording tack_250: Super Maelstrom's last throw at 659 though the clock held at
                // 255 for a step).
                if x.counted == Some(self.round_clock) {
                    i0 += n;
                    k += 1;
                    continue;
                }
                x.counted = Some(self.round_clock);
                x.frames_left -= 1.0;
                let fired = tw.weapons.get(i0..i0 + n).is_some_and(|c| c.iter().any(|w| w.last != NEVER && w.anim < 0));
                let done = if x.one_shot { fired || x.frames_left < -FPS } else { x.frames_left <= 0.0 };
                if done {
                    tw.temp.remove(k);
                    if tw.weapons.len() >= i0 + n {
                        tw.weapons.drain(i0..i0 + n);
                    }
                } else {
                    i0 += n;
                    k += 1;
                }
            }
        }
    }

    /// Processes (effects, then move) the bloons made before `stamp` that this frame hasn't yet:
    /// they come before it in the process list.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    /// The target index (`TargetIndex`), made from the bloons as they are now if they moved since.
    fn target_index_mut(&mut self) -> &mut TargetIndex {
        if self.target_index.is_none() {
            let mut segs: Vec<Vec<Vec<u32>>> = vec![Vec::new(); self.tracks.len()];
            for (bi, b) in self.bloons.iter().enumerate() {
                // The wind's bloons aren't filed on segments (towers_phase skips them).
                if b.wind.is_some() || b.held {
                    continue;
                }
                // (Its segment as of the frame's start: a tower processed after the bloons moved
                // still finds them where Path filed them; recording engineer_100, every Sentry
                // fires the frame after a bloon came in range, aimed at where it is now.)
                let k = (b.wind_seg.unwrap_or(b.prev_dist) / 7.0).max(0.0) as usize;
                let Some(path) = segs.get_mut(b.path as usize) else { continue };
                if path.len() <= k {
                    path.resize(k + 1, Vec::new());
                }
                path[k].push(bi as u32);
            }
            let mut base = Vec::with_capacity(segs.len());
            let mut total = 0;
            for path in &segs {
                base.push(total);
                total += path.len();
            }
            let mut seg_of = vec![u32::MAX; self.bloons.len()];
            for (pi, path) in segs.iter().enumerate() {
                for (k, ids) in path.iter().enumerate() {
                    for &i in ids {
                        seg_of[i as usize] = (base[pi] + k) as u32;
                    }
                }
            }
            self.target_index = Some(TargetIndex { segs, seg_of, base, ..Default::default() });
        }
        self.target_index.as_mut().expect("target index")
    }

    /// Each path's leading (or, with `last`, last) bloon an attack can see, for First / Last /
    /// Strong (Path.bestFirst*): its index, distance and Path.bloonsList rank.
    fn path_leads(&self, last: bool, camo: bool, no_moab: bool) -> Vec<Option<(usize, f32, u32)>> {
        let mut lead: Vec<Option<(usize, f32, u32)>> = vec![None; self.tracks.len()];
        for (bi, b) in self.bloons.iter().enumerate() {
            let d = &self.defs[b.kind];
            if (b.camo(d) && !camo) || (no_moab && d.tags.iter().any(|t| t == "Moabs")) || b.wind.is_some() || b.held {
                continue;
            }
            let rank = self.path_rank.get(&b.id).map_or(u32::MAX, |r| r.1);
            if let Some(l) = lead.get_mut(b.path as usize) {
                if l.is_none_or(|(_, dist, r)| if last { b.dist < dist } else { b.dist > dist } || (b.dist == dist && rank < r)) {
                    *l = Some((bi, b.dist, rank));
                }
            }
        }
        lead
    }

    /// Path.FindBestBloon for Strong with camo priority: per path, the strongest camo bloon.
    fn strong_leads(&self, camo: bool, no_moab: bool) -> Vec<Option<(usize, f32, f32)>> {
        let mut strong_lead: Vec<Option<(usize, f32, f32)>> = vec![None; self.tracks.len()];
        let mut order: Vec<usize> = (0..self.bloons.len()).collect();
        order.sort_by_cached_key(|&i| self.path_rank.get(&self.bloons[i].id).map_or(u32::MAX, |r| r.1));
        for bi in order {
            let b = &self.bloons[bi];
            let d = &self.defs[b.kind];
            if (b.camo(d) && !camo) || (no_moab && d.tags.iter().any(|t| t == "Moabs")) {
                continue;
            }
            // (TargetStrongPrioCamo: the strongest camo bloon first; without one, plain
            // TargetStrong (below): recording wizard_502_strong rt 81, Dragon's Breath
            // takes the first-listed Black of a Black and a White on one segment.)
            if !b.camo(d) {
                continue;
            }
            if let Some(l) = strong_lead.get_mut(b.path as usize) {
                if l.is_none_or(|(_, dg, ds)| d.danger > dg || (d.danger == dg && b.dist > ds)) {
                    *l = Some((bi, d.danger, b.dist));
                }
            }
        }
        strong_lead
    }

    fn process_bloons_before(&mut self, stamp: u64, early: &mut FastSet<u64>, round_clock: i64,
        dots: &mut Vec<DotTick>, emits: &mut Vec<([f32; 2], Box<Proj>, u64, u64, u64)>,
        zones: &[([f32; 2], f32, crate::data::Zone)], shoves: &[([f32; 2], crate::data::MoabShove)], leak_voice: &mut bool) {
        #[cfg(test)] let schedule_start=std::time::Instant::now();
        let mut j = 0;
        while j < self.bloons.len() {
            let b = &self.bloons[j];
            if b.born >= stamp || b.fresh || early.contains(&b.id) {
                j += 1;
                continue;
            }
            early.insert(b.id);
            self.target_index = None;
            self.fill_first_id.get_or_insert(self.next_id);
            self.tick_bloon_effects(j, round_clock, dots, emits);
            if !self.move_bloon(j, zones, shoves, leak_voice) {
                j += 1;
            }
        }
        #[cfg(test)] {self.perf_detail[1]+=schedule_start.elapsed().as_secs_f64();}
    }

    /// Bloon.Process's move for bloon `i` (speed, track, splitters, rotation, leak); true when
    /// it leaked and was removed (the next bloon is now at `i`).
    #[allow(clippy::type_complexity)]
    fn move_bloon(&mut self, i: usize, zones: &[([f32; 2], f32, crate::data::Zone)], shoves: &[([f32; 2], crate::data::MoabShove)], leak_voice: &mut bool) -> bool {
        self.bloons[i].height = if self.defs[self.bloons[i].kind].is_moab { 120.0 } else { 20.0 };
        let mut mult = self.bloons[i].speed_mult(&self.defs[self.bloons[i].kind]);
        if mult > 0.0 && !zones.is_empty() {
            let d = &self.defs[self.bloons[i].kind];
            let p = self.bloons[i].pos;
            let mut seen: Vec<&str> = Vec::new();
            for (c, r, z) in zones {
                // SlowBloonsZone.FilterBloon (VERIFIED-DISASM @0xd91ad0): its filters, then with
                // a bloonTag Bloon.HasTags (all of bloonTags) is required, or with inclusive false
                // forbidden. Zoned tests the bloon's centre against the range (recording ice_050:
                // a MOAB slows once its centre is 44.89 from the tower, range 40 + 5).
                let has_all = z.tags.iter().all(|t| d.tags.contains(t));
                if (z.camo || !d.is_camo)
                    && !z.exclude_tags.iter().any(|t| d.tags.contains(t))
                    && (z.tag.is_empty() || has_all == z.inclusive)
                    && (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) <= r * r
                    && !seen.contains(&z.id.as_str())
                {
                    // BloonEnter adds its mutator (speedScale; speedChange is 0 in every model),
                    // which multiplies with the bloon's other slows (recording ice_250: Arctic
                    // Wind's 0.7 x Permafrost's 0.75 on a MOAB). isUnique: one per mutation id
                    // (INFERRED: the first tower's).
                    seen.push(&z.id);
                    mult *= z.scale;
                }
            }
        }
        if !shoves.is_empty() {
            let d = &self.defs[self.bloons[i].kind];
            let p = self.bloons[i].pos;
            for (c, sh) in shoves {
                let rr = sh.range + d.radius;
                if (sh.camo || !self.bloons[i].camo(d)) && (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) <= rr * rr {
                    let has = |t: &str| d.tags.iter().any(|x| x == t);
                    let cap = if has("Moab") { Some(sh.moab) } else if has("Bfb") { Some(sh.bfb) } else if has("Zomg") { sh.zomg } else { None };
                    if let Some(cap) = cap {
                        mult = mult.min(cap);
                    }
                }
            }
        }
        let b = &mut self.bloons[i];
        if let Some(EffectKind::Trance { centre, radius, angle, rate }) = b.effects.iter_mut().map(|e| &mut e.kind).find(|k| matches!(k, EffectKind::Trance { .. })) {
            *angle += *rate;
            b.pos = [centre[0] + *radius * angle.cos(), centre[1] + *radius * angle.sin()];
            return false;
        }
        // MoveWithWind.Process (VERIFIED-DISASM @0xffeeb0) with Bloon.Process skipping the track
        // move while offTrackCount > 0: within 3 x speedFrames of its track spot the wind ends,
        // else it moves straight at it by speedFrames x speedMultiplier.
        if let Some(m) = b.wind {
            // (A child carried from birth has had its first Process: recording ninja_020 rt
            // 3054, a blown Pink moves on the frame after its wind ends.)
            b.fresh = false;
            // (Attached in this frame's collisions: it holds still until its first Process,
            // next frame; recording ninja_010, a Yellow blown back at 237 sits at -96.26.)
            if std::mem::take(&mut b.wind_new) {
                return false;
            }
            let sf = speed_frames(self.defs[b.kind].speed);
            let to = self.tracks[b.path as usize].bloon_pos(b.dist);
            let (dx, dy) = (to[0] - b.pos[0], to[1] - b.pos[1]);
            let lim = sf * 3.0;
            if dx * dx + dy * dy <= lim * lim {
                b.wind = None;
                b.wind_seg = None;
                b.wind_direct = false;
                b.wind_model = None;
            } else {
                let l = (dx * dx + dy * dy).sqrt();
                let st = sf * m;
                b.pos = [b.pos[0] + dx / l * st, b.pos[1] + dy / l * st];
            }
            return false;
        }
        let mut moved = None;
        if b.fresh {
            b.fresh = false;
        } else {
            b.first_move = std::mem::take(&mut b.unplaced);
            b.mult = mult;
            moved = Some(speed_frames(b.model_speed(b.frozen_speed(&self.defs[b.kind], self.defs[b.kind].speed))) * b.track_mult * 1.0 * mult);
            // Bloon.get_TotalSpeedFrames (VERIFIED-DISASM @0xfd7730): speedFrames (speed /
            // 60) x trackSpeedMultiplier x the spawner's multiplier (1) x behaviours'.
            b.dist = (b.dist + speed_frames(b.model_speed(b.frozen_speed(&self.defs[b.kind], self.defs[b.kind].speed))) * b.track_mult * 1.0 * mult).max(0.0);
            // Bloon.Move keeps a moving bloon from trailing the entrance by more than
            // 1.25 x its radius; a child made behind it this frame stays put until it moves
            // (recording dart_400: a Lead's fourth Pink sits at 169.2, then 173.6).
            if let Some(sd) = self.track_ends.get(b.path as usize).and_then(|e| e.0) {
                let m = sd - self.defs[b.kind].radius * 1.25;
                if b.dist <= m {
                    b.dist = m;
                }
            }
        }
        // At its track's end, a bloon with an exit splitter goes on to the splitter's next
        // path (Splitter.Rout: round-robin, counter per splitter), keeping the overflow.
        loop {
            let k = b.path as usize;
            let len = self.tracks[k].length();
            let (to, s) = &self.track_exit[k];
            if b.dist < len || to.is_empty() {
                break;
            }
            let r = &mut self.routs[*s];
            if *r >= to.len() {
                *r = 0;
            }
            let next = to[*r];
            *r += 1;
            b.dist -= len;
            b.base += len;
            b.path = next as u8;
        }
        let tr = &self.tracks[b.path as usize];
        b.pos = tr.bloon_pos(b.dist);
        b.track_mult = tr.speed_mult(b.dist);
        // Bloon.UpdatePositionAlongTrack (VERIFIED-DISASM @0xfd5e80, inlined in Process): a
        // rotateToFollowPath bloon turns Math.LerpAngle(prevRot, the point's rotation,
        // TotalSpeedFrames x 0.5) of the way to the path's heading.
        if let Some(tsf) = moved.filter(|_| self.defs[b.kind].rotate_to_follow_path) {
            if let Some(r) = self.track_rot.get(b.path as usize).filter(|v| !v.is_empty()).map(|v| v[((b.dist / 0.01).max(0.0) as usize).min(v.len() - 1)]) {
                b.rot = lerp_angle(b.rot, r, tsf * 0.5);
            }
        }
        // Bloon.Process (VERIFIED-DISASM): it leaks once distance - radius reaches leakDist
        // (the last point on the map), or at the very end of a track without an exit.
        let leak_at = self.track_ends.get(b.path as usize).and_then(|e| e.1);
        if leak_at.is_some_and(|l| b.dist - self.defs[b.kind].radius >= l) || b.dist >= tr.length() {
            // Ability.OnPreLivesLost (VERIFIED-DISASM @0x180dfa8e0): an activateOnPreLeak ability goes
            // off first (Bomb Blitz's Bomb Storm); if that destroys the bloon, it doesn't leak
            // (recording bomb_005: 52 Blacks popped at 1190, cash + 572). HYPOTHESIS P-PRELEAKLOOP:
            // the movement loop carries on from this index.
            let leak_id = self.bloons[i].id;
            if self.leak_immune <= 0.0 && self.pre_leak_abilities() && !self.bloons.iter().any(|x| x.id == leak_id) {
                return true;
            }
            let b = &mut self.bloons[i];
            let tr = &self.tracks[b.path as usize];
            let _ = tr;
            if self.leak_immune <= 0.0 {
                let lost = self.rbe[b.kind] as f64; // P-LEAK
                self.lives -= lost;
                self.logic.lives_lost += lost;
                self.leaked = true;
                let kind = b.kind;
                self.record_leak(kind);
                *leak_voice = true;
                self.vigilante_lives_lost();
            }
            let (id, at) = (self.bloons[i].id, self.bloons[i].pos);
            self.plist_kill(id);
            self.fact_kill(i);
            self.bloons.swap_remove(i);
            self.bloon_destroyed(id, at);
            return true;
        }
        false
    }

    /// VagrantWeaponBehavior's zones (GetTowerRange: the tower's range): other towers (INFERRED:
    /// not sub-towers or heroes, its Filter; centre within the range) and bloons within the range
    /// + 2 (recording desperado_002: a Pink 60.72 from a range-60 tower counts, one 62.11 doesn't).
    /// FilterMarkedToPop.FilterBloon (VERIFIED-DISASM @0x180c1fe30): a bloon with an id of its
    /// lineage in the attack's list is filtered out.
    fn mtp_filtered(&self, t: usize, name: &str, b: &Bloon) -> bool {
        let Some(m) = self.towers[t].mtp.as_ref() else { return false };
        let l = &m.lin[if name.contains("Marking") { 0 } else { 1 }];
        l.contains(&b.layer) || b.lineage.iter().flat_map(|v| v.iter()).any(|x| l.contains(x))
    }

    /// MarkedToPop.Process (VERIFIED-DISASM @0x180e3c0b0), before the tower's attacks: marking
    /// while its weapon is ready, its attack has a target (hadValidTarget) and markingTimeFrames
    /// haven't passed since the activation; else the execution phase starts (marking paused,
    /// its lineage cleared). Executing while its weapon is ready, it has a target and
    /// executionTimeMaxFrames haven't passed; else it ends (the others unpaused).
    fn mtp_step(&mut self, t: usize) {
        let now = self.round_clock;
        let tw = &self.towers[t];
        let Some(m) = tw.mtp.clone() else { return };
        let mi = tw.temp.iter().position(|x| x.attack.model_name.contains("MarkingAttack"));
        let ei = tw.temp.iter().position(|x| x.attack.model_name.contains("ExecutionAttack"));
        let (Some(mi), Some(ei)) = (mi, ei) else {
            self.towers[t].mtp = None;
            return;
        };
        let extra: usize = tw.temp.iter().map(|x| x.attack.weapons.len()).sum();
        let base = tw.weapons.len().saturating_sub(extra);
        let wpos = |k: usize| base + tw.temp[..k].iter().map(|x| x.attack.weapons.len()).sum::<usize>();
        let ready = |k: usize| tw.weapons.get(wpos(k)).is_none_or(|w| w.ready(now));
        let had = |k: usize| tw.had_target.get(&tw.temp[k].attack.model_name).copied().unwrap_or(false);
        if m.phase == 1 && ready(mi) && !(had(mi) && ((now - m.act_at) as f32) < m.frames[0]) {
            let tw = &mut self.towers[t];
            tw.temp[mi].delay = f32::INFINITY;
            tw.temp[ei].delay = 0.0;
            let mm = tw.mtp.as_mut().unwrap();
            mm.phase = 2;
            mm.exec_at = now;
            mm.lin[0].clear();
            return;
        }
        if m.phase == 2 && ready(ei) && !(had(ei) && ((now - m.exec_at) as f32) < m.frames[1]) {
            let tw = &mut self.towers[t];
            tw.temp.retain(|o| !(o.attack.model_name.contains("MarkingAttack") || o.attack.model_name.contains("ExecutionAttack")));
            tw.mtp = None;
        }
    }

    fn vagrant_counts(&self, t: usize, pos: [f32; 2]) -> (u32, u32) {
        self.vagrant_counts_in(t, pos, self.range(t))
    }

    fn vagrant_counts_in(&self, t: usize, pos: [f32; 2], range: f32) -> (u32, u32) {
        let dist = |q: [f32; 2]| (q[0] - pos[0]).hypot(q[1] - pos[1]);
        let towers = (0..self.towers.len())
            .filter(|&o| o != t && self.towers[o].sub.is_none() && self.hero_def(self.towers[o].def).is_none() && dist(self.towers[o].pos) <= range)
            .count() as u32;
        let bloons = self.bloons.iter().filter(|b| dist(b.pos) <= range + 2.0).count() as u32;
        (towers, bloons)
    }

    /// VigilanteTowerBehavior.OnLivesLost (VERIFIED-DISASM @0xde0080), on roundTime: nothing while
    /// its buff runs (within the duration of lastActivatedAt); none during the cooldown after it
    /// (duration + (int)(1 / (x + 1) x cooldownFrames / 60 x 60), x a model float at 0x28,
    /// INFERRED 0: no field of the exported model is there); else the buff (VigilanteMutator) for
    /// durationFrames and lastActivatedAt = now (recording desperado_002: the leak at 533 turns
    /// rateFrames 48 into 29).
    fn vigilante_lives_lost(&mut self) {
        let now = self.round_clock;
        for t in 0..self.towers.len() {
            let Some([_, _, duration, cooldown]) = self.tier(t).and_then(|tr| tr.vigilante) else { continue };
            let since = self.towers[t].vigilante_at.map(|a| now - a);
            let (duration, cool) = (duration as i64, ((1.0f32 / (0.0 + 1.0)) * cooldown * (1.0 / 60.0) * 60.0) as i64);
            if std::env::var_os("DBGE").is_some() { eprintln!("DBGAIM vig now {now} since {since:?} dur {duration} cool {cool}"); } // DBGX
            if since.is_some_and(|s| s <= duration + cool) {
                continue;
            }
            self.towers[t].vigilante_at = Some(now);
            // AddMutator applies the mutation at once: Weapon.UpdatedModel recalculates rateFrames
            // now, with the Vagrant zones as they are (recording desperado_002: 29 frames at 533).
            let rate = self.tier(t).and_then(|tr| tr.vigilante).map_or(1.0, |v| v[0]);
            let ws: Vec<crate::data::Weapon> = self.tier(t).map(|tr| tr.attacks.iter().flat_map(|a| a.weapons.iter().cloned()).collect()).unwrap_or_default();
            let p = self.towers[t].pos;
            let n = self.towers[t].vagrant_zone.unwrap_or_else(|| self.vagrant_counts(t, p));
            for st in &mut self.towers[t].weapons {
                if let Some(w) = ws.iter().filter(|w| w.name == st.key.0).nth(st.key.1.max(1) as usize - 1) {
                    st.buff_rate *= rate;
                    if w.vagrant.is_some() {
                        st.vagrant_n = (n.0, st.zone_in.as_ref().map_or(n.1, |z| z.len() as u32));
                    }
                    st.rate_frames = rate_frames(st.rate(w));
                }
            }
        }
    }

    /// Bloon.Process's mutator timers and DamageOverTime / emit ticks for bloon `i` (the damage
    /// and emissions are applied by the caller, after the moves).
    #[allow(clippy::type_complexity)]
    fn tick_bloon_effects(&mut self, i: usize, round_clock: i64, dots: &mut Vec<DotTick>, emits: &mut Vec<([f32; 2], Box<Proj>, u64, u64, u64)>) {
        let now = self.sim_elapsed;
        let b = &mut self.bloons[i];
        // (Bloons made this frame aren't in the process list yet.)
        if b.fresh {
            return;
        }
        // In behaviour order (`Effect::seq`).
        let mut by_seq: Vec<usize> = (0..b.effects.len()).collect();
        by_seq.sort_by_key(|&k| b.effects[k].seq);
        for k in by_seq {
            // (Only the first mutator with an id applies: ApplyMutation's limiter.)
            let behind = b.effects[..k].iter().any(|x| x.id == b.effects[k].id);
            let e = &mut b.effects[k];
            e.frames_left -= 1.0;
            // HYPOTHESIS P-NEWBEHAVIOR: a behaviour made this frame (its mutator went on in the
            // collision pass) isn't processed until the next (recording glue_005: a
            // triggerImmediate DoT put on at 81 ticks at 82; ninja_004: a Sticky Bomb's splash
            // 177 frames after the hit, not 176).
            if e.added == now && matches!(e.kind, EffectKind::Dot { .. } | EffectKind::PopEmit { .. } | EffectKind::Emit { .. }) {
                continue;
            }
            match &mut e.kind {
                // EmitOnPop.Process (@0x83a5a0): ignoringSameFrameDegrade = false.
                EffectKind::PopEmit { skip, .. } => *skip = false,
                EffectKind::Dot { .. } if behind => {}
                // DamageOverTime.CheckTick (VERIFIED-DISASM @0x1003ac0): once the
                // round clock is intervalFrames past lastTick (Initialise: the clock when it
                // went on), it ticks and lastTick = now (recording boomerang_500: on at 77,
                // 100 damage at 137).
                EffectKind::Dot { damage, interval, tick, immune, mods, .. } => {
                    if round_clock - *tick as i64 >= (*interval as f64).round_ties_even() as i64 {
                        let prev = *tick;
                        *tick = round_clock as f32;
                        dots.push((b.id, dot_damage(*damage, mods, &self.defs[b.kind].tags), *immune, e.owner, e.distribute, e.id.clone(), prev));
                    }
                }
                EffectKind::Emit { interval, tick, proj } => {
                    *tick += 1.0;
                    if *tick >= *interval * FPS {
                        *tick = 0.0;
                        emits.push((b.pos, proj.clone(), e.owner, b.id, b.layer));
                    }
                }
                _ => {}
            }
        }
        // An expiring first DoT hands its lastTick on to the next with its id (the same
        // DamageOverTime, now on that one's model).
        for k in 0..b.effects.len() {
            if b.effects[k].frames_left > 0.0 {
                continue;
            }
            let EffectKind::Dot { tick, .. } = b.effects[k].kind else { continue };
            if b.effects[..k].iter().any(|x| x.id == b.effects[k].id && x.frames_left > 0.0) {
                continue;
            }
            let id = b.effects[k].id.clone();
            if let Some(n) = b.effects[k + 1..].iter_mut().find(|x| x.id == id && x.frames_left > 0.0) {
                if let EffectKind::Dot { tick: t, .. } = &mut n.kind {
                    *t = tick;
                }
            }
        }
        b.effects.retain(|e| e.frames_left > 0.0);
    }

    /// BloonManager.GetClosestTarget (VERIFIED-DISASM @0xfe8700): track segments within
    /// `distance` + 40 of `q`, nearest segment point first; on it, the first bloon in list order
    /// that `ok` lets through whose point (by `at`) is within `distance` + 2 (a MOAB: any of its
    /// circles within `distance` + the circle's radius).
    fn closest_target(&self, q: [f32; 2], distance: f32, ok: impl Fn(&Bloon) -> bool, at: impl Fn(&Bloon) -> [f32; 2]) -> Option<usize> {
        #[cfg(test)] let _closest_timer=sim_profile::Timer::new(1);
        let reach = distance + 40.0;
        let mut best: Option<usize> = None;
        let mut best_d = f32::MAX;
        // The bloons on each segment within reach, in storage order: one pass over the bloons
        // instead of one per segment looked at.
        let near = |pi: usize, k: usize| {
            let live = self.track_seg_ok.get(pi).and_then(|v| v.get(k)).copied().unwrap_or(false);
            live && self.track_segs.get(pi).and_then(|sg| sg.get(k)).is_some_and(|sp| (sp[0] - q[0]).powi(2) + (sp[1] - q[1]).powi(2) <= reach * reach)
        };
        let mut on_seg: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        for (i, b) in self.bloons.iter().enumerate() {
            let k = (b.wind_seg.unwrap_or(b.prev_dist) / 7.0).max(0.0) as usize;
            if near(b.path as usize, k) {
                on_seg.entry((b.path as usize, k)).or_default().push(i);
            }
        }
        for (pi, sg) in self.track_segs.iter().enumerate() {
            for (k, sp) in sg.iter().enumerate() {
                let live = self.track_seg_ok.get(pi).and_then(|v| v.get(k)).copied().unwrap_or(false);
                let d2 = (sp[0] - q[0]) * (sp[0] - q[0]) + (sp[1] - q[1]) * (sp[1] - q[1]);
                if !live || (sp[0] - q[0]).powi(2) + (sp[1] - q[1]).powi(2) > reach * reach || !(d2 < best_d) {
                    continue;
                }
                let in_range = (sp[0] - q[0]).powi(2) + (sp[1] - q[1]).powi(2) <= (distance + 2.0) * (distance + 2.0);
                let mut on: Vec<usize> = on_seg.remove(&(pi, k)).unwrap_or_default();
                on.sort_by_cached_key(|&i| self.path_rank.get(&self.bloons[i].id).map_or(u32::MAX, |r| r.1));
                for i in on {
                    let b = &self.bloons[i];
                    let d = &self.defs[b.kind];
                    if !ok(b) {
                        continue;
                    }
                    let hit = if !d.circles.is_empty() {
                        let a = DEG2RAD * -b.rot;
                        let (sa, ca) = crt_sin_cos(a);
                        d.circles.iter().any(|c| {
                            let (ox, oy) = (ca * c[0] - sa * c[1], ca * c[1] + sa * c[0]);
                            let bp = at(b);
                            let (dx, dy) = (bp[0] + ox - q[0], bp[1] - oy - q[1]);
                            dx * dx + dy * dy < (distance + c[2]) * (distance + c[2])
                        })
                    } else {
                        in_range
                    };
                    if hit {
                        best = Some(i);
                        best_d = d2;
                        break;
                    }
                }
            }
        }
        best
    }

    /// RetargetOnContact.Retarget (VERIFIED-DISASM @0xcf6c80): one bounce used; BloonManager
    /// .GetClosestTarget (@0xfe8700) from the shot within `distance` (segments within distance +
    /// 40, nearest segment point first, its first bloon in list order the shot may hit: its point
    /// within distance + 2, or a MOAB by its circles); the shot then flies straight at it
    /// (normalized, at its speed, rotation from the direction), or expires if none and it should.
    fn retarget(&mut self, p: usize, rt: &crate::data::Retarget) {
        self.projectiles[p].bounces += 1;
        let pr = &self.projectiles[p];
        // An instant shot is where its Target is this frame (recording sniper_250 rt 1181: the bounce
        // off a Ceramic turns -7.87 from its new spot, not -9.02 from the last).
        // (Its Instant ran in process-list order: a bloon newer than the shot hadn't moved yet;
        // recording sniper_040 rt 2286, off a Blue made after it.)
        let q = match pr.target.filter(|_| pr.spec.instant).and_then(|id| self.bloons.iter().find(|b| b.id == id)) {
            Some(b) => {
                let at = if b.born > pr.born { b.prev } else { b.pos };
                [at[0] + pr.inst_off[0], at[1] + pr.inst_off[1]]
            }
            None => pr.prev,
        };
        let at = |b: &Bloon| if b.born < pr.born { b.pos } else { b.prev };
        let best = self.closest_target(q, rt.distance, |b| !b.hit_by(&pr.hit) && (!b.camo(&self.defs[b.kind]) || pr.camo), at);
        let tpos = best.map(|i| at(&self.bloons[i]));
        let tid = best.map(|i| self.bloons[i].id);
        let pr = &mut self.projectiles[p];
        // (Target.set_bloon: the new bloon becomes its Target.)
        if tid.is_some() {
            pr.aim = tid;
            // An instant shot sits on its Target: the next frame it is on the new bloon
            // (recording sniper_032 rt 1866: Bouncing Bullet jumps to the Blue and pops it).
            if pr.spec.instant {
                pr.target = tid;
                pr.inst_off = [0.0; 2];
                // Instant.SetTarget (VERIFIED-DISASM @0x180cc21c0): lastBloonPosition = the new
                // Target's position (recording sniper_250 rt 1190: a bounce onto a Ceramic popped
                // that frame goes to where it was).
                pr.instant_pos = tpos;
            }
        }
        match tpos {
            Some(t) => {
                let (dx, dy) = (t[0] - q[0], t[1] - q[1]);
                let m = (dx * dx + dy * dy).sqrt();
                if m > 1e-5 {
                    let (nx, ny) = (dx / m, dy / m);
                    let sf = pr.spec.speed * FRAME_TIME;
                    pr.vel = [nx * sf, ny * sf];
                    pr.grot = Some(RAD2DEG * crt_atan2(nx, -ny));
                }
            }
            None if rt.expire_if_none => pr.frames_left = 0.0,
            None => {}
        }
    }

    /// `bloons.swap_remove(i)`, keeping the broad-phase grid in step.
    /// Exchanges bloons `i` and `j` in storage (and the grids that index it).
    fn swap_bloons(&mut self, i: usize, j: usize) {
        if self.grid_on {
            self.grid.swap(i, j);
            if self.has_grid_rank {
                self.ggrid.swap(i, j);
            }
        }
        self.bloons.swap(i, j);
    }

    fn remove_bloon(&mut self, i: usize) {
        self.fact_kill(i);
        self.plist_kill(self.bloons[i].id);
        if self.grid_on {
            self.grid.swap_remove(i, self.bloons.len());
            if self.has_grid_rank {
                self.ggrid.swap_remove(i, self.bloons.len());
            }
        }
        self.bloons.swap_remove(i);
    }

    /// Uniform in [0, 1).
    /// A uniform 0..1 draw from the sim's own generator.
    /// Simulation.syncedRandom.NextDouble().
    fn synced_double(&mut self) -> f64 {
        let seed = self.freeplay_seed;
        let d = self.synced.get_or_insert_with(|| BufferedRandom::new(seed)).next_double();
        // RRBTD6_RNG_LOG: every draw with its round time, to line up with a recording's R lines.
        if std::env::var_os("RRBTD6_RNG_LOG").is_some() {
            eprintln!("RNG {} {d:?}", self.round_clock);
        }
        d
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Queues one of `names` (the game's sound models list variants and picks one).
    fn sound(&mut self, names: &[String], group: Option<(String, u32, u32, u32)>) {
        if names.is_empty() || !self.sound_room(&names[0], group.as_ref().map(|g| (g.0.as_str(), g.1))) {
            return;
        }
        self.sfx_rng ^= self.sfx_rng << 13;
        self.sfx_rng ^= self.sfx_rng >> 7;
        self.sfx_rng ^= self.sfx_rng << 17;
        let k = (self.sfx_rng >> 33) as usize % names.len();
        self.sfx.push(Sfx { name: names[k].clone(), group });
    }

    /// ComancheDefence.Process: when a bloon gets far along the track, `count` mini Comanches
    /// fly in from the map edge nearest it for `duration` frames, at most `per_round` times a
    /// round and `cooldown` frames apart; Comanche Commander's (immediate) are always there.
    /// HYPOTHESIS P-COMANCHE: "far along" is past 75% of its track; they spawn at the edge
    /// nearest that bloon and hunt the lead bloon (Pursuit).
    fn comanche_defence(&mut self) {
        for t in 0..self.towers.len() {
            let Some(cm) = self.tier(t).and_then(|tr| tr.comanche.clone()) else { continue };
            let id = self.towers[t].id;
            let alive = self.towers.iter().filter(|o| o.sub.as_ref().is_some_and(|s| s.parent == id && s.spec.name == cm.tower.name)).count();
            let tw = &mut self.towers[t];
            tw.comanche.0 = (tw.comanche.0 - 1.0).max(0.0);
            let lead = self
                .bloons
                .iter()
                .filter(|b| on_map(b.pos, 0.0))
                .find(|b| b.dist >= 0.75 * self.tracks[b.path as usize].length())
                .map(|b| b.pos);
            let want = if cm.immediate {
                (alive < cm.count as usize).then(|| self.towers[t].air.as_ref().map_or(self.towers[t].pos, |a| a.pos))
            } else if alive == 0 && tw.comanche.0 <= 0.0 && tw.comanche.1 < cm.per_round {
                lead
            } else {
                None
            };
            let Some(at) = want else { continue };
            if !cm.immediate {
                let tw = &mut self.towers[t];
                tw.comanche = (cm.cooldown, tw.comanche.1 + 1);
            }
            let (lo, hi) = self.bounds;
            let edge = [
                [lo[0], at[1]],
                [hi[0], at[1]],
                [at[0], lo[1]],
                [at[0], hi[1]],
            ]
            .into_iter()
            .min_by(|a, b| ((a[0] - at[0]).abs() + (a[1] - at[1]).abs()).total_cmp(&((b[0] - at[0]).abs() + (b[1] - at[1]).abs())))
            .unwrap();
            let from = if cm.immediate { at } else { edge };
            let n = if cm.immediate { cm.count as usize - alive } else { cm.count as usize };
            for k in 0..n {
                let before = self.towers.len();
                self.spawn_sub(cm.tower.clone(), [from[0] + k as f32 * 8.0, from[1]], id, false);
                if self.towers.len() > before {
                    let s = self.towers.len() - 1;
                    self.towers[s].target = TargetMode::Pursuit;
                    if let Some(sub) = self.towers[s].sub.as_mut() {
                        sub.frames_left = if cm.immediate { f32::INFINITY } else { cm.duration };
                    }
                }
            }
        }
    }

    /// NecromancerZone.OnBloonDegrade (VERIFIED-DISASM @0xd4c220): a Damage that degraded `n`
    /// layers (DamageResult.numOfDegrades) of a bloon at `p` gives them to a Necromancer. The
    /// tier 5 one (towerNecroTier5) first, when it isn't full and the bloon is within range;
    /// whatever didn't fit then goes to one of the others not full and within range, picked by
    /// syncedRandom.Next(count - 1) (the game's own off-by-one: the last is never picked, and
    /// with one the draw still happens), each given all `n` again.
    /// The range is the zone's attack range (attackUsedForRangeModel), the same for every tower
    /// it checks. HYPOTHESIS P-NECROZONE: the zone raising it is the first Necromancer in tower
    /// order with the bloon in range (BloonEnter's subscription), bloonIdLastChecked keeping
    /// it to one zone.
    fn necro_gather(&mut self, n: i64, p: [f32; 2]) {
        #[cfg(test)] let _gather_timer=sim_profile::Timer::new(3);
        if n < 1 || !self.towers.iter().any(|tw| tw.tiers[2] >= 4) {
            return;
        }
        let necro = |g: &Self, t: usize| g.tier(t).and_then(|tr| tr.attacks.iter().find_map(|a| a.necro.as_ref().map(|nc| (nc.max_stored, a.range, g.towers[t].tiers[2] >= 5))));
        let near = |g: &Self, t: usize, r: f32| {
            let q = g.towers[t].pos;
            (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) <= r * r
        };
        let Some(range) = (0..self.towers.len()).find_map(|t| necro(self, t).filter(|&(_, r, _)| near(self, t, r)).map(|x| x.1)) else { return };
        let stored = |g: &Self, t: usize| g.towers[t].grave.iter().map(|x| x.1).sum::<f32>();
        // GraveyardManager.towerNecroTier5: the first tier 5 one.
        let t5 = (0..self.towers.len()).find(|&t| necro(self, t).is_some_and(|x| x.2));
        let mut left = n;
        if let Some(t) = t5 {
            let max = necro(self, t).map_or(0.0, |x| x.0);
            if stored(self, t) < max && near(self, t, range) {
                left = self.necro_assign(t, n);
            }
        }
        if left < 1 {
            return;
        }
        let picks: Vec<usize> = (0..self.towers.len())
            .filter(|&t| Some(t) != t5)
            .filter(|&t| necro(self, t).is_some_and(|x| stored(self, t) < x.0) && near(self, t, range))
            .collect();
        if picks.is_empty() {
            return;
        }
        // BufferedRandom.Next(max) (@0x2dc6b70): (int)(NextDouble x max), a draw even for 0.
        let k = (self.synced_double() * (picks.len() - 1) as f64) as usize;
        self.necro_assign(picks[k], n);
    }

    /// NecromancerZone's Zone (in the tower phase, before the collisions): a bloon whose centre
    /// comes within a necromancer's range enters (BloonEnter subscribes to its degrades), one
    /// that leaves exits (unsubscribes).
    fn grave_zones(&mut self) {
        let zs: Vec<([f32; 2], f32)> = (0..self.towers.len())
            .filter_map(|t| self.tier(t).and_then(|tr| tr.attacks.iter().find(|a| a.necro.is_some())).map(|a| (self.towers[t].pos, a.range)))
            .collect();
        if zs.is_empty() {
            return;
        }
        for b in &mut self.bloons {
            let inside = zs.iter().any(|&(c, r)| (b.pos[0] - c[0]).powi(2) + (b.pos[1] - c[1]).powi(2) <= r * r);
            if inside && !b.grave_zone.0 {
                b.grave_zone = (true, true);
            } else if !inside {
                b.grave_zone = (false, false);
            }
        }
    }

    /// NecromancerZone.AssignRbeToTower (VERIFIED-DISASM @0xd4bc70): `n` RBE (past round 80,
    /// syncedRandom.Next(7, 11) for each instead), as much as fits under maxRbeStored, into the
    /// graveyard's current round; returns what didn't fit.
    fn necro_assign(&mut self, t: usize, n: i64) -> i64 {
        let rbe = if self.round > 79 { (0..n).map(|_| 7 + (self.synced_double() * 4.0) as i64).sum() } else { n };
        let Some(max) = self.tier(t).and_then(|tr| tr.attacks.iter().find_map(|a| a.necro.as_ref().map(|nc| nc.max_stored))) else { return 0 };
        let round = self.round;
        let tw = &mut self.towers[t];
        let pool = tw.grave.iter().map(|g| g.1).sum::<f32>() as i64;
        let left = (pool - max as i64 + rbe).max(0);
        let add = (rbe - left) as f32;
        if add != 0.0 {
            match tw.grave.last_mut() {
                Some(g) if g.0 == round => g.1 += add,
                _ => tw.grave.push((round, add)),
            }
        }
        self.necro_rate_update(t);
        left
    }

    /// Round end: graveyard RBE older than roundsBeforeDecay rounds is gone.
    fn necro_decay(&mut self) {
        let round = self.round;
        for t in 0..self.towers.len() {
            let Some(rounds) = self.tier(t).and_then(|tr| tr.attacks.iter().find_map(|a| a.necro.as_ref().map(|n| n.decay_rounds))) else { continue };
            self.towers[t].grave.retain(|g| round < g.0 + rounds as usize);
            self.necro_rate_update(t);
        }
    }

    /// NecromancerEmission: between min and max undead bloons per wave, each with pierce up to
    /// maxPiercePerBloon taken from the graveyard (at most maxRbeSpawnedPerSecond a second),
    /// walking back up the track from near the target. Stacks: +1 damage per damageRbePerStack
    /// stored (up to damageStackMax) and a 10% faster attack per rateRbePerStack (up to
    /// rateStackMax). Prince of Darkness sends its alternate projectile when the graveyard is
    /// empty, and every undead gets PrinceOfDarknessZombieBuff.
    /// HYPOTHESIS P-NECRO: the stack effects, the spawn point (the target's track spot plus up
    /// to maxPathRandomRange ahead) and the per-second budget are read from the model's fields,
    /// not transcribed.
    /// NecromancerEmission.CheckAfterChangingStoredRbe (VERIFIED-DISASM @0xc46df0), whenever the
    /// graveyard changes: stacks = pool / rateRbePerStack (up to rateStackMax); when that
    /// changes the emission's weapon takes rateMutators[stacks - 1] (none at 0), and its
    /// rateFrames follow at once (Weapon.UpdatedModel), the running cooldown with them.
    /// HYPOTHESIS P-NECRORATE: rateMutators[k - 1] multiplies the rate by 1 - 0.1 k (recording
    /// wizard_005: 90, 81, 72, 63, 54 and 45 frames; the Prince's weapon stays at 180).
    /// How high tower `t` is: PathMovementFromScreenCenter flies its tower at 250 (the target z
    /// of its Vector3.MoveTowards, VERIFIED-DISASM @0xd578d0), so its shots start 250 up and
    /// clear the map's blockers (recording wizard_050: Phoenix shots at z 257 fly past the
    /// entrance trees). HYPOTHESIS P-PHOENIXZ: it is at 250 from the start.
    fn tower_height(&self, t: usize) -> f32 {
        if self.towers[t].sub.as_ref().is_some_and(|s| s.spec.circle.is_some()) { 250.0 } else { 0.0 }
    }

    /// Bloon.PercThroughMap (VERIFIED-DISASM @0x180fb54f0): (distanceTraveled + MaxDistUntilSpawn) /
    /// (MaxDistUntilSpawn + MaxDistUntilLeak).
    fn perc_through_map(&self, b: &Bloon) -> f32 {
        let total = self.tracks[b.path as usize].length();
        let (sd, ld) = self.track_ends.get(b.path as usize).map_or((None, None), |e| *e);
        let (sd, ld) = (sd.unwrap_or(0.0), ld.unwrap_or(total));
        let until_spawn = (total - sd) - total;
        (b.dist + until_spawn) / (until_spawn + ld)
    }

    /// LeakDangerAttackSpeed's danger as the towers see it (after this frame's collisions).
    fn compute_danger(&mut self) {
        // LeakDangerAttackSpeed.GetRate (VERIFIED-DISASM @0xc7bd00): rate / (PercThroughMap x
        // leakDanger + 1), PercThroughMap (@0xfb54f0) = (distanceTraveled + MaxDistUntilSpawn) /
        // (MaxDistUntilSpawn + MaxDistUntilLeak) = (d - spawnDist) / (leakDist - spawnDist)
        // without routers. HYPOTHESIS P-LEAKDANGER: of the furthest-along bloon (the game takes
        // the first of a map list).
        self.danger = self
            .bloons
            .iter()
            .max_by(|x, y| x.progress().total_cmp(&y.progress()))
            .map_or(0.0, |b| {
                let total = self.tracks[b.path as usize].length();
                let (sd, ld) = self.track_ends.get(b.path as usize).map_or((None, None), |e| *e);
                let (sd, ld) = (sd.unwrap_or(0.0), ld.unwrap_or(total));
                // Path.MaxDistUntilSpawn: (totalPathLength - spawnDist) - totalPathLength in f32.
                let until_spawn = (total - sd) - total;
                (b.dist + until_spawn) / (until_spawn + ld)
            });
    }

    /// Where a bloon is as a target of an attack at `pos` (Targetting.GetTargets): on a segment in
    /// range + 2 whose line is clear, itself; a MOAB-class one otherwise by the first of its
    /// collision circles that's in range and seen (Targetting.CanHitMoab, as towers_phase); off
    /// the track, within range + radius. With the squared distance of its segment's point.
    fn visible_spot(&self, b: &Bloon, pos: [f32; 2], range: f32, camo: bool, no_moab: bool, attack: &crate::data::Attack) -> Option<(f32, [f32; 2])> {
        // AttackMinimumRange.FilterTarget (VERIFIED-DISASM @0x180dac2f0), in the attack's filter the
        // visible-target queries use: minimumRange^2 <= the bloon's 2D distance^2 from the tower
        // (recording desperado_250 rt 437: the rifle's distance / standoff bonuses skip bloons within
        // 40, 80 frames not 125).
        if attack.min_range > 0.0 && (b.pos[0] - pos[0]).powi(2) + (b.pos[1] - pos[1]).powi(2) < attack.min_range * attack.min_range {
            return None;
        }
        let d = &self.defs[b.kind];
        if (b.camo(d) && !camo) || (no_moab && d.tags.iter().any(|t| t == "Moabs")) {
            return None;
        }
        if !attack.require_tags.iter().all(|t| d.tags.contains(t)) || attack.exclude_tags.iter().any(|t| d.tags.contains(t)) {
            return None;
        }
        if b.wind.is_some() || b.held {
            let rr = range + d.radius;
            return ((b.pos[0] - pos[0]).powi(2) + (b.pos[1] - pos[1]).powi(2) <= rr * rr).then_some((f32::MAX, b.pos));
        }
        let k = (b.wind_seg.unwrap_or(b.prev_dist) / 7.0).max(0.0) as usize;
        if !self.track_seg_ok.get(b.path as usize).and_then(|v| v.get(k)).copied().unwrap_or(false) {
            return None;
        }
        let q = *self.track_segs.get(b.path as usize)?.get(k)?;
        let dd = (q[0] - pos[0]).powi(2) + (q[1] - pos[1]).powi(2);
        let reach = if no_moab { range } else { range + 2.0 };
        let clear = attack.through_walls || self.line_clear(pos, 0.0, q);
        if dd <= reach * reach && clear {
            return Some((dd, b.pos));
        }
        if d.circles.is_empty() || dd > (range + 40.0) * (range + 40.0) {
            return None;
        }
        d.circles.iter().find_map(|c| {
            let (sa, ca) = crt_sin_cos(DEG2RAD * -b.rot);
            let (ox, oy) = (ca * c[0] - sa * c[1], ca * c[1] + sa * c[0]);
            let p = [b.pos[0] + ox, b.pos[1] - oy];
            ((p[0] - pos[0]).powi(2) + (p[1] - pos[1]).powi(2) < (range + c[2]) * (range + c[2]) && (clear || self.line_clear(pos, 0.0, p))).then_some((dd, p))
        })
    }

    /// How many Attack.GetVisibleTargets has.
    fn visible_count(&self, pos: [f32; 2], range: f32, camo: bool, no_moab: bool, attack: &crate::data::Attack) -> usize {
        self.bloons.iter().filter(|b| self.visible_spot(b, pos, range, camo, no_moab, attack).is_some()).count()
    }

    /// Attack.GetVisibleTarget with Close: of the segments in range the one whose point is nearest
    /// the tower (BloonManager.CompareBracketClose), its first bloon in the path's list; those off
    /// the track after them.
    fn visible_close(&self, pos: [f32; 2], range: f32, camo: bool, no_moab: bool, attack: &crate::data::Attack) -> Option<[f32; 2]> {
        let mut best: Option<((f32, u32), [f32; 2])> = None;
        for b in &self.bloons {
            let Some((dd, p)) = self.visible_spot(b, pos, range, camo, no_moab, attack) else { continue };
            let key = (dd, self.path_rank.get(&b.id).map_or(u32::MAX, |r| r.1));
            if best.is_none_or(|(o, _)| key < o) {
                best = Some((key, p));
            }
        }
        best.map(|b| b.1)
    }

    /// Whether the emission weapon has a rate stack mutator on (necro_rate_update).
    fn necro_stacked(&self, t: usize) -> bool {
        let Some(tier) = self.tier(t) else { return false };
        let mut wi = 0;
        for a in &tier.attacks {
            for w in &a.weapons {
                if a.necro.is_some() && !w.prince {
                    return self.towers[t].weapons.get(wi).is_some_and(|st| st.necro != 1.0);
                }
                wi += 1;
            }
        }
        false
    }

    fn necro_rate_update(&mut self, t: usize) {
        let Some(tier) = self.tier(t) else { return };
        let mut slots = Vec::new();
        let mut wi = 0;
        for a in &tier.attacks {
            for w in &a.weapons {
                if let (Some(nc), false) = (&a.necro, w.prince) {
                    slots.push((wi, w.clone(), nc.rate_rbe_per_stack.max(1.0) as i64, nc.rate_stack_max as i64));
                }
                wi += 1;
            }
        }
        let pool = self.towers[t].grave.iter().map(|g| g.1).sum::<f32>() as i64;
        // currentDamageBonus = min(pool / damageRbePerStack, damageStackMax), on at once.
        let dmg = tier.attacks.iter().find_map(|a| a.necro.as_ref()).map(|nc| (nc.damage_rbe_per_stack.max(1.0) as i64, nc.damage_stack_max as i64));
        if let Some((per, max)) = dmg {
            let k = (pool / per).min(max) as f32;
            let old = std::mem::replace(&mut self.towers[t].necro_dmg, k);
            if let Some(b) = self.buffs.get_mut(t) {
                b.damage_add += k - old;
            }
        }
        for (wi, weapon, per, max) in slots {
            let k = (pool / per).min(max);
            let Some(st) = self.towers[t].weapons.get_mut(wi) else { continue };
            let m = 1.0 - 0.1 * k as f32;
            if st.necro != m {
                st.necro = m;
                let r = st.rate(&weapon);
                st.rate_frames = rate_frames(r);
            }
        }
    }

    /// NecromancerEmission.Emit (@0xc66360): nothing below minBloonsSpawnedPerWave RBE stored;
    /// of the RBE stored, at most maxRbeSpawnedPerSecond go this wave, as Random.Range(min, n)
    /// undead bloons (n = that, at most maxBloonsSpawnedPerWave), each pierce 1 plus a
    /// Random.Range(1, min(left, maxPiercePerBloon)) share of what's left over. Each starts on
    /// the target segment's path at its centre less Random.Next(maxPathRandomRange) + half of
    /// it (unless the player selected the segment), drawn bloon by bloon after its pierce,
    /// offset off the track by
    /// Vector3.GetOffsetFromRadius(maxPathOffset) (NextDouble x radius at -NextDouble x 360
    /// degrees) with y - 4 (and z + 20, its display height), and walks back up the track
    /// (TravelAlongPath with that offset). The RBE it took leaves the graveyard, oldest first.
    /// Prince of Darkness sends its alternate projectile when the graveyard has too little.
    fn necro_fire(&mut self, t: usize, nc: &crate::data::Necro, proj: &Proj, seg: Option<(u8, usize)>, tiers: [u8; 3], prince: bool) {
        let stored: f32 = self.towers[t].grave.iter().map(|g| g.1).sum();
        // NecromancerEmissionModel.DamageMutator (VERIFIED-DISASM @0x123aae0): + currentDamageBonus
        // on every DamageModel in the tower (recording wizard_005: the main bolt does 2 with 300
        // RBE stored); the undead and the ghost, emitted from the bare weapon here, get it too.
        let dmg_stacks = self.towers[t].necro_dmg;
        let (dmg_add, dist_mult) = self
            .towers
            .iter()
            .filter_map(|o| self.tier_of(o).and_then(|tr| tr.zombie_buff))
            .fold((0.0f32, 1.0f32), |(d, m), z| (d.max(z[0]), m.max(z[1])));
        // HYPOTHESIS P-NECRORATEBUFF: the rate stack mutator (CheckAfterChangingStoredRbe) remakes
        // the emission weapon's TravelAlongPath without the zombie buff's lifespan; its damage
        // stays (recording wizard_005: undead live 630 frames up to the first stack at rt 2473,
        // 420 after, and still pop a Green whole at rt 2805; the ghost's weapon keeps 1350).
        let stacked = !prince && self.necro_stacked(t);
        let dist_mult = if stacked { 1.0 } else { dist_mult };
        // The emission keeps the last target's path and segment (a target without one leaves
        // them as they were).
        if seg.is_some() {
            self.towers[t].necro_emit = seg;
        }
        let Some((path, k)) = self.towers[t].necro_emit else { return };
        let Some((centre, _)) = self.seg_span(path as usize, k) else { return };
        let selected = self.towers[t].necro_sel.is_some();
        // SetUpAttack: currentPathIndex = pathIndexToSet.
        self.towers[t].necro_cur = self.towers[t].necro_next;
        // Simulation.syncedRandom (NecromancerEmission.Emit, VERIFIED-DISASM @0xc66360):
        // BufferedRandom.Next(lo, hi) (@0x2dc6a30) = (int)(NextDouble x (hi - lo)) + lo, a draw
        // even when hi == lo (recording wizard_004: Next(1, 1) for a 2-pierce undead's share).
        let range = |g: &mut Self, lo: i64, hi: i64| lo + (g.synced_double() * (hi - lo) as f64) as i64;
        let pool = stored.floor() as i64;
        let (min, max_wave) = (nc.min_per_wave as i64, nc.max_per_wave as i64);
        let owner = self.towers[t].id;
        let single = Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None};
        // Each undead bloon in turn: its pierce share, its spot on the track (the segment's
        // centre, less Random.Next(maxPathRandomRange) + half of it unless the player chose the
        // segment), then its offset (Vector3.GetOffsetFromRadius) as it is emitted.
        let spawn = |g: &mut Self, mut sp: Proj, pierce: f32| {
            sp.damage += dmg_add + dmg_stacks;
            let mut d0 = centre;
            if !selected {
                let r = if prince { nc.prince_range } else { nc.path_range }.max(0.0) as i64;
                let back = range(g, 0, r) + r / 2;
                d0 -= back as f32;
            }
            // STAND-IN P-NECROPATH: Path.DistanceToPath (another path past either end) as a clamp
            // to this one.
            let len = g.tracks[path as usize].length();
            let d0 = d0.clamp(0.0, len);
            let p0 = g.tracks[path as usize].pos(d0);
            let start = g.projectiles.len();
            g.emit(p0, 0.0, &single, &sp, None, tiers, owner);
            // Vector3.GetOffsetFromRadius: NextDouble x radius, then NextDouble x 360 degrees.
            let rad = g.synced_double() as f32 * if prince { nc.prince_offset } else { nc.path_offset };
            let ang = -(g.synced_double() as f32 * 360.0).to_radians();
            let off = [rad * ang.cos(), 4.0 - rad * ang.sin()];
            let at = [p0[0] + off[0], p0[1] + off[1]];
            for pr in &mut g.projectiles[start..] {
                pr.pos = at;
                pr.vel = [0.0; 2];
                // NecromancerEmissionModel.GetPierceMutator(share) (@0x12422b0): a mutator adding
                // the share to the projectile's own pierce (recording wizard_004: shares 1 and 2
                // make pierce 2 and 3).
                pr.pierce = sp.pierce + pierce;
                pr.on_track = Some((path, d0));
                pr.track_off = off;
                // PrinceOfDarknessZombieBuffMutator.Mutate (VERIFIED-DISASM @0xd7a0d0):
                // TravelAlongPathModel.lifespanFrames = (int)(frames x distanceMultiplier).
                if let Some(tt) = &sp.track_travel {
                    let frames = (tt.lifespan * FPS).round();
                    pr.frames_left = (frames * dist_mult) as i32 as f32;
                }
            }
        };
        if prince {
            // PrinceOfDarknessEmission.Emit (VERIFIED-DISASM @0xc493b0): its own projectile, the
            // alternateProjectile once the pool reaches 0.7 x maxRbeStored; the projectile's
            // pierce leaves the graveyard, oldest rounds first.
            let sp = match &nc.prince {
                Some(alt) if stored >= nc.max_stored * 0.7 => (**alt).clone(),
                _ => proj.clone(),
            };
            let mut spent = sp.pierce;
            spawn(self, sp.clone(), 0.0);
            let tw = &mut self.towers[t];
            for g in tw.grave.iter_mut() {
                let k = g.1.min(spent);
                g.1 -= k;
                spent -= k;
            }
            tw.grave.retain(|g| g.1 > 0.0);
            self.necro_rate_update(t);
            return;
        }
        if pool >= min && pool > 0 {
            let lim = pool.min(nc.rbe_per_second as i64);
            let mut count = lim.min(max_wave);
            if min < count {
                count = range(self, min, count);
            }
            let mut extra = (lim - count).max(0);
            let mut spent = 0.0f32;
            for _ in 0..count.max(0) {
                let mut pierce = 1;
                if extra > 0 {
                    let k = range(self, 1, extra.min(nc.max_pierce as i64));
                    extra -= k;
                    pierce = k + 1;
                }
                spawn(self, proj.clone(), pierce as f32);
                spent += pierce as f32;
            }
            // The RBE taken leaves the graveyard, oldest rounds first.
            let tw = &mut self.towers[t];
            for g in tw.grave.iter_mut() {
                let k = g.1.min(spent);
                g.1 -= k;
                spent -= k;
            }
            tw.grave.retain(|g| g.1 > 0.0);
            self.necro_rate_update(t);
        }
    }

    /// PathSegment.distanceUntilLeak: the track's length less the segment's centre.
    /// HYPOTHESIS P-SEGLEAK: measured from the centre (Path.Init isn't transcribed).
    fn seg_until_leak(&self, pi: u8, k: usize) -> f32 {
        let len = self.tracks.get(pi as usize).map_or(0.0, |t| t.length());
        self.seg_span(pi as usize, k).map_or(f32::MAX, |(centre, _)| len - centre)
    }

    /// PathSegment.centre and max of segment `k` of track `pi` (Path.Init: MaxSegmentSize 7
    /// pieces from the start, the last one shorter).
    fn seg_span(&self, pi: usize, k: usize) -> Option<(f32, f32)> {
        let len = self.tracks.get(pi)?.length();
        let full = (len / 7.0) as usize;
        if k < full {
            Some((k as f32 * 7.0 + 3.5, (k + 1) as f32 * 7.0))
        } else if k == full && (len / 7.0).ceil() as usize > full {
            let min = full as f32 * 7.0;
            Some((min + (len - min) * 0.5, len))
        } else {
            None
        }
    }

    /// Attack.GetVisibleInRangePathSegments (VERIFIED-DISASM @0xdde7b0, InitTargetPathSegments
    /// @0xdcda00): the targetable segments (canTargetSegment, point on the map) whose point is
    /// within range (+ 2 unless the attack filters MOABs out) with a clear line from the tower,
    /// paths in order, segments ascending. (PathSegment.bloonsInvulnerable isn't imported.)
    fn visible_segments(&self, pos: [f32; 2], range: f32, no_moab: bool, through_walls: bool) -> Vec<(u8, usize)> {
        let reach = if no_moab { range } else { range + 2.0 };
        let mut out = Vec::new();
        for (pi, sg) in self.track_segs.iter().enumerate() {
            for (k, q) in sg.iter().enumerate() {
                let ok = self.track_seg_ok.get(pi).and_then(|v| v.get(k)).copied().unwrap_or(false);
                if ok && on_map(*q, 0.0) && (q[0] - pos[0]).powi(2) + (q[1] - pos[1]).powi(2) <= reach * reach && (through_walls || self.line_clear(pos, 0.0, *q)) {
                    out.push((pi as u8, k));
                }
            }
        }
        out
    }

    /// NecromancerTargetTrackWithinRange.GetTargets (VERIFIED-DISASM @0xdeaad0; GeneratePoints
    /// @0xdea480, FilterBloon @0xdea3c0, lambdas @0xdf96d0..): the first segment of
    /// lastPathSegments (visible_segments) that it keeps.
    /// - The player's selected segment while it is still one of them (CheckSelectedPathSegment
    ///   drops it once it isn't).
    /// - Else, with any bloon on a path in range short of that path's furthest in-range segment
    ///   (BloonManager.GetTarget, range 99999, FilterBloon): from the path after
    ///   currentPathIndex round, the first path with bloons; its furthest-along such bloon (the
    ///   first of equals in Path.bloonsList), and the segments on its path ending past it
    ///   (pathIndexToSet = that path).
    /// - Otherwise the segments on pathsInRange[currentPathIndex] (pathIndexToSet = the next).
    fn necro_target(&mut self, t: usize, pos: [f32; 2], range: f32, no_moab: bool, through_walls: bool) -> Option<(u8, usize)> {
        // The First list: SetVisiblePathSegments (VERIFIED-DISASM @0xdafaa0, b__48_0) orders them
        // by PathSegment.distanceUntilLeak, ascending (a stable OrderBy): the furthest along
        // first (recording wizard_004: the undead start near the far end of the range).
        let mut segs = self.visible_segments(pos, range, no_moab, through_walls);
        segs.sort_by(|a, b| self.seg_until_leak(a.0, a.1).total_cmp(&self.seg_until_leak(b.0, b.1)));
        let mut paths: Vec<u8> = Vec::new();
        let mut max_dist: HashMap<u8, f32> = HashMap::new();
        for &(pi, k) in &segs {
            if !paths.contains(&pi) {
                paths.push(pi);
            }
            let m = self.seg_span(pi as usize, k).map_or(0.0, |s| s.1);
            let e = max_dist.entry(pi).or_insert(0.0);
            if *e < m {
                *e = m;
            }
        }
        if paths.is_empty() {
            return None;
        }
        let n = paths.len();
        if self.towers[t].necro_cur >= n {
            self.towers[t].necro_cur = 0;
        }
        let cur = self.towers[t].necro_cur;
        if let Some(sel) = self.towers[t].necro_sel {
            if segs.contains(&sel) {
                return Some(sel);
            }
            self.towers[t].necro_sel = None;
        }
        let any = self.bloons.iter().any(|b| max_dist.get(&b.path).is_some_and(|&m| m > b.dist));
        let mut lead: Option<(u8, f32)> = None;
        let mut next = (cur + 1) % n;
        if any {
            for i in 0..n {
                let pi = (cur + 1 + i) % n;
                let path = paths[pi];
                if !self.bloons.iter().any(|b| b.path == path) {
                    continue;
                }
                let m = max_dist[&path];
                for b in self.bloons.iter().filter(|b| b.path == path) {
                    if lead.is_some_and(|(_, d)| b.dist <= d) {
                        continue;
                    }
                    if b.dist < m {
                        lead = Some((path, b.dist));
                    }
                }
                if lead.is_some() {
                    next = pi;
                }
                break;
            }
        }
        self.towers[t].necro_next = next;
        match lead {
            Some((path, d)) => segs.into_iter().find(|&(pi, k)| pi == path && self.seg_span(pi as usize, k).is_some_and(|s| s.1 > d)),
            None => segs.into_iter().find(|&(pi, _)| pi == paths[cur]),
        }
    }

    /// ApplyTargetTypeData (VERIFIED-DISASM @0xde9cd0): the Necromancer's spawn segment is the
    /// in-range one whose point is nearest `at` (PathSegment.DistanceSqrdFrom; the first of
    /// equals); none when no segment is in range.
    pub fn necro_select(&mut self, t: usize, at: [f32; 2]) {
        let Some(attack) = self.tier(t).and_then(|tr| tr.attacks.iter().find(|a| a.necro.is_some())).cloned() else { return };
        let range = self.range(t).max(attack.range);
        let pos = self.attack_pos(t);
        let no_moab = attack.exclude_tags.iter().any(|x| x == "Moabs");
        let mut best: Option<((u8, usize), f32)> = None;
        for (pi, k) in self.visible_segments(pos, range, no_moab, attack.through_walls) {
            let q = self.track_segs[pi as usize][k];
            let d = (q[0] - at[0]).powi(2) + (q[1] - at[1]).powi(2);
            if best.is_none_or(|(_, b)| d < b) {
                best = Some(((pi, k), d));
            }
        }
        self.towers[t].necro_sel = best.map(|x| x.0);
    }

    /// NecromancerTargetTrackWithinRange.Process: while selected, its target image sits on the
    /// player's chosen segment.
    pub fn necro_marker(&self, t: usize) -> Option<[f32; 2]> {
        let (pi, k) = self.towers[t].necro_sel?;
        self.track_segs.get(pi as usize)?.get(k).copied()
    }

    /// The tower has a Necromancer attack (its panel's set-spawn button).
    pub fn is_necro(&self, t: usize) -> bool {
        self.tier(t).is_some_and(|tr| tr.attacks.iter().any(|a| a.necro.is_some()))
    }

    /// One of `names` at random (an empty slot plays nothing: CreateSoundOnUpgrade picks
    /// Random.Range(0, 8) over its eight), in voice group `group` (one at a time, `gap` frames
    /// apart). Uses the sound RNG, not the simulation's.
    fn voice(&mut self, names: &[String], group: &str, gap: u32) {
        if names.is_empty() {
            return;
        }
        self.sfx_rng ^= self.sfx_rng << 13;
        self.sfx_rng ^= self.sfx_rng >> 7;
        self.sfx_rng ^= self.sfx_rng << 17;
        let name = &names[(self.sfx_rng >> 33) as usize % names.len()];
        if !name.is_empty() && self.sound_room(name, Some((group, 1))) {
            self.sfx.push(Sfx { name: name.clone(), group: Some((group.to_string(), 1, gap, gap)) });
        }
    }

    /// Bloon.PlayImmuneSound for a hit that can't damage: GameData.bloonImmuneSounds by bloon
    /// property, Diamond over Frozen over Lead over Purple (Bloon damage path), each a group of
    /// one with a 5-frame counter.
    fn immune_sound(&mut self, props: u32) {
        let Some((_, group, names)) = IMMUNE_SOUNDS.iter().find(|(bit, _, _)| props & bit != 0) else { return };
        if self.sound_room(names[0], Some((group, 1))) {
            let names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
            self.sound(&names, Some((group.to_string(), 1, 5, 5)));
        }
    }

    /// The placed hero's line from `pick` (its tier's sounds).
    fn hero_voice(&mut self, pick: impl Fn(&crate::data::TowerSounds) -> &Vec<String>, group: &str, gap: u32) {
        let Some(h) = (0..self.towers.len()).find(|&t| self.towers[t].sub.is_none() && self.hero_def(self.towers[t].def).is_some()) else { return };
        let s = self.tier_sounds(h);
        self.voice(pick(&s), group, gap);
    }

    /// CreateSoundOnBloonEnterTrack: the hero's warning when a MOAB-class bloon of a kind not
    /// yet seen this round comes on.
    fn enter_voice(&mut self, kind: usize) {
        let Some(class) = self.defs[kind].tags.iter().map(|t| t.to_lowercase()).find(|t| ["moab", "bfb", "zomg", "ddt", "bad"].contains(&t.as_str())) else { return };
        if self.voice_warned.contains(&class) {
            return;
        }
        let Some(h) = (0..self.towers.len()).find(|&t| self.towers[t].sub.is_none() && self.hero_def(self.towers[t].def).is_some()) else { return };
        if let Some(line) = self.tier_sounds(h).enter.get(&class).cloned() {
            self.voice_warned.push(class);
            self.voice(&[line], VOICE, VOICE_GAP);
        }
    }

    /// CreateSoundOnSelected: the hero's line when the player selects it.
    pub fn select_voice(&mut self, t: usize) {
        if t < self.towers.len() && self.hero_def(self.towers[t].def).is_some() {
            let s = self.tier_sounds(t);
            self.voice(&s.select, VOICE, VOICE_GAP);
        }
    }

    fn tier_sounds(&self, t: usize) -> crate::data::TowerSounds {
        self.tier(t).map(|tr| tr.sounds.clone()).unwrap_or_default()
    }

    /// Projectile.CollideBloon's behaviours before the DamageModel's pass:
    /// RemoveMutatorsFromBloon (-50), then the effects that go on before the damage
    /// (AddBehaviorToBloon -1, slows...). They run whether or not the damage can hurt the bloon
    /// (recording ice_004: a blast on a Zebra still draws its Icicles chance).
    fn pre_damage_effects(&mut self, i: usize, p: usize, dmg: f32) {
        let spec = &self.projectiles[p].spec;
        // HYPOTHESIS P-REMOVEMUT: a key removes effects whose mutator id contains it;
        // "Ice" also covers freezes and "Glue" glue slows whatever their id.
        if !spec.remove_effects.is_empty() {
            let keys = spec.remove_effects.clone();
            let mut gone_carried: Vec<u64> = Vec::new();
            self.bloons[i].effects.retain(|e| {
                let keep = !keys.iter().any(|k| {
                    k.strip_prefix('=').map_or_else(|| e.id.contains(k.as_str()), |x| e.id == x)
                        || (k == "Ice" && matches!(e.kind, EffectKind::Freeze))
                        || (k == "Glue" && matches!(e.kind, EffectKind::Slow { glue: 1.., .. }))
                });
                if !keep {
                    if let EffectKind::Carry { uids, .. } = &e.kind {
                        gone_carried.extend(uids.iter().copied());
                    }
                }
                keep
            });
            // Mutable.RemoveMutatorsById (VERIFIED-DISASM) applies the mutation at
            // once: CarryProjectile.OnDestroy takes its projectiles with it before they
            // collide again this pass (recording ice_025: a Yellow whose Icicles a
            // blast took isn't popped by its own icicle).
            if !gone_carried.is_empty() {
                for pr in self.projectiles.iter_mut().filter(|pr| gone_carried.contains(&pr.uid)) {
                    pr.pierce = 0.0;
                    pr.frames_left = 0.0;
                }
            }
        }
        if !self.projectiles[p].spec.effects.is_empty() {
            let (mut spec, owner) = (self.projectiles[p].spec.clone(), self.projectiles[p].owner);
            // SlowMinusAbilityDuration.GetLifespanFrames (VERIFIED-DISASM @0xcddc50):
            // lifespanFrames + activation time - now; the projectile is made as the
            // ability goes off (recording ninja_040: Sabotage halves a MOAB's speed).
            let age = self.projectiles[p].age;
            for e in spec.effects.iter_mut().filter(|e| e.minus_age) {
                e.lifespan = (e.lifespan - age / FPS).max(0.0);
            }
            self.apply_effects(i, &spec, owner, dmg, false);
        }
    }

    /// Put projectile `spec`'s effects on bloon `i` (before its damage, so children of a popped
    /// bloon inherit them through `layers`).
    /// `after`: the effects that go on after the damage (FreezeModel.applyAfterDamage), else the rest.
    fn apply_effects(&mut self, i: usize, spec: &Proj, owner: u64, dmg: f32, after: bool) {
        let d = &self.defs[self.bloons[i].kind];
        let tags = d.tags.clone();
        let has = |t: &str| tags.iter().any(|x| x == t);
        let props = self.bloons[i].properties(d);
        // AddBehaviorToBloon.Collide (VERIFIED-DISASM @0xc9fea0), applyOnlyIfDamaged: Bloon
        // .WillDamageBloon (not immune, damage >= 1) for its parentDamageModel, then for the
        // projectile's own DamageModel if it has one (recording boomerang_500: MoabDot, with
        // none, puts its DoT on a MOAB). INFERRED: the parent's damage unmodified.
        let wont = |e: &crate::data::EffectSpec| {
            e.only_if_damaged
                && (e.parent.as_ref().is_some_and(|pd| pd.immune & props != 0 || pd.damage < 1.0) || (spec.damage_model && dmg < 1.0))
        };
        let mut add = Vec::new();
        for e in spec.effects.iter().filter(|e| e.after_damage == after) {
            if (!e.tags.is_empty() && !e.tags.iter().any(|t| t.strip_prefix('!').map_or_else(|| has(t), |x| !has(x)))) || wont(e) {
                continue;
            }
            if !e.bloon_id.is_empty() {
                let name = self.names[self.bloons[i].kind].as_str();
                let is = if e.bloon_ids.len() > 1 { e.bloon_ids.iter().any(|x| x.contains(name)) } else { name.contains(e.bloon_id.as_str()) };
                if is == e.excluding {
                    continue;
                }
            }
            // Slow.Collide (VERIFIED-DISASM @0x180cde360): a slow whose mutation id contains
            // "Permafrost" isn't put on a bloon immune to the projectile's DamageModel
            // (Bloon.IsImmuneByProperty; recording ice_300: Whites under Arctic Wind's blast).
            if e.kind == "slow" && e.id.contains("Permafrost") && spec.damage_model && spec.immune & props != 0 {
                continue;
            }
            // Freeze.Collide (VERIFIED-DISASM @0xcdcc60): bossImmunity spares Bads and Minibosses;
            // without a chance it freezes only a bloon the projectile's DamageModel isn't
            // immune to (Bloon.IsImmuneByProperty; recording engineer_400: a Cold Sentry's
            // blast leaves a Zebra moving).
            if (e.kind == "freeze" && e.chance <= 0.0 && spec.damage_model && spec.immune & props != 0)
                || (e.kind == "freeze" && ((!e.moabs && has("Moabs")) || (e.boss_immunity && (has("Bad") || has("Miniboss")))))
                || (e.moabs_only && !has("Moabs"))
            {
                continue;
            }
            // AddBehaviorToBloon.Collide (VERIFIED-DISASM @0x180c81390): after its filters and
            // applyOnlyIfDamaged, a chance > 0 draws Simulation.syncedRandom.NextDouble and puts
            // nothing on when chance < the draw (recording ice_005: Icicle Impale's 25% Icicles
            // left the MOAB it hit at 81 without one).
            // Bloon.WillPopBloon (VERIFIED-DISASM @0x180fb8150): it will damage the bloon and
            // health - the modified damage < 1 (recording engineer_002: Pin stops the Pink a nail
            // pops a Black into).
            if e.only_if_pop && !(spec.damage_model && spec.immune & props == 0 && (self.bloons[i].hp as f32) - dmg < 1.0) {
                continue;
            }
            if e.chance > 0.0 && (e.chance as f64) < self.synced_double() {
                continue;
            }
            let (mut mult, mut lifespan) = (e.mult, e.lifespan);
            if !e.by_class.is_empty() {
                lifespan = moab_class(&self.names[self.bloons[i].kind]).and_then(|k| e.by_class.get(k)).copied().unwrap_or(0.0);
                if lifespan <= 0.0 {
                    continue;
                }
            }
            if e.kind == "freeze" {
                // Freeze.Collide (VERIFIED-DISASM @0xcdcc60): the first FreezeModifierForTags
                // whose freezeId the mutation id contains, with a tag the bloon has: prevent, or
                // the model's lifespan (resetToUnmodified), or it x freezeTimeMultiplier.
                if let Some(m) = spec.effect_mods.iter().find(|m| m.freeze && e.id.contains(m.id.as_str()) && m.tags.iter().any(|t| has(t))) {
                    if m.prevent {
                        continue;
                    }
                    if !m.reset {
                        lifespan *= m.time_mult;
                    }
                }
            } else if e.kind == "slow" {
                let mods = spec.effect_mods.iter().filter(|m| !m.freeze);
                // Slow.Collide (VERIFIED-DISASM @0xcfce70): a SlowModifierForTag whose slowId the
                // mutation id contains, for a tag the bloon has, overrides the lifespan (the last).
                for m in mods.clone() {
                    if e.id.contains(m.id.as_str()) && m.tags.iter().any(|t| has(t)) && m.lifespan > 0.0 {
                        lifespan = m.lifespan;
                    }
                }
                // SlowMutator.Mutate (VERIFIED-DISASM @0x121e5a0): the matching ones (the tag, or
                // with makeNotTag its absence) in order: preventMutation or resetToUnmodified and
                // it changes nothing, else the multiplier x slowMultiplier; with modifiers it only
                // slows below 1 (recording ice_500: Super Brittle's Permafrost, 1 x 0.75 on a MOAB,
                // its x 0.5 for the rest).
                let mut any = false;
                let mut affects = true;
                for m in mods {
                    any = true;
                    if e.id.contains(m.id.as_str()) && m.tags.iter().any(|t| has(t)) != m.not_tag {
                        if m.prevent || m.reset {
                            affects = false;
                            break;
                        }
                        mult *= m.mult;
                    }
                }
                // (Not affecting it, the mutator is still on the bloon with its glue level:
                // recording glue_400, a MOAB under Liquefier glue isn't thrown at again.)
                if !affects || (any && mult >= 1.0) {
                    mult = 1.0;
                }
            }
            let kind = match e.kind.as_str() {
                "slow" => EffectKind::Slow { mult, glue: e.glue },
                "freeze" => EffectKind::Freeze,
                // DamageOverTime.Initialise (VERIFIED-DISASM @0x1003e50): lastTick = the clock +
                // initialDelayFrames, or -999999999 with triggerImmediate (recording glue_005: Super
                // Glue's MOAB DoT deals its 30 the frame after the hit).
                "dot" => EffectKind::Dot {
                    damage: e.damage,
                    interval: e.interval * FPS,
                    tick: if e.trigger_immediate { -999999999.0 } else { self.round_clock as f32 + (e.initial_delay * FPS).round() },
                    immune: e.immune,
                    mods: e.tag_mods.clone().into(),
                    start: if e.trigger_immediate { None } else { Some((e.initial_delay * FPS).round()) },
                },
                "brittle" => EffectKind::Brittle { damage: e.damage },
                // HYPOTHESIS P-TRANCE: the bloon leaves its track spot and circles the tower that
                // hit it at orbitRadius, entering where it was hit, at its own speed x
                // speedMultiplier (MOAB-class: moabOrbitSpeed), then goes back to the spot it
                // left. It can't be entranced again for `cooldown` seconds after.
                "trance" => {
                    let b = &self.bloons[i];
                    if b.has_effect(&format!("{}Cooldown", e.id)) || e.immune & b.properties(&self.defs[b.kind]) != 0 {
                        continue;
                    }
                    let Some(c) = self.towers.iter().find(|t| t.id == owner).map(|t| t.pos) else { continue };
                    let f = if has("Moabs") { e.moab_mult } else { e.mult };
                    let r = e.radius.max(1.0);
                    add.push((
                        Effect { overlay: Default::default(), seq: 0, kind: EffectKind::Cooldown, id: format!("{}Cooldown", e.id), frames_left: (lifespan + e.cooldown) * FPS, total: (lifespan + e.cooldown) * FPS, added: self.sim_elapsed, layers: 1, owner, distribute: (true, false), keep: false },
                        false,
                        (false, false, false),
                    ));
                    EffectKind::Trance {
                        centre: c,
                        radius: r,
                        angle: (b.pos[1] - c[1]).atan2(b.pos[0] - c[0]),
                        rate: self.defs[self.bloons[i].kind].speed * f / FPS / r,
                    }
                }
                "emit" => match &e.projectile {
                    Some(p) => EffectKind::Emit { interval: e.interval, tick: 0.0, proj: p.clone() },
                    None => continue,
                },
                "mark" => EffectKind::Mark { damage_mult: e.mult, cash_mult: e.damage },
                "strip" => EffectKind::Strip,
                "unarmor" => EffectKind::Unarmor { bits: e.immune },
                "worth" => EffectKind::Worth { per_tier: e.damage },
                "destroy_emit" => match &e.projectile {
                    Some(p) => EffectKind::DestroyEmit {
                        proj: p.clone(),
                        emission: Box::new(e.emission.clone().unwrap_or(Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None})),
                        dont_copy: e.dont_copy,
                    },
                    None => continue,
                },
                "carry" => match &e.projectile {
                    Some(p) => EffectKind::Carry {
                        proj: p.clone(),
                        emission: Box::new(e.emission.clone().unwrap_or(Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None})),
                        uids: Vec::new(),
                    },
                    None => continue,
                },
                "pop_fx" => match &e.fx {
                    Some((fx, scale)) => EffectKind::PopFx { fx: fx.as_str().into(), scale: *scale },
                    None => continue,
                },
                "pop_emit" => match &e.projectile {
                    Some(p) => EffectKind::PopEmit {
                        ignore: e.ignore_same_frame,
                        skip: e.ignore_same_frame,
                        proj: p.clone(),
                        emission: Box::new(e.emission.clone().unwrap_or(Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None})),
                    },
                    None => continue,
                },
                _ => continue,
            };
            // HYPOTHESIS P-LIFEFRAMES: the mutator's lifespanFrames is (int)(lifespan x 60) in f32
            // (recording ninja_005 rt 204: a 0.325 s stun on a MOAB, 19.5 in f32, lasts 19
            // frames; no other recording changes).
            let lf = { let f = lifespan * FPS; if f.is_finite() && f < 1e9 { f.trunc() } else { f } };
            // Bloon.AddMutatorForNumLayers (VERIFIED-DISASM @0xfb01f0) gives layer charges only
            // for layers > 0: one with 0 is never depleted and goes to the children
            // (CopyMutatorsFrom; recording engineer_032, a foamed Black's pinned Pink half speed).
            add.push((Effect { kind, id: e.id.clone(), frames_left: lf, total: lf, added: self.sim_elapsed, layers: if e.layers == 0 { u32::MAX } else { e.layers }, owner, distribute: (e.distribute, e.distribute_override), keep: e.keep_on_degrade, overlay: e.overlay.as_str().into(), seq: 0 }, e.refresh, (e.replace, e.kind == "slow" && e.unique && !e.cascade && e.only_if_pop, e.kind == "slow" && !e.cascade && !e.only_if_pop)));
        }
        let kind = self.bloons[i].kind;
        add.retain(|(e, _, _)| !self.bad_immune_to(kind, &e.id));
        let mut next_seq = self.effect_seq;
        let b = &mut self.bloons[i];
        // The same model's mutator: a slow is told by its multiplier, glue level and lifespan.
        let same_model = |x: &Effect, e: &Effect| match (&x.kind, &e.kind) {
            (EffectKind::Slow { mult: a, glue: g }, EffectKind::Slow { mult: m, glue: h }) => a == m && g == h && x.total == e.total,
            _ => true,
        };
        for (eff, refresh, (replace, unique, dup_slow)) in add {
            // removeMutatorIfNotMatching: one another model put on goes (INFERRED: a model told
            // by its slow, glue level and lifespan; recording glue_upgrades_r30, Corrosive Glue's
            // level 2 over the level 1 it was upgraded from, so the glob skips it).
            if replace {
                if let Some(k) = b.effects.iter().position(|x| x.id == eff.id) {
                    if !same_model(&b.effects[k], &eff) {
                        b.effects.remove(k);
                    }
                }
            }
            // A SlowOnPop stun (Pin) on a bloon that has one (recordings engineer_002 / 032 with
            // SimRecorder's per-frame S lines): Mutable.AddMutator puts a second TimedMutator on, at
            // the binary-search place of its priority (`mutator_slot`), and AddMutatorForNumLayers
            // a charge entry for it; this pop's DepleteMutatorsForNumLayers empties the old entry and
            // Mutable.RemoveMutator takes the FIRST TimedMutator with that (shared) mutator. New one
            // in front: it goes, the old time stays (002: re-pinned at 56, moves at 74); behind: the
            // old one goes, a fresh lifespan (032: re-pinned at 2098, held to 2158).
            // Mutable.AddMutator (VERIFIED-DISASM @0x180f34f20) refreshes the mutator it has only with
            // cascadeMutators; without, a slow put on again is a second TimedMutator at the
            // binary-search slot (recording engineer_032's S lines: Cleansing Foam's two puddles
            // leave [Slow, Stun, Slow]). Only the first with an id applies (speed_mult).
            if dup_slow && b.effects.iter().any(|x| x.id == eff.id) {
                // (Mutable.highestGlueLevel still rises: the glue level stays on the first one, as
                // the refresh did; recording glue_upgrades_r30.)
                if let EffectKind::Slow { glue: g, .. } = eff.kind {
                    if let Some(EffectKind::Slow { glue, .. }) = b.effects.iter_mut().find(|x| x.id == eff.id).map(|x| &mut x.kind) {
                        *glue = (*glue).max(g);
                    }
                }
                let mut e = eff;
                e.seq = next_seq;
                next_seq += 1;
                let at = mutator_slot(&b.effects);
                b.effects.insert(at, e);
                continue;
            }
            if unique {
                if let Some(first) = b.effects.iter().position(|x| x.id == eff.id) {
                    let at = mutator_slot(&b.effects);
                    if at > first {
                        let mut e = eff;
                        e.seq = next_seq;
                        next_seq += 1;
                        b.effects.insert(at, e);
                        b.effects.remove(first);
                    } else {
                        b.effects[first].layers = eff.layers;
                    }
                    continue;
                }
            }
            match b.effects.iter_mut().find(|x| x.id == eff.id) {
                // Bloon.AddMutator on a mutator it has: a new lifespan, its behaviours kept (a
                // DamageOverTime keeps its lastTick: recording boomerang_500, re-dotted at 98 and
                // 122, still ticks 60 frames after 77).
                // Mutable.AddMutator (VERIFIED-DISASM @0xf53a30) with refresh, on a mutator it has:
                // that one stays as it is (its model, layers, behaviours: a DamageOverTime keeps
                // its lastTick, recording boomerang_500); only when it has less left than the new
                // lifespan does it get its own lifespan back (recording ice_052: a 3 s freeze
                // isn't cut to the Re-Freeze pulse's 0.6 s). Bloon.AddMutatorForNumLayers
                // (VERIFIED-DISASM @0xfced00) then resets its layer charges
                // (ResetMutatorChargeCountForId; recording ice_100: a Permafrosted Green's Blue
                // keeps the slow). INFERRED: to the new count (the same model's).
                // HYPOTHESIS P-SLOWONPOPUNIQUE: a SlowOnPop (isUnique, no cascadeMutators) over a
                // mutator with its id keeps that one's time left, only its layer charges reset
                // (AddMutatorForNumLayers; recording engineer_002: a pinned Pink a nail pops stays
                // pinned until the first stun ends). Super Glue's "Stun" (also isUnique, no cascade)
                // does refresh (recordings glue_025 / 205_*): what tells them apart in
                // Mutable.AddMutator / Slow.Collide isn't found yet.
                Some(x) if unique => {
                    x.layers = eff.layers;
                }
                Some(x) if refresh => {
                    // FilterGlueLevel reads the bloon's glue level, which a higher-level glue
                    // raises though its mutator isn't added (INFERRED: recording
                    // glue_upgrades_r30, Corrosive Glue over the level 1 put on before the
                    // upgrade, the glob then skips that Lead; glue_250, Glue Storm's level 1 keeps
                    // the attack's level 2 going).
                    if let (EffectKind::Slow { glue, .. }, EffectKind::Slow { glue: g, .. }) = (&mut x.kind, &eff.kind) {
                        *glue = (*glue).max(*g);
                    }
                    if x.frames_left < eff.frames_left {
                        x.frames_left = x.total;
                    }
                    x.layers = eff.layers;
                }
                // An isUnique AddBehaviorToBloonModel mutator gets a limiter of one
                // (BehaviorMutator.Initialise @0xf4e520), not a block: Mutable.AddMutator
                // (@0xf53a30) adds it to the list, at the binary-search place of its priority
                // (all 1 here: the middle, @0x18018dac0), and ApplyMutation (@0xf53ec0) applies
                // only the first one with its id. The bloon's DamageOverTime carries on with the
                // model of that one: lastTick kept, the new damage (recording
                // boomerang_paragon_r98: a 0-0-5 burn of 50 ticks 500 on its old beat once the
                // paragon's burn goes on). INFERRED for DoTs only: slows here all multiply.
                Some(x) if matches!(x.kind, EffectKind::Dot { .. }) => {
                    let tick = match &x.kind {
                        EffectKind::Dot { tick, .. } => *tick,
                        _ => unreachable!(),
                    };
                    let (added, old_seq) = (x.added, x.seq);
                    let at = b.effects.len() / 2;
                    let first = b.effects.iter().position(|x| x.id == eff.id).unwrap();
                    let mut eff = eff;
                    if at <= first {
                        if let EffectKind::Dot { tick: t, .. } = &mut eff.kind {
                            *t = tick;
                        }
                        // (The same DamageOverTime behaviour: it isn't new this frame; recording
                        // glue_205, Corrosive Glue's tick at 201 with a new copy put on then.)
                        eff.added = added;
                        eff.seq = old_seq;
                    } else {
                        eff.seq = next_seq;
                        next_seq += 1;
                    }
                    b.effects.insert(at, eff);
                }
                Some(_) => {}
                None => {
                    let mut eff = eff;
                    eff.seq = next_seq;
                    next_seq += 1;
                    let at = mutator_slot(&b.effects);
                    b.effects.insert(at, eff);
                }
            }
        }
        self.effect_seq = next_seq;
        self.attach_carries(i);
    }

    /// CarryProjectile.Attatched (@0x1000040): its projectiles are made where the bloon is, along
    /// its rotation, at its height (recording ice_004: an Icicles MOAB's icicle out in the
    /// collision pass that put the mutator on).
    fn attach_carries(&mut self, i: usize) {
        self.attach_carries_as(i, None);
    }

    /// `attach_carries` for a copy a degrade made: its behaviours run before it has a layerId of
    /// its own (Bloon.UpdateLayerId), so its projectiles' collidedWith gets `layer`, the popped
    /// bloon's, which every child has in its lineage (recording ice_005: an Icicles Rainbow's
    /// Blacks and Whites don't pop each other).
    fn attach_carries_as(&mut self, i: usize, layer_as: Option<u64>) {
        let todo: Vec<(usize, Box<Proj>, Box<Emission>, u64)> = self.bloons[i]
            .effects
            .iter()
            .enumerate()
            .filter_map(|(k, e)| match &e.kind {
                EffectKind::Carry { proj, emission, uids } if uids.is_empty() => Some((k, proj.clone(), emission.clone(), e.owner)),
                _ => None,
            })
            .collect();
        for (k, proj, emission, owner) in todo {
            let b = &self.bloons[i];
            let rot = if self.defs[b.kind].rotate_to_follow_path { b.rot } else { 0.0 };
            let (pos, z, id, layer) = (b.pos, b.height, b.id, layer_as.unwrap_or(b.layer));
            let tiers = self.towers.iter().find(|t| t.id == owner).map_or([0; 3], |t| t.tiers);
            let start = self.projectiles.len();
            self.emit_rot = Some(rot);
            self.emit_z = z;
            self.emit(pos, 0.0, &emission, &proj, None, tiers, owner);
            self.emit_z = 0.0;
            self.emit_rot = None;
            let mut made = Vec::new();
            // CarryProjectile.Emit (VERIFIED-DISASM @0x180fe1630): each projectile's collidedWith
            // gets the bloon's layerId, so it never hits its carrier or what that becomes.
            for pr in &mut self.projectiles[start..] {
                pr.carrier = id;
                pr.hit.push(layer);
                made.push(pr.uid);
            }
            if let EffectKind::Carry { uids, .. } = &mut self.bloons[i].effects[k].kind {
                *uids = made;
            }
        }
    }

    /// CarryProjectile.Process (@0x1000840): each carried projectile is put where its bloon is;
    /// CarryProjectile.OnDestroy / LayersExhaused: they go with the mutator.
    fn carry_follow(&mut self, drop: bool) {
        if !self.projectiles.iter().any(|p| p.carrier != 0) {
            return;
        }
        let mut at: FastMap<u64, ([f32; 2], u64, bool)> = FastMap::default();
        for b in &self.bloons {
            for e in &b.effects {
                if let EffectKind::Carry { uids, .. } = &e.kind {
                    for &u in uids {
                        // (Not processed the frame it went on: recording ice_004, the icicle
                        // stays where the MOAB was hit, then follows it.)
                        at.insert(u, (b.pos, b.id, e.added == self.sim_elapsed));
                    }
                }
            }
        }
        self.projectiles.retain_mut(|pr| {
            if pr.carrier == 0 {
                return true;
            }
            match at.get(&pr.uid) {
                Some(&(pos, id, new)) => {
                    if !drop && !new {
                        pr.pos = pos;
                        pr.carrier = id;
                    }
                    true
                }
                None => false,
            }
        });
    }

    /// EmitOnPop.PreDegrade (VERIFIED-DISASM @0x100bcd0): its projectiles from where the bloon
    /// popped, along its rotation, each with the popped bloon in collidedWith, so none hits what
    /// it degraded into (recording glue_005: a MOAB's glue splat passes over its Ceramics).
    fn emit_pop(&mut self, pos: [f32; 2], proj: &Proj, emission: &Emission, owner: u64, rot: f32, layer: u64, z: f32) {
        let tiers = self.towers.iter().find(|t| t.id == owner).map_or([0; 3], |t| t.tiers);
        let start = self.projectiles.len();
        self.emit_rot = Some(rot);
        // At the bloon's height (recording glue_005: a MOAB's splat at z 120).
        self.emit_z = z;
        self.emit(pos, 0.0, emission, proj, None, tiers, owner);
        self.emit_z = 0.0;
        self.emit_rot = None;
        for pr in &mut self.projectiles[start..] {
            pr.hit.push(layer);
        }
    }

    /// A projectile going (pierce or lifespan run out): its sounds, its exhaust and expire
    /// projectiles (with whether it ran out of pierce) and the sub-tower it makes.
    #[allow(clippy::type_complexity)]
    fn retire_projectile(&mut self, pr: &Projectile) -> (Vec<([f32; 2], f32, crate::data::Exhaust, [u8; 3], u64, Option<u64>, (Option<f32>, f32, f32), bool)>, Option<(std::sync::Arc<crate::data::SubTowerSpec>, [f32; 2], u64, Option<f32>, bool)>) {
        let (mut out, mut sub) = (Vec::new(), None);
            // HYPOTHESIS P-EXPIRESOUND: expire sounds play whenever the projectile goes,
            // exhaust sounds only when its pierce ran out.
            self.sound(&pr.spec.expire_sounds, None);
            // Its effects (visual): CreateEffectOnExhausted out of pierce (where it was hit),
            // CreateEffectOnExpire when its lifespan ran out.
            let (fxs, at) = if pr.pierce <= 0.0 { (&pr.spec.exhaust_fx, pr.prev) } else { (&pr.spec.expire_fx, pr.pos) };
            for (fx, k) in fxs {
                self.place_fx.push((at, fx.clone(), *k));
            }
            if pr.pierce <= 0.0 {
                self.sound(&pr.spec.exhaust_sounds, None);
            }
            // (Projectile.Expire and .Exhausted both run its Exhausted: any not done yet.
            // Only Expire, its lifespan run out, first runs Expired: CreateProjectileOnExpire,
            // VERIFIED-DISASM Projectile.Expire @0xcab2a0; recording sub_003, an Airburst
            // dart out of pierce on a Lead makes 3 darts, not 6.)
            let expired = pr.pierce > 0.0;
            let order = pr.spec.on_exhaust.iter().enumerate().filter(|e| e.1.expire).chain(pr.spec.on_exhaust.iter().enumerate().filter(|e| !e.1.expire));
            for (k, ex) in order {
                if pr.exhaust_done & (1 << k) != 0 || (!self.rounds_active() && !ex.between_rounds) || (ex.expire && !expired) {
                    continue;
                }
                let mut ex = ex.clone();
                if let Some([interval, pierce, radius]) = ex.per_distance.filter(|d| d[0] > 0.0) {
                    let n = ((pr.pos[0] - pr.origin[0]).hypot(pr.pos[1] - pr.origin[1]) / interval).floor();
                    ex.projectile.pierce += n * pierce;
                    ex.projectile.radius += n * radius;
                }
                // (Out of pierce, it was exhausted in Projectile.CollideBloon, before this
                // frame's move: recording boomerang_paragon_r63, a glaive on a Lead leaves
                // its MoabDot where it was.)
                let at = if pr.pierce <= 0.0 { pr.prev } else { pr.pos };
                // Projectile.GetPositionOfTarget (VERIFIED-DISASM @0xc8cba0): with the bloon gone, the
                // Target's cached first spot (recording sub_030: a Ballistic Missile explodes where
                // it first was, not where its bloon died); no target at all, its own position. A live
                // target: its position plus the aim's collision circle (sub_032: a blast on a MOAB).
                let at = if ex.use_target { pr.target.and_then(|id| self.bloons.iter().find(|b| b.id == id)).or_else(|| pr.target_obj.and_then(|o| self.bloons.iter().find(|b| b.obj == o))).map_or(pr.target_cache.unwrap_or(at), |b| [b.pos[0] + pr.inst_off[0], b.pos[1] + pr.inst_off[1]]) } else { at };
                if ex.use_target && !pr.target.is_some_and(|id| self.bloons.iter().any(|b| b.id == id)) {
                    ex.stale_target = pr.target_obj.map(|o| (o, pr.inst_off));
                }
                out.push((at, pr.vel[1].atan2(pr.vel[0]), ex, pr.tiers, pr.owner, pr.aim.or(pr.target), (pr.grot, pr.z, pr.dir_rot()), pr.pierce <= 0.0));
            }
            // CreateTypedTower.Exhausted: the towerModelToMake chosen when it was fired.
            let n = pr.spec.create_tower.len();
            if n > 0 {
                let spec = pr.spec.create_tower[pr.typed.unwrap_or(0).min(n - 1)].clone();
                // CreateTower.Exhausted: positionAtTarget puts it on the projectile's Target
                // (recording engineer_200: a Sentry on its point (-101.07, -1.13), not the
                // arrive lerp's float off it).
                let at = match (pr.spec.create_at_target, pr.arrive_at) {
                    (true, Some((_, _, tg))) => tg,
                    _ => pr.pos,
                };
                // (CreateTypedTower.CreateTower, VERIFIED-DISASM @0xcb0df0, always takes the
                // emitter's target type: recording engineer_420_close, Close Sentry Experts.)
                sub = Some((spec, at, pr.owner, pr.grot, pr.spec.create_parent_target || n == 4));
            }
        (out, sub)
    }

    /// CreateProjectileOnExhaustFraction's projectiles.
    #[allow(clippy::type_complexity)]
    fn spawn_exhausted(&mut self, exhausted: Vec<([f32; 2], f32, crate::data::Exhaust, [u8; 3], u64, Option<u64>, (Option<f32>, f32, f32))>) {
        for (pos, dir, ex, tiers, owner, target, (grot, z, dir_rot)) in exhausted {
            // (Recording sub_250: a Ballistic Missile's blast on the child that took its dead
            // target's object that very frame.)
            let pos = ex.stale_target.and_then(|(o, off)| self.bloons.iter().find(|b| b.obj == o).map(|b| { let at = if b.prev[0].is_finite() { b.prev } else { b.pos }; [at[0] + off[0], at[1] + off[1]] })).unwrap_or(pos);
            let start = self.projectiles.len();
            // ArcEmission.GetDirection starts from the emitting tower's rotation now (recording
            // dart_500: a Juggernaut's burst turns with the tower's newer throw); the projectile's
            // own when the tower's isn't known.
            self.emit_rot = self.towers.iter().find(|t| t.id == owner).and_then(|t| t.game_rot).or(grot);
            self.emit_z = z;
            self.emit_target = target;
            self.emit_dir_rot = Some(dir_rot);
            self.emit(pos, dir, &ex.emission, &ex.projectile, None, tiers, owner); // P-EXHAUST
            self.emit_target = None;
            self.emit_dir_rot = None;
            self.emit_rot = None;
            self.emit_z = 0.0;
            // Made while the projectiles were processed, after this frame's pass: they move on the
            // next frame (recording dart_500: the Ultra-Juggernaut's twelve balls).
            for pr in &mut self.projectiles[start..] {
                pr.fresh = false;
            }
            if ex.projectile.only_target {
                for pr in &mut self.projectiles[start..] {
                    pr.target = target;
                }
            }
        }
    }

    /// Emission.BaseEmit (VERIFIED-DISASM @0xc3dce0): the emit point is raised to the Target's
    /// height less its model's (20, or 120 for a MOAB; +0x88 of BloonModel): nothing for a bloon
    /// at its own height, but a MOAB just popped into a Ceramic is still 120 up, so 100 (recording
    /// sub_005: an Airburst dart popping its MOAB splits at z 100, over the 100-high logs).
    fn emit_height(&self) -> f32 {
        let z = self.emit_z;
        let Some(b) = self.emit_target.and_then(|id| self.bloons.iter().find(|b| b.id == id)) else { return z };
        let t = b.height - if self.defs[b.kind].is_moab { 120.0 } else { 20.0 };
        if z < t { t } else { z }
    }

    #[allow(clippy::too_many_arguments)]
    /// CreateProjectileOnIntervalModel (Ball Lightning). HYPOTHESIS P-INTERVAL: every
    /// `intervalFrames` frames the projectile fires its sub-projectile at a uniformly random
    /// bloon within `range` (targetType "Random"), skipping the shot when there is none.
    fn interval_shots(&mut self) {
        for p in 0..self.projectiles.len() {
            let Some(oi) = self.projectiles[p].spec.on_interval.clone() else { continue };
            let pr = &mut self.projectiles[p];
            pr.interval_t += 1.0;
            if pr.interval_t < oi.frames {
                continue;
            }
            pr.interval_t = 0.0;
            let (pos, camo, tiers, owner) = (pr.pos, pr.camo, pr.tiers, pr.owner);
            let near: Vec<(u64, [f32; 2])> = self
                .bloons
                .iter()
                .filter(|b| camo || !b.camo(&self.defs[b.kind]))
                .map(|b| (b.id, b.pos))
                .filter(|(_, q)| (q[0] - pos[0]).powi(2) + (q[1] - pos[1]).powi(2) <= oi.range * oi.range)
                .collect();
            if near.is_empty() && oi.only_if_target {
                continue;
            }
            let pick = (!near.is_empty()).then(|| near[((self.rand() * near.len() as f32) as usize).min(near.len() - 1)]);
            let dir = pick.map_or(0.0, |(_, q)| (q[1] - pos[1]).atan2(q[0] - pos[0]));
            if oi.once {
                self.projectiles[p].interval_t = f32::MIN;
            }
            let start = self.projectiles.len();
            self.emit(pos, dir, &oi.emission, &oi.projectile, None, tiers, owner);
            for sp in &mut self.projectiles[start..] {
                sp.camo = camo;
                if let (true, Some((id, q))) = (oi.projectile.instant, pick) {
                    sp.target = Some(id);
                    sp.pos = q;
                }
            }
        }
    }

    fn emit(&mut self, from: [f32; 2], dir: f32, em: &Emission, spec: &Proj, crit: Option<f32>, tiers: [u8; 3], owner: u64) {
        if self.mod_runtime.intercepts & crate::mod_runtime::FIRE==0 {
            self.emit_native(from,dir,em,spec,crit,tiers,owner);return;
        }
        let event=serde_json::json!({"owner":owner,"position":from,"direction":dir,"count":em.count,
            "damage":spec.damage,"speed":spec.speed,"pierce":spec.pierce,"tiers":tiers});
        // A mod's handler can change bloons.
        self.target_index = None;
        let d=self.mod_intercept("before_fire",event);if d.cancel{return;}
        let modified=(d.damage.is_some()||d.speed.is_some()||d.pierce.is_some()).then(||{
            let mut p=spec.clone();if let Some(v)=d.damage{p.damage=v as f32;}
            if let Some(v)=d.speed{p.speed=v;}if let Some(v)=d.pierce{p.pierce=v;}p
        });
        let emission=d.count.map(|n|{let mut e=em.clone();e.count=n;e});
        self.emit_native(from,d.direction.unwrap_or(dir),emission.as_ref().unwrap_or(em),modified.as_ref().unwrap_or(spec),crit,tiers,owner);
    }
    fn emit_native(&mut self, from: [f32; 2], dir: f32, em: &Emission, spec: &Proj, crit: Option<f32>, tiers: [u8; 3], owner: u64) {
        // Stationary area hits with no display of their own are drawn as bursts.
        if spec.link_radius || (spec.speed == 0.0 && !spec.instant && em.kind != "AtTarget" && em.kind != "InstantDamage" && spec.display.as_deref().is_none_or(str::is_empty)) {
            let ice = spec.effects.iter().any(|e| e.kind == "freeze");
            self.bursts.push(Burst { pos: from, radius: spec.radius, fire: spec.link_radius && !ice, ice });
        }
        let n = em.count.max(1);
        for k in 0..n {
            let (mut a, mut off) = (dir, 0.0f32);
            let (mut rnd_off, mut rnd_rot, mut rnd_speed): (Option<[f32; 2]>, Option<f32>, Option<f32>) = (None, None, None);
            match em.kind.as_str() {
                "Arc" if n > 1 => {
                    let total = em.angle.to_radians();
                    let full = (em.angle - 360.0).abs() < 1e-3;
                    let step = if full { total / n as f32 } else { total / (n - 1) as f32 };
                    // ArcEmission.GetDirection: rotation - start + offset + k x slice degrees in the
                    // game's frame, whose y is the clone's -y (so the angles turn the other way;
                    // recording dart_500: the Ultra-Juggernaut's two arcs, 30 degrees apart).
                    let start = if full { 0.0 } else { total / 2.0 };
                    a = dir + start - em.offset.to_radians() - step * k as f32;
                }
                "Parallel" => off = em.offset_start + em.spread * k as f32,
                // RandomEmission.Emit (VERIFIED-DISASM @0xc4a7a0), syncedRandom, per shot: (0, 1)
                // turned NextDouble x 360 times NextDouble x startOffset off the eject point; with
                // ejectPointRandomness another (0, 1) turned NextDouble x 360 (double) times
                // randomness x NextDouble; rotation = NextDouble x angle + the tower's - angle x 0.5;
                // with useSpeedMultiplier the speed x (min + NextDouble x (max - min)) (recording
                // desperado_003: three shots, six draws each).
                "Random" => {
                    let (d1, d2) = (self.synced_double(), self.synced_double());
                    let u = game_dir(d1 as f32 * 360.0);
                    let r = d2 as f32 * em.offset_start;
                    let mut o = [u[0] * r, u[1] * r];
                    if em.eject_random > 0.0 {
                        let (d3, d4) = (self.synced_double(), self.synced_double());
                        let u = game_dir((d4 * 360.0) as f32);
                        let r = (em.eject_random as f64 * d3) as f32;
                        o = [o[0] + u[0] * r, o[1] + u[1] * r];
                    }
                    rnd_off = Some(o);
                    let base = self.emit_rot.unwrap_or_else(|| RAD2DEG * crt_atan2(dir.cos(), -dir.sin()));
                    let d5 = self.synced_double();
                    rnd_rot = Some((d5 as f32 * em.angle + base) - em.angle * 0.5);
                    if em.use_speed {
                        let span = em.speed_max - em.speed_min;
                        rnd_speed = Some(if span != 0.0 { em.speed_min + (self.synced_double() * span as f64) as f32 } else { em.speed_min });
                    }
                }
                _ => {}
            }
            let (s, c) = a.sin_cos();
            // The game's direction (SingleEmission / ArcEmission.GetDirection: rotation - offsetStart
            // + offset + k x sliceSize degrees, Vector2.Rotate of (0, 1)).
            // EmissionRotationOffProjectileDirection.OnGetDirection (VERIFIED-DISASM @0xc5b290),
            // from a projectile: k x angleInBetween + its direction's rotation + startingOffset
            // (with alwaysCentre -(count - 1) x angleInBetween x 0.5) (recording sub_002: an
            // Airburst dart's three at -25, 0, +25 off where it flies, not the tower's facing).
            let off_dir = em.dir_off.zip(self.emit_dir_rot).map(|([start, step, centre], r)| {
                let start = if centre != 0.0 { -((n as f32 - 1.0) * step * 0.5) } else { start };
                // (The rotation and the start offset are summed first: recordings sub_002 / 004 /
                // 502_*, Airburst splits exactly 25 apart from -48.56562.)
                k as f32 * step + (r + start)
            });
            let grot = off_dir.or_else(|| self.emit_rot.and_then(|r| match em.kind.as_str() {
                "Arc" if n > 1 => {
                    let full = (em.angle - 360.0).abs() < 1e-3;
                    // ArcEmission.GetDirection (VERIFIED-DISASM @0xc56720): (rotation - angle / 2 +
                    // offset) + k x slice, a full circle too (recording dart_500's twelve balls).
                    let slice = if full { em.angle / n as f32 } else { em.angle / (n - 1) as f32 };
                    Some((r - em.angle / 2.0) + em.offset + k as f32 * slice)
                }
                "Single" | "Arc" | "Parallel" => Some(r),
                _ => None,
            })).or(rnd_rot);
            let gdir = grot.map(game_dir);
            let mut pos = [from[0] - s * off, from[1] + c * off];
            if let Some(o) = rnd_off {
                pos = [from[0] + o[0], from[1] + o[1]];
            }
            // ParallelEmission.Emit (VERIFIED-DISASM @0xc67930, game frame): side = the direction
            // turned 90 degrees (unless linear), base = side x spreadLength x 0.5 + back x yOffset,
            // shot k at base - side x (spreadLength / (count - 1)) x k + back x yOffset from the
            // eject point (recording dart_005: arrows 1.5 either side, 2 behind).
            if let (Some(g), "Parallel") = (gdir, em.kind.as_str()) {
                // Vector2.Rotate (VERIFIED-DISASM @0xf64800), in place, game frame.
                let rot = |v: [f32; 2], deg: f32| {
                    let a = DEG2RAD * -deg;
                    let (sa, ca) = crt_sin_cos(a);
                    [ca * v[0] - sa * v[1], ca * v[1] + sa * v[0]]
                };
                // (The direction as is, not renormalized; recordings dart_005 / 025 / 205.)
                let d = [g[0], -g[1]];
                let side = if em.random_angle > 0.0 { d } else { rot(d, -270.0) };
                let half = [side[0] * em.spread * 0.5, side[1] * em.spread * 0.5];
                let back = [side[1], -side[0]];
                let by = [back[0] * em.offset, back[1] * em.offset];
                let base = [half[0] + by[0], half[1] + by[1]];
                let side2 = rot(side, -180.0);
                let step = if n > 1 { em.spread / (n - 1) as f32 } else { 0.0 };
                let along = [side2[0] * step * k as f32, side2[1] * step * k as f32];
                let o = [along[0] + by[0], along[1] + by[1]];
                let gp = [base[0] + o[0], base[1] + o[1]];
                pos = [from[0] + gp[0], from[1] - gp[1]];
            }
            let (value,cash_base) = match &spec.cash {
                Some(cm) => {
                    let inc = self.towers.iter().position(|t| t.id == owner).and_then(|ti| self.buffs.get(ti)).map_or(1.0, |b| b.income_mult);
                    let base=cash_value(cm,self.rand());(base*inc as f64,base)
                }
                None => (0.0,0.0),
            };
            // RandomRangeTravelStrait.Initialise (VERIFIED-DISASM @0x180cd46c0): lifespanFrames =
            // (int)(((max - min) x syncedRandom.NextDouble() + min) / speed x 60 + 0.5) (recording
            // bomb_005: Bomb Blitz's Cluster fragments). HYPOTHESIS P-RANDRANGEEXPIRE: Process's
            // frame - made >= lifespanFrames counts like an age.
            let random_frames = spec.random_range.map(|[lo, hi, speed]| {
                let d = self.synced_double();
                ((((hi - lo) as f64 * d + lo as f64) / speed as f64) * 60.0 + 0.5) as i32 as f32
            });
            self.next_uid += 1;
            self.proj_list.push(self.next_uid);
            self.projectiles.push(Projectile {
                uid: self.next_uid,
                z: self.emit_height(),
                dir: gdir.map(|v| [v[0], -v[1]]),
                seek_moved: false,
                seek_t: self.sim_elapsed,
                pos,
                prev: pos,
                fresh: true,
                border_last: None,
                // Projectile rotation: its direction's Vector2.get_Rotation (atan2f(x, y) x Rad2Deg,
                // game frame), a float step or two from the emission's angle (recording
                // boomerang_000: -16.441418 thrown, -16.44142 on the boomerang).
                grot: gdir.map(|v| RAD2DEG * crt_atan2(v[0], -v[1])),
                vel: match gdir {
                    // TravelStrait: direction x speedFrames.
                    Some(v) => {
                        // (RandomEmission's speed multiplier on speedFrames, speed x 1/60: recording
                        // desperado_205, pellet 566 steps 3.236606 in y, not 3.2366054.)
                        let sf = spec.speed * FRAME_TIME;
                        let sp = rnd_speed.map_or(sf, |m| m * sf);
                        [v[0] * sp, v[1] * sp]
                    }
                    None => [c * spec.speed / FPS, s * spec.speed / FPS],
                },
                // AgeModel.set_Lifespan (VERIFIED-DISASM @0x11dd100): lifespanFrames =
                // (int)Math.Round((double)(lifespan x 60)), half to even (recording dart_051: a
                // 20.25-frame dart is gone after 20).
                frames_left: match random_frames {
                    Some(f) => f,
                    None => {
                        // A TravelStrait lifespan and an AgeModel each end the shot on their own: the
                        // shorter wins (recording tack_520: Inferno Ring's meteors, TravelStrait 99999 s,
                        // Age 20 s, gone after 1200 frames).
                        let life = match spec.age_lifespan.filter(|a| *a > 0.0) { Some(a) => spec.lifespan.min(a), None => spec.lifespan };
                        // (An Instant shot with neither has no lifespan: it stays till its target goes,
                        // destroyIfInvalid, or its pierce does; recording desperado_050 rt 105, the mark
                        // shot on its bloon.)
                        let life = if spec.instant && life <= 0.0 { f32::INFINITY } else { life };
                        let f = life * FPS;
                        if f.is_finite() { (f as f64).round_ties_even() as f32 } else { f }
                    }
                },
                // Projectile.Initialise (VERIFIED-DISASM @0xcac15d): ProjectileModel
                // .get_CappedPierce (pierce, or maxPierce when that's set and lower), at least 1
                // (recording glue_020: Corrosive Glue's pierce-4 glob with maxPierce 1 hits once).
                pierce: if spec.max_pierce > 0.0 && spec.max_pierce <= spec.pierce { spec.max_pierce } else { spec.pierce }.max(1.0),
                spec: {
                    let mut sp = spec.clone();
                    // SupportRemoveFilterOutTagModel: a buffed tower's shots skip their tag filters.
                    if !sp.exclude_tags.is_empty() && self.towers.iter().position(|t| t.id == owner).and_then(|ti| self.buffs.get(ti)).is_some_and(|b| b.unfilter) {
                        sp.exclude_tags.clear();
                    }
                    sp
                },
                crit,
                crit_roll: None,
                mirror: self.towers.iter().find(|t| t.id == owner).is_some_and(|t| t.flip),
                hit: HitSet::default(),
                clear_hits_next: false,
                refresh_due: false,
                carrier: 0,
                tiers,
                owner,
                target: None,
                cot: None,
                ignore_first: false,
                origin: pos,
                wait: 0.0,
                retarget_at: None,
                born: self.sim_elapsed * 4 + self.phase as u64,
                bounces: 0,
                camo: true,
                buff_camo: self.towers.iter().position(|t| t.id == owner).is_some_and(|i| self.buffs.get(i).is_some_and(|b| b.camo)),
                typed: None,
                fwd: [c, s],
                path_d: 0.0,
                on_track: None,
                track_off: [0.0; 2],
                clear_t: self.sim_elapsed as f32,
                age: 0.0,
                seek: None,
                seek_off: None,
                inst_off: [0.0; 2],
                instant_set: false,
                instant_pos: None,
                line_hits: None,
                line_spent: false,
                target_cache: None,
                target_obj: None,
                target_moved: false,
                effect_back: None,
                bolt_from: None,
                moving: 0.0,
                arrive_at: None,
                arrive_time: 0.0,
                rounds_left: spec.rounds,
                value,
                cash_base,
                refresh_t: 0.0,
                check_t: 0,
                interval_t: 0.0,
                exhaust_done: 0,
                aim: None,
            });
        }
    }

    pub fn tower_model(&self,def:usize)->&str {
        let (id,tower)=&self.tower_defs[def];tower.model.as_deref().unwrap_or(id)
    }

    /// InstantDamageEmission.Process / LineProjectileEmission.Emit: the pending instant shots hit
    /// now (their outputs wait in `instant_out` for the step to take).
    fn instant_pass(&mut self) {
        let sim_now = self.sim_elapsed;
        let mut kids: Vec<u64> = Vec::new();
        let mut subs: Vec<(std::sync::Arc<crate::data::SubTowerSpec>, [f32; 2], u64, Option<f32>, bool)> = Vec::new();
        let mut contacts: Vec<([f32; 2], f32, crate::data::Contact, [u8; 3], u64, Option<u64>, f32)> = Vec::new();
        let mut exhausted: Vec<([f32; 2], f32, crate::data::Exhaust, [u8; 3], u64, Option<u64>, (Option<f32>, f32, f32))> = Vec::new();
        if !self.instant_hits.is_empty() {
            let only = std::mem::take(&mut self.instant_hits);
            self.grid_on = true;
            let mut cand2 = std::mem::take(&mut self.grid_cand);
            let mut ex2 = Vec::new();
            let mut pi = 0;
            self.in_instant = true;
            collide_pass!(Some(&only[..]), self, pi, kids, ex2, subs, contacts, cand2, sim_now);
            self.in_instant = false;
            self.proj_cursor = usize::MAX;
            self.grid_on = false;
            self.grid_cand = cand2;
            let (now2, later2): (Vec<_>, Vec<_>) = ex2.into_iter().partition(|e| e.7);
            exhausted.extend(later2.into_iter().map(|(a, b, c, d, e, f, g, _)| (a, b, c, d, e, f, g)));
            for (pos, proj, emission, owner, rot, layer, z) in std::mem::take(&mut self.pop_emits) {
                self.emit_pop(pos, &proj, &emission, owner, rot, layer, z);
            }
            self.spawn_exhausted(now2.into_iter().map(|(a, b, c, d, e, f, g, _)| (a, b, c, d, e, f, g)).collect());
        }
        // (Bloons popped or made: the targeting index is rebuilt, from the frame-start spots.)
        self.target_index = None;
        let out = &mut self.instant_out;
        out.0.extend(kids);
        out.1.extend(subs);
        out.2.extend(contacts);
        out.3.extend(exhausted);
    }

    pub fn step(&mut self) {
        if !self.mod_runtime.active()&&!self.mod_runtime.pending_entities(){self.step_native();return;}
        let (round,active,towers,popped)=(self.round,self.round_active,self.mod_runtime.last_towers,self.popped);
        self.mod_entity_commands();
        self.mod_event("on_tick",None);
        self.step_native();
        if self.towers.len()!=towers {self.mod_runtime.last_towers=self.towers.len();self.mod_event("on_towers_changed",None);}
        if self.popped!=popped {self.mod_event("on_pop",None);}
        if self.round!=round || (active&&!self.round_active&&!self.game_over){self.mod_event("on_round_end",None);}
    }
    fn step_native(&mut self) {
        if self.game_over {
            return;
        }
        self.sim_elapsed += 1;
        self.lives_at_step = self.lives;
        self.in_step = true;
        self.step_first_id = self.next_id;
        self.phase = 0;
        self.grid_rank_pre = None;
        if self.sim_elapsed % 60 == 0 {
            if self.has_grid_rank {
                self.grid_rank_pre = Some(self.path_rank.iter().map(|(&id, &(_, k))| (id, k)).collect());
            }
            self.plist_flush();
            self.fact_flush();
            self.proj_flush();
        }
        self.ghosts_src = std::mem::take(&mut self.died_after_fill);
        self.fill_first_id = None;
        // HYPOTHESIS P-ACTIVERBE: a bloon counts its type's RBE less the damage it has taken.
        self.active_rbe = self
            .bloons
            .iter()
            .map(|b| (self.rbe[b.kind] - (self.defs[b.kind].max_health - b.hp).max(0) as f32).max(1.0))
            .sum();
        // Emissions: due now, made after this frame's collisions (the process list has the
        // CollisionChecker before the Spawner: recording dart_500, a pop's Red gets its id before
        // the round's next Black).
        self.frame_spawns.clear();
        if self.round_active {
            // Spawner.Process (VERIFIED-DISASM @0xf930c0): an emission goes once its time <= the
            // round's elapsed frames, which already count 1 on the round's first frame (recorded:
            // 55.3 frames apart -> frames 1, 56, 111).
            while let Some(&(f, k, idx)) = self.pending.last() {
                if f > self.round_frame + 1 {
                    break;
                }
                self.pending.pop();
                // STAND-IN P-SYPHON: while Syphon Funding runs, each bloon the round sends comes
                // one rank lower (its first child type; BADs excepted). Its extra cash isn't
                // modelled.
                let k = match self.child_kinds[k].first() {
                    Some(&c) if self.syphon > 0.0 && !self.defs[k].tags.iter().any(|x| x == "Bad") => c,
                    _ => k,
                };
                // Splitter.SpawnRout(emissionIndex): spawn paths[emissionIndex % count].
                self.logic_pre_spawn(k);
                let routed = self.spawn_route.as_ref().and_then(|r| r.route(idx as i32, self.round as i32, &self.defs[k].tags));
                let path = routed.unwrap_or(self.spawn_tracks[idx as usize % self.spawn_tracks.len()]) as u8;
                self.frame_spawns.push((k, path));
            }
            self.round_frame += 1;
        }
        // Sandbox spawns (any time, round or not).
        while let Some(&(f, k)) = self.sandbox_queue.last() {
            if f > self.steps {
                break;
            }
            self.sandbox_queue.pop();
            let path = self.spawn_tracks[self.steps as usize % self.spawn_tracks.len()] as u8;
            self.spawn_emitted(k, path);
        }
        self.steps += 1;
        self.logic_sim_time(self.steps as u32, self.round_clock as u32);
        // Simulation.Simulate: roundTime counts only while rounds are active (always in sandbox).
        if self.rounds_active() {
            self.round_clock += 1;
        }
        // Slow zones (Arctic Wind), measured from the tower.
        // MOAB Shove: around each Heli that has it (where the Heli is now).
        let shoves: Vec<([f32; 2], crate::data::MoabShove)> = (0..self.towers.len())
            .filter_map(|t| {
                let sh = self.tier(t)?.moab_shove.clone()?;
                Some((self.towers[t].air.as_ref().map_or(self.towers[t].pos, |a| a.pos), sh))
            })
            .collect();
        let mut zones = Vec::new();
        for t in 0..self.towers.len() {
            if let Some(tier) = self.tier(t) {
                for z in &tier.zones {
                    let r = if z.bind { self.range(t) + z.offset } else { z.radius };
                    zones.push((self.towers[t].pos, r, z.clone()));
                }
            }
        }
        let p0 = std::time::Instant::now();
        // EmitOnPopModel: pops after last frame's collision pass (in a bloon's own Process).
        for (pos, proj, emission, owner, rot, layer, z) in std::mem::take(&mut self.pop_emits) {
            self.emit_pop(pos, &proj, &emission, owner, rot, layer, z);
        }
        self.comanche_defence();
        self.forget_untouched();
        self.spirit_of_the_forest();
        self.collect_zones();
        self.auto_abilities();

        // AbilityReduceCooldownOnLivesLostModel: last frame's leak takes reductionSeconds off
        // the ability's cooldown, then waits delaySeconds before it can again (the wait is kept
        // as a "<ability>#leak" cooldown entry).
        if std::mem::take(&mut self.leaked) {
            for t in 0..self.towers.len() {
                let Some(list) = self.tier(t).map(|tr| tr.abilities.iter().filter_map(|a| Some((a.name.clone(), a.leak_cdr?))).collect::<Vec<_>>()) else { continue };
                let cd = &mut self.towers[t].ability_cd;
                for (name, (cut, wait)) in list {
                    let key = format!("{name}#leak");
                    if cd.get(&key).copied().unwrap_or(0.0) > 0.0 {
                        continue;
                    }
                    if let Some(v) = cd.get_mut(&name) {
                        *v = (*v - cut * FPS).max(0.0);
                    }
                    cd.insert(key, wait * FPS);
                }
            }
        }
        let e = p0.elapsed().as_secs_f64();
        self.prof[0] += e;
        self.prof_max[0] = self.prof_max[0].max(e);
        // Simulation.Simulate runs the process list in creation order, the CollisionChecker
        // first: this frame's hits (and pops) land before any tower acts (recording
        // dart_040_club: towers releasing at a Green the first one's darts popped that frame
        // keep their facing), on bloons where they are as the frame begins. Projectiles made
        // since the last pass are all in it.
        for b in &mut self.bloons {
            if !b.fresh {
                b.cell_pos = b.prev;
                b.prev = b.pos;
                b.prev_dist = b.dist;
            }
        }
        for pr in &mut self.projectiles {
            pr.fresh = false;
        }
        // Spawner.Process (VERIFIED-DISASM @0xf930c0): its bloons outside the track whose position
        // is now on the map (Map.IsPointOnMap) enter it, each tower's OnBloonEnterTrack: the
        // MOAB entrance voice and Pre-emptive Strike (recording sub_050: launched the frame after
        // a MOAB first stands on the map, not when it spawns). After the collision pass, so
        // what it makes waits a frame (fresh).
        let mut k = 0;
        while k < self.outside_track.len() {
            let id = self.outside_track[k];
            match self.bloons.iter().find(|b| b.id == id).map(|b| (b.pos, b.kind)) {
                None => {
                    self.outside_track.remove(k);
                }
                Some((p, kind)) if on_map(p, 0.0) => {
                    self.outside_track.remove(k);
                    self.preemptive_strikes(id);
                    self.enter_voice(kind);
                }
                Some(_) => k += 1,
            }
        }
        let p2 = std::time::Instant::now();
        // Projectiles.
        let (lo, hi) = self.bounds;
        let mut p = 0;
        let mut kids = Vec::new();
        let mut exhausted = Vec::new();
        let mut subs = Vec::new();
        let mut contacts = Vec::new();
        self.carry_follow(true);
        self.grid.build(&self.bloons, lo, hi);
        if self.has_grid_rank {
            let (rank, pre, single) = (&self.path_rank, &self.grid_rank_pre, self.tracks.len() == 1);
            let key = |b: &Bloon| match pre {
                Some(pre) => pre.get(&b.id).map_or(u64::MAX, |&k| k as u64),
                None => rank.get(&b.id).map_or(u64::MAX, |r| r.1 as u64),
            };
            // Bloons that died after the last fill, their objects pooled by a flush since: still
            // in the grid, dead, until a new bloon takes the object (recording glue_400).
            let ghosts: Vec<(u32, u64, [f32; 2], f32)> = self
                .ghosts_src
                .iter()
                .filter(|g| self.fact_at.get(g.0 as usize) == Some(&u32::MAX))
                .map(|&(o, id, at, order, r)| {
                    let k = if single {
                        match pre {
                            Some(pre) => pre.get(&id).map_or(u64::MAX, |&k| k as u64),
                            None => rank.get(&id).map_or(u64::MAX, |r| r.1 as u64),
                        }
                    } else {
                        order
                    };
                    (o, k, at, r)
                })
                .collect();
            let defs = &self.defs;
            self.ggrid.build(&self.bloons, |b| if single { key(b) } else { b.order }, |b| defs[b.kind].radius, self.step_first_id, &ghosts);
        }
        self.grid_on = true;
        let mut cand = std::mem::take(&mut self.grid_cand);
        let sim_now = self.sim_elapsed;
        let mut due_retargets: Vec<u64> = Vec::new();
        // The collision pass (CollisionChecker.Process), as a macro so InstantDamageEmission can run it
        // again for its own projectiles after the towers act.
        collide_pass!(None::<&[u64]>, self, p, kids, exhausted, subs, contacts, cand, sim_now);
        self.proj_cursor = usize::MAX;
        self.grid_on = false;
        self.grid_cand = cand;
        // Out of pierce, a shot is exhausted in Projectile.CollideBloon, in the collision pass:
        // its projectiles are made then, before any tower acts, and come before that frame's
        // throws (recording boomerang_paragon_r98: a MOAB Press blast gets a lower id than the
        // paragon's pulse thrown that frame, and collides first).
        let (now, later): (Vec<_>, Vec<_>) = exhausted.into_iter().partition(|e| e.7);
        let mut exhausted: Vec<_> = later.into_iter().map(|(a, b, c, d, e, f, g, _)| (a, b, c, d, e, f, g)).collect();
        // EmitOnPop.PreDegrade emits at the pop (VERIFIED-DISASM @0x100bcd0; recording ice_520:
        // the shards are out where the bloons popped on that frame), so this pass's pops come out
        // now. INFERRED: before the pass's exhaust projectiles.
        for (pos, proj, emission, owner, rot, layer, z) in std::mem::take(&mut self.pop_emits) {
            self.emit_pop(pos, &proj, &emission, owner, rot, layer, z);
        }
        self.spawn_exhausted(now.into_iter().map(|(a, b, c, d, e, f, g, _)| (a, b, c, d, e, f, g)).collect());
        let e = p2.elapsed().as_secs_f64();
        self.prof[2] += e;
        self.prof_max[2] = self.prof_max[2].max(e);
        self.plist_mark = self.path_lists.iter().map(Vec::len).collect();
        self.fact_mark = self.fact.len();
        self.phase = 1;
        // Bloons processed during the tower phase (they come before a newer Attack in the process
        // list), with what their Process produced; the rest are processed after the towers.
        let mut dots = Vec::new();
        let mut emits = Vec::new();
        let mut leak_voice = false;
        let mut early: FastSet<u64> = FastSet::default();
        let round_clock = self.round_clock;
        let p1 = std::time::Instant::now();
        // Towers.
        self.expire_subs(false);
        self.leak_immune = (self.leak_immune - 1.0).max(0.0);
        self.syphon = (self.syphon - 1.0).max(0.0);
        self.circle_subs();
        self.move_air();
        self.update_rage();
        self.buffs = self.compute_buffs();
        self.orbit_step(false);
        let cd_speed: Vec<f32> = (0..self.towers.len())
            .map(|t| self.submerged_scale(self.towers[t].pos, |s| (s.cooldown_scale, s.cooldown_scale_global)))
            .collect();
        // (GetRate runs as the weapon fires: bloons popped in this frame's collisions are gone;
        // recording sniper_005 rt 202.)
        self.compute_danger();
        self.grave_zones();
        self.towers_phase(None, &cd_speed, &mut early, round_clock, &mut dots, &mut emits, &zones, &shoves, &mut leak_voice);
        // InstantDamageEmission.Process: this frame's instant-damage shots hit their targets now,
        // before the bloons move. INFERRED: after every tower has acted (the game's emission
        // processes after its own weapon; the same unless two towers shoot one bloon that frame).
        self.instant_pass();
        {
            let (k, sb, c, e) = std::mem::take(&mut self.instant_out);
            kids.extend(k);
            subs.extend(sb);
            contacts.extend(c);
            exhausted.extend(e);
        }
        // A Paragon's Orbit is made again at the upgrade: in the process list it comes after its
        // kept Attacks and the bloons then out (recording boomerang_paragon_r78: it loses a bloon
        // that has just moved past 40).
        for t in 0..self.towers.len() {
            if self.towers[t].paragon.is_some() && self.tier(t).is_some_and(|tr| tr.orbit.is_some()) {
                let first = self.towers[t].paragon_active_at * 4;
                let stamp = *self.towers[t].attack_born.entry("#orbit".into()).or_insert(first);
                self.process_bloons_before(stamp, &mut early, round_clock, &mut dots, &mut emits, &zones, &shoves, &mut leak_voice);
            }
        }
        self.orbit_step(true);
        let e = p1.elapsed().as_secs_f64();
        self.prof[1] += e;
        self.prof_max[1] = self.prof_max[1].max(e);
        // Simulation.Simulate runs the process list in creation order: towers placed before a
        // bloon act on where it was last frame, then it moves (the recordings show a tower
        // throwing the frame after a bloon reaches its range).
        // Effect timers and damage over time: in Bloon.Process (mutator timers, DamageOverTime), after
        // the frame's collisions and towers, before the bloon moves (recording boomerang_005: a
        // MOAB Press stun with a frame left isn't re-applied by that frame's hit, then runs out).
        self.fill_first_id.get_or_insert(self.next_id);
        for i in 0..self.bloons.len() {
            if !early.contains(&self.bloons[i].id) {
                self.tick_bloon_effects(i, round_clock, &mut dots, &mut emits);
            }
        }
        let single = Emission { kind: "Single".into(), eject_random: 0.0, use_speed: false, speed_min: 1.0, speed_max: 1.0, count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0, dir_off: None , line_plain: false, line_tower_rot: false, line_ignore_blockers: false, line_from_target: None};
        let mut emitted: Vec<(std::ops::Range<usize>, u64)> = Vec::new();
        for (pos, proj, owner, from, layer) in emits {
            let tiers = self.towers.iter().find(|t| t.id == owner).map_or([0; 3], |t| t.tiers);
            let start = self.projectiles.len();
            self.emit(pos, 0.0, &single, &proj, None, tiers, owner);
            // (Its bloon is in the splash's collidedWith, its children by lineage: recording
            // ninja_004 rt 254, the splash passes over the four Ceramics of the MOAB it was on.)
            for pr in &mut self.projectiles[start..] {
                pr.hit.push(layer);
            }
            emitted.push((start..self.projectiles.len(), from));
        }
        self.phase = 2;
        // Movement + leaks.
        let mut i = 0;
        while i < self.bloons.len() {
            if early.contains(&self.bloons[i].id) || !self.move_bloon(i, &zones, &shoves, &mut leak_voice) {
                i += 1;
            }
        }
        // EmitOnTime (Sticky Bomb's splash): from where its bloon is after this frame's move
        // (recording ninja_004 rt 253: at -60.11, not -60.63).
        for (r, from) in emitted {
            if let Some(q) = self.bloons.iter().find(|b| b.id == from).map(|b| b.pos) {
                for pr in &mut self.projectiles[r] {
                    pr.pos = q;
                    pr.prev = q;
                }
            }
        }
        self.carry_follow(false);
        // Instant (its Process in the process list after the bloons it follows): an instant shot
        // takes its target's position from after this frame's move; not one made this frame
        // (recording sub_050: Pre-emptive Strike's missile sits at its launch point, then on
        // the MOAB where the MOAB is in each frame).
        let now = self.sim_elapsed * 4;
        // CreateProjectileOnExpire with useTargetPosition: the expiring shot's projectile goes
        // where its target is now that the bloons have moved (recording sub_040: a Ballistic
        // Missile's blast on the MOAB; Pre-emptive Strike's, without it, where the missile was).
        let mut p = 0;
        while p < self.projectiles.len() {
            let pr = &self.projectiles[p];
            if pr.spec.instant && pr.born < now && pr.frames_left <= 0.0 && pr.pierce > 0.0 && pr.spec.on_exhaust.iter().any(|e| e.use_target) {
                let pr = self.projectiles.remove(p);
                let (ex, sub) = self.retire_projectile(&pr);
                exhausted.extend(ex.into_iter().map(|(a, b, c, d, e, f, g, _)| (a, b, c, d, e, f, g)));
                subs.extend(sub);
            } else {
                p += 1;
            }
        }
        // Instant.Process (VERIFIED-DISASM @0x180cc1b30) with destroyIfInvalid: a shot whose Target is
        // gone goes now, after this frame's collisions (recording desperado_050 rt 120: the mark shot
        // on a bloon the execution popped that frame is gone at the frame's end).
        {
            let live: FastSet<u64> = self.bloons.iter().map(|b| b.id).collect();
            self.projectiles.retain(|pr| !(pr.spec.instant && pr.spec.instant_destroy && pr.born < now && pr.target.is_some_and(|id| !live.contains(&id))));
        }
        for pr in self.projectiles.iter_mut().filter(|pr| pr.spec.instant && pr.born < now && !(pr.spec.instant_once && pr.instant_set)) {
            // (Target.Position: the bloon's plus the collision circle it was aimed at; recording
            // sniper_032 rt 74, a shot on a MOAB keeps its 6.3 ahead.)
            let off = pr.inst_off;
            // (Process-list order: a bloon made after the shot hasn't moved yet when the shot's
            // Instant follows it, its previous spot; recording sniper_040 rt 1901, a bounced bullet
            // on Black 1718 at its rt 1900 place.)
            let born = pr.born;
            // (Instant.Process sets the whole Position, so z too: the bloon's height, 120 on a MOAB
            // (recording sniper_250: an Elite shot on a MOAB at z 120 is above the blocker that ends
            // one at 20).)
            let tz = pr.target.and_then(|id| self.bloons.iter().find(|b| b.id == id)).map(|b| b.height);
            if let Some(z) = tz {
                pr.z = z;
            }
            if let Some(q) = pr.target.and_then(|id| self.bloons.iter().find(|b| b.id == id)).map(|b| {
                let at = if b.born > born { b.prev } else { b.pos };
                [at[0] + off[0], at[1] + off[1]]
            }).or(pr.instant_pos) {
                pr.pos = q;
                pr.instant_set = true;
                // (Instant.Process, target valid: lastBloonPosition = Target.Position. Only a bouncing shot's
                // is kept here: the others stay put when their target goes, which is the same spot.)
                if pr.target.is_some() && pr.spec.retarget.is_some() {
                    pr.instant_pos = Some(q);
                }
            }
        }
        self.track_targets();
        // CreateSoundOnBloonLeak (OnLivesLost): the hero's line, at most every 2400 frames.
        if leak_voice {
            self.hero_voice(|s| &s.leak, "HeroLeak", 2400);
        }
        if self.lives <= 0.0 {
            self.lives = 0.0;
            self.game_over = !self.sandbox;
        }
        // Sandbox can't be lost. Simulation.OnSandboxHealthChanged (on lives lost / gained): at 0
        // health sandBoxResetHealthTimer starts at 300, else it's off; Simulation.Simulate counts it
        // down and at 0 sets health back to lastSetHealth.
        if self.sandbox {
            if self.lives != self.lives_at_step {
                self.sandbox_health_timer = if self.lives > 0.0 { -1 } else { 300 };
            }
            if self.sandbox_health_timer > 0 {
                self.sandbox_health_timer -= 1;
                if self.sandbox_health_timer == 0 {
                    self.lives = self.last_set_health;
                    self.sandbox_health_timer = -1;
                }
            }
        }
        // DamageOverTime damage: in Bloon.Process after Bloon.Move (recording boomerang_500: the
        // MOAB its tick pops leaves its first Ceramic where the MOAB had moved to).
        // Grow.Process: in Bloon.Process before its DamageOverTime damage, so a tick's pop that
        // restarts the timer (Grow.PreDegrade at the ceiling) isn't counted down that frame
        // (recording glue_300: Corrosive Glue pops a regrow bloon a layer every 30 frames and it
        // never grows back).
        self.regrow();
        let mut kids = Vec::new();
        // In process-list order (creation order, a first child in its parent's place), not
        // storage order (recording glue_250: a Black's tick pops it before a later White's).
        let order: FastMap<u64, u64> = self.bloons.iter().map(|b| (b.id, b.order)).collect();
        let mut dots = dots;
        dots.sort_by_key(|d| order.get(&d.0).copied().unwrap_or(u64::MAX));
        // Bloon.Process runs the behaviours in order, each DamageOverTime ticking and dealing its
        // damage in turn: once one degrades the bloon its behaviours are new (see degrade), and
        // the old ones after it don't run.
        let clock = self.round_clock as f32;
        let mut degraded: Vec<u64> = Vec::new();
        for k in 0..dots.len() {
            let (id, damage, immune, owner, distribute, _, _) = dots[k].clone();
            if degraded.contains(&id) {
                continue;
            }
            // The bloon's later DamageOverTime behaviours haven't run yet: their lastTick is still
            // the old one while this one's damage lands (what PostDegrade hands on).
            let later: Vec<(String, f32)> = dots[k + 1..].iter().filter(|d| d.0 == id).map(|d| (d.5.clone(), d.6)).collect();
            let set_ticks = |g: &mut Self, from: bool| {
                let Some(b) = g.bloons.iter_mut().find(|b| b.id == id) else { return };
                for (eid, prev) in &later {
                    let (a, z) = if from { (clock, *prev) } else { (*prev, clock) };
                    if let Some(EffectKind::Dot { tick, .. }) = b.effects.iter_mut().map(|e| (e.id == *eid, &mut e.kind)).find(|(m, k)| *m && matches!(k, EffectKind::Dot { tick, .. } if *tick == a)).map(|(_, k)| k) {
                        *tick = z;
                    }
                }
            };
            set_ticks(self, true);
            let Some(i) = self.bloons.iter().position(|b| b.id == id) else { continue };
            if immune & self.bloons[i].properties(&self.defs[self.bloons[i].kind]) != 0 {
                continue;
            }
            let kind0 = self.bloons[i].kind;
            let tally0 = self.damage_tally;
            self.pop_after_move = true;
            self.distribute = distribute;
            let n = self.damage(i, damage.round() as i32, &mut kids);
            (self.pop_after_move, self.distribute) = (false, (true, false));
            if self.bloons.iter().find(|b| b.id == id).is_some_and(|b| b.kind != kind0) || !self.bloons.iter().any(|b| b.id == id) {
                degraded.push(id);
            } else {
                set_ticks(self, false);
            }
            let self_tally_dealt = self.damage_tally - tally0;
            if let Some(tw) = self.towers.iter_mut().find(|t| t.id == owner) {
                tw.pops += n;
                tw.sun_destroyed(self.last_destroyed);
                tw.damage += self_tally_dealt;
                self.stats.pops += n;
                self.stats.damage += self_tally_dealt;
            }
        }
        // Process-time spawns of this frame's projectiles (after the towers acted: an exhaust
        // burst starts from its tower's rotation now).
        self.interval_shots();
        // RetargetOnContact.Process (VERIFIED-DISASM @0xcf6c20): delayFrames after the last
        // collision, which resets startFrame each hit, this frame's included since Simulate
        // collides everything before Process (recording boomerang_300: hits at 2309..2318 turn
        // the glaive only at 2321).
        for pr in &mut self.projectiles {
            if let (Some(s0), Some(rt)) = (pr.retarget_at, pr.spec.retarget.as_ref()) {
                if s0 + ((rt.delay * FPS) as f64).round_ties_even() as u64 <= sim_now {
                    pr.retarget_at = None;
                    due_retargets.push(pr.uid);
                }
            }
        }
        // Delayed bounces (RetargetOnContact.Process, before the shot travels): aimed where the
        // bloons made before the shot are after moving, the newer ones where they began; the
        // frame's straight move is redone along the new heading.
        for uid in std::mem::take(&mut due_retargets) {
            let Some(p) = self.projectiles.iter().position(|pr| pr.uid == uid) else { continue };
            let Some(rt) = self.projectiles[p].spec.retarget.clone() else { continue };
            let old = self.projectiles[p].vel;
            self.retarget(p, &rt);
            let pr = &mut self.projectiles[p];
            if pr.frames_left <= 0.0 {
                // No target: it expires there, before travelling.
                self.projectiles.remove(p);
                continue;
            }
            if pr.vel != old {
                pr.pos = [pr.prev[0] + pr.vel[0], pr.prev[1] + pr.vel[1]];
            }
        }
        // (A blast on a dead Target's object goes after this frame's spawn, which may take it over:
        // recording sub_032, on the Zebra entering that frame.)
        let (stale_blasts, exhausted): (Vec<_>, Vec<_>) = exhausted.into_iter().partition(|e| e.2.stale_target.is_some());
        self.spawn_exhausted(exhausted);
        for (spec, pos, parent, rot, parent_target) in subs {
            let made = self.towers.len();
            self.spawn_sub(spec, pos, parent, false);
            // CreateTower.Exhausted (VERIFIED-DISASM @0xcaef70, +0x3f useParentTargetPriority):
            // Tower.SetTargetType from the live tower that emitted it (recording
            // engineer_502_strong: a Strong Engineer's Sentry targets Strong).
            if let (true, true, Some(m)) = (parent_target, self.towers.len() > made, self.towers.iter().find(|t| t.id == parent).map(|t| t.target)) {
                self.towers[made].target = m;
            }
            // CreateTowerModel.useProjectileRotation: the tower takes the projectile's rotation,
            // its turret aiming relative to it (RotateToTarget.ApplyRotation stores rotation -
            // the tower's; recording engineer_200: a Sentry at 97.260574 fires at -55.11087).
            if let (Some(r), Some(t)) = (rot, self.towers.len().checked_sub(1)) {
                self.towers[t].base_rot = r;
                self.towers[t].sub_base = Some(r);
            }
        }
        for (pos, dir, c, tiers, owner, skip, z) in contacts {
            let start = self.projectiles.len();
            self.emit_z = z;
            self.emit(pos, dir, &c.emission, &c.projectile, None, tiers, owner);
            self.emit_z = 0.0;
            if let Some(id) = skip {
                for pr in &mut self.projectiles[start..] {
                    pr.hit.push(id);
                }
            }
        }
        // The Spawner's emissions for this frame (see the top of step). Made after this frame's
        // movement, they wait for the next frame like before. Their process-list place is the
        // Spawner's, before the towers (recording boomerang_400: a round's Black has a lower id
        // than the glaive thrown that frame, and has moved when the glaive aims at it).
        self.phase = 0;
        for (k, path) in std::mem::take(&mut self.frame_spawns) {
            self.emit_frame_spawn(k, path);
        }
        self.spawn_exhausted(stale_blasts);
        // (The bird's process is after the bloons': recording beast_001.)
        self.fly_birds();
        self.hold_bird_bloons();
        // ChurchillBaseRotation.Process (VERIFIED-DISASM @0xd26830), at the frame's end (what the turret's aim and the exhausts read is the
        // base as the frame began: recording dart_paragon_r40, rings at rt 216 / 217):
        // unless paused, the base takes the turret's angle as its target and turns
        // Math.MoveTowardsAngle 0.5 a frame, the turret with it; on reaching it, or with no bloon
        // out (BloonManager.GetGlobalBest), it pauses, and looks again once 240 frames
        // (MAX_PAUSE_TIME) have passed and there is a bloon (recording dart_paragon_r40).
        let any_bloon = !self.bloons.is_empty();
        for t in 0..self.towers.len() {
            if !self.tier(t).is_some_and(|x| x.base_rotation) {
                continue;
            }
            let now = self.sim_elapsed;
            let tw = &mut self.towers[t];
            if tw.base_state.0 {
                if now > tw.base_state.1 + 240 && any_bloon {
                    tw.base_state.0 = false;
                }
                continue;
            }
            if !tw.base_state.2 {
                let Some(disp) = tw.game_rot else { continue };
                if disp == tw.base_rot {
                    continue;
                }
                tw.base_state.2 = true;
                tw.base_state.3 = disp;
            }
            let target = tw.base_state.3;
            tw.base_rot = move_towards_angle(tw.base_rot, target, 0.5);
            tw.game_rot = Some(tw.base_rot + tw.turret_rel);
            if tw.base_rot == target || !any_bloon {
                tw.base_state = (true, now, false, target);
            }
        }
        // Grow ticks in Bloon.Process, after the frame's collisions.
        // Round end. Apopalypse: once the round has emitted everything (the last round still
        // waits for the bloons, for the win). STAND-IN P-APOP: our rounds don't overlap, so the
        // next round's emissions start then rather than running alongside.
        let last = self.round + 1 >= self.total_rounds();
        let emitted = self.apopalypse && !last;
        if self.round_active && self.pending.is_empty() && (self.bloons.is_empty() || emitted) {
            self.round_active = false;
            // TargetTrack's round events (OnEarlyRoundEnd / OnRoundSet): GeneratePoints.
            for tw in &mut self.towers {
                tw.necro_points = true;
            }
            self.finish_round();
        }
        self.in_step = false;
    }
    /// Simulation.AreRoundsActive (VERIFIED-DISASM @0xa75170): the map's spawner condition first
    /// (sandbox: always; time runs there whether or not anything was sent), else a round is on or
    /// any bloon exists.
    pub fn rounds_active(&self) -> bool {
        self.sandbox || self.round_active || !self.bloons.is_empty()
    }
    fn finish_round(&mut self){
        self.logic_round_end();
            // BurstWeaponBehavior.OnRoundEnd (VERIFIED-DISASM @0xc75370): lastEmission = now -
            // intervalFrames on the weapon's clock and the burst full again (recording
            // desperado_001: the round's last single shot isn't followed up; the next round opens
            // with a pair).
            for t in 0..self.towers.len() {
                let Some(tier) = self.tier(t) else { continue };
                // (Each weapon by its place: the tier's weapons come first in the tower's list, and
                // several may share a name, as Desperado's pistol and shotgun "Weapon"; recording
                // desperado_005 rt 91: the shotgun 106 - 12 = 94 on its Simulation.time clock.)
                let bursts: Vec<Option<(bool, crate::data::BurstFire)>> = tier
                    .attacks
                    .iter()
                    .flat_map(|a| a.weapons.iter())
                    .map(|w| w.burst.clone().filter(|b| !b.over_time).map(|b| (w.between_rounds, b)))
                    .collect();
                let (steps, rc) = (self.sim_elapsed as i64, self.round_clock);
                for (st, bw) in self.towers[t].weapons.iter_mut().zip(bursts.iter()) {
                    if let Some((between, b)) = bw {
                        st.last = if *between { steps } else { rc } - b.frames as i64;
                        st.burst_left = b.count;
                    }
                }
            }
            // The round's end flushes the factories too (VERIFIED by recording: Path.bloonsList
            // drops its dead entries that frame).
            self.plist_flush();
            self.fact_flush();
            // Simulation.RoundEnd -> DistributeXp(completed round index): heroes get the round's
            // XP (DistributeXpToHeroes), the towers' share it (InputManager.DistributeXp).
            let xp = Self::xp_for_round(self.round as i32) * self.map_difficulty.map_or(1.0, |m| TRACK_DIFFICULTY_XP_SCALE[m.min(3)]);
            for t in 0..self.towers.len() {
                if self.hero_def(self.towers[t].def).is_some() {
                    let scale = self.submerged_scale(self.towers[t].pos, |s| (s.hero_xp, 1.0));
                    self.add_hero_xp(t, xp * scale);
                }
            }
            self.distribute_tower_xp(self.round, xp);
            self.round += 1;
            // HYPOTHESIS P-ROUNDS: projectiles are cleared at round end unless AgeModel.rounds
            // says they survive more round ends (EndOfRoundClearBypassModel is Apopalypse-only:
            // there nothing is cleared).
            if !self.apopalypse {
                self.projectiles.retain_mut(|pr| {
                    let keep = pr.rounds_left > 0;
                    pr.rounds_left = pr.rounds_left.saturating_sub(1);
                    keep
                });
            }
            self.proj_flush();
            // HYPOTHESIS P-BANK: interest is added at round end, capped at the capacity.
            for t in 0..self.towers.len() {
                if let Some(bank) = self.tier(t).and_then(|tr| tr.bank.clone()) {
                    let tw = &mut self.towers[t];
                    tw.bank = (tw.bank * (1.0 + bank.interest as f64)).min(bank.capacity as f64);
                    if bank.auto {
                        self.collect_bank(t);
                    }
                }
            }
            self.items_round_end();
            self.necro_decay();
            // CreateSoundOnEndOfRound (Rare Quincy action figure).
            for t in 0..self.towers.len() {
                let s = self.tier_sounds(t);
                self.sound(&s.round_end, None);
            }
            // BonusLivesPerRoundModel: lives back each round, up to a share above the start.
            for t in 0..self.towers.len() {
                if let Some((n, cap)) = self.tier(t).and_then(|tr| tr.bonus_lives) {
                    let cap = (self.start_lives * cap).floor();
                    if self.lives < cap {
                        let before = self.lives;
                        self.lives = (self.lives + n).min(cap);
                        self.towers[t].lives_gained += self.lives - before;
                    }
                }
            }
            // Abilities with activateOnRoundEnd go off now. CashPerTowerInRange.Activate
            // (VERIFIED-DISASM @0xe3c330): baseCash + extraCashPerTier[tier] for each tower within
            // the tower's range + rangeIncrease whose base id is listed, Mathf.Min with
            // maxCashGeneration, x its multiplier (1 unless a BAD / boss skull changed it); then
            // BonusLivesOnAbility adds its lives. INFERRED: towers in range by their centres.
            for t in 0..self.towers.len() {
                let Some(tier) = self.tier(t) else { continue };
                let abilities: Vec<crate::data::Ability> = tier.abilities.iter().filter(|a| a.round_end).cloned().collect();
                for a in abilities {
                    if let Some(b) = &a.bounty {
                        if b.base != 0.0 || b.per_tier.iter().any(|&x| x != 0.0) {
                            let (pos, range) = (self.towers[t].pos, self.tier(t).map_or(self.tower_defs[self.towers[t].def].1.range, |tr| tr.range) + b.range_add);
                            let mut cash = b.base;
                            for o in 0..self.towers.len() {
                                let ot = &self.towers[o];
                                let d = (ot.pos[0] - pos[0]).hypot(ot.pos[1] - pos[1]);
                                if d <= range && b.towers.iter().any(|id| *id == self.tower_defs[ot.def].0) {
                                    let tier = *ot.tiers.iter().max().unwrap_or(&0) as usize;
                                    cash += b.per_tier.get(tier).copied().unwrap_or(0.0);
                                }
                            }
                            let v = cash.min(b.max) as f64;
                            self.earn(v);
                            self.towers[t].cash_earned += v;
                            self.cash_pops.push((self.towers[t].pos, v));
                        }
                    }
                    if a.lives != 0.0 {
                        self.lives += a.lives;
                        self.towers[t].lives_gained += a.lives;
                    }
                }
            }
            // PerRoundCashBonusTowerModel: each tower's per-round cash, shown over it.
            for t in 0..self.towers.len() {
                let v = self.tier(t).map_or(0.0, |tr| tr.round_cash) * self.buffs.get(t).map_or(1.0, |b| b.income_mult) as f64;
                if v > 0.0 {
                    if self.coop_wallets.is_some(){
                        let shared=self.tier(t).map_or(0.0,|tr|tr.round_cash_shared)*self.buffs.get(t).map_or(1.0,|b|b.income_mult) as f64;
                        self.earn_for(self.towers[t].id,v-shared);self.earn(shared);
                    }else{self.earn(v);}
                    self.towers[t].cash_earned += v;
                    self.cash_pops.push((self.towers[t].pos, v));
                }
            }
            // HYPOTHESIS P-TRADEEMPIRE: Trade Empire pays cashPerRoundPerMechantship per other
            // merchant ship (up to maxMerchantmanCapBonus) and cashPerRoundPerFavouredTrades per
            // Favored Trades ship, once per round however many Trade Empires there are.
            let te = (0..self.towers.len()).find_map(|t| self.tier(t).and_then(|tr| tr.trade_empire.clone()).map(|x| (t, x)));
            if let Some((te_t, te)) = te {
                let ships = (0..self.towers.len()).filter(|&o| o != te_t && self.tier(o).is_some_and(|tr| tr.merchant)).count().min(te.cap as usize);
                let favoured = (0..self.towers.len()).filter(|&o| o != te_t && self.tier(o).is_some_and(|tr| tr.cashback.is_some())).count();
                let v = ships as f64 * te.per_ship + favoured as f64 * te.per_favoured;
                if v > 0.0 {
                    self.earn_for(self.towers[te_t].id,v);
                    self.cash_pops.push((self.towers[te_t].pos, v));
                }
            }
            self.expire_subs(true);
            // BonusCashPerRoundModModel: baseCash + roundMultiple x round (none in CHIMPS).
            if let Some([base, per]) = self.round_bonus {
                self.earn(base + per * self.round as f64);
            }
            if self.round == self.end_round && !self.freeplay {
                self.won = true;
                self.sound(&["VictoryCampaign".to_string()], None);
            } else if self.apopalypse {
                self.start_round();
            }
    }

}

#[cfg(test)]
pub(crate) mod tests_support {
    pub use super::*;

    #[test]fn render_preparation_is_read_only_and_covers_round_descendants(){
        let mut g=game();g.round=139;g.start_round();
        let before=g.coop_digest();let models=g.upcoming_bloon_models();
        assert_eq!(before,g.coop_digest());assert!(!models.is_empty());
        assert!(models.windows(2).all(|w|w[0]<w[1]));
        let mut roots:Vec<_>=g.pending.iter().map(|p|p.1).collect();
        let mut seen=vec![false;g.defs.len()];
        while let Some(kind)=roots.pop(){
            if seen[kind]{continue;}seen[kind]=true;
            for model in g.defs[kind].model.iter().chain(g.defs[kind].damage_models.iter().map(|m|&m.model)){
                assert!(models.binary_search(model).is_ok(),"missing {model}");
            }
            roots.extend(g.child_kinds[kind].iter().copied());
        }
    }

    fn entity_script(source:&str)->crate::mod_runtime::Runtime{
        crate::mod_runtime::Runtime::new(vec![crate::mod_runtime::Definition{
            id:"entity-tests".into(),source:source.into(),tick_interval:60,
        }])
    }
    #[test]
    #[ignore="requires locally extracted game data"]
    fn mod_entity_damage_intercepts_once_and_preserves_children(){
        let mut g=game();
        g.mod_runtime=entity_script("fn before_damage(ctx) { #{damage:ctx.event.damage*2} }");
        g.spawn(g.by_name["Green"],100.0);
        g.mod_damage_bloon(g.bloons[0].id,None,1);
        assert_eq!(g.popped,2,"overflow must not be multiplied a second time");
        assert_eq!(g.names[g.bloons[0].kind],"Red");
        g.mod_runtime=entity_script("fn before_damage(ctx) { #{cancel:true} }");
        let (id,hp)=(g.bloons[0].id,g.bloons[0].hp);
        g.mod_damage_bloon(id,None,100);
        assert_eq!(g.bloons[0].hp,hp);
        assert_eq!(g.popped,2);
        g.mod_runtime=entity_script("fn before_damage(ctx) { #{damage:-1} }");
        g.mod_damage_bloon(id,None,1);
        assert!(g.bloons.is_empty(),"invalid decision must fall back to original damage");
        assert_eq!(g.mod_runtime.intercepts,0,"failed hooks must leave the hot path");
    }
    #[test]
    #[ignore="requires locally extracted game data"]
    fn mod_entity_fire_and_purchase_hooks(){
        let mut g=game();
        let def=g.tower_defs.iter().position(|(n,_)|n=="DartMonkey").unwrap();
        let t=place_near(&mut g,def,300.0);let owner=g.towers[t].id;
        let w=g.tier(t).unwrap().attacks[0].weapons[0].clone();
        g.mod_runtime=entity_script("fn before_fire(ctx) { #{count:3,damage:7,speed:10} }");
        g.emit([0.0,0.0],0.0,&w.emission,&w.projectile,None,[0,0,0],owner);
        assert_eq!(g.projectiles.len(),3);
        assert!(g.projectiles.iter().all(|p|p.spec.damage==7.0&&p.spec.speed==10.0));
        g.mod_runtime=entity_script("fn before_fire(ctx) { #{cancel:true} } fn before_sell(ctx) { #{cancel:true} } fn before_upgrade(ctx) { #{cancel:true} } fn before_place(ctx) { #{cancel:true} }");
        g.emit([0.0,0.0],0.0,&w.emission,&w.projectile,None,[0,0,0],owner);
        assert_eq!(g.projectiles.len(),3);
        let cash=g.cash;g.sell(t);assert_eq!(g.towers.len(),1);
        assert!(!g.upgrade(t,0));assert_eq!(g.cash,cash);
        assert!(!g.place(def,[100.0,100.0]));assert_eq!(g.towers.len(),1);
    }
    #[test]
    #[ignore="requires locally extracted game data"]
    fn mod_entity_queries_and_stable_patches(){
        let mut g=game();
        let def=g.tower_defs.iter().position(|(n,_)|n=="DartMonkey").unwrap();
        let t=place_near(&mut g,def,300.0);
        g.mod_runtime=entity_script(r#"
            fn on_new_game(ctx) { [#{op:"inspect",query:#{kind:"towers",limit:1,tag:"target"}}] }
            fn on_query(ctx) { [#{op:"entity_patch",patch:#{kind:"towers",id:ctx.event.items[0].id,fields:#{target:"Strong"}}}] }
        "#);
        g.mod_event("on_new_game",None);g.mod_entity_commands();
        assert_eq!(g.towers[t].target,TargetMode::Strong);
        g.spawn(g.by_name["Red"],100.0);
        let b=&g.bloons[0];let id=b.id;let layer=b.layer;
        let patch=crate::mod_entities::Patch{kind:crate::mod_entities::Kind::Bloons,id,layer:Some(layer+1),
            fields:crate::mod_entities::Fields{hp:Some(10),..Default::default()}};
        patch.apply(&mut g);assert_eq!(g.bloons[0].hp,1);
        let patch=crate::mod_entities::Patch{layer:Some(layer),..patch};
        patch.apply(&mut g);assert_eq!(g.bloons[0].hp,10);
        let query=crate::mod_entities::Query{kind:crate::mod_entities::Kind::Towers,offset:999,limit:128,tag:"end".into()};
        assert_eq!(query.snapshot(&g)["items"],serde_json::json!([]));
    }

    pub fn game() -> Game {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).expect("game.json")).unwrap();
        // RRBTD6_TEST_MODE picks the difficulty (default Medium).
        d.mode = std::env::var("RRBTD6_TEST_MODE").unwrap_or_default();
        let mut g = Game::new(d);
        g.cash = 1e9;
        g
    }

    /// Places tower `def` somewhere near track distance `d`.
    pub fn place_near(g: &mut Game, def: usize, d: f32) -> usize {
        let c = g.track.pos(d);
        for r in [24.0f32, 34.0, 44.0] {
            for k in 0..24 {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                if g.place(def, [c[0] + r * a.cos(), c[1] + r * a.sin()]) {
                    // Pets / drones are spawned after the tower itself.
                    return g.towers.iter().rposition(|t| t.sub.is_none()).unwrap();
                }
            }
        }
        panic!("no spot near {d}");
    }
}

#[cfg(test)]
mod sim_profile {
    thread_local! {pub static TIMES:std::cell::Cell<[f64;6]>=const{std::cell::Cell::new([0.0;6])};}
    pub struct Timer{start:std::time::Instant,index:usize}
    impl Timer{pub fn new(index:usize)->Self{Self{start:std::time::Instant::now(),index}}}
    impl Drop for Timer{fn drop(&mut self){TIMES.with(|v|{let mut t=v.get();t[self.index]+=self.start.elapsed().as_secs_f64();v.set(t);});}}
}

#[cfg(test)]
mod tests {
    use super::tests_support::*;

    /// Late-game sim load, headless: ~20 tier-5 towers along the track and a flood of Ceramics
    /// (4 a frame up to `RRBTD6_BENCH_BLOONS`, default 6000). Prints the average and worst
    /// `Game::step` time. `cargo test --profile game -p btd6-game sim_bench -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn sim_bench() {
        let rate: usize = std::env::var("RRBTD6_BENCH_RATE").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
        let cap: usize = std::env::var("RRBTD6_BENCH_BLOONS").ok().and_then(|v| v.parse().ok()).unwrap_or(6000);
        late_game_load(rate, cap, 3000, true);
    }

    /// Performance guard, run with every `cargo test --profile game`: the late-game load must
    /// stay within budget so new content doesn't quietly slow the sim down. Skipped in debug
    /// builds (unoptimized timings mean nothing). Budgets are ~3x today's numbers on the dev
    /// machine (0.85 ms avg / 1.8 ms worst at ~2.3k bloons, 2026-10-23). Timing only means
    /// something with the machine to itself, so it runs alone:
    /// `cargo test --profile game -p btd6-game perf_budget -- --ignored`.
    #[test]
    #[ignore = "timing; run alone"]
    fn perf_budget() {
        if cfg!(debug_assertions) {
            return;
        }
        // Bloons spawn just off the map edge (Path.spawnDist), so the flood is in tower range
        // from its first frames: a heavier load than when they started at the path's far end.
        let (avg, worst) = late_game_load(4, 6000, 1800, false);
        eprintln!("perf_budget: direct_native_step={} avg={:.3}ms worst={:.3}ms",std::env::var_os("RRBTD6_NATIVE_STEP").is_some(),avg*1000.0,worst*1000.0);
        // Hits now follow the collision grid's query order (every reachable bloon is ordered,
        // not the first few in storage order): a little more work per projectile.
        assert!(avg < 2.6e-3, "sim step avg {:.3} ms over the 2.6 ms budget", avg * 1e3);
        // A frame is 16.7 ms; the worst steps here pop 1000-1900 layers at once.
        assert!(worst < 10e-3, "sim step worst {:.3} ms over the 10 ms budget", worst * 1e3);
    }

    #[test]
    fn stack_tier_keys_preserve_model_names(){
        for a in [0,1,5,9,10,20,99,100,255]{
            for b in [0,1,5,9,10,20,99,100,255]{
                for c in [0,1,5,9,10,20,99,100,255]{
                    assert_eq!(super::tier_model_key([a,b,c],&mut [0;9]),format!("{a}{b}{c}"));
                }
            }
        }
    }

    fn late_game_scenario()->(super::Game,usize){
        let mut g = game();
        g.cash = 1e12;
        let plan: [(&str, [u8; 3]); 10] = [
            ("DartMonkey", [0, 2, 5]), ("TackShooter", [5, 0, 2]), ("BombShooter", [2, 0, 5]), ("NinjaMonkey", [5, 0, 2]),
            ("WizardMonkey", [0, 5, 2]), ("SuperMonkey", [2, 0, 5]), ("BoomerangMonkey", [5, 2, 0]), ("IceMonkey", [2, 0, 5]),
            ("SniperMonkey", [0, 2, 5]), ("Druid", [5, 0, 2]),
        ];
        let len = g.track.length();
        let mut k = 0;
        for i in 0..20 {
            let (name, tiers) = plan[i % plan.len()];
            let def = g.tower_defs.iter().position(|(n, _)| n == name).unwrap();
            let d = len * (0.1 + 0.8 * i as f32 / 20.0);
            let c = g.track.pos(d);
            'spot: for r in [24.0f32, 34.0, 44.0, 60.0] {
                for a in 0..24 {
                    let a = a as f32 / 24.0 * std::f32::consts::TAU;
                    if g.place(def, [c[0] + r * a.cos(), c[1] + r * a.sin()]) {
                        let t = g.towers.iter().rposition(|t| t.sub.is_none()).unwrap();
                        g.towers[t].tiers = tiers;
                        k += 1;
                        break 'spot;
                    }
                }
            }
        }
        g.round = 30;
        g.start_round();
        (g,k)
    }

    fn simulation_signature(g:&super::Game)->u64{
        use std::hash::{Hash,Hasher};
        let mut h=std::collections::hash_map::DefaultHasher::new();
        (g.popped,g.cash.to_bits(),g.lives.to_bits(),g.round,g.round_frame,g.next_id,g.rng).hash(&mut h);
        for b in &g.bloons{
            (b.id,b.kind,b.hp,b.born,b.order,b.path,b.dist.to_bits(),b.pos.map(f32::to_bits),b.fresh).hash(&mut h);
            for e in &b.effects{(e.id.as_str(),e.frames_left.to_bits(),e.owner).hash(&mut h);}
        }
        for p in &g.projectiles{
            (p.uid,p.owner,p.born,p.pos.map(f32::to_bits),p.vel.map(f32::to_bits),p.pierce.to_bits(),p.frames_left.to_bits(),p.target,p.spec.damage.to_bits()).hash(&mut h);
            p.hit.small.hash(&mut h);let mut hits:Vec<_>=p.hit.big.iter().copied().collect();hits.sort_unstable();hits.hash(&mut h);
        }
        for t in &g.towers{
            (t.id,t.tiers,t.pops,t.angle.to_bits()).hash(&mut h);
            for w in &t.weapons{(w.last,w.anim,w.rate_frames,w.aim_id,w.aim_at.map(f32::to_bits),w.aim_off.map(f32::to_bits),w.aim_circle,w.emits,w.crit).hash(&mut h);}
        }
        h.finish()
    }

    /// The owner's stress test: one degree-20 Dart Paragon, then the sandbox sends 10,000 ZOMGs with
    /// no spacing. Prints step times and where they go every 100 frames.
    /// `cargo test --profile game -p btd6-game paragon_zomg_bench -- --ignored --nocapture`
    #[test]
    #[ignore = "timing; run alone"]
    fn paragon_zomg_bench() {
        let mut g = game();
        crate::round_bench::setup(&mut g, 98, 20);
        g.towers.retain(|t| t.paragon.is_some());
        g.sandbox = true;
        let count: u32 = std::env::var("RRBTD6_BENCH_COUNT").ok().and_then(|v| v.parse().ok()).unwrap_or(10000);
        g.sandbox_spawn(&std::env::var("RRBTD6_BENCH_BLOON").unwrap_or_else(|_| "Zomg".into()), false, false, false, count, 0.0);
        let frames: usize = std::env::var("RRBTD6_BENCH_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(1500);
        super::sim_profile::TIMES.with(|v| v.set([0.0; 6]));
        let (mut sum, mut worst) = (0.0f64, 0.0f64);
        for f in 0..frames {
            let t0 = std::time::Instant::now();
            g.step();
            let dt = t0.elapsed().as_secs_f64();
            sum += dt;
            worst = worst.max(dt);
            if (f + 1) % 100 == 0 {
                let d = super::sim_profile::TIMES.with(|v| v.get());
                eprintln!("frames {}..{}: step avg {:.2} ms worst {:.2} ms, bloons {}, projectiles {}, pops {} | towers {:.0} ms (target {:.0}) projectiles {:.0} ms (closest {:.0} damage {:.0})",
                    f - 98, f + 1, sum / 100.0 * 1e3, worst * 1e3, g.bloons.len(), g.projectiles.len(), g.popped,
                    g.prof[1] * 1e3, g.perf_detail[2] * 1e3, g.prof[2] * 1e3, d[1] * 1e3, d[0] * 1e3);
                (sum, worst) = (0.0, 0.0);
                g.prof = [0.0; 3];
                g.perf_detail = [0.0; 3];
                super::sim_profile::TIMES.with(|v| v.set([0.0; 6]));
            }
        }
    }

    /// RRBTD6_SIGNATURE_TRACE=<file>: a heavy mixed load (Ceramic flood, MOAB-class, camo, regrow,
    /// every target mode, a blocker switched off and on) with `simulation_signature` written every
    /// frame, to diff a speed-only change against the code before it.
    #[test]
    #[ignore="requires locally extracted game data"]
    fn signature_trace(){
        let Some(out)=std::env::var_os("RRBTD6_SIGNATURE_TRACE") else {return};
        let frames:usize=std::env::var("RRBTD6_SIGNATURE_FRAMES").ok().and_then(|v|v.parse().ok()).unwrap_or(2400);
        let (mut g,_)=late_game_scenario();
        // The clock seeds freeplay_seed (and the synced random from it): a fixed one repeats.
        g.freeplay_seed=12345;g.synced=None;
        let modes=[super::TargetMode::First,super::TargetMode::Last,super::TargetMode::Close,super::TargetMode::Strong];
        for (i,t) in g.towers.iter_mut().enumerate(){t.target=modes[i%4];t.camo_prio=i%3==0;}
        g.areas.push(serde_json::from_value(serde_json::json!({"type":3,"points":[[-20.0,-20.0],[-20.0,20.0],[20.0,20.0],[20.0,-20.0]],"height":50.0,"isBlocker":true})).unwrap());
        let mut lines=String::new();
        for frame in 0..frames{
            if frame==600 {g.areas.last_mut().unwrap().is_disabled=true;}
            if frame==1200 {g.areas.last_mut().unwrap().is_disabled=false;}
            if g.bloons.len()<8000{g.debug_spawn("Ceramic",6,0.0);}
            if frame%60==0 {g.debug_spawn("Moab",1,0.0);g.debug_spawn("RedCamo",2,0.0);g.debug_spawn("RainbowRegrow",1,0.0);}
            if frame%240==0 {g.debug_spawn("Zomg",1,0.0);g.debug_spawn("Ddt",1,0.0);}
            g.step();
            lines.push_str(&format!("{frame} {} {}
",g.bloons.len(),simulation_signature(&g)));
        }
        std::fs::write(out,lines).unwrap();
    }

    #[test]
    #[ignore="requires locally extracted game data"]
    fn target_segment_cache_preserves_simulation(){
        let (mut cached,_)=late_game_scenario();
        let (mut reference,_)=late_game_scenario();reference.reference_targeting=true;
        for g in [&mut cached,&mut reference]{
            for (i,t) in g.towers.iter_mut().enumerate(){t.target=[super::TargetMode::First,super::TargetMode::Last,super::TargetMode::Close,super::TargetMode::Strong][i%4];}
            g.areas.push(serde_json::from_value(serde_json::json!({"type":3,"points":[[-20.0,-20.0],[-20.0,20.0],[20.0,20.0],[20.0,-20.0]],"height":50.0,"isBlocker":true})).unwrap());
        }
        for frame in 0..1800 {
            for g in [&mut cached,&mut reference]{
                // Results must not survive changes to blockers between searches.
                if frame==600 {g.areas.last_mut().unwrap().is_disabled=true;}
                if frame==1200 {g.areas.last_mut().unwrap().is_disabled=false;}
                if g.bloons.len()<6000{g.debug_spawn("Ceramic",4,0.0);}
                // Exercise MOAB circles, camo and regrow with the same native emission order.
                if frame%120==0 {g.debug_spawn("Moab",1,0.0);g.debug_spawn("RedCamo",1,0.0);g.debug_spawn("RainbowRegrow",1,0.0);}
                g.step();
            }
            assert_eq!(simulation_signature(&cached),simulation_signature(&reference),"target cache diverged at frame {frame}");
        }
    }

    /// ~20 tier-5 towers and a Ceramic flood (`rate` a frame up to `cap` bloons) for `frames`
    /// steps; returns the average and worst step time (seconds) after the first 600.
    fn late_game_load(rate: usize, cap: usize, frames: usize, print: bool) -> (f64, f64) {
        let native_step=std::env::var_os("RRBTD6_NATIVE_STEP").is_some();
        let (mut g,k)=late_game_scenario();
        super::sim_profile::TIMES.with(|v|v.set([0.0;6]));
        g.reference_targeting=std::env::var_os("RRBTD6_REFERENCE_TARGETING").is_some();
        let (mut total, mut worst, mut n, mut peak) = (0.0f64, 0.0f64, 0u32, 0usize);
        for f in 0..frames {
            if g.bloons.len() < cap {
                g.debug_spawn(&std::env::var("RRBTD6_BENCH_BLOON").unwrap_or_else(|_| "Ceramic".into()), rate, 0.0);
            }
            let (pp, np) = (g.popped, g.projectiles.len());
            let detail_before=super::sim_profile::TIMES.with(|v|v.get());
            let t0 = std::time::Instant::now();
            if native_step{g.step_native();}else{g.step();}
            let dt = t0.elapsed().as_secs_f64();
            if print && dt > 0.008 {
                let d=super::sim_profile::TIMES.with(|v|v.get());
                eprintln!("sim_bench: slow collision detail frame {f}: damage {:.2}ms closest {:.2}ms remap {:.2}ms gather {:.2}ms spawn {:.2}ms destroy {:.2}ms",(d[0]-detail_before[0])*1e3,(d[1]-detail_before[1])*1e3,(d[2]-detail_before[2])*1e3,(d[3]-detail_before[3])*1e3,(d[4]-detail_before[4])*1e3,(d[5]-detail_before[5])*1e3);
            }
            if print && dt > 0.004 {
                eprintln!("sim_bench: slow frame {f}: {:.2} ms, {} pops, projectiles {np} -> {}, bloons {}", dt * 1e3, g.popped - pp, g.projectiles.len(), g.bloons.len());
            }
            if f >= 600 {
                total += dt;
                worst = worst.max(dt);
                n += 1;
            }
            peak = peak.max(g.bloons.len());
        }
        if print {
            let pops: u64 = g.towers.iter().map(|t| t.pops).sum();
            let detail=super::sim_profile::TIMES.with(|v|v.get());
            eprintln!("sim_bench: collision detail (ms total) damage {:.0} closest search {:.0} remap {:.0} gather {:.0} spawn {:.0} destroy {:.0}",detail[0]*1e3,detail[1]*1e3,detail[2]*1e3,detail[3]*1e3,detail[4]*1e3,detail[5]*1e3);
            eprintln!("sim_bench: tower detail (ms total) model clone {:.0} scheduling {:.0} target search {:.0}",g.perf_detail[0]*1e3,g.perf_detail[1]*1e3,g.perf_detail[2]*1e3);
            eprintln!("sim_bench: phases (ms total) effects+move {:.0} towers {:.0} projectiles {:.0}", g.prof[0] * 1e3, g.prof[1] * 1e3, g.prof[2] * 1e3);
            eprintln!("sim_bench: phase worst (ms) {:.2} {:.2} {:.2}", g.prof_max[0] * 1e3, g.prof_max[1] * 1e3, g.prof_max[2] * 1e3);
            eprintln!("sim_bench: {k} towers, peak {peak} bloons, {} projectiles at end, step avg {:.3} ms worst {:.3} ms, {pops} pops",
                g.projectiles.len(), total * 1000.0 / n as f64, worst * 1000.0);
        }
        (total / n as f64, worst)
    }

    #[test]
    fn sun_temple_sacrifices() {
        let mut g = game();
        let sm = g.tower_defs.iter().position(|(n, _)| n == "SuperMonkey").unwrap();
        let dart = g.tower_defs.iter().position(|(n, _)| n == "DartMonkey").unwrap();
        let t = place_near(&mut g, sm, 300.0);
        let c = g.towers[t].pos;
        // A tier-5 Dart Monkey right next to it (Primary, worth over 10000).
        let d = (0..48)
            .find_map(|k| {
                let a = k as f32 / 48.0 * std::f32::consts::TAU;
                g.place(dart, [c[0] + 30.0 * a.cos(), c[1] + 30.0 * a.sin()]).then(|| g.towers.len() - 1)
            })
            .expect("dart spot");
        while g.upgrade(d, 0) {}
        let dart_id = g.towers[d].id;
        assert!(g.towers[d].invested >= 10001.0);
        let t = g.towers.iter().position(|x| x.def == sm).unwrap();
        let base_attacks = {
            for _ in 0..3 {
                assert!(g.upgrade(t, 0));
            }
            g.tier(t).unwrap().attacks.len()
        };
        assert_eq!(g.temple_prompt(t, 0), Some(false), "Sun Temple asks first");
        assert_eq!(g.temple_prompt(t, 1), None);
        assert!(g.upgrade(t, 0), "Sun Temple");
        let t = g.towers.iter().position(|x| x.def == sm).unwrap();
        assert_eq!(g.temple_prompt(t, 0), Some(true), "True Sun God asks first");
        assert_eq!(g.temple_prompt(t, 1), None, "crosspath keeps the temple");
        assert!(g.towers.iter().all(|x| x.id != dart_id), "dart sacrificed");
        let t = g.towers.iter().position(|x| x.def == sm).unwrap();
        let (groups, _) = g.towers[t].temple.as_ref().expect("temple groups");
        assert!(groups.iter().any(|gr| gr.set == 1), "Primary groups earned");
        assert!(g.tier(t).unwrap().attacks.len() > base_attacks);
    }

    /// Every class sacrificed to both temples, then a round played (planes, avatar, blasts).
    #[test]
    fn true_sun_god_runs() {
        let mut g = game();
        let sm = g.tower_defs.iter().position(|(n, _)| n == "SuperMonkey").unwrap();
        let t = place_near(&mut g, sm, 300.0);
        let c = g.towers[t].pos;
        let near = |g: &mut Game, name: &str| {
            let d = g.tower_defs.iter().position(|(n, _)| n == name).unwrap();
            let i = (0..96).find_map(|k| {
                let r = 20.0 + (k / 24) as f32 * 10.0;
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                g.place(d, [c[0] + r * a.cos(), c[1] + r * a.sin()]).then(|| g.towers.len() - 1)
            });
            if let Some(i) = i {
                let i = g.towers.iter().rposition(|x| x.sub.is_none() && x.def == d).unwrap_or(i);
                while g.upgrade(i, 0) {}
            }
        };
        for _ in 0..3 {
            assert!(g.upgrade(t, 0));
        }
        for name in ["DartMonkey", "BombShooter", "WizardMonkey", "BananaFarm"] {
            near(&mut g, name);
        }
        let t = g.towers.iter().position(|x| x.def == sm && x.sub.is_none()).unwrap();
        assert!(g.upgrade(t, 0), "Sun Temple");
        for name in ["TackShooter", "MonkeyAce", "Alchemist", "MonkeyVillage"] {
            near(&mut g, name);
        }
        let t = g.towers.iter().position(|x| x.def == sm && x.sub.is_none()).unwrap();
        assert!(g.upgrade(t, 0), "True Sun God");
        g.round = 39;
        g.start_round();
        for _ in 0..3000 {
            g.step();
        }
    }

    /// Past the round set, freeplay rounds come from the game's groups: the same seed gives the
    /// same rounds, later rounds get bigger, and a round plays.
    #[test]
    fn sandbox_round_selection_reaches_generated_rounds_and_reverts_scaling(){
        let mut g=game();g.sandbox=true;
        assert!(!g.freeplay_groups.is_empty());
        let ceramic=g.by_name["Ceramic"];
        let original_hp=g.defs[ceramic].max_health;
        let original_children=g.child_kinds[ceramic].clone();
        g.set_round(200);
        assert_eq!(g.round,199);assert!(g.freeplay&&g.round_active);assert!(!g.pending.is_empty());
        assert!(g.pre_super.is_some());assert!(g.defs[ceramic].max_health>original_hp);
        g.clear_bloons();g.set_round(1);
        assert_eq!(g.round,0);assert!(g.round_active);assert!(!g.won);
        assert!(g.pre_super.is_none());assert_eq!(g.defs[ceramic].max_health,original_hp);
        assert_eq!(g.child_kinds[ceramic],original_children);
        g.set_round(0);assert_eq!(g.round,0);
        // A custom round set without generated groups keeps its existing upper bound.
        g.freeplay_groups.clear();g.rounds.truncate(2);g.set_round(200);
        assert_eq!(g.round,1);assert!(g.round_active);
        // Large typed values must not overflow the generator's intermediate budget.
        let _=game().freeplay_round(999999);
    }
    #[test]
    fn sandbox_reset_clears_pops_and_damage_without_changing_game_resources(){
        let mut g=game();g.sandbox=true;
        let dart=g.tower_defs.iter().position(|(id,_)|id=="DartMonkey").unwrap();
        place_near(&mut g,dart,300.0);place_near(&mut g,dart,450.0);
        for t in &mut g.towers{t.pops=123;t.damage=456;}
        let before=(g.cash,g.lives,g.round,g.towers.len());
        g.reset_damage();
        assert!(g.towers.iter().all(|t|t.pops==0&&t.damage==0));
        assert_eq!((g.cash,g.lives,g.round,g.towers.len()),before);
    }

    #[test]
    fn freeplay_rounds_generate() {
        let mut g = game();
        g.freeplay_seed = 12345;
        let a = g.freeplay_round(141);
        let b = g.freeplay_round(141);
        assert!(!a.is_empty());
        assert_eq!(a.iter().map(|x| (&x.bloon, x.count)).collect::<Vec<_>>(), b.iter().map(|x| (&x.bloon, x.count)).collect::<Vec<_>>());
        let size = |r: i32| g.freeplay_round(r).iter().map(|x| x.count).sum::<u32>();
        eprintln!("freeplay 141: {:?}", a.iter().map(|x| format!("{} x{}", x.bloon, x.count)).collect::<Vec<_>>());
        eprintln!("bloons per round 141 / 200 / 300: {} / {} / {}", size(141), size(200), size(300));
        g.start_freeplay();
        g.round = 140;
        g.start_round();
        assert!(g.round_active, "round 141 starts");
        for _ in 0..600 {
            g.step();
        }
        assert!(!g.bloons.is_empty() || g.round > 140);
    }

    /// ProgressiveDifficultyManager: nothing before round 81, MOAB health +2% a round from 81,
    /// +5% from 101; speed +2% a round from 81 plus its one-off jumps.
    #[test]
    fn late_round_scaling() {
        assert_eq!(progressive_health(80), 1.0);
        assert!((progressive_health(81) - 1.02).abs() < 1e-5);
        assert!((progressive_health(100) - 1.40).abs() < 1e-4);
        assert!((progressive_health(101) - 1.45).abs() < 1e-4);
        assert!((progressive_speed(100) - 1.40).abs() < 1e-4);
        assert!((progressive_speed(101) - (1.42 + 0.18)).abs() < 1e-4);
        let mut g = game();
        let moab = g.by_name["Moab"];
        let base = g.defs[moab].max_health;
        g.progressive_difficulty(101);
        assert_eq!(g.defs[moab].max_health, round_half_even(base as f32 * 1.45) as i32);
        g.progressive_difficulty(1);
        assert_eq!(g.defs[moab].max_health, base);
    }

    /// ChangeCeramicsAndChildren from round 81: Ceramics x6 health (60 / 120 fortified), every
    /// non-MOAB bloon keeps its first child, the Ceramic's total leak stays 104 / 114 when it
    /// changes and drops to 65 / 75 once the Rainbow after it in GameModel.bloons loses its
    /// second Zebra; a destroyed Ceramic still counts and pays its original 95 layers.
    #[test]
    fn super_ceramics() {
        let mut g = game();
        let k = |g: &Game, n: &str| g.by_name[n];
        let (cer, fcer, rain, zebra, moab) = (k(&g, "Ceramic"), k(&g, "CeramicFortified"), k(&g, "Rainbow"), k(&g, "Zebra"), k(&g, "Moab"));
        let (pops, cash, moab_leak) = (g.pops_in[cer], g.cash_in[cer], g.rbe[moab]);
        assert_eq!((g.defs[cer].max_health, g.rbe[cer], pops), (10, 104.0, 95));
        g.progressive_difficulty(80);
        assert_eq!(g.defs[cer].max_health, 10);
        g.progressive_difficulty(81);
        assert_eq!((g.defs[cer].max_health, g.defs[fcer].max_health), (60, 120));
        assert_eq!((g.rbe[cer], g.rbe[fcer], g.rbe[rain]), (65.0, 75.0, 8.0));
        assert_eq!(g.child_kinds[rain], vec![zebra]);
        assert_eq!(g.defs[zebra].children, vec!["Black".to_string()]);
        assert_eq!(g.child_kinds[moab].len(), 4);
        assert!(g.defs[cer].tags.iter().any(|t| t == "Super"));
        assert_eq!((g.pops_in[cer], g.cash_in[cer]), (pops, cash));
        assert_eq!(g.rbe[moab], 200.0 + 4.0 * 65.0);
        g.progressive_difficulty(82);
        assert_eq!(g.defs[cer].max_health, 60);
        g.progressive_difficulty(1);
        assert_eq!((g.defs[cer].max_health, g.rbe[cer], g.child_kinds[rain].len(), g.rbe[moab]), (10, 104.0, 2, moab_leak));
        assert!(!g.defs[cer].tags.iter().any(|t| t == "Super"));
    }

    /// Apopalypse: rounds come from the freeplay groups and follow one another without a break.
    #[test]
    fn apopalypse_rounds_run_on() {
        let mut g = game();
        g.apopalypse = true;
        g.start_round();
        assert!(g.round_active);
        for _ in 0..60 * 120 {
            g.step();
        }
        assert!(g.round >= 2, "next rounds were sent without waiting (round {})", g.round);
        assert!(g.round_active);
    }

    /// Regrow: a popped Rainbow Regrow's children grow back a layer every 3 s, up to Rainbow.
    #[test]
    fn regrow_grows_back_to_its_ceiling() {
        let mut g = game();
        let rainbow = g.by_name["RainbowRegrow"];
        g.spawn_on(rainbow, 50.0, 0);
        let hp = g.bloons[0].hp;
        let mut kids = Vec::new();
        g.damage(0, hp, &mut kids);
        assert!(kids.len() == 2 && g.bloons.iter().all(|b| g.defs[b.kind].layer_number == 7), "two Zebra Regrows");
        for b in &mut g.bloons {
            b.dist = 50.0; // keep them on the track
        }
        // (A step begins: the children aren't new any more.)
        g.step_first_id = g.next_id;
        for _ in 0..(3 * 60 + 2) {
            g.regrow();
        }
        assert!(g.bloons.iter().all(|b| b.kind == rainbow), "grew back to Rainbow Regrow");
        for _ in 0..(6 * 60) {
            g.regrow();
        }
        assert!(g.bloons.iter().all(|b| b.kind == rainbow), "not past the ceiling");
    }

    /// Total Transformation: nearby low-tier monkeys become the monster for 20 s, then go back.
    #[test]
    fn total_transformation_morphs_nearby_towers() {
        let mut g = game();
        let alch = g.tower_defs.iter().position(|(n, _)| n == "Alchemist").unwrap();
        let dart = g.tower_defs.iter().position(|(n, _)| n == "DartMonkey").unwrap();
        let a = place_near(&mut g, alch, 300.0);
        let c = g.towers[a].pos;
        let d = (0..48)
            .find_map(|k| {
                let ang = k as f32 / 48.0 * std::f32::consts::TAU;
                g.place(dart, [c[0] + 25.0 * ang.cos(), c[1] + 25.0 * ang.sin()]).then(|| g.towers.len() - 1)
            })
            .expect("dart spot");
        let dart_id = g.towers[d].id;
        let a = g.towers.iter().position(|x| x.def == alch).unwrap();
        for _ in 0..5 {
            assert!(g.upgrade(a, 1));
        }
        let name = g.tier(a).unwrap().abilities.iter().find(|x| x.morph.is_some()).expect("morph ability").name.clone();
        g.round = 29;
        g.start_round();
        g.towers[a].ability_cd.clear();
        assert!(g.activate_ability(&name));
        let d = g.towers.iter().position(|x| x.id == dart_id).unwrap();
        assert!(g.towers[d].morph.is_some(), "dart transformed");
        for _ in 0..(21 * 60) {
            g.step();
        }
        let d = g.towers.iter().position(|x| x.id == dart_id).unwrap();
        assert!(g.towers[d].morph.is_none(), "back after 20 s");
    }

    /// Replays SimRecorder recordings of the real game (tools/mod/README.md) in the clone and
    /// compares them frame by frame: every bloon's type and distance, and every weapon's throws.
    /// RRBTD6_REC names one recording; by default every research/dumps/recordings/*.rec.
    /// The game's map y axis is the clone's -y. Prints one summary line per recording.
    #[test]
    #[ignore]
    fn blocker_probe() {
        let g = game();
        // RRBTD6_PROBE=x,y (clone frame): another point.
        let q = std::env::var("RRBTD6_PROBE").ok().and_then(|v| v.split_once(',').and_then(|(a, b)| Some([a.parse().ok()?, b.parse().ok()?]))).unwrap_or([-156.13152f32, 13.558226]);
        let (qx, qy) = grid_cell(q);
        eprintln!("offsets {} cell {qx},{qy}", g.grid_offsets.len());
        for (dx, dy) in [(0, 0), (0, -1), (0, 1)] {
            let c = &g.blocker_grid[((qy + dy) * 40 + qx + dx) as usize];
            eprintln!("cell {},{} span {} {:?}", qx + dx, qy + dy, c.0, c.1);
        }
        eprintln!("first offsets {:?}", &g.grid_offsets[..g.grid_offsets.len().min(20)]);
        eprintln!("hit {}", g.blocker_hit(q, 0.0, 0.0));
        // Which blocker: the same walk as blocker_hit (radius 0).
        let reach = cell_span(0.0);
        let side = ((35.0 + 5.0) / GAME_GRID_CELL) as i32 * 2 + 1;
        let n = ((side * side) as usize).min(g.grid_offsets.len() / 2);
        for (k, w) in g.grid_offsets.chunks(2).take(n).enumerate() {
            let (x, y) = (qx + w[0], qy + w[1]);
            if x < 0 || y < 0 || x >= 40 || y >= 33 {
                continue;
            }
            let (span, list) = &g.blocker_grid[(y * 40 + x) as usize];
            let m = w[0].max(w[1]) - reach;
            if *span <= 0 || !(m as f32 <= 0.0 || m <= *span) {
                continue;
            }
            for b in list {
                if b[3] > 0.0 && (k == 0 || (b[0] - q[0]).powi(2) + (b[1] + q[1]).powi(2) <= b[2] * b[2]) {
                    eprintln!("blocked by {b:?} via offset {w:?} (k {k}) cell {x},{y} span {span}");
                }
            }
        }
    }

    /// Free Dart spots near the recording position (game coordinates), for SimRecorder scenarios.
    #[test]
    #[ignore]
    fn dart_spots() {
        let mut g = game();
        let dart = g.tower_defs.iter().position(|(n, _)| n == "DartMonkey").unwrap();
        let base = [-112.815414f32, 30.9];
        assert!(g.place(dart, base));
        let mut found = Vec::new();
        for r in [16.0f32, 20.0, 24.0, 28.0] {
            for k in 0..16 {
                let a = k as f32 / 16.0 * std::f32::consts::TAU;
                let q = [base[0] + r * a.cos(), base[1] + r * a.sin()];
                if found.len() < 3 && g.place(dart, q) {
                    found.push([q[0], -q[1]]);
                }
            }
        }
        eprintln!("spots (game x, y): {found:?}");
    }

    #[test]
    #[ignore]
    fn compare_recording() {
        let paths: Vec<String> = match std::env::var("RRBTD6_REC") {
            Ok(p) => vec![p],
            Err(_) => {
                // RRBTD6_REC_DIR: another folder (research/dumps/recordings/pending/<tower>).
                let dir = std::env::var("RRBTD6_REC_DIR").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../research/dumps/recordings").to_string());
                let mut v: Vec<String> = std::fs::read_dir(dir)
                    .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "rec")).map(|p| p.display().to_string()).collect())
                    .unwrap_or_default();
                v.sort();
                v
            }
        };
        let verbose = std::env::var("RRBTD6_REC_TRACE").is_ok();
        for path in paths {
            compare_one(&path, verbose);
        }
    }

    /// One recording: see `compare_recording`.
    fn compare_one(path: &str, verbose: bool) {
        let text = std::fs::read_to_string(path).expect("recording");
        let mut meta = std::collections::HashMap::new();
        let mut towers: Vec<(String, f32, f32)> = Vec::new();
        let mut targets: Vec<(usize, String)> = Vec::new();
        let mut ability_at: Option<usize> = None;
        // `# seed`: GameModel.randomSeed (freeplay rounds, the end screens' callout pick).
        let mut seed: Option<i32> = None;
        let mut map_difficulty: Option<usize> = None;
        // SimRecorder's `# camo k` (camo priority on) and `# upgrade k path frame`.
        let mut camos: Vec<usize> = Vec::new();
        let mut flips: Vec<usize> = Vec::new();
        let mut upgrades: Vec<(usize, usize, usize)> = Vec::new();
        // Per frame (roundTime): bloons (type, distance) and weapons (tower id, index, lastEmission).
        let mut frames: Vec<(i64, Vec<(String, f32)>, Vec<(u64, usize, i64)>)> = Vec::new();
        // Per frame: positions of bloons (0) and projectiles (1), in the game's frame.
        let mut places: Vec<Vec<(u8, u32, u32)>> = Vec::new();
        // Per frame: Path.bloonsList as (type, distance), when recorded.
        let mut lists: Vec<Option<Vec<(String, u32)>>> = Vec::new();
        let mut ids: std::collections::HashMap<u64, (String, u32)> = Default::default();
        let (mut first_time, mut times): (Option<i64>, Vec<i64>) = (None, Vec::new());
        let bits = |v: f32| (v + 0.0).to_bits();
        for line in text.lines() {
            let f: Vec<&str> = line.split_whitespace().collect();
            match f.first().copied() {
                Some("#") if f.get(1) == Some(&"SimRecorder") => {
                    for kv in &f[3..] {
                        if let Some((k, v)) = kv.split_once('=') {
                            meta.insert(k.to_string(), v.to_string());
                        }
                    }
                    // Older recordings don't name the mode: their scenario does (all "Clicks",
                    // CHIMPS: recording boomerang_paragon_r98 stops at the end of round 100).
                    if !meta.contains_key("mode") {
                        let sc = format!("{}/../tools/mod/scenarios/{}.json", env!("CARGO_MANIFEST_DIR"), f.get(2).copied().unwrap_or(""));
                        if let Some(m) = std::fs::read_to_string(sc).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()).and_then(|v| v["mode"].as_str().map(String::from)) {
                            meta.insert("mode".into(), m);
                        }
                    }
                }
                Some("#") if f.get(1) == Some(&"tower") => towers.push((f[2].into(), f[3].parse().unwrap(), f[4].parse().unwrap())),
                // SimRecorder inputs: a tower's targeting mode, and the frame abilities go off after.
                Some("#") if f.get(1) == Some(&"target") => targets.push((f[2].parse().unwrap(), f[3].to_string())),
                Some("#") if f.get(1) == Some(&"ability") => ability_at = f[2].parse().ok(),
                Some("#") if f.get(1) == Some(&"seed") => seed = f[2].parse().ok(),
                // `# xpscale <scales> map <mapDifficulty> ...`: the map's XP tier.
                Some("#") if f.get(1) == Some(&"xpscale") => map_difficulty = f.iter().position(|x| *x == "map").and_then(|i| f.get(i + 1)?.parse().ok()),
                Some("#") if f.get(1) == Some(&"camo") => camos.push(f[2].parse().unwrap()),
                Some("#") if f.get(1) == Some(&"flip") => flips.push(f[2].parse().unwrap()),
                Some("#") if f.get(1) == Some(&"upgrade") => upgrades.push((f[2].parse().unwrap(), f[3].parse().unwrap(), f[4].parse().unwrap())),
                Some("F") => {
                    if first_time.is_none() {
                        first_time = f[1].parse::<i64>().ok();
                    }
                    times.push(f[1].parse::<i64>().unwrap_or(0));
                    frames.push((f[2].parse().unwrap(), Vec::new(), Vec::new()));
                    places.push(Vec::new());
                    lists.push(None);
                    ids.clear();
                }
                Some("B") if !frames.is_empty() => {
                    frames.last_mut().unwrap().1.push((f[2].into(), f[3].parse().unwrap()));
                    ids.insert(f[1].parse().unwrap(), (f[2].to_string(), f[3].parse::<f32>().unwrap().to_bits()));
                    places.last_mut().unwrap().push((0, bits(f[4].parse().unwrap()), bits(f[5].parse().unwrap())));
                }
                Some("L") if !frames.is_empty() && f[1] == "0" => {
                    *lists.last_mut().unwrap() = Some(f[2..].iter().filter_map(|x| ids.get(&x.parse::<u64>().unwrap()).cloned()).collect());
                }
                Some("P") if !frames.is_empty() => places.last_mut().unwrap().push((1, bits(f[2].parse().unwrap()), bits(f[3].parse().unwrap()))),
                Some("W") if !frames.is_empty() => frames.last_mut().unwrap().2.push((f[1].parse().unwrap(), f[2].parse().unwrap(), f[3].parse().unwrap())),
                _ => {}
            }
        }
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        // Another map than the exported one (water towers on Logs): its own map.json.
        if let Some(m) = meta.get("map").filter(|m| !m.eq_ignore_ascii_case("Tutorial")) {
            let mp = format!("{}/../local_extracted/maps/{}/map.json", env!("CARGO_MANIFEST_DIR"), m.to_lowercase());
            d.map = serde_json::from_str(&std::fs::read_to_string(&mp).unwrap_or_else(|_| panic!("map {mp}"))).unwrap();
        }
        let (diff, mode) = (meta.get("difficulty").cloned().unwrap_or("Medium".into()), meta.get("mode").cloned().unwrap_or("Standard".into()));
        d.mode = match mode.as_str() {
            "Clicks" => "CHIMPS".into(),
            "Standard" => diff.clone(),
            m => format!("{diff} {m}"),
        };
        let mut g = Game::new(d);
        g.cash = 1e9;
        if let Some(s) = seed {
            g.freeplay_seed = s;
        }
        g.map_difficulty = map_difficulty;
        // The recorder raises the lives cap so leaks don't end the run (SimRecorder).
        g.lives = 1e6;
        // "DartMonkey-205": base tower and tiers.
        for (id, x, y) in &towers {
            let (base, tiers) = match id.split_once('-') {
                Some((b, t)) if t.len() == 3 => (b, t.bytes().map(|c| c - b'0').collect::<Vec<u8>>()),
                _ => (id.as_str(), vec![0, 0, 0]),
            };
            let def = g.tower_defs.iter().position(|(n, _)| n == base).unwrap();
            // InputManager.CreateTowerAt skips the mode's tower locks (recording farm_000: a
            // Banana Farm in CHIMPS).
            let locked = std::mem::take(&mut g.locked);
            let rng_before = g.synced.clone();
            assert!(g.place_at(def, [*x, -*y], false), "can't place {id} at {x},{}", -y);
            g.locked = locked;
            let t = g.towers.iter().rposition(|t| t.sub.is_none()).unwrap();
            g.towers[t].tiers = [tiers[0], tiers[1], tiers[2]];
            // (Made at this tier in one go: the base tier's shuffles didn't happen.)
            g.synced = rng_before;
            g.towers[t].randpos.clear();
            g.random_pos_init(t);
            // (The tier's permanent sub-towers, as an upgrade makes them: Lord Phoenix.)
            g.sync_pets(t);
            // SwitchTargetSupplierOnUpgrade.Attatched (VERIFIED-DISASM @0x180d9e5b0) runs when the behaviour
            // is attached, so made at the tier it sets the target too (recording sniper_250: Elite).
            if let Some(m) = g.tier(t).and_then(|x| x.switch_target.as_deref()).and_then(TargetMode::parse) {
                g.towers[t].target = m;
            }
            // InputManager.CreateTowerAt as SimRecorder calls it leaves Tower.worth at 0
            // (logged); bought upgrades add to it.
            g.towers[t].invested = 0.0;
        }
        for (k, name) in &targets {
            let t = g.towers.iter().enumerate().filter(|(_, t)| t.sub.is_none()).nth(*k).map(|(i, _)| i).unwrap();
            g.towers[t].target = TargetMode::parse(name).unwrap_or_else(|| panic!("target {name}"));
        }
        for k in &camos {
            let t = g.towers.iter().enumerate().filter(|(_, t)| t.sub.is_none()).nth(*k).map(|(i, _)| i).unwrap();
            g.towers[t].camo_prio = true;
        }
        for k in &flips {
            let t = g.towers.iter().enumerate().filter(|(_, t)| t.sub.is_none()).nth(*k).map(|(i, _)| i).unwrap();
            g.towers[t].flip = true;
        }
        g.round = meta.get("round").and_then(|r| r.parse::<usize>().ok()).unwrap_or(1) - 1;
        // (Past the mode's last round the game plays on as freeplay: SetRound(89) on Hard.)
        if g.round >= g.end_round {
            g.freeplay = true;
        }
        g.start_round();
        // The game's frame count when the recording starts (its flush every 60 frames keys off it).
        if let (Some(t0), Some((rt0, _, _))) = (first_time, frames.first()) {
            g.sim_elapsed = (t0 - rt0).max(0) as u64;
        }
        if verbose {
            eprintln!("    clone track ends (spawnDist, leakDist): {:?}, lengths {:?}", g.track_ends, g.tracks.iter().map(|t| t.length()).collect::<Vec<_>>());
        }
        let mut frames_diff = 0usize;
        let mut first_diff: Option<(i64, Vec<(String, u32)>, Vec<(String, u32)>)> = None;
        // Throw frames per (tower order, weapon index): game and clone.
        let mut game_ids: Vec<u64> = Vec::new();
        let mut throws_game: std::collections::BTreeMap<(usize, usize), Vec<i64>> = Default::default();
        let mut throws_clone: std::collections::BTreeMap<(usize, usize), Vec<i64>> = Default::default();
        let mut clone_ids: Vec<u64> = Vec::new();
        let mut time_off: Option<(i64, i64, u64)> = None;
        let mut idled = false;
        let (mut lists_diff, mut first_list): (usize, Option<(i64, Vec<(String, f32)>, Vec<(String, f32)>)>) = (0, None);
        let (mut places_diff, mut first_place): (usize, Option<(i64, Vec<(u8, u32, u32)>, Vec<(u8, u32, u32)>)>) = (0, None);
        for (fi, (rt, bloons, weapons)) in frames.iter().enumerate() {
            // SimRecorder's `# ability N`: every ability, cooldown ignored, between recorded frames
            // N and N + 1 (Ability.Activate(true) on each).
            for &(k, path, at) in &upgrades {
                if at == fi {
                    let t = g.towers.iter().enumerate().filter(|(_, t)| t.sub.is_none()).nth(k).map(|(i, _)| i).unwrap();
                    let cash = g.cash;
                    g.cash = 1e12;
                    // Path 3 is the paragon's upgrade (its UpgradeModel.path; Tower.GetUpgrade).
                    let ok = if path == 3 { g.buy_paragon(t, 0.0) } else { g.upgrade(t, path) };
                    g.cash = cash;
                    if verbose {
                        eprintln!("    upgrade tower {k} path {path} at frame {fi}: {ok}, tiers {:?}, paragon {:?}", g.towers[t].tiers, g.towers[t].paragon.as_ref().map(|p| p.0));
                    }
                }
            }
            let mut guard = 0;
            // Frame by frame on the game's own count (roundTime repeats on the frame between
            // rounds, which the next round starts at the end of: Simulation.Simulate,
            // BossBloonManager.AutoStartNextRound).
            while (g.sim_elapsed as i64) < times[fi] && guard < 100_000 {
                if !g.round_active && g.bloons.is_empty() {
                    if idled {
                        g.start_round();
                        idled = false;
                    } else {
                        idled = true;
                    }
                }
                g.step();
                guard += 1;
            }
            // SimRecorder activates after its `ability`-th frame and before writing that frame's
            // lines: after stepping the frame at index ability - 1.
            if ability_at == Some(fi + 1) {
                // Ability.Activate(true) neither needs nor starts the cooldown (VERIFIED-DISASM
                // @0x180df7e20: IsReady(true), cooldown set only when false): keep what each had.
                let saved: Vec<_> = g.towers.iter().map(|tw| tw.ability_cd.clone()).collect();
                for tw in &mut g.towers {
                    tw.ability_cd.clear();
                }
                // (At most once per tower: an ability with no cooldown would otherwise go on
                // forever, as Jungle's Bounty did in druid_040.)
                for name in g.ability_groups().into_iter().map(|a| a.name).collect::<Vec<_>>() {
                    for _ in 0..g.towers.len() {
                        if !g.activate_ability(&name) {
                            break;
                        }
                    }
                }
                for (tw, cd) in g.towers.iter_mut().zip(saved) {
                    tw.ability_cd = cd;
                }
                // The ability's one-shot weapons fire now and are gone after the next frame:
                // their throws count here.
                for tw in g.towers.iter() {
                    // (Keyed by the tower's order of first appearance, as the game's are.)
                    let ti = match clone_ids.iter().position(|x| *x == tw.id) {
                        Some(i) => i,
                        None => {
                            clone_ids.push(tw.id);
                            clone_ids.len() - 1
                        }
                    };
                    // (At the game's slots: the tier's weapons, then each ability's by ability order.)
                    let extra: usize = tw.temp.iter().map(|x| x.attack.weapons.len()).sum();
                    let base = tw.weapons.len().saturating_sub(extra);
                    let slots: Vec<usize> = (0..base).chain(tw.temp.iter().flat_map(|x| (0..x.attack.weapons.len()).map(move |k| base + x.slot + k))).collect();
                    for (wi, st) in tw.weapons.iter().enumerate() {
                        let wi = slots.get(wi).copied().unwrap_or(st.slot);
                        let v = throws_clone.entry((ti, wi)).or_default();
                        if v.last() != Some(&st.last) && st.last > NEVER {
                            v.push(st.last);
                        }
                    }
                }
            }
            if let Some((lo, hi)) = std::env::var("RRBTD6_REC_AT").ok().and_then(|v| v.split_once("..").map(|(a, b)| (a.parse::<i64>().unwrap_or(0), b.parse::<i64>().unwrap_or(0)))) {
                if (lo..=hi).contains(rt) {
                    eprintln!("  rt {rt} clone bloons {:?}", g.bloons.iter().map(|b| (b.id, g.names[b.kind].as_str(), b.dist, b.hp)).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone effects {:?}", g.bloons.iter().map(|b| (b.id, b.effects.iter().map(|e| (e.id.clone(), e.frames_left, if let EffectKind::Dot { tick, .. } = e.kind { tick } else { f32::NAN })).collect::<Vec<_>>())).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone darts {:?}", g.projectiles.iter().map(|p| (p.pos, p.vel, p.pierce, p.frames_left, p.grot, p.z, g.blocker_hit(p.prev, p.spec.vs_blocker_radius, p.z), p.target.is_some(), p.aim, p.seek, p.seek_t)).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone towerpos {:?}", g.towers.iter().map(|t| (t.pos[0], -t.pos[1])).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone towers {:?}", g.towers.iter().enumerate().map(|(i, t)| (t.game_rot, t.base_rot, g.range(i), t.weapons.iter().map(|w| (w.last, w.anim, w.rate_frames)).collect::<Vec<_>>(), t.orbit.as_ref().map(|o| (o.speed, o.angle, o.seen)))).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone hits {:?} layers {:?}", g.projectiles.iter().map(|p| (p.hit.small.clone(), p.clear_t, p.clear_hits_next)).collect::<Vec<_>>(), g.bloons.iter().map(|b| (b.id, b.layer, b.lineage.clone())).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone subpos {:?}", g.towers.iter().filter(|t| t.sub.is_some()).map(|t| (t.pos, t.game_rot, t.sub.as_ref().and_then(|s| s.path_index))).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone plist {:?} live {:?}", g.proj_list, g.projectiles.iter().map(|p| p.uid).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone clock {} steps {} round {} active {} pending {} elapsed {} end {} won {} freeplay {}", g.round_clock, g.steps, g.round, g.round_active, g.pending.len(), g.sim_elapsed, g.end_round, g.won, g.freeplay);
                    eprintln!("  rt {rt} clone list {:?}", g.path_lists.first());
                }
            }
            // Exact: the recording prints each f32 so it reads back to the same value.
            let mut fg: Vec<(String, u32)> = bloons.iter().map(|b| (b.0.clone(), b.1.to_bits())).collect();
            let mut fc: Vec<(String, u32)> = g.bloons.iter().map(|b| (g.names[b.kind].clone(), b.dist.to_bits())).collect();
            fg.sort();
            fc.sort();
            if fg != fc {
                frames_diff += 1;
                if first_diff.is_none() {
                    first_diff = Some((*rt, fg, fc));
                }
            }
            if g.sim_elapsed as i64 != times[fi] && time_off.is_none() {
                time_off = Some((*rt, times[fi], g.sim_elapsed));
            }
            if let Some(l) = &lists[fi] {
                let by_id: std::collections::HashMap<u64, &Bloon> = g.bloons.iter().map(|b| (b.id, b)).collect();
                let c: Vec<(String, u32)> = g.path_lists.first().into_iter().flatten().filter(|e| e.1).filter_map(|e| by_id.get(&e.0)).map(|b| (g.names[b.kind].clone(), b.dist.to_bits())).collect();
                if *l != c {
                    lists_diff += 1;
                    if first_list.is_none() {
                        first_list = Some((*rt, l.iter().map(|(k, d)| (k.clone(), f32::from_bits(*d))).collect::<Vec<_>>(), c.iter().map(|(k, d)| (k.clone(), f32::from_bits(*d))).collect::<Vec<_>>()));
                    }
                }
            }
            let mut pg = places[fi].clone();
            let mut pc: Vec<(u8, u32, u32)> = g.bloons.iter().map(|b| (0, bits(b.pos[0]), bits(-b.pos[1]))).chain(g.projectiles.iter().map(|p| (1, bits(p.pos[0]), bits(-p.pos[1])))).chain(g.towers.iter().flat_map(|t| t.orbit.iter().flat_map(|o| o.glaives.iter())).map(|p| (1, bits(p[0]), bits(-p[1])))).collect();
            pg.sort();
            pc.sort();
            if pg != pc {
                places_diff += 1;
                if std::env::var("RRBTD6_REC_ALLDIFF").is_ok() {
                    let only = |v: &Vec<(u8, u32, u32)>, o: &Vec<(u8, u32, u32)>| v.iter().filter(|x| !o.contains(x)).map(|&(k, x, y)| (k, f32::from_bits(x), f32::from_bits(y))).collect::<Vec<_>>();
                    eprintln!("  posdiff rt {rt}: game {:?} clone {:?}", only(&pg, &pc), only(&pc, &pg));
                }
                if first_place.is_none() {
                    first_place = Some((*rt, pg, pc));
                }
            }
            for (tid, wi, last) in weapons {
                let ti = match game_ids.iter().position(|x| x == tid) {
                    Some(i) => i,
                    None => {
                        game_ids.push(*tid);
                        game_ids.len() - 1
                    }
                };
                let v = throws_game.entry((ti, *wi)).or_default();
                if v.last() != Some(last) && *last > i32::MIN as i64 {
                    v.push(*last);
                }
            }
            // (The recorder lists towers with weapons only.)
            for tw in g.towers.iter().filter(|tw| !tw.weapons.is_empty()) {
                let ti = match clone_ids.iter().position(|x| *x == tw.id) {
                    Some(i) => i,
                    None => {
                        clone_ids.push(tw.id);
                        clone_ids.len() - 1
                    }
                };
                for st in &tw.weapons {
                    let wi = st.slot;
                    let v = throws_clone.entry((ti, wi)).or_default();
                    if v.last() != Some(&st.last) && st.last > NEVER {
                        v.push(st.last);
                    }
                }
            }
        }
        let name = std::path::Path::new(path).file_stem().unwrap().to_string_lossy().to_string();
        let mut throw_report = Vec::new();
        let keys: std::collections::BTreeSet<(usize, usize)> = throws_game.keys().chain(throws_clone.keys()).copied().collect();
        let mut throws_ok = true;
        for k in keys {
            let (a, b) = (throws_game.get(&k).cloned().unwrap_or_default(), throws_clone.get(&k).cloned().unwrap_or_default());
            let first = (0..a.len().max(b.len())).find(|&i| a.get(i) != b.get(i));
            if let Some(i) = first {
                throws_ok = false;
                throw_report.push(format!("tower {} weapon {}: {} vs {} throws, #{i} game {:?} clone {:?}", k.0, k.1, a.len(), b.len(), a.get(i), b.get(i)));
            }
        }
        let ok = frames_diff == 0 && throws_ok && places_diff == 0 && lists_diff == 0;
        eprintln!(
            "{} {name}: {} frames, bloon frames differing {frames_diff}, position frames differing {places_diff}, list order frames differing {lists_diff}, weapons {}",
            if ok { "MATCH" } else { "DIFF " },
            frames.len(),
            if throws_ok { "same".to_string() } else { throw_report.join("; ") }
        );
        if let Some((rt, a, b)) = &first_diff {
            let only_game: Vec<_> = a.iter().filter(|x| !b.contains(x)).map(|(t, d)| (t, f32::from_bits(*d))).collect();
            let only_clone: Vec<_> = b.iter().filter(|x| !a.contains(x)).map(|(t, d)| (t, f32::from_bits(*d))).collect();
            eprintln!("    first bloon difference at roundTime {rt}: only in game {:?}, only in clone {:?}", only_game, only_clone);
        }
        if let Some((rt, t, c)) = time_off {
            eprintln!("    frame count out of step from roundTime {rt}: game {t}, clone {c}");
        }
        if let Some((rt, a, b)) = &first_list {
            eprintln!("    first list order difference at roundTime {rt}:
      game  {:?}
      clone {:?}", a, b);
        }
        if let Some((rt, a, b)) = &first_place {
            let show = |v: &Vec<(u8, u32, u32)>, o: &Vec<(u8, u32, u32)>| v.iter().filter(|x| !o.contains(x)).map(|&(k, x, y)| (if k == 0 { "bloon" } else { "proj" }, f32::from_bits(x), f32::from_bits(y))).collect::<Vec<_>>();
            eprintln!("    first position difference at roundTime {rt}: only in game {:?}, only in clone {:?}", show(a, b), show(b, a));
        }
        // The end screens' player stats after the replay (compare a recording's end-screen text).
        let star = g.towers.iter().filter(|t| t.sub.is_none()).max_by_key(|t| t.pops).map(|t| (t.tiers, t.pops));
        eprintln!("    stats {:?} monkey star {star:?} callout {:?}", g.stats, crate::callouts::pick(&g.stats, &g.difficulty, g.freeplay_seed));
        for t in &g.towers {
            eprintln!("    tower {} {:?} pops {} cash earned {}", g.tower_defs[t.def].0, t.tiers, t.pops, t.cash_earned);
        }
        // Monkey XP earned (the game's per round: its "X" lines, Btd6Player.DrainXP).
        eprintln!("    xp account {} towers {:?}", g.xp_account, g.xp_towers);
        if verbose {
            eprintln!("    throws game {:?}", throws_game);
            eprintln!("    throws clone {:?}", throws_clone);
        }
    }

    /// Prints a spot near the track for each shop tower, in the game's coordinates (y flipped),
    /// as SIMSPOT lines for SimRecorder scenarios.
    #[test]
    #[ignore]
    fn print_tower_spots() {
        let names: Vec<String> = game().tower_defs.iter().map(|(n, _)| n.clone()).collect();
        for n in names {
            let mut g = game();
            let def = g.tower_defs.iter().position(|(x, _)| *x == n).unwrap();
            let c = g.track.pos(200.0);
            let mut found = None;
            'f: for r in [24.0f32, 34.0, 44.0, 60.0] {
                for k in 0..24 {
                    let a = k as f32 / 24.0 * std::f32::consts::TAU;
                    let q = [c[0] + r * a.cos(), c[1] + r * a.sin()];
                    if g.place(def, q) {
                        found = Some(q);
                        break 'f;
                    }
                }
            }
            if let Some(q) = found {
                eprintln!("SIMSPOT {n} {} {}", q[0], -q[1]);
            }
        }
    }

    /// Prints a Dart Monkey spot near the track (for SimRecorder scenarios).
    #[test]
    #[ignore]
    fn print_dart_spot() {
        let mut g = game();
        let dart = g.tower_defs.iter().position(|(n, _)| n == "DartMonkey").unwrap();
        let t = place_near(&mut g, dart, 200.0);
        eprintln!("SPOT {} {} bounds {:?}", g.towers[t].pos[0], g.towers[t].pos[1], g.bounds);
    }

    /// Weapon.CalcRateFrames / UpdateMissedFrames rounding.
    #[test]
    fn weapon_rate_frames() {
        assert_eq!(rate_frames(0.95), 57);
        assert_eq!(rate_frames(0.8075), 48);
        assert_eq!(rate_frames(0.775), 46, "half to even");
        assert_eq!(rate_frames(0.025), 2, "up to 0.1 s rounds up");
        assert_eq!(rate_frames(0.001), 1);
        assert!((missed_rate(0.025) - (2.0 - 1.5) / 1.5).abs() < 1e-6);
        assert_eq!(missed_rate(0.2), 0.0);
    }

    /// A Dart Monkey throws every 57 frames (0.95 s) and its dart leaves 6 frames
    /// (animationOffset 0.1 s) after the throw starts.
    #[test]
    fn dart_monkey_throw_timing() {
        let mut g = game();
        g.cash = 1e7;
        let dart = g.tower_defs.iter().position(|(n, _)| n == "DartMonkey").unwrap();
        let t = place_near(&mut g, dart, 200.0);
        g.round = 30;
        g.start_round();
        let (mut starts, mut shots) = (Vec::new(), Vec::new());
        let mut last = g.towers[t].shots;
        for _ in 0..2000 {
            g.step();
            if g.towers[t].weapons[0].anim == g.steps as i64 {
                starts.push(g.steps);
            }
            if g.towers[t].shots != last {
                last = g.towers[t].shots;
                shots.push(g.steps);
            }
        }
        assert!(shots.len() > 10, "{} shots", shots.len());
        for (a, b) in starts.iter().zip(&shots) {
            assert_eq!(b - a, 6, "dart leaves 6 frames into the throw");
        }
        for w in shots.windows(2).take(5) {
            assert_eq!(w[1] - w[0], 57, "{shots:?}");
        }
    }

    /// CritMultiplier: Sharp Shooter (0-0-4) crits on every 10th shot, counted from the
    /// upgrade; Robo Monkey's arms crit every 15-20 shots each.
    #[test]
    fn crits_follow_crit_multiplier() {
        let mut g = game();
        g.cash = 1e7;
        let dart = g.tower_defs.iter().position(|(n, _)| n == "DartMonkey").unwrap();
        let t = place_near(&mut g, dart, 200.0);
        for _ in 0..4 {
            assert!(g.upgrade(t, 2));
        }
        g.round = 30;
        g.start_round();
        let (mut shots, mut crits) = (Vec::new(), 0);
        for _ in 0..4000 {
            g.step();
            let s = g.towers[t].shots;
            if s > 0 && shots.last() != Some(&s) {
                shots.push(s);
                if g.towers[t].weapons[0].crit.0 == g.towers[t].weapons[0].crit.1 {
                    crits += 1;
                    assert_eq!(s % 10, 0, "crit on shot {s}");
                }
                assert_eq!(g.towers[t].weapons[0].crit.1, 10);
            }
        }
        assert!(crits >= 2, "{crits} crits");
    }

    /// Comanche Defense (Heli x-x-4): a bloon far along the track calls in the mini Comanches.
    #[test]
    fn comanche_defence_calls_in_minis() {
        let mut g = game();
        g.cash = 1e7;
        let heli = g.tower_defs.iter().position(|(n, _)| n == "HeliPilot").unwrap();
        let h = place_near(&mut g, heli, 200.0);
        for _ in 0..4 {
            assert!(g.upgrade(h, 2));
        }
        let id = g.towers[h].id;
        assert!(g.debug_spawn("Lead", 1, 0.0));
        let len = g.tracks[0].length();
        g.bloons[0].dist = 0.8 * len;
        let mut minis = 0;
        for _ in 0..120 {
            g.step();
            minis = minis.max(g.towers.iter().filter(|t| t.sub.as_ref().is_some_and(|s| s.parent == id && s.spec.name == "ComancheDefenceHeli")).count());
        }
        assert_eq!(minis, 3, "three mini Comanches");
    }

    /// Necromancer (Wizard x-x-4): pops in range fill its graveyard, which it spends on undead
    /// bloons walking back up the track.
    #[test]
    fn necromancer_graveyard() {
        let mut g = game();
        g.cash = 1e7;
        let wiz = g.tower_defs.iter().position(|(n, _)| n == "WizardMonkey").unwrap();
        let dart = g.tower_defs.iter().position(|(n, _)| n == "DartMonkey").unwrap();
        let w = place_near(&mut g, wiz, 300.0);
        for _ in 0..4 {
            assert!(g.upgrade(w, 2));
        }
        let id = g.towers[w].id;
        for d in [260.0, 340.0] {
            let t = place_near(&mut g, dart, d);
            assert!(g.upgrade(t, 0));
        }
        g.round = 9;
        g.start_round();
        let mut stored = 0.0f32;
        let mut undead = false;
        for _ in 0..3000 {
            g.step();
            stored = stored.max(g.towers[w].grave.iter().map(|x| x.1).sum());
            undead |= g.projectiles.iter().any(|p| p.owner == id && p.on_track.is_some());
        }
        assert!(stored > 0.0, "graveyard filled");
        assert!(undead, "undead bloons on the track");
        // NecromancerTargetTrackWithinRange: undead start from the target segment's centre, at
        // most maxPathRandomRange x 1.5 back; a segment the player picks is used as it is.
        let r = g.tier(w).and_then(|tr| tr.attacks.iter().find_map(|a| a.necro.as_ref().map(|n| n.path_range))).unwrap();
        let mut seen = std::collections::HashSet::new();
        // (Weapons fire with rounds active: sandbox; a full graveyard.)
        g.sandbox = true;
        let round = g.round;
        g.towers[w].grave = vec![(round, 400.0)];
        for _ in 0..3000 {
            let before: std::collections::HashSet<u64> = g.projectiles.iter().map(|p| p.uid).collect();
            let aim = g.towers[w].necro_emit;
            g.step();
            for p in g.projectiles.iter().filter(|p| p.owner == id && !before.contains(&p.uid) && p.on_track.is_some() && seen.insert(p.uid)) {
                let (path, d0) = p.on_track.expect("on the track");
                let (pi, k) = g.towers[w].necro_emit.or(aim).expect("a target segment");
                assert_eq!(path, pi);
                let (centre, _) = g.seg_span(pi as usize, k).unwrap();
                assert!(d0 <= centre && d0 >= (centre - r * 1.5).max(0.0) - 1e-3, "{d0} from {centre}");
            }
        }
        assert!(!seen.is_empty(), "undead spawned");
        let at = g.towers[w].pos;
        g.necro_select(w, at);
        let sel = g.towers[w].necro_sel.expect("a segment in range");
        let (centre, _) = g.seg_span(sel.0 as usize, sel.1).unwrap();
        let mut picked = false;
        g.towers[w].grave = vec![(round, 400.0)];
        for _ in 0..3000 {
            let before: std::collections::HashSet<u64> = g.projectiles.iter().map(|p| p.uid).collect();
            g.step();
            for p in g.projectiles.iter().filter(|p| p.owner == id && !before.contains(&p.uid) && p.on_track.is_some()) {
                assert_eq!(p.on_track, Some((sel.0, centre)));
                picked = true;
            }
        }
        assert!(picked, "undead from the chosen segment");
    }

    /// Geraldo's shop: a Shooty Turret is placed from stock, pickles speed a Dart Monkey up,
    /// and a sold-out item refills at its round.
    #[test]
    fn geraldo_shop_items() {
        let mut g = game();
        let ger = g.tower_defs.iter().position(|(n, _)| n == "Geraldo").expect("Geraldo");
        let dart = g.tower_defs.iter().position(|(n, _)| n == "DartMonkey").unwrap();
        let gt = place_near(&mut g, ger, 300.0);
        g.add_hero_xp(gt, 1e9);
        let d = place_near(&mut g, dart, 520.0);
        let idx = |g: &Game, n: &str| g.geraldo_items().iter().position(|i| i.name == n).unwrap();
        let turret = idx(&g, "ShootyTurret");
        let before = (g.item_stock(turret), g.cash);
        let spot = g.towers[d].pos;
        let free = (0..96)
            .map(|k| {
                let a = k as f32 / 24.0 * std::f32::consts::TAU / 4.0;
                let r = 30.0 + (k / 24) as f32 * 12.0;
                [spot[0] + r * a.cos(), spot[1] + r * a.sin()]
            })
            .find(|&q| g.can_place(dart, q))
            .expect("free spot");
        assert!(g.use_item(turret, free), "turret placed");
        assert_eq!(g.item_stock(turret), before.0 - 1);
        assert!(g.cash < before.1);
        assert!(g.towers.iter().any(|t| t.sub.as_ref().is_some_and(|s| s.spec.name.starts_with("ShootyTurret"))));
        let pickles = idx(&g, "JarOfPickles");
        g.buffs = g.compute_buffs();
        let rate0 = g.buffs[d].rate_mult;
        assert!(g.use_item(pickles, g.towers[d].pos), "pickles on the dart");
        g.buffs = g.compute_buffs();
        assert!(g.buffs[d].rate_mult < rate0, "faster with pickles");
        let rejuv = idx(&g, "RejuvPotion");
        let lives = g.lives;
        assert!(g.use_item(rejuv, [0.0, 0.0]));
        assert!(g.lives > lives);
        // Hot sauce: a fire creature circles the dart for its rounds (V2 at Geraldo's level 20: 10).
        g.cash = 1e7;
        let sauce = idx(&g, "BottleHotSauce");
        let did = g.towers[d].id;
        let creature = |g: &Game| g.towers.iter().any(|t| t.sub.as_ref().is_some_and(|s| s.parent == did && s.spec.name.starts_with("HotSauceCreature")));
        assert!(g.use_item(sauce, g.towers[d].pos), "sauce on the dart");
        assert!(creature(&g));
        for _ in 0..9 {
            g.expire_subs(true);
        }
        assert!(creature(&g), "still there before its last round");
        g.expire_subs(true);
        assert!(!creature(&g), "gone after its rounds");
        // Four pet rabbits become a Wererabbit.
        let rabbit = idx(&g, "PetRabbit");
        for k in 0..4 {
            if let Some(s) = g.geraldo_stock.get_mut(0).and_then(|v| v.get_mut(rabbit)) {
                s.qty = s.qty.max(1);
            }
            // On the track within Geraldo's range (PetRabbitBehavior.CheckLocation).
            let p = g.track.pos(285.0 + 10.0 * k as f32);
            assert!(g.use_item(rabbit, p), "rabbit {k}");
        }
        let named = |g: &Game, n: &str| g.towers.iter().filter(|t| t.sub.as_ref().is_some_and(|s| s.spec.name == n)).count();
        assert_eq!(named(&g, "PetRabbitBase"), 0);
        assert_eq!(named(&g, "WererabbitTower"), 1);
    }

    #[test]
    fn fan_club_converts_nearby_darts() {
        let mut g = game();
        let dart = g.tower_defs.iter().position(|(n, _)| n == "DartMonkey").unwrap();
        let club = place_near(&mut g, dart, 300.0);
        for _ in 0..4 {
            assert!(g.upgrade(club, 1));
        }
        let near = place_near(&mut g, dart, 310.0);
        let upgraded = place_near(&mut g, dart, 290.0);
        for _ in 0..3 {
            g.upgrade(upgraded, 0);
        }
        g.start_round();
        let name = g.ability_groups().into_iter().next().expect("ability").name;
        g.towers[club].ability_cd.clear();
        assert!(g.activate_ability(&name));
        assert!(g.towers[club].fan.is_some());
        assert!(g.towers[near].fan.is_some());
        assert!(g.towers[upgraded].fan.is_none(), "tier 3 is above maxTier");
        let fc = g.towers[near].fan.clone().unwrap().0;
        assert!(fc.rate < 0.1);
    }
}

#[cfg(test)]
mod beast_tests {
    use super::tests_support::*;

    #[test]
    fn birds_grab_and_pop() {
        let mut g = game();
        let bh = g.tower_defs.iter().position(|(n, _)| n == "BeastHandler").unwrap();
        let t = place_near(&mut g, bh, 300.0);
        while g.upgrade(t, 2) {}
        assert!(g.towers.iter().any(|x| x.sub.as_ref().is_some_and(|s| s.pet) && x.air.is_some()), "bird pet spawned");
        g.round = 29;
        g.start_round();
        for _ in 0..1200 {
            g.step();
        }
        assert!(g.popped > 0);
    }

    #[test]
    fn skywarden_stance_and_rage() {
        let mut g = game();
        let sw = g.tower_defs.iter().position(|(n, _)| n == "Skywarden").unwrap();
        let t = place_near(&mut g, sw, 300.0);
        g.buffs = g.compute_buffs();
        let spread = g.buffs[t].range(g.range(t));
        assert!(g.toggle_stance(t));
        g.buffs = g.compute_buffs();
        assert!(g.buffs[t].range(g.range(t)) > spread * 1.3);
        g.toggle_stance(t);
        g.round = 9;
        g.start_round();
        let mut most = 0;
        for _ in 0..1500 {
            g.step();
            most = most.max(g.towers[t].rage.2);
        }
        assert!(g.towers[t].pops > 0, "Skywarden pops");
        assert!(most > 0, "damage-based attack speed stacks");
    }

    #[test]
    fn summon_phoenix_pops() {
        let run = |use_it: bool| {
            let mut g = game();
            let wiz = g.tower_defs.iter().position(|(n, _)| n == "WizardMonkey").unwrap();
            let t = place_near(&mut g, wiz, 900.0);
            for _ in 0..4 {
                assert!(g.upgrade(t, 1));
            }
            g.round = 19;
            g.start_round();
            if use_it {
                g.towers[t].ability_cd.clear();
                let name = g.ability_groups().into_iter().next().unwrap().name;
                assert!(g.activate_ability(&name));
                assert!(g.towers.iter().any(|x| x.sub.as_ref().is_some_and(|s| s.spec.circle.is_some())));
            }
            for _ in 0..900 {
                g.step();
            }
            g.popped
        };
        let (with, without) = (run(true), run(false));
        assert!(with > without + 20, "phoenix adds pops: {with} vs {without}");
    }

    #[test]
    fn overclock_boosts_picked_tower() {
        let mut g = game();
        let eng = g.tower_defs.iter().position(|(n, _)| n == "EngineerMonkey").unwrap();
        let dart = g.tower_defs.iter().position(|(n, _)| n == "DartMonkey").unwrap();
        let e = place_near(&mut g, eng, 300.0);
        let d = place_near(&mut g, dart, 500.0);
        while g.upgrade(e, 1) {}
        g.start_round();
        g.buffs = g.compute_buffs();
        let before = g.buffs[d].rate_mult;
        let name = g.ability_groups().into_iter().find(|a| g.ability_targets_tower(&a.name)).expect("overclock").name;
        g.towers[e].ability_cd.clear();
        assert!(!g.activate_ability_on(&name, e), "not on itself");
        assert!(g.activate_ability_on(&name, d));
        g.buffs = g.compute_buffs();
        assert!(g.buffs[d].rate_mult < before * 0.7);
        assert_eq!(g.towers[d].ultra.0, 1, "Ultraboost stack");
    }

    #[test]
    fn supply_drop_gives_cash() {
        let mut g = game();
        let sn = g.tower_defs.iter().position(|(n, _)| n == "SniperMonkey").unwrap();
        let t = place_near(&mut g, sn, 300.0);
        for _ in 0..4 {
            assert!(g.upgrade(t, 1));
        }
        g.start_round();
        g.towers[t].ability_cd.clear();
        let groups: Vec<String> = g.ability_groups().into_iter().map(|a| a.name).collect();
        assert!(g.activate_ability("Supply Drop"), "{groups:?}");
        let before = g.cash;
        for _ in 0..600 {
            g.step();
            // The player sweeps the cursor over the crate.
            let crates: Vec<[f32; 2]> = g.projectiles.iter().filter(|p| p.spec.cash.is_some()).map(|p| p.pos).collect();
            for c in crates {
                g.collect_cash(c);
            }
        }
        assert!(g.cash >= before + 1000.0, "cash {} -> {}", before, g.cash);
    }

    #[test]
    fn darkshift_moves_tower() {
        let mut g = game();
        let sm = g.tower_defs.iter().position(|(n, _)| n == "SuperMonkey").unwrap();
        let t = place_near(&mut g, sm, 300.0);
        for _ in 0..4 {
            assert!(g.upgrade(t, 2));
        }
        g.start_round();
        let name = g.ability_groups().into_iter().find(|a| g.ability_targets_point(&a.name)).expect("darkshift").name;
        g.towers[t].ability_cd.clear();
        let from = g.towers[t].pos;
        assert!(!g.activate_ability_at(&name, [from[0] + 200.0, from[1]]), "too far");
        let spot = (0..48)
            .map(|k| {
                let a = k as f32 / 48.0 * std::f32::consts::TAU;
                [from[0] + 30.0 * a.cos(), from[1] + 30.0 * a.sin()]
            })
            .find(|&q| {
                let saved = g.towers[t].pos;
                g.towers[t].pos = [f32::MAX; 2];
                let ok = g.can_place(g.towers[t].def, q);
                g.towers[t].pos = saved;
                ok
            })
            .expect("a free spot");
        assert!(g.activate_ability_at(&name, spot));
        assert_eq!(g.towers[t].pos, spot);
    }

    #[test]
    fn marked_to_pop_fires_after_marking() {
        // MarkedToPop: activated with no marking target, the execution starts at once (recordings
        // desperado_050 / 052 / 250) and the ability is over within its 60 frames.
        let run = |use_it: bool| {
            let mut g = game();
            let de = g.tower_defs.iter().position(|(n, _)| n == "Desperado").unwrap();
            let t = place_near(&mut g, de, 900.0);
            for _ in 0..4 {
                assert!(g.upgrade(t, 1));
            }
            g.round = 24;
            g.start_round();
            let mut at = 0;
            for f in 0..300 {
                if f == 120 && use_it {
                    g.towers[t].ability_cd.clear();
                    assert!(g.activate_ability("Marked To Pop"));
                    at = g.popped;
                }
                g.step();
                if f == 190 && use_it {
                    return (g.popped - at, g.towers[t].mtp.is_none());
                }
            }
            (0, true)
        };
        let (pops, ended) = run(true);
        let _ = pops;
        assert!(ended, "over after its 60 frames");
    }

    /// Every hero places, levels up and pops bloons (prints a per-hero report).
    #[test]
    fn heroes_pop() {
        let mut bad = Vec::new();
        let names: Vec<String> = game().heroes.iter().map(|h| h.id.clone()).collect();
        for name in names {
            let mut g = game();
            let Some(d) = g.tower_defs.iter().position(|(n, _)| *n == name) else {
                bad.push(format!("{name}: no tower def"));
                continue;
            };
            // Water-only heroes (Admiral Brickell) can't go on Monkey Meadow.
            if g.tower_defs[d].1.areas == [1] {
                continue;
            }
            let t = place_near(&mut g, d, 300.0);
            for lvl in [1usize, 10, 20] {
                g.add_hero_xp(t, 1e9);
                let reached = g.towers[t].tiers[0];
                g.round = 9 + lvl;
                g.start_round();
                let before = g.towers[t].pops;
                for _ in 0..900 {
                    g.step();
                }
                let pops = g.towers[t].pops - before;
                eprintln!("{name} level {reached}: {pops} pops");
                if pops == 0 && lvl == 1 {
                    bad.push(format!("{name}: no pops"));
                }
                g.bloons.clear();
                g.round_active = false;
            }
        }
        // Benjamin doesn't attack (he makes cash).
        assert_eq!(bad, vec!["Benjamin: no pops".to_string()]);
    }

    #[test]
    fn water_towers_on_logs() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let m = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/maps/logs/map.json");
        d.map = serde_json::from_str(&std::fs::read_to_string(m).expect("logs map exported")).unwrap();
        let pond = d.map.areas.iter().find(|a| a.kind == 1).expect("a water area").points.clone();
        let mut g = Game::new(d);
        g.cash = 1e9;
        let c = [pond.iter().map(|q| q[0]).sum::<f32>() / pond.len() as f32, pond.iter().map(|q| q[1]).sum::<f32>() / pond.len() as f32];
        let id = |g: &Game, n: &str| g.tower_defs.iter().position(|(x, _)| x == n).unwrap();
        let (sub, dart) = (id(&g, "MonkeySub"), id(&g, "DartMonkey"));
        assert!(!g.can_place(dart, c), "land tower in the pond");
        assert!(g.can_place(sub, c), "sub in the pond");
        let land = place_near(&mut g, dart, 300.0);
        let lp = g.towers[land].pos;
        g.sell(land);
        assert!(!g.can_place(sub, lp), "sub on land");
        assert!(g.place(sub, c));
        let t = g.towers.len() - 1;
        g.round = 5;
        g.start_round();
        for _ in 0..1500 {
            g.step();
        }
        assert!(g.towers[t].pops > 0, "the sub pops");
    }

    /// Every tier of every tower, every hero level and every Paragon is placed in round 25 and
    /// must pop something within 20 s, except the support towers (Banana Farm, Village).
    /// `cargo test --profile game -p btd6-game all_tiers_pop -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn all_tiers_pop() {
        let (mut g, pond) = logs_game();
        // Support towers, cash heroes, and glue that only slows until Corrosive Glue.
        let support = ["BananaFarm", "MonkeyVillage", "Benjamin"];
        let no_damage = |name: &str, key: &str| name == "GlueGunner" && key.as_bytes()[0] < b'2';
        let mut bad = Vec::new();
        let mut runs = 0;
        for def in 0..g.tower_defs.len() {
            let name = g.tower_defs[def].0.clone();
            let Some(tt) = g.tower_tiers[def].clone() else { continue };
            let hero = tt.hero.is_some();
            let mut keys: Vec<String> = tt.tiers.keys().cloned().collect();
            keys.sort();
            if tt.paragon.is_some() {
                keys.push("paragon".into());
            }
            for key in keys {
                g.cash = 1e9;
                let water = g.tower_defs[def].1.areas == [1];
                // The Beast Handler's fish path needs water in range: on land by the pond.
                let by_pond = name == "BeastHandler" && key.as_bytes()[0] > b'0';
                let t = if water {
                    if !g.place(def, pond) {
                        bad.push(format!("{name} {key}: can't place"));
                        continue;
                    }
                    g.towers.iter().rposition(|t| t.sub.is_none()).unwrap()
                } else if by_pond {
                    let spot = (1..12).flat_map(|r| (0..24).map(move |k| (r, k))).map(|(r, k)| {
                        let a = k as f32 / 24.0 * std::f32::consts::TAU;
                        [pond[0] + 8.0 * r as f32 * a.cos(), pond[1] + 8.0 * r as f32 * a.sin()]
                    }).find(|&q| g.can_place(def, q));
                    match spot {
                        Some(q) if g.place(def, q) => g.towers.iter().rposition(|t| t.sub.is_none()).unwrap(),
                        _ => {
                            bad.push(format!("{name} {key}: no spot by the pond"));
                            continue;
                        }
                    }
                } else {
                    place_near(&mut g, def, 300.0)
                };
                if key == "paragon" {
                    let p = tt.paragon.clone().unwrap();
                    let tier = std::sync::Arc::new(g.paragon_tier(&p.tier, 1));
                    g.towers[t].tiers = [5, 0, 0];
                    g.towers[t].paragon = Some((1, tier));
                } else if hero {
                    g.towers[t].tiers = [key[..key.len() - 2].parse().unwrap(), 0, 0];
                } else {
                    let k: Vec<u8> = key.bytes().map(|b| b - b'0').collect();
                    g.towers[t].tiers = [k[0], k[1], k[2]];
                }
                // What placing / upgrading normally sets up: pets, air units, abilities; and a
                // cursor on the track for cursor-aimed towers (Dartling).
                g.sync_pets(t);
                g.init_air(t);
                g.sync_abilities(t);
                g.pointer = Some(g.track.pos(320.0));
                g.round = 24;
                g.start_round();
                for _ in 0..1200 {
                    g.step();
                }
                runs += 1;
                let id = g.towers[t].id;
                let pops: u64 = g.towers.iter().filter(|o| o.id == id || o.sub.as_ref().is_some_and(|s| s.parent == id)).map(|o| o.pops).sum();
                if pops == 0 && !support.contains(&name.as_str()) && !no_damage(&name, &key) {
                    bad.push(format!("{name} {key}"));
                }
                while !g.towers.is_empty() {
                    let n = g.towers.len() - 1;
                    g.towers.swap_remove(n);
                }
                g.bloons.clear();
                g.projectiles.clear();
                g.pending.clear();
                g.round_active = false;
            }
        }
        eprintln!("all_tiers_pop: {runs} runs, {} without pops", bad.len());
        for b in &bad {
            eprintln!("  {b}");
        }
        assert!(bad.is_empty());
    }

    /// Every tier of the water towers pops something (prints the ones that don't).
    #[test]
    #[ignore]
    fn water_tiers_pop() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let m = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/maps/logs/map.json");
        d.map = serde_json::from_str(&std::fs::read_to_string(m).expect("logs map exported")).unwrap();
        let pond = d.map.areas.iter().find(|a| a.kind == 1).expect("a water area").points.clone();
        let c = [pond.iter().map(|q| q[0]).sum::<f32>() / pond.len() as f32, pond.iter().map(|q| q[1]).sum::<f32>() / pond.len() as f32];
        let mut g = Game::new(d);
        let mut bad = Vec::new();
        for name in ["MonkeySub", "MonkeyBuccaneer", "Mermonkey"] {
            let def = g.tower_defs.iter().position(|(x, _)| x == name).unwrap();
            let mut keys: Vec<String> = g.tower_tiers[def].as_ref().unwrap().tiers.keys().cloned().collect();
            keys.sort();
            for key in keys {
                let tiers: Vec<u8> = key.bytes().map(|b| b - b'0').collect();
                if tiers.len() != 3 {
                    continue;
                }
                g.cash = 1e9;
                let start = std::time::Instant::now();
                assert!(g.place(def, c), "{name} placeable");
                let t = g.towers.len() - 1;
                g.towers[t].tiers = [tiers[0], tiers[1], tiers[2]];
                g.round = 25;
                g.start_round();
                for _ in 0..1200 {
                    g.step();
                }
                let pops = g.towers[t].pops;
                eprintln!("{name} {key}: {pops} pops, {} ms", start.elapsed().as_millis());
                if pops == 0 {
                    bad.push(format!("{name} {key}"));
                }
                while !g.towers.is_empty() {
                    g.sell(g.towers.len() - 1);
                }
                g.bloons.clear();
                g.projectiles.clear();
                g.pending.clear();
                g.round_active = false;
            }
        }
        assert!(bad.is_empty(), "no pops: {bad:?}");
    }

    /// Submerge (Sub 5-0-0, Energizer): the pulse attack replaces the darts and pops bloons,
    /// and ability cooldowns of towers in range run faster.
    #[test]
    fn sub_submerges() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let m = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/maps/logs/map.json");
        d.map = serde_json::from_str(&std::fs::read_to_string(m).expect("logs map exported")).unwrap();
        let pond = d.map.areas.iter().find(|a| a.kind == 1).expect("a water area").points.clone();
        let mut g = Game::new(d);
        g.cash = 1e9;
        let c = [pond.iter().map(|q| q[0]).sum::<f32>() / pond.len() as f32, pond.iter().map(|q| q[1]).sum::<f32>() / pond.len() as f32];
        let def = g.tower_defs.iter().position(|(x, _)| x == "MonkeySub").unwrap();
        assert!(g.place(def, c));
        let t = g.towers.len() - 1;
        for _ in 0..5 {
            assert!(g.upgrade(t, 0));
        }
        assert!(g.can_submerge(t));
        g.towers[t].submerged = true;
        g.round = 10;
        g.start_round();
        for _ in 0..1200 {
            g.step();
        }
        assert!(g.towers[t].pops > 0, "the pulse pops");
        let p = g.towers[t].pos;
        assert!((g.submerged_scale(p, |s| (s.cooldown_scale, s.cooldown_scale_global)) - 1.2).abs() < 1e-3);
    }

    /// P-TRANCE: Mermonkey 0-0-3 sends bloons circling it; P-WATERRANGE: more range on water.
    #[test]
    fn mermonkey_trance_and_water_range() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let m = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/maps/logs/map.json");
        d.map = serde_json::from_str(&std::fs::read_to_string(m).expect("logs map exported")).unwrap();
        let pond = d.map.areas.iter().find(|a| a.kind == 1).expect("a water area").points.clone();
        let mut g = Game::new(d);
        g.cash = 1e9;
        let c = [pond.iter().map(|q| q[0]).sum::<f32>() / pond.len() as f32, pond.iter().map(|q| q[1]).sum::<f32>() / pond.len() as f32];
        let def = g.tower_defs.iter().position(|(x, _)| x == "Mermonkey").unwrap();
        let land = place_near(&mut g, def, 300.0);
        assert!(g.place(def, c));
        let t = g.towers.len() - 1;
        g.buffs = g.compute_buffs();
        assert!(g.range(t) > g.range(land) * 1.3, "water range bonus");
        for _ in 0..3 {
            assert!(g.upgrade(t, 2));
        }
        g.round = 10;
        g.start_round();
        let mut tranced = false;
        for _ in 0..1800 {
            g.step();
            tranced |= g.bloons.iter().any(|b| b.effects.iter().any(|e| matches!(e.kind, EffectKind::Trance { .. })));
        }
        assert!(tranced, "a bloon was entranced");
    }

    /// P-TRADEEMPIRE / P-CASHBACK: Trade Empire pays per merchant ship, Favored Trades raises
    /// the sell value of towers in range.
    #[test]
    fn buccaneer_trade() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let m = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/maps/logs/map.json");
        d.map = serde_json::from_str(&std::fs::read_to_string(m).expect("logs map exported")).unwrap();
        let mut g = Game::new(d);
        g.cash = 1e9;
        let def = g.tower_defs.iter().position(|(x, _)| x == "MonkeyBuccaneer").unwrap();
        let mut placed = Vec::new();
        'outer: for y in (-200..200).step_by(6) {
            for x in (-300..300).step_by(6) {
                if g.place(def, [x as f32, y as f32]) {
                    placed.push(g.towers.len() - 1);
                    if placed.len() == 3 {
                        break 'outer;
                    }
                }
            }
        }
        assert_eq!(placed.len(), 3, "room for three ships");
        g.towers[placed[0]].tiers = [0, 0, 5];
        g.towers[placed[1]].tiers = [0, 0, 3];
        g.towers[placed[2]].tiers = [0, 0, 4];
        g.buffs = g.compute_buffs();
        // Two zones in range: the Favored Trades ship and Trade Empire (it keeps the zone).
        assert!((g.sell_fraction(placed[1]) - 0.78).abs() < 1e-6, "{}", g.sell_fraction(placed[1]));
        g.start_round();
        for _ in 0..20000 {
            g.step();
            if !g.round_active {
                break;
            }
        }
        let te = g.towers[placed[0]].pos;
        assert!(g.cash_pops.iter().any(|&(q, v)| q == te && (v - 40.0).abs() < 1e-6), "Trade Empire pays 2 ships x 10 + 1 Favored Trades x 20");
    }

    /// P-FIGHTER: Aircraft Carrier keeps at most 3 planes (SubTowerFilterModel); they pop and
    /// leave when the ship is sold.
    #[test]
    fn carrier_planes() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let m = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/maps/logs/map.json");
        d.map = serde_json::from_str(&std::fs::read_to_string(m).expect("logs map exported")).unwrap();
        let pond = d.map.areas.iter().find(|a| a.kind == 1).expect("a water area").points.clone();
        let mut g = Game::new(d);
        g.cash = 1e9;
        let c = [pond.iter().map(|q| q[0]).sum::<f32>() / pond.len() as f32, pond.iter().map(|q| q[1]).sum::<f32>() / pond.len() as f32];
        let def = g.tower_defs.iter().position(|(x, _)| x == "MonkeyBuccaneer").unwrap();
        assert!(g.place(def, c));
        g.towers[0].tiers = [4, 0, 0];
        g.round = 15;
        g.start_round();
        let mut most = 0;
        for _ in 0..1800 {
            g.step();
            most = most.max(g.towers.iter().filter(|t| t.sub.is_some()).count());
        }
        assert_eq!(most, 3, "three planes");
        let planes: u64 = g.towers.iter().filter(|t| t.sub.is_some()).map(|t| t.pops).sum();
        assert!(g.towers[0].pops > 0 || planes > 0);
        let ship = g.towers.iter().position(|t| t.sub.is_none()).unwrap();
        g.sell(ship);
        assert!(g.towers.is_empty(), "planes go with the ship");
    }

    /// Logs map game with cash and the pond centre (water towers' tests).
    fn logs_game() -> (Game, [f32; 2]) {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let m = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/maps/logs/map.json");
        d.map = serde_json::from_str(&std::fs::read_to_string(m).expect("logs map exported")).unwrap();
        let pond = d.map.areas.iter().find(|a| a.kind == 1).expect("a water area").points.clone();
        let mut g = Game::new(d);
        g.cash = 1e9;
        let c = [pond.iter().map(|q| q[0]).sum::<f32>() / pond.len() as f32, pond.iter().map(|q| q[1]).sum::<f32>() / pond.len() as f32];
        (g, c)
    }

    /// P-TAKEDOWN: MOAB Takedown destroys the strongest MOAB whole for double pop cash; ZOMG
    /// Takedown spends 3 of its 6 hooks on a ZOMG.
    #[test]
    fn takedown_hooks() {
        let (mut g, c) = logs_game();
        let def = g.tower_defs.iter().position(|(x, _)| x == "MonkeyBuccaneer").unwrap();
        assert!(g.place(def, c));
        g.towers[0].tiers = [0, 4, 0];
        g.start_round();
        g.bloons.clear();
        g.pending.clear();
        let mid = g.track.length() * 0.4;
        g.debug_spawn("Moab", 1, 0.0);
        g.debug_spawn("Bfb", 1, 0.0);
        for b in &mut g.bloons {
            b.dist = mid;
            b.pos = g.track.pos(mid);
        }
        g.sync_abilities(0);
        g.towers[0].ability_cd.clear();
        let cash = g.cash;
        assert!(g.activate_ability("MOAB Takedown"));
        let names: Vec<&str> = g.bloons.iter().map(|b| g.names[b.kind].as_str()).collect();
        assert_eq!(names, ["Moab"], "the BFB (strongest) went");
        let bfb = g.by_name["Bfb"];
        assert_eq!(g.cash - cash, 2.0 * g.pops_in[bfb] as f64);

        g.towers[0].tiers = [0, 5, 0];
        g.debug_spawn("Zomg", 1, 0.0);
        g.debug_spawn("Moab", 2, 0.0);
        for b in &mut g.bloons {
            b.dist = mid;
            b.pos = g.track.pos(mid);
        }
        g.sync_abilities(0);
        g.towers[0].ability_cd.clear();
        assert!(g.activate_ability("ZOMG Takedown"));
        assert!(g.bloons.is_empty(), "ZOMG (3 hooks) + 3 MOABs (1 each) = 6 hooks");
    }

    /// First Strike Capability: the strike lands on the strongest bloon after its 0.6 s flight
    /// (CreateProjectileOnExpireModel, target-only 10000 damage) and takes a ZOMG out.
    #[test]
    fn first_strike() {
        let (mut g, c) = logs_game();
        let def = g.tower_defs.iter().position(|(x, _)| x == "MonkeySub").unwrap();
        assert!(g.place(def, c));
        g.towers[0].tiers = [0, 4, 0];
        g.start_round();
        g.bloons.clear();
        g.pending.clear();
        let mid = g.track.length() * 0.4;
        g.debug_spawn("Zomg", 1, 0.0);
        g.bloons[0].dist = mid;
        g.bloons[0].pos = g.track.pos(mid);
        g.sync_abilities(0);
        g.towers[0].ability_cd.clear();
        assert!(g.activate_ability("First Strike Capability"));
        for _ in 0..90 {
            g.step();
        }
        assert!(!g.bloons.iter().any(|b| g.names[b.kind] == "Zomg"), "ZOMG destroyed");
    }

    /// P-PREEMPT: Pre-emptive Strike destroys round 40's MOAB within a second of it entering the
    /// map (it launches on OnBloonEnterTrack, so frames spent off-map before that don't count).
    #[test]
    fn preemptive_strike() {
        let (mut g, c) = logs_game();
        let def = g.tower_defs.iter().position(|(x, _)| x == "MonkeySub").unwrap();
        assert!(g.place(def, c));
        g.towers[0].tiers = [0, 5, 0];
        g.round = 39;
        g.start_round();
        let (mut seen, mut longest, mut life) = (false, 0, 0);
        for _ in 0..6000 {
            g.step();
            if g.bloons.iter().any(|b| g.names[b.kind] == "Moab" && !g.outside_track.contains(&b.id)) {
                seen = true;
                life += 1;
                longest = longest.max(life);
            } else {
                life = 0;
            }
        }
        assert!(seen, "round 40 sends a MOAB");
        assert!(longest <= 60, "a MOAB lasted {longest} frames");
    }

    /// P-SUBCOMMANDER: Sub Commander doubles damage and adds 4 pierce for Subs in range,
    /// itself included; other towers don't get it.
    #[test]
    fn sub_commander() {
        let (mut g, _) = logs_game();
        let sub = g.tower_defs.iter().position(|(x, _)| x == "MonkeySub").unwrap();
        let mut placed = Vec::new();
        'outer: for y in (-200..200).step_by(4) {
            for x in (-300..300).step_by(4) {
                if g.place(sub, [x as f32, y as f32]) {
                    placed.push(g.towers.len() - 1);
                    if placed.len() == 2 {
                        break 'outer;
                    }
                }
            }
        }
        assert_eq!(placed.len(), 2);
        g.towers[placed[0]].tiers = [0, 0, 5];
        g.buffs = g.compute_buffs();
        for &t in &placed {
            assert_eq!(g.buffs[t].damage_mult, 2.0);
            assert_eq!(g.buffs[t].pierce_add, 4.0);
        }
    }

    /// Every ability of every tower tier activates mid-round and the sim keeps running (a
    /// smoke test so new content can't crash an ability path unnoticed).
    #[test]
    fn every_ability_runs() {
        let (mut g, pond) = logs_game();
        let mut count = 0;
        for def in 0..g.tower_defs.len() {
            let Some(tt) = g.tower_tiers[def].clone() else { continue };
            let hero = tt.hero.is_some();
            let mut keys: Vec<&String> = tt.tiers.iter().filter(|(_, t)| !t.abilities.is_empty()).map(|(k, _)| k).collect();
            keys.sort();
            for key in keys {
                // Hero tiers are "<level>00".
                let tiers: Vec<u8> = if hero { vec![key[..key.len() - 2].parse().unwrap(), 0, 0] } else { key.bytes().map(|b| b - b'0').collect() };
                let water = g.tower_defs[def].1.areas == [1];
                let t = if water {
                    if !g.place(def, pond) {
                        continue;
                    }
                    g.towers.iter().rposition(|t| t.sub.is_none()).unwrap()
                } else {
                    place_near(&mut g, def, 300.0)
                };
                g.towers[t].tiers = [tiers[0], tiers[1], tiers[2]];
                g.round = 30;
                g.start_round();
                for _ in 0..60 {
                    g.step();
                }
                g.sync_abilities(t);
                g.towers[t].ability_cd.clear();
                for a in tt.tiers[key].abilities.iter() {
                    if g.ability_targets_point(&a.name) {
                        let p = g.track.pos(200.0);
                        g.activate_ability_at(&a.name, p);
                    } else if g.ability_targets_tower(&a.name) {
                        g.activate_ability_on(&a.name, t);
                    } else {
                        g.activate_ability(&a.name);
                    }
                    count += 1;
                }
                for _ in 0..240 {
                    g.step();
                }
                while !g.towers.is_empty() {
                    g.sell(g.towers.len() - 1);
                }
                g.bloons.clear();
                g.projectiles.clear();
                g.pending.clear();
                g.round_active = false;
            }
        }
        assert!(count > 50, "only {count} abilities found");
    }

    /// P-DECK: a Dart Monkey goes on the Carrier Flagship's deck (not a Banana Farm), stands at
    /// deck height and goes when the ship is sold.
    #[test]
    fn flagship_deck() {
        let (mut g, c) = logs_game();
        let def = g.tower_defs.iter().position(|(x, _)| x == "MonkeyBuccaneer").unwrap();
        assert!(g.place(def, c));
        g.towers[0].tiers = [5, 0, 0];
        let tw = &g.towers[0];
        let d = g.tier_of(tw).unwrap().decks[0].clone();
        let (sn, cs) = tw.angle.sin_cos();
        let m = [d.points.iter().map(|q| q[0]).sum::<f32>() / d.points.len() as f32, d.points.iter().map(|q| q[1]).sum::<f32>() / d.points.len() as f32];
        let p = [tw.pos[0] + cs * m[1] + sn * m[0], tw.pos[1] + sn * m[1] - cs * m[0]];
        let dart = g.tower_defs.iter().position(|(x, _)| x == "DartMonkey").unwrap();
        let farm = g.tower_defs.iter().position(|(x, _)| x == "BananaFarm").unwrap();
        assert!(!g.can_place(farm, p) || g.deck_at(farm, p).is_none(), "no farms on deck");
        assert!(g.can_place(dart, p), "dart on deck");
        assert!(g.place(dart, p));
        assert_eq!(g.towers[1].deck.map(|x| x.1), Some(d.height));
        g.sell(0);
        assert!(g.towers.is_empty(), "deck towers go with the ship");
    }

    /// P-BARRAGE: MOAB Barrage puts its damage-over-time on the strongest MOAB-class bloon; P-BIOHACK:
    /// Biohack boosts the nearest towers' damage.
    #[test]
    fn sandbox_zero_lives_stops_attacks_without_defeat_and_can_resume(){
        let mut g=game();g.sandbox=true;g.cash=1e9;
        let dart=g.tower_defs.iter().position(|(id,_)|id=="DartMonkey").unwrap();
        let t=place_near(&mut g,dart,300.0);
        g.debug_spawn("Red",1,0.0);g.bloons[0].dist=300.0;g.bloons[0].pos=g.track.pos(300.0);
        g.lives=0.0;let before=g.bloons[0].dist;
        for _ in 0..10{g.step();}
        assert!(!g.game_over);assert!(g.bloons[0].dist>before);assert_eq!(g.towers[t].pops,0);assert!(g.projectiles.is_empty());
        crate::sandbox::SandboxAction::SetLives(100.0).apply(&mut g);
        for _ in 0..120{g.step();}
        assert!(g.towers[t].pops>0);assert!(!g.game_over);
        g.sandbox=false;g.lives=0.0;g.step();assert!(g.game_over);
    }
    #[test]
    fn hero_barrage_and_biohack() {
        let mut g = game();
        g.cash = 1e9;
        let ch = g.tower_defs.iter().position(|(x, _)| x == "CaptainChurchill").unwrap();
        let t = place_near(&mut g, ch, 300.0);
        g.towers[t].tiers = [10, 0, 0];
        g.start_round();
        g.bloons.clear();
        g.pending.clear();
        g.debug_spawn("Bfb", 1, 0.0);
        let mid = g.track.length() * 0.3;
        g.bloons[0].dist = mid;
        g.bloons[0].pos = g.track.pos(mid);
        g.sync_abilities(t);
        g.towers[t].ability_cd.clear();
        assert!(g.activate_ability("MOAB Barrage"));
        assert!(g.bloons[0].has_effect("MoabBarrage"));
        g.sell(t);
        let bj = g.tower_defs.iter().position(|(x, _)| x == "Benjamin").unwrap();
        let dart = g.tower_defs.iter().position(|(x, _)| x == "DartMonkey").unwrap();
        let b = place_near(&mut g, bj, 300.0);
        g.towers[b].tiers = [3, 0, 0];
        let d = place_near(&mut g, dart, 320.0);
        g.sync_abilities(b);
        g.towers[b].ability_cd.clear();
        assert!(g.activate_ability("Biohack"));
        g.buffs = g.compute_buffs();
        assert_eq!(g.buffs[d].damage_add, 1.0);
    }

    /// Mega Mine: placed ahead of a ZOMG, it blows up when the ZOMG reaches it (4000 damage).
    #[test]
    fn mega_mine() {
        let (mut g, c) = logs_game();
        let def = g.tower_defs.iter().position(|(x, _)| x == "AdmiralBrickell").unwrap();
        assert!(g.place(def, c));
        g.towers[0].tiers = [10, 0, 0];
        g.start_round();
        g.bloons.clear();
        g.pending.clear();
        let mid = g.track.length() * 0.3;
        g.debug_spawn("Zomg", 1, 0.0);
        g.bloons[0].dist = mid;
        g.bloons[0].pos = g.track.pos(mid);
        g.sync_abilities(0);
        g.towers[0].ability_cd.clear();
        assert!(g.ability_targets_point("Mega Mine"));
        assert!(g.activate_ability_at("Mega Mine", g.track.pos(mid + 30.0)));
        for _ in 0..600 {
            g.step();
        }
        assert!(!g.bloons.iter().any(|b| g.names[b.kind] == "Zomg"), "the mine took the ZOMG");
    }

    /// Paragon: needs a tier 5 on every path among the towers of its kind; buying it sacrifices
    /// the others, gives degree 1 with nothing extra invested, and higher degrees speed up and
    /// strengthen it as the game's PowerDegreeMutator does.
    #[test]
    fn paragon_degree_and_upgrade() {
        let mut g = game();
        g.cash = 1e9;
        let dart = g.tower_defs.iter().position(|(x, _)| x == "DartMonkey").unwrap();
        let a = place_near(&mut g, dart, 200.0);
        let b = place_near(&mut g, dart, 400.0);
        g.towers[a].tiers = [5, 2, 0];
        g.towers[b].tiers = [0, 5, 2];
        assert!(!g.paragon_ready(a), "no tier 5 on the third path yet");
        let c = place_near(&mut g, dart, 600.0);
        g.towers[c].tiers = [2, 0, 5];
        assert!(g.paragon_ready(a));
        let (name, cost) = g.next_upgrade(a, 0).unwrap();
        assert_eq!(name, "DartMonkey Paragon");
        let cash = g.cash;
        assert!(g.upgrade(a, 0));
        assert_eq!(g.cash, cash - cost);
        assert_eq!(g.towers.iter().filter(|t| t.sub.is_none()).count(), 1, "the others were sacrificed");
        assert_eq!(g.towers[0].paragon.as_ref().unwrap().0, 1, "three tier 5s, no pops: degree 1");
        assert!(g.next_upgrade(0, 2).is_none() && !g.upgrade(0, 2), "a Paragon takes no path upgrades");
        assert_eq!(g.paragon_level(&g.towers[0]), Some(1));
        g.towers[0].paragon.as_mut().unwrap().0 = 100;
        assert_eq!(g.paragon_level(&g.towers[0]), Some(5), "degree 100: the last display level");
        g.towers[0].paragon.as_mut().unwrap().0 = 39;
        assert_eq!(g.paragon_level(&g.towers[0]), Some(2), "39 / 20 = 1: level 2");
        // Degree 1 is the Paragon's own stats; degree 100 has every bonus.
        let base = g.tower_tiers[dart].as_ref().unwrap().paragon.clone().unwrap().tier;
        let d1 = g.paragon_tier(&base, 1);
        assert_eq!(d1.attacks[0].weapons[0].rate, base.attacks[0].weapons[0].rate);
        let d100 = g.paragon_tier(&base, 100);
        let (w0, w1) = (&base.attacks[0].weapons[0], &d100.attacks[0].weapons[0]);
        // round1(sqrt(99 x 50)) = 70.4 % faster; +100 % damage + 10.9 +1 at degree 100.
        assert!((w1.rate - w0.rate / 1.704).abs() < 1e-4, "{} vs {}", w1.rate, w0.rate / 1.704);
        if w0.projectile.damage > 0.0 {
            let want = w0.projectile.damage * 2.0 + 99.0 / 10.0 + 1.0;
            assert!((w1.projectile.damage - want).abs() < 1e-3, "{} vs {want}", w1.projectile.damage);
        }
        // No towers sacrificed: degree 1.
        assert!(g.paragon_degree(&[], 150000.0) == 1);
    }

    /// Difficulty prices match the game's: a $200 Dart Monkey is $170 on Easy, $215 on Hard and
    /// $240 on Impoppable (x / 5 rounded half to even, x5); ties round to the even multiple.
    #[test]
    fn difficulty_costs_and_modes() {
        let mults = [0.85f32, 1.0, 1.08, 1.2];
        let darts: Vec<f64> = mults.iter().map(|&m| game_cost(200.0, m, 0.0)).collect();
        assert_eq!(darts, vec![170.0, 200.0, 215.0, 240.0]);
        assert_eq!(game_cost(50.0, 0.85, 0.0), 40.0, "42.5 / 5 = 8.5 rounds to 8");
        assert_eq!(game_cost(1000.0, 1.0, 0.1), 900.0);
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        d.mode = "Hard".into();
        let g = Game::new(d);
        assert_eq!((g.cash, g.lives, g.round, g.mode_name.as_str()), (650.0, 100.0, 2, "Hard"));
        let red = g.by_name["Red"];
        assert!((g.defs[red].speed - 25.0 * 1.25).abs() < 1e-4, "Hard bloons x1.25");
        let dart = g.tower_defs.iter().position(|(x, _)| x == "DartMonkey").unwrap();
        assert_eq!(g.place_cost(dart, [0.0, 0.0]), 215.0);
        // Easy: round 40's MOAB has (int)(200 x 2/3) = 133 health.
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        d.mode = "Easy".into();
        let mut g = Game::new(d);
        g.round = 39;
        g.start_round();
        let mut seen = false;
        for _ in 0..3000 {
            g.step();
            if let Some(b) = g.bloons.iter().find(|b| g.names[b.kind] == "Moab") {
                assert_eq!(b.hp, 133);
                seen = true;
                break;
            }
        }
        assert!(seen);
    }

    /// Sword Charge runs back along the track from its end at its speed and pops what it meets;
    /// Leaping Sword's impact lands on the lead bloon after its delay.
    #[test]
    fn sauda_abilities() {
        let mut g = game();
        g.cash = 1e9;
        let sd = g.tower_defs.iter().position(|(x, _)| x == "Sauda").unwrap();
        let t = place_near(&mut g, sd, 300.0);
        g.towers[t].tiers = [10, 0, 0];
        g.start_round();
        g.bloons.clear();
        g.pending.clear();
        g.debug_spawn("Red", 40, 600.0);
        g.sync_abilities(t);
        g.towers[t].ability_cd.clear();
        let before = g.projectiles.len();
        assert!(g.activate_ability("Sword Charge"));
        let i = g.projectiles.len() - 1;
        assert!(g.projectiles.len() > before);
        let d0 = g.projectiles[i].on_track.unwrap().1;
        // Made between frames (an ability), it is processed on the next one: one step of movement.
        g.step();
        let pr = g.projectiles.iter().find(|p| p.on_track.is_some()).expect("still running");
        let speed = pr.spec.track_travel.as_ref().unwrap().speed;
        assert!((d0 - pr.on_track.unwrap().1 - speed / FPS).abs() < 1e-3, "one step of speed / 60 back");
        let p = pr.pos;
        let q = g.track.pos(pr.on_track.unwrap().1);
        assert!((p[0] - q[0]).abs() < 1e-3 && (p[1] - q[1]).abs() < 1e-3, "on the track");
        for _ in 0..240 {
            g.step();
        }
        assert!(g.towers[t].pops > 0, "the charge popped bloons");
        g.towers[t].ability_cd.clear();
        let lead = g.bloons.iter().max_by(|a, b| a.dist.total_cmp(&b.dist)).map(|b| b.pos);
        if lead.is_some() {
            assert!(g.activate_ability("Leaping Sword Attack"));
        }
    }

    /// Splitter routing on Balance: round bloons spawn on the two start paths (not the boss
    /// path), split round-robin into the middle paths at each start's end, and leak at the ends.
    #[test]
    fn balance_splitters() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let m = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/maps/balance/map.json");
        d.map = serde_json::from_str(&std::fs::read_to_string(m).expect("balance map exported")).unwrap();
        let names: Vec<String> = d.map.paths.iter().filter(|p| p.active && p.points.len() > 1).map(|p| p.name.clone()).collect();
        let mut g = Game::new(d);
        let spawn: Vec<&str> = g.spawn_tracks.iter().map(|&k| names[k].as_str()).collect();
        assert_eq!(spawn, ["PathRHStart", "PathLHStart"]);
        g.round = 5;
        g.start_round();
        let mut seen = std::collections::HashSet::new();
        let lives = g.lives;
        for _ in 0..20000 {
            g.step();
            for b in &g.bloons {
                seen.insert(names[b.path as usize].clone());
            }
            if !g.round_active {
                break;
            }
        }
        for want in ["PathLHSplit1", "PathLHSplit2", "PathRHSplit1", "PathRHSplit4", "PathLHEnd", "PathRHEnd"] {
            assert!(seen.contains(want), "no bloon went down {want}: {seen:?}");
        }
        assert!(g.lives < lives, "they leak at the ends");
        // Pushed back past a split path's start, a bloon goes back onto its entry path
        // (Path.DistanceToPath) with that path's length added, and its progress drops by `back`.
        let split = names.iter().position(|n| n == "PathLHSplit1").unwrap();
        let start = names.iter().position(|n| n == "PathLHStart").unwrap();
        g.bloons.clear();
        let id = g.spawn_on(g.by_name["Red"], 5.0, split as u8);
        let i = g.bloons.iter().position(|b| b.id == id).unwrap();
        g.bloons[i].base = g.tracks[start].length();
        let before = g.bloons[i].progress();
        g.push_back(i, 20.0);
        let b = &g.bloons[i];
        assert_eq!(b.path as usize, start);
        assert!((b.dist - (g.tracks[start].length() - 15.0)).abs() < 1e-3);
        assert!((b.progress() - (before - 20.0)).abs() < 1e-3);
    }

    /// Writes where a Dart Monkey can be placed on map `RRBTD6_PLACEMAP_KEY` (default logs) as
    /// a text grid (1 = placeable) for checking against the terrain image.
    #[test]
    #[ignore]
    fn placement_grid() {
        let key = std::env::var("RRBTD6_PLACEMAP_KEY").unwrap_or_else(|_| "logs".into());
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let m = format!("{}/../local_extracted/maps/{key}/map.json", env!("CARGO_MANIFEST_DIR"));
        d.map = serde_json::from_str(&std::fs::read_to_string(m).unwrap()).unwrap();
        let g = Game::new(d);
        let dart = g.tower_defs.iter().position(|(x, _)| x == "DartMonkey").unwrap();
        let (lo, hi) = g.bounds;
        let mut out = format!("{} {} {} {}\n", lo[0], lo[1], hi[0], hi[1]);
        let step = 2.0;
        let mut y = hi[1];
        while y >= lo[1] {
            let mut x = lo[0];
            while x <= hi[0] {
                out.push(if g.can_place(dart, [x, y]) { '1' } else { '0' });
                x += step;
            }
            out.push('\n');
            y -= step;
        }
        std::fs::write(std::env::var("RRBTD6_PLACEMAP_OUT").unwrap_or_else(|_| "placement.txt".into()), out).unwrap();
    }

    /// Every tower upgrade resolves to an icon with the HUD's lookup (upgrade name, then its
    /// localised name). Writes "tower|upgrade|icon" lines to RRBTD6_ICONS_OUT for a contact sheet.
    #[test]
    #[ignore]
    fn upgrade_icons_resolve() {
        let g = game();
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted");
        let mut icons = HashMap::new();
        for e in std::fs::read_dir(format!("{root}/ui/upgrades")).unwrap().flatten() {
            let f = e.file_name().to_string_lossy().to_string();
            if let Some(stem) = f.strip_suffix(".png") {
                icons.insert(crate::ui::icon_key(stem), stem.to_string());
            }
        }
        let xml = std::fs::read_to_string(format!("{root}/text/English.xml")).unwrap_or_default();
        let loc = |k: &str| {
            let tag = format!("<T id=\"{k}\">");
            xml.find(&tag).map(|i| xml[i + tag.len()..].split('<').next().unwrap_or("").to_string())
        };
        let (mut out, mut missing) = (String::new(), Vec::new());
        for def in 0..g.tower_defs.len() {
            let Some(tt) = g.tower_tiers[def].as_ref() else { continue };
            if tt.hero.is_some() {
                continue;
            }
            let tower = &g.tower_defs[def].0;
            for u in &tt.upgrades {
                let icon = crate::ui::find_icon(&icons, &u.name, tower, u.path as usize, u.tier as usize)
                    .or_else(|| loc(&u.name).and_then(|l| crate::ui::find_icon(&icons, &l, tower, u.path as usize, u.tier as usize)));
                match &icon {
                    Some(i) => out.push_str(&format!("{tower}|{}|{i}
", u.name)),
                    None => missing.push(format!("{tower}: {}", u.name)),
                }
            }
        }
        if let Ok(p) = std::env::var("RRBTD6_ICONS_OUT") {
            std::fs::write(p, out).unwrap();
        }
        assert!(missing.is_empty(), "no icon: {missing:?}");
    }

    /// Every exported map loads into a game and its round bloons can run (none silently falls
    /// back to another map).
    #[test]
    fn every_map_loads() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let json = std::fs::read_to_string(p).unwrap();
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/maps");
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let mp = e.path().join("map.json");
            let Ok(text) = std::fs::read_to_string(&mp) else { continue };
            let mut d: crate::data::GameData = serde_json::from_str(&json).unwrap();
            d.map = serde_json::from_str(&text).unwrap_or_else(|err| panic!("{}: {err}", mp.display()));
            let mut g = Game::new(d);
            g.start_round();
            for _ in 0..600 {
                g.step();
            }
            n += 1;
        }
        assert!(n >= 2, "only {n} maps");
    }

    /// P-BROADSIDE: the Buccaneer's back-side gun fires only with its main gun, not at its own
    /// 0.05 s rate.
    #[test]
    fn buccaneer_broadside() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/game/game.json");
        let mut d: crate::data::GameData = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let m = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/maps/logs/map.json");
        d.map = serde_json::from_str(&std::fs::read_to_string(m).expect("logs map exported")).unwrap();
        let pond = d.map.areas.iter().find(|a| a.kind == 1).expect("a water area").points.clone();
        let mut g = Game::new(d);
        g.cash = 1e9;
        let c = [pond.iter().map(|q| q[0]).sum::<f32>() / pond.len() as f32, pond.iter().map(|q| q[1]).sum::<f32>() / pond.len() as f32];
        let def = g.tower_defs.iter().position(|(x, _)| x == "MonkeyBuccaneer").unwrap();
        assert!(g.place(def, c));
        let t = g.towers.len() - 1;
        g.round = 10;
        g.start_round();
        for _ in 0..1200 {
            g.step();
        }
        let shots = g.towers[t].shots;
        assert!(g.towers[t].pops > 0, "the buccaneer pops");
        // Main gun at most once a second plus its paired gun: well under 20 shots a second.
        assert!(shots <= 2 * 21, "{shots} shots in 20 s");
    }
}
