//! Tests for the chaos slider: the level is clamped, the bar and the click
//! mapping agree, turning the gain down actually silences the eye, and the bar
//! gets out of the way when nobody is using it.

use glitch_eye::chaos::{Slider, IDLE_HIDE, STEP};
use glitch_eye::eye::Eye;
use glitch_eye::frame::Frame;
use glitch_eye::rng::Rng;

const W: usize = 100;
const H: usize = 30;

/// Settle `shown` onto `level` so the assertions are not racing the glide, and
/// keep the bar up so it has not faded out.
fn settled(mut s: Slider) -> Slider {
    for _ in 0..200 {
        s.update(1.0 / 60.0, false);
        s.touch();
    }
    s
}

/// Run the clock forward without touching the fader.
fn idle(s: &mut Slider, seconds: f32) {
    for _ in 0..(seconds * 60.0) as u32 {
        s.update(1.0 / 60.0, false);
    }
}

/// Is anything of the bar actually painted on the frame's bottom row?
fn bar_painted(f: &Frame) -> bool {
    let (_, row, _) = Slider::layout(f.w, f.h).expect("bar fits");
    (0..f.w as i32).any(|x| f.get(x, row).ch != ' ')
}

#[test]
fn level_is_clamped_to_its_range() {
    let mut s = Slider::new(0.5);
    for _ in 0..100 {
        s.adjust(1.0);
    }
    assert_eq!(s.level(), 1.0, "should stop at the top");
    for _ in 0..200 {
        s.adjust(-1.0);
    }
    assert_eq!(s.level(), 0.0, "should stop at the bottom");

    s.set(9.0);
    assert_eq!(s.level(), 1.0);
    s.set(-9.0);
    assert_eq!(s.level(), 0.0);
    assert_eq!(Slider::new(-4.0).level(), 0.0);
    assert_eq!(Slider::new(4.0).level(), 1.0);
}

#[test]
fn one_press_moves_exactly_one_step() {
    let mut s = Slider::new(0.5);
    s.adjust(1.0);
    assert!((s.level() - (0.5 + STEP)).abs() < 1e-6, "got {}", s.level());
    s.adjust(-1.0);
    assert!((s.level() - 0.5).abs() < 1e-6);
}

#[test]
fn bar_glides_to_the_level_and_then_stops() {
    // Read the bar back out of the frame rather than trusting the internals.
    let mut f = Frame::new(W, H);
    let slider = settled(Slider::new(0.8));
    slider.draw(&mut f, W, H, false);
    let (_, row, cells) = Slider::layout(W, H).unwrap();
    assert_eq!(row, H as i32 - 1, "bar should sit on the bottom row");
    let start = (W as i32 - 14 - cells as i32) / 2 + 3 + "CHAOS".len() as i32;
    let full = (start..start + cells as i32)
        .filter(|&x| f.get(x, row).ch != '░' && f.get(x, row).ch != ' ')
        .count();
    let frac = full as f32 / cells as f32;
    assert!(
        (frac - 0.8).abs() < 0.12,
        "bar should read about 0.8, got {frac}"
    );
}

#[test]
fn clicking_the_track_sets_the_level() {
    let s = Slider::new(0.0);
    let (x, row, cells) = Slider::layout(W, H).unwrap();
    let start = x + 3 + "CHAOS".len() as i32;

    // Both ends of the track must be exactly reachable, or the bar feels like
    // it stops short.
    assert_eq!(s.level_at(start, row, W, H), Some(0.0), "first cell");
    assert_eq!(
        s.level_at(start + cells as i32 - 1, row, W, H),
        Some(1.0),
        "last cell"
    );
    let mid = s
        .level_at(start + (cells as i32 - 1) / 2, row, W, H)
        .expect("on the track");
    assert!(
        (mid - 0.5).abs() < 0.08,
        "middle should be about 0.5, got {mid}"
    );

    // The level it reports must be a legal one, and monotonic in x.
    let mut prev = -1.0;
    for col in start..start + cells as i32 {
        let v = s.level_at(col, row, W, H).unwrap();
        assert!((0.0..=1.0).contains(&v), "click gave {v}");
        assert!(v >= prev, "level should not go backwards: {prev} -> {v}");
        prev = v;
    }
}

#[test]
fn clicks_off_the_track_are_ignored() {
    let s = Slider::new(0.5);
    let (x, row, cells) = Slider::layout(W, H).unwrap();
    let start = x + 3 + "CHAOS".len() as i32;

    assert!(s.level_at(start - 1, row, W, H).is_none(), "the label");
    assert!(
        s.level_at(start + cells as i32, row, W, H).is_none(),
        "past the end"
    );
    assert!(s.level_at(start, row - 1, W, H).is_none(), "wrong row");
    assert!(s.level_at(0, row, W, H).is_none(), "far left");
    // The brackets are part of the bar but not of the track: clicking one must
    // not jump the level, or the controls feel like they are lying.
    assert!(s.level_at(x, row, W, H).is_none());
}

