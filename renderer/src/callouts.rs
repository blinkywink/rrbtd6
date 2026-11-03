//! CalloutManager, single player: the featured callout on the end screens' player stats
//! ("Fumigator - 376 Regrow Bloons Popped").
//!
//! CalloutManager's constructor builds its single-player CalloutTypes (weight, minimum value,
//! the difficulties they're awarded in), seeds a System.Random with GameModel.randomSeed
//! (UnityToSimulation.GetFreeplayRoundSeed) and lets each available type (IsAvailable: this
//! difficulty) assign itself (CalloutType.AssignCalloutToPlayer: its DataToGet value above 0 and
//! at least its minimum). The player's callouts are sorted by weight, highest first, and one is
//! picked at NextRandom: Random.Next(0, count). With a single player the pairs' low callouts and
//! TryAssignLowCallouts never apply (they compare players).

use crate::game::SessionStats;

/// CalloutTypes.Callout ids (CalloutUiData.calloutId).
pub const BUILD_MASTER: u32 = 0;
pub const POP_MASTER: u32 = 2;
pub const MONKEY_FINANCIER: u32 = 4;
pub const BOSS_SKILLZ: u32 = 6;
pub const WELL_ROUNDED: u32 = 9;
pub const HEAD_DETECTIVE: u32 = 11;
pub const METAL_MUNCHER: u32 = 12;
pub const FUMIGATOR: u32 = 13;
pub const FIRED_UP: u32 = 14;
pub const BFB_BRAWLER: u32 = 15;
pub const ZOMG_ZAPPER: u32 = 16;
pub const DDT_DESTROYER: u32 = 17;
pub const ACTIVATOR: u32 = 19;
pub const POWERED_UP: u32 = 20;
pub const INSTA_WINNER: u32 = 21;

/// The single-player list as CalloutManager..ctor (@0x1809b2e70) builds it: (callout, weight,
/// minimum value, Easy too (else Medium and Hard only)).
const SINGLE_PLAYER: [(u32, f32, f64, bool); 15] = [
    (BUILD_MASTER, 0.85, 5.0, true),
    (POP_MASTER, 0.75, 10000.0, true),
    (MONKEY_FINANCIER, 0.65, 5000.0, false),
    (BOSS_SKILLZ, 0.55, 5.0, false),
    (WELL_ROUNDED, 0.44, 5.0, true),
    (POWERED_UP, 0.51, 5.0, true),
    (INSTA_WINNER, 0.52, 5.0, true),
    (DDT_DESTROYER, 0.38, 5.0, true),
    (ZOMG_ZAPPER, 0.37, 5.0, true),
    (BFB_BRAWLER, 0.36, 5.0, true),
    (METAL_MUNCHER, 0.35, 100.0, true),
    (FIRED_UP, 0.34, 100.0, true),
    (FUMIGATOR, 0.33, 100.0, true),
    (HEAD_DETECTIVE, 0.32, 100.0, true),
    (ACTIVATOR, 0.3, 5.0, true),
];

/// Each type's DataToGet over the session (AnalyticsGameSessionInfo fields).
fn value(callout: u32, st: &SessionStats) -> f64 {
    match callout {
        BUILD_MASTER => st.towers_placed as f64,
        POP_MASTER => st.pops as f64,
        MONKEY_FINANCIER => st.eco,
        BOSS_SKILLZ => st.moabs() as f64,
        WELL_ROUNDED => st.well_rounded() as f64,
        HEAD_DETECTIVE => st.camo_pops as f64,
        METAL_MUNCHER => st.lead as f64,
        FUMIGATOR => st.regrow_pops as f64,
        FIRED_UP => st.ceramic as f64,
        BFB_BRAWLER => st.bfb as f64,
        ZOMG_ZAPPER => st.zomg as f64,
        DDT_DESTROYER => st.ddt as f64,
        ACTIVATOR => st.abilities as f64,
        // Powers and Insta Monkeys don't exist in the clone.
        _ => 0.0,
    }
}

