use super::instrument::InstrumentId;
use super::note::Note;
use super::note_clip::NoteClip;
use super::effects::TrackEffects;
use super::loop_region::LoopRegion;
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
    #[serde(default)]
    pub clips: Vec<NoteClip>,
    /// Legacy flat notes from older project files — migrated into clips on load.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    notes: Vec<Note>,
    pub color: [u8; 3],
    #[serde(default)]
    pub effects: TrackEffects,
    #[serde(default)]
    pub loop_region: LoopRegion,
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
            clips: Vec::new(),
            notes: Vec::new(),
            color: colors[index % colors.len()],
            effects: TrackEffects::default(),
            loop_region: LoopRegion::default(),
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

    pub fn ensure_clips_migrated(&mut self, next_id: &mut u64) {
        if self.clips.is_empty() && !self.notes.is_empty() {
            let clip = NoteClip::from_legacy_notes(*next_id, std::mem::take(&mut self.notes));
            *next_id += 1;
            self.clips.push(clip);
        }
    }

    pub fn all_notes_flat(&self) -> Vec<(usize, usize, Note)> {
        self.clips
            .iter()
            .enumerate()
            .flat_map(|(ci, clip)| {
                clip.notes
                    .iter()
                    .enumerate()
                    .map(move |(ni, n)| {
                        (
                            ci,
                            ni,
                            Note {
                                pitch: n.pitch,
                                start_beat: clip.note_absolute_start(n),
                                duration_beats: n.duration_beats,
                                velocity: n.velocity,
                            },
                        )
                    })
            })
            .collect()
    }

    pub fn clip_mut(&mut self, index: usize) -> Option<&mut NoteClip> {
        self.clips.get_mut(index)
    }

    pub fn add_clip(&mut self, clip: NoteClip) {
        self.clips.push(clip);
        self.clips.sort_by(|a, b| {
            a.start_beat
                .partial_cmp(&b.start_beat)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn add_note_to_clip(&mut self, clip_idx: usize, note: Note) {
        if let Some(clip) = self.clips.get_mut(clip_idx) {
            clip.add_note(note);
        }
    }

    pub fn remove_clip(&mut self, clip_idx: usize) {
        if clip_idx < self.clips.len() {
            self.clips.remove(clip_idx);
        }
    }

    pub fn clear_clips(&mut self) {
        self.clips.clear();
    }

    /// Beat times this note should fire (delegates to clip loop logic).
    pub fn note_trigger_times(
        &self,
        clip: &NoteClip,
        note: &Note,
        total_beats: f64,
    ) -> Vec<f64> {
        clip.note_trigger_times(note, total_beats)
    }

    pub fn notes_in_range(&self, clip_idx: usize, start: f64, end: f64) -> Vec<usize> {
        self.clips
            .get(clip_idx)
            .map(|clip| {
                let rel_start = start - clip.start_beat;
                let rel_end = end - clip.start_beat;
                clip.notes_in_visible_range(rel_start, rel_end)
            })
            .unwrap_or_default()
    }

    pub fn offset_clip_notes(
        &mut self,
        clip_idx: usize,
        indices: &[usize],
        beat_delta: f64,
        pitch_delta: i32,
    ) {
        if let Some(clip) = self.clips.get_mut(clip_idx) {
            for &idx in indices {
                if let Some(note) = clip.notes.get_mut(idx) {
                    note.start_beat = (note.start_beat + beat_delta).max(0.0);
                    if pitch_delta != 0 {
                        let p = note.pitch as i32 + pitch_delta;
                        note.pitch = p.clamp(0, 127) as u8;
                    }
                }
            }
            clip.notes.sort_by(|a, b| {
                a.start_beat
                    .partial_cmp(&b.start_beat)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            clip.recompute_bounds();
        }
    }

    /// Flat note list for drum pattern export (absolute beats).
    pub fn flat_notes(&self) -> Vec<Note> {
        self.clips
            .iter()
            .flat_map(|clip| {
                clip.notes.iter().map(|n| Note {
                    pitch: n.pitch,
                    start_beat: clip.note_absolute_start(n),
                    duration_beats: n.duration_beats,
                    velocity: n.velocity,
                })
            })
            .collect()
    }
}
