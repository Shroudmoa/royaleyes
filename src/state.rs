//! The eye's states and the behaviour each one drives: NORMAL, ALERT, WARNING and CRITICAL.

use crate::util::lerp;

/// What the eye is doing, and how it should feel.
///
/// The eye never gets four separate implementations: every state is a set of
/// behaviour targets that the eye eases towards, so switching states cross-fades
/// instead of snapping between code paths.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EyeState {
    /// Calm eye: natural movement and blinking, the default.
    #[default]
    Normal,
    /// Attentive: opens wider, reacts faster, slightly stronger animation.
    Alert,
    /// Red accents with intermittent red glitches, more aggressive movement.
    Warning,
    /// Aggressive distortion, stronger glitches, erratic movement. Urgent.
    Critical,
}

impl EyeState {
    /// The states, in menu order.
    pub const ALL: [EyeState; 4] = [
        EyeState::Normal,
        EyeState::Alert,
        EyeState::Warning,
        EyeState::Critical,
    ];

    /// Lowercase identifier used in config files and on the command line.
    pub fn name(self) -> &'static str {
        match self {
            EyeState::Normal => "normal",
            EyeState::Alert => "alert",
            EyeState::Warning => "warning",
            EyeState::Critical => "critical",
        }
    }

    /// Short label used in the menu.
    pub fn label(self) -> &'static str {
        match self {
            EyeState::Normal => "NORMAL",
            EyeState::Alert => "ALERT",
            EyeState::Warning => "WARNING",
            EyeState::Critical => "CRITICAL",
        }
    }

    /// The behaviour targets for this state.
    pub fn profile(self) -> Profile {
        match self {
            EyeState::Normal => Profile {
                open: 1.0,
                follow_rate: 11.0,
                dart_rate: 16.0,
                wander: 1.0,
                burst: 1.0,
                glitch: 1.0,
                red: false,
            },
            EyeState::Alert => Profile {
                open: 1.15,
                follow_rate: 14.0,
                dart_rate: 20.0,
                wander: 1.5,
                burst: 1.4,
                glitch: 1.6,
                red: false,
            },
            EyeState::Warning => Profile {
                open: 1.05,
                follow_rate: 12.0,
                dart_rate: 26.0,
                wander: 2.4,
                burst: 2.2,
                glitch: 2.4,
                red: true,
            },
            EyeState::Critical => Profile {
                open: 1.25,
                follow_rate: 9.0,
                dart_rate: 42.0,
                wander: 5.0,
                burst: 3.5,
                glitch: 4.0,
                red: true,
            },
        }
    }
}

impl std::str::FromStr for EyeState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Ok(EyeState::Normal),
            "alert" => Ok(EyeState::Alert),
            "warning" => Ok(EyeState::Warning),
            "critical" => Ok(EyeState::Critical),
            _ => Err(()),
        }
    }
}

/// Behavioural targets for one state. The eye eases between profiles, so a
/// state switch cross-fades instead of snapping.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    /// Target lid aperture while calm: 1 = wide open, 0 = shut.
    pub open: f32,
    /// Gaze pursuit rate (lerp factor per second) while following.
    pub follow_rate: f32,
    /// Gaze lerp rate (lerp factor per second) while wandering on its own.
    pub dart_rate: f32,
    /// Multiplier on the random gaze wander.
    pub wander: f32,
    /// Multiplier on interference bursts.
    pub burst: f32,
    /// Multiplier on the glitch layer.
    pub glitch: f32,
    /// Red accent colour for veins, iris and glitches.
    pub red: bool,
}

impl Profile {
    /// Ease from the current profile towards `target`.
    pub fn mix(&self, target: Profile, t: f32) -> Profile {
        let t = t.clamp(0.0, 1.0);
        Profile {
            open: lerp(self.open, target.open, t),
            follow_rate: lerp(self.follow_rate, target.follow_rate, t),
            dart_rate: lerp(self.dart_rate, target.dart_rate, t),
            wander: lerp(self.wander, target.wander, t),
            burst: lerp(self.burst, target.burst, t),
            glitch: lerp(self.glitch, target.glitch, t),
            // Booleans cannot lerp; settle on the new side halfway through.
            red: if t < 0.5 { self.red } else { target.red },
        }
    }
}

/// Seconds a state switch takes to ease into the new behaviour.
pub const TRANSITION: f32 = 0.7;
