# glitch-eye

A single ASCII eye that watches your cursor and slowly comes apart.

```
$ cargo run --release
```

The eye is drawn at whatever size your terminal is: 24-bit colour if
`COLORTERM` advertises it, xterm-256 otherwise. It follows the mouse, blinks
on its own, and every few seconds a burst of interference rolls through —
banded RGB splits, row tears, block warps, scanline roll, a CRT brightness
ripple.

There is a fader along the bottom, so you decide how much of that you want.
It runs from peaceful to chaos, and it is the only thing standing between you
and a completely still eye.

```
 CHAOS [████████████▏░░░░░░░░░] 0.55
```

## Controls

| Input | Effect |
| --- | --- |
| move the mouse | the eye looks at it (gives up after 2.5s and starts looking around on its own) |
| click | glitch burst — or sets the level, if you click the bar |
| scroll over the bar | nudge the chaos level |
| `g` | glitch burst |
| `space` / `b` | blink |
| `up` / `down` / `right` / `left` | chaos up / down (hold `shift` to move faster) |
| `+` / `-` | chaos up / down |
| `p` / `c` | peaceful / chaos, straight there |
| `q` / `Esc` / `Ctrl-C` | quit |

The bar is drawn *after* the interference, so it stays readable however badly
the frame is coming apart, and it turns from green at the bottom to red at the
top. Clicking a track cell sets the level to that point; the far ends reach 0.0
and 1.0 exactly.

At peaceful the eye still blinks and still follows the mouse — it just stops
coming apart. It is calmer, not dead.

The first few seconds show a fading hint just above the bar. Under 30x8 the eye
asks for a bigger terminal instead, and below about 19 columns wide there is no
room for the bar, so the fader falls back to the keys.

## Options

```
--chaos <0.0..1.0>   where the fader starts (default 0.55)
-h, --help
```

This only sets the *starting position*: the bar is there to be moved, and the
level you leave it at is not written anywhere.

## How it is put together

The eye is a hybrid: the *background colour* of each cell carries the tone and
the *glyph* carries the texture. Iris fibres, stipple, blood vessels and the
lash line are drawn as characters, but the light and dark come from the
background, which is what makes it legible at small sizes.

- `src/eye.rs` — lid geometry, sclera, iris, specular hits. The lids are
  `sqrt(1-x²)`-ish curves in a normalised frame; they squeeze together for a
  blink but never fully collapse, and a minimum-aperture floor keeps the
  corners of the almond inside the frame when they go sub-cell.
- `src/chaos.rs` — the fader: level, the bar's layout, and the click-to-set
  mapping. The track is a ramp across its whole length, so both ends are
  exactly reachable.
- `src/glitch.rs` — the interference layer, applied on top of a finished frame.
  Everything is chaos-gated, so at `chaos = 0` the eye is completely clean.
- `src/frame.rs` — the cell buffer and a diffing writer, so only cells that
  actually changed get written to the terminal each frame.
- `src/color.rs` — 24-bit to xterm-256 fallback.
- `src/rng.rs` — a small xorshift PRNG, so the animation is reproducible from
  a seed. `src/util.rs` has the value noise and easing helpers.

The eye runs its own interference driver regardless of where the fader is: it
idles at a low level and spikes on its own. The fader is a gain on the *output*
of that, not a throttle on the driver. So turning the chaos down does not stop
the eye having an opinion, it just stops that opinion reaching the screen — and
turning it back up releases whatever the driver was doing while it was quiet.

Only dependency is `crossterm`.

## Development

```
cargo test                    # structural tests: is it shaped like an eye?
cargo clippy --all-targets
```

`tests/eye_shape.rs` checks the things that are easy to break silently: the
eye stays centred and wider than it is tall, the lids taper towards the corners,
there is a bright sclera, a dark pupil and a visible iris, a blink squashes the
height without moving or narrowing the eye, the iris tracks the mouse, chaos
stays clamped, and nothing panics on a 1x1 terminal.

`tests/slider.rs` covers the fader: the level clamps at both ends, a key press
moves it exactly one step, the track maps clicks monotonically with 0 and 1
exactly reachable, the brackets around it are not part of it, it always fits
the width it is drawn into, and — the one worth having — the eye is provably
silent at zero gain, provably no louder than promised at full gain, and
provably ordered in between.

The renderer is a library, so frames can be generated without a terminal:

```
cargo run --example snapshot -- out.html                        # a spread of frames as HTML
cargo run --example snapshot -- out.html 1.0,4.1 clean           # just those two, no glitch
cargo run --example snapshot -- out.html 12.0 cells.txt          # a raw cell dump as well
cargo run --example slider_preview -- bar.html                   # the bar alone, many widths
cargo run --example gain_preview -- gain.html                    # the eye + bar, many levels
```

The dump is one line per row, tab separated, `<bg hex><fg hex><glyph>` per
cell, with a blank line between frames — useful for reviewing the exact output
without relying on your terminal's colour support.
