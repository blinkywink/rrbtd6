//! The path a map's spawner sends each emitted bloon down (Spawner.Emit -> Splitter.SpawnRout), for
//! the splitter types other than the plain one. `MapData::spawner` comes from
//! tools/assets/export_map_mechanics.py; maps with a plain splitter have none and keep
//! `Game::spawn_tracks`.
//!
//! VERIFIED-DISASM: AlternateRoundSplitter.SpawnRout @0x180f64010, MoabOnlySplitter.SpawnRout
//! @0x180f67690 and UpdateActivePaths @0x180f67710, BloonTagOnlySplitter.SpawnRout @0x180f64160,
//! AlternateRoundMultiPathSplitter.SpawnRout. Not run against a recording of these maps.
//! Paths a map event turns on join later (map_logic.rs).

use serde::Deserialize;

/// A spawner's forward splitter as exported.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct SpawnerDef {
    pub kind: String,
    /// Path ids (`MapPath::id`), in the splitter's order.
    pub paths: Vec<i64>,
    #[serde(default)]
    pub rounds_per_path: i32,
    #[serde(default)]
    pub round_offset: i32,
    #[serde(default)]
    pub does_not_loop: bool,
    /// MoabOnlySplitter.moabOnlyPath.
    #[serde(default)]
    pub moab_only: Vec<i32>,
    /// BloonTagOnlySplitter.tagOnlyPath.
    #[serde(default)]
    pub tags: Vec<String>,
    /// AlternateRoundMultiPathSplitter.groupPaths: how many of `paths` each round group takes.
    #[serde(default)]
    pub groups: Vec<usize>,
    /// BloonTagSplitter.pathTagPairs: each of `paths`' tag, empty for any bloon.
    #[serde(default)]
    pub path_tags: Vec<String>,
    /// RuleSplitter.pathsRules: the rule for each of `paths`.
    #[serde(default)]
    pub rules: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum SpawnRoute {
    Alternate { active: Vec<usize>, per: i32, offset: i32, no_loop: bool },
    Moab { active: Vec<usize>, bloons: Vec<usize>, moabs: Vec<usize> },
    Tag { active: Vec<usize>, tags: Vec<String>, tagged: Vec<usize>, others: Vec<usize> },
    Multi { groups: Vec<Vec<usize>> },
    /// AlternateBloonSplitter: the base Splitter.SpawnRout.
    Plain { active: Vec<usize> },
    /// BloonTagSplitter: (tag, track) for tagged paths, and the untagged ones.
    AnyTag { active: Vec<usize>, tagged: Vec<(String, usize)>, others: Vec<usize> },
    /// RuleSplitter: (rule, track if active) per path, and the active tracks.
    Rule { rules: Vec<(String, Option<usize>)>, active: Vec<usize> },
}

impl SpawnRoute {
    /// `track_of`: a path id's index in the active tracks, None when it's inactive.
    pub fn new(def: &SpawnerDef, track_of: impl Fn(i64) -> Option<usize>) -> Option<Self> {
        let active: Vec<usize> = def.paths.iter().filter_map(|&id| track_of(id)).collect();
        Some(match def.kind.as_str() {
            "AlternateRoundSplitter" => {
                SpawnRoute::Alternate { active, per: def.rounds_per_path.max(1), offset: def.round_offset, no_loop: def.does_not_loop }
            }
            // UpdateActivePaths: the flags are indexed by position in the *active* paths.
            "MoabOnlySplitter" => {
                let flag = |i: usize| def.moab_only.get(i).copied().unwrap_or(0) != 0;
                let pick = |want: bool| active.iter().enumerate().filter(|(i, _)| flag(*i) == want).map(|(_, &t)| t).collect();
                let (bloons, moabs) = (pick(false), pick(true));
                SpawnRoute::Moab { active, bloons, moabs }
            }
            "BloonTagOnlySplitter" => {
                let tag = |i: usize| def.tags.get(i).map_or("", |s| s.as_str());
                let tags = (0..active.len()).map(tag).filter(|t| !t.is_empty()).map(String::from).collect();
                let pick = |want: bool| active.iter().enumerate().filter(|(i, _)| tag(*i).is_empty() != want).map(|(_, &t)| t).collect();
                let (tagged, others) = (pick(true), pick(false));
                SpawnRoute::Tag { active, tags, tagged, others }
            }
            "AlternateRoundMultiPathSplitter" => {
                let mut rest = def.paths.as_slice();
                let mut groups = Vec::new();
                for &n in &def.groups {
                    let (group, tail) = rest.split_at(n.min(rest.len()));
                    groups.push(group.iter().filter_map(|&id| track_of(id)).collect());
                    rest = tail;
                }
                SpawnRoute::Multi { groups }
            }
            "AlternateBloonSplitter" => SpawnRoute::Plain { active },
            "BloonTagSplitter" => {
                let (mut tagged, mut others) = (Vec::new(), Vec::new());
                for (i, &id) in def.paths.iter().enumerate() {
                    let Some(track) = track_of(id) else { continue };
                    match def.path_tags.get(i).filter(|t| !t.is_empty()) {
                        Some(tag) => tagged.push((tag.clone(), track)),
                        None => others.push(track),
                    }
                }
                SpawnRoute::AnyTag { active, tagged, others }
            }
            "RuleSplitter" => SpawnRoute::Rule {
                rules: def.rules.iter().zip(&def.paths).map(|(rule, &id)| (rule.clone(), track_of(id))).collect(),
                active,
            },
            _ => return None,
        })
    }

