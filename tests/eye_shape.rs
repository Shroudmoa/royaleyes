//! Structural tests: these check the eye actually comes out shaped like an eye
//! rather than merely "not panicking".

use glitch_eye::color::Rgb;
use glitch_eye::eye::Eye;
use glitch_eye::frame::Frame;
use glitch_eye::glitch;
use glitch_eye::rng::Rng;

const W: usize = 100;
const H: usize = 30;

struct Rendered {
    rows: Vec<usize>,
    min_x: usize,
    max_x: usize,
    min_y: usize,
    max_y: usize,
    brightest: f32,
    darkest: f32,
    iris_pixels: usize,
    iris_centre_x: f32,
}

fn render(eye: &Eye) -> Rendered {
    let mut f = Frame::new(W, H);
    let mut rng = Rng::new(1);
    eye.draw(&mut f, &mut rng);

    let mut out = Rendered {
        rows: vec![0; H],
        min_x: usize::MAX,
        max_x: 0,
        min_y: usize::MAX,
        max_y: 0,
        brightest: 0.0,
        darkest: 255.0,
        iris_pixels: 0,
        iris_centre_x: 0.0,
    };
    let mut iris_x_sum = 0.0f32;

    for y in 0..H {
        for x in 0..W {
            let c = f.get(x as i32, y as i32);
            let Some(bg) = c.bg else { continue };
            let lum = (bg.0 as f32 * 30.0 + bg.1 as f32 * 59.0 + bg.2 as f32 * 11.0) / 100.0;
            if lum < 8.0 {
                continue; // still the void
            }
            out.rows[y] += 1;
            out.min_x = out.min_x.min(x);
            out.max_x = out.max_x.max(x);
            out.min_y = out.min_y.min(y);
            out.max_y = out.max_y.max(y);
            out.brightest = out.brightest.max(lum);
            out.darkest = out.darkest.min(lum);
            if bg.1 > bg.0 + 20 && bg.1 > bg.2 + 20 {
                out.iris_pixels += 1; // clearly green-ish: the iris
                iris_x_sum += x as f32;
            }
        }
    }
    if out.iris_pixels > 0 {
        out.iris_centre_x = iris_x_sum / out.iris_pixels as f32;
    }
    out
}

fn settled_eye() -> Eye {
    let mut eye = Eye::new(0.0);
    let mut rng = Rng::new(7);
    for _ in 0..90 {
        eye.update(1.0 / 32.0, &mut rng, Some((62.0, 12.0)), W, H);
    }
    eye
}

#[test]
fn eye_is_centred_and_wide() {
    let eye = settled_eye();
    let r = render(&eye);
    assert!(r.min_x < W, "nothing was drawn at all");

    let cx = (r.min_x + r.max_x) as f32 / 2.0;
    assert!(
        (cx - W as f32 / 2.0).abs() < 8.0,
        "eye should be roughly centred, got cx={cx}"
    );

    let w_cells = r.max_x - r.min_x + 1;
    let h_cells = r.max_y - r.min_y + 1;
    // Cells are twice as tall as wide, so 3:1 in physical terms is ~1.5:1 here.
    assert!(
        w_cells > h_cells * 3 / 2,
        "eye should be wider than it is tall: {w_cells}x{h_cells}"
    );
    assert!(h_cells >= 5, "eye is too squat to read: {h_cells} rows");
    assert!(h_cells <= 24, "eye overflows the terminal: {h_cells} rows");
}

#[test]
fn lids_taper_towards_the_corners() {
    // An almond: the first and last lit rows are much narrower than the middle.
    let eye = settled_eye();
    let r = render(&eye);
    let top = r.min_y;
    let bottom = r.max_y;
    let mid = (top + bottom) / 2;
    assert!(
        r.rows[top] * 3 < r.rows[mid],
        "top row should taper: {} vs {}",
        r.rows[top],
        r.rows[mid]
    );
    assert!(
        r.rows[bottom] * 3 < r.rows[mid],
        "bottom row should taper: {} vs {}",
        r.rows[bottom],
        r.rows[mid]
    );
    // ...and the widest row is somewhere in the middle, not at an edge.
    let peak = r
        .rows
        .iter()
        .enumerate()
        .max_by_key(|(_, n)| **n)
        .map(|(i, _)| i)
        .unwrap();
    assert!(
        peak > top && peak < bottom,
        "widest row {peak} is not strictly inside {top}..{bottom}"
    );
}

