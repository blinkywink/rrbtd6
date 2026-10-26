//! Headless simulation benchmarks.
//!
//! Run: `cargo run -p sim-bench --release -- [bloons] [frames]`
//!
//! Measures wall time per frame and per bloon-frame for the movement
//! subsystem, and counts heap allocations made during the timed frames
//! (target: zero). Uses a synthetic path and synthetic bloon data; numbers
//! are NOT comparable with the original game until a matched workload is
//! measured there (see benchmarks/README.md).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use btd6_sim::bloons::{BloonDef, BloonTypeId};
use btd6_sim::math::Vec3;
use btd6_sim::path::{Path, PathPoint};
use btd6_sim::sim::Simulation;

struct CountingAlloc;
static ALLOCS: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

/// A long serpentine path, so bloons never leak during the run.
/// Also exercises the exact 0.01-step position table (~100 units per row).
fn serpentine(rows: usize) -> Path {
    let mut pts = Vec::with_capacity(rows * 2);
    for r in 0..rows {
        let y = r as f32 * 10.0;
        let (a, b) = if r % 2 == 0 { (0.0, 100.0) } else { (100.0, 0.0) };
        pts.push(PathPoint { point: Vec3::new(a, y, 0.0), has_extra: false });
        pts.push(PathPoint { point: Vec3::new(b, y, 0.0), has_extra: false });
    }
    Path::new(&pts, None, None)
}

fn main() {
    let args: Vec<usize> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let n_bloons = args.first().copied().unwrap_or(10_000);
    let frames = args.get(1).copied().unwrap_or(2_000);

    let def = BloonDef {
        id: "Synthetic".into(),
        speed_frames: 25.0 / 60.0,
        radius: 0.0,
        max_health: 1,
        total_leak_damage: 1.0,
        is_moab: false,
        is_boss: false,
        rotate_to_follow_path: false,
    };
    let mut sim = Simulation::new(vec![def], vec![serpentine(200)], 1e9);
    let len = sim.paths[0].length();
    for i in 0..n_bloons {
        // Spread over the first half of the track.
        let d = (i as f32 / n_bloons as f32) * len * 0.5;
        sim.spawn_at(BloonTypeId(0), 0, d, 1, i as i32);
    }

    // Warm-up.
    for _ in 0..50 {
        sim.step();
    }

    let a0 = ALLOCS.load(Ordering::Relaxed);
    let t0 = Instant::now();
    for _ in 0..frames {
        sim.step();
    }
    let dt = t0.elapsed();
    let allocs = ALLOCS.load(Ordering::Relaxed) - a0;

    let ns_frame = dt.as_nanos() as f64 / frames as f64;
    let ns_bloon_frame = ns_frame / n_bloons.max(1) as f64;
    println!(
        "{{\"bench\":\"movement\",\"bloons\":{n_bloons},\"frames\":{frames},\"ns_per_frame\":{ns_frame:.1},\
         \"ns_per_bloon_frame\":{ns_bloon_frame:.3},\"frames_per_sec\":{:.0},\"allocs_in_timed_frames\":{allocs},\
         \"leaked\":{}}}",
        1e9 / ns_frame,
        sim.leaked
    );
    assert_eq!(sim.leaked, 0, "benchmark path too short: bloons leaked");
}
