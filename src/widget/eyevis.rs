use std::time::Instant;

use mpris::PlaybackStatus;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::{Block, Widget},
};

use crate::{
    eyevis::{frame::Frame, glitch, rng::Rng},
    get_border, get_color,
    state::{EyeVisState, FumState},
};

use super::FumWidget;

pub fn render(widget: &FumWidget, area: Rect, buf: &mut Buffer, state: &mut FumState) {
    if let FumWidget::Eye { border, bg, fg, chaos, .. } = widget {
        let (bg, fg_col) = get_color!(bg, fg, &state.parent_bg, &state.parent_fg);
        let border_borders = get_border!(border);

        // Render bg/border
        Block::new()
            .borders(border_borders)
            .bg(*bg)
            .fg(*fg_col)
            .render(area, buf);

        // Inner area for eye
        let inner = if *border {
            Rect {
                x: area.x + 1,
                y: area.y + 1,
                width: if area.width > 2 { area.width - 2 } else { 0 },
                height: if area.height > 2 { area.height - 2 } else { 0 },
            }
        } else {
            area
        };

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        // Initialize eyevis state if needed
        if state.eyevis.is_none() {
            let gain = chaos.unwrap_or(0.55).clamp(0.0, 1.0);
            state.eyevis = Some(EyeVisState {
                eye: crate::eyevis::eye::Eye::new(gain),
                rng: Rng::from_entropy(),
                last: Instant::now(),
                last_track: format!("{}|{}", state.meta.title, state.meta.artists.join(",")),
                was_playing: matches!(state.meta.status, PlaybackStatus::Playing),
            });
        }

        let now = Instant::now();
        if let Some(ev) = state.eyevis.as_mut() {
            let dt = (now - ev.last).as_secs_f32().min(0.1);
            ev.last = now;

            // Detect track change
            let track_key = format!("{}|{}", state.meta.title, state.meta.artists.join(","));
            if track_key != ev.last_track && track_key != "|".to_string() {
                ev.last_track = track_key.clone();
                ev.eye.burst(&mut ev.rng);
            }

            // React to playback state
            let is_playing = matches!(state.meta.status, PlaybackStatus::Playing);
            if is_playing && !ev.was_playing {
                ev.eye.burst(&mut ev.rng);
                ev.was_playing = true;
            } else if !is_playing {
                ev.was_playing = false;
                ev.eye.gain = (ev.eye.gain * 0.995).max(0.1);
            } else {
                ev.was_playing = true;
                ev.eye.gain = ev.eye.gain.max(0.4).min(1.0);
            }

            // Update eye - no mouse in fum context
            ev.eye.update(dt, &mut ev.rng, None, inner.width as usize, inner.height as usize);

            // Render to frame
            let mut frame = Frame::new(inner.width as usize, inner.height as usize);
            ev.eye.draw(&mut frame, &mut ev.rng);
            glitch::apply(&mut frame, &mut ev.rng, ev.eye.chaos, ev.eye.t);

            // Convert to ratatui buffer
            for y in 0..inner.height as usize {
                for x in 0..inner.width as usize {
                    let cell = frame.cells[y * frame.w + x];
                    let rx = inner.x + x as u16;
                    let ry = inner.y + y as u16;
                    if rx < area.x + area.width && ry < area.y + area.height {
                        let mut style = Style::default().bg(*bg);
                        if let Some(bgcol) = cell.bg {
                            style = style.bg(Color::Rgb(bgcol.0, bgcol.1, bgcol.2));
                        }
                        if let Some(fgcol) = cell.fg {
                            style = style.fg(Color::Rgb(fgcol.0, fgcol.1, fgcol.2));
                        } else {
                            style = style.fg(*fg_col);
                        }
                        buf.set_string(rx, ry, cell.ch.to_string(), style);
                    }
                }
            }
        }
    }
}
