//! Tests for the persistent configuration: defaults, parsing, saving.

use std::path::PathBuf;

use royaleyes::config::{self, Config};
use royaleyes::state::EyeState;

fn temp_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("royaleyes-config-{}-{tag}", std::process::id()))
}

#[test]
fn missing_file_gives_the_defaults() {
    let cfg = config::load_from(&temp_path("missing"));
    assert_eq!(cfg, Config::default());
    assert_eq!(cfg.chaos, 0.55);
    assert_eq!(cfg.state, EyeState::Normal);
    assert!(cfg.startup);
}

#[test]
fn a_valid_file_is_read() {
    let path = temp_path("valid");
    std::fs::write(&path, "chaos = 0.2\nstate = alert\nstartup = false\n").expect("write");
    let cfg = config::load_from(&path);
    assert_eq!(cfg.chaos, 0.2);
    assert_eq!(cfg.state, EyeState::Alert);
    assert!(!cfg.startup);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn an_invalid_file_falls_back_to_the_defaults() {
    let path = temp_path("invalid");
    std::fs::write(&path, "chaos = banana\nstate = bogus\nstartup maybe\n").expect("write");
    let cfg = config::load_from(&path);
    assert_eq!(cfg, Config::default());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn saving_round_trips_through_the_file() {
    let path = temp_path("roundtrip");
    let cfg = Config {
        chaos: 0.75,
        state: EyeState::Critical,
        startup: false,
    };
    cfg.save_to(&path).expect("save");
    let loaded = config::load_from(&path);
    assert_eq!(loaded, cfg);
    let _ = std::fs::remove_file(&path);
}
