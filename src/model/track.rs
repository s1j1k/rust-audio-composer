use super::instrument::InstrumentId;
use super::note::Note;
use super::note_clip::NoteClip;
use super::effects::TrackEffects;
use super::loop_region::LoopRegion;
use super::automation::{AutomationLane, FxParam, TrackLane};
use super::dj::TrackDj;
use super::track_icon::TrackIcon;
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Effects over time (one lane per automated parameter).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub automation: Vec<TrackLane>,
    /// Which automation lane is open under the track on the timeline (None = closed).
    #[serde(default)]
    pub lane_view: Option<FxParam>,
    /// DJ mixer channel and crossfader side.
    #[serde(default)]
    pub dj: TrackDj,
    /// Picked icon; `None` = derive one from the instrument.
    #[serde(default)]
    pub icon: Option<TrackIcon>,
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
            automation: Vec::new(),
            lane_view: None,
            dj: TrackDj::default(),
            icon: None,
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

    pub fn icon(&self) -> TrackIcon {
        self.icon.unwrap_or_else(|| TrackIcon::default_for(&self.instrument))
    }

    /// The automation lane for `param`, if it has any points.
    pub fn lane(&self, param: FxParam) -> Option<&AutomationLane> {
        self.automation
            .iter()
            .find(|l| l.param == param)
            .map(|l| &l.lane)
            .filter(|l| !l.is_empty())
    }

    /// The automation lane for `param`, created empty if missing.
    pub fn lane_mut(&mut self, param: FxParam) -> &mut AutomationLane {
        if let Some(i) = self.automation.iter().position(|l| l.param == param) {
            return &mut self.automation[i].lane;
        }
        self.automation.push(TrackLane {
            param,
            lane: AutomationLane::default(),
        });
        &mut self.automation.last_mut().expect("just pushed").lane
    }

    pub fn clear_lane(&mut self, param: FxParam) {
        self.automation.retain(|l| l.param != param);
    }

    pub fn has_automation(&self) -> bool {
        self.automation.iter().any(|l| !l.lane.is_empty())
    }

    /// Fixed (un-automated) value of a parameter.
    pub fn static_value(&self, param: FxParam) -> f32 {
        param.get(&self.effects, self.volume)
    }

    pub fn set_static_value(&mut self, param: FxParam, v: f32) {
        param.set(&mut self.effects, &mut self.volume, v);
    }

    /// Value of a parameter at `beat`: the automation if the lane has points, else the
    /// fixed value.
    pub fn value_at(&self, param: FxParam, beat: f64) -> f32 {
        self.lane(param)
            .and_then(|l| l.value_at(beat))
            .map(|v| param.clamp(v))
            .unwrap_or_else(|| self.static_value(param))
    }

    /// Effects and fader level at `beat`, with automation applied.
    pub fn mix_at(&self, beat: f64) -> (TrackEffects, f32) {
        let mut fx = self.effects;
        let mut volume = self.volume;
        for l in &self.automation {
            if let Some(v) = l.lane.value_at(beat) {
                l.param.set(&mut fx, &mut volume, v);
            }
        }
        (fx, volume)
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
