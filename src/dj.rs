//! DJ mode logic (no drawing): how a moved control changes the project, recording knob
//! moves as automation, scrubbing from the platters, and live mixdown recording.
//!
//! Rules for a moved control, from any surface (desktop knob, phone, ...):
//! * WRITE armed and the song playing → the move is recorded into the parameter's
//!   automation lane at the playhead ("touch" style: it overwrites while you hold it).
//! * Otherwise, if the parameter already has automation → the point at the playhead is
//!   set (so you can stop, move the playhead, turn a knob, and shape a curve).
//! * Otherwise → the control's fixed value changes, like a normal mixer.

use crate::app::DAWApp;
use crate::audio::engine::BusMix;
use crate::model::automation::{AutomationLane, FxParam, RECORD_RESOLUTION};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Something a DJ control can move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DjTarget {
    Track(usize, FxParam),
    Crossfader,
}

/// A phone finger resting on a control (sending nothing) still "holds" it this long.
const REMOTE_HOLD: Duration = Duration::from_millis(300);

#[derive(Default)]
pub struct DjState {
    /// Record control moves as automation while playing.
    pub write: bool,
    /// Last recorded beat for each control currently held (a gesture in progress).
    gestures: HashMap<DjTarget, f64>,
    touched_this_frame: HashSet<DjTarget>,
    /// Remote-held controls: last value and when it was sent.
    remote_holds: HashMap<DjTarget, (f32, Instant)>,
    /// A platter is held: the playhead follows the hand instead of the clock.
    pub scrubbing: bool,
    /// Post-fader peak per track and on the master, refreshed every frame for meters.
    pub levels: Vec<f32>,
    pub master_level: f32,
    /// Live mixdown in progress (when it started).
    pub mix_rec_since: Option<Instant>,
    /// Recording the mixdown started the transport (so stopping it stops playback).
    mix_rec_started_transport: bool,
    /// Where the last mixdown went.
    pub last_mixdown: Option<PathBuf>,
    /// Length of the one-click crossfade (0 = default of 4 bars).
    pub fade_bars: u32,
    /// DJ mode is in its own window instead of the bottom panel.
    pub popped_out: bool,
}

impl DjState {
    pub fn is_held(&self, target: DjTarget) -> bool {
        self.gestures.contains_key(&target)
    }
}

impl DAWApp {
    /// Current value of a control at the playhead (automation if any, else fixed value).
    pub fn dj_value(&self, target: DjTarget) -> f32 {
        let beat = self.project.playhead_beat;
        match target {
            DjTarget::Track(i, p) => self
                .project
                .tracks
                .get(i)
                .map(|t| t.value_at(p, beat))
                .unwrap_or_else(|| p.neutral()),
            DjTarget::Crossfader => self.project.dj.crossfader_at(beat),
        }
    }

    pub fn dj_has_automation(&self, target: DjTarget) -> bool {
        self.lane_ref(target).map(|l| !l.is_empty()).unwrap_or(false)
    }

    fn lane_ref(&self, target: DjTarget) -> Option<&AutomationLane> {
        match target {
            DjTarget::Track(i, p) => self.project.tracks.get(i).and_then(|t| t.lane(p)),
            DjTarget::Crossfader => Some(&self.project.dj.crossfader_lane),
        }
    }

    fn lane_for(&mut self, target: DjTarget) -> Option<&mut AutomationLane> {
        match target {
            DjTarget::Track(i, p) => self.project.tracks.get_mut(i).map(|t| t.lane_mut(p)),
            DjTarget::Crossfader => Some(&mut self.project.dj.crossfader_lane),
        }
    }

    fn set_fixed(&mut self, target: DjTarget, v: f32) {
        match target {
            DjTarget::Track(i, p) => {
                if let Some(t) = self.project.tracks.get_mut(i) {
                    t.set_static_value(p, v);
                }
            }
            DjTarget::Crossfader => self.project.dj.crossfader = v.clamp(-1.0, 1.0),
        }
    }

    fn clamp_target(target: DjTarget, v: f32) -> f32 {
        match target {
            DjTarget::Track(_, p) => p.clamp(v),
            DjTarget::Crossfader => v.clamp(-1.0, 1.0),
        }
    }

