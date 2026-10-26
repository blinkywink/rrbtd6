//! Building `PathModel` points from map-scene data, transcribed from the
//! map loader (research/04-native-analysis.md §8).
//!
//! VERIFIED-DISASM:
//! - `SMath.Math.MakeBezierPoints(start, end, startTangent, endTangent, n)`
//!   @0x180EA11D0: `t = i/(n-1)`, both endpoints included.
//! - `BezierLineNodes.<GetPoints>d__0.MoveNext` @0x1814ABB40: walk the
//!   control points as `start, (t1, t2, end)*`. Each triple produces
//!   `MakeBezierPoints(prevEnd, end, t1, t2, 30)`, so a shared endpoint
//!   appears twice, as the last point of one segment and the first of the next.
//! - `SMath.Vector3.ToSimulationSpace(u, true, 60)` @0x180F605C0, as called
//!   by `Unity.Map.Path.get_Def`: `(u.x, (-u.z) - u.y / tanf(60*Deg2Rad), u.y)`.
//!
//! HYPOTHESIS H-PATH-3: world-space input positions (Unity Transform chain)
//! are reproduced exactly by the exporter, and child nodes are flattened in
//! transform order (`PolyLineNodes.GetPointInfo` not fully transcribed).

use crate::math::Vec3;

/// `SMath.Math.Deg2Rad` (@0x183F75C18).
pub const DEG2RAD: f32 = f32::from_bits(0x3C8E_FA35);
/// Bezier divisions used by `BezierLineNodes` (immediate 0x1E).
pub const BEZIER_DIVISIONS: i32 = 30;
/// Camera tilt passed to `ToSimulationSpace` by `Path.get_Def` (f32 60.0).
pub const PATH_TILT_DEGREES: f32 = 60.0;

/// `MakeBezierPoints`, preserving the f32 operation order:
/// `((s*uuu + ((t1*uu)*t)*3) + ((t2*u)*tt)*3) + e*ttt`.
pub fn make_bezier_points(start: Vec3, end: Vec3, t1: Vec3, t2: Vec3, division: i32, out: &mut Vec<Vec3>) {
    if division <= 0 {
        return;
    }
    let denom = (division - 1) as f32;
    for i in 0..division {
        let t = i as f32 / denom;
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;
        let a = start * uuu;
        let b = (t1 * uu) * t * 3.0;
        let c = (t2 * u) * tt * 3.0;
        let d = end * ttt;
        out.push(((a + b) + c) + d);
    }
}

/// `BezierLineNodes.GetPoints` over raw control points.
/// Trailing points that don't complete a triple are dropped, as in the
/// original.
pub fn bezier_line_points(control: &[Vec3]) -> Vec<Vec3> {
    let mut out = Vec::new();
    let Some((&first, rest)) = control.split_first() else { return out };
    let mut d = first;
    for tri in rest.chunks_exact(3) {
        let (t1, t2, end) = (tri[0], tri[1], tri[2]);
        make_bezier_points(d, end, t1, t2, BEZIER_DIVISIONS, &mut out);
        d = end;
    }
    out
}

