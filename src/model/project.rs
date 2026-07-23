use super::{track::Track, Note};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub tracks: Vec<Track>,
    pub bpm: f32,
    pub time_sig_numerator: u8,
    pub time_sig_denominator: u8,
    pub playhead_beat: f64,
    pub total_beats: f64,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            tracks: vec![Track::new(0), Track::new_drum(1)],
            bpm: 120.0,
            time_sig_numerator: 4,
            time_sig_denominator: 4,
            playhead_beat: 0.0,
            total_beats: 64.0,
        }
    }
}

impl Project {
    pub fn add_track(&mut self) {
        let index = self.tracks.len();
        self.tracks.push(Track::new(index));
    }

    pub fn add_drum_track(&mut self) {
        let index = self.tracks.len();
        self.tracks.push(Track::new_drum(index));
    }

    pub fn beats_per_second(&self) -> f64 {
        self.bpm as f64 / 60.0
    }

    pub fn beat_duration_secs(&self) -> f64 {
        60.0 / self.bpm as f64
    }

    pub fn duration_secs(&self) -> f64 {
        self.total_beats * self.beat_duration_secs()
    }

    pub fn all_notes(&self) -> Vec<&Note> {
        self.tracks
            .iter()
            .flat_map(|t| t.notes.iter())
            .collect()
    }
}
