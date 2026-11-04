//! Small math helpers: interpolation, smoothstep, value noise, hashing.

#[inline]
pub fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = clamp((x - e0) / (e1 - e0), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
pub fn fract(v: f32) -> f32 {
    v - v.floor()
}

/// Hash a 2D grid point into `[0, 1)`.
#[inline]
pub fn hash21(x: i32, y: i32, seed: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA6B)
        ^ (seed as u32).wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0x00FF_FFFF) as f32 / 16_777_215.0
}

/// Hash a single integer into `[0, 1)`.
#[inline]
pub fn hash11(x: i32, seed: i32) -> f32 {
    hash21(x, x.wrapping_mul(7), seed)
}

/// Bilinear value noise, good enough for veins / fibres / grain.
pub fn value_noise(x: f32, y: f32, seed: i32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let (xf, yf) = (x - xi, y - yi);
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let (ix, iy) = (xi as i32, yi as i32);
    let a = hash21(ix, iy, seed);
    let b = hash21(ix + 1, iy, seed);
    let c = hash21(ix, iy + 1, seed);
    let d = hash21(ix + 1, iy + 1, seed);
    lerp(lerp(a, b, u), lerp(c, d, u), v)
}

/// Cheap hue -> rgb. `h` in `[0, 1)`.
pub fn hsv(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = fract(h) * 6.0;
    let i = h.floor();
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i as i32 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}