#[test]
fn the_bar_fits_whenever_it_is_drawn() {
    for w in 19..200 {
        let Some((x, row, cells)) = Slider::layout(w, H) else {
            continue;
        };
        let total = (3 + "CHAOS".len() + 2 + 5 + cells) as i32;
        assert!(x >= 0, "w={w}: bar starts off screen at {x}");
        assert!(
            x + total <= w as i32,
            "w={w}: bar needs {total} cells from {x}"
        );
        assert!(row >= 0 && (row as usize) < H);
    }
}

#[test]
fn tiny_terminals_get_no_bar_at_all() {
    // " CHAOS [" + 4 track cells + "] 0.50" is the narrowest bar worth showing.
    for (w, h) in [(1, 1), (10, 5), (18, 30), (100, 2), (0, 0), (19, 2)] {
        assert!(
            Slider::layout(w, h).is_none(),
            "w={w} h={h} should not fit a bar"
        );
        // And it must not panic when asked to draw one anyway.
        let mut f = Frame::new(w.max(1), h.max(1));
        Slider::new(0.5).draw(&mut f, w, h, false);
        assert!(Slider::new(0.5)
            .level_at(w as i32 / 2, h as i32, w, h)
            .is_none());
    }
    // Just wide enough, it comes back.
    assert!(Slider::layout(19, 3).is_some());
}

#[test]
fn the_bar_fades_away_after_three_idle_seconds() {
    let mut s = Slider::new(0.5);
    assert!(s.visible(), "the bar should be up at the start");

    // It must still be there right up to the deadline, then go.
    idle(&mut s, IDLE_HIDE - 0.2);
    assert!(s.visible(), "should still be up just before 3s");

    idle(&mut s, 0.2 + 0.4);
    assert!(!s.visible(), "should be gone after 3s");
}

#[test]
fn the_bar_is_not_drawn_once_it_has_gone() {
    let mut s = Slider::new(0.6);
    let mut f = Frame::new(W, H);
    s.draw(&mut f, W, H, false);
    assert!(bar_painted(&f), "the bar should be on screen to begin with");

    // Once hidden it must not paint a single cell. The eye repaints the frame
    // underneath, so anything left here would linger as a ghost.
    idle(&mut s, IDLE_HIDE + 0.5);
    f.cells.fill(glitch_eye::frame::Cell::default());
    s.draw(&mut f, W, H, false);
    assert!(!bar_painted(&f), "a hidden bar should leave the row alone");
}

#[test]
fn changing_the_level_brings_the_bar_back() {
    let mut s = Slider::new(0.5);
    idle(&mut s, IDLE_HIDE + 0.5);
    assert!(!s.visible(), "precondition: the bar has gone");

    s.adjust(1.0);
    assert!(s.visible(), "a keypress should bring it straight back");
    assert!((s.level() - 0.55).abs() < 1e-6);

    idle(&mut s, IDLE_HIDE + 0.5);
    assert!(!s.visible());
    s.set(0.2);
    assert!(s.visible(), "so should a click on the track");
}

#[test]
fn a_press_that_does_not_move_the_level_still_counts_as_using_it() {
    // Holding `+` against the top of the range is still someone using the
    // fader, and the bar must not fade out from under them.
    let mut s = Slider::new(1.0);
    for _ in 0..20 {
        s.adjust(1.0);
    }
    idle(&mut s, IDLE_HIDE - 0.5);
    assert!(s.visible(), "pressing + at the top should keep the bar up");
}

#[test]
fn hovering_keeps_the_bar_up() {
    // The bar should not dissolve while the cursor is resting on it.
    let mut s = Slider::new(0.5);
    let (x, row, _) = Slider::layout(W, H).unwrap();
    for _ in 0..(10.0 * 60.0) as u32 {
        s.update(1.0 / 60.0, true);
    }
    assert!(
        s.visible(),
        "10s of hovering should not hide the bar (still at {x},{row})"
    );
}

#[test]
fn a_hidden_bar_cannot_be_clicked_but_its_old_spot_wakes_it() {
    let mut s = Slider::new(0.5);
    let (x, row, cells) = Slider::layout(W, H).unwrap();
    let track = x + 3 + "CHAOS".len() as i32 + cells as i32 / 2;

    idle(&mut s, IDLE_HIDE + 0.5);
    // You cannot aim at a control you cannot see...
    assert!(s.level_at(track, row, W, H).is_none());
    assert!(!s.hovering(track, row, W, H));
    // ...but the cell is still remembered, so a click there brings it back.
    assert!(
        s.footprint(track, row, W, H),
        "the old bar's position should still be recognised"
    );
    s.touch();
    assert!(s.visible());
    assert!(s.level_at(track, row, W, H).is_some());
}

