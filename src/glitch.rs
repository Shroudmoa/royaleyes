//! The interference layer. Everything here operates on the finished frame,
//! warping it the way a bad cable warps a video signal: channel separation,
//! band displacement, block tearing, scanline roll, and static.

use crate::color::Rgb;
use crate::frame::{Cell, Frame};
use crate::rng::Rng;

const NOISE_CHARS: [char; 30] = [
    '\\', '|', '/', '[', ']', '{', '}', '<', '>', '_', '-', '+', '=', '*', '#', '%', '@', '$', '&',
    '^', '~', '`', '1', '0', '?', ';', ':', '.', ',', '"',
];

/// Apply one frame of glitching. `chaos` is 0..1.
pub fn apply(f: &mut Frame, rng: &mut Rng, chaos: f32, t: f32) {
    let (w, h) = (f.w, f.h);
    if w < 6 || h < 3 {
        return;
    }
    let n_cells = (w * h) as f32;

    // Snapshot for every read-after-write effect.
    let src = f.cells.clone();

    channel_split(f, &src, rng, chaos, w, h);
    band_shift(f, &src, rng, chaos, w, h);
    row_tear(f, &src, rng, chaos, w, h);
    block_warp(f, &src, rng, chaos, w, h);
    scanline_roll(f, rng, chaos, t, w, h);
    static_noise(f, rng, chaos, n_cells, w, h);
}

#[inline]
fn at_row(src: &[Cell], w: usize, col: i32, row: usize) -> Cell {
    src[row * w + col.clamp(0, w as i32 - 1) as usize]
}

/// Chromatic aberration in horizontal bands: the red channel arrives late, the
/// blue channel early, so bright parts of the eye split into colour fringes.
fn channel_split(f: &mut Frame, src: &[Cell], rng: &mut Rng, chaos: f32, w: usize, h: usize) {
    let band_p = 0.08 + chaos * 0.55;
    let max_shift = (2.0 + chaos * 16.0) as i32;

    let mut row = 0usize;
    while row < h {
        let band = if rng.chance(band_p) {
            rng.i(1, 4) as usize
        } else {
            0
        };
        if band == 0 {
            row += 1;
            continue;
        }

        let shift = rng.i(1, max_shift.max(2));
        let dir = if rng.chance(0.5) { 1 } else { -1 };
        let amount = shift * dir;
        let k = rng.range(0.35, 0.55 + chaos * 0.45);

        for r in row..(row + band).min(h) {
            for c in 0..w as i32 {
                let i = r * w + c as usize;
                let base = src[i];
                let left = at_row(src, w, c + amount, r);
                let right = at_row(src, w, c - amount, r);
                f.cells[i] = split_cell(base, left, right, k);
            }
        }
        row += band;
    }
}

