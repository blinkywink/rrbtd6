//! Loading decoded game data (exported by tools/data_export/).
//!
//! The data files are produced locally from the user's own install and are
//! never committed. Derived quantities use the original formulas
//! (research/04-native-analysis.md):
//! - `speedFrames = speed * (1/60f)`: `BloonModel.set_Speed`, an f32 multiply
//!   by the constant 0x3C888889, not a division.
//! - `GetTotalLeakDamage = round_ties_even(total * leakDamageMultiplier)`,
//!   where `total = leakDamageSet > 0 ? leakDamageSet : leakDamage + Σ children`
//!   (children's already-rounded totals, added in list order).

use crate::bloons::{BloonDef, BloonTypeId};
use crate::rounds::{BloonGroup, Round};
use std::collections::HashMap;

/// `BloonModel.set_Speed` multiplier (@0x183F75CA0).
pub const SPEED_TO_FRAMES: f32 = f32::from_bits(0x3C88_8889);

#[derive(Clone, Debug)]
pub struct BloonRecord {
    pub id: String,
    pub speed: f32,
    pub radius: f32,
    pub max_health: i32,
    pub leak_damage: f32,
    pub leak_damage_set: f32,
    pub leak_damage_multiplier: f32,
    pub is_moab: bool,
    pub is_boss: bool,
    pub rotate_to_follow_path: bool,
    pub is_immune_to_slow: bool,
    pub children: Vec<String>,
}

#[derive(Debug)]
pub struct BloonTable {
    pub records: Vec<BloonRecord>,
    pub by_name: HashMap<String, BloonTypeId>,
}

fn hexf(s: &str) -> Result<f32, String> {
    let h = s.strip_prefix("0x").ok_or_else(|| format!("expected hex float bits, got {s}"))?;
    u32::from_str_radix(h, 16).map(f32::from_bits).map_err(|e| e.to_string())
}

impl BloonTable {
    pub fn parse(tsv: &str) -> Result<Self, String> {
        let mut records = Vec::new();
        let mut header_seen = false;
        for (n, line) in tsv.lines().enumerate() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            if !header_seen {
                header_seen = true;
                if !line.starts_with("id\t") {
                    return Err("missing header".into());
                }
                continue;
            }
            let c: Vec<&str> = line.split('\t').collect();
            if c.len() != 12 {
                return Err(format!("line {}: {} columns", n + 1, c.len()));
            }
            let flag = |s: &str| s == "1";
            records.push(BloonRecord {
                id: c[0].to_string(),
                speed: hexf(c[1])?,
                radius: hexf(c[2])?,
                max_health: c[3].parse().map_err(|e| format!("line {}: {e}", n + 1))?,
                leak_damage: hexf(c[4])?,
                leak_damage_set: hexf(c[5])?,
                leak_damage_multiplier: hexf(c[6])?,
                is_moab: flag(c[7]),
                is_boss: flag(c[8]),
                rotate_to_follow_path: flag(c[9]),
                is_immune_to_slow: flag(c[10]),
                children: c[11].split(',').filter(|s| !s.is_empty()).map(str::to_string).collect(),
            });
        }
        let by_name = records.iter().enumerate().map(|(i, r)| (r.id.clone(), BloonTypeId(i as u16))).collect();
        Ok(Self { records, by_name })
    }

    /// `GlobalSpeedModModel.ModBloon` (@0x1811370F0), applied to every
    /// BloonModel by `GameModelUtil.ApplyModToAllBloons`:
    /// `set_Speed(speed * multiplier + addition)`, with `isImmuneToSlow`
    /// temporarily cleared so the slow guard can't block it. Difficulty data:
    /// Medium ×1.1, Hard ×1.25, Easy none (modes.json). Whether Impoppable
    /// inherits Hard's mods is H-MODE-2.
    pub fn apply_global_speed_mod(&mut self, multiplier: f32, addition: f32) {
        for r in &mut self.records {
            r.speed = r.speed * multiplier + addition;
        }
    }

    pub fn id(&self, name: &str) -> Option<BloonTypeId> {
        self.by_name.get(name).copied()
    }

    /// `BloonModel.GetTotalLeakDamage`.
    pub fn total_leak_damage(&self, name: &str) -> Result<f32, String> {
        self.total_leak_damage_inner(name, 0)
    }

    fn total_leak_damage_inner(&self, name: &str, depth: u32) -> Result<f32, String> {
        if depth > 64 {
            return Err(format!("child cycle at {name}"));
        }
        let r = &self.records[self.id(name).ok_or_else(|| format!("unknown bloon {name}"))?.0 as usize];
        let total = if r.leak_damage_set > 0.0 {
            r.leak_damage_set
        } else {
            let mut t = r.leak_damage;
            for c in &r.children {
                t = t + self.total_leak_damage_inner(c, depth + 1)?;
            }
            t
        };
        Ok((total * r.leak_damage_multiplier).round_ties_even())
    }

    /// Engine definitions, indexed by `BloonTypeId`.
    pub fn defs(&self) -> Result<Vec<BloonDef>, String> {
        self.records
            .iter()
            .map(|r| {
                Ok(BloonDef {
                    id: r.id.clone(),
                    speed_frames: r.speed * SPEED_TO_FRAMES,
                    radius: r.radius,
                    max_health: r.max_health,
                    total_leak_damage: self.total_leak_damage(&r.id)?,
                    is_moab: r.is_moab,
                    is_boss: r.is_boss,
                    rotate_to_follow_path: r.rotate_to_follow_path,
                })
            })
            .collect()
    }
}