#[test]
fn the_fade_is_graded_rather_than_a_blink() {
    // If it snapped off in one frame it would strobe against the eye. Sample
    // across the fade and the brightness has to decrease every step.
    let mut s = Slider::new(0.9);
    idle(&mut s, IDLE_HIDE);

    let (_, row, _) = Slider::layout(W, H).unwrap();
    // Brightest cell of the bar's row, i.e. how present the bar currently is.
    let lum = |s: &Slider| {
        let mut f = Frame::new(W, H);
        s.draw(&mut f, W, H, false);
        (0..f.w as i32)
            .map(|x| f.get(x, row).fg.map_or(0u8, |c| c.0.max(c.1).max(c.2)))
            .max()
            .unwrap_or(0)
    };

    let first = lum(&s);
    let mut prev = first;
    let mut steps = 0;
    while s.visible() {
        s.update(1.0 / 60.0, false);
        let now = lum(&s);
        assert!(
            now <= prev,
            "the bar should only get dimmer while fading: {prev} -> {now}"
        );
        prev = now;
        steps += 1;
        assert!(steps < 100, "the fade should finish, not crawl");
    }
    assert!(steps >= 2, "the fade should take more than one frame");
    assert!(first > 0, "the bar should start out visible");
    assert!(
        prev <= 3,
        "and end up indistinguishable from the void, got {prev}"
    );
}

#[test]
fn gain_zero_leaves_the_eye_completely_alone() {
    // 40 simulated seconds, sampled every frame. With the fader at zero the
    // glitch layer must never see anything to do.
    let mut eye = Eye::new(0.0);
    let mut rng = Rng::new(21);
    let mut peak = 0.0f32;
    for _ in 0..(40 * 32) {
        eye.update(1.0 / 32.0, &mut rng, Some((50.0, 15.0)), W, H);
        peak = peak.max(eye.chaos);
        assert_eq!(eye.chaos, 0.0, "a silent eye should stay silent");
    }
    assert_eq!(peak, 0.0);
}

#[test]
fn gain_scales_chaos_without_exceeding_one() {
    for gain in [0.05_f32, 0.25, 0.5, 0.75, 1.0] {
        let mut eye = Eye::new(gain);
        let mut rng = Rng::new(33);
        let mut peak = 0.0f32;
        for _ in 0..(20 * 32) {
            eye.update(1.0 / 32.0, &mut rng, Some((50.0, 15.0)), W, H);
            assert!(
                (0.0..=1.0).contains(&eye.chaos),
                "gain {gain}: {}",
                eye.chaos
            );
            peak = peak.max(eye.chaos);
        }
        // The driver idles at 0.18 and spikes, so the output must actually move
        // at every gain above zero, and must stay under the cap.
        assert!(peak > 0.0, "gain {gain}: nothing ever happened");
        assert!(peak <= 1.0, "gain {gain}: overshot to {peak}");
    }
}

#[test]
fn full_gain_actually_reaches_the_top() {
    // Otherwise the top of the fader is a lie: you push it to maximum and get
    // something noticeably tamer than the bar promised.
    let mut eye = Eye::new(1.0);
    let mut rng = Rng::new(33);
    let mut peak = 0.0f32;
    for _ in 0..(120 * 32) {
        eye.update(1.0 / 32.0, &mut rng, Some((50.0, 15.0)), W, H);
        peak = peak.max(eye.chaos);
    }
    assert!(peak > 0.9, "full gain only reached {peak}");
}

#[test]
fn gain_is_ordered_quiet_below_loud() {
    // Same seed, same gestures: a higher fader must never be calmer.
    let peak = |gain: f32| {
        let mut eye = Eye::new(gain);
        let mut rng = Rng::new(5);
        let mut p = 0.0f32;
        for _ in 0..(20 * 32) {
            eye.update(1.0 / 32.0, &mut rng, None, W, H);
            p = p.max(eye.chaos);
        }
        p
    };
    let levels: Vec<f32> = (1..=5).map(|i| peak(i as f32 / 5.0)).collect();
    for w in levels.windows(2) {
        assert!(w[1] > w[0], "peaks should rise with gain: {levels:?}");
    }
}

#[test]
fn a_manual_burst_still_registers_at_zero_gain() {
    // Clicking at peaceful should be *quieter*, not completely ignored.
    let mut eye = Eye::new(0.0);
    let mut rng = Rng::new(9);
    eye.burst(&mut rng);
    eye.update(1.0 / 32.0, &mut rng, None, W, H);
    assert_eq!(eye.chaos, 0.0, "gain 0 still mutes the output");

    // ...and the driver underneath is wound up, so raising the fader releases
    // the stored burst instead of starting from scratch.
    let mut eye = Eye::new(0.0);
    let mut rng = Rng::new(9);
    for _ in 0..5 {
        eye.burst(&mut rng);
    }
    eye.gain = 1.0;
    eye.update(1.0 / 32.0, &mut rng, None, W, H);
    assert!(
        eye.chaos > 0.3,
        "stored burst should surface: {}",
        eye.chaos
    );
}
