//! Compatibility-test harness.
//!
//! An *observation* is a trace recorded from the original game (see
//! `tests/observations/README.md` for the format and how to record one).
//! Each test replays the same setup in `btd6-sim` and compares per-frame
//! values within an explicit tolerance. A test whose observation file is
//! missing is reported as SKIPPED, never as passed.
//!
//! Trace format (plain text, one sample per line, `#` comments):
//! `frame <i32> bloon <id> distance <f32>`

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub frame: i32,
    pub bloon: u32,
    pub distance: f32,
}

pub fn observations_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("observations")
}

pub fn parse_trace(text: &str) -> Result<Vec<Sample>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        match f.as_slice() {
            ["frame", fr, "bloon", id, "distance", d] => out.push(Sample {
                frame: fr.parse().map_err(|e| format!("line {}: {e}", n + 1))?,
                bloon: id.parse().map_err(|e| format!("line {}: {e}", n + 1))?,
                distance: d.parse().map_err(|e| format!("line {}: {e}", n + 1))?,
            }),
            _ => return Err(format!("line {}: unrecognised: {line}", n + 1)),
        }
    }
    Ok(out)
}

/// Loads an observation, or `None` if it has not been recorded yet.
pub fn load_observation(name: &str) -> Option<Vec<Sample>> {
    let p = observations_dir().join(name);
    let text = std::fs::read_to_string(&p).ok()?;
    Some(parse_trace(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
}

/// Compares two traces sample-by-sample; returns the first mismatch.
pub fn compare(expected: &[Sample], actual: &[Sample], tol: f32) -> Result<(), String> {
    if expected.len() != actual.len() {
        return Err(format!("sample count: expected {}, got {}", expected.len(), actual.len()));
    }
    for (e, a) in expected.iter().zip(actual) {
        if e.frame != a.frame || e.bloon != a.bloon || (e.distance - a.distance).abs() > tol {
            return Err(format!("expected {e:?}, got {a:?}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use btd6_sim::bloons::{BloonDef, BloonTypeId};
    use btd6_sim::math::Vec3;
    use btd6_sim::path::{Path as TrackPath, PathPoint};
    use btd6_sim::sim::Simulation;

    #[test]
    fn trace_parser_roundtrip() {
        let t = parse_trace("# c\nframe 3 bloon 1 distance 2.5\n").unwrap();
        assert_eq!(t, vec![Sample { frame: 3, bloon: 1, distance: 2.5 }]);
        assert!(parse_trace("nonsense").is_err());
    }

    #[test]
    fn simulation_is_deterministic() {
        let run = || {
            let def = BloonDef {
                id: "T".into(),
                speed_frames: 0.62,
                radius: 3.0,
                max_health: 1,
                total_leak_damage: 1.0,
                is_moab: false,
                is_boss: false,
                rotate_to_follow_path: true,
            };
            let p = |x, y| PathPoint { point: Vec3::new(x, y, 0.0), has_extra: false };
            let path = TrackPath::new(&[p(0.0, 0.0), p(300.0, 40.0), p(10.0, 200.0)], None, None);
            let mut s = Simulation::new(vec![def], vec![path], 100.0);
            for i in 0..50 {
                s.spawn_at(BloonTypeId(0), 0, i as f32 * 3.0, 1, i);
            }
            let mut out = Vec::new();
            for _ in 0..400 {
                s.step();
                for i in 0..s.bloons.len() {
                    out.push((s.frame, s.bloons.id[i], s.bloons.distance[i].to_bits()));
                }
            }
            out
        };
        assert_eq!(run(), run());
    }

    /// Red bloon on a measured straight segment, compared with a recorded
    /// trace. Ignored until `red_bloon_straight.trace` is recorded; run with
    /// `cargo test -p compat-tests -- --ignored`.
    #[test]
    #[ignore = "needs tests/observations/red_bloon_straight.trace"]
    fn red_bloon_movement_matches_original() {
        let Some(expected) = load_observation("red_bloon_straight.trace") else {
            eprintln!("SKIPPED: tests/observations/red_bloon_straight.trace not recorded yet");
            return;
        };
        let _ = expected;
        panic!("observation present but replay setup not implemented: needs decoded Red BloonModel + map path");
    }
}
