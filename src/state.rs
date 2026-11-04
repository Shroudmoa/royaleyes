use std::collections::HashMap;
use std::time::Instant;

use ratatui::{layout::Rect, style::Color};

use crate::{action::Action, meta::Meta, widget::{Direction, SliderSource}};
use crate::eyevis::{eye::Eye, rng::Rng};

pub struct EyeVisState {
    pub eye: Eye,
    pub rng: Rng,
    pub last: Instant,
    pub last_track: String,
    pub was_playing: bool,
}

pub struct FumState {
    pub meta: Meta,
    pub cover_art_ascii: String,
    pub buttons: HashMap<String, (Rect, Option<Action>, Option<Action>, Option<String>)>,
    pub sliders: HashMap<String, (Rect, Direction, SliderSource)>,
    pub vars: HashMap<String, String>,
    pub parent_direction: Direction,
    pub parent_bg: Color,
    pub parent_fg: Color,
    pub eyevis: Option<EyeVisState>,
}

impl FumState {
    pub fn new(meta: Meta) -> Self {
        Self {
            meta,
            cover_art_ascii: String::new(),
            buttons: HashMap::new(),
            sliders: HashMap::new(),
            vars: HashMap::new(),
            parent_direction: Direction::default(),
            parent_bg: Color::Reset,
            parent_fg: Color::Reset,
            eyevis: None,
        }
    }
}
