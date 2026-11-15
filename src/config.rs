//! Persistent preferences: `~/.config/royaleyes/`.

use std::fs;
use std::path::Path;

use crate::state::EyeState;

/// Starting chaos level used when nothing better is known.
pub const DEFAULT_CHAOS: f32 = 0.55;

/// The preferences RoyalEyes keeps between runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// Starting chaos level, 0.0..=1.0.
    pub chaos: f32,
    /// Eye state the session starts in.
    pub state: EyeState,
    /// Whether the startup / wake-up sequence plays.
    pub startup: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            chaos: DEFAULT_CHAOS,
            state: EyeState::Normal,
            startup: true,
        }
    }
}

impl Config {
    /// Write the preferences to the config directory.
    pub fn save(&self) -> std::io::Result<()> {
        save_to(self, &config_dir().join("config"))
    }

    /// Write the preferences to an explicit path, so saving can be exercised
    /// without touching the real config directory.
    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        std::fs::create_dir_all(dir)?;
        std::fs::write(
            path,
            format!(
                "# RoyalEyes configuration\nchaos = {:.2}\nstate = {}\nstartup = {}\n",
                self.chaos, self.state.name(), self.startup
            ),
        )
    }
}

/// Write preferences to an explicit path.
pub fn save_to(cfg: &Config, path: &std::path::Path) -> std::io::Result<()> {
    cfg.save_to(path)
}

/// The config file: `~/.config/royaleyes/config`.
pub fn config_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".config"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    base.join("royaleyes")
}

pub fn config_path() -> std::path::PathBuf {
    config_dir().join("config")
}

/// Load the persisted preferences, falling back to defaults when the file is
/// missing or does not parse.
pub fn load() -> Config {
    load_from(&config_path())
}

/// Load from an explicit path, so the loading rules can be exercised without
/// touching the real config directory.
pub fn load_from(path: &Path) -> Config {
    let Ok(text) = fs::read_to_string(path) else {
        return Config::default();
    };
    let mut cfg = Config::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "chaos" => {
                if let Ok(v) = value.parse::<f32>() {
                    cfg.chaos = v.clamp(0.0, 1.0);
                }
            }
            "state" => {
                cfg.state = value.parse::<EyeState>().unwrap_or(EyeState::Normal);
            }
            "startup" => {
                cfg.startup = value.eq_ignore_ascii_case("true");
            }
            _ => {}
        }
    }
    cfg
}
