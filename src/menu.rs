//! The small TUI menu for switching between eye states.

use crate::color::Rgb;
use crate::frame::{Cell, Frame};
use crate::state::EyeState;

const HINT: &str = " enter: select   esc: close ";

/// The little state menu. Drawn as an overlay on the eye frame, so it inherits
/// the same cells, colours and pacing as the rest of the rendering.
pub struct Menu {
    open: bool,
    selected: usize,
}

impl Menu {
    pub fn new() -> Menu {
        Menu {
            open: false,
            selected: 0,
        }
    }

    pub fn open(&mut self, current: EyeState) {
        self.open = true;
        self.selected = EyeState::ALL
            .iter()
            .position(|s| *s == current)
            .unwrap_or(0);
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Toggle with the current state as the starting selection.
    pub fn toggle(&mut self, current: EyeState) {
        if self.open {
            self.close();
        } else {
            self.open(current);
        }
    }

    /// Move the cursor, wrapping around the ends.
    pub fn select_next(&mut self) {
        let n = EyeState::ALL.len();
        self.selected = (self.selected + 1) % n;
    }

    /// Move the cursor, wrapping around the ends.
    pub fn select_prev(&mut self) {
        let n = EyeState::ALL.len();
        self.selected = if self.selected == 0 { n - 1 } else { self.selected - 1 };
    }

    /// The state the cursor is on.
    pub fn selected(&self) -> EyeState {
        EyeState::ALL[self.selected]
    }

    /// Draw the menu over the current frame, or return false when it does not
    /// fit (in which case nothing is drawn).
    pub fn draw(&self, f: &mut Frame, running: EyeState) -> bool {
        if f.h < 6 || f.w < 20 {
            return false;
        }
        let bright = Rgb(226, 232, 244);
        let dim = Rgb(96, 104, 130);
        let faint = Rgb(140, 148, 170);

        // Status row: what the eye is doing right now.
        draw_text(f, 0, &format!(" state: {} ", running.label()), faint);
        for (i, s) in EyeState::ALL.iter().enumerate() {
            let row = 1 + i;
            let fg = if i == self.selected { bright } else { dim };
            let text = if i == self.selected {
                format!("> {}", s.label())
            } else {
                format!("  {}", s.label())
            };
            draw_text(f, row, &text, fg);
        }
        draw_text(f, 5, HINT, faint);
        true
    }
}

/// Paint a centred line of text onto the frame.
fn draw_text(f: &mut Frame, row: usize, text: &str, fg: Rgb) {
    let start = ((f.w as i32 - text.len() as i32) / 2).max(0);
    for (i, ch) in text.chars().enumerate() {
        f.set(
            start + i as i32,
            row as i32,
            Cell {
                ch,
                fg: Some(fg),
                bg: Some(Rgb::VOID),
            },
        );
    }
}
