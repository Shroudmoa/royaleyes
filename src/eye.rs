//! The eye itself: lid geometry, sclera, iris, pupil, specular hits.
//!
//! Coordinate system: +x right, +y down, measured in "square units" where one
//! unit is the width of a character cell (cells are about twice as tall as
//! they are wide, hence the 0.5 factor on y). The eye is defined in a
//! normalised frame where `xr = rx / half_w` and `yr = ry / half_h`.

use crate::color::Rgb;
use crate::frame::{Cell, Frame};
use crate::rng::Rng;
use crate::util::{clamp, fract, hash11, hash21, hsv, lerp, smoothstep, value_noise};

/// Texture ramp, sparse -> dense. Background colour carries the tone, these
/// glyphs carve the detail into it.
const RAMP: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

/// Eye height / width ratio in square units: wider than tall like a real eye,
/// but not so flat that the iris has nowhere to live.
const HALF_H_RATIO: f32 = 0.13;

const VOID: Rgb = Rgb::VOID;
const SCLERA_HI: Rgb = Rgb(234, 237, 244);
const SCLERA_LO: Rgb = Rgb(78, 68, 92);
const VEIN: Rgb = Rgb(176, 70, 86);
const LASH_BG: Rgb = Rgb(11, 10, 18);
const LASH_FG: Rgb = Rgb(74, 64, 104);
const PUPIL_BG: Rgb = Rgb(3, 3, 7);

const BLINK_LEN: f32 = 0.17;
const TAU: f32 = std::f32::consts::TAU;
const IRIS_R: f32 = 0.35; // iris radius as a fraction of half_w
const LASH_W: f32 = 1.15; // lash thickness, in square units (see `near_lid`)
/// Smallest lid-to-lid gap the eye will ever have, in square units. Cell
/// centres are 0.5 apart vertically, so anything under that and the corners of
/// the almond drop out of the frame entirely.
const MIN_GAP: f32 = 0.55;

/// The interference driver's resting level, before `gain` is applied. Low
/// enough to be calm, high enough that the eye never looks switched off.
const REST: f32 = 0.18;

pub struct Eye {
    pub t: f32,
    /// Eyelid aperture: 1 = wide open, 0 = squeezed shut.
    pub open: f32,
    blink_t: f32,
    blink_wait: f32,
    /// Socket rotation, radians.
    pub tilt: f32,
    tilt_target: f32,
    /// Iris centre in the eye's own normalised frame.
    gaze: [f32; 2],
    gaze_target: [f32; 2],
    dart_wait: f32,
    /// Pupil radius as a fraction of the iris radius.
    pupil: f32,
    iris_hue: f32,
    /// What the glitch layer and the pupil actually see: `drive * gain`.
    /// 0 = stable, 1 = full meltdown.
    pub chaos: f32,
    /// The interference driver, before `gain`: it idles at [`REST`] and spikes
    /// on its own. `gain` is the fader in front of it.
    drive: f32,
    /// Master volume, 0.0..=1.0. 0 leaves the eye completely alone, even
    /// though the driver underneath is still running.
    pub gain: f32,
    burst_wait: f32,
    /// Bumped every frame so hash-based noise animates.
    grain: i32,
    following: bool,
}

impl Eye {
    /// `gain` is the starting position of the chaos fader, 0.0..=1.0.
    pub fn new(gain: f32) -> Eye {
        let gain = gain.clamp(0.0, 1.0);
        Eye {
            t: 0.0,
            open: 1.0,
            blink_t: 0.0,
            blink_wait: 1.2,
            tilt: 0.0,
            tilt_target: 0.0,
            gaze: [0.0, 0.0],
            gaze_target: [0.0, 0.0],
            dart_wait: 0.0,
            pupil: 0.36,
            iris_hue: 0.32,
            chaos: REST * gain,
            drive: REST,
            gain,
            burst_wait: 0.8,
            grain: 0,
            following: false,
        }
    }

    /// Force a blink.
    pub fn blink(&mut self) {
        if self.blink_t <= 0.0 {
            self.blink_t = BLINK_LEN;
        }
    }

