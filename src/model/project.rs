use super::{song_section::SongSection, track::Track, Note};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub tracks: Vec<Track>,
    pub bpm: f32,
    pub time_sig_numerator: u8,
    pub time_sig_denominator: u8,
    pub playhead_beat: f64,
    pub total_beats: f64,
    #[serde(default)]
    pub sections: Vec<SongSection>,
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
            sections: Vec::new(),
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

    pub fn beats_per_bar(&self) -> f64 {
        self.time_sig_numerator as f64
    }

    pub fn bar_count(&self) -> u32 {
        (self.total_beats / self.beats_per_bar()).ceil() as u32
    }

    pub fn ensure_length_for_sections(&mut self) {
        let beats_per_bar = self.beats_per_bar();
        let section_end = self
            .sections
            .iter()
            .map(|s| s.end_beat(beats_per_bar))
            .fold(0.0_f64, f64::max);
        if section_end > self.total_beats {
            self.total_beats = section_end.ceil();
        }
    }

    pub fn add_section(&mut self, id: u64, label: String, start_beat: f64, bar_count: u32) {
        let section = SongSection::new(id, label, start_beat, bar_count);
        self.sections.push(section);
        self.sections.sort_by(|a, b| {
            a.start_beat
                .partial_cmp(&b.start_beat)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.ensure_length_for_sections();
    }

    pub fn remove_section(&mut self, id: u64) {
        self.sections.retain(|s| s.id != id);
    }

    pub fn section_after_last(&self) -> f64 {
        let beats_per_bar = self.beats_per_bar();
        self.sections
            .iter()
            .map(|s| s.end_beat(beats_per_bar))
            .fold(0.0_f64, f64::max)
    }

    pub fn beat_duration_secs(&self) -> f64 {
        60.0 / self.bpm as f64
    }

    pub fn duration_secs(&self) -> f64 {
        self.total_beats * self.beat_duration_secs()
    }

    pub fn all_notes(&self) -> Vec<Note> {
        self.tracks
            .iter()
            .flat_map(|t| t.flat_notes())
            .collect()
    }

    pub fn migrate_all_tracks(&mut self, next_id: &mut u64) {
        for track in &mut self.tracks {
            track.ensure_clips_migrated(next_id);
        }
    }
}
