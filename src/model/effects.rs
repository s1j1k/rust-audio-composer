use serde::{Deserialize, Serialize};

/// Filter knob positions closer to centre than this are "off" (a real DJ filter's detent).
pub const FILTER_DEAD_ZONE: f32 = 0.03;

/// Default echo time: a dotted eighth (3/4 of a beat), the classic DJ echo.
pub const DEFAULT_ECHO_BEATS: f32 = 0.75;

/// DJ-style EQ knob (-1 = kill, 0 = flat, +1 = +6 dB) to a linear gain.
pub fn eq_gain(knob: f32) -> f32 {
    let k = knob.clamp(-1.0, 1.0);
    if k < 0.0 {
        (1.0 + k) * (1.0 + k)
    } else {
        1.0 + k
    }
}

/// Per-track mix effects: channel EQ, DJ filter, echo, plus the original
/// distance / underwater / reverb / phaser "space" effects.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackEffects {
    /// 0 = up close, 1 = far away / underwater (auto low-pass + reverb).
    pub distance: f32,
    /// Reverb wet mix 0–1.
    pub reverb: f32,
    /// Phaser / swirl depth 0–1.
    pub phaser: f32,
    /// Phaser LFO rate in Hz.
    pub phaser_rate: f32,
    /// Manual low-pass 0 = muffled, 1 = bright.
    pub lowpass: f32,
    /// 3-band EQ, each -1 (kill) .. 0 (flat) .. +1 (+6 dB).
    pub eq_low: f32,
    pub eq_mid: f32,
    pub eq_high: f32,
    /// DJ colour filter: -1 = low-pass closed, 0 = off, +1 = high-pass closed.
    pub filter: f32,
    /// Echo amount 0–1 (send and feedback rise together).
    pub echo: f32,
    /// Echo time in beats (tempo-synced).
    pub echo_beats: f32,
}

impl Default for TrackEffects {
    fn default() -> Self {
        Self {
            distance: 0.0,
            reverb: 0.0,
            phaser: 0.0,
            phaser_rate: 0.4,
            lowpass: 1.0,
            eq_low: 0.0,
            eq_mid: 0.0,
            eq_high: 0.0,
            filter: 0.0,
            echo: 0.0,
            echo_beats: DEFAULT_ECHO_BEATS,
        }
    }
}

impl TrackEffects {
    pub fn effective_lowpass(&self) -> f32 {
        let dist_cutoff = 1.0 - self.distance * 0.88;
        self.lowpass.min(dist_cutoff).clamp(0.02, 1.0)
    }

    pub fn effective_reverb(&self) -> f32 {
        (self.reverb + self.distance * 0.55).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MasterEffects {
    pub distance: f32,
    pub reverb: f32,
    pub phaser: f32,
    pub phaser_rate: f32,
    pub lowpass: f32,
}

impl Default for MasterEffects {
    fn default() -> Self {
        Self {
            distance: 0.0,
            reverb: 0.0,
            phaser: 0.0,
            phaser_rate: 0.35,
            lowpass: 1.0,
        }
    }
}

impl MasterEffects {
    pub fn effective_lowpass(&self) -> f32 {
        let dist_cutoff = 1.0 - self.distance * 0.88;
        self.lowpass.min(dist_cutoff).clamp(0.02, 1.0)
    }

    pub fn effective_reverb(&self) -> f32 {
        (self.reverb + self.distance * 0.55).clamp(0.0, 1.0)
    }

    /// The master bus as a [`TrackEffects`] (no EQ / filter / echo on the master).
    pub fn as_track(&self) -> TrackEffects {
        TrackEffects {
            distance: self.distance,
            reverb: self.reverb,
            phaser: self.phaser,
            phaser_rate: self.phaser_rate,
            lowpass: self.lowpass,
            ..TrackEffects::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_knob_maps_kill_flat_boost() {
        assert_eq!(eq_gain(-1.0), 0.0);
        assert_eq!(eq_gain(0.0), 1.0);
        assert_eq!(eq_gain(1.0), 2.0);
        assert!(eq_gain(-0.5) < 1.0 && eq_gain(-0.5) > 0.0);
    }

    #[test]
    fn old_project_effects_still_load() {
        // A v5 file only had the five original fields.
        let json = r#"{"distance":0.2,"reverb":0.1,"phaser":0.0,"phaser_rate":0.4,"lowpass":1.0}"#;
        let fx: TrackEffects = serde_json::from_str(json).unwrap();
        assert_eq!(fx.distance, 0.2);
        assert_eq!(fx.filter, 0.0);
        assert_eq!(fx.echo_beats, DEFAULT_ECHO_BEATS);
    }
}
