use super::note::Note;
use serde::{Deserialize, Serialize};

/// One-bar drum pattern saved from the beat sequencer (notes relative to beat 0).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedDrumPattern {
    pub name: String,
    pub notes: Vec<Note>,
}

impl SavedDrumPattern {
    pub fn from_track_notes(notes: &[Note], name: String, bar_length: f64) -> Self {
        let normalized: Vec<Note> = notes
            .iter()
            .filter(|n| n.start_beat >= 0.0 && n.start_beat < bar_length)
            .map(|n| Note {
                pitch: n.pitch,
                start_beat: n.start_beat,
                duration_beats: n.duration_beats,
                velocity: n.velocity,
            })
            .collect();
        Self {
            name,
            notes: normalized,
        }
    }
}