/// One `Data.Gameplay.BloonGroup` as stored in the round-set MonoBehaviour
/// (seconds, start + duration).
#[derive(Clone, Debug)]
pub struct RawGroup {
    pub bloon: String,
    pub count: i32,
    pub start: f32,
    pub duration: f32,
}

/// `Data.Gameplay.RoundSet` assets, keyed by name.
#[derive(Debug, Default)]
pub struct RoundSets {
    /// name -> (scale, rounds[groups])
    pub sets: HashMap<String, (f32, Vec<Vec<RawGroup>>)>,
}

/// 60.0f used by `RoundData.GetRoundDef` (@0x183F75C50).
const GROUP_FRAMES_PER_SECOND: f32 = 60.0;

impl RoundSets {
    pub fn parse(tsv: &str) -> Result<Self, String> {
        let mut out = RoundSets::default();
        for (n, line) in tsv.lines().enumerate() {
            if line.starts_with('#') || line.starts_with("set\t") || line.trim().is_empty() {
                continue;
            }
            let c: Vec<&str> = line.split('\t').collect();
            if c.len() != 8 {
                return Err(format!("line {}: {} columns", n + 1, c.len()));
            }
            let pi = |s: &str| s.parse::<usize>().map_err(|e| format!("line {}: {e}", n + 1));
            let (round, group) = (pi(c[2])?, pi(c[3])?);
            let entry = out.sets.entry(c[0].to_string()).or_insert((hexf(c[1])?, Vec::new()));
            if entry.1.len() <= round {
                entry.1.resize_with(round + 1, Vec::new);
            }
            let groups = &mut entry.1[round];
            if groups.len() != group {
                return Err(format!("line {}: groups out of order", n + 1));
            }
            groups.push(RawGroup {
                bloon: c[4].to_string(),
                count: c[5].parse().map_err(|e| format!("line {}: {e}", n + 1))?,
                start: hexf(c[6])?,
                duration: hexf(c[7])?,
            });
        }
        Ok(out)
    }

    /// `RoundSet.get_Def` → `RoundData.GetRoundDef(scale)` per round:
    /// `start = (scale*start)*60f`, `end = (scale*(duration+start))*60f`.
    pub fn round_groups(&self, set: &str, round_index: usize, bloons: &BloonTable) -> Result<Vec<BloonGroup>, String> {
        let (scale, rounds) = self.sets.get(set).ok_or_else(|| format!("unknown round set {set}"))?;
        let groups = rounds.get(round_index).ok_or_else(|| format!("{set} has no round index {round_index}"))?;
        groups
            .iter()
            .map(|g| {
                let end_s = g.duration + g.start; // BloonGroup.get_End
                Ok(BloonGroup {
                    kind: bloons.id(&g.bloon).ok_or_else(|| format!("unknown bloon {}", g.bloon))?,
                    start: (*scale * g.start) * GROUP_FRAMES_PER_SECOND,
                    end: (*scale * end_s) * GROUP_FRAMES_PER_SECOND,
                    count: g.count,
                })
            })
            .collect()
    }

