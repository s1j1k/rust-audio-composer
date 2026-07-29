use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct LoopRegion {
    pub enabled: bool,
    pub start_beat: f64,
    pub end_beat: f64,
}

impl Default for LoopRegion {
    fn default() -> Self {
        Self {
            enabled: false,
            start_beat: 0.0,
            end_beat: 4.0,
        }
    }
}

impl LoopRegion {
    pub fn length(&self) -> f64 {
        (self.end_beat - self.start_beat).max(0.0)
    }

    pub fn contains(&self, beat: f64) -> bool {
        beat >= self.start_beat && beat < self.end_beat
    }
}
