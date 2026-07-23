use super::instrument::InstrumentId;
use super::note::Note;
use super::effects::TrackEffects;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    Melodic,
    Drum,
}

impl Default for TrackKind {
    fn default() -> Self {
        Self::Melodic
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    pub instrument: InstrumentId,
    pub kind: TrackKind,
    pub volume: f32,
    pub muted: bool,
    pub notes: Vec<Note>,
    pub color: [u8; 3],
    #[serde(default)]
    pub effects: TrackEffects,
}

impl Track {
    pub fn new(index: usize) -> Self {
        let colors: [[u8; 3]; 6] = [
            [100, 180, 255],
            [255, 140, 100],
            [140, 220, 140],
            [220, 140, 220],
            [255, 220, 100],
            [180, 180, 255],
        ];
        Self {
            name: format!("Track {}", index + 1),
            instrument: InstrumentId::Piano,
            kind: TrackKind::Melodic,
            volume: 0.8,
            muted: false,
            notes: Vec::new(),
            color: colors[index % colors.len()],
            effects: TrackEffects::default(),
        }
    }

    pub fn new_drum(index: usize) -> Self {
        let mut track = Self::new(index);
        track.name = format!("Drums {}", index + 1);
        track.instrument = InstrumentId::Drums;
        track.kind = TrackKind::Drum;
        track.color = [255, 100, 80];
        track
    }

    pub fn is_drum(&self) -> bool {
        self.kind == TrackKind::Drum || self.instrument.is_drum_track()
    }

    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
        self.notes.sort_by(|a, b| {
            a.start_beat
                .partial_cmp(&b.start_beat)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}
