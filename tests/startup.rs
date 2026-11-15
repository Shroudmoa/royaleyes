//! Tests for the startup / initialization sequence.

use royaleyes::startup;

#[test]
fn ease_starts_quiet_and_ends_open() {
    assert_eq!(startup::ease(0.0), 0.0);
    assert_eq!(startup::ease(f32::MAX), 1.0);
}

#[test]
fn ease_is_smooth_in_and_out() {
    // The smoothstep midpoint of 0..1 is 0.5.
    assert!((startup::ease(1.25) - 0.5).abs() < 1e-6);
}

#[test]
fn the_wake_up_lasts_about_two_to_three_seconds() {
    assert!(startup::DURATION > 0.0);
    assert!(startup::DURATION <= 3.0, "took {}s", startup::DURATION);
}
