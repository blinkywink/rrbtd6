//! Random number generators, reproduced from GameAssembly.dll disassembly
//! (BTD6 57.2.12199). See research/04-native-rng.md for the annotated
//! machine code each function was transcribed from.
//!
//! Status: VERIFIED-DISASM. The instruction sequences were transcribed
//! directly. What remains is checking the outputs against the running game
//! (H-RNG-OBS-1).

/// `NinjaKiwi.Common.SeededRandom`: Park–Miller "minimal standard" LCG on a
/// 64-bit state (`long currentSeed` at +0x10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeededRandom {
    pub current_seed: i64,
}

const PM_MUL: i64 = 16807; // 0x41A7
const PM_MOD: i64 = 0x7FFF_FFFF;
/// `mulss` constant at 0x1840EBF40: 4.656612873077393e-10f (= 2^-31 as f32).
const SR_FLOAT_SCALE: f32 = f32::from_bits(0x3000_0000);
/// `divsd` constant at 0x1840EBDE0.
const SR_DOUBLE_DIV: f64 = 2147483646.0;

impl SeededRandom {
    /// `.ctor(long seed)`: `seed >= 1 ? seed : -seed`.
    pub fn from_long(seed: i64) -> Self {
        Self { current_seed: if seed >= 1 { seed } else { seed.wrapping_neg() } }
    }

    /// `.ctor(int seed)`: the same on 32 bits, then sign-extended.
    pub fn from_int(seed: i32) -> Self {
        let s = if seed >= 1 { seed } else { seed.wrapping_neg() };
        Self { current_seed: s as i64 }
    }

    /// `SetSeed(uint)`: zero-extended.
    pub fn set_seed_u32(&mut self, seed: u32) {
        self.current_seed = seed as i64;
    }

    /// `SetSeed(long)`: absolute value.
    pub fn set_seed_i64(&mut self, seed: i64) {
        self.current_seed = seed.wrapping_abs();
    }

    #[inline]
    fn step(&mut self) -> i64 {
        // imul (64-bit, wrapping), signed truncating remainder, abs via cmovns.
        let p = self.current_seed.wrapping_mul(PM_MUL);
        let r = (p % PM_MOD).wrapping_abs();
        self.current_seed = r;
        r
    }

    /// `Next()`: returns the low 32 bits of the new state.
    pub fn next(&mut self) -> i32 {
        self.step() as i32
    }

    /// `NextFloat()`: `(float)(int)state * 2^-31f`.
    pub fn next_float(&mut self) -> f32 {
        (self.step() as i32) as f32 * SR_FLOAT_SCALE
    }

    /// `NextDouble()`: `(double)(int)state / 2147483646.0`.
    pub fn next_double(&mut self) -> f64 {
        (self.step() as i32) as f64 / SR_DOUBLE_DIV
    }

    /// `Range(int, int)`: bounds are sorted; equal bounds return without
    /// advancing the state.
    pub fn range_i32(&mut self, a: i32, b: i32) -> i32 {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        if lo == hi {
            return lo;
        }
        let v = self.step() as i32;
        lo.wrapping_add(v.wrapping_rem(hi.wrapping_sub(lo)))
    }

    /// `Range(float, float)`: bounds sorted; `(v*scale)*(hi-lo)+lo` in f32.
    pub fn range_f32(&mut self, a: f32, b: f32) -> f32 {
        let (lo, hi) = if a > b { (b, a) } else { (a, b) };
        let span = hi - lo;
        let v = (self.step() as i32) as f32 * SR_FLOAT_SCALE;
        v * span + lo
    }
}

/// `NinjaKiwi.Common.Random`: identical machine code to .NET Framework
/// `System.Random` (Knuth subtractive generator). The linker folded
/// `InternalSample` into the same function as mscorlib's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NkRandom {
    inext: i32,
    inextp: i32,
    seed_array: [i32; 56],
}

const MBIG: i32 = i32::MAX;
const MSEED: i32 = 161_803_398; // 0x9A4EC86
/// `mulsd` constant at 0x183F76540: 1.0 / int.MaxValue.
const SAMPLE_SCALE: f64 = 4.656612875245797e-10;

impl NkRandom {
    pub fn new(seed: i32) -> Self {
        let subtraction = if seed == i32::MIN { i32::MAX } else { seed.wrapping_abs() };
        let mut sa = [0i32; 56];
        let mut mj = MSEED.wrapping_sub(subtraction);
        sa[55] = mj;
        let mut mk: i32 = 1;
        for i in 1..55 {
            let ii = (21 * i) % 55;
            sa[ii] = mk;
            mk = mj.wrapping_sub(mk);
            if mk < 0 {
                mk = mk.wrapping_add(MBIG);
            }
            mj = sa[ii];
        }
        for _ in 1..5 {
            for i in 1..56 {
                sa[i] = sa[i].wrapping_sub(sa[1 + (i + 30) % 55]);
                if sa[i] < 0 {
                    sa[i] = sa[i].wrapping_add(MBIG);
                }
            }
        }
        Self { inext: 0, inextp: 21, seed_array: sa }
    }