    /// `RoundModel.emissions`: group emissions concatenated in group order
    /// (verified, `RoundModel.get_emissions` @0x1810D5710).
    pub fn round(&self, set: &str, round_index: usize, bloons: &BloonTable) -> Result<Round, String> {
        Ok(Round::from_groups(&self.round_groups(set, round_index, bloons)?))
    }

    pub fn round_count(&self, set: &str) -> usize {
        self.sets.get(set).map_or(0, |s| s.1.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# t\nid\tspeed\tradius\tmaxHealth\tleakDamage\tleakDamageSet\tleakDamageMultiplier\tisMoab\tisBoss\trotateToFollowPath\tisImmuneToSlow\tchildren\n\
Red\t0x41C80000\t0x41000000\t1\t0x3F800000\t0x00000000\t0x3F800000\t0\t0\t0\t0\t\n\
Blue\t0x420C0000\t0x41000000\t1\t0x3F800000\t0x00000000\t0x3F800000\t0\t0\t0\t0\tRed\n\
Half\t0x41C80000\t0x41000000\t1\t0x3F800000\t0x00000000\t0x40200000\t0\t0\t0\t0\tRed,Red\n";

    #[test]
    fn leak_damage_recursion_and_rounding() {
        let t = BloonTable::parse(SAMPLE).unwrap();
        assert_eq!(t.total_leak_damage("Red").unwrap(), 1.0);
        assert_eq!(t.total_leak_damage("Blue").unwrap(), 2.0);
        // (1 + 1 + 1) * 2.5 = 7.5 → ties-to-even → 8
        assert_eq!(t.total_leak_damage("Half").unwrap(), 8.0);
    }

    #[test]
    fn speed_frames_is_multiply_not_divide() {
        let t = BloonTable::parse(SAMPLE).unwrap();
        let d = t.defs().unwrap();
        assert_eq!(d[0].speed_frames, 25.0f32 * SPEED_TO_FRAMES);
    }

    /// Runs against the real export when it exists locally.
    #[test]
    fn real_export_if_present() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/engine_data/bloons.tsv");
        let Ok(text) = std::fs::read_to_string(p) else {
            eprintln!("SKIPPED: {p} not present");
            return;
        };
        let t = BloonTable::parse(&text).unwrap();
        let defs = t.defs().unwrap();
        assert!(defs.len() > 100);
        let red = &defs[t.id("Red").unwrap().0 as usize];
        assert_eq!(red.total_leak_damage, 1.0);
        // MOAB = 200 + 4 * Ceramic(10 + 2 * Rainbow(...)): a sanity check
        // on the recursion, not an asserted community number.
        // Known community values; these confirm the decode and the formula together.
        for (name, leak) in [("Moab", 616.0), ("Bfb", 3164.0), ("Zomg", 16656.0), ("Ddt", 816.0), ("Bad", 55760.0)] {
            assert_eq!(t.total_leak_damage(name).unwrap(), leak, "{name}");
        }
    }

    #[test]
    fn real_round_one_if_present() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/engine_data/");
        let (Ok(b), Ok(r)) = (
            std::fs::read_to_string(format!("{dir}bloons.tsv")),
            std::fs::read_to_string(format!("{dir}round_sets.tsv")),
        ) else {
            eprintln!("SKIPPED: engine_data not present");
            return;
        };
        let bloons = BloonTable::parse(&b).unwrap();
        let sets = RoundSets::parse(&r).unwrap();
        assert_eq!(sets.round_count("DefaultRoundSet"), 140);
        let g = sets.round_groups("DefaultRoundSet", 0, &bloons).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].count, 20);
        // Cross-check: the same group appears in sim_model as
        // "BloonGroupModel_Red x20 [1-10]" with end 1050.75 frames.
        assert_eq!(g[0].end, 1050.75);
        let round = sets.round("DefaultRoundSet", 0, &bloons).unwrap();
        assert_eq!(round.emissions.len(), 20);
    }
}
