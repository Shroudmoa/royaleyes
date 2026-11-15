//! Tests for the command line parsing: flags override defaults, values clamp.

use royaleyes::cli;
use royaleyes::state::EyeState;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn no_arguments_leaves_everything_unset() {
    let opts = cli::parse_args(Vec::<String>::new().into_iter());
    assert_eq!(opts.chaos, None);
    assert_eq!(opts.state, None);
    assert_eq!(opts.startup, None);
}

#[test]
fn chaos_flag_is_parsed_and_clamped() {
    let opts = cli::parse_args(args(&["--chaos", "0.2"]));
    assert_eq!(opts.chaos, Some(0.2));

    let opts = cli::parse_args(args(&["--chaos", "9"]));
    assert_eq!(opts.chaos, Some(1.0));

    let opts = cli::parse_args(args(&["--chaos", "nope"]));
    assert_eq!(opts.chaos, None);
}

#[test]
fn state_flag_is_parsed() {
    let opts = cli::parse_args(args(&["--state", "critical"]));
    assert_eq!(opts.state, Some(EyeState::Critical));

    let opts = cli::parse_args(args(&["--state", "bogus"]));
    assert_eq!(opts.state, None);
}

#[test]
fn startup_flag_can_be_given_explicitly() {
    let opts = cli::parse_args(args(&["--startup"]));
    assert_eq!(opts.startup, Some(true));

    let opts = cli::parse_args(args(&["--no-startup"]));
    assert_eq!(opts.startup, Some(false));
}
