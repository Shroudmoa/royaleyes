# glitch-eye

A single ASCII eye that watches your cursor and slowly comes apart.

```
$ cargo run --release
```

The eye is drawn at whatever size your terminal is: 24-bit colour if
`COLORTERM` advertises it, xterm-256 otherwise. It follows the mouse, blinks
on its own, and every few seconds a burst of interference rolls through —
banded RGB splits, row tears, block warps, scanline roll, a CRT brightness
ripple. Click or press `g` to set one off by hand.

## Controls

| Input | Effect |
| --- | --- |
| move the mouse | the eye looks at it (gives up after 2.5s and starts looking around on its own) |
| click | glitch burst |
| scroll | glitch burst |
| `g` | glitch burst |
| `space` / `b` | blink |
| `q` / `Esc` / `Ctrl-C` | quit |

The first few seconds show a fading hint along the bottom. Under 30x8 the eye
asks for a bigger terminal instead.

## Options

```
--chaos <0.0..1.0>   baseline interference level (default 0.18; the eye still
                     spikes above this on its own)
-h, --help
```

`--chaos 0` gives you a calm eye that still blinks and still tracks. `--chaos
1` is a permanent meltdown.

## How it is put together

The eye is a hybrid: the *background colour* of each cell carries the tone and
the *glyph* carries the texture. Iris fibres, stipple, blood vessels and the
lash line are drawn as characters, but the light and dark come from the
background, which is what makes it legible at small sizes.

- `src/eye.rs` — lid geometry, sclera, iris, specular hits. The lids are
  `sqrt(1-x²)`-ish curves in a normalised frame; they squeeze together for a
  blink but never fully collapse, and a minimum-aperture floor keeps the
  corners of the almond inside the frame when they go sub-cell.
- `src/glitch.rs` — the interference layer, applied on top of a finished frame.
  Everything is chaos-gated, so at `chaos = 0` the eye is completely clean.
- `src/frame.rs` — the cell buffer and a diffing writer, so only cells that
  actually changed get written to the terminal each frame.
- `src/color.rs` — 24-bit to xterm-256 fallback.
- `src/rng.rs` — a small xorshift PRNG, so the animation is reproducible from
  a seed. `src/util.rs` has the value noise and easing helpers.

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

The renderer is a library, so frames can be generated without a terminal:

```
cargo run --example snapshot -- out.html                        # a spread of frames as HTML
cargo run --example snapshot -- out.html 1.0,4.1 clean           # just those two, no glitch
cargo run --example snapshot -- out.html 12.0 cells.txt          # a raw cell dump as well
```

The dump is one line per row, tab separated, `<bg hex><fg hex><glyph>` per
cell, with a blank line between frames — useful for reviewing the exact output
without relying on your terminal's colour support.