    fn internal_sample(&mut self) -> i32 {
        let mut loc_inext = self.inext + 1;
        if loc_inext >= 56 {
            loc_inext = 1;
        }
        let mut loc_inextp = self.inextp + 1;
        if loc_inextp >= 56 {
            loc_inextp = 1;
        }
        let mut ret = self.seed_array[loc_inext as usize].wrapping_sub(self.seed_array[loc_inextp as usize]);
        if ret == MBIG {
            ret -= 1;
        }
        if ret < 0 {
            ret = ret.wrapping_add(MBIG);
        }
        self.seed_array[loc_inext as usize] = ret;
        self.inext = loc_inext;
        self.inextp = loc_inextp;
        ret
    }

    pub fn sample(&mut self) -> f64 {
        self.internal_sample() as f64 * SAMPLE_SCALE
    }

    /// `Next()` (vtable slot 5).
    pub fn next(&mut self) -> i32 {
        self.internal_sample()
    }

    /// `Next(int max)`: `(int)(Sample() * max)`. Panics where the original throws.
    pub fn next_max(&mut self, max: i32) -> i32 {
        assert!(max >= 0, "ArgumentOutOfRangeException");
        (self.sample() * max as f64) as i32
    }

    fn sample_large_range(&mut self) -> f64 {
        let mut result = self.internal_sample();
        if self.internal_sample() % 2 == 0 {
            result = result.wrapping_neg();
        }
        (result as f64 + 2147483646.0) / 4294967293.0
    }

    /// `Next(int min, int max)`.
    pub fn next_range(&mut self, min: i32, max: i32) -> i32 {
        assert!(min <= max, "ArgumentOutOfRangeException");
        let range = max as i64 - min as i64;
        if range <= i32::MAX as i64 {
            (self.sample() * range as f64) as i32 + min
        } else {
            (self.sample_large_range() * range as f64) as i64 as i32 + min
        }
    }

    pub fn next_double(&mut self) -> f64 {
        self.sample()
    }
}

/// `NinjaKiwi.Common.BufferedRandom`: wraps [`NkRandom`] with one sample of
/// lookahead (`int next` at +0x18). Used for `Simulation.syncedRandom` /
/// `unsyncedRandom`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferedRandom {
    rng: NkRandom,
    next: i32,
}

impl BufferedRandom {
    pub fn new(seed: i32) -> Self {
        let mut rng = NkRandom::new(seed);
        let next = rng.next();
        Self { rng, next }
    }

    #[inline]
    fn take(&mut self) -> i32 {
        let v = self.next;
        self.next = self.rng.next();
        v
    }

    pub fn peek_next(&self) -> i32 {
        self.next
    }

    pub fn peek_next_double(&self) -> f64 {
        self.next as f64 * SAMPLE_SCALE
    }

    pub fn next(&mut self) -> i32 {
        self.take()
    }

    pub fn next_double(&mut self) -> f64 {
        self.take() as f64 * SAMPLE_SCALE
    }

    /// `Next(int max)`: `(int)((v * scale) * max)`.
    pub fn next_max(&mut self, max: i32) -> i32 {
        assert!(max >= 0, "ArgumentOutOfRangeException");
        let v = self.take() as f64 * SAMPLE_SCALE;
        (v * max as f64) as i32
    }

    /// `Next(int min, int max)`. Unlike `System.Random`, the original
    /// *throws* when `max - min > int.MaxValue`, so that case panics here.
    pub fn next_range(&mut self, min: i32, max: i32) -> i32 {
        assert!(min <= max, "ArgumentOutOfRangeException");
        let range = max as i64 - min as i64;
        assert!(range <= i32::MAX as i64, "ArgumentOutOfRangeException (range)");
        let v = self.take() as f64 * SAMPLE_SCALE;
        ((v * range as f64) as i32).wrapping_add(min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn park_miller_reference_sequence() {
        // Published minimal-standard sequence from seed 1.
        let mut r = SeededRandom::from_long(1);
        assert_eq!(r.next(), 16807);
        assert_eq!(r.next(), 282475249);
        assert_eq!(r.next(), 1622650073);
        // Park & Miller's check value: the 10,000th output from seed 1.
        let mut r = SeededRandom::from_long(1);
        let mut v = 0;
        for _ in 0..10_000 {
            v = r.next();
        }
        assert_eq!(v, 1043618065);
    }

    #[test]
    fn seeded_random_seed_rules() {
        assert_eq!(SeededRandom::from_long(0).current_seed, 0);
        assert_eq!(SeededRandom::from_long(-5).current_seed, 5);
        assert_eq!(SeededRandom::from_int(-7).current_seed, 7);
        let mut r = SeededRandom::from_long(3);
        assert_eq!(r.range_i32(4, 4), 4);
        assert_eq!(r.current_seed, 3, "equal bounds must not advance the state");
    }

    #[test]
    fn dotnet_framework_random_seed0() {
        // new System.Random(0).Next() on .NET Framework.
        let mut r = NkRandom::new(0);
        assert_eq!(r.next(), 1559595546);
        assert_eq!(r.next(), 1755192844);
        assert_eq!(r.next(), 1649316166);
    }

    #[test]
    fn buffered_random_lookahead() {
        let mut a = BufferedRandom::new(0);
        assert_eq!(a.peek_next(), 1559595546);
        assert_eq!(a.next(), 1559595546);
        assert_eq!(a.peek_next(), 1755192844);
    }
}
