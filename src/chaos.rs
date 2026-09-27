//! The chaos slider: a master volume control for the interference.
//!
//! The eye always has its own interference driver running (bursts on top of a
//! low rest level). This is the fader in front of it: at 0 the eye is left in
//! peace, at 1 you get everything the driver asks for.
//!
//! The bar fades itself out after [`IDLE_HIDE`] seconds without a change, so
//! it stops sitting under the eye for the whole run. Touching it brings it back.

use crate::color::Rgb;
use crate::frame::{Cell, Frame};
use crate::util::{clamp, hsv, lerp};

/// How far one key press moves the level.
pub const STEP: f32 = 0.05;

/// Seconds without a change before the bar fades away.
pub const IDLE_HIDE: f32 = 3.0;
/// Seconds to fade in or out. Short enough to feel like it blinks rather than
/// lingers, long enough not to snap.
const FADE: f32 = 0.35;

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
    /// Seconds since the fader was last touched.
    idle: f32,
    /// 1 while the bar should be up, 0 while it should be gone. Eased, so it
    /// fades rather than blinking out between frames.
    alpha: f32,
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
            idle: 0.0,
            alpha: 1.0,
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
        // Reaching for the fader counts as using it, even if the press landed
        // on the value it was already at. A control that vanishes under your
        // finger while you are holding it down is not a control.
        self.touch();
    }

    /// Call whenever the fader is being used, to stop it fading out. Nudges the
    /// fade up as well, so the bar counts as back immediately rather than
    /// staying invisible for the frame before it starts fading in.
    pub fn touch(&mut self) {
        self.idle = 0.0;
        self.alpha = self.alpha.max(0.05);
    }

    /// `hover` is whether the pointer is over the bar, which also counts as
    /// using it: it should not fade out from under the cursor.
    pub fn update(&mut self, dt: f32, hover: bool) {
        let k = 1.0 - (-dt * self.glide).exp();
        self.shown = lerp(self.shown, self.level, k);
        self.pulse = (self.pulse - dt * 2.6).max(0.0);

        self.idle += dt;
        if hover {
            self.idle = 0.0;
        }
        let want: f32 = if self.idle < IDLE_HIDE { 1.0 } else { 0.0 };
        let step = dt / FADE;
        self.alpha = if want > self.alpha {
            (self.alpha + step).min(want)
        } else {
            (self.alpha - step).max(want)
        };
    }

    /// Is the bar up? Below this it is too faint to read or to aim at, so it
    /// stops being drawn and stops accepting clicks.
    pub fn visible(&self) -> bool {
        self.alpha > 0.02
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
    /// not land on the track — including when the bar has faded out, since you
    /// should not be able to set a level with a control you cannot see.
    ///
    /// The track is a ramp across its *whole* length, so the first cell is
    /// exactly 0 and the last is exactly 1. Dividing by the cell count instead
    /// would leave the right end a step short of full, and a bar you cannot
    /// push all the way over feels broken.
    pub fn level_at(&self, col: i32, row: i32, w: usize, h: usize) -> Option<f32> {
        if !self.visible() {
            return None;
        }
        let (x, y, cells) = Self::layout(w, h)?;
        let start = track_start(x);
        if row != y || col < start || col >= start + cells as i32 {
            return None;
        }
        let span = (cells - 1).max(1) as f32;
        Some(clamp((col - start) as f32 / span, 0.0, 1.0))
    }

    /// Is this cell part of the bar, whether or not the bar is currently up?
    /// Clicking where the bar used to be should bring it back rather than
    /// being taken as a poke at the eye.
    pub fn footprint(&self, col: i32, row: i32, w: usize, h: usize) -> bool {
        let Some((x, y, cells)) = Self::layout(w, h) else {
            return false;
        };
        let total = (TRACK_X as usize + LABEL.len() + 2 + READOUT + cells) as i32;
        row == y && col >= x && col < x + total
    }

    /// Is the pointer anywhere over the bar, not just the track? Used to show
    /// that the bar is live, and to hold it up while the cursor is on it.
    pub fn hovering(&self, col: i32, row: i32, w: usize, h: usize) -> bool {
        self.visible() && self.footprint(col, row, w, h)
    }

    /// Paint the bar. Drawn after the glitch layer so it stays readable, and
    /// not at all once it has faded out: the eye repaints that row every frame,
    /// so the bar leaves nothing behind.
    pub fn draw(&self, f: &mut Frame, w: usize, h: usize, hover: bool) {
        if !self.visible() {
            return;
        }
        let Some((x, y, cells)) = Self::layout(w, h) else {
            return;
        };
        let cells = cells as i32;
        let fade = self.alpha;

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

        // Fading towards the background rather than towards black, so the bar
        // dissolves into the same void the eye is drawn on.
        let mut put = |i: i32, ch: char, fg: Rgb| {
            f.set(
                x + i,
                y,
                Cell {
                    ch,
                    fg: Some(Rgb::lerp(Rgb::VOID, fg, fade)),
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