    /// The user is moving (or holding) a control at value `v` this frame.
    pub fn dj_set(&mut self, target: DjTarget, v: f32) {
        let v = Self::clamp_target(target, v);
        let beat = self.project.playhead_beat;
        self.dj.touched_this_frame.insert(target);

        if self.playing && self.dj.write {
            let prev = self.dj.gestures.get(&target).copied();
            let fixed = self.dj_value_fixed(target);
            let Some(lane) = self.lane_for(target) else { return };
            match prev {
                // Continuing a gesture (and the playhead didn't wrap or jump).
                Some(prev) if prev <= beat && beat - prev < 2.0 => lane.record_step(prev, beat, v),
                _ => {
                    // New gesture: pin what was there just before, so only the part you
                    // touch changes.
                    let before = lane.value_at(beat).unwrap_or(fixed);
                    let anchor = beat - RECORD_RESOLUTION;
                    if anchor > 0.0 {
                        lane.clear_range(anchor - 1e-9, beat);
                        lane.set_point(anchor, before);
                    }
                    lane.set_point(beat, v);
                }
            }
            self.dj.gestures.insert(target, beat);
        } else if self.dj_has_automation(target) {
            if let Some(lane) = self.lane_for(target) {
                lane.set_point(beat, v);
            }
        } else {
            self.set_fixed(target, v);
        }
    }

    fn dj_value_fixed(&self, target: DjTarget) -> f32 {
        match target {
            DjTarget::Track(i, p) => self
                .project
                .tracks
                .get(i)
                .map(|t| t.static_value(p))
                .unwrap_or_else(|| p.neutral()),
            DjTarget::Crossfader => self.project.dj.crossfader,
        }
    }

    /// A phone moved a control. Its finger keeps holding it briefly between messages.
    pub fn dj_set_remote(&mut self, target: DjTarget, v: f32) {
        self.dj.remote_holds.insert(target, (v, Instant::now()));
        self.dj_set(target, v);
    }

    /// Remove all automation from a control (it keeps its current value as fixed value).
    pub fn dj_clear_automation(&mut self, target: DjTarget) {
        let v = self.dj_value(target);
        match target {
            DjTarget::Track(i, p) => {
                if let Some(t) = self.project.tracks.get_mut(i) {
                    t.clear_lane(p);
                }
            }
            DjTarget::Crossfader => self.project.dj.crossfader_lane = AutomationLane::default(),
        }
        self.set_fixed(target, v);
    }

    /// Write a crossfader move from the playhead to `to` over `bars` bars as automation:
    /// a one-click, beat-exact transition for beginners.
    pub fn dj_auto_crossfade(&mut self, to: f32, bars: f64) {
        let start = self.project.playhead_beat;
        let end = (start + bars * self.project.beats_per_bar()).min(self.project.total_beats);
        let from = self.project.dj.crossfader_at(start);
        let lane = &mut self.project.dj.crossfader_lane;
        let after = lane.value_at(end + 1e-6);
        lane.clear_range(start - 1e-9, end);
        if start > 0.0 {
            lane.set_point((start - RECORD_RESOLUTION).max(0.0), from);
        }
        lane.set_point(start, from);
        lane.set_point(end, to.clamp(-1.0, 1.0));
        // Hold the new side afterwards unless later automation already says otherwise.
        if after.is_none() {
            self.project.dj.crossfader = to;
        }
        self.status_error = None;
        self.status_message = Some(format!(
            "Crossfade to {} written over {} bars — press play to hear it",
            if to < 0.0 { "A" } else { "B" },
            bars
        ));
    }

