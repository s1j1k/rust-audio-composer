use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SongSection {
    pub id: u64,
    pub label: String,
    pub start_beat: f64,
    pub bar_count: u32,
}

impl SongSection {
    pub fn new(id: u64, label: String, start_beat: f64, bar_count: u32) -> Self {
        Self {
            id,
            label,
            start_beat,
            bar_count: bar_count.max(1),
        }
    }

    pub fn length_beats(&self, beats_per_bar: f64) -> f64 {
        self.bar_count as f64 * beats_per_bar
    }

    pub fn end_beat(&self, beats_per_bar: f64) -> f64 {
        self.start_beat + self.length_beats(beats_per_bar)
    }
}