    /// Kick off a burst of interference. Manual bursts ignore `gain` in one
    /// respect: the eye still flinches when you poke it, so a click is never
    /// completely inert.
    pub fn burst(&mut self, rng: &mut Rng) {
        // A poke is scaled by the fader too, but never vanishes entirely.
        let amt = 0.35 + 0.65 * self.gain;
        self.drive = (self.drive + amt).min(1.0);
        self.burst_wait = rng.range(0.9, 2.6);
        self.tilt_target = rng.sym() * 0.30;
        self.iris_hue = fract(self.iris_hue + rng.sym() * 0.10);
        self.blink();
    }

    pub fn update(
        &mut self,
        dt: f32,
        rng: &mut Rng,
        mouse: Option<(f32, f32)>,
        w: usize,
        h: usize,
    ) {
        self.t += dt;
        self.grain = self.grain.wrapping_add(1);

        // --- blinking -----------------------------------------------------
        self.blink_wait -= dt;
        if self.blink_t <= 0.0 && self.blink_wait <= 0.0 {
            self.blink_wait = rng.range(1.4, 5.2);
            self.blink();
        }
        if self.blink_t > 0.0 {
            self.blink_t -= dt;
            let p = clamp(1.0 - self.blink_t / BLINK_LEN, 0.0, 1.0);
            // snap shut, ease back open
            let v = if p < 0.38 {
                ease(p / 0.38)
            } else {
                1.0 - ease((p - 0.38) / 0.62)
            };
            self.open = v;
        } else {
            self.open = lerp(self.open, 1.0, 1.0 - (-dt * 9.0).exp());
        }

        // --- what are we looking at ----------------------------------------
        const GX: f32 = 0.60;
        const GY: f32 = 0.55;
        match mouse {
            Some((mx, my)) if w > 0 && h > 0 => {
                let nx = (mx / w as f32) * 2.0 - 1.0;
                let ny = (my / h as f32) * 2.0 - 1.0;
                self.gaze_target = [clamp(nx * GX * 1.7, -GX, GX), clamp(ny * GY * 1.7, -GY, GY)];
                self.following = true;
            }
            _ => {
                self.following = false;
                self.dart_wait -= dt;
                if self.dart_wait <= 0.0 {
                    self.dart_wait = rng.range(0.3, 1.8);
                    self.gaze_target = [rng.range(-GX, GX), rng.range(-GY * 0.85, GY)];
                }
            }
        }
        self.gaze_target[0] = clamp(self.gaze_target[0], -GX, GX);
        self.gaze_target[1] = clamp(self.gaze_target[1], -GY, GY);

        // Smooth pursuit, plus a fine tremor so it never sits perfectly still.
        let rate = if self.following { 11.0 } else { 16.0 };
        let k = 1.0 - (-dt * rate).exp();
        let prev = self.gaze;
        self.gaze[0] = lerp(self.gaze[0], self.gaze_target[0], k);
        self.gaze[1] = lerp(self.gaze[1], self.gaze_target[1], k);
        let speed =
            ((self.gaze[0] - prev[0]).abs() + (self.gaze[1] - prev[1]).abs()) / dt.max(1e-4);
        let tremor = 0.020 * (1.0 - smoothstep(0.0, 2.5, speed));
        self.gaze[0] = clamp(
            self.gaze[0] + (self.t * 2.3).sin() * tremor * dt * 12.0,
            -GX * 1.05,
            GX * 1.05,
        );
        self.gaze[1] = clamp(
            self.gaze[1] + (self.t * 1.7 + 1.3).sin() * tremor * dt * 9.0,
            -GY * 1.05,
            GY * 1.05,
        );

        // --- interference --------------------------------------------------
        // The driver always runs on its own; `gain` is only the fader on the
        // output, so turning the chaos down does not stop the eye from having
        // an opinion, it just stops that opinion reaching the screen.
        self.burst_wait -= dt;
        if self.burst_wait <= 0.0 {
            self.burst_wait = rng.range(0.9, 2.8);
            // A burst has to be able to carry the driver all the way to 1.0,
            // otherwise pushing the fader to the top buys nothing and the bar
            // is lying about the top of its range.
            self.drive = (self.drive + rng.range(0.30, 0.95)).min(1.0);
        }
        self.drive = (self.drive - dt * 0.55).max(REST);
        self.gain = self.gain.clamp(0.0, 1.0);
        self.chaos = (self.drive * self.gain).clamp(0.0, 1.0);

        // --- pupil, socket, iris --------------------------------------------
        // The pupil blows wide with panic, contracts when it locks on.
        let focus = smoothstep(0.6, 4.0, speed);
        let want = clamp(
            0.36 + 0.10 * (self.t * 0.8).sin() + 0.30 * self.chaos - 0.10 * focus,
            0.22,
            0.66,
        );
        self.pupil = lerp(self.pupil, want, 1.0 - (-dt * 7.0).exp());

        if self.burst_wait > 2.9 {
            self.tilt_target = rng.sym() * 0.22;
        }
        self.tilt = lerp(self.tilt, self.tilt_target, 1.0 - (-dt * 2.5).exp());
        self.iris_hue = fract(self.iris_hue + dt * 0.012 * (1.0 + self.chaos));
    }