    /// Called once per frame before drawing: end gestures nobody is holding any more,
    /// keep phone holds alive, push the mix to the audio engine and read the meters.
    pub fn dj_frame(&mut self) {
        // Phone fingers resting on a control.
        let now = Instant::now();
        self.dj
            .remote_holds
            .retain(|_, (_, at)| now.duration_since(*at) < REMOTE_HOLD);
        let holds: Vec<(DjTarget, f32)> = self
            .dj
            .remote_holds
            .iter()
            .filter(|(t, _)| !self.dj.touched_this_frame.contains(*t))
            .map(|(t, (v, _))| (*t, *v))
            .collect();
        for (t, v) in holds {
            self.dj_set(t, v);
        }
        // Released controls end their gesture.
        let touched = std::mem::take(&mut self.dj.touched_this_frame);
        self.dj.gestures.retain(|t, _| touched.contains(t));

        // Mix to the engine.
        let beat = self.project.playhead_beat;
        let mixes: Vec<BusMix> = (0..self.project.tracks.len())
            .filter_map(|i| self.project.track_mix_at(i, beat))
            .map(|(fx, gain)| BusMix { fx, gain })
            .collect();
        let live = (self.current_track < mixes.len()).then_some(self.current_track);
        let bpm = self.project.bpm;
        let mut finished_capture = None;
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.sync_tracks(&mixes, bpm);
            eng.live_track = live;
            self.dj.levels = eng.bus_levels();
            self.dj.master_level = eng.master_level();
            if self.dj.mix_rec_since.is_some() && !eng.is_capturing() {
                finished_capture = Some(());
            }
        }
        if finished_capture.is_some() {
            // Hit the length limit inside the engine.
            self.stop_mix_recording();
        }
    }

    /// Hold / release a platter. Holding stops the playhead like a hand on vinyl.
    pub fn dj_platter_hold(&mut self, held: bool) {
        if held && !self.dj.scrubbing {
            if let Ok(mut eng) = self.audio_engine.lock() {
                eng.stop_all();
            }
        }
        if !held && self.dj.scrubbing {
            // Resume from where the hand left it without a time jump.
            self.last_frame = Instant::now();
        }
        self.dj.scrubbing = held;
    }

    /// Move the playhead by a platter turn (in beats).
    pub fn dj_scrub(&mut self, beats: f64) {
        let total = self.project.total_beats;
        let mut b = self.project.playhead_beat + beats;
        if b < 0.0 {
            b = 0.0;
        }
        if b > total {
            b = total;
        }
        self.set_playhead(b);
    }

    pub fn is_mix_recording(&self) -> bool {
        self.dj.mix_rec_since.is_some()
    }

    /// REC MIX: capture exactly what you hear (all tracks, effects, your live moves) to a
    /// WAV file. Starts playback if needed.
    pub fn toggle_mix_recording(&mut self) {
        if self.is_mix_recording() {
            self.stop_mix_recording();
        } else {
            self.start_mix_recording();
        }
    }

    pub fn start_mix_recording(&mut self) {
        if self.is_mix_recording() {
            return;
        }
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.start_capture();
        }
        self.dj.mix_rec_since = Some(Instant::now());
        self.dj.mix_rec_started_transport = !self.playing;
        if !self.playing {
            self.toggle_playback();
        }
        self.status_error = None;
        self.status_message = Some("Recording the mix — press REC MIX again to stop".into());
    }

    pub fn stop_mix_recording(&mut self) {
        if self.dj.mix_rec_since.take().is_none() {
            return;
        }
        let (samples, sr) = match self.audio_engine.lock() {
            Ok(mut eng) => (eng.stop_capture().unwrap_or_default(), eng.sample_rate()),
            Err(_) => (Vec::new(), 44_100.0),
        };
        if self.dj.mix_rec_started_transport && self.playing {
            self.toggle_playback();
        }
        self.dj.mix_rec_started_transport = false;
        if samples.len() < (sr * 0.2) as usize {
            self.status_message = Some("Mix recording was too short — nothing saved".into());
            return;
        }
        let secs = samples.len() as f32 / sr;
        let path = rfd::FileDialog::new()
            .add_filter("WAV Audio", &["wav"])
            .set_file_name("live-mix.wav")
            .save_file()
            .or_else(|| default_mixdown_path().ok());
        match path {
            Some(path) => match crate::audio::export::write_wav(&path, &samples, sr) {
                Ok(()) => {
                    self.status_error = None;
                    self.status_message = Some(format!(
                        "Saved {:.0}:{:02.0} live mix to {}",
                        (secs / 60.0).floor(),
                        secs % 60.0,
                        path.display()
                    ));
                    self.dj.last_mixdown = Some(path);
                }
                Err(e) => self.status_error = Some(format!("Couldn't save the mix: {e}")),
            },
            None => self.status_error = Some("Couldn't find a folder to save the mix".into()),
        }
    }

    pub fn mix_recording_secs(&self) -> f32 {
        self.dj
            .mix_rec_since
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0)
    }
}

