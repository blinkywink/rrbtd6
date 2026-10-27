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

#[path = "game/race.rs"]
mod race;
#[path = "game/coop_state.rs"]
mod coop_state;
#[path = "game/coop_cash.rs"]
mod coop_cash;
pub use race::Race;

use crate::data::{AREA_LAND, AREA_SKIPPED, Area, BloonDef, Brew, Emission, GameData, Proj, TagMod, Tier, TowerDef, TowerTiers};
use std::collections::HashMap;

mod geraldo;

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
    race_round:usize,
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
}

#[derive(Clone)]
pub enum EffectKind {
    Slow { mult: f32, glue: u32 },
    Freeze,
    /// DamageOverTime: `interval` in frames; `tick` is lastTick on the round clock (whole frames).
    Dot { damage: f32, interval: f32, tick: f32, immune: u32 },
    Brittle { damage: f32 },
    Emit { interval: f32, tick: f32, proj: Box<Proj> },
    /// EmitOnPopModel: fired where the bloon pops.
    PopEmit { proj: Box<Proj>, emission: Box<Emission> },
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
}

impl Bloon {
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
            "Stun" => self.effects.iter().any(|e| matches!(e.kind, EffectKind::Slow { mult, glue: 0 } if mult == 0.0)),
            _ => self.has_effect(state),
        }
    }

    pub fn has_effect(&self, id: &str) -> bool {
        self.effects.iter().any(|e| e.id == id)
    }

    pub fn frozen(&self) -> bool {
        self.effects.iter().any(|e| matches!(e.kind, EffectKind::Freeze))
    }

    pub fn glue(&self) -> u32 {
        self.effects.iter().map(|e| if let EffectKind::Slow { glue, .. } = e.kind { glue } else { 0 }).max().unwrap_or(0)
    }

    /// HYPOTHESIS P-SLOW: slows with different ids multiply; a freeze stops the bloon.
    pub fn speed_mult(&self) -> f32 {
        if self.frozen() {
            return 0.0;
        }
        self.effects.iter().map(|e| if let EffectKind::Slow { mult, .. } = e.kind { mult } else { 1.0 }).product()
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
    if i + 1 == len0 {
        return;
    }
    for c in cand.iter_mut().skip(k + 1) {
        if *c as usize == len0 - 1 {
            *c = i as u32;
        }
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
    let (s, c) = (a.sin(), a.cos());
    [0.0 * c - s, -(c + s * 0.0)]
}

/// RotateToTarget.ApplyRotation: towers aim from 10 above their position.
const DISPLAY_TOWER_HEIGHT: f32 = 10.0;

/// lastEmission of a weapon that never fired (int.MinValue: ready at once).
const NEVER: i64 = i32::MIN as i64;

/// Simulation.Towers.Weapons.Weapon state.
#[derive(Clone, Debug)]
pub struct WeaponState {
    /// Weapon name and occurrence (the k-th of that name): how a model change finds the same
    /// weapon.
    pub key: (String, u32),
    /// lastEmission: when the last throw started, on the weapon's clock (roundTime, or
    /// Simulation.time for fireBetweenRounds).
    pub last: i64,
    /// animationStartedAt (Simulation.time) of a throw whose projectile hasn't left yet; -1.
    pub anim: i64,
    /// Where that throw was aimed, and at which bloon (0: a point): the throw stays on the
    /// target it started with (recorded: the tower keeps turning to it as it leaves range).
    pub aim_at: [f32; 2],
    pub aim_id: u64,
    /// Target's offset from its bloon (Targetting.CanHitMoab: the collision circle a MOAB was
    /// seen by); the throw leaves towards the bloon's position plus it.
    pub aim_off: [f32; 2],
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
    danger: f32,
    necro: f32,
    /// EmissionsPerRoundFilterModel: throws this round.
    pub round_shots: u32,
    /// CritMultiplier: (current, nextCritAt); (0, 0) until the weapon has a crit.
    pub crit: (u32, u32),
}

impl WeaponState {
    fn new(key: (String, u32), w: &crate::data::Weapon, clock: i64) -> Self {
        let mut s = WeaponState {
            key,
            last: NEVER,
            anim: -1,
            aim_at: [0.0; 2],
            aim_id: 0,
            aim_off: [0.0; 2],
            rate_frames: 1,
            missed: 0.0,
            dmg_carry: 0.0,
            buff_rate: f32::NAN,
            model_rate: f32::NAN,
            dist_k: 1.0,
            danger: 0.0,
            necro: 1.0,
            round_shots: 0,
            crit: (0, 0),
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
        let r = w.rate * self.buff_rate / self.dist_k / (1.0 + self.danger * w.leak_danger) / self.necro;
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
/// the glaives' positions (projectiles of their own in the game, display only).
#[derive(Clone, Debug)]
pub struct OrbitState {
    pub angle: f32,
    pub speed: f32,
    pub seen: Option<u64>,
    pub glaives: Vec<[f32; 2]>,
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

pub struct Tower {
    pub id: u64,
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
    pub temp_supports: Vec<(crate::data::Support, f32)>,
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
    /// Comanche Defense: frames until it can call in again, call-ins this round.
    pub comanche: (f32, u32),
    /// Necromancer graveyard: RBE gathered per round (round, rbe); spent on undead bloons.
    pub grave: Vec<(usize, f32)>,
    /// RBE spent on undead this second (maxRbeSpawnedPerSecond) and the step that second began.
    pub grave_spent: (f32, u64),
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

pub struct TempAttack {
    pub attack: crate::data::Attack,
    pub frames_left: f32,
    /// Removed after its first volley (isOneShot).
    pub one_shot: bool,
    /// The tower's own attacks pause while it runs (turnOffExisting).
    pub pauses: bool,
    /// Frames before it starts.
    pub delay: f32,
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
}

#[derive(Clone)]
pub struct ActiveBrew {
    pub brew: Brew,
    pub shots_left: u32,
    /// Frames left (infinite without a time limit).
    pub frames_left: f32,
}

impl Tower {
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
}

const EMPTY: u32 = u32::MAX;

impl GameGrid {
    fn cell(&self, p: [f32; 2]) -> usize {
        let (x, y) = grid_cell(p);
        (y.clamp(0, self.h - 1) * self.w + x.clamp(0, self.w - 1)) as usize
    }

    /// CollisionGrid.Fill: the bloons in the factory's list order (`key`), all but those made
    /// this frame (ids from `new_from`: the spawner and pops run after the grid is filled).
    fn build(&mut self, bloons: &[Bloon], key: impl Fn(&Bloon) -> u64, new_from: u64) {
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
        let mut idx: Vec<u32> = (0..bloons.len() as u32).filter(|&i| bloons[i as usize].id < new_from).collect();
        idx.sort_unstable_by_key(|&i| key(&bloons[i as usize]));
        for i in idx {
            let c = self.cell(bloons[i as usize].cell_pos);
            self.of[i as usize] = c as u32;
            self.slot[i as usize] = self.cells[c].len() as u32;
            self.cells[c].push(i);
        }
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
    big: std::collections::HashSet<u64>,
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
    /// Its FollowPath is mirrored (thrown by the other hand: FlipFollowPath).
    pub mirror: bool,
    pub hit: HitSet,
    /// A bounce (clearCollidedWith) empties `hit` once this frame's collisions are done.
    pub clear_hits_next: bool,
    /// Tiers of the tower that fired it (display only).
    pub tiers: [u8; 3],
    /// Id of the tower that fired it (pop credit).
    pub owner: u64,
    /// Instant projectiles: the only bloon they can hit. Bouncing glaives: the bloon they home in on.
    pub target: Option<u64>,
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
    /// ClearHitBloons.lastClear: Simulation.time the hit list was last cleared (made at).
    clear_t: f32,
    /// Frames since fired (display spin).
    pub age: f32,
    /// Seeking projectiles: the bloon they steer towards.
    seek: Option<u64>,
    /// Arrive-and-stay projectiles (caltrops): frames until they reach their spot and stop.
    moving: f32,
    /// ArriveAtTarget: (startFrame, origPos, targetPos).
    arrive_at: Option<(u64, [f32; 2], [f32; 2])>,
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
    /// Projectile radius multiplier (Long Arm of Light).
    pub radius_mult: f32,
    pub range_mult: f32,
    pub range_add: f32,
    pub rate_mult: f32,
    pub pierce_add: f32,
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
            radius_mult: 1.0,
            range_mult: 0.0,
            range_add: 0.0,
            rate_mult: 1.0,
            pierce_add: 0.0,
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

/// Whether bloon tags `tags` match a DamageModifierForTagModel's tag list.
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
        if r >= 1e5 {
            return r;
        }
        let r = r * (1.0 + self.range_mult) + self.range_add;
        match &self.fan {
            Some(fc) => r.max(fc.range),
            None => r,
        }
    }

    /// The projectile as fired under these buffs (None if unchanged). Damage-type support reaches
    /// sub-projectiles too (explosions); pierce and speed only the shot itself.
    fn projectile(&self, p: &Proj) -> Option<Proj> {
        if self.pierce_add == 0.0 && self.speed_mult == 0.0 && self.immune.is_none() && self.damage_add == 0.0 && self.damage_mult == 1.0 && self.radius_mult == 1.0 && self.tag_mods.is_empty() && self.fan.is_none() {
            return None;
        }
        let mut q = p.clone();
        q.pierce += self.pierce_add;
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
            fn boost(p: &mut Proj, mult: f32, add: f32, mods: &[TagMod]) {
                if p.damage > 0.0 {
                    p.damage = p.damage * mult + add;
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

pub struct Game {
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
    /// Path.bloonsList per path: bloon ids and whether they're still alive, in the game's order
    /// (appended when made; popped ones stay until the flush every 60 frames), and where each
    /// id sits.
    path_lists: Vec<Vec<(u64, bool)>>,
    path_rank: std::collections::HashMap<u64, (u8, u32)>,
    /// On a flush frame, the list places from before it: the collision grid is filled in that
    /// order (recording dart_052: a Blue ahead of a Yellow the flush moved in front of it).
    grid_rank_pre: Option<std::collections::HashMap<u64, u32>>,
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
    /// Simulation.time.elapsed: frames simulated (the factory flush runs when it is a multiple
    /// of 60).
    pub sim_elapsed: u64,
    rbe: Vec<f32>,
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
    geraldo_stock: Vec<geraldo::ItemStock>,
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
    /// Cash collected since last drained: where and how much (display only).
    pub cash_pops: Vec<([f32; 2], f64)>,
    /// ShowTextOnHit texts this frame (the renderer draws "Crit!" there).
    pub hit_texts: Vec<[f32; 2]>,
    /// Upgrades bought this frame, for the upgrade effect: (tower position, effect prefab).
    pub upgrade_fx: Vec<([f32; 2], &'static str)>,
    /// Towers placed this frame: (position, placement effect prefab, scale).
    pub place_fx: Vec<([f32; 2], String, f32)>,
    /// The round Monkey City's free Dart Monkey was taken.
    free_tower_round: Option<usize>,
    /// MOAB classes a hero has already warned about this round
    /// (CreateSoundOnBloonEnterTrack.ResetPlayedBooleans on round start).
    voice_warned: Vec<String>,
    /// EmitOnPopModel shots from bloons that popped: (where, projectile, emission, owner).
    pop_emits: Vec<([f32; 2], Box<Proj>, Box<Emission>, u64)>,
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
    pub race:Option<Race>,
    pub challenge_limits:Option<Race>,
    pub mode_name: String,
    /// The round index the difficulty starts at (StartingRoundModModel - 1).
    pub start_round: usize,
    /// Frames of Syphon Funding left (P-SYPHON).
    pub syphon: f32,
    /// Seconds spent per step phase (effects + movement, towers, projectiles), for profiling.
    pub prof: [f64; 3],
    pub prof_max: [f64; 3],
    /// Support buffs on each tower, refreshed every step.
    pub buffs: Vec<Buff>,
    /// Sounds since last drained (display only).
    pub sfx: Vec<Sfx>,
    next_id: u64,
    /// Bloon.layerId counter.
    next_layer: u64,
    /// next_id after this step's collisions (what this step's pops made sits below it).
    popped_born_to: u64,
    pub game_over: bool,
    /// The mode's last round was beaten (the Victory screen); `freeplay` then continues.
    pub won: bool,
    pub freeplay: bool,
    /// The mode's last round (EndRoundModel); the round set goes on to 140 for freeplay.
    pub end_round: usize,
    /// STAND-IN random source (xorshift), not the game's seeded RNG: Distraction's blow-back
    /// chance and distance, shuriken spread.
    rng: u64,
    /// Picks sound variants, kept apart from `rng` so sounds don't change the simulation.
    sfx_rng: u64,
    /// The cursor on the map, for TargetPointerModel attacks. Kept when the cursor leaves the
    /// map (DartlingMaintainLastPosModel).
    pub pointer: Option<[f32; 2]>,
    pub(crate) coop_pointers: HashMap<u64, Option<[f32; 2]>>,
    pub(crate) coop_wallets: Option<rrbtd6_coop::economy::Wallets>,
    /// Indices of towers removed during `step` (expired sub-towers), in removal order, so the
    /// caller can fix indices it holds.
    pub removed: Vec<usize>,
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
        let mut tower_defs: Vec<(String, TowerDef)> = d.towers.into_iter().collect();
        let shop_pos = |n: &str| d.shop.iter().position(|s| s.id == n).unwrap_or(usize::MAX);
        tower_defs.sort_by(|a, b| shop_pos(&a.0).cmp(&shop_pos(&b.0)).then(a.0.cmp(&b.0)));
        let tower_tiers = tower_defs.iter().map(|(n, _)| d.tower_tiers.get(n).cloned()).collect();
        Self {
            base_defs: defs.iter().map(|d| (d.speed, d.max_health)).collect(),
            defs,
            names,
            max_bloon_radius,
            damage_tally: 0,
            has_grid_rank: !d.grid_offsets.is_empty(),
            grid_offsets: d.grid_offsets.clone(),
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
            path_lists: Vec::new(),
            path_rank: Default::default(),
            grid_rank_pre: None,
            proj_list: Vec::new(),
            pop_depth: 0,
            pop_nested: false,
            pop_first_id: 0,
            step_first_id: 0,
            phase: 0,
            sim_elapsed: 0,
            by_name,
            rbe,
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
            routs: vec![0; n_splitters],
            track: match tracks.first() {
                Some(p) if p.points == d.map.path => Track::with_info(p.points.clone(), &p.dists, &p.speeds, &p.quick),
                _ => Track::new(d.map.path),
            },
            blockers: d.map.areas.iter().flat_map(|a| a.blockers.iter().map(move |&c| (c, a.is_disabled))).collect(),
            blocker_grid: {
                let mut g: Vec<(i32, Vec<[f32; 4]>)> = vec![(0, Vec::new()); 40 * 33];
                for a in d.map.areas.iter().filter(|a| !a.is_disabled) {
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
            },
            areas: {
                let base = Area {
                    kind: AREA_LAND,
                    points: vec![[-WORLD_X, -WORLD_Z], [WORLD_X, -WORLD_Z], [WORLD_X, WORLD_Z], [-WORLD_X, WORLD_Z]],
                    height: 0.0,
                    blockers: Vec::new(),
                    is_disabled: false,
                    is_blocker: false,
                    holes: Vec::new(),
                };
                std::iter::once(base).chain(d.map.areas).collect()
            },
            // Without an exported terrain extent: the game's world bounds.
            bounds: d.map.terrain.as_ref().map_or(([-WORLD_X, -WORLD_Z], [WORLD_X, WORLD_Z]), |t| (t.min, t.max)),
            has_terrain: d.map.terrain.is_some(),
            leak_immune: 0.0,
            end_round: mode.end_round.min(d.rounds.len()),
            won: false,
            freeplay: false,
            rounds: d.rounds,
            apopalypse: mode.apopalypse && !d.freeplay.is_empty(),
            geraldo_items: d.geraldo,
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
            race:None,challenge_limits:None,
            round: mode.start_round.saturating_sub(1),
            round_active: false,
            round_frame: 0,
            sandbox_queue: Vec::new(),
            steps: 0,
            round_clock: 0,
            pop_after_move: false,
            proj_cursor: usize::MAX,
            distribute: (true, false),
            next_uid: 0,
            active_rbe: 0.0,
            cash: mode.cash,
            debt: 0.0,
            debt_rate: 0.0,
            lives: mode.lives,
            start_lives: mode.lives,
            danger: 0.0,
            leaked: false,
            popped: 0,
            pops: Vec::new(),
            bursts: Vec::new(),
            tracers: Vec::new(),
            hit_texts: Vec::new(),
            cash_pops: Vec::new(),
            upgrade_fx: Vec::new(),
            place_fx: Vec::new(),
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
            no_sell: mode.no_sell,
            health_mods: mode.health.clone(),
            mode_name: if d.modes.contains_key(&d.mode) { d.mode.clone() } else { "Medium".into() },
            start_round: mode.start_round.saturating_sub(1),
            prof_max: [0.0; 3],
            buffs: Vec::new(),
            sfx: Vec::new(),
            sfx_rng: 0x2545_F491_4F6C_DD1D,
            pointer: None,
            coop_pointers: HashMap::new(),
            coop_wallets: None,
            removed: Vec::new(),
            typed_made: 0,
            rng: 0x9E37_79B9_7F4A_7C15,
            next_id: 1,
            next_layer: 0,
            popped_born_to: 0,
            game_over: false,
        }
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
            (round * 4000 - 225000) as f64
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
        if self.race.is_some(){self.request_race_round();return;}
        if self.round_active || self.game_over || self.won || self.round >= self.total_rounds() {
            return;
        }
        self.voice_warned.clear();
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
    pub fn bloon_sprite(&self, name: &str) -> Option<&str> {
        self.by_name.get(name).and_then(|&k| self.defs[k].sprite.as_deref())
    }

    /// Sandbox bloon menu: `count` bloons of type `name` (with the camo / regrow / fortified
    /// variant if it exists, else the closest one that does), `spacing` seconds apart.
    pub fn sandbox_spawn(&mut self, name: &str, camo: bool, regrow: bool, fortified: bool, count: u32, spacing: f32) -> bool {
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
        let alive: std::collections::HashSet<u64> = self.projectiles.iter().map(|p| p.uid).collect();
        let l = &mut self.proj_list;
        let mut i = 0;
        while i < l.len() {
            if alive.contains(&l[i]) {
                i += 1;
            } else {
                l.swap_remove(i);
            }
        }
        let at: std::collections::HashMap<u64, usize> = l.iter().enumerate().map(|(k, &u)| (u, k)).collect();
        self.projectiles.sort_by_key(|p| at.get(&p.uid).copied().unwrap_or(usize::MAX));
    }

    /// Sandbox: every bloon gone (and the round's / sandbox's queued ones).
    pub fn clear_bloons(&mut self) {
        self.bloons.clear();
        self.path_lists.clear();
        self.path_rank.clear();
        self.pending.clear();
        self.sandbox_queue.clear();
        if self.grid_on {
            self.grid = Default::default();
        }
    }

    /// Sandbox: every tower gone (no refund).
    pub fn clear_towers(&mut self) {
        while let Some(t) = self.towers.len().checked_sub(1) {
            self.towers.swap_remove(t);
            self.removed.push(t);
        }
    }

    /// Sandbox: every ability ready.
    pub fn reset_cooldowns(&mut self) {
        for tw in &mut self.towers {
            tw.ability_cd.clear();
        }
    }

    /// Sandbox: every tower's pop count back to 0.
    pub fn reset_damage(&mut self) {
        for tw in &mut self.towers {
            tw.pops = 0;
        }
    }

    /// Sandbox "Set Round": round `n` (1-based) starts now.
    pub fn set_round(&mut self, n: usize) {
        self.pending.clear();
        self.round_active = false;
        self.round = n.clamp(1, self.rounds.len()) - 1;
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

    /// A round / sandbox / race emission onto a spawn path.
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
        let id = self.next_id;
        self.next_id += 1;
        let pos = self.tracks[path as usize].bloon_pos(dist);
        let track_mult = self.tracks[path as usize].speed_mult(dist);
        let grow_t = self.grow_kinds[kind].map_or(0, |g| g.1);
        let grow_ceiling = self.defs[kind].layer_number;
        let race_round=self.race_bloon_spawn();
        self.bloons.push(Bloon { id, kind,race_round, dist, pos, hp: self.defs[kind].max_health, effects: Vec::new(), decamo: false, path, base: 0.0, grow_t, grow_ceiling, fresh: false, prev: pos, prev_dist: dist, mult: 1.0, cell_pos: pos, track_mult, order: id, lineage: None, layer: { self.next_layer += 1; self.next_layer }, rot: self.path_rot(path, dist).unwrap_or(0.0), born: self.sim_elapsed * 4 + self.phase as u64 });
        self.plist_add(id, path);
        if self.grid_on {
            self.grid.add(self.bloons.len() - 1, pos);
            if self.has_grid_rank {
                self.ggrid.push(self.bloons.len() - 1);
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
        if !self.race_can_place(def){return false;}
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
    /// the query's own cell, else within radius + VsBlockerRadius. INFERRED: projectiles fly
    /// below every blocker (heights 100 on Monkey Meadow, darts at 7).
    fn blocker_hit(&self, q: [f32; 2], vsr: f32) -> bool {
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
                if b[3] > 0.0 && (k == 0 || (b[0] - gx).powi(2) + (b[1] - gy).powi(2) <= (b[2] + r) * (b[2] + r)) {
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
    /// `GameModel.selectedDifficultyRating` and the map-difficulty scale (Monkey Meadow is
    /// beginner: 1.0). HYPOTHESIS H-XP-DIFF: selectedDifficultyRating is 1.0 here.
    pub fn xp_for_round(r: i32) -> f32 {
        let a = r + 2 + (r - 19).max(0);
        (20 * a + 50 * (r - 49).max(0)) as f32
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
        // One hero per game.
        if self.hero_def(def).is_some() && self.towers.iter().any(|t| self.hero_def(t.def).is_some()) {
            return false;
        }
        let cost = self.place_cost(def, p);
        if self.cash < cost || (checked && !self.can_place(def, p)) {
            return false;
        }
        if self.free_tower(def).is_some() {
            self.free_tower_round = Some(self.round);
        }
        self.cash -= cost;
        let id = self.next_id;
        self.next_id += 1;
        // CreateEffectOnPlaceModel: PlacementEffect / HeroPlacementEffect where it lands.
        if let Some((fx, scale)) = self.tower_defs[def].1.place_fx.clone() {
            self.place_fx.push((p, fx, scale));
        }
        let deck = self.deck_at(def, p).map(|(sh, h)| (self.towers[sh].id, h));
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
            weapons: Vec::new(),
            shots: 0,
            angle: -std::f32::consts::FRAC_PI_2,
            aim: -std::f32::consts::FRAC_PI_2,
            game_rot: None,
            base_rot: 0.0,
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
            grave: Vec::new(),
            grave_spent: (0.0, 0),
            comanche: (0.0, 0),
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
            self.towers[t].point = self.closest_track_point(p, f32::MAX, 0.0).map(|(_, q, _)| q);
        }
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

    /// Beast Handler birds (GyrfalconPatternModel). STAND-IN P-BIRD: the bird hovers over its
    /// perch; off cooldown it dives at the bloon its target mode picks in range, grabs bloons
    /// around it (initial damage), then holds them in place and hits them every grapple period.
    /// When they're all popped it grabs again near itself if keepPickingUpBloons, else returns.
    /// The player-chosen dive point, carrying bloons back and pet power levels aren't done.
    fn fly_birds(&mut self) {
        for t in 0..self.towers.len() {
            let Some(g) = self.tier(t).and_then(|tr| tr.grapple.clone()) else { continue };
            self.init_air(t);
            let (home, range, mode, id) = (self.towers[t].pos, self.range(t), self.towers[t].target, self.towers[t].id);
            let owner = match &self.towers[t].sub {
                Some(s) if s.spec.credit_parent => s.parent,
                _ => id,
            };
            let bloons = &self.bloons;
            let a = self.towers[t].air.as_mut().unwrap();
            a.grab.retain(|g| bloons.iter().any(|b| b.id == *g));
            let (pos, dive, grabbed) = (a.pos, a.dive, a.grab.clone());
            if !grabbed.is_empty() {
                let lead = self.bloons.iter().find(|b| b.id == grabbed[0]).map_or(pos, |b| b.pos);
                for b in self.bloons.iter_mut().filter(|b| grabbed.contains(&b.id)) {
                    b.effects.retain(|e| e.id != "BeastGrab");
                    b.effects.push(Effect { kind: EffectKind::Slow { mult: 0.0, glue: 0 }, id: "BeastGrab".into(), frames_left: 2.0, layers: 1, owner, distribute: (true, false) });
                }
                let a = self.towers[t].air.as_mut().unwrap();
                a.pos = lead;
                a.height = 15.0;
                a.tick -= 1.0;
                if a.tick > 0.0 {
                    continue;
                }
                a.tick = g.rate * FPS;
                self.towers[t].attack_t = 0.0;
                self.bird_hit(&grabbed, &g, g.grapple, g.grapple_moab, owner);
                if grabbed.iter().any(|i| self.bloons.iter().any(|b| b.id == *i)) {
                    continue;
                }
                let more = if g.keep { self.bird_grab(lead, g.keep_range, &g) } else { Vec::new() };
                self.bird_hit(&more, &g, g.initial, g.initial_moab, owner);
                let a = self.towers[t].air.as_mut().unwrap();
                if more.is_empty() {
                    a.wait = g.cooldown * FPS;
                }
                a.grab = more;
                continue;
            }
            if let Some(target) = dive {
                let Some(goal) = self.bloons.iter().find(|b| b.id == target).map(|b| b.pos) else {
                    self.towers[t].air.as_mut().unwrap().dive = None;
                    continue;
                };
                let a = self.towers[t].air.as_mut().unwrap();
                let (d, len) = ([goal[0] - a.pos[0], goal[1] - a.pos[1]], (goal[0] - a.pos[0]).hypot(goal[1] - a.pos[1]));
                let step = g.speed * g.dive_mult / FPS;
                a.height = (a.height - 1.0).max(15.0);
                if len > 1e-3 {
                    a.angle = d[1].atan2(d[0]);
                }
                if len > step.max(5.0) {
                    a.pos = [a.pos[0] + d[0] / len * step, a.pos[1] + d[1] / len * step];
                    continue;
                }
                a.pos = goal;
                a.dive = None;
                let grabbed = self.bird_grab(goal, g.radius, &g);
                self.bird_hit(&grabbed, &g, g.initial, g.initial_moab, owner);
                self.towers[t].attack_t = 0.0;
                let a = self.towers[t].air.as_mut().unwrap();
                if grabbed.is_empty() {
                    a.wait = g.cooldown * FPS;
                }
                a.grab = grabbed;
                a.tick = g.rate * FPS;
                continue;
            }
            // Idle: back to the perch, then pick a bloon in range.
            let a = self.towers[t].air.as_mut().unwrap();
            let (d, len) = ([home[0] - a.pos[0], home[1] - a.pos[1]], (home[0] - a.pos[0]).hypot(home[1] - a.pos[1]));
            let step = g.speed / FPS;
            a.pos = if len <= step { home } else { [a.pos[0] + d[0] / len * step, a.pos[1] + d[1] / len * step] };
            if len > 1.0 {
                a.angle = d[1].atan2(d[0]);
            }
            a.height = (a.height + 1.0).min(30.0);
            a.wait -= 1.0;
            if a.wait > 0.0 {
                continue;
            }
            let rr = range * range;
            let key = |b: &Bloon| match mode {
                TargetMode::Last => -b.progress(),
                TargetMode::Close => -(b.pos[0] - home[0]).hypot(b.pos[1] - home[1]),
                TargetMode::Strong => self.rbe[b.kind] as f32 * 1e4 + b.progress(),
                _ => b.progress(),
            };
            let pick = self
                .bloons
                .iter()
                .filter(|b| on_map(b.pos, 0.0) && (b.pos[0] - home[0]).powi(2) + (b.pos[1] - home[1]).powi(2) <= rr)
                .filter(|b| self.grab_cost(b, &g) <= g.amount)
                .max_by(|x, y| key(x).total_cmp(&key(y)))
                .map(|b| b.id);
            self.towers[t].air.as_mut().unwrap().dive = pick;
        }
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
        let tags = &self.defs[b.kind].tags;
        if tags.iter().any(|t| t == "Bad") {
            return u32::MAX;
        }
        1 + tags.iter().filter_map(|t| g.penalty.get(t)).copied().max().unwrap_or(0)
    }

    /// Ids of the bloons a bird grabs around `p`: nearest first while the grab amount lasts.
    fn bird_grab(&self, p: [f32; 2], radius: f32, g: &crate::data::Grapple) -> Vec<u64> {
        let mut near: Vec<(f32, &Bloon)> = self
            .bloons
            .iter()
            .map(|b| ((b.pos[0] - p[0]).hypot(b.pos[1] - p[1]) - self.defs[b.kind].radius, b))
            .filter(|(d, _)| *d <= radius)
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut left = g.amount;
        let mut out = Vec::new();
        for (_, b) in near {
            let c = self.grab_cost(b, g);
            if c <= left {
                left -= c;
                out.push(b.id);
            }
        }
        out
    }

    /// Hits the grabbed bloons for `dmg` (+`moab` on MOAB-class, +regrowDamageModifier on regrow).
    fn bird_hit(&mut self, ids: &[u64], g: &crate::data::Grapple, dmg: f32, moab: f32, owner: u64) {
        let mut kids = Vec::new();
        for id in ids {
            let Some(i) = self.bloons.iter().position(|b| b.id == *id) else { continue };
            let tags = &self.defs[self.bloons[i].kind].tags;
            let extra = if tags.iter().any(|t| t == "Moabs") { moab } else { 0.0 } + if tags.iter().any(|t| t == "Grow") { g.regrow_add } else { 0.0 };
            let tally0 = self.damage_tally;
            let n = self.damage(i, (dmg + extra).round() as i32, &mut kids);
            let self_tally_dealt = self.damage_tally - tally0;
            if let Some(tw) = self.towers.iter_mut().find(|t| t.id == owner) {
                tw.pops += n;
                tw.damage += self_tally_dealt;
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
            weapons: Vec::new(),
            shots: 0,
            angle: -std::f32::consts::FRAC_PI_2,
            aim: -std::f32::consts::FRAC_PI_2,
            game_rot: None,
            base_rot: 0.0,
            turret_rel: 0.0,
            base_state: (false, 0, false, 0.0),
            aims: Vec::new(),
            attack_t: f32::MAX,
            idle_t: 0.0,
            bank: 0.0,
            brews: Vec::new(),
            sub: Some(SubTower { frames_left: if spec.lifespan > 0.0 { spec.lifespan * FPS } else { f32::INFINITY }, age: 0.0, rounds_left: spec.rounds, pet, spec, parent }),
            air: None,
            ability_cd: HashMap::new(),
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
            grave: Vec::new(),
            grave_spent: (0.0, 0),
            comanche: (0.0, 0),
            left_target: TargetMode::First,
            camo_prio: false,
            submerged: false,
            air_reverse: false,
            reverse_at: 0,
            flip: false,
        });
        let n = self.towers.len() - 1;
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
        if pet.base.is_none() && pet.model.is_none() {
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
                self.removed.push(i);
            }
        }
        for pet in pets {
            if let Some(q) = self.pet_spot(t, &pet) {
                self.spawn_sub(pet, q, id, true);
            }
        }
    }

    /// PathMovementFromScreenCenterModel (Phoenix): circles the map centre at `speed`.
    /// STAND-IN P-PHOENIXPATH: starting at angle 0, counter-clockwise.
    fn circle_subs(&mut self) {
        let c = [(self.bounds.0[0] + self.bounds.1[0]) / 2.0, (self.bounds.0[1] + self.bounds.1[1]) / 2.0];
        for tw in &mut self.towers {
            let Some(s) = &tw.sub else { continue };
            let Some(ci) = &s.spec.circle else { continue };
            let age = s.age;
            let a = age / FPS * ci.speed / ci.radius.max(1.0);
            let next = [c[0] + ci.radius * a.cos(), c[1] + ci.radius * a.sin()];
            tw.angle = (next[1] - tw.pos[1]).atan2(next[0] - tw.pos[0]);
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

    /// Removes expired sub-towers (indices go to `removed`).
    fn expire_subs(&mut self, round_end: bool) {
        for t in (0..self.towers.len()).rev() {
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
            if s.frames_left <= 0.0 || (round_end && s.spec.round_end) || rounds_up {
                if let Some((Some(fx), k)) = s.spec.expire_fx.clone() {
                    let p = self.towers[t].pos;
                    self.place_fx.push((p, fx, k));
                }
                self.towers.remove(t);
                self.removed.push(t);
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

    /// Gives tower `t` a cooldown entry for each ability its tier has. STAND-IN P-ABILITYSTART:
    /// a newly gained ability starts on full cooldown unless startOffCooldown.
    fn sync_abilities(&mut self, t: usize) {
        let Some(tier) = self.tier(t) else { return };
        let fresh: Vec<(String, f32)> = tier
            .abilities
            .iter()
            .filter(|a| !self.towers[t].ability_cd.contains_key(&a.name))
            .map(|a| (a.name.clone(), if a.ready { 0.0 } else { a.cooldown * FPS }))
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
        if !self.round_active {
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
                let single = Emission { kind: "Single".into(), count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0 };
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
        if !self.round_active || target >= self.towers.len() || self.towers[target].sub.is_some() {
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

    pub fn activate_ability(&mut self, name: &str) -> bool {
        if !self.round_active {
            return false;
        }
        for t in 0..self.towers.len() {
            let Some(a) = self.tier(t).and_then(|tr| tr.abilities.iter().find(|a| a.name == name)).cloned() else { continue };
            if self.towers[t].ability_cd.get(name).copied().unwrap_or(0.0) > 0.0 {
                continue;
            }
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
        let at: HashMap<u64, ([f32; 2], f32)> = self.bloons.iter().map(|b| (b.id, (b.pos, self.defs[b.kind].radius))).collect();
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
        let tw = &mut self.towers[t];
        for x in &a.activate {
            for at in &x.attacks {
                tw.temp.push(TempAttack {
                    attack: at.clone(),
                    frames_left: x.lifespan * FPS,
                    one_shot: x.one_shot,
                    pauses: x.turn_off_existing,
                    delay: x.delay * FPS,
                });
            }
        }
        if let Some(tb) = &a.turbo {
            tw.turbo = Some(ActiveTurbo { mult: tb.mult, extra_damage: tb.extra_damage, frames_left: tb.lifespan * FPS });
        }
        for sp in &a.supports {
            tw.temp_supports.retain(|(o, _)| o.id != sp.id);
            tw.temp_supports.push((sp.clone(), sp.lifespan * FPS));
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
            tw.mute = tw.mute.max(a.mute * FPS);
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
                    kind: EffectKind::Dot { damage: br.damage, interval: br.interval * FPS, tick: self.round_clock as f32, immune: br.immune },
                    id: "MoabBarrage".into(),
                    frames_left: br.lifespan * FPS,
                    layers: 1,
                    owner,
                    distribute: (true, false),
                });
            }
        }
        // Sword Charge: the projectile starts at the end of every track (the start if it
        // doesn't run in reverse).
        if let Some(proj) = &a.track_charge {
            let (tiers, owner) = (self.towers[t].tiers, self.towers[t].id);
            let single = Emission { kind: "Single".into(), count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0 };
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
                let single = Emission { kind: "Single".into(), count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0 };
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
        self.tower_tiers[tw.def].as_ref()?.tiers.get(&tw.tier_key())
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
        if !self.race_upgrade_allowed(t,path){return None;}
        // A Paragon has no more upgrades.
        if self.towers[t].paragon.is_some() {
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
                let (c, r) = (tw.pos, self.tier(t).map_or(0.0, |x| x.range));
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
            let gone: Vec<u64> = all.iter().map(|&o| self.towers[o].id).filter(|&o| o != id).collect();
            self.towers.retain(|o| !gone.contains(&o.id) && !o.sub.as_ref().is_some_and(|s| gone.contains(&s.parent)));
            let t = self.towers.iter().position(|o| o.id == id).unwrap();
            let kept: Vec<String> = self.tier(t).map(|tr| tr.attacks.iter().map(|a| a.model_name.clone()).collect()).unwrap_or_default();
            let tier = std::sync::Arc::new(self.paragon_tier(&pdef.tier, degree));
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
            // STAND-IN: the Paragon's own transformation effect isn't exported; tier 5's.
            self.upgrade_fx.push((self.towers[t].pos, "UpgradeEffectTier5"));
            true
        }
    }

    fn upgrade_bought(&mut self, t: usize, path: usize, cost: f64) -> bool {
        // Was a Sun Temple (Some(false)) / True Sun God (Some(true)) before.
        let was_temple = self.tier(t).and_then(|x| x.temple.as_ref()).map(|tp| tp.groups.iter().any(|g| g.two));
        let had_switch = self.tier(t).is_some_and(|x| x.switch_target.is_some());
        self.cash -= cost;
        let tw = &mut self.towers[t];
        tw.tiers[path] += 1;
        tw.invested += cost;
        tw.xp = 0.0; // a bought hero level spends the XP towards it
        // The new model's upgrade state (AnimationBakerStateConfig.isUpgradeAnimation), if any.
        tw.show = Some((crate::anim::UPGRADE_CLIP.into(), 0.0));
        // SwitchTargetSupplierOnUpgrade (Ice x-x-3: First, Sniper x-5-x: Elite). HYPOTHESIS
        // P-SWITCHTARGET: only when the tier before didn't already have it.
        if !had_switch {
            if let Some(m) = self.tier(t).and_then(|x| x.switch_target.as_deref()).and_then(TargetMode::parse) {
                self.towers[t].target = m;
            }
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
            self.sell(o);
        }
        let Some(t) = self.towers.iter().position(|x| x.id == id) else { return };
        let frac = self.sell_fraction(t);
        let tw = self.towers.swap_remove(t);
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
            for sp in tier.supports.iter().chain(src.temp_supports.iter().map(|(sp, _)| sp)) {
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
                for (sp, n) in self.supports_at(self.towers[t].def, self.towers[t].pos, Some(t)) {
                    let k = n as f32;
                    match sp.kind.as_str() {
                        "range" => {
                            b.range_mult += sp.mult * k;
                            b.range_add += sp.add * k;
                        }
                        "rate" => b.rate_mult *= sp.mult.powi(n as i32),
                        "pierce" => b.pierce_add += sp.add * k,
                        "speed" => b.speed_mult += sp.mult * k,
                        "camo" => b.camo = true,
                        "immune" => b.immune = Some(b.immune.unwrap_or(u32::MAX) & sp.add as u32),
                        "cash" => b.cash_mult *= sp.mult.powi(n as i32),
                        "income" => b.income_mult *= sp.mult.powi(n as i32),
                        "ability_cd" => b.ability_cd_mult *= sp.mult.powi(n as i32),
                        "unfilter" => b.unfilter = true,
                        "damage" => b.damage_add += sp.add * k,
                        // HYPOTHESIS P-SUBCOMMANDER: x damageScale then +damageIncrease.
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
            if t.sub.is_some() {
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
    fn damage(&mut self, i: usize, dmg: i32, out_children: &mut Vec<u64>) -> u64 {
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
        self.necro_gather(kind, self.bloons[i].pos);
        // CreateSoundOnBloonDestroyed (tag Moabs): the hero remarks on a popped MOAB-class bloon.
        if self.defs[kind].tags.iter().any(|t| t == "Moabs") {
            self.hero_voice(|s| &s.moab_pop, VOICE, VOICE_GAP);
        }
        // Bloon.Damage works the whole result out first (BloonDegradeStepper: what's left of the
        // damage goes to every child, layer by layer), then SpawnChildren makes only the bloons
        // that are left: here the layers pop one by one and the survivors are placed after.
        let top = self.pop_depth == 0;
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
        let race_round=b.race_round;
        let (dist, decamo, path, base, ceiling) = (b.dist, b.decamo, b.path, b.base, b.grow_ceiling);
        let regrow = self.grow_kinds[kind].is_some();
        // Grow keeps its timer through a degrade (the game's bloon is the same object): running
        // if the parent was below its ceiling, else it starts now.
        let parent_grow = (self.defs[kind].layer_number < ceiling).then_some(b.grow_t);
        let parent_cell = b.cell_pos;
        let parent_order = b.order;
        let parent_born = b.born;
        let parent_id = b.id;
        let lineage: std::sync::Arc<[u64]> = b.lineage.iter().flat_map(|l| l.iter().copied()).chain(std::iter::once(b.layer)).collect();
        let parent_place = if self.grid_on && self.has_grid_rank { self.ggrid.place_of(i) } else { (EMPTY, EMPTY) };
        let pp0 = b.pos;
        let pop_emits: Vec<_> = b
            .effects
            .iter()
            .filter_map(|e| match &e.kind {
                EffectKind::PopEmit { proj, emission } => Some((pp0, proj.clone(), emission.clone(), e.owner)),
                _ => None,
            })
            .collect();
        self.pop_emits.extend(pop_emits);
        let b = &mut self.bloons[i];
        // Mark / Trojan: pop cash x the mark, + worth per layer; Strip: no children.
        let mut pop_cash = 1.0f64;
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
        let (mut start_dist, mut moved, mult, parent_tm, parent_fresh) = (b.prev_dist, self.phase >= 2 && b.dist != b.prev_dist, b.mult, b.track_mult, b.fresh);
        // A pop in the bloon's own Process after its Move (DamageOverTime): children start from
        // where it is now, the first (the bloon itself) having moved already (recording
        // boomerang_500: the MOAB's DoT pop leaves Ceramics at 213.08 back by 6.675 each).
        if self.pop_after_move {
            (start_dist, moved) = (b.dist, false);
        }
        let inherited: Vec<Effect> = b
            .effects
            .iter()
            .filter(|e| e.layers > 1)
            .map(|e| Effect { layers: e.layers - 1, ..e.clone() })
            .collect();
        self.remove_bloon(i);
        self.earn(pop_cash); // P-CASH
        self.popped += 1;
        let pp = self.tracks[path as usize].pos(dist);
        self.pops.push(pp);
        let n_kids = if strip { 0 } else { self.child_kinds[kind].len() };
        if n_kids == 0 {
            self.bloon_destroyed(parent_id, pp0);
        }
        // SpawnChildren.CreatedChildren (VERIFIED-DISASM @0x1027440): the first child is the
        // parent itself (it moves on, at its new speed); child k sits k x min(26.7 / count, 8.6)
        // behind (not below 0) and, being new, waits for the next frame to move.
        let spacing = (26.7f32 / n_kids.max(1) as f32).min(8.6);
        for n in 0..n_kids {
            let c = self.child_kinds[kind][n];
            if let Some(r)=&mut self.race{r.spawn_round=race_round;}
            let d0 = (start_dist - n as f32 * spacing).max(0.0);
            let d1 = if moved && n == 0 { (d0 + speed_frames(self.defs[c].speed) * parent_tm * 1.0 * mult).max(0.0) } else { d0 };
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
                }
                let j = self.bloons.len() - 1;
                out_children.push(id);
                if self.grid_on && self.has_grid_rank {
                    // (The others only join the grid when it's next filled.)
                    if n == 0 {
                        self.ggrid.take_place(j, parent_place);
                    }
                }
            }
            let child_tags = &self.defs[c].tags;
            let keep: Vec<Effect> = inherited
                .iter()
                .filter(|e| !matches!(e.kind, EffectKind::Freeze) || !child_tags.iter().any(|t| t == "Moabs"))
                .cloned()
                .collect();
            let child = self.bloons.last_mut().unwrap();
            // Made in the collision pass, before this frame's movement: new objects, they move
            // from the next frame (the first child is the parent object and moves on, unless that
            // was made this frame too: recording dart_400, a Lead's second Black's Pinks).
            if self.grid_on {
                child.fresh = n > 0 || parent_fresh;
            }
            child.effects = keep;
            child.decamo = decamo;
            child.base = base;
            // HYPOTHESIS P-GROWCHILD: children of a regrow bloon grow back up to its ceiling
            // (starting a fresh timer).
            if regrow {
                child.grow_ceiling = child.grow_ceiling.max(ceiling);
                if let Some(t) = parent_grow {
                    child.grow_t = t;
                }
            }
            child.cell_pos = parent_cell;
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
                b.dist = dist;
                b.pos = pos;
                b.prev = prev;
                b.prev_dist = d0;
                b.track_mult = tm;
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
                    tw.damage += self_tally_dealt;
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
        let born = self.step_first_id..self.popped_born_to;
        for i in 0..self.bloons.len() {
            let b = &self.bloons[i];
            let Some((to, every)) = self.grow_kinds[b.kind] else { continue };
            if born.contains(&b.id) {
                continue;
            }
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
            b.kind = to;
            b.hp = self.defs[to].max_health;
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

    /// TargetTrackOrDefaultModel with useClosestTrack: the in-range track point nearest the tower
    /// (sampled every 2 units), offset by up to maxTrackOffset.
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

    /// Pops `bloons[i]` in one go: it and every layer inside it count as popped (no children
    /// spawn). Returns the number of pops.
    fn destroy_whole(&mut self, i: usize) -> u64 {
        let n = self.pops_in[self.bloons[i].kind];
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
            let n = self.destroy_whole(i);
            let self_tally_dealt = self.damage_tally - tally0;
            self.earn(n as f64 * td.mult + td.add);
            if let Some(tw) = self.towers.iter_mut().find(|x| x.id == owner) {
                tw.pops += n;
                tw.damage += self_tally_dealt;
            }
            if left == 0 {
                break;
            }
        }
    }

    /// HYPOTHESIS P-PREEMPT: every Pre-emptive Strike Sub launches its strike at each MOAB-class
    /// bloon the round sends, the moment it appears (an instant projectile on it; its
    /// CreateProjectileOnExpireModel does the damage). Children of popped bloons don't count.
    fn preemptive_strikes(&mut self, target: u64) {
        let Some(pos) = self.bloons.iter().find(|b| b.id == target).map(|b| b.pos) else { return };
        let single = Emission { kind: "Single".into(), count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0 };
        for t in 0..self.towers.len() {
            let Some(proj) = self.tier(t).and_then(|tr| tr.preempt.clone()) else { continue };
            let (tiers, owner) = (self.towers[t].tiers, self.towers[t].id);
            let start = self.projectiles.len();
            self.emit(pos, 0.0, &single, &proj, None, tiers, owner);
            for pr in &mut self.projectiles[start..] {
                pr.target = Some(target);
                pr.pos = pos;
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
    /// the bloon it saw while that is alive and within 40 (centre to centre), else asks
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
                    }
                    continue;
                }
                drop_glaives = self.sim_elapsed == first;
            }
            let mut st = self.towers[t].orbit.take().unwrap_or(OrbitState { angle: 0.0, speed: 3.0, seen: None, glaives: Vec::new() });
            let kept = st.seen.and_then(|id| self.bloons.iter().find(|b| b.id == id)).is_some_and(|b| {
                let (dx, dy) = (b.pos[0] - c[0], b.pos[1] - c[1]);
                !(1600.0 < dx * dx + dy * dy)
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
                let perc = (((now - *t0) as f32) * (1.0f32 / 60.0) / ar.time).min(1.0);
                let e = curve_eval(&ar.samples, perc);
                let np = [o[0] + (at[0] - o[0]) * e, o[1] + (at[1] - o[1]) * e];
                pr.vel = [np[0] - pr.pos[0], np[1] - pr.pos[1]];
                pr.pos = np;
            }
        }
    }

    /// Processes (effects, then move) the bloons made before `stamp` that this frame hasn't yet:
    /// they come before it in the process list.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    fn process_bloons_before(&mut self, stamp: u64, early: &mut std::collections::HashSet<u64>, round_clock: i64,
        dots: &mut Vec<(u64, f32, u32, u64, (bool, bool))>, emits: &mut Vec<([f32; 2], Box<Proj>, u64)>,
        zones: &[([f32; 2], f32, crate::data::Zone)], shoves: &[([f32; 2], crate::data::MoabShove)], leak_voice: &mut bool) {
        let mut j = 0;
        while j < self.bloons.len() {
            let b = &self.bloons[j];
            if b.born >= stamp || b.fresh || early.contains(&b.id) {
                j += 1;
                continue;
            }
            early.insert(b.id);
            self.tick_bloon_effects(j, round_clock, dots, emits);
            if !self.move_bloon(j, zones, shoves, leak_voice) {
                j += 1;
            }
        }
    }

    /// Bloon.Process's move for bloon `i` (speed, track, splitters, rotation, leak); true when
    /// it leaked and was removed (the next bloon is now at `i`).
    #[allow(clippy::type_complexity)]
    fn move_bloon(&mut self, i: usize, zones: &[([f32; 2], f32, crate::data::Zone)], shoves: &[([f32; 2], crate::data::MoabShove)], leak_voice: &mut bool) -> bool {
        let mut mult = self.bloons[i].speed_mult();
        if mult > 0.0 && !zones.is_empty() {
            let d = &self.defs[self.bloons[i].kind];
            let p = self.bloons[i].pos;
            for (c, r, z) in zones {
                let rr = r + d.radius;
                if (z.camo || !d.is_camo)
                    && !z.exclude_tags.iter().any(|t| d.tags.contains(t))
                    && (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) <= rr * rr
                {
                    mult = mult.min(z.scale);
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
        let mut moved = None;
        if b.fresh {
            b.fresh = false;
        } else {
            b.mult = mult;
            moved = Some(speed_frames(b.model_speed(self.defs[b.kind].speed)) * b.track_mult * 1.0 * mult);
            // Bloon.get_TotalSpeedFrames (VERIFIED-DISASM @0xfd7730): speedFrames (speed /
            // 60) x trackSpeedMultiplier x the spawner's multiplier (1) x behaviours'.
            b.dist = (b.dist + speed_frames(b.model_speed(self.defs[b.kind].speed)) * b.track_mult * 1.0 * mult).max(0.0);
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
            if self.leak_immune <= 0.0 {
                self.lives -= self.rbe[b.kind] as f64; // P-LEAK
                self.leaked = true;
                *leak_voice = true;
            }
            let (id, at) = (self.bloons[i].id, self.bloons[i].pos);
            self.plist_kill(id);
            self.bloons.swap_remove(i);
            self.bloon_destroyed(id, at);
            return true;
        }
        false
    }

    /// Bloon.Process's mutator timers and DamageOverTime / emit ticks for bloon `i` (the damage
    /// and emissions are applied by the caller, after the moves).
    #[allow(clippy::type_complexity)]
    fn tick_bloon_effects(&mut self, i: usize, round_clock: i64, dots: &mut Vec<(u64, f32, u32, u64, (bool, bool))>, emits: &mut Vec<([f32; 2], Box<Proj>, u64)>) {
        let b = &mut self.bloons[i];
        // (Bloons made this frame aren't in the process list yet.)
        if b.fresh {
            return;
        }
        for e in &mut b.effects {
            e.frames_left -= 1.0;
            match &mut e.kind {
                // DamageOverTime.CheckTick (VERIFIED-DISASM @0x1003ac0): once the
                // round clock is intervalFrames past lastTick (Initialise: the clock when it
                // went on), it ticks and lastTick = now (recording boomerang_500: on at 77,
                // 100 damage at 137).
                EffectKind::Dot { damage, interval, tick, immune } => {
                    if round_clock - *tick as i64 >= (*interval as f64).round_ties_even() as i64 {
                        *tick = round_clock as f32;
                        dots.push((b.id, *damage, *immune, e.owner, e.distribute));
                    }
                }
                EffectKind::Emit { interval, tick, proj } => {
                    *tick += 1.0;
                    if *tick >= *interval * FPS {
                        *tick = 0.0;
                        emits.push((b.pos, proj.clone(), e.owner));
                    }
                }
                _ => {}
            }
        }
        b.effects.retain(|e| e.frames_left > 0.0);
    }

    /// BloonManager.GetClosestTarget (VERIFIED-DISASM @0xfe8700): track segments within
    /// `distance` + 40 of `q`, nearest segment point first; on it, the first bloon in list order
    /// that `ok` lets through whose point (by `at`) is within `distance` + 2 (a MOAB: any of its
    /// circles within `distance` + the circle's radius).
    fn closest_target(&self, q: [f32; 2], distance: f32, ok: impl Fn(&Bloon) -> bool, at: impl Fn(&Bloon) -> [f32; 2]) -> Option<usize> {
        let reach = distance + 40.0;
        let mut best: Option<usize> = None;
        let mut best_d = f32::MAX;
        for (pi, sg) in self.track_segs.iter().enumerate() {
            for (k, sp) in sg.iter().enumerate() {
                let live = self.track_seg_ok.get(pi).and_then(|v| v.get(k)).copied().unwrap_or(false);
                let d2 = (sp[0] - q[0]) * (sp[0] - q[0]) + (sp[1] - q[1]) * (sp[1] - q[1]);
                if !live || (sp[0] - q[0]).powi(2) + (sp[1] - q[1]).powi(2) > reach * reach || !(d2 < best_d) {
                    continue;
                }
                let in_range = (sp[0] - q[0]).powi(2) + (sp[1] - q[1]).powi(2) <= (distance + 2.0) * (distance + 2.0);
                let mut on: Vec<usize> = (0..self.bloons.len())
                    .filter(|&i| self.bloons[i].path as usize == pi && (self.bloons[i].prev_dist / 7.0).max(0.0) as usize == k)
                    .collect();
                on.sort_by_key(|&i| self.path_rank.get(&self.bloons[i].id).map_or(u32::MAX, |r| r.1));
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
        let q = pr.prev;
        let at = |b: &Bloon| if b.born < pr.born { b.pos } else { b.prev };
        let best = self.closest_target(q, rt.distance, |b| !b.hit_by(&pr.hit) && (!b.camo(&self.defs[b.kind]) || pr.camo), at);
        let tpos = best.map(|i| at(&self.bloons[i]));
        let tid = best.map(|i| self.bloons[i].id);
        let pr = &mut self.projectiles[p];
        // (Target.set_bloon: the new bloon becomes its Target.)
        if tid.is_some() {
            pr.aim = tid;
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
        self.race_bloon_remove(self.bloons[i].race_round);
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

    /// NecromancerZone.OnBloonDegrade: a layer popped within a Necromancer's range adds its own
    /// RBE (its RBE less its children's) to that tower's graveyard, up to maxRbeStored.
    /// HYPOTHESIS P-NECROGATHER: every Necromancer in range gets it.
    fn necro_gather(&mut self, kind: usize, p: [f32; 2]) {
        if !self.towers.iter().any(|tw| tw.tiers[2] >= 4) {
            return;
        }
        let own = self.rbe[kind] - self.child_kinds[kind].iter().map(|&c| self.rbe[c]).sum::<f32>();
        let round = self.round;
        for t in 0..self.towers.len() {
            let Some((max, range)) = self.tier(t).and_then(|tr| tr.attacks.iter().find_map(|a| a.necro.as_ref().map(|n| (n.max_stored, a.range)))) else { continue };
            let q = self.towers[t].pos;
            if (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) > range * range {
                continue;
            }
            let tw = &mut self.towers[t];
            let stored: f32 = tw.grave.iter().map(|g| g.1).sum();
            let add = own.max(1.0).min((max - stored).max(0.0));
            if add > 0.0 {
                match tw.grave.last_mut() {
                    Some(g) if g.0 == round => g.1 += add,
                    _ => tw.grave.push((round, add)),
                }
            }
        }
    }

    /// Round end: graveyard RBE older than roundsBeforeDecay rounds is gone.
    fn necro_decay(&mut self) {
        let round = self.round;
        for t in 0..self.towers.len() {
            let Some(rounds) = self.tier(t).and_then(|tr| tr.attacks.iter().find_map(|a| a.necro.as_ref().map(|n| n.decay_rounds))) else { continue };
            self.towers[t].grave.retain(|g| round < g.0 + rounds as usize);
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
    /// Necromancer rate stacks: the weapon's rate / this.
    fn necro_rate(&self, t: usize, nc: &crate::data::Necro) -> f32 {
        let stored: f32 = self.towers[t].grave.iter().map(|g| g.1).sum();
        1.0 + 0.1 * (stored / nc.rate_rbe_per_stack.max(1.0)).floor().min(nc.rate_stack_max)
    }

    fn necro_fire(&mut self, t: usize, nc: &crate::data::Necro, proj: &Proj, target_id: u64, tiers: [u8; 3]) {
        let stored: f32 = self.towers[t].grave.iter().map(|g| g.1).sum();
        let dmg_stacks = (stored / nc.damage_rbe_per_stack.max(1.0)).floor().min(nc.damage_stack_max);
        let (dmg_add, dist_mult) = self
            .towers
            .iter()
            .filter_map(|o| self.tier_of(o).and_then(|tr| tr.zombie_buff))
            .fold((0.0f32, 1.0f32), |(d, m), z| (d.max(z[0]), m.max(z[1])));
        let Some(b) = self.bloons.iter().find(|b| b.id == target_id) else { return };
        let (path, dist) = (b.path, b.dist);
        // The per-second budget.
        let steps = self.steps;
        let tw = &mut self.towers[t];
        if steps >= tw.grave_spent.1 + FPS as u64 {
            tw.grave_spent = (0.0, steps);
        }
        let n = nc.min_per_wave + (self.rand() * (nc.max_per_wave.saturating_sub(nc.min_per_wave) + 1) as f32) as u32;
        let mut sent = 0;
        for _ in 0..n.max(1) {
            let tw = &self.towers[t];
            let left: f32 = tw.grave.iter().map(|g| g.1).sum();
            let budget = nc.rbe_per_second - tw.grave_spent.0;
            let pierce = nc.max_pierce.min(left).min(budget).floor();
            let (spec, pierce) = if pierce >= 1.0 {
                // Spend it, oldest rounds first.
                let mut need = pierce;
                let tw = &mut self.towers[t];
                for g in tw.grave.iter_mut() {
                    let k = g.1.min(need);
                    g.1 -= k;
                    need -= k;
                }
                tw.grave.retain(|g| g.1 > 0.0);
                tw.grave_spent.0 += pierce;
                let mut sp = proj.clone();
                sp.damage += dmg_stacks;
                (sp, pierce)
            } else if let (Some(alt), true) = (&nc.prince, sent == 0) {
                ((**alt).clone(), nc.prince_pierce.max(alt.pierce))
            } else {
                break;
            };
            let mut sp = spec;
            sp.damage += dmg_add;
            let len = self.tracks[path as usize].length();
            let d0 = (dist + self.rand() * nc.path_range).min(len);
            let p0 = self.tracks[path as usize].pos(d0);
            let owner = self.towers[t].id;
            let single = Emission { kind: "Single".into(), count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0 };
            let start = self.projectiles.len();
            self.emit(p0, 0.0, &single, &sp, None, tiers, owner);
            for pr in &mut self.projectiles[start..] {
                pr.pos = p0;
                pr.vel = [0.0; 2];
                pr.pierce = pierce;
                pr.on_track = Some((path, d0));
                if let Some(tt) = &sp.track_travel {
                    pr.frames_left = tt.lifespan * FPS * dist_mult;
                }
            }
            sent += 1;
        }
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

    /// Put projectile `spec`'s effects on bloon `i` (before its damage, so children of a popped
    /// bloon inherit them through `layers`).
    fn apply_effects(&mut self, i: usize, spec: &Proj, owner: u64, dmg: f32) {
        let d = &self.defs[self.bloons[i].kind];
        let has = |t: &str| d.tags.iter().any(|x| x == t);
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
        for e in &spec.effects {
            if (!e.tags.is_empty() && !e.tags.iter().any(|t| t.strip_prefix('!').map_or_else(|| has(t), |x| !has(x)))) || wont(e) {
                continue;
            }
            if (e.kind == "freeze" && !e.moabs && has("Moabs")) || (e.moabs_only && !has("Moabs")) {
                continue;
            }
            let (mut mult, mut lifespan) = (e.mult, e.lifespan);
            if !e.by_class.is_empty() {
                lifespan = moab_class(&self.names[self.bloons[i].kind]).and_then(|k| e.by_class.get(k)).copied().unwrap_or(0.0);
                if lifespan <= 0.0 {
                    continue;
                }
            }
            let mut prevent = false;
            for m in spec.effect_mods.iter().filter(|m| m.id == e.id && m.tags.iter().any(|t| has(t))) {
                prevent |= m.prevent;
                // HYPOTHESIS P-SLOWMOD: resetToUnmodified replaces the multiplier, otherwise it scales it.
                mult = if m.reset { m.mult } else { mult * m.mult };
                if m.lifespan > 0.0 {
                    lifespan = m.lifespan;
                }
                lifespan *= m.time_mult;
            }
            if prevent {
                continue;
            }
            let kind = match e.kind.as_str() {
                "slow" => EffectKind::Slow { mult, glue: e.glue },
                "freeze" => EffectKind::Freeze,
                "dot" => EffectKind::Dot { damage: e.damage, interval: e.interval * FPS, tick: self.round_clock as f32, immune: e.immune },
                "brittle" => EffectKind::Brittle { damage: e.damage },
                // HYPOTHESIS P-TRANCE: the bloon leaves its track spot and circles the tower that
                // hit it at orbitRadius, entering where it was hit, at its own speed x
                // speedMultiplier (MOAB-class: moabOrbitSpeed), then goes back to the spot it
                // left. It can't be entranced again for `cooldown` seconds after.
                "trance" => {
                    let b = &self.bloons[i];
                    if b.has_effect(&format!("{}Cooldown", e.id)) || e.immune & b.properties(d) != 0 {
                        continue;
                    }
                    let Some(c) = self.towers.iter().find(|t| t.id == owner).map(|t| t.pos) else { continue };
                    let f = if has("Moabs") { e.moab_mult } else { e.mult };
                    let r = e.radius.max(1.0);
                    add.push((
                        Effect { kind: EffectKind::Cooldown, id: format!("{}Cooldown", e.id), frames_left: (lifespan + e.cooldown) * FPS, layers: 1, owner, distribute: (true, false) },
                        false,
                    ));
                    EffectKind::Trance {
                        centre: c,
                        radius: r,
                        angle: (b.pos[1] - c[1]).atan2(b.pos[0] - c[0]),
                        rate: d.speed * f / FPS / r,
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
                "pop_emit" => match &e.projectile {
                    Some(p) => EffectKind::PopEmit {
                        proj: p.clone(),
                        emission: Box::new(e.emission.clone().unwrap_or(Emission { kind: "Single".into(), count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0 })),
                    },
                    None => continue,
                },
                _ => continue,
            };
            add.push((Effect { kind, id: e.id.clone(), frames_left: lifespan * FPS, layers: e.layers.max(1), owner, distribute: (e.distribute, e.distribute_override) }, e.refresh));
        }
        let b = &mut self.bloons[i];
        for (eff, refresh) in add {
            match b.effects.iter_mut().find(|x| x.id == eff.id) {
                // Bloon.AddMutator on a mutator it has: a new lifespan, its behaviours kept (a
                // DamageOverTime keeps its lastTick: recording boomerang_500, re-dotted at 98 and
                // 122, still ticks 60 frames after 77).
                Some(x) if refresh => {
                    let last = match &x.kind {
                        EffectKind::Dot { tick, .. } => Some(*tick),
                        _ => None,
                    };
                    *x = eff;
                    if let (EffectKind::Dot { tick: l, .. }, Some(v)) = (&mut x.kind, last) {
                        *l = v;
                    }
                }
                Some(_) => {}
                None => b.effects.push(eff),
            }
        }
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
        // Stationary area hits with no display of their own are drawn as bursts.
        if spec.link_radius || (spec.speed == 0.0 && !spec.instant && em.kind != "AtTarget" && spec.display.as_deref().is_none_or(str::is_empty)) {
            let ice = spec.effects.iter().any(|e| e.kind == "freeze");
            self.bursts.push(Burst { pos: from, radius: spec.radius, fire: spec.link_radius && !ice, ice });
        }
        let n = em.count.max(1);
        for k in 0..n {
            let (mut a, mut off) = (dir, 0.0f32);
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
                // HYPOTHESIS P-EMIT-RANDOM: RandomEmissionModel spreads each shot uniformly over
                // `angle` degrees centred on the target, starting `startOffset` ahead.
                "Random" => a = dir + (self.rand() - 0.5) * em.angle.to_radians(),
                _ => {}
            }
            let (s, c) = a.sin_cos();
            // The game's direction (SingleEmission / ArcEmission.GetDirection: rotation - offsetStart
            // + offset + k x sliceSize degrees, Vector2.Rotate of (0, 1)).
            let grot = self.emit_rot.and_then(|r| match em.kind.as_str() {
                "Arc" if n > 1 => {
                    let full = (em.angle - 360.0).abs() < 1e-3;
                    // ArcEmission.GetDirection (VERIFIED-DISASM @0xc56720): (rotation - angle / 2 +
                    // offset) + k x slice, a full circle too (recording dart_500's twelve balls).
                    let slice = if full { em.angle / n as f32 } else { em.angle / (n - 1) as f32 };
                    Some((r - em.angle / 2.0) + em.offset + k as f32 * slice)
                }
                "Single" | "Arc" | "Parallel" => Some(r),
                _ => None,
            });
            let gdir = grot.map(game_dir);
            let ahead = if em.kind == "Random" { em.offset_start } else { 0.0 };
            let mut pos = [from[0] - s * off + c * ahead, from[1] + c * off + s * ahead];
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
            self.next_uid += 1;
            self.proj_list.push(self.next_uid);
            self.projectiles.push(Projectile {
                uid: self.next_uid,
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
                    Some(v) => [v[0] * (spec.speed * FRAME_TIME), v[1] * (spec.speed * FRAME_TIME)],
                    None => [c * spec.speed / FPS, s * spec.speed / FPS],
                },
                // AgeModel.set_Lifespan (VERIFIED-DISASM @0x11dd100): lifespanFrames =
                // (int)Math.Round((double)(lifespan x 60)), half to even (recording dart_051: a
                // 20.25-frame dart is gone after 20).
                frames_left: {
                    let f = spec.lifespan * FPS;
                    if f.is_finite() { (f as f64).round_ties_even() as f32 } else { f }
                },
                pierce: spec.pierce,
                spec: {
                    let mut sp = spec.clone();
                    // SupportRemoveFilterOutTagModel: a buffed tower's shots skip their tag filters.
                    if !sp.exclude_tags.is_empty() && self.towers.iter().position(|t| t.id == owner).and_then(|ti| self.buffs.get(ti)).is_some_and(|b| b.unfilter) {
                        sp.exclude_tags.clear();
                    }
                    sp
                },
                crit,
                mirror: self.towers.iter().find(|t| t.id == owner).is_some_and(|t| t.flip),
                hit: HitSet::default(),
                clear_hits_next: false,
                tiers,
                owner,
                target: None,
                origin: pos,
                wait: 0.0,
                retarget_at: None,
                born: self.sim_elapsed * 4 + self.phase as u64,
                bounces: 0,
                camo: true,
                fwd: [c, s],
                path_d: 0.0,
                on_track: None,
                clear_t: self.sim_elapsed as f32,
                age: 0.0,
                seek: None,
                moving: 0.0,
                arrive_at: None,
                rounds_left: spec.rounds,
                value,
                cash_base,
                refresh_t: 0.0,
                interval_t: 0.0,
                exhaust_done: 0,
                aim: None,
            });
        }
    }

    pub fn step(&mut self) {
        if self.game_over {
            return;
        }
        self.sim_elapsed += 1;
        self.step_first_id = self.next_id;
        self.phase = 0;
        self.grid_rank_pre = None;
        if self.sim_elapsed % 60 == 0 {
            if self.has_grid_rank {
                self.grid_rank_pre = Some(self.path_rank.iter().map(|(&id, &(_, k))| (id, k)).collect());
            }
            self.plist_flush();
            self.proj_flush();
        }
        self.race_step_start();
        // HYPOTHESIS P-ACTIVERBE: a bloon counts its type's RBE less the damage it has taken.
        self.active_rbe = self
            .bloons
            .iter()
            .map(|b| (self.rbe[b.kind] - (self.defs[b.kind].max_health - b.hp).max(0) as f32).max(1.0))
            .sum();
        // Emissions: due now, made after this frame's collisions (the process list has the
        // CollisionChecker before the Spawner: recording dart_500, a pop's Red gets its id before
        // the round's next Black).
        let mut to_emit: Vec<(usize, u8)> = Vec::new();
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
                let path = self.spawn_tracks[idx as usize % self.spawn_tracks.len()] as u8;
                to_emit.push((k, path));
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
        // Simulation.Simulate: roundTime counts only while a round is on or bloons are out.
        if self.round_active || !self.bloons.is_empty() {
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
        // EmitOnPopModel: last frame's pops.
        for (pos, proj, emission, owner) in std::mem::take(&mut self.pop_emits) {
            let tiers = self.towers.iter().find(|t| t.id == owner).map_or([0; 3], |t| t.tiers);
            self.emit(pos, 0.0, &emission, &proj, None, tiers, owner);
        }
        self.comanche_defence();
        self.forget_untouched();
        self.spirit_of_the_forest();
        self.collect_zones();
        self.auto_abilities();
        // HYPOTHESIS P-LEAKDANGER: danger is the furthest bloon's share of its track run.
        self.danger = self.bloons.iter().map(|b| b.dist / self.tracks[b.path as usize].length().max(1.0)).fold(0.0, f32::max).clamp(0.0, 1.0);
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
        let p2 = std::time::Instant::now();
        // Projectiles.
        let (lo, hi) = self.bounds;
        let mut p = 0;
        let mut kids = Vec::new();
        let mut exhausted = Vec::new();
        let mut subs = Vec::new();
        let mut contacts = Vec::new();
        self.grid.build(&self.bloons, lo, hi);
        if self.has_grid_rank {
            let (rank, pre, single) = (&self.path_rank, &self.grid_rank_pre, self.tracks.len() == 1);
            let key = |b: &Bloon| match pre {
                Some(pre) => pre.get(&b.id).map_or(u64::MAX, |&k| k as u64),
                None => rank.get(&b.id).map_or(u64::MAX, |r| r.1 as u64),
            };
            self.ggrid.build(&self.bloons, |b| if single { key(b) } else { b.order }, self.step_first_id);
        }
        self.grid_on = true;
        let mut cand = std::mem::take(&mut self.grid_cand);
        let sim_now = self.sim_elapsed;
        let mut due_retargets: Vec<u64> = Vec::new();
        while p < self.projectiles.len() {
            self.proj_cursor = p;
            // Simulation.Simulate checks collisions before the process list moves anything
            // (VERIFIED by recording: a dart is created, waits a frame, then hits where it and
            // the bloon were when the frame began).
            if self.projectiles[p].fresh {
                self.projectiles[p].fresh = false;
                p += 1;
                continue;
            }
            self.projectiles[p].prev = self.projectiles[p].pos;
            if std::mem::take(&mut self.projectiles[p].clear_hits_next) {
                self.projectiles[p].hit.clear();
            }
            {
                let pr = &mut self.projectiles[p];
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
                    // HYPOTHESIS P-SEEK: seeking shots turn towards their target at turnRate
                    // deg/s; once it is gone they pick the closest unhit bloon within `distance`
                    // and half of maxSeekAngle either side of their heading.
                    if let Some(sk) = &pr.spec.seek {
                        let heading = pr.vel[1].atan2(pr.vel[0]);
                        let mut aim = pr.seek.and_then(|id| self.bloons.iter().find(|b| b.id == id)).map(|b| b.pos);
                        if aim.is_none() && sk.new_targets {
                            let best = self
                                .bloons
                                .iter()
                                .filter(|b| !b.hit_by(&pr.hit) && (pr.camo || !b.camo(&self.defs[b.kind])))
                                .filter_map(|b| {
                                    let q = b.pos;
                                    let (dx, dy) = (q[0] - pr.pos[0], q[1] - pr.pos[1]);
                                    let dd = dx * dx + dy * dy;
                                    let off = angle_diff(dy.atan2(dx), heading).abs().to_degrees();
                                    (dd <= sk.distance * sk.distance && off <= sk.max_angle / 2.0).then_some((b.id, q, dd))
                                })
                                .min_by(|a, b| a.2.total_cmp(&b.2));
                            pr.seek = best.map(|x| x.0);
                            aim = best.map(|x| x.1);
                        }
                        if let Some(q) = aim {
                            let max = (sk.turn_rate / FPS).to_radians();
                            let h = heading + angle_diff((q[1] - pr.pos[1]).atan2(q[0] - pr.pos[0]), heading).clamp(-max, max);
                            let sp = pr.vel[0].hypot(pr.vel[1]);
                            pr.vel = [h.cos() * sp, h.sin() * sp];
                            pr.grot = None;
                        }
                    }
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
                        let perc = (((sim_now - t0) as f32) * (1.0f32 / 60.0) / ar.time).min(1.0);
                        let e = curve_eval(&ar.samples, perc);
                        let np = [o[0] + (tg[0] - o[0]) * e, o[1] + (tg[1] - o[1]) * e];
                        pr.vel = [np[0] - pr.pos[0], np[1] - pr.pos[1]];
                        pr.pos = np;
                    } else if !last_frame {
                        pr.pos[0] += pr.vel[0];
                        pr.pos[1] += pr.vel[1];
                    }
                    if pr.moving > 0.0 {
                        pr.moving -= 1.0;
                        if pr.moving <= 0.0 {
                            pr.vel = [0.0, 0.0];
                        }
                    }
                }
                if !pr.spec.round_time || self.round_active {
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
                    let now = sim_now as f32;
                    if frames + pr.clear_t <= now {
                        pr.clear_t = now;
                        pr.clear_hits_next = true;
                    }
                }
                if let Some(iv) = pr.spec.refresh_pierce {
                    pr.refresh_t += 1.0;
                    if pr.refresh_t >= iv * FPS {
                        pr.refresh_t = 0.0;
                        pr.pierce = pr.spec.pierce;
                    }
                }
                // ProjectileBlockerCollisionRebound.Process (VERIFIED-DISASM @0xcefc20): after it
                // moves, the first blocker area (in the map's order, not below it) whose edge the
                // move crossed from outside turns it: mirrored across that edge, the rest of the
                // move carried on from the crossing.
                // (Not once ExpireProjectileAtScreenEdge, listed before it, has expired it.)
                let edge_expired = pr.spec.expire_at_edge && pr.frames_left <= 0.0;
                if let (Some(rb), false, false) = (&pr.spec.blocker_rebound, pr.spec.ignore_blockers, edge_expired) {
                    if let Some((at, dir)) = self.areas.iter().filter(|a| a.is_blocker && a.height >= 0.0).find_map(|a| blocker_bounce(a, pr.prev, pr.pos)) {
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
                    let tr = &self.tracks[k as usize];
                    let nd = d + if tt.reverse { -tt.speed } else { tt.speed } / FPS;
                    let np = tr.pos(nd.clamp(0.0, tr.length()));
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
                    let at = self.bloons.iter().find(|b| b.id == id).map(|b| b.pos);
                    match (at, pr.spec.instant) {
                        (Some(q), true) => pr.pos = q,
                        (None, true) => pr.frames_left = 0.0,
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
            if let Some(c) = &self.projectiles[p].spec.cash {
                if c.pickup.is_none() {
                    let pr = self.projectiles.remove(p);
                    self.collect_projectile_cash(&pr);
                    self.sound(&pr.spec.pickup_sounds, None);
                    self.cash_pops.push((pr.pos, pr.value));
                } else if self.projectiles[p].frames_left <= 0.0 {
                    self.projectiles.remove(p);
                } else {
                    p += 1;
                }
                continue;
            }
            let flying = self.projectiles[p].spec.no_hit
                || (self.projectiles[p].moving > 0.0 && self.projectiles[p].spec.arrive.as_ref().is_some_and(|a| a.filter_moving));
            // Broad phase: the bloons in grid cells within reach, in storage order (the order
            // a full scan would meet them in).
            cand.clear();
            if !flying && self.projectiles[p].pierce > 0.0 && self.projectiles[p].wait <= 0.0 {
                let pr = &self.projectiles[p];
                // Positions from the frame's start: allow for this frame's movement.
                let at = if pr.spec.instant || (pr.target.is_some() && !pr.spec.only_target) { pr.pos } else { pr.prev };
                if self.has_grid_rank && !(pr.spec.instant || (pr.target.is_some() && !pr.spec.only_target)) {
                    // CollisionChecker.Process: gridMargin 40 (0 for shots that can't hit MOABs).
                    let margin = if pr.spec.exclude_tags.iter().any(|t| t == "Moabs") { 0.0 } else { 40.0 };
                    self.ggrid.query(at, pr.spec.radius, margin, &self.grid_offsets, &mut cand);
                } else {
                    // A bloon in the projectile's own 10-unit cell can be up to ~14 away.
                    self.grid.query(at, (pr.spec.radius + self.max_bloon_radius).max(GAME_GRID_CELL * std::f32::consts::SQRT_2) + self.max_bloon_step, &mut cand);
                }
            }
            // CollisionGrid query order: cells by orderedNeighborOffsets around the projectile's
            // cell, bloons within a cell in list order. Only bloons it can reach are kept (the hit
            // test below rejects the rest anyway), so the sort stays small.
            if cand.len() > 1 && self.has_grid_rank && (self.projectiles[p].spec.instant || self.projectiles[p].target.is_some()) {
                let pr = &self.projectiles[p];
                let direct = pr.spec.instant || (pr.target.is_some() && !pr.spec.only_target);
                let at = if direct { pr.pos } else { pr.prev };
                let (bl, defs) = (&self.bloons, &self.defs);
                cand.retain(|&i| {
                    let b = &bl[i as usize];
                    let (bp, bc) = if direct { (b.pos, b.pos) } else { (b.prev, b.cell_pos) };
                    grid_hit(at, pr.spec.radius, bp, bc, defs[b.kind].radius)
                });
                // With pierce for all of them, the order can't change who gets hit.
                if cand.len() > 1 && (cand.len() as f32) > pr.pierce {
                    let pc = grid_cell(at);
                    let rank = &self.grid_rank;
                    cand.sort_by_key(|&i| {
                        let b = &bl[i as usize];
                        let bc = grid_cell(if direct { b.pos } else { b.cell_pos });
                        let (dx, dy) = (bc.0 - pc.0, bc.1 - pc.1);
                        let r = if dx.abs() <= 64 && dy.abs() <= 64 { rank[((dx + 64) * 129 + (dy + 64)) as usize] } else { u16::MAX };
                        (r, b.order)
                    });
                }
            }
            let mut k = 0;
            while !flying && k < cand.len() && self.projectiles[p].pierce > 0.0 && self.projectiles[p].wait <= 0.0 {
                let i = cand[k] as usize;
                let len0 = self.bloons.len();
                let b = &self.bloons[i];
                let d = &self.defs[b.kind];
                let pr = &self.projectiles[p];
                // Straight-flying shots meet bloons where both were when the frame began.
                let (bp, pp) = if pr.spec.instant || (pr.target.is_some() && !pr.spec.only_target) { (b.pos, pr.pos) } else { (b.prev, pr.prev) };
                let rr = pr.spec.radius + d.radius;
                let bcell = if pr.spec.instant || (pr.target.is_some() && !pr.spec.only_target) { b.pos } else { b.cell_pos };
                if !grid_hit(pp, pr.spec.radius, bp, bcell, d.radius)
                    || !circles_hit(pp, pr.spec.radius, bp, b.rot, &d.circles)
                    || (pr.spec.instant && pr.target.is_some_and(|t| t != b.id))
                    // CollideOnlyWithTarget.FilterCollision (VERIFIED-DISASM @0xcbb6e0): only its
                    // Target while that is valid; none once it is gone (recording boomerang_500: a
                    // MoabDot whose glaive's target popped sits out its life, its pierce unused).
                    // INFERRED: a target-less one takes any (ignoreFirstCheck, pierce 1 here).
                    || (pr.spec.only_target && !pr.spec.instant && pr.target.is_some_and(|t| t != b.id))
                    || b.hit_by(&pr.hit)
                    || pr.spec.exclude_bloons.iter().any(|x| self.names[b.kind].starts_with(x.as_str()))
                    || pr.spec.exclude_tags.iter().any(|t| d.tags.contains(t))
                    || (pr.spec.glue_filter > 0 && b.glue() >= pr.spec.glue_filter)
                    || (pr.spec.skip_frozen && b.frozen())
                    || cant_hit(pr.spec.cant_hit, &pr.spec, b.properties(d))
                    // FilterOveridingMutatedTarget names the mutation by its end ("SuperBrittle" for
                    // "Ice:Debuff:SuperBrittle"); INFERRED: matched as a substring.
                    || (!pr.spec.require_effects.is_empty() && !pr.spec.require_effects.iter().any(|r| b.effects.iter().any(|e| e.id.contains(r.as_str()))))
                {
                    k += 1;
                    continue;
                }
                // HYPOTHESIS P-EAT: a trap swallows every bloon it touches whole (no pops, no pop
                // cash) while it holds less than rbeCapacity RBE; then it closes and waits to be
                // collected for the eaten RBE x rbeCashMultiplier.
                if let Some(eat) = pr.spec.eat.clone() {
                    let rbe = self.rbe[b.kind] as f64;
                    let len0 = self.bloons.len();
                    self.remove_bloon(i);
                    // The old last bloon now sits in slot i.
                    remap_moved(&mut cand, k, i, len0);
                    k += 1;
                    let pr = &mut self.projectiles[p];
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
                let mut popped_now = 0u64;
                let mut dmg = pr.crit.unwrap_or(pr.spec.damage);
                for m in &pr.spec.state_mods {
                    if m.states.iter().any(|s| b.in_state(s)) {
                        dmg = dmg * m.mult + m.add;
                    }
                }
                // DesperadoMarkModel: marked bloons take extra damage from Desperados.
                if let Some(m) = b.effects.iter().find_map(|e| match e.kind { EffectKind::Mark { damage_mult, .. } => Some(damage_mult), _ => None }) {
                    if self.towers.iter().find(|t| t.id == pr.owner).is_some_and(|t| self.tower_defs[t.def].0 == "Desperado") {
                        dmg *= m;
                    }
                }
                if let Some([mult, add, at]) = pr.spec.cash_damage {
                    if self.cash_for_tower(pr.owner) >= at as f64 {
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
                    let name = &self.names[b.kind];
                    if name == &m.id || (m.children && name.starts_with(m.id.as_str())) {
                        dmg = dmg * m.mult + m.add;
                    }
                }
                if let Some(w) = &pr.spec.wrath {
                    dmg += (self.active_rbe / w.threshold * w.damage).clamp(0.0, w.max);
                }
                // PushBack.Collide (VERIFIED-DISASM @0xcf29a0): pushAmount x the class multiplier,
                // through Bloon.Move(-amount), which keeps it from trailing the entrance by more than
                // 1.25 x its radius (recording boomerang_005: a MOAB pressed back to 157.35 moves on
                // from there once its stun is gone).
                if let Some(pb) = &pr.spec.push {
                    if pb.tag.is_empty() || d.tags.contains(&pb.tag) {
                        let m = moab_class(&self.names[b.kind]).and_then(|k| pb.mult.get(k)).copied().unwrap_or(1.0);
                        let back = pb.amount * m;
                        let r = d.radius;
                        self.push_back(i, back);
                        if let Some(sd) = self.track_ends.get(self.bloons[i].path as usize).and_then(|e| e.0) {
                            let lim = sd - r * 1.25;
                            let b = &mut self.bloons[i];
                            if b.dist <= lim && lim != b.dist {
                                b.dist = lim;
                            }
                        }
                        self.bloons[i].pos = self.bloons[i].pos;
                    }
                }
                // WindModel (Distraction): HYPOTHESIS P-WIND, `chance` per hit to move the bloon
                // a uniform min..max units back along the track (scaled for the listed tags).
                if let Some(w) = self.projectiles[p].spec.wind.clone() {
                    let is_moab = self.defs[self.bloons[i].kind].tags.iter().any(|t| t == "Moabs");
                    if (w.moabs || !is_moab) && self.rand() < w.chance {
                        let mut back = w.min + (w.max - w.min) * self.rand();
                        if w.tag_scale > 0.0 && w.tags.iter().any(|t| self.defs[self.bloons[i].kind].tags.contains(t)) {
                            back *= w.tag_scale;
                        }
                        self.push_back(i, back);
                        self.bloons[i].pos = self.bloons[i].pos;
                    }
                }
                kids.clear();
                if let Some(at) = crit_text {
                    self.hit_texts.push(at);
                }
                if immune {
                    let props = self.bloons[i].properties(&self.defs[self.bloons[i].kind]);
                    self.immune_sound(props);
                    // Bloon.Damage with destroyProjectileIfImmune (VERIFIED-DISASM Damage.Collide
                    // @0xcd5e63; recording dart_upgrades_r30: a dart on a Lead is gone).
                    if !self.projectiles[p].spec.immune_keep && self.projectiles[p].spec.damage > 0.0 {
                        self.projectiles[p].pierce = 0.0;
                    }
                }
                if !immune {
                    let spec = &self.projectiles[p].spec;
                    if spec.decamo && (dmg >= 0.5 || !spec.decamo_needs_damage) {
                        self.bloons[i].decamo = true;
                    }
                    // HYPOTHESIS P-REMOVEMUT: a key removes effects whose mutator id contains it;
                    // "Ice" also covers freezes and "Glue" glue slows whatever their id.
                    if !spec.remove_effects.is_empty() {
                        let keys = spec.remove_effects.clone();
                        self.bloons[i].effects.retain(|e| {
                            !keys.iter().any(|k| {
                                e.id.contains(k.as_str())
                                    || (k == "Ice" && matches!(e.kind, EffectKind::Freeze))
                                    || (k == "Glue" && matches!(e.kind, EffectKind::Slow { glue: 1.., .. }))
                            })
                        });
                    }
                    if !self.projectiles[p].spec.effects.is_empty() {
                        let (spec, owner) = (self.projectiles[p].spec.clone(), self.projectiles[p].owner);
                        self.apply_effects(i, &spec, owner, dmg);
                    }
                    // HYPOTHESIS P-LEADGOLD: Lead to Gold pays its flat amount when a hit pops a
                    // bloon with the tag (the game attaches it to the bloon for its next pop).
                    let gold = self.projectiles[p].spec.cash_on_pop.as_ref().filter(|c| self.defs[self.bloons[i].kind].tags.contains(&c.tag)).map(|c| c.amount as f64);
                    let tally0 = self.damage_tally;
                    self.distribute = (self.projectiles[p].spec.distribute, self.projectiles[p].spec.distribute_override);
                    let n = self.damage(i, dmg.round() as i32, &mut kids);
                    self.distribute = (true, false);
                    let self_tally_dealt = self.damage_tally - tally0;
                    // Knockback.Collide (VERIFIED-DISASM @0xce56e0), after the damage: on what the
                    // hit bloon is now and what it split into (their timer starts with them: they
                    // don't move this frame; recording dart_500), unless it
                    // already has it or isn't valid for pushback (IsBloonValidForPushback: on the
                    // map; its near-the-exit checks aren't done). It turns at once: a first child
                    // that already moved this frame moves at the new speed.
                    if let Some(kb) = self.projectiles[p].spec.knockback.clone() {
                        let mut valid = None;
                        for kid in std::iter::once(id).chain(kids.iter().copied()) {
                            let Some(j) = self.bloons.iter().position(|b| b.id == kid) else { continue };
                            let tags = &self.defs[self.bloons[j].kind].tags;
                            let has = |t: &str| tags.iter().any(|x| x == t);
                            let m = if has("Moabs") { kb.moab } else if has("Lead") || has("Ceramic") || has("Fortified") { kb.heavy } else { kb.light };
                            // Bloon.IsBloonValidForPushback (VERIFIED-DISASM @0xfd2d50): on the map, not
                            // behind spawnDist - 1.25 x radius, and at least 10 past the centre of the
                            // first segment whose point is on the map (distance as the frame began).
                            let path = self.bloons[j].path as usize;
                            let at = if self.bloons[j].dist != self.bloons[j].prev_dist { self.bloons[j].prev_dist } else { self.bloons[j].dist };
                            let spawn_ok = self.track_ends.get(path).and_then(|e| e.0).is_none_or(|sd| at >= sd - self.defs[self.bloons[j].kind].radius * 1.25);
                            let first_seen = self.track_segs.get(path).and_then(|sg| sg.iter().position(|q| on_map(*q, 0.0)));
                            let past_entry = first_seen.is_none_or(|k| at >= k as f32 * 7.0 + 3.5 + 10.0);
                            let b = &mut self.bloons[j];
                            // The hit bloon decides for its children (they carry its mutator on;
                            // recording dart_520: a Black's second Red, made behind the first
                            // on-map segment, goes back with it).
                            let ok = *valid.get_or_insert(!b.effects.iter().any(|e| matches!(e.kind, EffectKind::Knock { .. })) && on_map(b.pos, 0.0) && spawn_ok && past_entry);
                            if !ok || b.effects.iter().any(|e| matches!(e.kind, EffectKind::Knock { .. })) {
                                continue;
                            }
                            b.effects.push(Effect { kind: EffectKind::Knock { m }, id: "Knockback".into(), frames_left: (kb.lifespan * FPS).round(), layers: 1, owner: 0, distribute: (true, false) });
                            if b.dist != b.prev_dist {
                                let s = speed_frames(b.model_speed(self.defs[b.kind].speed));
                                b.dist = (b.prev_dist + s * b.track_mult * 1.0 * b.mult).max(0.0);
                                b.pos = self.tracks[b.path as usize].bloon_pos(b.dist);
                            }
                        }
                    }
                    popped_now = n;
                    if let (Some(v), true) = (gold, n > 0) {
                        self.earn(v);
                        self.cash_pops.push((bp, v));
                    }
                    let owner = self.projectiles[p].owner;
                    if let Some(ti) = self.towers.iter().position(|t| t.id == owner) {
                        self.towers[ti].pops += n;
                        self.towers[ti].damage += self_tally_dealt;
                        // Monkey Town (CashIncreaseModel): pop cash x multiplier.
                        let m = self.buffs.get(ti).map_or(1.0, |b| b.cash_mult as f64);
                        self.earn(n as f64 * (m - 1.0));
                    }
                }
                if !self.projectiles[p].spec.hit_sounds.is_empty() {
                    let names = self.projectiles[p].spec.hit_sounds.clone();
                    self.sound(&names, None);
                }
                let pr = &mut self.projectiles[p];
                // CollideBloon adds the layer it hit; what that layer became carries it in its
                // lineage.
                pr.hit.push(layer);
                pr.pierce -= pr.spec.pierce_cost;
                let heading = if pr.spec.instant && pr.target.is_some() {
                    self.tracers.push((pr.origin, bp));
                    let h = (bp[1] - pr.origin[1]).atan2(bp[0] - pr.origin[0]);
                    pr.origin = bp;
                    h
                } else {
                    pr.vel[1].atan2(pr.vel[0])
                };
                for c in &pr.spec.on_contact { // P-CONTACT
                    if (c.on_damage && immune) || (c.on_pop && popped_now == 0) {
                        continue;
                    }
                    let skip = if c.pass_on_collided { Some(layer) } else { None };
                    contacts.push((bp, heading, c.clone(), pr.tiers, pr.owner, skip));
                }
                // INFERRED: a bouncing glaive's Target becomes the bloon it hit (recording
                // boomerang_500: Glaive Lord's MoabDots take the glaive's last-hit bloon, not its
                // throw target: one whose hit bloon popped that frame never lands, one whose hit
                // Zebra lived on hits it next frame). Other shots keep theirs (recording
                // boomerang_paragon_r63: a MOAB Press still follows its MOAB after other hits).
                if pr.spec.retarget.is_some() {
                    pr.aim = Some(id);
                }
                // RetargetOnContact.Collide (VERIFIED-DISASM @0xcf6ae0): with bounces left, a delayed
                // one notes the frame (Process retargets delayFrames later), else it retargets now.
                if let (Some(rt), true) = (pr.spec.retarget.clone(), pr.pierce > 0.0) {
                    if pr.bounces < rt.max_bounces {
                        if ((rt.delay * FPS) as f64).round_ties_even() > 0.0 {
                            pr.retarget_at = Some(sim_now);
                        } else {
                            self.retarget(p, &rt);
                        }
                    }
                }
                // Storage only changes at or after `i`: a popped bloon is swap-removed (the last
                // bloon moves into slot i) and children are appended. So re-check slot i if its
                // occupant changed, else move on, instead of rescanning from 0 (quadratic in big
                // pop cascades). Each bloon is still checked once per projectile per frame.
                if !self.bloons.get(i).is_some_and(|b| b.id == id) {
                    // Popped: the old last bloon moved into slot i (children are appended after
                    // it, already in this projectile's hit list).
                    remap_moved(&mut cand, k, i, len0);
                }
                k += 1;
            }
            // CollisionChecker.Process, after bloons: a projectile touching a MapBlocker taller
            // than it (VsBlockerRadius, from where it began the frame) is destroyed, unless its
            // model ignores blockers (VERIFIED by recording: darts flying into Monkey Meadow's
            // trees). INFERRED: projectiles fly at height 0 here.
            let pr = &self.projectiles[p];
            if !pr.spec.ignore_blockers && pr.spec.blocker_rebound.is_none() && !pr.spec.instant && pr.target.is_none() && pr.pierce > 0.0 && pr.frames_left > 0.0 && self.blocker_hit(pr.prev, pr.spec.vs_blocker_radius) {
                self.projectiles[p].frames_left = 0.0;
            }
            // HYPOTHESIS P-ACIDPOOL: a splash that expires without hitting anything becomes a pool
            // (lifespanIfMisses, radiusIfMisses, pierce).
            let pr = &mut self.projectiles[p];
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
                let pr = &mut self.projectiles[p];
                for (k, ex) in pr.spec.on_exhaust.iter().enumerate() {
                    if pr.exhaust_done & (1 << k) != 0 {
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
                    if !self.round_active && !ex.between_rounds {
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
                    exhausted.push((at, pr.vel[1].atan2(pr.vel[0]), ex, pr.tiers, pr.owner, pr.aim.or(pr.target), pr.grot));
                }
            }
            let pr = &self.projectiles[p];
            if pr.pierce <= 0.0 || pr.frames_left <= 0.0 {
                let pr = self.projectiles.remove(p);
                // HYPOTHESIS P-EXPIRESOUND: expire sounds play whenever the projectile goes,
                // exhaust sounds only when its pierce ran out.
                self.sound(&pr.spec.expire_sounds, None);
                if pr.pierce <= 0.0 {
                    self.sound(&pr.spec.exhaust_sounds, None);
                }
                // (Projectile.Expire and .Exhausted both run its Exhausted: any not done yet.)
                for (k, ex) in pr.spec.on_exhaust.iter().enumerate() {
                    if pr.exhaust_done & (1 << k) != 0 || (!self.round_active && !ex.between_rounds) {
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
                    exhausted.push((at, pr.vel[1].atan2(pr.vel[0]), ex, pr.tiers, pr.owner, pr.aim.or(pr.target), pr.grot));
                }
                // HYPOTHESIS P-TYPEDTOWER: typed sub-towers (Sentry Expert) take turns in
                // crushing, boom, cold, energy order.
                let n = pr.spec.create_tower.len();
                if n > 0 {
                    let spec = pr.spec.create_tower[self.typed_made % n].clone();
                    self.typed_made += 1;
                    subs.push((spec, pr.pos, pr.owner));
                }
            } else {
                p += 1;
            }
        }
        self.proj_cursor = usize::MAX;
        self.grid_on = false;
        self.grid_cand = cand;
        let e = p2.elapsed().as_secs_f64();
        self.prof[2] += e;
        self.prof_max[2] = self.prof_max[2].max(e);
        // Bloons made by this frame's pops (ids from here back to step_first_id): not in the
        // process list yet, so their Grow doesn't tick this frame (recording dart_400: Pinks
        // made on frame 3181 regrow on 3361).
        self.popped_born_to = self.next_id;
        self.phase = 1;
        // Bloons processed during the tower phase (they come before a newer Attack in the process
        // list), with what their Process produced; the rest are processed after the towers.
        let mut dots = Vec::new();
        let mut emits = Vec::new();
        let mut leak_voice = false;
        let mut early: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let round_clock = self.round_clock;
        let p1 = std::time::Instant::now();
        // Towers.
        self.expire_subs(false);
        self.leak_immune = (self.leak_immune - 1.0).max(0.0);
        self.syphon = (self.syphon - 1.0).max(0.0);
        self.circle_subs();
        self.move_air();
        self.fly_birds();
        self.update_rage();
        self.buffs = self.compute_buffs();
        self.orbit_step(false);
        let cd_speed: Vec<f32> = (0..self.towers.len())
            .map(|t| self.submerged_scale(self.towers[t].pos, |s| (s.cooldown_scale, s.cooldown_scale_global)))
            .collect();
        for t in 0..self.towers.len() {
            let buff = self.buffs[t].clone();
            {
                let tw = &mut self.towers[t];
                if tw.attack_t < f32::MAX {
                    tw.attack_t += 1.0 / FPS;
                }
                tw.idle_t += 1.0 / FPS;
                if let Some(s) = tw.show.as_mut() {
                    s.1 += 1.0 / FPS;
                }
                for ab in &mut tw.brews {
                    ab.frames_left -= 1.0;
                }
                tw.brews.retain(|a| a.frames_left > 0.0 && a.shots_left > 0);
                // HYPOTHESIS P-ABILITYCD: ability cooldowns run only during rounds.
                if self.round_active {
                    let speed = cd_speed[t] * self.buffs.get(t).map_or(1.0, |b| b.ability_cd_mult);
                    for cd in tw.ability_cd.values_mut() {
                        *cd = (*cd - speed).max(0.0);
                    }
                }
                for (_, f) in &mut tw.temp_supports {
                    *f -= 1.0;
                }
                tw.temp_supports.retain(|(_, f)| *f > 0.0);
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
            let Some(mut tier) = self.tier(t).cloned() else { continue };
            // Submerged: the submerge attack replaces the tower's own.
            if self.towers[t].submerged {
                match &tier.submerge {
                    Some(sm) => tier.attacks = vec![sm.attack.clone()],
                    None => self.towers[t].submerged = false,
                }
            }
            let extra: Vec<crate::data::Attack> = self.towers[t].temp.iter().map(|x| x.attack.clone()).collect();
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
                let (steps, rc) = (self.steps as i64, self.round_clock);
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
            let pos = self.attack_pos(t);
            // HYPOTHESIS P-ROUNDRATE: StartOfRoundRateBuffModel.modifier multiplies the weapon
            // interval for the first `duration` seconds of each round (0.25 = 4x as fast).
            let start_mult = match &tier.round_start_rate {
                Some(r) if self.round_active && (self.round_frame as f32) < r.duration * FPS && r.mult > 0.0 => r.mult,
                _ => 1.0,
            };
            // A buff change is a model change: Weapon.UpdatedModel recalculates rateFrames.
            for (st, w) in self.towers[t].weapons.iter_mut().zip(all()) {
                let br = buff.rate_mult * start_mult;
                // (And so is an upgrade: recording dart_upgrades_r30, Quick Shots bought at 600
                // throws on 601, 48 frames after the last.)
                if st.buff_rate != br || st.model_rate != w.rate {
                    st.buff_rate = br;
                    st.model_rate = w.rate;
                    st.rate_frames = rate_frames(st.rate(w));
                }
            }
            let mut wi = 0;
            let mut turned = false;
            // RotateToMiddleOfTargetsModel on any attack: the body faces its attacks' targets'
            // midpoint instead of turning with each throw.
            let middle = tier.attacks.iter().any(|a| a.face_middle);
            for (ai, attack) in tier.attacks.iter().chain(extra.iter()).enumerate() {
                // (A Paragon's new Attacks are made paused from its first frame and wake with its
                // Orbit, a frame before the kept ones: recordings boomerang_paragon_r40 (its MOAB
                // Press waits) and r98 (it throws at 181, the glaives at 182).)
                // Simulation.Simulate runs the process list in creation order: an Attack made after
                // some bloons (a tower placed or upgraded mid-round) acts after they have been
                // processed this frame (recording boomerang_paragon_r98: the Paragon's new MOAB
                // Press aims where a BFB is after this frame's move). Made when it first runs here.
                if ai < tier.attacks.len() {
                    let stamp = *self.towers[t].attack_born.entry(attack.model_name.clone()).or_insert(self.sim_elapsed * 4);
                    self.process_bloons_before(stamp, &mut early, round_clock, &mut dots, &mut emits, &zones, &shoves, &mut leak_voice);
                }
                let fresh_attack = self.towers[t].paragon.is_some() && self.sim_elapsed < self.towers[t].paragon_active_at
                    && !self.towers[t].paragon_kept.contains(&attack.model_name);
                let paused = paused && (self.towers[t].paragon.is_none() || self.towers[t].paragon_kept.contains(&attack.model_name)
                    || self.towers[t].temp.iter().any(|x| x.pauses && x.delay <= 0.0) || self.towers[t].mute > 0.0);
                if ((paused || fresh_attack) && ai < tier.attacks.len()) || ai.checked_sub(tier.attacks.len()).is_some_and(|k| waiting.get(k) == Some(&true)) {
                    wi += attack.weapons.len();
                    continue;
                }
                if let Some(brew) = &attack.brew {
                    for weapon in &attack.weapons {
                        let i = wi;
                        wi += 1;
                        let clock = if weapon.between_rounds { self.steps as i64 } else { self.round_clock };
                        if !self.towers[t].weapons[i].ready(clock) || !self.round_active {
                            continue;
                        }
                        match self.brew_target(t, brew, buff.range(attack.range)) {
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
                    let (now_t, rc) = (self.steps as i64, self.round_clock);
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
                // Target selection for this attack.
                let mut best: Option<(f32, [f32; 2], u64)> = None;
                // The chosen target's offset from its bloon (CanHitMoab's precalculatedOffset), kept
                // exact rather than recomputed from the point.
                let mut best_off = [0.0f32; 2];
                let mut any_target = false;
                let (camo, range) = (attack.camo || buff.camo, buff.range(attack.range));
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
                let mut lead: Vec<Option<(usize, f32)>> = vec![None; self.tracks.len()];
                if mode == TargetMode::First {
                    for (bi, b) in self.bloons.iter().enumerate() {
                        let d = &self.defs[b.kind];
                        if (b.camo(d) && !camo) || (no_moab && d.tags.iter().any(|t| t == "Moabs")) {
                            continue;
                        }
                        if let Some(l) = lead.get_mut(b.path as usize) {
                            if l.is_none_or(|(_, dist)| b.dist > dist) {
                                *l = Some((bi, b.dist));
                            }
                        }
                    }
                }
                // Path.FindBestBloon (VERIFIED-DISASM @0xf8d160) for Strong: per path, in its list
                // order, the highest BloonModel.danger, ties to the one further along (strictly);
                // the bloons the attack can see. Targetting.GetTargets takes it if its segment is
                // in range, else the in-range segments in First's order (recording
                // dart_004_camo_strong_r36: the second of a line of Pinks).
                let mut strong_lead: Vec<Option<(usize, f32, f32)>> = vec![None; self.tracks.len()];
                if mode == TargetMode::Strong {
                    let mut order: Vec<usize> = (0..self.bloons.len()).collect();
                    order.sort_by_key(|&i| self.path_rank.get(&self.bloons[i].id).map_or(u32::MAX, |r| r.1));
                    for bi in order {
                        let b = &self.bloons[bi];
                        let d = &self.defs[b.kind];
                        if (b.camo(d) && !camo) || (no_moab && d.tags.iter().any(|t| t == "Moabs")) {
                            continue;
                        }
                        if let Some(l) = strong_lead.get_mut(b.path as usize) {
                            if l.is_none_or(|(_, dg, ds)| d.danger > dg || (d.danger == dg && b.dist > ds)) {
                                *l = Some((bi, d.danger, b.dist));
                            }
                        }
                    }
                }
                let mut best_key = f64::NEG_INFINITY;
                for (bi, b) in self.bloons.iter().enumerate() {
                    let d = &self.defs[b.kind];
                    let p = b.pos;
                    let dd = (p[0] - pos[0]).powi(2) + (p[1] - pos[1]).powi(2);
                    // Attack targets the bloons on its in-range path segments (PathSegment.IsInRange:
                    // the segment's point within range; Path.AddBloon: a bloon is on segment
                    // (int)(distance / 7)), not by the bloon's own distance.
                    let seg_idx = (b.dist / 7.0).max(0.0) as usize;
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
                    let seg_clear = seg_q.is_some_and(|q| attack.through_walls || self.line_clear(pos, 0.0, q));
                    let plain = seg_q.is_some_and(|q| seg_near(q, range + 2.0)) && seg_clear;
                    let mut aim_at = b.pos;
                    let mut aim_off = [0.0f32; 2];
                    let seg_in = plain || (!d.circles.is_empty() && seg_q.is_some_and(|q| seg_near(q, range + 40.0)) && {
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
                            .map(|c| match rot {
                                Some(r) => {
                                    let a = DEG2RAD * -r;
                                    let (sa, ca) = crt_sin_cos(a);
                                    let (ox, oy) = (ca * c[0] - sa * c[1], ca * c[1] + sa * c[0]);
                                    [b.pos[0] + ox, b.pos[1] - oy, c[2], ox, -oy]
                                }
                                None => {
                                    let (ox, oy) = (fwd[0] * c[1] - fwd[1] * c[0], fwd[1] * c[1] + fwd[0] * c[0]);
                                    [b.pos[0] + ox, b.pos[1] + oy, c[2], ox, oy]
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
                            })
                            .is_some()
                    });
                    if !seg_in && shared.is_empty() {
                        continue;
                    }
                    if b.camo(d) && !camo {
                        continue;
                    }
                    if !attack.require_tags.iter().all(|t| d.tags.contains(t)) || (!buff.unfilter && attack.exclude_tags.iter().any(|t| d.tags.contains(t))) {
                        continue;
                    }
                    if (attack.glue_filter > 0 && b.glue() >= attack.glue_filter)
                        || (attack.skip_frozen && b.frozen())
                        || attack.weapons.first().is_some_and(|w| cant_hit(attack.cant_hit, &w.projectile, b.properties(d)))
                        || attack.skip_effects.iter().any(|id| b.has_effect(id))
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
                        // TargetEliteTargetting.Sort (VERIFIED-DISASM @0xe005c0, simplified
                        // HYPOTHESIS P-ELITE): past 75% of the track first, then BloonModel
                        // danger (RBE here), then how far along.
                        TargetMode::Elite => {
                            let perc = b.dist / self.tracks[b.path as usize].length().max(1.0);
                            (if perc > 0.75 { 1e12 } else { 0.0 }) + self.rbe[b.kind] * 1e4 + b.progress()
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
                        let rank = close_rank.get(&(b.path, (b.dist / 7.0).max(0.0) as usize)).map_or(1e6, |&r| r as f64);
                        prio * 1e12 - rank * 1e3 - self.path_rank.get(&b.id).map_or(999.0, |r| r.1 as f64).min(999.0)
                    } else if mode == TargetMode::Strong && seg_in {
                        let len = self.tracks[b.path as usize].length() as f64;
                        let until_leak = len - ((b.dist / 7.0).max(0.0) as usize as f64 * 7.0 + 3.5);
                        // TargetStrong.GetTarget (VERIFIED-DISASM @0xe0dfa0): over Attack
                        // .GetVisibleTargets, the first with a strictly higher danger (recording
                        // boomerang_000_strong: an in-range Blue over a Red further along).
                        prio * 1e12
                            + if strong_lead[b.path as usize].is_some_and(|(l, _, _)| l == bi) { 1e10 } else { d.danger as f64 * 1e8 - until_leak * 1e4 - self.path_rank.get(&b.id).map_or(1e6, |r| r.1 as f64) * 1e-3 }
                    } else if mode == TargetMode::First && seg_in {
                        // (prio is the camo / MOAB priority count: the f32 score near 1e12 can't
                        // order bloons; recording dart_002_camo_r37.)
                        let len = self.tracks[b.path as usize].length() as f64;
                        let until_leak = len - ((b.dist / 7.0).max(0.0) as usize as f64 * 7.0 + 3.5);
                        prio * 1e12 + if lead[b.path as usize].is_some_and(|(l, _)| l == bi) { 1e9 } else { -until_leak * 1e4 - self.path_rank.get(&b.id).map_or(1e6, |r| r.1 as f64) * 1e-3 }
                    } else {
                        prio * 1e12 + base as f64
                    };
                    if best.is_none() || key > best_key {
                        best_key = key;
                        best = Some((score, aim_at, b.id));
                        best_off = aim_off;
                    }
                }
                if attack.pool_fallback && best.is_none() && self.round_active {
                    best = self.track_point(pos, range, 0.0);
                }
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
                    best = if !self.round_active && self.bloons.is_empty() {
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
                            _ if attack.track_closest => self.closest_track_point(pos, range, off),
                            _ => self.track_point(pos, range, off),
                        }
                    };
                }
                // HYPOTHESIS P-RANDPOS: RandomPositionBasicModel picks a uniform direction and a
                // distance in [minRadius, maxRadius], kept inside the map bounds.
                if let Some([lo, hi]) = attack.random_position {
                    best = if self.round_active {
                        // HYPOTHESIS P-RANDLAND: RandomPositionModel (areaType Land) redraws
                        // until the spot is placeable land clear by targetRadius; no shot if 20
                        // draws all fail.
                        let mut found = None;
                        for _ in 0..if attack.random_land.is_some() { 20 } else { 1 } {
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
                // HYPOTHESIS P-POINTER: point-aiming attacks fire at their point while any bloon
                // they could hit is in range (map-wide for Dartling / Mortar), and turn towards it
                // at RotateToPointerModel.rate (turning shots fire along the barrel).
                let mut turning = false;
                if mode.aims_at_point() {
                    let tw = &self.towers[t];
                    let fwd = [pos[0] + tw.angle.cos() * 40.0, pos[1] + tw.angle.sin() * 40.0];
                    let aim = match mode {
                        TargetMode::Pointer => self.coop_pointer(tw.id).unwrap_or(fwd),
                        _ => tw.point.unwrap_or(fwd),
                    };
                    best = any_target.then_some((0.0, aim, 0));
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
                    // LimitProjectileModel: traps of this tower that aren't full yet.
                    let at_limit = weapon.limit.is_some_and(|n| {
                        let id = self.towers[t].id;
                        let out = self.projectiles.iter().filter(|pr| pr.owner == id && pr.spec.eat.is_some() && pr.spec.cash.is_none()).count();
                        out >= n as usize
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
                    let now_t = self.steps as i64;
                    let clock = if weapon.between_rounds { now_t } else { self.round_clock };
                    let tw = &mut self.towers[t];
                    let quota_left = weapon.per_round.map_or(true, |n| tw.weapons[w].round_shots < n) && !at_limit;
                    let decide = best.or(ambient.then_some((0.0, pos, 0))).filter(|_| quota_left);
                    // Weapon.Process (VERIFIED-DISASM @0xc6f800): once lastEmission + rateFrames
                    // <= its clock and the attack has a target, the throw animation starts and
                    // the cooldown restarts there; the projectile leaves animationOffsetFrames
                    // later (WeaponModel: (int)(animationOffset x 60)), at the attack's target
                    // then (SpawnDart). A throw still pending when the next starts leaves first.
                    let off = (weapon.animation_offset * 60.0) as i32 as i64;
                    let mut spawns = 0;
                    let st = &mut tw.weapons[w];
                    let started_id = decide.filter(|_| st.ready(clock)).map(|(_, _, id)| id);
                    let started = decide.filter(|_| st.ready(clock)).map(|(_, p, _)| p);
                    if let Some(id) = started_id {
                        st.aim_id = id;
                        st.aim_off = best_off;
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
                    if st.anim >= 0 && st.anim + off <= now_t {
                        st.anim = -1;
                        spawns += 1;
                    }
                    let (aimed, aim_id, aim_off) = (st.aim_at, st.aim_id, st.aim_off);
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
                        }
                    }
                    if let (Some(p), false) = (started, ambient) {
                        if attack.rotate && !turning {
                            tw.aim = (p[1] - pos[1]).atan2(p[0] - pos[0]) - extra_rot;
                            tw.set_attack_aim(ai, tw.aim, p, pos, middle, attack.turn_body);
                        }
                        tw.attack_t = 0.0;
                        tw.idle_t = 0.0;
                    }
                    if spawns == 0 {
                        continue;
                    }
                    fired_ids.extend(weapon.alt_id);
                    // Necromancer stacks speed the weapon up (as of the shot, before it spends
                    // the graveyard).
                    let necro = attack.necro.as_ref().map_or(1.0, |nc| self.necro_rate(t, nc));
                    // The shots to emit, in order: each spawned throw, then the missed-frame
                    // shots its rate leaves (doubleShot).
                    let mut queue = vec![false; spawns];
                    let mut qi = 0;
                    while qi < queue.len() {
                        let double = queue[qi];
                        qi += 1;
                        // With no target any more the throw still leaves: Target.IsValid (VERIFIED-DISASM
                        // @0xb9bb90) fails for a destroyed bloon, so RotateToTarget no longer turns the
                        // tower and the shot goes along its current facing (recording dart_040_club:
                        // three towers on a Green the first one's darts popped that frame).
                        let locked = (aim_id != 0).then(|| self.bloons.iter().find(|b| b.id == aim_id).map(|b| ([b.pos[0] + aim_off[0], b.pos[1] + aim_off[1]], b.id))).flatten();
                        // (The attack only looks for a new target when a throw starts: Weapon.Process
                        // calls Attack.FindTarget once lastEmission + rateFrames is reached; recording
                        // dart_040_club, rt 2378.)
                        let lost = locked.is_none() && !ambient && aim_id != 0;
                        let (mut target, target_id) = match (locked, best) {
                            (Some((p, id)), _) => (p, id),
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
                                self.necro_fire(t, &nc, &proj, target_id, tiers);
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
                            if let (true, Some(_), Some(g)) = (lost, grot, tw.game_rot) {
                                grot = Some(g);
                                dir = (g - 90.0) * DEG2RAD;
                            }
                            if grot.is_some() {
                                tw.game_rot = grot;
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
                            let (tiers, shots) = (tw.tiers, tw.shots);
                            // HYPOTHESIS P-ALT: AlternateProjectileModel replaces every interval-th shot.
                            let (proj, emission) = match weapon.alternate.as_ref().filter(|a| a.interval > 0 && shots % a.interval == 0) {
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
                            // RandomArcEmissionModel.randomAngle works like the weapon's own spread.
                            let spread = weapon.spread.max(emission.random_angle);
                            if spread > 0.0 {
                                dir += ((self.rand() * 2.0 - 1.0) * spread).to_radians();
                                grot = None;
                            }
                            // HYPOTHESIS P-TARGETSPREAD: RandomTargetSpreadModel moves the aim point
                            // uniformly within a disc of radius `spread`.
                            if emission.target_spread > 0.0 {
                                let (a, r) = (self.rand() * std::f32::consts::TAU, emission.target_spread * self.rand().sqrt());
                                target = [target[0] + r * a.cos(), target[1] + r * a.sin()];
                            }
                            // Launch from the weapon's eject point. HYPOTHESIS P-EJECT: x is to the
                            // tower's right and y forward, as for sprites facing +y.
                            let (s, c) = if weapon.aim_offset.is_some() { face.sin_cos() } else { dir.sin_cos() };
                            // EmissionWithOffsetsModel: each shot from the next throw marker in turn.
                            let n = emission.markers.len();
                            let [ex, ey, ez] = if n > 0 { emission.markers[(self.towers[t].shots as usize).saturating_sub(1) % n] } else { weapon_eject };
                            // Weapon.GetEjectPosition (VERIFIED-DISASM @0xc6e520 + recording): the eject offset
                            // turned with the tower (game: Vector2.Rotate by -rotation; the clone's y is flipped,
                            // so its x mirrors), then Vector3.AddDisplaySpaceHeight moves it ejectZ / tan 60 down
                            // the game's map (up the clone's).
                            // The offset is built whole, then added to the tower's position.
                            let from = match grot {
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
                                    [base[0] + ox, base[1] + -(oy - ez / TAN60)]
                                }
                                None => [pos[0] + (c * ey - s * ex), pos[1] + ((s * ey + c * ex) + ez / TAN60)],
                            };
                            let start = self.projectiles.len();
                            if proj.link_radius {
                                let mut spec = proj.clone();
                                spec.radius = range;
                                self.emit(pos, dir, emission, &spec, crit, tiers, owner);
                            } else {
                                let bursts = self.bursts.len();
                                self.emit_rot = grot;
                                self.emit(from, dir, emission, proj, crit, tiers, owner);
                                self.emit_rot = None;
                                // MultiEmissionModel: its secondary projectile from the tower too.
                                if let Some(sec) = &emission.secondary {
                                    let single = Emission { kind: "Single".into(), count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0 };
                                    self.emit(pos, dir, &single, sec, None, tiers, owner);
                                }
                                if ambient {
                                    self.bursts.truncate(bursts);
                                }
                            }
                            for pr in &mut self.projectiles[start..] {
                                pr.aim = (target_id != 0).then_some(target_id);
                            }
                            if !ambient {
                                self.sound(&weapon.fire_sounds, None);
                            }
                            // Instant shots at a track point (Wall of Fire) sit there instead of on a bloon.
                            // HYPOTHESIS P-ATTARGET: InstantDamageEmissionModel (Jungle Vines) puts the
                            // projectile on its target bloon, hitting only it, like an instant shot.
                            if proj.instant || emission.kind == "AtTarget" {
                                for pr in &mut self.projectiles[start..] {
                                    pr.target = (target_id != 0).then_some(target_id);
                                    pr.pos = target;
                                }
                            }
                            // HYPOTHESIS P-BURST: BurstWeaponBehaviorModel repeats the shot count-1 more
                            // times, intervalFrames apart, at the same target. Only instant shots are
                            // delayed; travelling ones leave together.
                            if let Some(bf) = weapon.burst.as_ref().filter(|b| b.count > 1) {
                                for k in 1..bf.count {
                                    let s0 = self.projectiles.len();
                                    self.emit_rot = grot;
                                    self.emit(from, dir, emission, proj, crit, tiers, owner);
                                    self.emit_rot = None;
                                    if proj.instant || emission.kind == "AtTarget" {
                                        for pr in &mut self.projectiles[s0..] {
                                            pr.target = (target_id != 0).then_some(target_id);
                                            pr.pos = target;
                                            pr.wait = k as f32 * bf.frames;
                                            pr.frames_left = pr.frames_left.max(pr.wait + 2.0);
                                        }
                                    }
                                }
                            }
                            // HYPOTHESIS P-LINE: LineProjectileEmissionModel sweeps the shot along the
                            // range over its lifespan, wide enough that no bloon on the line is skipped.
                            if emission.kind == "Line" {
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
                                self.tracers.push((from, [from[0] + lc * range, from[1] + ls * range]));
                            }
                            if proj.seek.is_some() && !ambient {
                                for pr in &mut self.projectiles[start..] {
                                    pr.seek = Some(target_id);
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
                                    if ar.time > 0.0 && !ar.samples.is_empty() {
                                        pr.arrive_at = Some((now, pr.pos, target));
                                        let mut k = (ar.time * FPS).floor().max(1.0);
                                        while (k * (1.0f32 / 60.0)) / ar.time < 1.0 {
                                            k += 1.0;
                                        }
                                        pr.vel = [0.0, 0.0];
                                        pr.moving = k;
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
                            for pr in &mut self.projectiles[start..] {
                                pr.camo = camo;
                            }
                        }
                        // Weapon.SpawnDart after Emit: CalcRateFrames and UpdateMissedFrames;
                        // each whole missed frame is another shot now.
                        if !double {
                            let danger = self.danger;
                            let st = &mut self.towers[t].weapons[w];
                            st.dist_k = dist_k;
                            st.danger = danger;
                            st.necro = necro;
                            let r = st.rate(weapon);
                            st.rate_frames = rate_frames(r);
                            st.missed += missed_rate(r);
                            while st.missed > 1.0 {
                                st.missed -= 1.0;
                                queue.insert(qi, true);
                            }
                        }
                    }
                }
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
        for i in 0..self.bloons.len() {
            if !early.contains(&self.bloons[i].id) {
                self.tick_bloon_effects(i, round_clock, &mut dots, &mut emits);
            }
        }
        let single = Emission { kind: "Single".into(), count: 1, angle: 0.0, spread: 0.0, offset_start: 0.0, target_spread: 0.0, markers: Vec::new(), secondary: None, random_angle: 0.0, offset: 0.0 };
        for (pos, proj, owner) in emits {
            let tiers = self.towers.iter().find(|t| t.id == owner).map_or([0; 3], |t| t.tiers);
            self.emit(pos, 0.0, &single, &proj, None, tiers, owner);
        }
        self.phase = 2;
        // Movement + leaks.
        let mut i = 0;
        while i < self.bloons.len() {
            if early.contains(&self.bloons[i].id) || !self.move_bloon(i, &zones, &shoves, &mut leak_voice) {
                i += 1;
            }
        }
        // CreateSoundOnBloonLeak (OnLivesLost): the hero's line, at most every 2400 frames.
        if leak_voice {
            self.hero_voice(|s| &s.leak, "HeroLeak", 2400);
        }
        if self.lives <= 0.0 {
            self.lives = 0.0;
            self.game_over = true;
        }
        // DamageOverTime damage: in Bloon.Process after Bloon.Move (recording boomerang_500: the
        // MOAB its tick pops leaves its first Ceramic where the MOAB had moved to).
        let mut kids = Vec::new();
        for (id, damage, immune, owner, distribute) in dots {
            let Some(i) = self.bloons.iter().position(|b| b.id == id) else { continue };
            if immune & self.bloons[i].properties(&self.defs[self.bloons[i].kind]) != 0 {
                continue;
            }
            let tally0 = self.damage_tally;
            self.pop_after_move = true;
            self.distribute = distribute;
            let n = self.damage(i, damage.round() as i32, &mut kids);
            (self.pop_after_move, self.distribute) = (false, (true, false));
            let self_tally_dealt = self.damage_tally - tally0;
            if let Some(tw) = self.towers.iter_mut().find(|t| t.id == owner) {
                tw.pops += n;
                tw.damage += self_tally_dealt;
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
        for (pos, dir, ex, tiers, owner, target, grot) in exhausted {
            let start = self.projectiles.len();
            // ArcEmission.GetDirection starts from the emitting tower's rotation now (recording
            // dart_500: a Juggernaut's burst turns with the tower's newer throw); the projectile's
            // own when the tower's isn't known.
            self.emit_rot = self.towers.iter().find(|t| t.id == owner).and_then(|t| t.game_rot).or(grot);
            self.emit(pos, dir, &ex.emission, &ex.projectile, None, tiers, owner); // P-EXHAUST
            self.emit_rot = None;
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
        for (spec, pos, parent) in subs {
            self.spawn_sub(spec, pos, parent, false);
        }
        for (pos, dir, c, tiers, owner, skip) in contacts {
            let start = self.projectiles.len();
            self.emit(pos, dir, &c.emission, &c.projectile, None, tiers, owner);
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
        for (k, path) in to_emit {
            let id = self.spawn_emitted(k, path);
            if let Some(b) = self.bloons.last_mut() {
                b.fresh = false;
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
                self.preemptive_strikes(id);
                self.enter_voice(k);
            }
        }
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
        self.regrow();
        // Round end. Apopalypse: once the round has emitted everything (the last round still
        // waits for the bloons, for the win). STAND-IN P-APOP: our rounds don't overlap, so the
        // next round's emissions start then rather than running alongside.
        let last = self.round + 1 >= self.total_rounds();
        let emitted = self.apopalypse && !last;
        if self.race.is_none() && self.round_active && self.pending.is_empty() && (self.bloons.is_empty() || emitted) {
            self.round_active = false;
            self.finish_round();
        }
        self.race_step_end();
    }
    fn finish_round(&mut self){
            // The round's end flushes the factories too (VERIFIED by recording: Path.bloonsList
            // drops its dead entries that frame).
            self.plist_flush();
            // Simulation.RoundEnd -> DistributeXpToHeroes(GetXpForRound(completed round index)).
            let xp = Self::xp_for_round(self.round as i32);
            for t in 0..self.towers.len() {
                if self.hero_def(self.towers[t].def).is_some() {
                    let scale = self.submerged_scale(self.towers[t].pos, |s| (s.hero_xp, 1.0));
                    self.add_hero_xp(t, xp * scale);
                }
            }
            self.round += 1;
            // HYPOTHESIS P-ROUNDS: projectiles are cleared at round end unless AgeModel.rounds
            // says they survive more round ends (EndOfRoundClearBypassModel is Apopalypse-only:
            // there nothing is cleared).
            if !self.apopalypse && self.race.is_none() {
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
            if self.race.is_none() && self.round == self.end_round && !self.freeplay {
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
        // Hits now follow the collision grid's query order (every reachable bloon is ordered,
        // not the first few in storage order): a little more work per projectile.
        assert!(avg < 2.6e-3, "sim step avg {:.3} ms over the 2.6 ms budget", avg * 1e3);
        // A frame is 16.7 ms; the worst steps here pop 1000-1900 layers at once.
        assert!(worst < 10e-3, "sim step worst {:.3} ms over the 10 ms budget", worst * 1e3);
    }

    /// ~20 tier-5 towers and a Ceramic flood (`rate` a frame up to `cap` bloons) for `frames`
    /// steps; returns the average and worst step time (seconds) after the first 600.
    fn late_game_load(rate: usize, cap: usize, frames: usize, print: bool) -> (f64, f64) {
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
        let (mut total, mut worst, mut n, mut peak) = (0.0f64, 0.0f64, 0u32, 0usize);
        for f in 0..frames {
            if g.bloons.len() < cap {
                g.debug_spawn("Ceramic", rate, 0.0);
            }
            let (pp, np) = (g.popped, g.projectiles.len());
            let t0 = std::time::Instant::now();
            g.step();
            let dt = t0.elapsed().as_secs_f64();
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
        let q = [-156.13152f32, 13.558226];
        let (qx, qy) = grid_cell(q);
        eprintln!("offsets {} cell {qx},{qy}", g.grid_offsets.len());
        for (dx, dy) in [(0, 0), (0, -1), (0, 1)] {
            let c = &g.blocker_grid[((qy + dy) * 40 + qx + dx) as usize];
            eprintln!("cell {},{} span {} {:?}", qx + dx, qy + dy, c.0, c.1);
        }
        eprintln!("first offsets {:?}", &g.grid_offsets[..g.grid_offsets.len().min(20)]);
        eprintln!("hit {}", g.blocker_hit(q, 0.0));
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
                let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../research/dumps/recordings");
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
                }
                Some("#") if f.get(1) == Some(&"tower") => towers.push((f[2].into(), f[3].parse().unwrap(), f[4].parse().unwrap())),
                // SimRecorder inputs: a tower's targeting mode, and the frame abilities go off after.
                Some("#") if f.get(1) == Some(&"target") => targets.push((f[2].parse().unwrap(), f[3].to_string())),
                Some("#") if f.get(1) == Some(&"ability") => ability_at = f[2].parse().ok(),
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
        let (diff, mode) = (meta.get("difficulty").cloned().unwrap_or("Medium".into()), meta.get("mode").cloned().unwrap_or("Standard".into()));
        d.mode = match mode.as_str() {
            "Clicks" => "CHIMPS".into(),
            "Standard" => diff.clone(),
            m => format!("{diff} {m}"),
        };
        let mut g = Game::new(d);
        g.cash = 1e9;
        // The recorder raises the lives cap so leaks don't end the run (SimRecorder).
        g.lives = 1e6;
        // "DartMonkey-205": base tower and tiers.
        for (id, x, y) in &towers {
            let (base, tiers) = match id.split_once('-') {
                Some((b, t)) if t.len() == 3 => (b, t.bytes().map(|c| c - b'0').collect::<Vec<u8>>()),
                _ => (id.as_str(), vec![0, 0, 0]),
            };
            let def = g.tower_defs.iter().position(|(n, _)| n == base).unwrap();
            assert!(g.place_at(def, [*x, -*y], false), "can't place {id} at {x},{}", -y);
            let t = g.towers.iter().rposition(|t| t.sub.is_none()).unwrap();
            g.towers[t].tiers = [tiers[0], tiers[1], tiers[2]];
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
            if ability_at == Some(fi) {
                for tw in &mut g.towers {
                    tw.ability_cd.clear();
                }
                for name in g.ability_groups().into_iter().map(|a| a.name).collect::<Vec<_>>() {
                    while g.activate_ability(&name) {}
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
            if let Some((lo, hi)) = std::env::var("RRBTD6_REC_AT").ok().and_then(|v| v.split_once("..").map(|(a, b)| (a.parse::<i64>().unwrap_or(0), b.parse::<i64>().unwrap_or(0)))) {
                if (lo..=hi).contains(rt) {
                    eprintln!("  rt {rt} clone bloons {:?}", g.bloons.iter().map(|b| (b.id, g.names[b.kind].as_str(), b.dist, b.hp)).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone effects {:?}", g.bloons.iter().map(|b| (b.id, b.effects.iter().map(|e| (e.id.clone(), e.frames_left)).collect::<Vec<_>>())).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone darts {:?}", g.projectiles.iter().map(|p| (p.pos, p.vel, p.pierce, p.frames_left, p.grot)).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone towers {:?}", g.towers.iter().map(|t| (t.game_rot, t.base_rot)).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone plist {:?} live {:?}", g.proj_list, g.projectiles.iter().map(|p| p.uid).collect::<Vec<_>>());
                    eprintln!("  rt {rt} clone clock {} steps {} round {} active {} pending {} elapsed {}", g.round_clock, g.steps, g.round, g.round_active, g.pending.len(), g.sim_elapsed);
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
            for (ti, tw) in g.towers.iter().filter(|t| t.sub.is_none()).enumerate() {
                for (wi, st) in tw.weapons.iter().enumerate() {
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
            if let Some(s) = g.geraldo_stock.get_mut(rabbit) {
                s.qty = s.qty.max(1);
            }
            let p = g.track.pos(150.0 + 60.0 * k as f32);
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
        let run = |use_it: bool| {
            let mut g = game();
            let de = g.tower_defs.iter().position(|(n, _)| n == "Desperado").unwrap();
            let t = place_near(&mut g, de, 900.0);
            for _ in 0..4 {
                assert!(g.upgrade(t, 1));
            }
            g.round = 24;
            g.start_round();
            let mut early = 0;
            for f in 0..300 {
                if f == 120 && use_it {
                    g.towers[t].ability_cd.clear();
                    assert!(g.activate_ability("Marked To Pop"));
                }
                g.step();
                if f == 230 {
                    early = g.popped;
                }
            }
            (early, g.popped)
        };
        let (with, without) = (run(true), run(false));
        assert!(with.0 <= without.0 + 5, "nothing extra while marking: {with:?} {without:?}");
        assert!(with.1 > without.1 + 10, "execution pops: {with:?} {without:?}");
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

    /// P-PREEMPT: Pre-emptive Strike destroys round 40's MOAB within a second of it appearing.
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
            if g.bloons.iter().any(|b| g.names[b.kind] == "Moab") {
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