#[test]
fn has_sclera_pupil_and_iris() {
    let eye = settled_eye();
    let r = render(&eye);
    assert!(r.brightest > 200.0, "no bright sclera: {}", r.brightest);
    assert!(r.darkest < 40.0, "no dark pupil: {}", r.darkest);
    assert!(
        r.iris_pixels > 40,
        "iris barely visible: {} pixels",
        r.iris_pixels
    );
}

#[test]
fn blinking_squashes_the_eye() {
    let mut eye = settled_eye();
    let open = render(&eye);
    let open_h = open.rows.iter().sum::<usize>();
    let wide = open.max_x - open.min_x;

    eye.open = 0.0;
    let shut = render(&eye);
    let shut_h = shut.rows.iter().sum::<usize>();

    assert!(
        shut_h * 3 < open_h,
        "a shut eye should have far less of it: {shut_h} vs {open_h}"
    );
    // The shut eye keeps the same horizontal extent: the lids squeeze together
    // but the tips of the almond stay put.
    let shut_w = shut.max_x - shut.min_x;
    let open_centre = (open.min_x + open.max_x) as f32 / 2.0;
    let shut_centre = (shut.min_x + shut.max_x) as f32 / 2.0;
    assert!(
        (shut_centre - open_centre).abs() < 3.0,
        "a blink should not move the eye: {open_centre} -> {shut_centre}"
    );
    assert!(
        shut_w + 2 >= wide && shut_w <= wide + 2,
        "a blink should not change the width: {shut_w} vs {wide}"
    );
    assert!(shut_h > 0, "a shut eye still needs a visible lash line");
}

#[test]
fn gaze_follows_the_mouse() {
    let mut eye = Eye::new(0.0);
    let mut rng = Rng::new(3);
    for _ in 0..60 {
        eye.update(1.0 / 32.0, &mut rng, Some((6.0, 6.0)), W, H);
    }
    let left = render(&eye).iris_centre_x;
    for _ in 0..60 {
        eye.update(1.0 / 32.0, &mut rng, Some((94.0, 24.0)), W, H);
    }
    let right = render(&eye).iris_centre_x;
    assert!(
        right > left + 8.0,
        "iris should track the cursor: {left} -> {right}"
    );
}

#[test]
fn chaos_never_runs_away() {
    let mut eye = Eye::new(0.0);
    let mut rng = Rng::new(11);
    for _ in 0..2000 {
        eye.update(1.0 / 32.0, &mut rng, Some((50.0, 15.0)), W, H);
        assert!((0.0..=1.0).contains(&eye.chaos), "chaos {}", eye.chaos);
    }
    assert!(eye.chaos > 0.0, "chaos should never be fully off");
}

#[test]
fn glitch_layer_survives_extremes() {
    let eye = settled_eye();
    let mut f = Frame::new(W, H);
    let mut rng = Rng::new(5);
    for chaos in [0.0, 0.5, 1.0] {
        for t in [0.0, 3.3, 17.75] {
            eye.draw(&mut f, &mut rng);
            assert_eq!(f.cells.len(), W * H);
            glitch::apply(&mut f, &mut rng, chaos, t);
            glitch::crt_drift(&mut f, t, chaos);
            assert_eq!(f.cells.len(), W * H);
            // Nothing may write outside the buffer or emit an invalid colour.
            for c in &f.cells {
                for col in [c.bg, c.fg].into_iter().flatten() {
                    let _ = col.0; // u8 by construction; just prove it is one
                }
            }
        }
    }
}

#[test]
fn degenerate_terminal_sizes_do_not_panic() {
    let eye = settled_eye();
    let mut rng = Rng::new(13);
    for (w, h) in [(1, 1), (2, 1), (1, 40), (8, 4), (5, 5), (200, 3)] {
        let mut f = Frame::new(w, h);
        eye.draw(&mut f, &mut rng);
        glitch::apply(&mut f, &mut rng, 0.9, 2.0);
        glitch::crt_drift(&mut f, 2.0, 0.9);
        assert_eq!(f.cells.len(), w * h);
    }
}

#[test]
fn set_ignores_out_of_bounds() {
    use glitch_eye::frame::Cell;
    let mut f = Frame::new(4, 3);
    let paint = Cell::solid(Rgb(1, 2, 3));
    f.set(-1, 0, paint);
    f.set(0, 99, paint);
    f.set(4, 0, paint);
    f.set(0, 3, paint);
    assert_eq!(f.cells[0], Cell::default());
    assert_eq!(f.cells[3], Cell::default());
}
