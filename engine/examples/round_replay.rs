//! Headless replay of one round on a real map with real data, no towers.
//!
//!     cargo run -p btd6-sim --release --example round_replay -- [map] [round_set] [round_number] [easy|medium|hard]
//!
//! Reads local_extracted/engine_data/ (built from your own install). Prints
//! when each bloon spawns and leaks. These are predictions to compare
//! against the original game (tests/observations/README.md).

use btd6_sim::data::{BloonTable, RoundSets};
use btd6_sim::mapbuild::{build_path_points, parse_raw_path};
use btd6_sim::path::Path;
use btd6_sim::sim::Simulation;
use btd6_sim::time::FRAMES_PER_SECOND;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let map = args.first().map_or("tutorial", String::as_str);
    let set = args.get(1).map_or("DefaultRoundSet", String::as_str);
    let round_no: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    // GlobalSpeedModModel multipliers from modes.json (addition 0).
    let difficulty = args.get(3).map_or("easy", String::as_str);
    let speed_mult: f32 = match difficulty {
        "easy" => 1.0,
        "medium" => f32::from_bits(0x3F8C_CCCD), // 1.1f
        "hard" => 1.25,
        d => return Err(format!("unknown difficulty {d}")),
    };
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/engine_data/");
    let read = |p: &str| std::fs::read_to_string(format!("{dir}{p}")).map_err(|e| format!("{p}: {e}"));

    let mut bloons = BloonTable::parse(&read("bloons.tsv")?)?;
    if speed_mult != 1.0 {
        bloons.apply_global_speed_mod(speed_mult, 0.0);
    }
    println!("difficulty {difficulty}: global bloon speed x{speed_mult}");
    let sets = RoundSets::parse(&read("round_sets.tsv")?)?;
    let path = Path::new(&build_path_points(&parse_raw_path(&read(&format!("paths/{map}.tsv"))?)?), None, None);
    println!("map {map}: path length {} ({} table entries)", path.length(), path.table_len());

    let round = sets.round(set, round_no - 1, &bloons)?;
    let mut sim = Simulation::new(bloons.defs()?, vec![path], 1e9);
    sim.start_round(round_no as i32, &round);

    let mut spawned = 0usize;
    let mut first_leak = None;
    let mut last_leak = 0;
    while sim.rounds_emitting() || !sim.bloons.is_empty() {
        let before = sim.bloons.len() + sim.leaked_last_frame().len();
        sim.step();
        let now_spawned = sim.bloons.len() + sim.leaked as usize;
        if now_spawned > spawned {
            if spawned == 0 {
                println!("first spawn at frame {}", sim.frame);
            }
            spawned = now_spawned;
        }
        if !sim.leaked_last_frame().is_empty() {
            first_leak.get_or_insert(sim.frame);
            last_leak = sim.frame;
        }
        let _ = before;
        if sim.frame > 10_000_000 {
            return Err("did not finish".into());
        }
    }
    let fps = FRAMES_PER_SECOND as f32;
    let fl = first_leak.unwrap_or(0);
    println!("{set} round {round_no}: {} emissions, {} leaked", round.emissions.len(), sim.leaked);
    println!("first leak at frame {fl} ({:.3} s after round start)", fl as f32 / fps);
    println!("last leak at frame {last_leak} ({:.3} s)", last_leak as f32 / fps);
    println!("lives lost: {}", 1e9 - sim.lives);
    Ok(())
}