    /// Splitter.SpawnRout(emissionIndex, round (0-based), bloon). None: nothing to route on.
    pub fn route(&self, emission: i32, round: i32, bloon_tags: &[String]) -> Option<usize> {
        let at = |list: &[usize], i: i32| (!list.is_empty()).then(|| list[i.rem_euclid(list.len() as i32) as usize]);
        // Moab and tag splitters fall back on every active path when the chosen list is empty.
        let or_active = |list: &'_ [usize], active: &[usize]| at(if list.is_empty() { active } else { list }, emission);
        match self {
            SpawnRoute::Alternate { active, per, offset, no_loop } => {
                let index = (offset + round) / per;
                if *no_loop {
                    let last = active.len().checked_sub(1)?;
                    active.get((index as usize).min(last)).copied()
                } else {
                    at(active, index)
                }
            }
            SpawnRoute::Moab { active, bloons, moabs } => {
                let moab = bloon_tags.iter().any(|t| t == "Moabs");
                or_active(if moab { moabs } else { bloons }, active)
            }
            SpawnRoute::Tag { active, tags, tagged, others } => {
                let has = tags.is_empty() || bloon_tags.iter().any(|t| *t == tags[emission.rem_euclid(tags.len() as i32) as usize]);
                or_active(if has { tagged } else { others }, active)
            }
            SpawnRoute::Multi { groups } => {
                let group = groups.get(round.rem_euclid(groups.len().max(1) as i32) as usize)?;
                at(group, emission)
            }
            SpawnRoute::Plain { active } => at(active, emission),
            // SpawnRout: the paths whose tag the bloon has (GetViablePathIndexFromTags), else the
            // untagged ones. STAND-IN: the iterator's order is taken as the paths' order.
            SpawnRoute::AnyTag { active, tagged, others } => {
                let viable: Vec<usize> = tagged.iter().filter(|(tag, _)| bloon_tags.contains(tag)).map(|(_, t)| *t).collect();
                if viable.is_empty() { or_active(others, active) } else { at(&viable, emission) }
            }
            // SpawnRout: the first active path whose rule passes, else the last active path.
            SpawnRoute::Rule { rules, active } => {
                let passes = |rule: &str| match rule {
                    "RuleBossBloon" => bloon_tags.iter().any(|t| t == "Boss"),
                    "RulePrimeNumberRound" => is_prime(round + 1),
                    _ => false,
                };
                rules.iter().find_map(|(rule, track)| track.filter(|_| passes(rule))).or_else(|| active.last().copied())
            }
        }
    }
}