#[inline]
fn split_cell(base: Cell, left: Cell, right: Cell, k: f32) -> Cell {
    let mix = |s: Option<Rgb>, own: Option<Rgb>| -> Option<Rgb> {
        match (s, own) {
            (Some(a), Some(b)) => {
                let l = a.lum() as f32;
                let (br, bg, bb) = (b.0 as f32, b.1 as f32, b.2 as f32);
                Some(Rgb(
                    blend(br, l, k),
                    blend(bg, b.1 as f32, k),
                    blend(bb, l, k),
                ))
            }
            (Some(a), None) => Some(a.scale(0.9)),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    };
    Cell {
        ch: base.ch,
        fg: mix(base.fg, left.fg),
        bg: mix(base.bg, right.bg),
    }
}

#[inline]
fn blend(own: f32, from_lum: f32, k: f32) -> u8 {
    let v = own * (1.0 - k) + from_lum * k;
    if v <= 0.0 {
        0
    } else if v >= 255.0 {
        255
    } else {
        v as u8
    }
}

/// Shear whole rows sideways, leaving hard vertical cuts.
fn band_shift(f: &mut Frame, src: &[Cell], rng: &mut Rng, chaos: f32, w: usize, h: usize) {
    let max_shift = (3.0 + chaos * 20.0) as i32;
    let bands = 1 + (chaos * 7.0) as i32;

    for _ in 0..bands {
        let start = rng.i(0, h as i32) as usize;
        let band = rng.i(1, 4) as usize;
        let shift = rng.i(1, max_shift.max(2)) * if rng.chance(0.5) { 1 } else { -1 };
        for r in start..(start + band).min(h) {
            for c in 0..w as i32 {
                f.cells[r * w + c as usize] = at_row(src, w, c + shift, r);
            }
        }
    }
}

/// Copy rows over from somewhere else in the frame: the picture tears.
fn row_tear(f: &mut Frame, src: &[Cell], rng: &mut Rng, chaos: f32, w: usize, h: usize) {
    let tears = 1 + (chaos * 5.0) as i32;
    let span = ((h / 5) as i32).max(1);
    for _ in 0..tears {
        let r = rng.i(0, h as i32);
        let off = rng.i(-span, span + 1);
        let from = (r + off).clamp(0, h as i32 - 1);
        if from == r {
            continue;
        }
        let (r, from) = (r as usize, from as usize);
        let width = rng.i(w as i32 / 4, w as i32).max(1);
        let start = rng.i(0, w as i32 - width);
        for c in start..(start + width) {
            f.cells[r * w + c as usize] = src[from * w + c as usize];
        }
    }
}

/// Rectangular blocks of the picture dragged in from elsewhere.
fn block_warp(f: &mut Frame, src: &[Cell], rng: &mut Rng, chaos: f32, w: usize, h: usize) {
    let blocks = (chaos * 7.0) as i32;
    for _ in 0..blocks {
        let bw = rng.i(2, (w as i32 / 3).max(3)) as usize;
        let bh = rng.i(1, 4) as usize;
        let dx = rng.i(0, w as i32 - bw as i32) as usize;
        let dy = rng.i(0, h as i32 - bh as i32) as usize;
        let sx = rng.i(0, w as i32 - bw as i32) as usize;
        let sy = rng.i(0, h as i32 - bh as i32) as usize;
        for y in 0..bh {
            for x in 0..bw {
                f.cells[(dy + y) * w + dx + x] = src[(sy + y) * w + sx + x];
            }
        }
    }
}

/// A hot band rolling down the screen, like a CRT losing sync.
fn scanline_roll(f: &mut Frame, rng: &mut Rng, chaos: f32, t: f32, w: usize, h: usize) {
    if !rng.chance(0.10 + chaos * 0.55) {
        return;
    }
    let thick = rng.i(1, 3) as usize;
    let speed = 3.0 + chaos * 26.0;
    let pos = ((t * speed) % ((h + thick * 3) as f32)) as i32 - thick as i32;
    let start = pos.clamp(0, h as i32) as usize;

    for r in start..(start + thick).min(h) {
        for c in 0..w {
            let cell = &mut f.cells[r * w + c];
            cell.bg = cell.bg.map(|v| v.scale(1.0 + 0.9 * chaos));
            cell.fg = cell.fg.map(|v| v.scale(1.0 + 0.9 * chaos));
        }
    }
}

/// Grain, and the occasional shredded row of pure static.
fn static_noise(f: &mut Frame, rng: &mut Rng, chaos: f32, n_cells: f32, w: usize, h: usize) {
    // A permanent dusting of single-cell corruption: this is the "always
    // glitching" baseline.
    let n = (n_cells * 0.0035 * (0.35 + chaos * 1.4)) as i32;
    for _ in 0..n {
        let c = rng.i(0, w as i32);
        let r = rng.i(0, h as i32);
        let bright = rng.range(0.25, 1.0);
        f.cells[r as usize * w + c as usize] = Cell {
            ch: *rng.pick(&NOISE_CHARS),
            fg: Some(Rgb(
                (120.0 * bright) as u8,
                (255.0 * bright) as u8,
                (210.0 * bright) as u8,
            )),
            bg: if rng.chance(0.35) {
                Some(Rgb(
                    (8.0 * bright) as u8,
                    (10.0 * bright) as u8,
                    (16.0 * bright) as u8,
                ))
            } else {
                None
            },
        };
    }

    // Signal loss: a couple of rows replaced wholesale by static.
    if rng.chance(chaos * chaos * 0.5) {
        let rows = rng.i(1, 3);
        for _ in 0..rows {
            let r = rng.i(0, h as i32) as usize;
            let dense = rng.range(0.25, 0.95);
            for c in 0..w {
                if !rng.chance(dense) {
                    continue;
                }
                let bright = rng.range(0.3, 1.0);
                f.cells[r * w + c] = Cell {
                    ch: *rng.pick(&NOISE_CHARS),
                    fg: Some(Rgb(
                        (255.0 * bright) as u8,
                        (255.0 * bright) as u8,
                        (255.0 * bright) as u8,
                    )),
                    bg: Some(Rgb(
                        (14.0 * bright) as u8,
                        (16.0 * bright) as u8,
                        (24.0 * bright) as u8,
                    )),
                };
            }
        }
    }
}

/// Colour the whole frame like a badly tuned monitor: a stepped brightness
/// ripple, fixed CRT banding, and an occasional partial inversion.
///
/// The ripple is deliberately quantised: the renderer only rewrites cells that
/// changed, so a smoothly varying brightness would repaint the whole screen
/// every frame. Quantising it means most frames touch a few rows at most.
pub fn crt_drift(f: &mut Frame, t: f32, chaos: f32) {
    if chaos < 0.30 {
        return;
    }
    let w = f.w.max(1);
    let step = (t * 7.0).sin();
    let ripple = 1.0 + 0.07 * (step * 4.0).round() / 4.0;
    let invert = (chaos - 0.30) * 0.30;

    for (i, cell) in f.cells.iter_mut().enumerate() {
        let row = i / w;
        let col = i % w;
        let band = 1.0 + 0.05 * (row as f32 * 0.4).sin();
        cell.bg = cell.bg.map(|v| v.scale(ripple * band));
        cell.fg = cell.fg.map(|v| v.scale(ripple * band));
        if invert > 0.0 {
            // Blocky diagonal patches, like a decoder losing its colour bars.
            // Stepped in time so the patches snap a few times a second rather
            // than sliding smoothly, which also keeps the repaint small.
            let ts = (t * 8.0).floor() / 8.0;
            let wobble = ((row as f32 * 0.8) + (col as f32 / 6.0) * 0.5 + ts * 25.0).sin();
            if wobble * 0.5 + 0.5 < invert {
                cell.bg = cell.bg.map(|v| invert_rgb(v, 0.85));
                cell.fg = cell.fg.map(|v| invert_rgb(v, 0.85));
            }
        }
    }
}

/// Contrast inversion rather than a plain `255 - v`: a plain flip would turn
/// the near-black background into a blinding white field.
#[inline]
fn invert_rgb(v: Rgb, k: f32) -> Rgb {
    let f = |c: u8| {
        let x = c as f32;
        (x + (255.0 - 2.0 * x) * k).clamp(0.0, 255.0) as u8
    };
    Rgb(f(v.0), f(v.1), f(v.2))
}
