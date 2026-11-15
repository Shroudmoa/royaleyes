# RoyalEyes

An ASCII eye that watches your cursor and slowly comes apart.

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

## The eye states

The eye wears one of four states, chosen with the menu (`m`) or from the
command line.

| State | Feeling |
| --- | --- |
| `NORMAL` | Calm eye. Natural movement and blinking, the default. |
| `ALERT` | Opens wider, reacts faster, slightly stronger animation. |
| `WARNING` | Red accents with intermittent red glitches, more aggressive movement. |
| `CRITICAL` | Aggressive distortion, stronger glitches, erratic movement. |

States are behaviour profiles the eye eases into, so switching never snaps —
the transition cross-fades over about half a second.

## Startup sequence

On launch the terminal goes dark, the eye materializes out of a gentle wake-up
(about two and a half seconds), and settles into its initial state. The sequence
can be skipped with `--no-startup` or turned off in the configuration.

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
| `m` | toggle the eye state menu |
| `enter` | select the highlighted state |
| `esc` | close the menu (quit when the menu is closed) |
| `q` / `Esc` / `Ctrl-C` | quit |

While the state menu is open, the arrow keys drive the menu instead of the
chaos fader.

## Options

```
--chaos <0.0..1.0>             where the fader starts (default 0.55)
--state <normal|alert|warning|critical>   where the eye starts
--startup / --no-startup       whether the wake-up sequence plays
-h, --help
```

The saved configuration lives in `~/.config/royaleyes/` and holds the initial
chaos level, the initial eye state and the startup animation preference. An
explicit command line flag always wins over the file.

## How it is put together

The eye is a hybrid: the *background colour* of each cell carries the tone and
the *glyph* carries the texture. Iris fibres, stipple, blood vessels and the
lash line are drawn as characters, but the light and dark come from the
background, which is what makes it legible at small sizes.

- `src/eye.rs` — lid geometry, sclera, iris, specular hits, and the behaviour
  profiles of the states. The lids are `sqrt(1-x²)`-ish curves in a normalised
  frame; they squeeze together for a blink but never fully collapse, and a
  minimum-aperture floor keeps the corners of the almond inside the frame when
  they go sub-cell.
- `src/state.rs` — NORMAL, ALERT, WARNING and CRITICAL behaviour profiles, and
  the cross-fade that eases the eye into a new state.
- `src/chaos.rs` — the fader: level, the bar's layout, the click-to-set mapping,
  and the idle fade. The track is a ramp across its whole length, so both ends
  are exactly reachable. Hit-testing is gated on the bar being up, so a faded-out
  control cannot be aimed at, but its old footprint is still remembered so a
  click there can wake it.
- `src/glitch.rs` — the interference layer, applied on top of a finished frame.
  Everything is chaos-gated, so at `chaos = 0` the eye is completely clean; the
  red states add a red accent to the static and signal-loss noise.
- `src/frame.rs` — the cell buffer and a diffing writer, so only cells that
  actually changed get written to the terminal each frame.
- `src/color.rs` — 24-bit to xterm-256 fallback.
- `src/rng.rs` — a small xorshift PRNG, so the animation is reproducible from
  a seed. `src/util.rs` has the value noise and easing helpers.
- `src/menu.rs` — the small state menu overlay.
- `src/config.rs` — the `~/.config/royaleyes/` preferences.
- `src/startup.rs` — the wake-up sequence.

The eye runs its own interference driver regardless of where the fader is: it
idles at a low level and spikes on its own. The fader is a gain on the *output*
of that, not a throttle on the driver. So turning the chaos down does not stop
the eye having an opinion, it just stops that opinion reaching the screen — and
turning it back up releases whatever the driver was doing while it was quiet.

Only dependency is `crossterm`.

## Development

```
cargo test                    # structural tests: is it shaped like an eye?
cargo build --all-targets    # also compile the examples and tests
```

`tests/eye_shape.rs` checks the things that are easy to break silently: the
eye stays centred and wider than it is tall, the lids taper towards the corners,
there is a bright sclera, a dark pupil and a visible iris, a blink squashes the
height without moving or narrowing the eye, the iris tracks the mouse, chaos
stays clamped, and nothing panics on a 1x1 terminal.

`tests/states.rs` covers the event states: calm by default, alert wider and
stronger, warning red and more aggressive, critical the most intense — plus the
smooth cross-fade on switch.

`tests/menu.rs` covers the state menu: navigation wraps, selection, drawing.

`tests/config.rs` covers the persisted preferences: defaults, parsing, saving.

`tests/cli.rs` covers the argument parsing and clamping.

`tests/startup.rs` covers the wake-up easing and duration.

`tests/slider.rs` covers the fader: the level clamps at both ends, a key press
moves it exactly one step, the track maps clicks monotonically with 0 and 1
exactly reachable, and it always fits the width it is drawn into. The idle fade
gets its own group — it is up at startup, still up a moment before the deadline,
gone after it, brought back by a change (or by a press that lands on the value it
was already at, because somebody holding `+` against the top of the range is
still using the control), held up by the pointer, painted not at all once it has
gone, and dimming monotonically over more than one frame rather than blinking
out.

Then the part that matters: the eye is provably silent at zero gain, provably no
louder than promised at full gain, and provably ordered in between.

The renderer is a library, so frames can be generated without a terminal:

```
cargo run --example snapshot -- out.html                        # a spread of frames as HTML
cargo run --example snapshot -- out.html 1.0,4.1 clean        # just those two, no glitch
cargo run --example snapshot -- out.html 12.0 cells.txt      # a raw cell dump as well
cargo run --example slider_preview -- bar.html               # the bar alone, many widths
cargo run --example gain_preview -- gain.html                # the eye + bar, many levels
```

The dump is one line per row, tab separated, `<bg hex><fg hex><glyph>` per
cell, with a blank line between frames — useful for reviewing the exact output
without relying on your terminal's colour support.
