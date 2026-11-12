use super::dj::{crossfader_gain, DjMix};
use super::effects::TrackEffects;
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
    #[serde(default)]
    pub dj: DjMix,
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
            dj: DjMix::default(),
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

    /// Everything that shapes track `index`'s sound at `beat`: its effects and the final
    /// post-fader gain (fader x crossfader x mute).
    pub fn track_mix_at(&self, index: usize, beat: f64) -> Option<(TrackEffects, f32)> {
        let track = self.tracks.get(index)?;
        let (fx, volume) = track.mix_at(beat);
        let gain = if track.muted {
            0.0
        } else {
            volume * crossfader_gain(track.dj.side, self.dj.crossfader_at(beat))
        };
        Some((fx, gain))
    }

    /// Track on DJ channel `ch`, if one is loaded.
    pub fn track_on_channel(&self, ch: usize) -> Option<usize> {
        self.tracks
            .iter()
            .position(|t| t.dj.channel == Some(ch as u8))
    }

    /// Load track `index` onto DJ channel `ch` (clearing whatever was there), or unload
    /// the channel with `None`.
    pub fn assign_channel(&mut self, ch: usize, index: Option<usize>) {
        for t in &mut self.tracks {
            if t.dj.channel == Some(ch as u8) {
                t.dj.channel = None;
            }
        }
        if let Some(t) = index.and_then(|i| self.tracks.get_mut(i)) {
            t.dj.channel = Some(ch as u8);
        }
    }

    /// Put the first tracks on empty DJ channels (first two on A, next two on B), so DJ
    /// mode is useful the moment it opens.
    pub fn auto_assign_channels(&mut self) {
        if self.tracks.iter().any(|t| t.dj.channel.is_some()) {
            return;
        }
        use super::dj::{XfadeSide, DJ_CHANNELS};
        let n = self.tracks.len().min(DJ_CHANNELS);
        for (i, t) in self.tracks.iter_mut().take(DJ_CHANNELS).enumerate() {
            t.dj.channel = Some(i as u8);
            // Two tracks: one each side. More: first half on A, the rest on B.
            t.dj.side = if i < n.div_ceil(2) { XfadeSide::A } else { XfadeSide::B };
        }
    }

    pub fn migrate_all_tracks(&mut self, next_id: &mut u64) {
        for track in &mut self.tracks {
            track.ensure_clips_migrated(next_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::automation::FxParam;
    use crate::model::dj::XfadeSide;

    #[test]
    fn track_mix_applies_automation_crossfader_and_mute() {
        let mut p = Project::default();
        p.tracks[0].volume = 0.5;
        p.tracks[0].dj.side = XfadeSide::A;
        let lane = p.tracks[0].lane_mut(FxParam::Filter);
        lane.set_point(0.0, 0.0);
        lane.set_point(8.0, -1.0);

        let (fx, gain) = p.track_mix_at(0, 4.0).unwrap();
        assert!((fx.filter + 0.5).abs() < 1e-6);
        assert!((gain - 0.5).abs() < 1e-6);

        p.dj.crossfader = 1.0; // full B: a track on A is silent
        assert!(p.track_mix_at(0, 4.0).unwrap().1 < 1e-6);
        p.dj.crossfader = 0.0;
        p.tracks[0].muted = true;
        assert_eq!(p.track_mix_at(0, 4.0).unwrap().1, 0.0);
    }

    #[test]
    fn channel_assignment_is_exclusive() {
        let mut p = Project::default();
        p.assign_channel(0, Some(0));
        p.assign_channel(0, Some(1));
        assert_eq!(p.track_on_channel(0), Some(1));
        assert_eq!(p.tracks[0].dj.channel, None);
        p.assign_channel(0, None);
        assert_eq!(p.track_on_channel(0), None);
    }

    #[test]
    fn auto_assign_splits_sides() {
        let mut p = Project::default(); // two tracks
        p.auto_assign_channels();
        assert_eq!(p.tracks[0].dj.side, XfadeSide::A);
        assert_eq!(p.tracks[1].dj.side, XfadeSide::B);
        assert_eq!(p.track_on_channel(1), Some(1));
    }

    #[test]
    fn automation_survives_save_and_load() {
        let mut p = Project::default();
        p.tracks[1].lane_mut(FxParam::Echo).set_point(2.0, 0.7);
        p.dj.crossfader_lane.set_point(4.0, 1.0);
        let json = serde_json::to_string(&p).unwrap();
        let back: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(back.tracks[1].lane(FxParam::Echo), p.tracks[1].lane(FxParam::Echo));
        assert_eq!(back.dj, p.dj);
    }
}
