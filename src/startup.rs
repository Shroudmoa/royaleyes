//! The startup / initialization sequence: how the eye wakes up.

/// How long the wake-up sequence lasts, in seconds.
pub const DURATION: f32 = 2.5;

/// Eased wake-up progress (0..1) for a given amount of elapsed time.
pub fn ease(elapsed: f32) -> f32 {
    crate::util::smoothstep(0.0, 1.0, (elapsed / DURATION).clamp(0.0, 1.0))
}
