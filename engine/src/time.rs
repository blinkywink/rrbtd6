//! Simulation time (research/04-native-analysis.md §1–2).
//!
//! VERIFIED-DISASM: `Simulation.time.elapsed` is an `int` frame counter
//! incremented once per tick, and ticks are produced by
//! `InGame.UpdateSimulation` at 60 per (time-scaled) second.

/// Ticks per in-game second at 1× (`InGame.UpdateSimulation`, f32 60.0
/// @0x183F75C50).
pub const FRAMES_PER_SECOND: i32 = 60;
/// Fast-forward multiplier applied to `Time.timeScale` (@0x183F75E58).
pub const FAST_FORWARD_SCALE: f64 = 3.0;
/// Upper clamp on `Time.timeScale` (@0x183F75F38).
pub const MAX_TIME_SCALE: f64 = 100.0;

/// A frame count, matching the original `int elapsed`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Frames(pub i32);

impl Frames {
    pub fn as_seconds(self) -> f32 {
        self.0 as f32 / FRAMES_PER_SECOND as f32
    }
}

/// The real-time to tick accumulator from `InGame.UpdateSimulation`.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameClock {
    /// Accumulated frames (`InGame` +0x170).
    pub acc: f64,
}

impl FrameClock {
    /// `scaled_dt` is Unity's `Time.deltaTime` (already multiplied by
    /// `time_scale`). Returns how many ticks to run.
    ///
    /// `capped` mirrors the `this[0xE0] == null` branch: true in normal
    /// play, so at most `time_scale + 1` ticks accrue per rendered frame.
    pub fn advance(&mut self, scaled_dt: f32, time_scale: f32, capped: bool) -> u32 {
        self.acc = (scaled_dt * 60.0f32) as f64 + self.acc;
        if capped {
            self.acc = self.acc.min(time_scale as f64 + 1.0);
        }
        let mut n = 0;
        while self.acc >= 1.0 {
            n += 1;
            self.acc -= 1.0;
        }
        n
    }

    /// Interpolation fraction passed to `EndSimulate`
    /// (`acc * (double)(1/60f)`).
    pub fn remainder_seconds(&self) -> f64 {
        self.acc * (1.0f32 / 60.0) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixty_ticks_per_second_and_cap() {
        let mut c = FrameClock::default();
        let mut n = 0;
        for _ in 0..60 {
            n += c.advance(1.0 / 60.0, 1.0, true);
        }
        assert!((59..=60).contains(&n));
        // A 1-second hitch at 1x only yields timeScale + 1 = 2 ticks.
        let mut c = FrameClock::default();
        assert_eq!(c.advance(1.0, 1.0, true), 2);
    }
}
