//! Colour type plus truecolor / xterm-256 output conversion.

use crossterm::style::Color;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const VOID: Rgb = Rgb(4, 4, 9);

    #[inline]
    pub fn scale(self, k: f32) -> Rgb {
        let f = |v: u8| clamp_u8(v as f32 * k);
        Rgb(f(self.0), f(self.1), f(self.2))
    }

    #[inline]
    pub fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
        let f = |x: u8, y: u8| clamp_u8(lerp_u8(x, y, t));
        Rgb(f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
    }

    /// Brightest channel, used to flatten a cell to a grey for RGB-splitting.
    #[inline]
    pub fn lum(self) -> u8 {
        self.0.max(self.1).max(self.2)
    }

    /// Emit as a crossterm colour: 24-bit when the terminal supports it,
    /// otherwise the closest xterm-256 index.
    pub fn to_color(self, truecolor: bool) -> Color {
        if truecolor {
            Color::Rgb {
                r: self.0,
                g: self.1,
                b: self.2,
            }
        } else {
            Color::AnsiValue(self.to_xterm256())
        }
    }

    pub fn to_xterm256(self) -> u8 {
        let (r, g, b) = (self.0 as i32, self.1 as i32, self.2 as i32);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);

        // Greys go to the dedicated 24-step ramp: smoother than the colour cube.
        if max - min <= 10 {
            let avg = (r + g + b) / 3;
            if avg < 8 {
                return 16;
            }
            return (232 + ((avg - 8) * 24 + 127) / 247).clamp(232, 255) as u8;
        }

        let q = |v: i32| ((v * 5 + 127) / 255).clamp(0, 5);
        (16 + 36 * q(r) + 6 * q(g) + q(b)) as u8
    }
}

#[inline]
fn clamp_u8(v: f32) -> u8 {
    if v <= 0.0 {
        0
    } else if v >= 255.0 {
        255
    } else {
        v as u8
    }
}

#[inline]
fn lerp_u8(a: u8, b: u8, t: f32) -> f32 {
    a as f32 + (b as f32 - a as f32) * t
}
