//! The chaos slider: a master volume control for the interference.
//!
//! The eye always has its own interference driver running (bursts on top of a
//! low rest level). This is the fader in front of it: at 0 the eye is left in
//! peace, at 1 you get everything the driver asks for.

use crate::color::Rgb;
use crate::frame::{Cell, Frame};
use crate::util::{clamp, hsv, lerp};

/// How far one key press moves the level.
pub const STEP: f32 = 0.05;

/// Offset of the first track cell from the left edge of the bar. The cells
/// before it are `' '`, the label, `' '` and `'['`.
const TRACK_X: i32 = 3;
const LABEL: &str = "CHAOS";
/// Longest the track gets; it shrinks on narrow terminals.
const TRACK_MAX: usize = 22;
/// The readout, e.g. `" 1.00"`.
const READOUT: usize = 5;
/// Below this the track would be too stubby to aim at.
const MIN_TRACK: usize = 4;

/// Block glyphs: a solid fill, a dim track, and eighths for the exact position.
/// The eighths run empty -> nearly full, so they index directly on
/// `round(fraction * 8)`.
const FULL: char = '█';
const EMPTY: char = '░';
const EIGHTHS: [char; 8] = ['░', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];

pub struct Slider {
    level: f32,
    /// Eased copy of `level`, so the bar glides instead of jumping.
    shown: f32,
    /// How fast `shown` catches up. A big jump is quicker to chase than a nudge.
    glide: f32,
    /// 1 right after a change, decaying to 0. Brightens the bar so a keypress
    /// is acknowledged even when the change is only one step.
    pulse: f32,
}

impl Slider {
    /// `level` is clamped to `0.0..=1.0`: peaceful to full meltdown.
    pub fn new(level: f32) -> Slider {
        let level = clamp(level, 0.0, 1.0);
        Slider {
            level,
            shown: level,
            glide: 14.0,
            pulse: 0.0,
        }
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    /// Jump straight to a level. The bar still glides there.
    pub fn set(&mut self, level: f32) {
        self.set_inner(level, 3.0);
    }

    /// Move by `n` steps of [`STEP`].
    pub fn adjust(&mut self, n: f32) {
        self.set_inner(self.level + n * STEP, 14.0);
    }

    fn set_inner(&mut self, level: f32, glide: f32) {
        let level = clamp(level, 0.0, 1.0);
        if level != self.level {
            self.pulse = 1.0;
        }
        self.level = level;
        self.glide = glide;
    }

    pub fn update(&mut self, dt: f32) {
        let k = 1.0 - (-dt * self.glide).exp();
        self.shown = lerp(self.shown, self.level, k);
        self.pulse = (self.pulse - dt * 2.6).max(0.0);
    }

    /// Where the bar sits for a given terminal size, as `(col, row, cells)`.
    /// `None` if the terminal is too small to hold one.
    pub fn layout(w: usize, h: usize) -> Option<(i32, i32, usize)> {
        // h must have the bar's own row plus the hint line above it.
        if h < 3 {
            return None;
        }
        // leading space, label, space, bracket, track, bracket, readout
        let fixed = TRACK_X as usize + LABEL.len() + 2 + READOUT;
        let track = w.checked_sub(fixed).map(|room| TRACK_MAX.min(room))?;
        if track < MIN_TRACK {
            return None;
        }
        let total = fixed + track;
        let col = ((w as i32 - total as i32) / 2).max(0);
        // Bottom row; the hint line sits just above it.
        Some((col, h as i32 - 1, track))
    }

    /// The level a click at `(col, row)` would set, or `None` if the click did
    /// not land on the track.
    ///
    /// The track is a ramp across its *whole* length, so the first cell is
    /// exactly 0 and the last is exactly 1. Dividing by the cell count instead
    /// would leave the right end a step short of full, and a bar you cannot
    /// push all the way over feels broken.
    pub fn level_at(&self, col: i32, row: i32, w: usize, h: usize) -> Option<f32> {
        let (x, y, cells) = Self::layout(w, h)?;
        let start = track_start(x);
        if row != y || col < start || col >= start + cells as i32 {
            return None;
        }
        let span = (cells - 1).max(1) as f32;
        Some(clamp((col - start) as f32 / span, 0.0, 1.0))
    }

    /// Is the pointer anywhere over the bar, not just the track? Used to show
    /// that the bar is live.
    pub fn hovering(&self, col: i32, row: i32, w: usize, h: usize) -> bool {
        let Some((x, y, cells)) = Self::layout(w, h) else {
            return false;
        };
        let total = (TRACK_X as usize + LABEL.len() + 2 + READOUT + cells) as i32;
        row == y && col >= x && col < x + total
    }

    /// Paint the bar. Drawn after the glitch layer so it stays readable.
    pub fn draw(&self, f: &mut Frame, w: usize, h: usize, hover: bool) {
        let Some((x, y, cells)) = Self::layout(w, h) else {
            return;
        };
        let cells = cells as i32;

        // Green when peaceful, red when the eye is coming apart: the colour is
        // the level, so the bar explains itself without a legend.
        let (r, g, b) = hsv(lerp(0.36, 0.99, self.shown), 0.72, 1.0);
        let body = Rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8);
        let hot = self.pulse.max(if hover { 0.35 } else { 0.0 });
        let fill = Rgb::lerp(body.scale(0.62), body, hot);
        let empty = body.scale(0.20);
        let bracket = body.scale(0.45);
        let label = Rgb::lerp(Rgb(96, 104, 130), Rgb(232, 236, 255), hot);
        let readout = Rgb::lerp(Rgb(70, 76, 100), Rgb(236, 240, 255), hot.max(0.55));

        let mut put = |i: i32, ch: char, fg: Rgb| {
            f.set(
                x + i,
                y,
                Cell {
                    ch,
                    fg: Some(fg),
                    bg: Some(Rgb::VOID),
                },
            );
        };

        put(0, if hover { '>' } else { ' ' }, label);
        for (i, c) in LABEL.chars().enumerate() {
            put(1 + i as i32, c, label);
        }
        put(1 + LABEL.len() as i32, ' ', label);
        put(TRACK_X - 1 + LABEL.len() as i32, '[', bracket);

        // The track, filled to `shown`. The cell the level lands in is drawn as
        // a partial block, so the position stays readable between cells.
        let track = TRACK_X + LABEL.len() as i32;
        let filled = self.shown * cells as f32;
        let whole = filled.floor() as i32;
        let frac = filled - whole as f32;
        for i in 0..cells {
            let ch = if i < whole {
                FULL
            } else if i > whole {
                EMPTY
            } else {
                // `frac` is how much of one cell is filled, so pick the
                // closest eighth-block.
                EIGHTHS[((frac * 8.0).round() as usize).min(7)]
            };
            put(track + i, ch, if i <= whole { fill } else { empty });
        }
        put(track + cells, ']', bracket);

        for (i, c) in format!(" {:.2}", self.shown).chars().enumerate() {
            put(track + cells + 1 + i as i32, c, readout);
        }
    }
}

/// Absolute column of the first track cell.
fn track_start(x: i32) -> i32 {
    x + TRACK_X + LABEL.len() as i32
}