    /// Paint the void and the eye into `f`.
    pub fn draw(&self, f: &mut Frame, rng: &mut Rng) {
        let (w, h) = (f.w, f.h);
        if w < 8 || h < 4 {
            return;
        }
        f.cells.fill(Cell::solid(VOID));

        let (fw, fh) = (w as f32, h as f32);
        // Fit to the smaller of "62% of the width" and "half the height".
        let half_w = (0.31 * fw).min(fh * 0.98).max(2.0);
        let half_h = half_w * HALF_H_RATIO;
        let (cx, cy) = (fw * 0.5, fh * 0.5);

        let ir = half_w * IRIS_R;
        let pr = ir * self.pupil;
        let (sn, cs) = self.tilt.sin_cos();
        let ix = self.gaze[0] * half_w;
        let iy = self.gaze[1] * half_h;

        // A blink squeezes the lids together but never fully collapses them: a
        // shut eye is a lash line about two cells thick, not a gap in the void.
        let s_up = 0.13 + 0.87 * (self.open * 1.18).min(1.0);
        let s_dn = 0.13 + 0.87 * self.open.powf(1.25);
        let (grain, hue) = (self.grain, self.iris_hue);

        for row in 0..h {
            let y = (row as f32 + 0.5 - cy) * 0.5;
            for col in 0..w {
                let x = col as f32 + 0.5 - cx;
                let rx = x * cs + y * sn;
                let ry = -x * sn + y * cs;
                let xr = rx / half_w;
                let ax = xr.abs();
                if ax >= 1.0 {
                    continue;
                }
                let yr = ry / half_h;
                // Note +y is *down*: the upper lid sits at negative yr.
                let (mut up, mut dn) = (lid_up(xr, s_up), lid_dn(xr, s_dn));
                // The almond pinches to nothing at the corners, so once the gap
                // drops below the cell pitch those cells vanish and the eye
                // loses a few columns at each end. Hold the aperture open by
                // enough for one cell to land, which is invisible in the middle
                // and keeps the tips of a shut eye full width.
                if (up + dn) * half_h < MIN_GAP {
                    let k = (MIN_GAP / half_h) / (up + dn).max(1e-4);
                    up *= k;
                    dn *= k;
                }
                if yr < -up || yr > dn {
                    continue;
                }

                // Distance to the nearest lid, in square units. Both lids are
                // guaranteed to bracket the cell by the test above.
                let d_up = (yr + up) * half_h;
                let d_dn = (dn - yr) * half_h;
                let d_lid = d_up.min(d_dn);

                // A vertical distance badly overestimates the gap when the lid
                // is steep, which would smear the lash line across the corners.
                // Divide by the slope to approximate the perpendicular distance.
                let near_lid = d_lid < 3.0;
                let (d_perp, upper, slope) = if near_lid {
                    let upper = d_up <= d_dn;
                    let slope = lid_slope(xr, upper, s_up, s_dn);
                    (d_lid * (1.0 + slope * slope).sqrt().recip(), upper, slope)
                } else {
                    (d_lid, false, 0.0)
                };

                let dx = rx - ix;
                let dy = ry - iy;
                let d = (dx * dx + dy * dy).sqrt();

                let cell = if near_lid && d_perp < LASH_W {
                    // The lash line is drawn over everything at the boundary.
                    // The upper lid is drawn at yr = -up, so its screen slope is
                    // the negative of the aperture slope.
                    let ch = if (if upper { -slope } else { slope }) > 0.5 {
                        '\\'
                    } else if (if upper { -slope } else { slope }) < -0.5 {
                        '/'
                    } else if upper {
                        '_'
                    } else {
                        '\''
                    };
                    Cell {
                        ch,
                        fg: Some(LASH_FG),
                        bg: Some(LASH_BG),
                    }
                } else if d < ir {
                    self.iris_cell(d, ir, pr, dx, dy, col as i32, row as i32, grain, hue, rng)
                } else {
                    self.sclera_cell(d, ir, d_perp, d_up, ax, col as i32, row as i32, grain)
                };
                f.set(col as i32, row as i32, cell);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn iris_cell(
        &self,
        d: f32,
        ir: f32,
        pr: f32,
        dx: f32,
        dy: f32,
        col: i32,
        row: i32,
        grain: i32,
        hue: f32,
        rng: &mut Rng,
    ) -> Cell {
        let t = d / ir;

        // --- pupil --------------------------------------------------------
        if t < pr / ir {
            let spark = hash21(col, row, grain) > 0.97;
            return Cell {
                ch: if spark { '.' } else { ' ' },
                fg: if spark { Some(Rgb(44, 48, 76)) } else { None },
                bg: Some(PUPIL_BG),
            };
        }

        // --- iris fibres ---------------------------------------------------
        let turn = dy.atan2(dx) / TAU;
        let sect = (turn * 30.0).floor();
        let phase = hash11(sect as i32, 77) * TAU;
        let fib = (turn * 30.0 * TAU + phase).sin();
        let fib2 = (turn * 61.0 * TAU + phase * 1.7).sin();
        // Fibres crowd at the pupil and fade into the limbal ring.
        let env = smoothstep(0.20, 0.55, t) * (1.0 - 0.55 * smoothstep(0.80, 1.0, t));

        let mut v = 0.34 + 0.56 * t.powf(0.80);
        v *= 1.0 - 0.45 * smoothstep(0.90, 1.0, t); // dark limbal ring
        v *= 0.46 + 0.54 * smoothstep(pr / ir, pr / ir + 0.22, t); // pupil shadow
        v += 0.20 * (fib * 0.72 + fib2 * 0.28) * env;
        v += 0.05 * (t * 22.0).sin(); // concentric ripples
        v += (hash21(col, row, grain) - 0.5) * 0.075; // dither
        v = clamp(v, 0.0, 1.0);

        // --- colour ---------------------------------------------------------
        let sat = clamp(0.50 + 0.40 * (1.0 - t) + 0.12 * self.chaos, 0.0, 1.0);
        let (cr, cg, cb) = hsv(hue, sat, 1.0);
        let bright = 0.16 + 0.84 * v;
        let raw = Rgb(
            (cr * bright * 255.0) as u8,
            (cg * bright * 255.0) as u8,
            (cb * bright * 255.0) as u8,
        );
        // Cool ambient bounce in the darkest parts.
        let mut color = Rgb::lerp(Rgb(16, 20, 34), raw, 0.25 + 0.75 * v);

        // --- specular hits ---------------------------------------------------
        let mut ch = RAMP[(v * 9.99) as usize];
        let twinkle = rng.range(0.84, 1.0);
        let hx = dx + ir * 0.40;
        let hy = dy + ir * 0.46;
        let hd = (hx * hx + hy * hy).sqrt() / (ir * 0.30);
        if hd < 1.0 {
            let g = (1.0 - hd).powf(0.55) * twinkle;
            color = Rgb::lerp(color, Rgb(255, 255, 255), g * 0.96);
            if g > 0.55 {
                ch = '@';
            } else if g > 0.28 {
                ch = '#';
            }
        }
        // Dimmer bounce light on the far side of the iris.
        let hx2 = dx - ir * 0.34;
        let hy2 = dy - ir * 0.38;
        let hd2 = (hx2 * hx2 + hy2 * hy2).sqrt() / (ir * 0.20);
        if hd2 < 1.0 {
            let g = (1.0 - hd2).powf(0.8) * 0.5;
            color = Rgb::lerp(color, Rgb(198, 222, 255), g);
            if g > 0.3 && ch == ' ' {
                ch = '.';
            }
        }

        Cell {
            ch,
            fg: Some(color.scale(0.34)),
            bg: Some(color),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn sclera_cell(
        &self,
        d: f32,
        ir: f32,
        d_lid: f32,
        d_up: f32,
        ax: f32,
        col: i32,
        row: i32,
        grain: i32,
    ) -> Cell {
        // --- shading ---------------------------------------------------------
        let edge = smoothstep(0.0, 1.4, d_lid); // darkens towards the lids
        let corner = smoothstep(0.0, 0.32, 1.0 - ax); // darkens towards the corners
        let mut s = (0.26 + 0.74 * edge) * (0.42 + 0.58 * corner);
        s *= 0.60 + 0.40 * smoothstep(0.0, 2.4, d_up); // shadow cast by the upper lid
        s *= 1.0 - 0.32 * (1.0 - smoothstep(ir, ir * 1.75, d)); // iris occlusion
        s = clamp(s, 0.0, 1.0);

        let mut color = Rgb::lerp(SCLERA_LO, SCLERA_HI, s);
        let mut ch = ' ';
        let mut fg = None;

        // --- veins -----------------------------------------------------------
        let warp = value_noise(ax * 9.0, col as f32 * 0.35, 5) * 1.4;
        let n = value_noise(col as f32 * 0.22 + warp, row as f32 * 0.5, 9);
        let ridge = 1.0 - smoothstep(0.0, 0.030, (n - 0.5).abs());
        let vein = ridge * (0.20 + 0.80 * (1.0 - s)) * smoothstep(0.35, 0.9, 1.0 - corner);
        if vein > 0.30 {
            color = Rgb::lerp(color, VEIN, vein * 0.8);
            ch = if hash21(col, row, grain) > 0.5 {
                '\\'
            } else {
                '/'
            };
            fg = Some(Rgb(96, 30, 42));
        } else {
            // Stipple that thickens as the white falls off, so the whole of the
            // eye is visibly made of characters and not just a flat fill.
            let g = hash21(col, row, grain);
            let p = 0.05 + 0.40 * (1.0 - s).powf(1.4);
            if g < p {
                ch = match (g / p * 4.0) as usize {
                    0 => '.',
                    1 => '.',
                    2 => ':',
                    _ => '-',
                };
                fg = Some(color.scale(0.45));
            }
        }

        Cell {
            ch,
            fg,
            bg: Some(color),
        }
    }
}

#[inline]
fn ease(t: f32) -> f32 {
    let t = clamp(t, 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Upper lid aperture at `xr` (the part above the centre line, so it is
/// reached at negative `yr`).
#[inline]
fn lid_up(xr: f32, s: f32) -> f32 {
    let ax = xr.abs();
    if ax >= 1.0 {
        return 0.0;
    }
    let k = 1.0 - ax * ax;
    k.sqrt() * (0.98 + 0.30 * xr) * s
}

/// Lower lid aperture at `xr`.
#[inline]
fn lid_dn(xr: f32, s: f32) -> f32 {
    let ax = xr.abs();
    if ax >= 1.0 {
        return 0.0;
    }
    let k = 1.0 - ax * ax;
    k.powf(0.62) * (0.98 - 0.16 * xr) * s
}

/// Local slope of a lid, so the lash glyphs can follow its curve.
#[inline]
fn lid_slope(xr: f32, upper: bool, s_up: f32, s_dn: f32) -> f32 {
    let e = 0.08;
    let (a, b) = if upper {
        (lid_up(xr + e, s_up), lid_up(xr - e, s_up))
    } else {
        (lid_dn(xr + e, s_dn), lid_dn(xr - e, s_dn))
    };
    (a - b) / (2.0 * e)
}