/// `Vector3.ToSimulationSpace(u, applyTilt: true, angle)`.
pub fn to_simulation_space(u: Vec3, tilt_degrees: f32) -> Vec3 {
    let mut s = Vec3::new(u.x, -u.z, u.y);
    let k = (DEG2RAD * tilt_degrees).tan(); // CRT tanf in the original (H-MATH-1)
    if k != 0.0 {
        s.y -= s.z / k;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bezier_endpoints_and_count() {
        let s = Vec3::new(0.0, 0.0, 0.0);
        let e = Vec3::new(3.0, 0.0, 0.0);
        let pts = bezier_line_points(&[s, Vec3::new(1.0, 1.0, 0.0), Vec3::new(2.0, 1.0, 0.0), e]);
        assert_eq!(pts.len(), 30);
        assert_eq!(pts[0], s);
        assert_eq!(pts[29], e);
    }

    #[test]
    fn shared_endpoint_is_duplicated() {
        let p = |x: f32| Vec3::new(x, 0.0, 0.0);
        let pts = bezier_line_points(&[p(0.0), p(1.0), p(2.0), p(3.0), p(4.0), p(5.0), p(6.0), p(7.0)]);
        assert_eq!(pts.len(), 60); // trailing p(7) dropped
        assert_eq!(pts[29], pts[30]);
    }

    #[test]
    fn simulation_space_axes() {
        let s = to_simulation_space(Vec3::new(10.0, 0.0, 20.0), PATH_TILT_DEGREES);
        assert_eq!(s, Vec3::new(10.0, -20.0, 0.0));
        let h = to_simulation_space(Vec3::new(0.0, 5.0, 0.0), PATH_TILT_DEGREES);
        assert!((h.y + 5.0 / 3f32.sqrt()).abs() < 1e-5);
    }
}

/// One child of a Unity `Path` object, in Unity world space.
#[derive(Clone, Debug, PartialEq)]
pub enum RawNode {
    /// Plain transform: `PointSource.CreatePointInfo` with default settings.
    Line(Vec3),
    /// `BezierLineNodes`: control points in child order.
    Curve(Vec<Vec3>),
}

/// Parses the TSV written by `tools/data_export/export_paths.py`.
pub fn parse_raw_path(tsv: &str) -> Result<Vec<RawNode>, String> {
    let hexf = |s: &str| -> Result<f32, String> {
        let h = s.strip_prefix("0x").ok_or_else(|| format!("bad float {s}"))?;
        u32::from_str_radix(h, 16).map(f32::from_bits).map_err(|e| e.to_string())
    };
    let mut out = Vec::new();
    for (n, line) in tsv.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let c: Vec<&str> = line.split('\t').collect();
        let nums = c[1..].iter().map(|s| hexf(s)).collect::<Result<Vec<f32>, _>>()?;
        if nums.len() % 3 != 0 {
            return Err(format!("line {}: coordinate count", n + 1));
        }
        let pts: Vec<Vec3> = nums.chunks_exact(3).map(|v| Vec3::new(v[0], v[1], v[2])).collect();
        out.push(match c[0] {
            "L" if pts.len() == 1 => RawNode::Line(pts[0]),
            "C" => RawNode::Curve(pts),
            k => return Err(format!("line {}: kind {k}", n + 1)),
        });
    }
    Ok(out)
}

/// `PolyLineNodes.GetPointInfo` (@0x1814A7330) flattening, followed by
/// `Unity.Map.Path.get_Def`'s `ToSimulationSpace(p, true, 60)`.
/// No `PointSource` components are modelled (none on Monkey Meadow), so
/// every point has default settings (`has_extra = false`).
pub fn build_path_points(nodes: &[RawNode]) -> Vec<crate::path::PathPoint> {
    let mut unity = Vec::new();
    for n in nodes {
        match n {
            RawNode::Line(p) => unity.push(*p),
            RawNode::Curve(ctrl) => unity.extend(bezier_line_points(ctrl)),
        }
    }
    unity
        .into_iter()
        .map(|u| crate::path::PathPoint { point: to_simulation_space(u, PATH_TILT_DEGREES), has_extra: false })
        .collect()
}

#[cfg(test)]
mod real_map_tests {
    use super::*;

    #[test]
    fn monkey_meadow_if_present() {
        let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../local_extracted/engine_data/paths/tutorial.tsv");
        let Ok(text) = std::fs::read_to_string(p) else {
            eprintln!("SKIPPED: {p} not present");
            return;
        };
        let nodes = parse_raw_path(&text).unwrap();
        let pts = build_path_points(&nodes);
        let path = crate::path::Path::new(&pts, None, None);
        eprintln!(
            "Monkey Meadow: {} raw nodes, {} points, length {}, table {}",
            nodes.len(),
            pts.len(),
            path.length(),
            path.table_len()
        );
        assert!(path.length() > 500.0);
    }
}
