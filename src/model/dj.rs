//! DJ mode: which tracks sit on the four mixer channels, crossfader assignment, and the
//! crossfader itself (which can be automated like any other control).

use super::automation::AutomationLane;
use serde::{Deserialize, Serialize};

/// Number of channel strips on the DJ mixer.
pub const DJ_CHANNELS: usize = 4;

/// Which side of the crossfader a channel is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XfadeSide {
    A,
    /// Not affected by the crossfader.
    #[default]
    Thru,
    B,
}

impl XfadeSide {
    pub fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::Thru => "THRU",
            Self::B => "B",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::Thru => "thru",
            Self::B => "b",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "a" => Some(Self::A),
            "thru" => Some(Self::Thru),
            "b" => Some(Self::B),
            _ => None,
        }
    }
}

/// Per-track DJ settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackDj {
    /// Mixer channel (0..DJ_CHANNELS) this track is loaded on, if any.
    pub channel: Option<u8>,
    pub side: XfadeSide,
}

/// Project-wide DJ mixer state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DjMix {
    /// -1 = full A, 0 = centre (both), +1 = full B.
    pub crossfader: f32,
    /// Crossfader moves over time (empty = use `crossfader`).
    pub crossfader_lane: AutomationLane,
}

impl Default for DjMix {
    fn default() -> Self {
        Self {
            crossfader: 0.0,
            crossfader_lane: AutomationLane::default(),
        }
    }
}

impl DjMix {
    /// Crossfader position at `beat` (automation wins over the fixed position).
    pub fn crossfader_at(&self, beat: f64) -> f32 {
        self.crossfader_lane
            .value_at(beat)
            .unwrap_or(self.crossfader)
            .clamp(-1.0, 1.0)
    }
}

/// Gain for a channel on `side` with the crossfader at `x` (-1..1).
///
/// A "no dip" curve, like the default on most club mixers: both sides are at full level
/// in the middle, and each fades out smoothly (constant-power shape) over its far half.
pub fn crossfader_gain(side: XfadeSide, x: f32) -> f32 {
    let t = (x.clamp(-1.0, 1.0) + 1.0) * 0.5; // 0 = full A, 1 = full B
    let toward = match side {
        XfadeSide::Thru => return 1.0,
        XfadeSide::A => t,
        XfadeSide::B => 1.0 - t,
    };
    (std::f32::consts::SQRT_2 * (toward * std::f32::consts::FRAC_PI_2).cos()).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossfader_curve() {
        use XfadeSide::*;
        assert_eq!(crossfader_gain(A, -1.0), 1.0);
        assert!(crossfader_gain(A, 1.0).abs() < 1e-6);
        assert!(crossfader_gain(B, -1.0).abs() < 1e-6);
        assert_eq!(crossfader_gain(B, 1.0), 1.0);
        // No dip in the middle.
        assert!((crossfader_gain(A, 0.0) - 1.0).abs() < 1e-5);
        assert!((crossfader_gain(B, 0.0) - 1.0).abs() < 1e-5);
        // Thru ignores the fader.
        assert_eq!(crossfader_gain(Thru, 1.0), 1.0);
        // Monotonic fade.
        let mut prev = 1.1;
        for i in 0..=20 {
            let g = crossfader_gain(A, -1.0 + i as f32 / 10.0);
            assert!(g <= prev + 1e-6);
            prev = g;
        }
    }

    #[test]
    fn crossfader_automation_wins() {
        let mut mix = DjMix::default();
        mix.crossfader = -1.0;
        assert_eq!(mix.crossfader_at(5.0), -1.0);
        mix.crossfader_lane.set_point(0.0, -1.0);
        mix.crossfader_lane.set_point(8.0, 1.0);
        assert!((mix.crossfader_at(4.0)).abs() < 1e-6);
    }
}
