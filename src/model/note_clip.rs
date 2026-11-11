use super::note::Note;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteClip {
    pub id: u64,
    pub name: String,
    /// Timeline position where the clip starts.
    pub start_beat: f64,
    /// Full content length (one loop iteration before trim).
    pub content_length: f64,
    /// Original content length — used to recover trimmed content when extending.
    pub source_length: f64,
    /// Non-destructive trim from the start of content.
    pub trim_start: f64,
    /// Non-destructive trim from the end of content.
    pub trim_end: f64,
    pub loop_enabled: bool,
    /// How far the clip extends on the timeline when looping.
    pub loop_end_beat: f64,
    /// Notes with `start_beat` relative to clip content origin (0).
    pub notes: Vec<Note>,
}

impl NoteClip {
    pub fn new(id: u64, name: String, start_beat: f64) -> Self {
        Self {
            id,
            name,
            start_beat,
            content_length: 4.0,
            source_length: 4.0,
            trim_start: 0.0,
            trim_end: 0.0,
            loop_enabled: false,
            loop_end_beat: start_beat + 4.0,
            notes: Vec::new(),
        }
    }

    pub fn from_legacy_notes(id: u64, notes: Vec<Note>) -> Self {
        let mut clip = Self::new(id, "Imported".to_string(), 0.0);
        if notes.is_empty() {
            return clip;
        }
        let min_start = notes.iter().map(|n| n.start_beat).fold(f64::MAX, f64::min);
        clip.start_beat = min_start;
        clip.notes = notes
            .into_iter()
            .map(|mut n| {
                n.start_beat -= min_start;
                n
            })
            .collect();
        clip.recompute_bounds();
        clip
    }

    pub fn visible_length(&self) -> f64 {
        (self.content_length - self.trim_start - self.trim_end).max(0.25)
    }

    pub fn visible_content_end(&self) -> f64 {
        self.content_length - self.trim_end
    }

    pub fn timeline_end(&self) -> f64 {
        if self.loop_enabled {
            self.loop_end_beat.max(self.start_beat + self.visible_length())
        } else {
            self.start_beat + self.visible_length()
        }
    }

    pub fn note_is_visible(&self, note: &Note) -> bool {
        let rel_end = note.end_beat();
        note.start_beat >= self.trim_start - 0.001
            && rel_end <= self.visible_content_end() + 0.001
    }

    pub fn note_absolute_start(&self, note: &Note) -> f64 {
        self.start_beat + note.start_beat
    }

    pub fn note_absolute_end(&self, note: &Note) -> f64 {
        self.start_beat + note.end_beat()
    }

    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
        self.notes.sort_by(|a, b| {
            a.start_beat
                .partial_cmp(&b.start_beat)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.recompute_bounds();
    }

    pub fn recompute_bounds(&mut self) {
        if self.notes.is_empty() {
            return;
        }
        let content_end = self
            .notes
            .iter()
            .map(|n| n.end_beat())
            .fold(0.0_f64, f64::max)
            .max(0.25);
        let bar = 4.0_f64;
        let rounded = (content_end / bar).ceil() * bar;
        self.source_length = self.source_length.max(rounded);
        self.content_length = self.source_length;
        if !self.loop_enabled {
            self.loop_end_beat = self.start_beat + self.visible_length();
        } else {
            self.loop_end_beat = self.loop_end_beat.max(self.start_beat + self.visible_length());
        }
    }

    pub fn finalize_recording(&mut self) {
        self.recompute_bounds();
        if self.name == "Recording…" {
            self.name = format!("Take {}", self.id);
        }
    }

    /// Beat times this note should fire, including loop repetitions.
    pub fn note_trigger_times(&self, note: &Note, total_beats: f64) -> Vec<f64> {
        if !self.note_is_visible(note) {
            return vec![];
        }
        let abs_start = self.note_absolute_start(note);
        if !self.loop_enabled {
            if abs_start < total_beats + 0.001 {
                return vec![abs_start];
            }
            return vec![];
        }

        let loop_len = self.visible_length();
        if loop_len < 0.01 {
            return vec![];
        }

        let loop_start = self.start_beat;
        let loop_timeline_end = self.loop_end_beat;
        let note_offset = note.start_beat - self.trim_start;

        let mut times = Vec::new();
        let mut iteration = 0.0_f64;
        loop {
            let trigger = loop_start + iteration * loop_len + note_offset;
            if trigger >= loop_timeline_end - 0.001 {
                break;
            }
            if trigger < total_beats + 0.001 {
                times.push(trigger);
            }
            iteration += 1.0;
            if iteration > 10_000.0 {
                break;
            }
        }
        times
    }

    pub fn offset_on_timeline(&mut self, beat_delta: f64) {
        self.start_beat = (self.start_beat + beat_delta).max(0.0);
        if self.loop_enabled {
            self.loop_end_beat = (self.loop_end_beat + beat_delta).max(self.start_beat + self.visible_length());
        }
    }

    pub fn set_trim_start(&mut self, trim: f64) {
        let max_trim = (self.source_length - self.trim_end - 0.25).max(0.0);
        self.trim_start = trim.clamp(0.0, max_trim);
        self.content_length = self.source_length;
    }

    pub fn set_trim_end(&mut self, trim: f64) {
        let max_trim = (self.source_length - self.trim_start - 0.25).max(0.0);
        self.trim_end = trim.clamp(0.0, max_trim);
        self.content_length = self.source_length;
    }

    pub fn set_loop_end(&mut self, end_beat: f64) {
        self.loop_end_beat = end_beat.max(self.start_beat + self.visible_length());
    }

    pub fn notes_in_visible_range(&self, rel_start: f64, rel_end: f64) -> Vec<usize> {
        self.notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.start_beat >= rel_start && n.start_beat < rel_end)
            .map(|(i, _)| i)
            .collect()
    }
}
