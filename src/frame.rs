//! Screen buffer + a diffing writer, so we only emit escape sequences for the
//! cells that actually changed since the last frame.

use std::io::{self, Write};

use crossterm::cursor::MoveTo;
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::QueueableCommand;

use crate::color::Rgb;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            fg: None,
            bg: None,
        }
    }
}

impl Cell {
    /// A blank cell painted with a background colour.
    #[inline]
    pub fn solid(bg: Rgb) -> Cell {
        Cell {
            ch: ' ',
            fg: None,
            bg: Some(bg),
        }
    }
}

pub struct Frame {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<Cell>,
}

impl Frame {
    pub fn new(w: usize, h: usize) -> Frame {
        Frame {
            w,
            h,
            cells: vec![Cell::default(); w * h],
        }
    }

    /// A resize invalidates the diff history, so the whole buffer is rebuilt.
    pub fn resize(&mut self, w: usize, h: usize) {
        if self.w != w || self.h != h {
            *self = Frame::new(w, h);
        }
    }

    #[inline]
    pub fn get(&self, col: i32, row: i32) -> &Cell {
        &self.cells[row as usize * self.w + col as usize]
    }

    #[inline]
    pub fn set(&mut self, col: i32, row: i32, cell: Cell) {
        if col < 0 || row < 0 || col >= self.w as i32 || row >= self.h as i32 {
            return;
        }
        self.cells[row as usize * self.w + col as usize] = cell;
    }

    /// Write the frame, skipping cells identical to what the terminal already
    /// shows. `prev` is updated in place to become the new reference.
    pub fn present(
        &self,
        out: &mut impl Write,
        prev: &mut Vec<Cell>,
        truecolor: bool,
    ) -> io::Result<()> {
        if prev.len() != self.cells.len() {
            prev.clear();
            prev.resize(self.cells.len(), Cell::default());
        }

        // The colour state we believe the terminal is in; a cell only needs an
        // escape sequence when it differs from this.
        let mut cur_fg: Option<Rgb> = None;
        let mut cur_bg: Option<Rgb> = None;
        let mut cursor: Option<(u16, u16)> = None;

        for row in 0..self.h {
            let base = row * self.w;
            for col in 0..self.w {
                let i = base + col;
                let cell = self.cells[i];
                if cell == prev[i] {
                    continue;
                }

                if cursor != Some((col as u16, row as u16)) {
                    out.queue(MoveTo(col as u16, row as u16))?;
                }
                if cell.bg != cur_bg {
                    out.queue(SetBackgroundColor(to_ct(cell.bg, truecolor)))?;
                    cur_bg = cell.bg;
                }
                if cell.fg != cur_fg {
                    out.queue(SetForegroundColor(to_ct(cell.fg, truecolor)))?;
                    cur_fg = cell.fg;
                }
                out.queue(Print(cell.ch))?;
                cursor = Some((col as u16 + 1, row as u16));
            }
        }

        out.queue(ResetColor)?;
        out.flush()?;
        prev.copy_from_slice(&self.cells);
        Ok(())
    }
}

#[inline]
fn to_ct(c: Option<Rgb>, truecolor: bool) -> Color {
    match c {
        Some(v) => v.to_color(truecolor),
        None => Color::Reset,
    }
}
