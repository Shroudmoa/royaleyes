//! Renders the chaos bar on its own, at a spread of levels and widths, so the
//! layout and the colours can be reviewed without a terminal.
//! `cargo run --example slider_preview -- out.html`

use glitch_eye::chaos::Slider;
use glitch_eye::frame::Frame;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/opencode/slider.html".to_string());
    let mut html = String::new();
    html.push_str(
        "<!doctype html><meta charset=utf-8><style>\
         body{background:#111;margin:0;padding:14px;font:12px/1 monospace}\
         pre{margin:0 0 12px;display:inline-block;letter-spacing:0}\
         .cap{color:#666;font:11px monospace;margin:0 0 4px}</style>",
    );

    // (width, height, level, hover)
    let cases: [(usize, usize, f32, bool); 14] = [
        (100, 30, 0.0, false),
        (100, 30, 0.18, false),
        (100, 30, 0.55, false),
        (100, 30, 0.8, true),
        (100, 30, 1.0, false),
        (80, 24, 0.37, false),
        (60, 20, 0.62, true),
        (46, 12, 0.5, false),
        (34, 8, 0.25, false),
        (30, 6, 0.9, false),
        (24, 4, 0.5, false),
        (23, 30, 0.5, false),
        (19, 3, 0.5, false),
        (18, 30, 0.5, false),
    ];

    for (w, h, level, hover) in cases {
        let mut f = Frame::new(w, h);
        let mut s = Slider::new(level);
        // Settle the glide so the bar is shown at the level asked for, and keep
        // touching it so it has not faded out by the time we draw.
        for _ in 0..200 {
            s.update(1.0 / 60.0, false);
            s.touch();
        }
        s.draw(&mut f, w, h, hover);

        html.push_str(&format!(
            "<div class=cap>{w}x{h} level={level:.2} hover={hover}{}</div><pre>",
            if Slider::layout(w, h).is_some() {
                ""
            } else {
                "  (no bar)"
            }
        ));
        for row in 0..h {
            for col in 0..w {
                push_span(&mut html, f.cells[row * w + col]);
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
