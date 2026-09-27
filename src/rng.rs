//! Tiny xorshift64* PRNG. No external deps, plenty fast for pixel-level noise.

#[derive(Clone, Copy)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // xorshift is unhappy with a zero state.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15 | 1)
    }

    pub fn from_entropy() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xDEAD_BEEF);
        // Mix in the address of a stack local so two processes started in the
        // same nanosecond still diverge.
        let stack_probe = &nanos as *const u64 as u64;
        Rng::new(nanos ^ stack_probe.rotate_left(17) ^ 0xA5A5_1234_DEAD_BEEF)
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[0, 1)`.
    #[inline]
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16_777_216.0
    }

    /// Uniform in `[lo, hi)`.
    #[inline]
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }

    /// Uniform in `[lo, hi)` as an integer.
    #[inline]
    pub fn i(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.f32() * (hi - lo) as f32) as i32
    }

    /// Uniform in `[-1, 1)`.
    #[inline]
    pub fn sym(&mut self) -> f32 {
        self.f32() * 2.0 - 1.0
    }

    #[inline]
    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }

    #[inline]
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.i(0, items.len() as i32) as usize]
    }
}