impl DAWApp {
    /// The DJ mixer as the phone remote shows it.
    pub fn dj_remote_snapshot(&self) -> crate::remote::protocol::DjSnapshot {
        use crate::model::DJ_CHANNELS;
        use crate::remote::protocol::{DjChannel, DjSnapshot, DjTrack};
        let hex = |c: [u8; 3]| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]);
        let round = |v: f32| (v * 1000.0).round() / 1000.0;
        const PHONE_PARAMS: [FxParam; 7] = [
            FxParam::Volume,
            FxParam::EqHigh,
            FxParam::EqMid,
            FxParam::EqLow,
            FxParam::Filter,
            FxParam::Echo,
            FxParam::Reverb,
        ];
        let channels = (0..DJ_CHANNELS)
            .map(|ch| match self.project.track_on_channel(ch) {
                Some(i) => {
                    let t = &self.project.tracks[i];
                    DjChannel {
                        ch: ch as u8,
                        track: Some(i),
                        name: t.name.clone(),
                        icon: t.icon().key().to_string(),
                        glyph: t.icon().glyph().to_string(),
                        color: hex(t.color),
                        side: t.dj.side.key().to_string(),
                        muted: t.muted,
                        vals: PHONE_PARAMS
                            .iter()
                            .map(|&p| (p.key().to_string(), round(self.dj_value(DjTarget::Track(i, p)))))
                            .collect(),
                        auto: PHONE_PARAMS
                            .iter()
                            .filter(|&&p| t.lane(p).is_some())
                            .map(|p| p.key().to_string())
                            .collect(),
                    }
                }
                None => DjChannel {
                    ch: ch as u8,
                    track: None,
                    name: String::new(),
                    icon: String::new(),
                    glyph: String::new(),
                    color: "#5a5c66".into(),
                    side: "thru".into(),
                    muted: false,
                    vals: Vec::new(),
                    auto: Vec::new(),
                },
            })
            .collect();
        let tracks = self
            .project
            .tracks
            .iter()
            .enumerate()
            .map(|(i, t)| DjTrack {
                i,
                name: t.name.clone(),
                icon: t.icon().key().to_string(),
                glyph: t.icon().glyph().to_string(),
                color: hex(t.color),
            })
            .collect();
        DjSnapshot {
            channels,
            tracks,
            xf: round(self.dj_value(DjTarget::Crossfader)),
            xf_auto: self.dj_has_automation(DjTarget::Crossfader),
            playing: self.playing,
            write: self.dj.write,
            mixrec: self.is_mix_recording(),
            mixrec_secs: self.mix_recording_secs() as u32,
        }
    }
}

/// Fallback when the save dialog is cancelled: never throw a performance away.
fn default_mixdown_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or("no home folder")?;
    let dir = base.join("Music").join("Rust Audio Composer");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(dir.join(format!("live-mix-{stamp}.wav")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> DAWApp {
        let mut app = DAWApp::build(false);
        app.project.playhead_beat = 4.0;
        app
    }

    #[test]
    fn moving_a_knob_without_write_sets_the_fixed_value() {
        let mut a = app();
        let t = DjTarget::Track(0, FxParam::Filter);
        a.dj_set(t, -0.5);
        assert_eq!(a.project.tracks[0].effects.filter, -0.5);
        assert!(!a.dj_has_automation(t));
    }

    #[test]
    fn write_records_a_gesture_and_release_ends_it() {
        let mut a = app();
        a.playing = true;
        a.dj.write = true;
        let t = DjTarget::Track(0, FxParam::Echo);
        for step in 0..120 {
            a.project.playhead_beat = 4.0 + step as f64 / 30.0; // 4 beats of gesture
            a.dj_set(t, step as f32 / 120.0);
            a.dj_frame();
        }
        assert!(a.dj_has_automation(t));
        let track = &a.project.tracks[0];
        // Before the gesture: the old fixed value (0); during: the ramp.
        assert!(track.value_at(FxParam::Echo, 1.0).abs() < 1e-6);
        assert!((track.value_at(FxParam::Echo, 6.0) - 0.5).abs() < 0.05);
        // A frame without touching ends the gesture.
        a.dj_frame();
        assert!(!a.dj.is_held(t));
    }

    #[test]
    fn with_automation_a_knob_edits_the_point_at_the_playhead() {
        let mut a = app();
        let t = DjTarget::Crossfader;
        a.project.dj.crossfader_lane.set_point(0.0, -1.0);
        a.project.playhead_beat = 8.0;
        a.dj_set(t, 1.0);
        assert_eq!(a.project.dj.crossfader_at(8.0), 1.0);
        assert_eq!(a.project.dj.crossfader_at(0.0), -1.0);
        a.dj_clear_automation(t);
        assert!(!a.dj_has_automation(t));
        assert_eq!(a.project.dj.crossfader, 1.0); // keeps the value it had
    }

    #[test]
    fn auto_crossfade_writes_a_ramp() {
        let mut a = app();
        a.project.playhead_beat = 8.0;
        a.dj_auto_crossfade(1.0, 2.0); // 2 bars of 4/4 = 8 beats
        assert!((a.project.dj.crossfader_at(12.0) - 0.5).abs() < 1e-3);
        assert_eq!(a.project.dj.crossfader_at(20.0), 1.0);
        assert_eq!(a.project.dj.crossfader_at(2.0), 0.0);
    }
}
