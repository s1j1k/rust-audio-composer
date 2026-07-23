use serde::{Deserialize, Serialize};

/// Per-track DJ-style mix effects (GarageBand-like distance / underwater / phasing).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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
}

impl Default for TrackEffects {
    fn default() -> Self {
        Self {
            distance: 0.0,
            reverb: 0.0,
            phaser: 0.0,
            phaser_rate: 0.4,
            lowpass: 1.0,
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
}
