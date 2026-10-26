//! Bloon track paths, transcribed from `Simulation.Track.Path`
//! (research/04-native-analysis.md §4). VERIFIED-DISASM for single paths;
//! splitters/routers (multi-path maps) are not implemented yet.
//!
//! The original samples each path into a `QuickPointInfo` table every
//! `CacheDistance = 0.01` units, and bloon positions come from that table
//! *without interpolation*. We build the identical table. It's stored as an
//! array of 20-byte records because each lookup reads all fields: one cache
//! miss instead of five (measured, benchmarks/README.md).

use crate::math::{Vec3, cap_angle, cvttss2si};

/// `Path.CacheDistance` (static, set in `Path..cctor`).
pub const CACHE_DISTANCE: f32 = f32::from_bits(0x3C23_D70A); // 0.01
/// `Path.MaxSegmentSize` (static). Not used by movement yet.
pub const MAX_SEGMENT_SIZE: f32 = 7.0;
/// Sentinel meaning "not set" for `spawnDist` / `leakDist`.
pub const UNSET: f32 = -1.0;

/// One `Models.Map.PointInfo`, reduced to what path building reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathPoint {
    pub point: Vec3,
    /// `id != null || PointInfo.HasExtraSettings(p)`.
    pub has_extra: bool,
}

/// One `QuickPointInfo` table entry (same 20-byte layout as the original).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuickPoint {
    pub extra: i32,
    pub pos: Vec3,
    pub rotation: f32,
}

#[derive(Clone, Debug)]
pub struct Path {
    /// Recomputed `PointInfo.distance` for each point.
    pub point_dist: Vec<f32>,
    pub total_length: f32,
    pub spawn_dist: f32,
    pub leak_dist: f32,
    table: Vec<QuickPoint>,
}

impl Path {
    /// `Path.CalcPointInfo`. `spawn_point`/`leak_point` are
    /// `PathModel.spawnPoint`/`leakPoint`.
    pub fn new(points: &[PathPoint], spawn_point: Option<Vec3>, leak_point: Option<Vec3>) -> Self {
        assert!(points.len() >= 2, "a path needs at least two points");
        // Pass 1: cumulative distances, spawn/leak markers.
        let mut total = 0.0f32;
        let mut spawn_dist = UNSET;
        let mut leak_dist = UNSET;
        let mut point_dist = Vec::with_capacity(points.len());
        for i in 0..points.len() {
            point_dist.push(total);
            if spawn_point.is_some_and(|s| points[i].point.approx_eq(s)) {
                spawn_dist = total;
            }
            if leak_point.is_some_and(|l| points[i].point.approx_eq(l)) {
                leak_dist = total;
            }
            if i + 1 < points.len() {
                total = points[i].point.distance(points[i + 1].point) + total;
            }
        }

        // Pass 2: the 0.01-step table.
        let n = cvttss2si(total / CACHE_DISTANCE).max(0) as usize;
        let cap = n + 1;
        let mut q = Self {
            point_dist,
            total_length: total,
            spawn_dist,
            leak_dist,
            table: Vec::with_capacity(cap),
        };
        let seg_rot = |s: usize| cap_angle((points[s].point.xy() - points[s - 1].point.xy()).rotation());
        let mut s = 1usize;
        let mut rot = seg_rot(s);
        let (mut d0, mut d1) = (q.point_dist[0], q.point_dist[1]);
        for i in 0..n {
            let d = i as f32 * CACHE_DISTANCE;
            while d >= d1 {
                s += 1;
                assert!(s < points.len(), "sample past the last point");
                rot = seg_rot(s);
                d0 = q.point_dist[s - 1];
                d1 = q.point_dist[s];
            }
            let t = (d - d0) / (d1 - d0);
            let (p0, p1) = (points[s - 1], points[s]);
            let pos = p0.point * (1.0 - t) + p1.point * t;
            let extra = if p0.has_extra || p1.has_extra { (s - 1) as i32 } else { -1 };
            q.push(QuickPoint { extra, pos, rotation: rot });
        }
        // table[n] = table[n-1] (or a zero entry if n == 0).
        let last = if n > 0 { q.get(n - 1) } else { QuickPoint { extra: -1, pos: Vec3::default(), rotation: 0.0 } };
        q.push(last);
        q
    }

