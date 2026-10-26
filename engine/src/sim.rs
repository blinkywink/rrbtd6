//! Simulation driver: one [`Simulation::step`] is one `Simulation.Simulate()`.
//!
//! Tick order (VERIFIED-DISASM, research/04-native-analysis.md §2):
//! `time.elapsed++`, then every root object's `Process(time.elapsed)` in
//! process-list order. The spawner is created before any bloon, so it runs
//! first, and bloons follow in creation order.
//!
//! HYPOTHESIS H-ORDER-3: objects created during a frame join the process
//! list at the next `FlushProcessList`, so a bloon emitted on frame F first
//! moves on frame F+1. Inferred from the flush at the top of `Simulate`, not
//! yet traced through `RootObject` creation.

use crate::bloons::{BloonDef, BloonTypeId, Bloons};
use crate::path::Path;
use crate::rounds::{ActiveRound, Round};

/// `Spawner.Emit` start distance for bosses (@0x183F75F84).
pub const BOSS_START_DISTANCE: f32 = 40.0;
/// Spawn-clamp radius factor (@0x183F75E00).
pub const SPAWN_CLAMP_RADIUS_FACTOR: f32 = 1.25;

pub struct Simulation {
    /// `Simulation.time.elapsed`.
    pub frame: i32,
    pub defs: Vec<BloonDef>,
    pub paths: Vec<Path>,
    pub bloons: Bloons,
    /// `Map.mapWideBloonSpeed` (Map +0xC0).
    pub map_wide_bloon_speed: f32,
    /// `Simulation.health`. A KonFuze double in the original.
    pub lives: f64,
    pub leaked: u32,
    rounds: Vec<ActiveRound>,
    next_id: u32,
    /// Ids leaked during the last frame (reused buffer, no per-frame alloc).
    leak_scratch: Vec<u32>,
}

impl Simulation {
    pub fn new(defs: Vec<BloonDef>, paths: Vec<Path>, lives: f64) -> Self {
        assert!(!paths.is_empty());
        Self {
            frame: 0,
            defs,
            paths,
            bloons: Bloons::with_capacity(1024),
            map_wide_bloon_speed: 1.0,
            lives,
            leaked: 0,
            rounds: Vec::with_capacity(4),
            next_id: 1,
            leak_scratch: Vec::with_capacity(256),
        }
    }

    /// `Spawner.StartRound` → `AddEmissions`: times are offset by the
    /// current `time.elapsed`.
    pub fn start_round(&mut self, round_number: i32, round: &Round) {
        self.rounds.push(ActiveRound::new(round_number, self.frame, round));
    }

    pub fn rounds_emitting(&self) -> bool {
        self.rounds.iter().any(|r| r.emitting())
    }

    /// `Spawner.Emit` + `Bloon.SetPath(path, startDist, true)`.
    ///
    /// HYPOTHESIS H-SPAWN-1: single-path maps; spawn path selection via
    /// `spawnJunction` is not modelled.
    pub fn spawn(&mut self, kind: BloonTypeId, path: u16, round: i32, emission: i32) -> u32 {
        let def = &self.defs[kind.0 as usize];
        let start = if def.is_boss { BOSS_START_DISTANCE } else { 0.0 };
        self.spawn_at(kind, path, start, round, emission)
    }

    /// `SetPath` at an explicit distance (tests and benchmarks).
    pub fn spawn_at(&mut self, kind: BloonTypeId, path: u16, distance: f32, round: i32, emission: i32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let def = &self.defs[kind.0 as usize];
        let p = &self.paths[path as usize];
        let d = p.clamp_distance(distance);
        self.bloons.push(id, kind, path, d, def, round, emission);
        let i = self.bloons.len() - 1;
        let q = p.point_info_bloon(d, def.is_moab, def.is_boss);
        self.bloons.x[i] = q.pos.x;
        self.bloons.y[i] = q.pos.y;
        self.bloons.z[i] = q.pos.z;
        if def.rotate_to_follow_path {
            self.bloons.rotation[i] = q.rotation;
        }
        id
    }

