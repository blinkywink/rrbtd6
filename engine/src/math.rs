//! Math mirroring `Il2CppAssets.Scripts.Simulation.SMath`, transcribed from
//! the original machine code (research/04-native-analysis.md §4). The
//! original's f32 operation order is preserved exactly; IEEE-754 `+ - * /`
//! and `sqrt` are correctly rounded, so results match bit-for-bit.
//! `atan2f` comes from the C runtime in both builds, but may still differ by
//! an ulp (H-MATH-1). It is only used for rotations.

use core::ops::{Add, Mul, Sub};

/// `SMath.Math.Rad2Deg` (constant @0x183F76220).
pub const RAD2DEG: f32 = f32::from_bits(0x4265_2EE1); // 57.29578

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_squared(self, b: Vec2) -> f32 {
        let dx = self.x - b.x;
        let dy = self.y - b.y;
        dx * dx + dy * dy
    }

    pub fn distance(self, b: Vec2) -> f32 {
        self.distance_squared(b).sqrt()
    }

    /// `Vector2.Rotation`: `atan2f(x, y) * Rad2Deg`. The argument order
    /// `(x, y)` is what the original passes (angle measured from +y).
    pub fn rotation(self) -> f32 {
        self.x.atan2(self.y) * RAD2DEG
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: f32) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn xy(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// `Vector3.Distance` @0x180F5F820: `sqrtss((dx*dx + dy*dy) + dz*dz)`.
    pub fn distance(self, b: Vec3) -> f32 {
        let dx = self.x - b.x;
        let dy = self.y - b.y;
        let dz = self.z - b.z;
        ((dx * dx + dy * dy) + dz * dz).sqrt()
    }

    /// `Vector3 ==` @0x180F60A10: squared distance <= 1.4e-45f (smallest
    /// denormal), i.e. effectively exact equality.
    pub fn approx_eq(self, b: Vec3) -> bool {
        let dx = self.x - b.x;
        let dy = self.y - b.y;
        let dz = self.z - b.z;
        f32::from_bits(1) >= (dy * dy + dx * dx) + dz * dz
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    /// `Vector3 +` @0x1805197D0: componentwise.
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    /// `Vector3 * float` @0x180F60B40: `s*x, s*y, s*z`.
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(s * self.x, s * self.y, s * self.z)
    }
}

/// `SMath.Math.CapAngle` @0x180EA0140: wrap into `[0, 360)` by repeated
/// `+360` / `-360`.
pub fn cap_angle(mut a: f32) -> f32 {
    if 0.0 > a {
        loop {
            a += 360.0;
            if !(0.0 > a) {
                break;
            }
        }
    }
    while a >= 360.0 {
        a += -360.0;
    }
    a
}

/// x86 `cvttss2si` semantics: truncate toward zero; NaN or out-of-range
/// gives `i32::MIN` (Rust's `as` would saturate instead).
#[inline]
pub fn cvttss2si(v: f32) -> i32 {
    if v.is_nan() || v >= 2147483648.0 || v < -2147483648.0 { i32::MIN } else { v as i32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_angle_wraps() {
        assert_eq!(cap_angle(-90.0), 270.0);
        assert_eq!(cap_angle(360.0), 0.0);
        assert_eq!(cap_angle(725.0), 5.0);
    }

    #[test]
    fn cvtt_matches_x86() {
        assert_eq!(cvttss2si(2.9), 2);
        assert_eq!(cvttss2si(-2.9), -2);
        assert_eq!(cvttss2si(f32::NAN), i32::MIN);
        assert_eq!(cvttss2si(3e9), i32::MIN);
    }

    #[test]
    fn rotation_is_atan2_x_y() {
        assert_eq!(Vec2::new(0.0, 1.0).rotation(), 0.0);
        assert!((Vec2::new(1.0, 0.0).rotation() - 90.0).abs() < 1e-4);
    }
}