    fn push(&mut self, p: QuickPoint) {
        self.table.push(p);
    }

    fn get(&self, i: usize) -> QuickPoint {
        self.table[i]
    }

    pub fn length(&self) -> f32 {
        self.total_length
    }

    pub fn table_len(&self) -> usize {
        self.table.len()
    }

    /// `Path.DistanceToPointInfo`: truncating index, clamped to the table.
    #[inline]
    pub fn point_info(&self, d: f32) -> QuickPoint {
        let idx = cvttss2si(d / CACHE_DISTANCE);
        let len = self.table.len() as i32;
        let i = if idx >= len { len - 1 } else if idx < 0 { 0 } else { idx };
        self.get(i as usize)
    }

    /// `Path.DistanceToPointInfoBloon`: `y -= 4`, `z += height`.
    #[inline]
    pub fn point_info_bloon(&self, d: f32, is_moab: bool, is_boss: bool) -> QuickPoint {
        let mut p = self.point_info(d);
        let h = if is_boss { 180.0 } else if is_moab { 120.0 } else { 20.0 };
        p.pos.z = h + p.pos.z;
        p.pos.y -= 4.0;
        p
    }

    /// `Path.DistanceToPath` for a path without entry/exit routers:
    /// exact clamp to `[0, total]`.
    #[inline]
    pub fn clamp_distance(&self, d: f32) -> f32 {
        if d >= self.total_length {
            self.total_length
        } else if 0.0 > d {
            0.0
        } else {
            d
        }
    }

    /// `Path.IsEndOfMap(d, false)` for a path without an exit router.
    #[inline]
    pub fn is_end_of_map(&self, d: f32) -> bool {
        d == self.total_length || (self.leak_dist != UNSET && d == self.leak_dist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(v: &[(f32, f32)]) -> Vec<PathPoint> {
        v.iter().map(|&(x, y)| PathPoint { point: Vec3::new(x, y, 0.0), has_extra: false }).collect()
    }

    #[test]
    fn distances_and_table_size() {
        let p = Path::new(&pts(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]), None, None);
        assert_eq!(p.length(), 20.0);
        assert_eq!(p.point_dist, vec![0.0, 10.0, 20.0]);
        // (int)(20 / 0.01f) entries plus the duplicated last one.
        assert_eq!(p.table_len(), cvttss2si(20.0 / CACHE_DISTANCE) as usize + 1);
        assert_eq!(p.spawn_dist, UNSET);
    }

    #[test]
    fn lookup_is_quantised_not_interpolated() {
        let p = Path::new(&pts(&[(0.0, 0.0), (10.0, 0.0)]), None, None);
        let a = p.point_info(5.0);
        let b = p.point_info(5.0 + CACHE_DISTANCE * 0.4);
        assert_eq!(a, b, "positions inside one 0.01 step must be identical");
        assert!((a.pos.x - 5.0).abs() < 0.011);
        assert_eq!(p.point_info(-3.0), p.point_info(0.0));
        assert_eq!(p.point_info(1e6), p.point_info(10.0));
    }

    #[test]
    fn spawn_and_leak_points_are_matched() {
        let pp = pts(&[(0.0, 0.0), (5.0, 0.0), (10.0, 0.0)]);
        let p = Path::new(&pp, Some(Vec3::new(5.0, 0.0, 0.0)), Some(Vec3::new(10.0, 0.0, 0.0)));
        assert_eq!(p.spawn_dist, 5.0);
        assert_eq!(p.leak_dist, 10.0);
    }

    #[test]
    fn end_of_map_needs_exact_clamp() {
        let p = Path::new(&pts(&[(0.0, 0.0), (10.0, 0.0)]), None, None);
        assert!(!p.is_end_of_map(10.5));
        assert!(p.is_end_of_map(p.clamp_distance(10.5)));
    }
}
