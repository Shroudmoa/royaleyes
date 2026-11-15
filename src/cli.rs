//! Command line arguments; explicit flags override the saved configuration.

use crate::state::EyeState;

/// Explicit flags beat the saved configuration, so "not given" is `None`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Explicit `--chaos` value, if given.
    pub chaos: Option<f32>,
    /// Explicit `--state` value, if given.
    pub state: Option<EyeState>,
    /// Explicit `--startup` / `--no-startup`, if given.
    pub startup: Option<bool>,
}

/// Parse the arguments after the program name.
pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Options {
    let mut opts = Options::default();
    let mut iter = args.into_iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--chaos" => {
                if let Some(v) = iter.next() {
                    opts.chaos = v.parse::<f32>().ok().map(|v| v.clamp(0.0, 1.0));
                }
            }
            "--state" => {
                if let Some(v) = iter.next() {
                    opts.state = v.parse::<EyeState>().ok();
                }
            }
            "--startup" => opts.startup = Some(true),
            "--no-startup" => opts.startup = Some(false),
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
    }
    opts
}

/// The help text, in the voice of the project.
pub fn print_help() {
    println!(
        "royaleyes [--chaos <0.0..1.0>] [--state <normal|alert|warning|critical>] [--startup|--no-startup]"
    );
    println!("  move the mouse : the eye follows");
    println!("  click / g      : force a glitch burst");
    println!("  space / b      : blink");
    println!("  up / down      : chaos, up to full meltdown (or menu: pick a state)");
    println!("  p / c          : peaceful / chaos, straight there");
    println!("  m              : toggle the eye state menu");
    println!("  enter          : select the highlighted state");
    println!("  esc            : close the menu / quit");
    println!("  q / C-c        : quit");
}