/// The player's callout and its value, if any qualified.
pub fn pick(st: &SessionStats, difficulty: &str, seed: i32) -> Option<(u32, f64)> {
    let mut list: Vec<(u32, f32, f64)> = SINGLE_PLAYER
        .iter()
        .filter(|&&(_, _, _, easy)| easy || difficulty != "Easy")
        .filter_map(|&(c, w, min, _)| {
            let v = value(c, st);
            (v > 0.0 && v >= min).then_some((c, w, v))
        })
        .collect();
    if list.is_empty() {
        return None;
    }
    // <>c.<.ctor>b__6_0: by weight, highest first (the weights all differ).
    list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let k = DotNetRandom::new(seed).next_range(0, list.len() as i32) as usize;
    list.get(k).map(|&(c, _, v)| (c, v))
}

/// System.Random with a seed, as the game's mscorlib has it (Knuth's subtractive generator).
#[derive(Clone, Debug)]
pub struct DotNetRandom {
    seeds: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl DotNetRandom {
    pub fn new(seed: i32) -> Self {
        const MSEED: i32 = 161803398;
        let mut a = [0i32; 56];
        let sub = if seed == i32::MIN { i32::MAX } else { seed.abs() };
        let mut mj = MSEED - sub;
        a[55] = mj;
        let mut mk = 1i32;
        for i in 1..55 {
            let ii = (21 * i) % 55;
            a[ii] = mk;
            mk = mj.wrapping_sub(mk);
            if mk < 0 {
                mk = mk.wrapping_add(i32::MAX);
            }
            mj = a[ii];
        }
        for _ in 1..5 {
            for i in 1..56 {
                a[i] = a[i].wrapping_sub(a[1 + (i + 30) % 55]);
                if a[i] < 0 {
                    a[i] = a[i].wrapping_add(i32::MAX);
                }
            }
        }
        Self { seeds: a, inext: 0, inextp: 21 }
    }

    /// Random.Next(): InternalSample.
    pub fn next(&mut self) -> i32 {
        self.internal_sample()
    }

    fn internal_sample(&mut self) -> i32 {
        let inext = if self.inext + 1 >= 56 { 1 } else { self.inext + 1 };
        let inextp = if self.inextp + 1 >= 56 { 1 } else { self.inextp + 1 };
        let mut r = self.seeds[inext].wrapping_sub(self.seeds[inextp]);
        if r == i32::MAX {
            r -= 1;
        }
        if r < 0 {
            r = r.wrapping_add(i32::MAX);
        }
        self.seeds[inext] = r;
        self.inext = inext;
        self.inextp = inextp;
        r
    }

    /// Random.Next(min, max) for ranges that fit an int (all CalloutManager asks for).
    pub fn next_range(&mut self, min: i32, max: i32) -> i32 {
        let range = max as i64 - min as i64;
        (self.internal_sample() as f64 * (1.0 / i32::MAX as f64) * range as f64) as i32 + min
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `new System.Random(seed).Next(0, n)` as .NET Framework gives it (PowerShell 5.1).
    #[test]
    fn dotnet_random_matches_reference_sequence() {
        let mut r = DotNetRandom::new(42);
        let v: Vec<i32> = (0..5).map(|_| r.next_range(0, 100)).collect();
        assert_eq!(v, [66, 14, 12, 52, 16]);
        let mut r = DotNetRandom::new(530436181);
        let v: Vec<i32> = (0..3).map(|_| r.next_range(0, 15)).collect();
        assert_eq!(v, [3, 11, 0]);
        let mut r = DotNetRandom::new(0);
        let v: Vec<i32> = (0..3).map(|_| r.next_range(0, 100)).collect();
        assert_eq!(v, [72, 81, 76]);
    }

    #[test]
    fn single_qualifying_callout_is_picked() {
        let st = SessionStats { regrow_pops: 376, ..Default::default() };
        assert_eq!(pick(&st, "Medium", 530436181), Some((FUMIGATOR, 376.0)));
        assert_eq!(pick(&SessionStats::default(), "Medium", 1), None);
    }
}