/// RulePrimeNumberRound.RulePassed's test.
fn is_prime(n: i32) -> bool {
    n >= 2 && (2..n).take_while(|d| d * d <= n).all(|d| n % d != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_splitter_takes_the_first_passing_path_else_the_last() {
        let def = SpawnerDef { kind: "RuleSplitter".into(), paths: vec![0, 1, 2], rules: vec!["RuleBossBloon".into(), "RulePrimeNumberRound".into()], ..Default::default() };
        let r = SpawnRoute::new(&def, all).unwrap();
        assert_eq!(r.route(0, 3, &tags(&["Boss"])), Some(0));
        // Round index 4 is round 5, a prime.
        assert_eq!(r.route(0, 4, &tags(&["Red"])), Some(1));
        assert_eq!(r.route(0, 5, &tags(&["Red"])), Some(2));
    }

    #[test]
    fn tag_splitter_uses_paths_the_bloon_can_take() {
        let def = SpawnerDef { kind: "BloonTagSplitter".into(), paths: vec![0, 1, 2], path_tags: vec!["".into(), "Moabs".into(), "Boss".into()], ..Default::default() };
        let r = SpawnRoute::new(&def, all).unwrap();
        assert_eq!(r.route(7, 0, &tags(&["Red"])), Some(0));
        assert_eq!(r.route(7, 0, &tags(&["Moabs"])), Some(1));
        assert_eq!([0, 1].map(|e| r.route(e, 0, &tags(&["Moabs", "Boss"]))), [Some(1), Some(2)]);
    }

    #[test]
    fn alternate_bloon_splitter_goes_round_by_emission() {
        let def = SpawnerDef { kind: "AlternateBloonSplitter".into(), paths: vec![0, 1], ..Default::default() };
        let r = SpawnRoute::new(&def, all).unwrap();
        assert_eq!([0, 1, 2].map(|e| r.route(e, 0, &[])), [Some(0), Some(1), Some(0)]);
    }

    fn tags(t: &[&str]) -> Vec<String> {
        t.iter().map(|s| s.to_string()).collect()
    }

    fn all(id: i64) -> Option<usize> {
        Some(id as usize)
    }

    #[test]
    fn alternate_round_changes_path_per_round() {
        let def = SpawnerDef { kind: "AlternateRoundSplitter".into(), paths: vec![0, 1], ..Default::default() };
        let r = SpawnRoute::new(&def, all).unwrap();
        assert_eq!([0, 1, 2].map(|round| r.route(5, round, &[])), [Some(0), Some(1), Some(0)]);
        let def = SpawnerDef { rounds_per_path: 2, round_offset: 1, does_not_loop: true, ..def };
        let r = SpawnRoute::new(&def, all).unwrap();
        assert_eq!([0, 1, 9].map(|round| r.route(0, round, &[])), [Some(0), Some(1), Some(1)]);
    }

    #[test]
    fn moab_flags_index_the_active_paths() {
        let def = SpawnerDef { kind: "MoabOnlySplitter".into(), paths: vec![0, 1, 2], moab_only: vec![0, 1, 0], ..Default::default() };
        // Path 0 inactive: flags [0, 1] fall on paths 1 and 2.
        let r = SpawnRoute::new(&def, |id| (id != 0).then_some(id as usize)).unwrap();
        assert_eq!(r.route(0, 0, &tags(&["Red"])), Some(1));
        assert_eq!(r.route(0, 0, &tags(&["Moabs"])), Some(2));
        // No MOAB path active: every active path.
        let r = SpawnRoute::new(&def, |id| (id != 1).then_some(id as usize)).unwrap();
        assert_eq!(r.route(1, 0, &tags(&["Moabs"])), Some(2));
    }

    #[test]
    fn tag_only_path_takes_only_that_tag() {
        let def = SpawnerDef { kind: "BloonTagOnlySplitter".into(), paths: vec![0, 1, 2], tags: tags(&["", "", "Boss"]), ..Default::default() };
        let r = SpawnRoute::new(&def, all).unwrap();
        assert_eq!(r.route(0, 0, &tags(&["Boss"])), Some(2));
        assert_eq!([0, 1, 2].map(|e| r.route(e, 0, &tags(&["Red"]))), [Some(0), Some(1), Some(0)]);
    }

    #[test]
    fn multi_path_takes_a_group_per_round() {
        let def = SpawnerDef { kind: "AlternateRoundMultiPathSplitter".into(), paths: vec![0, 1, 2, 3], groups: vec![2, 2], ..Default::default() };
        let r = SpawnRoute::new(&def, all).unwrap();
        assert_eq!([(0, 0), (1, 0), (0, 1), (1, 1), (0, 2)].map(|(e, round)| r.route(e, round, &[])), [Some(0), Some(1), Some(2), Some(3), Some(0)]);
    }

    #[test]
    fn plain_splitter_keeps_spawn_tracks() {
        assert!(SpawnRoute::new(&SpawnerDef { kind: "Splitter".into(), ..Default::default() }, all).is_none());
    }
}
