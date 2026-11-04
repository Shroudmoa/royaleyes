//! Renders the eye with the chaos bar over it, at a spread of gain levels, so
//! the fader can be reviewed in context. `cargo run --example gain_preview --
//! out.html`

use glitch_eye::chaos::Slider;
use glitch_eye::eye::Eye;
use glitch_eye::frame::Frame;
use glitch_eye::glitch;
use glitch_eye::rng::Rng;

const W: usize = 100;
const H: usize = 30;
const FPS: f32 = 32.0;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/opencode/gain.html".to_string());
    let mut html = String::with_capacity(1 << 22);
    html.push_str(
        "<!doctype html><meta charset=utf-8><style>\
         body{background:#0b0b12;margin:0;padding:14px;font:12px/1 monospace}\
         pre{margin:0 0 14px;display:inline-block;letter-spacing:0}\
         .cap{color:#8a8fa8;font:11px monospace;margin:0 0 4px}</style>",
    );

    // Peak chaos seen at each gain, measured over 30s, so each still is close
    // to the level the fader is actually asking for.
    let levels = [0.0_f32, 0.25, 0.55, 0.8, 1.0];
    for gain in levels {
        let mut rng = Rng::new(0xBEEF);
        let mut eye = Eye::new(gain);
        let mut slider = Slider::new(gain);
        let mut frame = Frame::new(W, H);
        let mut t = 0.0f32;
        let mut peak = 0.0f32;
        let mut peak_cells: Option<Vec<glitch_eye::frame::Cell>> = None;

        // Run a while, then keep the loudest frame: the rest level is calm, so
        // a single arbitrary still would not show what the fader does.
        while t < 30.0 {
            slider.update(1.0 / FPS, false);
            eye.gain = slider.level();
            eye.update(
                1.0 / FPS,
                &mut rng,
                Some((W as f32 * 0.62, H as f32 * 0.4)),
                W,
                H,
            );
            eye.draw(&mut frame, &mut rng);
            glitch::apply(&mut frame, &mut rng, eye.chaos, t);
            glitch::crt_drift(&mut frame, t, eye.chaos);
            // `>=` on the first frame matters: at gain 0 chaos is always 0, so
            // a strict comparison would never record anything at all.
            if eye.chaos >= peak {
                peak = eye.chaos;
                peak_cells = Some(frame.cells.clone());
            }
            t += 1.0 / FPS;
        }

        // Draw the bar onto the peak frame at a settled level.
        let mut f = Frame::new(W, H);
        f.cells = peak_cells.expect("at least one frame");
        for _ in 0..200 {
            slider.update(1.0 / FPS, false);
        }
        slider.draw(&mut f, W, H, false);

        html.push_str(&format!(
            "<div class=cap>gain {gain:.2}  (peak chaos {peak:.2})</div><pre>"
        ));
        for row in 0..H {
            for col in 0..W {
                push_span(&mut html, f.cells[row * W + col]);
            }
            html.push('\n');
        }
        html.push_str("</pre>");
    }

    std::fs::write(&path, html).expect("write html");
    println!("wrote {path}");
}

fn push_span(html: &mut String, cell: glitch_eye::frame::Cell) {
    use glitch_eye::color::Rgb;
    let fg = cell.fg.unwrap_or(Rgb(200, 200, 200));
    let bg = cell.bg.unwrap_or(Rgb(8, 8, 14));
    html.push_str(&format!(
        "<span style=\"color:#{:02x}{:02x}{:02x};background-color:#{:02x}{:02x}{:02x}\">",
        fg.0, fg.1, fg.2, bg.0, bg.1, bg.2
    ));
    match cell.ch {
        ' ' => html.push_str("&nbsp;"),
        c => html.push(c),
    }
    html.push_str("</span>");
}
