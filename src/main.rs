//! glitch-eye: an ASCII eye that watches your cursor and slowly comes apart.
//!
//! Run with `cargo run --release`. `q` or `Esc` to leave.

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseEventKind,
};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{cursor, execute, queue, terminal};

use glitch_eye::color::Rgb;
use glitch_eye::eye::Eye;
use glitch_eye::frame::{Cell, Frame};
use glitch_eye::glitch;
use glitch_eye::rng::Rng;

const FPS: f32 = 32.0;
const HINT_SECONDS: f32 = 7.0;

/// Restores the terminal no matter how we leave: normal return, `q`, or a
/// panic (so the shell is not left in raw mode with the alt screen active).
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<TerminalGuard> {
        enable_raw_mode()?;
        let mut out = io::stdout();
        execute!(
            out,
            terminal::EnterAlternateScreen,
            cursor::Hide,
            EnableMouseCapture,
            terminal::Clear(terminal::ClearType::All)
        )?;
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            DisableMouseCapture,
            cursor::Show,
            terminal::LeaveAlternateScreen
        );
    }
}
struct Options {
    chaos: f32,
}

fn parse_args() -> Options {
    let mut opts = Options { chaos: 0.18 };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--chaos" => {
                if let Some(v) = args.next().and_then(|v| v.parse::<f32>().ok()) {
                    opts.chaos = v.clamp(0.0, 1.0);
                }
            }
            "-h" | "--help" => {
                println!("glitch-eye [--chaos <0.0..1.0>]");
                println!("  move the mouse : the eye follows");
                println!("  click / g      : force a glitch burst");
                println!("  space / b      : blink");
                println!("  q / Esc / C-c  : quit");
                std::process::exit(0);
            }
            _ => {}
        }
    }
    opts
}

fn main() -> io::Result<()> {
    let opts = parse_args();
    let _guard = TerminalGuard::enter()?;

    let truecolor = std::env::var("COLORTERM")
        .map(|v| v.contains("truecolor") || v.contains("24bit"))
        .unwrap_or(false);

    let mut rng = Rng::from_entropy();
    let mut eye = Eye::new(opts.chaos);
    let mut out = io::stdout();
    let mut prev: Vec<Cell> = Vec::new();
    let mut frame = Frame::new(80, 24);
    let mut mouse: Option<(f32, f32)> = None;
    let mut mouse_moved_at = 0.0f32;

    let start = Instant::now();
    let mut last = start;
    let mut t = 0.0f32;

    loop {
        // ---- input ------------------------------------------------------
        while event::poll(Duration::from_millis(0))? {
            match event::read()? {
                Event::Key(k) if k.kind != KeyEventKind::Release => {
                    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                    match k.code {
                        KeyCode::Char('c') if ctrl => return Ok(()),
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char(' ') | KeyCode::Char('b') => eye.blink(),
                        KeyCode::Char('g') => eye.burst(&mut rng),
                        _ => {}
                    }
                }
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                        mouse = Some((m.column as f32, m.row as f32));
                        mouse_moved_at = t;
                    }
                    MouseEventKind::Down(_) => {
                        eye.burst(&mut rng);
                        if m.modifiers.contains(KeyModifiers::SHIFT) {
                            eye.blink();
                        }
                    }
                    MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                        eye.burst(&mut rng);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // ---- time ---------------------------------------------------------
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.1);
        last = now;
        t += dt;

        let (w, h) = terminal::size().map(|(w, h)| (w as usize, h as usize))?;
        if w != frame.w || h != frame.h {
            frame.resize(w, h);
            prev.clear();
            queue!(out, terminal::Clear(terminal::ClearType::All))?;
        }

        // Once the mouse goes quiet, hand the gaze back to the eye itself.
        if mouse.is_some() && t - mouse_moved_at > 2.5 {
            mouse = None;
        }

        // ---- simulate ------------------------------------------------------
        eye.update(dt, &mut rng, mouse, w, h);

        // ---- render --------------------------------------------------------
        eye.draw(&mut frame, &mut rng);
        glitch::apply(&mut frame, &mut rng, eye.chaos, t);
        glitch::crt_drift(&mut frame, t, eye.chaos);
        if w < 30 || h < 8 {
            too_small(&mut frame, t);
        } else {
            hint(&mut frame, t, w, h);
        }
        frame.present(&mut out, &mut prev, truecolor)?;

        // ---- pace ------------------------------------------------------------
        let spent = now.elapsed().as_secs_f32();
        let budget = 1.0 / FPS;
        if spent < budget {
            std::thread::sleep(Duration::from_secs_f32(budget - spent));
        }
    }
}

/// The terminal is too cramped to draw an eye in.
fn too_small(frame: &mut Frame, t: f32) {
    let msg = "make me bigger";
    let col = (frame.w as i32 - msg.len() as i32) / 2;
    let row = frame.h as i32 / 2;
    if col < 0 {
        return;
    }
    let a = (t * 3.0) as i32;
    let b = (t * 5.0 + 1.0) as i32;
    for (i, ch) in msg.chars().enumerate() {
        frame.set(
            col + i as i32,
            row + ((a - b).rem_euclid(3) - 1),
            Cell {
                ch,
                fg: Some(Rgb(120, 255, 200)),
                bg: Some(Rgb(6, 6, 12)),
            },
        );
    }
}

fn hint(frame: &mut Frame, t: f32, w: usize, h: usize) {
    let left = HINT_SECONDS - t;
    if left <= 0.0 || h < 3 {
        return;
    }
    let msg = " move mouse: look   click: glitch   space: blink   q: quit ";
    let start = ((w as i32 - msg.len() as i32) / 2).max(0);
    let row = h as i32 - 1;
    let fade = (left / 1.5).clamp(0.0, 1.0);
    let dim = 60.0 + 90.0 * fade;
    for (i, ch) in msg.chars().enumerate() {
        frame.set(
            start + i as i32,
            row,
            Cell {
                ch,
                fg: Some(Rgb(dim as u8, (dim * 1.1) as u8, (dim * 1.2) as u8)),
                bg: Some(Rgb::VOID),
            },
        );
    }
}