    pub fn step(&mut self) {
        self.frame += 1;
        let elapsed = self.frame;
        self.leak_scratch.clear();

        // Bloons that exist before the spawner runs are the ones processed
        // this frame (H-ORDER-3).
        let processed = self.bloons.len();

        // Spawner.Process(elapsed): active rounds in start order, due
        // emissions in list order.
        self.emit_due(elapsed);

        // Bloon.Process for each pre-existing bloon, in creation order.
        let map_speed = self.map_wide_bloon_speed;
        for i in 0..processed {
            let b = &mut self.bloons;
            let def = &self.defs[b.kind[i].0 as usize];
            let path = &self.paths[b.path[i] as usize];
            // get_TotalSpeedFrames (no speed-modifying behaviours yet).
            let tsf = (b.speed_frames[i] * b.track_speed_mult[i]) * map_speed;
            let mut d = tsf + b.distance[i];

            // UpdatePositionAlongTrack(true)
            let r = b.radius[i];
            if path.leak_dist != crate::path::UNSET && d - r >= path.leak_dist {
                b.distance[i] = d;
                self.leak(i);
                continue;
            }
            d = path.clamp_distance(d);
            b.distance[i] = d;
            if path.is_end_of_map(d) {
                self.leak(i);
                continue;
            }
            let b = &mut self.bloons;
            if path.spawn_dist != crate::path::UNSET && path.spawn_dist >= r * SPAWN_CLAMP_RADIUS_FACTOR + d {
                d = path.spawn_dist - r * SPAWN_CLAMP_RADIUS_FACTOR;
                b.distance[i] = d;
            }
            let q = path.point_info_bloon(d, def.is_moab, def.is_boss);
            b.x[i] = q.pos.x;
            b.y[i] = q.pos.y;
            b.z[i] = q.pos.z;
            if def.rotate_to_follow_path {
                // HYPOTHESIS H-ROT-1: LerpAngle(prev, target, tsf * 0.5)
                // is not yet transcribed; snap to the target rotation.
                b.rotation[i] = q.rotation;
            }
        }

        self.bloons.compact();
        self.rounds.retain(|r| r.emitting());
    }

    fn emit_due(&mut self, elapsed: i32) {
        for r in 0..self.rounds.len() {
            let round_no = self.rounds[r].round;
            for k in self.rounds[r].advance_due(elapsed) {
                let p = self.rounds[r].pending_at(k);
                self.spawn(p.kind, 0, round_no, p.emission_index);
            }
        }
    }

    /// `Bloon.Leaked`. HYPOTHESIS H-LEAK-2: lives lost = total leak damage
    /// with no `CalcDamageFromBloon` modifiers (difficulty, shields,
    /// mutators) applied yet.
    fn leak(&mut self, i: usize) {
        let def = &self.defs[self.bloons.kind[i].0 as usize];
        self.lives -= def.total_leak_damage as f64;
        self.leaked += 1;
        self.leak_scratch.push(self.bloons.id[i]);
        self.bloons.dead[i] = true;
    }

    /// Ids leaked during the most recent frame.
    pub fn leaked_last_frame(&self) -> &[u32] {
        &self.leak_scratch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;
    use crate::path::PathPoint;
    use crate::rounds::Emission;

    fn def(speed_frames: f32) -> BloonDef {
        BloonDef {
            id: "T".into(),
            speed_frames,
            radius: 0.0,
            max_health: 1,
            total_leak_damage: 1.0,
            is_moab: false,
            is_boss: false,
            rotate_to_follow_path: false,
        }
    }

    fn straight(len: f32) -> Path {
        let p = |x| PathPoint { point: Vec3::new(x, 0.0, 0.0), has_extra: false };
        Path::new(&[p(0.0), p(len)], None, None)
    }

    #[test]
    fn emission_spawns_on_due_frame_and_moves_next_frame() {
        let mut s = Simulation::new(vec![def(1.0)], vec![straight(100.0)], 100.0);
        s.start_round(1, &Round::from_emissions(vec![Emission { time: 2.0, kind: BloonTypeId(0) }]));
        s.step(); // frame 1
        assert!(s.bloons.is_empty());
        s.step(); // frame 2: 0 + 2.0 <= 2 → emitted, not yet moved
        assert_eq!(s.bloons.len(), 1);
        assert_eq!(s.bloons.distance[0], 0.0);
        assert_eq!(s.bloons.y[0], -4.0, "DistanceToPointInfoBloon applies y -= 4");
        s.step(); // frame 3: first movement
        assert_eq!(s.bloons.distance[0], 1.0);
    }

    #[test]
    fn leak_happens_when_clamped_to_path_end() {
        let mut s = Simulation::new(vec![def(30.0)], vec![straight(100.0)], 100.0);
        s.spawn_at(BloonTypeId(0), 0, 0.0, 1, 0);
        for _ in 0..3 {
            s.step(); // 30, 60, 90
        }
        assert_eq!(s.leaked, 0);
        s.step(); // 120 → clamped to 100 → IsEndOfMap
        assert_eq!(s.leaked, 1);
        assert_eq!(s.lives, 99.0);
        assert!(s.bloons.is_empty());
    }

    #[test]
    fn removal_preserves_creation_order() {
        let mut s = Simulation::new(vec![def(1.0), def(50.0)], vec![straight(100.0)], 100.0);
        s.spawn_at(BloonTypeId(0), 0, 0.0, 1, 0);
        s.spawn_at(BloonTypeId(1), 0, 60.0, 1, 1);
        s.spawn_at(BloonTypeId(0), 0, 10.0, 1, 2);
        s.step();
        assert_eq!(s.leaked, 1);
        assert_eq!(s.bloons.id, vec![1, 3]);
    }

    #[test]
    fn map_wide_speed_multiplies() {
        let mut s = Simulation::new(vec![def(2.0)], vec![straight(100.0)], 100.0);
        s.map_wide_bloon_speed = 1.5;
        s.spawn_at(BloonTypeId(0), 0, 0.0, 1, 0);
        s.step();
        assert_eq!(s.bloons.distance[0], 3.0);
    }
}
