//! Renders frames straight to an HTML file so the output can be eyeballed
//! outside a terminal. `cargo run --example snapshot -- out.html`
//!
//! Pass a second argument to pick the timestamps to capture, and a third to
//! also dump the raw cell grid (`bg fg ch` per cell) for pixel-exact review.

use glitch_eye::color::Rgb;
use glitch_eye::eye::Eye;
use glitch_eye::frame::{Cell, Frame};
use glitch_eye::glitch;
use glitch_eye::rng::Rng;

const W: usize = 100;
const H: usize = 30;
const FPS: f32 = 32.0;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/opencode/eye.html".to_string());
    let dump = std::env::args().nth(3);

    let mut rng = Rng::new(0xC0FFEE);
    let mut eye = Eye::new(0.12);
    let mut frame = Frame::new(W, H);
    let mut html = String::with_capacity(1 << 22);
    html.push_str(
        "<!doctype html><meta charset=utf-8><style>\
         body{background:#111;margin:0;padding:14px;font:12px/1 monospace}\
         pre{margin:0 0 10px;display:inline-block;letter-spacing:0}\
         .cap{color:#666;font:11px monospace;margin:0 0 4px}</style>",
    );

    // Pick a spread of interesting moments: calm, chaos peak, mid-blink.
    let stops: Vec<f32> = std::env::args()
        .nth(2)
        .map(|s| s.split(',').filter_map(|v| v.parse().ok()).collect())
        .unwrap_or_else(|| vec![1.0, 2.35, 4.1, 5.0, 7.5, 9.2, 12.0, 14.5]);

    let mut t = 0.0f32;
    let mut wanted = stops.into_iter().peekable();
    let mut steps = 0u32;
    // "clean" renders the eye with the interference layer switched off, which
    // is how you debug geometry.
    let clean = std::env::args().nth(4).as_deref() == Some("clean");
    let mut dump_out = dump.as_ref().map(|_| {
        let mut s = String::with_capacity(1 << 20);
        s.push_str(&format!("{W} {H}\n"));
        s
    });
    while let Some(target) = wanted.peek().copied() {
        while t < target {
            eye.update(
                1.0 / FPS,
                &mut rng,
                Some((W as f32 * 0.62, H as f32 * 0.4)),
                W,
                H,
            );
            eye.draw(&mut frame, &mut rng);
            if !clean {
                glitch::apply(&mut frame, &mut rng, eye.chaos, t);
                glitch::crt_drift(&mut frame, t, eye.chaos);
            }
            t += 1.0 / FPS;
            steps += 1;
        }
        wanted.next();

        html.push_str(&format!(
            "<div class=cap>t={t:.2}s chaos={:.2} open={:.2}</div><pre>",
            eye.chaos, eye.open
        ));
        for row in 0..H {
            for col in 0..W {
                let cell = frame.cells[row * W + col];
                push_span(&mut html, cell);
                if let Some(s) = dump_out.as_mut() {
                    push_cell(s, cell);
                }
            }
            html.push('\n');
            if let Some(s) = dump_out.as_mut() {
                s.push('\n');
            }
        }
        html.push_str("</pre>");
        if let Some(s) = dump_out.as_mut() {
            s.push('\n');
        }
    }

    std::fs::write(&path, html).expect("write html");
    if let (Some(p), Some(s)) = (dump, dump_out) {
        std::fs::write(p, s).expect("write dump");
    }
    println!("wrote {path} ({steps} simulated frames)");
}

fn push_cell(s: &mut String, cell: Cell) {
    use std::fmt::Write;
    let fg = cell.fg.unwrap_or(Rgb(204, 204, 204));
    let bg = cell.bg.unwrap_or(Rgb(8, 8, 14));
    // Tab separated: no glyph in the eye or the glitch set is a tab, unlike '|'.
    let _ = write!(
        s,
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{}\t",
        bg.0, bg.1, bg.2, fg.0, fg.1, fg.2, cell.ch
    );
}

fn push_span(html: &mut String, cell: Cell) {
    let fg = cell.fg.unwrap_or(Rgb(200, 200, 200));
    let bg = cell.bg.unwrap_or(Rgb(8, 8, 14));
    html.push_str("<span style=\"color:");
    push_hex(html, fg);
    html.push_str(";background-color:");
    push_hex(html, bg);
    html.push_str("\">");
    // Escape the handful of chars that matter inside a span.
    match cell.ch {
        '<' => html.push_str("&lt;"),
        '>' => html.push_str("&gt;"),
        '&' => html.push_str("&amp;"),
        ' ' => html.push_str("&nbsp;"),
        c => html.push(c),
    }
    html.push_str("</span>");
}

fn push_hex(html: &mut String, c: Rgb) {
    use std::fmt::Write;
    let _ = write!(html, "#{:02x}{:02x}{:02x}", c.0, c.1, c.2);
}
