//! Tests for the eye states: behaviour per state, and smooth transitions.

use royaleyes::eye::Eye;
use royaleyes::frame::Frame;
use royaleyes::glitch;
use royaleyes::rng::Rng;
use royaleyes::state::EyeState;

const W: usize = 100;
const H: usize = 30;

/// Run the clock forward without gestures, letting a state settle.
fn settle(eye: &mut Eye, frames: usize) {
    let mut rng = Rng::new(7);
    for _ in 0..frames {
        eye.update(1.0 / 32.0, &mut rng, None, W, H);
    }
}

#[test]
fn normal_is_the_calm_default() {
    let mut eye = Eye::new(0.5);
    settle(&mut eye, 200);
    let p = eye.behaviour();
    assert_eq!(eye.state, EyeState::Normal);
    assert_eq!(p.open, 1.0, "the calm eye should rest wide open");
    assert!(!p.red, "calm eyes should not use the red accent");
    assert_eq!(p.glitch, 1.0, "chaos multiplier should be neutral");
    assert_eq!(p.burst, 1.0);
}

#[test]
fn alert_opens_wider_and_is_slightly_more_intense() {
    let mut eye = Eye::new(0.5);
    eye.set_state(EyeState::Alert);
    settle(&mut eye, 300);
    let p = eye.behaviour();
    assert_eq!(eye.state, EyeState::Alert);
    assert!(p.open > 1.0, "alert should open wider: {}", p.open);
    assert!(p.glitch > 1.0, "alert should be a bit stronger: {}", p.glitch);
    assert!(!p.red, "alert should not be red yet");
    assert!(p.burst > 1.0, "alert should flinch harder: {}", p.burst);
}

#[test]
fn warning_uses_the_red_accent_and_a_stronger_glitch() {
    let mut eye = Eye::new(0.5);
    eye.set_state(EyeState::Warning);
    settle(&mut eye, 300);
    let p = eye.behaviour();
    assert!(p.red, "warning should carry the red accent");
    assert!(p.glitch > 1.0, "warning should glitch more: {}", p.glitch);
    assert!(p.wander > 1.0, "warning should move more aggressively: {}", p.wander);
}

#[test]
fn critical_is_the_most_intense_of_all() {
    let mut eye = Eye::new(0.5);
    eye.set_state(EyeState::Critical);
    settle(&mut eye, 300);
    let p = eye.behaviour();
    assert!(p.red, "critical should carry the red accent");
    assert!(p.glitch > 2.0, "critical should glitch strongly: {}", p.glitch);
    assert!(p.wander > 3.0, "critical should wander erratically: {}", p.wander);
    assert!(p.burst > 2.0, "critical should distort aggressively: {}", p.burst);
}

#[test]
fn state_profiles_differ_in_their_targets() {
    let normal = EyeState::Normal.profile();
    let alert = EyeState::Alert.profile();
    let warning = EyeState::Warning.profile();
    let critical = EyeState::Critical.profile();
    // Every state has a distinct combination, and calm is the mildest.
    assert!(critical.open > alert.open);
    assert!(warning.glitch > normal.glitch);
    assert!(critical.glitch > warning.glitch);
    assert!(critical.wander > warning.wander);
    assert!(critical.burst > warning.burst);
    assert!(critical.red);
}

#[test]
fn state_switch_eases_in_instead_of_snapping() {
    let mut eye = Eye::new(0.0);
    eye.set_state(EyeState::Critical);
    let mut rng = Rng::new(3);
    for _ in 0..10 {
        eye.update(1.0 / 32.0, &mut rng, None, W, H);
        // Halfway through the transition the eye has not reached the critical
        // aperture yet, and it is still opening from the calm value.
        assert!(
            eye.open < 1.25 && eye.open > 1.0,
            "the transition should cross-fade, got open={}",
            eye.open
        );
    }
}

#[test]
fn settled_eye_reaches_its_state_target() {
    let mut eye = Eye::new(0.0);
    eye.set_state(EyeState::Critical);
    let mut rng = Rng::new(3);
    for _ in 0..60 {
        eye.update(1.0 / 32.0, &mut rng, None, W, H);
    }
    assert_eq!(eye.behaviour().open, 1.25);
}

#[test]
fn glitch_intensity_is_scaled_by_the_state() {
    let mut eye = Eye::new(0.0);
    settle(&mut eye, 200);
    assert_eq!(eye.behaviour().glitch, 1.0);

    let mut hot = Eye::new(0.0);
    hot.set_state(EyeState::Critical);
    settle(&mut hot, 300);
    assert_eq!(hot.behaviour().glitch, 4.0);

    let mut red = Eye::new(0.0);
    red.set_state(EyeState::Warning);
    settle(&mut red, 300);
    assert_eq!(red.behaviour().glitch, 2.4);
}

#[test]
fn red_glitch_uses_red_static_noise() {
    let mut f = Frame::new(40, 12);
    let mut rng = Rng::new(5);
    for _ in 0..8 {
        glitch::apply(&mut f, &mut rng, 0.9, 2.0, true);
    }
}
