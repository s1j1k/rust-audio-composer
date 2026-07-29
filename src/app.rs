use crate::audio::export::export_project_wav;
use crate::audio::AudioEngine;
use crate::model::drum::{beat_to_step, step_to_beat, DrumKind, SEQUENCER_STEPS};
use crate::model::instrument::{CustomInstrument, InstrumentId, InstrumentProfile, SampleKind};
use crate::model::{Note, NoteClip, Project, SavedDrumPattern, SavedProject};
use crate::music::Key;
use crate::music::theory::NOTE_NAMES;
use crate::ui::theme::UiFontFamily;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteDragMode {
    Move,
    Resize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipDragMode {
    Move,
    TrimStart,
    TrimEnd,
    ExtendLoop,
}

#[derive(Clone, Copy, Debug)]
pub struct NoteDragState {
    pub clip_index: usize,
    pub note_index: usize,
    pub mode: NoteDragMode,
    pub grab_beat: f64,
    pub grab_pitch: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct ClipDragState {
    pub track_index: usize,
    pub clip_index: usize,
    pub mode: ClipDragMode,
    pub grab_beat: f64,
    pub orig_start: f64,
    pub orig_trim_start: f64,
    pub orig_trim_end: f64,
    pub orig_loop_end: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct HeldNoteState {
    pub press_start: Instant,
    pub peak_velocity: f32,
    /// `(track_idx, clip_idx, note start within clip)` when recorded live.
    pub recording_anchor: Option<(usize, usize, f64)>,
}

pub struct DAWApp {
    pub project: Project,
    pub audio: AudioEngine,
    pub audio_stream: Option<Stream>,
    pub audio_engine: Arc<Mutex<AudioEngine>>,

    pub playing: bool,
    pub recording: bool,
    pub current_track: usize,

    pub show_piano: bool,
    pub show_composition: bool,
    pub show_instrument_designer: bool,
    pub show_drum_sequencer: bool,
    pub show_dj_panel: bool,
    pub show_options: bool,
    pub show_user_guide: bool,

    pub ui_font: UiFontFamily,
    pub ui_hints_expanded: bool,

    pub held_notes: HashSet<u8>,
    pub held_note_details: HashMap<u8, HeldNoteState>,
    pub playback_notes_active: HashSet<u8>,
    pub playback_events_done: HashSet<u32>,
    pub last_frame: Instant,
    pub playback_start: Instant,

    pub detected_key: Option<Key>,
    pub composition_hint: Option<String>,

    pub instrument_name_input: String,
    pub instrument_description_input: String,
    pub instrument_gen_result: Option<String>,
    pub pending_custom_instrument: Option<CustomInstrument>,
    pub custom_instruments: Vec<CustomInstrument>,

    pub selected_instrument: String,
    pub ai_base_kind: SampleKind,
    pub status_message: Option<String>,
    pub status_error: Option<String>,
    pub project_file_path: Option<String>,

    pub selected_note: Option<(usize, usize)>,
    pub selected_clip: Option<usize>,
    pub note_drag: Option<NoteDragState>,
    pub clip_drag: Option<ClipDragState>,
    pub next_clip_id: u64,
    pub recording_clip: Option<(usize, usize)>,
    pub section_range: Option<(f64, f64)>,
    pub section_select_drag: Option<f64>,
    pub section_drag: Option<(f64, f64)>,
    pub section_note_indices: Option<Vec<usize>>,
    pub section_drag_origins: HashMap<usize, f64>,
    pub section_clip_index: Option<usize>,
    pub piano_roll_grid_rect: Option<egui::Rect>,
    pub timeline_grid_origin_x: Option<f32>,
    pub piano_roll_octave_base: u8,
    pub piano_roll_octave_count: u8,
    pub piano_keyboard_octaves: u8,
    pub piano_keyboard_base: u8,
    pub keyboard_mapping_minimized: bool,
    pub keyboard_mapping_seen: bool,

    pub saved_drum_patterns: Vec<SavedDrumPattern>,
    pub drum_pattern_name_input: String,
    pub selected_pattern_index: usize,

    pub timeline_zoom: f32,
    pub next_section_id: u64,
    pub new_section_label: String,
    pub new_section_bars: u32,
    pub selected_song_section: Option<usize>,

    /// In-memory beat sequencer pattern (not written to clips unless recording).
    pub sequencer_draft: Vec<(DrumKind, u8, f32)>,
    pub sequencer_draft_bar_start: f64,
    pub sequencer_draft_track: Option<usize>,
    pub sequencer_running: bool,
    pub sequencer_loop_beat: f64,
    pub sequencer_preview_step: u8,
    pub sequencer_last_frame: Instant,
}

impl Default for DAWApp {
    fn default() -> Self {
        let mut app = Self {
            project: Project::default(),
            audio: AudioEngine::new(44100.0),
            audio_stream: None,
            audio_engine: Arc::new(Mutex::new(AudioEngine::new(44100.0))),

            playing: false,
            recording: false,
            current_track: 0,

            show_piano: false,
            show_composition: true,
            show_instrument_designer: false,
            show_drum_sequencer: false,
            show_dj_panel: false,
            show_options: false,
            show_user_guide: false,

            ui_font: UiFontFamily::default(),
            ui_hints_expanded: false,

            held_notes: HashSet::new(),
            held_note_details: HashMap::new(),
            playback_notes_active: HashSet::new(),
            playback_events_done: HashSet::new(),
            last_frame: Instant::now(),
            playback_start: Instant::now(),

            detected_key: None,
            composition_hint: None,

            instrument_name_input: String::new(),
            instrument_description_input: String::from(
                "warm mellow piano with gentle attack",
            ),
            instrument_gen_result: None,
            pending_custom_instrument: None,
            custom_instruments: Vec::new(),

            selected_instrument: "Piano".to_string(),
            ai_base_kind: SampleKind::Piano,
            status_message: None,
            status_error: None,
            project_file_path: None,

            selected_note: None,
            selected_clip: None,
            note_drag: None,
            clip_drag: None,
            next_clip_id: 1,
            recording_clip: None,
            section_range: None,
            section_select_drag: None,
            section_drag: None,
            section_note_indices: None,
            section_drag_origins: HashMap::new(),
            section_clip_index: None,
            piano_roll_grid_rect: None,
            timeline_grid_origin_x: None,
            piano_roll_octave_base: 48,
            piano_roll_octave_count: 3,
            piano_keyboard_octaves: 2,
            piano_keyboard_base: 48,
            keyboard_mapping_minimized: false,
            keyboard_mapping_seen: false,

            saved_drum_patterns: Vec::new(),
            drum_pattern_name_input: String::from("Pattern 1"),
            selected_pattern_index: 0,

            timeline_zoom: 1.0,
            next_section_id: 1,
            new_section_label: String::from("A"),
            new_section_bars: 8,
            selected_song_section: None,

            sequencer_draft: Vec::new(),
            sequencer_draft_bar_start: 0.0,
            sequencer_draft_track: None,
            sequencer_running: false,
            sequencer_loop_beat: 0.0,
            sequencer_preview_step: u8::MAX,
            sequencer_last_frame: Instant::now(),
        };

        if let Ok(stream) = app.setup_audio_stream() {
            app.audio_stream = Some(stream);
        }

        app.sync_instrument_to_track();
        app.migrate_tracks();
        app
    }
}

impl DAWApp {
    fn setup_audio_stream(&mut self) -> Result<Stream, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        let config = device.default_output_config()?;
        let sample_rate = config.sample_rate().0 as f32;

        let engine = Arc::new(Mutex::new(AudioEngine::new(sample_rate)));
        self.audio_engine = engine.clone();
        self.audio = AudioEngine::new(sample_rate);

        let stream = device.build_output_stream(
            &config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(mut eng) = engine.lock() {
                    for sample in data.iter_mut() {
                        *sample = eng.next_sample();
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        stream.play()?;
        Ok(stream)
    }

    pub fn migrate_tracks(&mut self) {
        self.project.migrate_all_tracks(&mut self.next_clip_id);
    }

    pub fn piano_roll_low(&self) -> u8 {
        self.piano_roll_octave_base
    }

    pub fn piano_roll_high(&self) -> u8 {
        (self.piano_roll_octave_base as u16 + self.piano_roll_octave_count as u16 * 12 + 11)
            .min(127) as u8
    }

    pub fn adjust_piano_roll_pitch(&mut self, semitones: i32) {
        if semitones == 0 {
            return;
        }
        let span = self.piano_roll_octave_count as i32 * 12 + 11;
        let max_base = (127 - span).max(0);
        self.piano_roll_octave_base = (self.piano_roll_octave_base as i32 + semitones)
            .clamp(0, max_base) as u8;
    }

    pub fn piano_keyboard_end(&self) -> u8 {
        (self.piano_keyboard_base as u16 + self.piano_keyboard_octaves as u16 * 12 + 11)
            .min(127) as u8
    }

    pub fn beat_width(&self) -> f32 {
        crate::ui::grid::beat_width(self)
    }

    pub fn adjust_timeline_zoom(&mut self, delta: f32) {
        self.timeline_zoom = (self.timeline_zoom + delta).clamp(
            crate::ui::grid::MIN_ZOOM,
            crate::ui::grid::MAX_ZOOM,
        );
    }

    pub fn add_song_section(&mut self) {
        let label = if self.new_section_label.trim().is_empty() {
            let n = self.project.sections.len();
            ((b'A' + (n % 26) as u8) as char).to_string()
        } else {
            self.new_section_label.trim().to_string()
        };
        let start = self.project.section_after_last();
        let id = self.next_section_id;
        self.next_section_id += 1;
        self.project
            .add_section(id, label, start, self.new_section_bars.max(1));
        self.selected_song_section = Some(self.project.sections.len().saturating_sub(1));
        let next_letter = ((b'A' + (self.project.sections.len() % 26) as u8) as char).to_string();
        self.new_section_label = next_letter;
    }

    pub fn remove_selected_song_section(&mut self) {
        if let Some(idx) = self.selected_song_section {
            if let Some(section) = self.project.sections.get(idx) {
                let id = section.id;
                self.project.remove_section(id);
            }
            self.selected_song_section = None;
        }
    }

    pub fn jump_to_song_section(&mut self, index: usize) {
        if let Some(section) = self.project.sections.get(index) {
            self.set_playhead(section.start_beat);
            self.selected_song_section = Some(index);
        }
    }

    fn create_clip(&mut self, track_idx: usize, name: &str, at_beat: f64) -> usize {
        let id = self.next_clip_id;
        self.next_clip_id += 1;
        let clip = NoteClip::new(id, name.to_string(), at_beat);
        if let Some(track) = self.project.tracks.get_mut(track_idx) {
            track.add_clip(clip);
            track.clips.len() - 1
        } else {
            0
        }
    }

    fn ensure_recording_clip(&mut self) -> Option<(usize, usize)> {
        let track_idx = self.current_track;
        if self.recording_clip.is_none() {
            let beat = self.project.playhead_beat;
            let clip_idx = self.create_clip(track_idx, "Recording…", beat);
            self.recording_clip = Some((track_idx, clip_idx));
            self.selected_clip = Some(clip_idx);
        }
        self.recording_clip
    }

    pub fn toggle_recording(&mut self) {
        if self.recording {
            self.finish_recording();
        } else {
            self.recording = true;
            self.recording_clip = None;
            if self.sequencer_context_active() {
                self.ensure_sequencer_recording_clip();
            } else {
                self.ensure_recording_clip();
            }
        }
    }

    pub fn finish_recording(&mut self) {
        if let Some((track_idx, clip_idx)) = self.recording_clip.take() {
            if let Some(clip) = self
                .project
                .tracks
                .get_mut(track_idx)
                .and_then(|t| t.clips.get_mut(clip_idx))
            {
                clip.finalize_recording();
            }
        }
        self.recording = false;
    }

    fn add_note_to_clip_relative(
        &mut self,
        track_idx: usize,
        clip_idx: usize,
        pitch: u8,
        rel_beat: f64,
        duration: f64,
        velocity: f32,
    ) {
        let mut note = Note::new(pitch, rel_beat.max(0.0), duration);
        note.velocity = velocity.clamp(0.05, 1.0);
        if let Some(track) = self.project.tracks.get_mut(track_idx) {
            track.add_note_to_clip(clip_idx, note);
        }
    }

    pub fn add_note_batch_clip(&mut self, notes: Vec<(u8, f64, f64, f32)>) {
        if notes.is_empty() {
            return;
        }
        let track_idx = self.current_track;
        let beat = self.project.playhead_beat;
        let clip_idx = self.create_clip(track_idx, "Clip", beat);
        self.selected_clip = Some(clip_idx);
        for (pitch, rel, dur, vel) in notes {
            self.add_note_to_clip_relative(track_idx, clip_idx, pitch, rel, dur, vel);
        }
    }

    pub fn sync_instrument_to_track(&mut self) {
        if let Some(track) = self.project.tracks.get(self.current_track) {
            let name = track.instrument.label();
            self.selected_instrument = name.clone();
            if let Ok(mut eng) = self.audio_engine.lock() {
                eng.set_profile(&name);
            }
            self.audio.set_profile(&name);
        }
    }

    pub fn current_instrument_name(&self) -> String {
        self.selected_instrument.clone()
    }

    pub fn play_note(&mut self, pitch: u8) {
        self.play_note_with_velocity(pitch, 0.75);
    }

    pub fn play_note_with_velocity(&mut self, pitch: u8, velocity: f32) {
        if !self.held_notes.contains(&pitch) {
            self.held_notes.insert(pitch);
            let vel = velocity.clamp(0.05, 1.0);
            let (instrument, track_filter) = self.current_track_playback_info();
            if let Ok(mut eng) = self.audio_engine.lock() {
                eng.play_note_for_instrument(pitch, vel, &instrument, track_filter);
            }

            let mut recording_anchor = None;
            if self.recording {
                let beat = self.project.playhead_beat;
                if let Some((track_idx, clip_idx)) = self.ensure_recording_clip() {
                    if track_idx == self.current_track {
                        if let Some(clip) = self
                            .project
                            .tracks
                            .get(track_idx)
                            .and_then(|t| t.clips.get(clip_idx))
                        {
                            let rel = beat - clip.start_beat;
                            self.add_note_to_clip_relative(
                                track_idx,
                                clip_idx,
                                pitch,
                                rel,
                                0.25,
                                vel,
                            );
                            recording_anchor = Some((track_idx, clip_idx, rel));
                        }
                    }
                }
            }

            self.held_note_details.insert(
                pitch,
                HeldNoteState {
                    press_start: Instant::now(),
                    peak_velocity: vel,
                    recording_anchor,
                },
            );
        }
    }

    pub fn update_held_note_velocity(&mut self, pitch: u8, velocity: f32) {
        if !self.held_notes.contains(&pitch) {
            return;
        }
        let vel = velocity.clamp(0.05, 1.0);
        if let Some(state) = self.held_note_details.get_mut(&pitch) {
            state.peak_velocity = state.peak_velocity.max(vel);
        }
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.set_note_velocity(pitch, vel);
        }
    }

    fn current_track_playback_info(&self) -> (InstrumentId, f32) {
        self.project
            .tracks
            .get(self.current_track)
            .map(|t| (t.instrument.clone(), t.effects.effective_lowpass()))
            .unwrap_or((InstrumentId::Piano, 1.0))
    }

    pub fn release_note(&mut self, pitch: u8) {
        if let Some(state) = self.held_note_details.remove(&pitch) {
            if let Some((track_idx, clip_idx, rel_start)) = state.recording_anchor {
                let hold_secs = state.press_start.elapsed().as_secs_f64();
                let duration =
                    (hold_secs * self.project.beats_per_second()).clamp(0.125, 16.0);
                if let Some(track) = self.project.tracks.get_mut(track_idx) {
                    if let Some(clip) = track.clips.get_mut(clip_idx) {
                        for note in &mut clip.notes {
                            if note.pitch == pitch
                                && (note.start_beat - rel_start).abs() < 0.001
                            {
                                note.duration_beats = duration;
                                note.velocity = state.peak_velocity;
                                break;
                            }
                        }
                        clip.recompute_bounds();
                    }
                }
            }
        }
        self.held_notes.remove(&pitch);
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.stop_note(pitch);
        }
    }

    pub fn add_note_at(&mut self, pitch: u8, start_beat: f64, duration: f64) {
        self.add_note_at_with_velocity(pitch, start_beat, duration, 0.8);
    }

    pub fn add_note_at_with_velocity(
        &mut self,
        pitch: u8,
        start_beat: f64,
        duration: f64,
        velocity: f32,
    ) {
        let track_idx = self.current_track;
        let clip_idx = if let Some(ci) = self.selected_clip {
            ci
        } else {
            let ci = self.create_clip(track_idx, "Clip", start_beat);
            self.selected_clip = Some(ci);
            ci
        };
        if let Some(clip) = self
            .project
            .tracks
            .get(track_idx)
            .and_then(|t| t.clips.get(clip_idx))
        {
            let rel = start_beat - clip.start_beat;
            self.add_note_to_clip_relative(track_idx, clip_idx, pitch, rel, duration, velocity);
        }
        let (instrument, track_filter) = self.current_track_playback_info();
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.play_note_for_instrument(pitch, velocity, &instrument, track_filter);
        }
    }

    pub fn select_track(&mut self, index: usize) {
        if index < self.project.tracks.len() {
            self.current_track = index;
            self.sync_instrument_to_track();
        }
    }

    pub fn set_playhead(&mut self, beat: f64) {
        self.project.playhead_beat = beat.clamp(0.0, self.project.total_beats);
    }

    pub fn go_to_start(&mut self) {
        self.project.playhead_beat = 0.0;
        self.stop_playback_notes();
    }

    pub fn go_to_end(&mut self) {
        let content_end = self.project_end_beat();
        self.project.playhead_beat = content_end.max(0.0);
        self.stop_playback_notes();
    }

    fn project_end_beat(&self) -> f64 {
        let last_note = self
            .project
            .tracks
            .iter()
            .flat_map(|t| t.clips.iter())
            .map(|c| c.timeline_end())
            .fold(0.0_f64, f64::max);
        if last_note > 0.0 {
            last_note.min(self.project.total_beats)
        } else {
            self.project.total_beats
        }
    }

    pub fn handle_transport_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Home) {
                self.go_to_start();
            }
            if i.key_pressed(egui::Key::End) {
                self.go_to_end();
            }
        });
    }

    pub fn toggle_playback(&mut self) {
        self.playing = !self.playing;
        if self.playing {
            self.playback_start = Instant::now();
            self.last_frame = Instant::now();
        } else {
            self.stop_playback_notes();
        }
    }

    pub fn stop_playback(&mut self) {
        self.playing = false;
        self.go_to_start();
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.stop_all();
        }
    }

    fn stop_playback_notes(&mut self) {
        if let Ok(mut eng) = self.audio_engine.lock() {
            for pitch in self.playback_notes_active.drain() {
                eng.stop_note(pitch);
            }
        }
        self.playback_events_done.clear();
    }

    pub fn update_playback(&mut self) {
        if !self.playing {
            return;
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;

        self.project.playhead_beat += dt * self.project.beats_per_second();
        if self.project.playhead_beat >= self.project.total_beats {
            self.project.playhead_beat = 0.0;
            self.stop_playback_notes();
        }

        let beat = self.project.playhead_beat;
        for (track_idx, track) in self.project.tracks.iter().enumerate() {
            if track.muted {
                continue;
            }
            let is_drum = track.is_drum();
            for (clip_idx, clip) in track.clips.iter().enumerate() {
                for (note_idx, note) in clip.notes.iter().enumerate() {
                    let note_end = clip.note_absolute_end(note);
                    let tolerance = 0.05;

                    for trigger_beat in track.note_trigger_times(clip, note, self.project.total_beats) {
                        let event_key = ((track_idx as u32) << 22)
                            | ((clip_idx as u32) << 16)
                            | (((trigger_beat * 100.0) as u32) & 0xFFFF)
                            | ((note.pitch as u32) << 8);

                        if (beat - trigger_beat).abs() < tolerance
                            && !self.playback_events_done.contains(&event_key)
                        {
                            if let Ok(mut eng) = self.audio_engine.lock() {
                                let inst = track.instrument.clone();
                                let filter = track.effects.effective_lowpass();
                                eng.play_note_for_instrument(
                                    note.pitch,
                                    note.velocity * track.volume,
                                    &inst,
                                    filter,
                                );
                            }
                            self.playback_events_done.insert(event_key);
                            if !is_drum {
                                self.playback_notes_active.insert(note.pitch);
                            }
                        }
                    }

                    if !is_drum
                        && beat >= note_end
                        && self.playback_notes_active.contains(&note.pitch)
                    {
                        if let Ok(mut eng) = self.audio_engine.lock() {
                            eng.stop_note(note.pitch);
                        }
                        self.playback_notes_active.remove(&note.pitch);
                    }
                    let _ = note_idx;
                }
            }
        }
    }

    pub fn save_custom_instrument(&mut self, custom: CustomInstrument) {
        let profile = InstrumentProfile::from_custom(&custom);
        let name = custom.name.clone();
        self.custom_instruments.push(custom);
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.add_profile(profile.clone());
        }
        self.audio.add_profile(profile);
        self.select_custom_instrument(&name);
        self.pending_custom_instrument = None;
        self.instrument_name_input.clear();
        self.status_message = Some(format!("Added \"{}\" to instrument library", name));
        self.status_error = None;
    }

    pub fn preview_custom_instrument(&mut self) {
        if let Some(custom) = &self.pending_custom_instrument {
            let profile = InstrumentProfile::from_custom(custom);
            if let Ok(mut eng) = self.audio_engine.lock() {
                eng.add_profile(profile.clone());
                eng.set_profile(&custom.name);
                eng.play_note(60, 0.8);
            }
        }
    }

    pub fn preview_base_instrument(&mut self) {
        let name = self.ai_base_kind.label();
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.set_profile(name);
            eng.play_note(60, 0.8);
        }
    }

    pub fn preview_builtin(&mut self, kind: SampleKind) {
        let name = kind.label();
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.set_profile(name);
            eng.play_note(60, 0.8);
        }
    }

    pub fn select_builtin_instrument(&mut self, kind: SampleKind) {
        let name = kind.label();
        self.selected_instrument = name.to_string();
        if let Some(track) = self.project.tracks.get_mut(self.current_track) {
            track.instrument = InstrumentId::from_sample_kind(kind);
        }
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.set_profile(name);
        }
        self.audio.set_profile(name);
    }

    pub fn select_custom_instrument(&mut self, name: &str) {
        self.selected_instrument = name.to_string();
        if let Some(track) = self.project.tracks.get_mut(self.current_track) {
            track.instrument = InstrumentId::Custom(name.to_string());
        }
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.set_profile(name);
        }
        self.audio.set_profile(name);
    }

    pub fn save_project_dialog(&mut self) {
        self.status_error = None;
        let path = rfd::FileDialog::new()
            .add_filter("Rust Audio Composer", &["rac.json", "json"])
            .set_file_name("my-song.rac.json")
            .save_file();
        if let Some(path) = path {
            let saved = SavedProject::from_app(
                self.project.clone(),
                self.custom_instruments.clone(),
                self.saved_drum_patterns.clone(),
                self.audio.master_volume,
                self.audio.master_effects,
            );
            match saved.save_to_path(&path) {
                Ok(()) => {
                    self.project_file_path = Some(path.display().to_string());
                    self.status_message = Some(format!("Saved project to {}", path.display()));
                }
                Err(e) => self.status_error = Some(e),
            }
        }
    }

    pub fn load_project_dialog(&mut self) {
        self.status_error = None;
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Rust Audio Composer", &["rac.json", "json"])
            .pick_file()
        {
            match SavedProject::load_from_path(&path) {
                Ok(saved) => {
                    self.apply_saved_project(saved);
                    self.project_file_path = Some(path.display().to_string());
                    self.status_message = Some(format!("Loaded project from {}", path.display()));
                }
                Err(e) => self.status_error = Some(e),
            }
        }
    }

    pub fn export_wav_dialog(&mut self) {
        self.status_error = None;
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("WAV Audio", &["wav"])
            .set_file_name("export.wav")
            .save_file()
        {
            let sr = self.audio.sample_rate();
            match export_project_wav(
                &self.project,
                &self.custom_instruments,
                &path,
                sr,
                self.audio.master_volume,
                &self.audio.master_effects,
            ) {
                Ok(()) => {
                    self.status_message = Some(format!("Exported audio to {}", path.display()))
                }
                Err(e) => self.status_error = Some(e),
            }
        }
    }

    fn apply_saved_project(&mut self, saved: SavedProject) {
        self.project = saved.project;
        self.custom_instruments = saved.custom_instruments;
        self.saved_drum_patterns = saved.drum_patterns;
        self.audio.master_volume = saved.master_volume;
        let master_fx = saved.master_effects;
        self.audio.master_effects = master_fx;
        self.current_track = 0;
        if self.current_track >= self.project.tracks.len() && !self.project.tracks.is_empty() {
            self.current_track = self.project.tracks.len() - 1;
        }

        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.master_volume = saved.master_volume;
            eng.master_effects = master_fx;
            for profile in crate::model::instrument::default_profiles() {
                eng.add_profile(profile);
            }
            for custom in &self.custom_instruments {
                eng.add_profile(InstrumentProfile::from_custom(custom));
            }
        }
        for profile in crate::model::instrument::default_profiles() {
            self.audio.add_profile(profile);
        }
        for custom in &self.custom_instruments.clone() {
            self.audio.add_profile(InstrumentProfile::from_custom(custom));
        }
        self.sync_instrument_to_track();
        self.migrate_tracks();
        self.stop_playback();
    }

    pub fn insert_chord_from_roman(&mut self, roman: &str, key: &Key) {
        let root_pc = key.root;
        let intervals: &[u8] = match roman {
            "I" | "i" => &[0, 4, 7],
            "ii" | "II" => &[2, 5, 9],
            "iii" | "III" => &[4, 7, 11],
            "IV" | "iv" => &[5, 9, 0],
            "V" | "v" => &[7, 11, 2],
            "vi" | "VI" => &[9, 0, 4],
            "vii°" => &[11, 2, 5],
            "vii" | "VII" => &[10, 2, 5],
            _ => {
                if let Some(pc) = NOTE_NAMES.iter().position(|&n| n == roman) {
                    let base = 60 + pc as u8;
                    let beat = self.project.playhead_beat;
                    let mut batch = Vec::new();
                    for offset in [0u8, 4, 7] {
                        let pitch = base + offset - (base % 12) + (pc as u8);
                        batch.push((pitch, 0.0, 2.0, 0.8));
                        self.play_note_at_preview(pitch, 0.8);
                    }
                    let _ = beat;
                    self.add_note_batch_clip(batch);
                    return;
                }
                &[0, 4, 7]
            }
        };

        let beat = self.project.playhead_beat;
        let mut batch = Vec::new();
        for &interval in intervals {
            let pc = (root_pc + interval) % 12;
            let pitch = 60 + pc;
            batch.push((pitch, 0.0, 2.0, 0.8));
        }
        self.add_note_batch_clip(batch.clone());
        for &(pitch, _, _, vel) in &batch {
            self.play_note_at_preview(pitch, vel);
        }
        let _ = beat;
    }

    fn play_note_at_preview(&mut self, pitch: u8, velocity: f32) {
        let (instrument, track_filter) = self.current_track_playback_info();
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.play_note_for_instrument(pitch, velocity, &instrument, track_filter);
        }
    }

    fn drum_clip_idx(&mut self, track_idx: usize, at_beat: f64) -> usize {
        if let Some(ci) = self.selected_clip {
            if self.current_track == track_idx {
                return ci;
            }
        }
        let ci = self.create_clip(track_idx, "Drums", at_beat);
        if track_idx == self.current_track {
            self.selected_clip = Some(ci);
        }
        ci
    }

    fn find_drum_in_clip(
        track: &crate::model::Track,
        clip_idx: usize,
        pitch: u8,
        rel_beat: f64,
    ) -> Option<usize> {
        track.clips.get(clip_idx).and_then(|clip| {
            clip.notes.iter().position(|n| {
                n.pitch == pitch && (n.start_beat - rel_beat).abs() < 0.001
            })
        })
    }

    pub fn preview_drum(&mut self, kind: crate::model::DrumKind) {
        self.preview_drum_with_velocity(kind, 0.85);
    }

    pub fn preview_drum_with_velocity(&mut self, kind: crate::model::DrumKind, velocity: f32) {
        let pitch = kind.midi_pitch();
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.play_note_for_instrument(pitch, velocity, &InstrumentId::Drums, 1.0);
        }
    }

    pub fn sequencer_context_active(&self) -> bool {
        self.show_drum_sequencer
            && self
                .project
                .tracks
                .get(self.current_track)
                .map(|t| t.is_drum())
                .unwrap_or(false)
    }

    pub fn sequencer_bar_start(&self) -> f64 {
        let bar = self.project.beats_per_bar();
        (self.project.playhead_beat / bar).floor() * bar
    }

    pub fn refresh_sequencer_draft(&mut self) {
        let track_idx = self.current_track;
        let bar_start = self.sequencer_bar_start();
        let beats_per_bar = self.project.beats_per_bar();
        let steps = SEQUENCER_STEPS;

        self.sequencer_draft.clear();
        self.sequencer_draft_bar_start = bar_start;
        self.sequencer_draft_track = Some(track_idx);
        self.sequencer_loop_beat = 0.0;
        self.sequencer_preview_step = u8::MAX;
        self.sequencer_last_frame = Instant::now();

        if let Some(track) = self.project.tracks.get(track_idx) {
            for note in track.flat_notes() {
                let rel = note.start_beat - bar_start;
                if rel >= 0.0 && rel < beats_per_bar - 0.001 {
                    if let Some(drum) = DrumKind::from_midi_pitch(note.pitch) {
                        let step = beat_to_step(rel, beats_per_bar, steps);
                        self.sequencer_draft.push((drum, step, note.velocity));
                    }
                }
            }
        }
    }

    pub fn open_drum_sequencer(&mut self) {
        self.show_drum_sequencer = true;
        self.sequencer_running = true;
        self.refresh_sequencer_draft();
    }

    pub fn close_drum_sequencer(&mut self) {
        self.show_drum_sequencer = false;
        self.sequencer_running = false;
    }

    pub fn toggle_sequencer_run(&mut self) {
        self.sequencer_running = !self.sequencer_running;
        if self.sequencer_running {
            self.sequencer_last_frame = Instant::now();
            self.sequencer_preview_step = u8::MAX;
        }
    }

    pub fn sequencer_draft_hit(&self, drum: DrumKind, step: u8) -> Option<f32> {
        self.sequencer_draft
            .iter()
            .find(|(d, s, _)| *d == drum && *s == step)
            .map(|(_, _, v)| *v)
    }

    fn ensure_sequencer_recording_clip(&mut self) -> Option<(usize, usize)> {
        let track_idx = self.current_track;
        let bar_start = self.sequencer_draft_bar_start;
        let beats_per_bar = self.project.beats_per_bar();

        if self.recording_clip.is_none() {
            let clip_idx = self.create_clip(track_idx, "Recording…", bar_start);
            if let Some(clip) = self
                .project
                .tracks
                .get_mut(track_idx)
                .and_then(|t| t.clips.get_mut(clip_idx))
            {
                clip.content_length = beats_per_bar;
                clip.source_length = beats_per_bar;
            }
            self.recording_clip = Some((track_idx, clip_idx));
            self.selected_clip = Some(clip_idx);
        }

        self.flush_sequencer_draft_to_recording_clip();
        self.recording_clip
    }

    fn flush_sequencer_draft_to_recording_clip(&mut self) {
        let Some((track_idx, clip_idx)) = self.recording_clip else {
            return;
        };
        let beats_per_bar = self.project.beats_per_bar();
        let steps = SEQUENCER_STEPS;
        let draft = self.sequencer_draft.clone();

        if let Some(track) = self.project.tracks.get_mut(track_idx) {
            if let Some(clip) = track.clips.get_mut(clip_idx) {
                clip.notes.clear();
                for (drum, step, velocity) in draft {
                    let rel = step_to_beat(step, beats_per_bar, steps);
                    let mut note = Note::new(drum.midi_pitch(), rel, 0.25);
                    note.velocity = velocity;
                    clip.notes.push(note);
                }
                clip.recompute_bounds();
            }
        }
    }

    fn sync_sequencer_step_to_recording_clip(
        &mut self,
        drum: DrumKind,
        step: u8,
        beats_per_bar: f64,
        steps: u8,
        velocity: Option<f32>,
    ) {
        let Some((track_idx, clip_idx)) = self.recording_clip else {
            return;
        };
        let rel = step_to_beat(step, beats_per_bar, steps);
        let pitch = drum.midi_pitch();

        if let Some(track) = self.project.tracks.get_mut(track_idx) {
            let pos = track.clips.get(clip_idx).and_then(|clip| {
                clip.notes
                    .iter()
                    .position(|n| n.pitch == pitch && (n.start_beat - rel).abs() < 0.001)
            });
            if let Some(clip) = track.clips.get_mut(clip_idx) {
                if let Some(pos) = pos {
                    clip.notes.remove(pos);
                } else if let Some(vel) = velocity {
                    let mut note = Note::new(pitch, rel, 0.25);
                    note.velocity = vel;
                    clip.notes.push(note);
                }
                clip.recompute_bounds();
            }
        }
    }

    pub fn update_sequencer_preview(&mut self) {
        if !self.sequencer_running || !self.show_drum_sequencer {
            return;
        }
        let track_idx = self.current_track;
        let Some(track) = self.project.tracks.get(track_idx) else {
            return;
        };
        if !track.is_drum() || track.muted {
            return;
        }

        let now = Instant::now();
        let dt = now.duration_since(self.sequencer_last_frame).as_secs_f64();
        self.sequencer_last_frame = now;

        let beats_per_bar = self.project.beats_per_bar();
        let steps = SEQUENCER_STEPS;
        let step_duration = beats_per_bar / steps as f64;

        let prev_beat = self.sequencer_loop_beat;
        self.sequencer_loop_beat += dt * self.project.beats_per_second();
        if self.sequencer_loop_beat >= beats_per_bar {
            self.sequencer_loop_beat -= beats_per_bar;
            self.sequencer_preview_step = u8::MAX;
        }

        let prev_step = (prev_beat / step_duration).floor() as u8 % steps;
        let curr_step = (self.sequencer_loop_beat / step_duration).floor() as u8 % steps;

        if curr_step != prev_step || self.sequencer_loop_beat < prev_beat {
            self.trigger_sequencer_step(curr_step);
        }
    }

    fn trigger_sequencer_step(&mut self, step: u8) {
        if step == self.sequencer_preview_step {
            return;
        }
        self.sequencer_preview_step = step;

        let hits: Vec<(DrumKind, f32)> = self
            .sequencer_draft
            .iter()
            .filter(|(_, s, _)| *s == step)
            .map(|(d, _, v)| (*d, *v))
            .collect();

        for (drum, velocity) in hits {
            self.preview_drum_with_velocity(drum, velocity);
        }
    }

    pub fn toggle_drum_step(
        &mut self,
        track_idx: usize,
        drum: crate::model::DrumKind,
        step: u8,
        beats_per_bar: f64,
        steps: u8,
        accent: bool,
    ) {
        let existing = self.sequencer_draft_hit(drum, step);
        let velocity = if accent { 1.0 } else { 0.72 };

        if existing.is_some() {
            self.sequencer_draft
                .retain(|(d, s, _)| !(*d == drum && *s == step));
            if self.recording {
                self.sync_sequencer_step_to_recording_clip(drum, step, beats_per_bar, steps, None);
            }
        } else {
            self.sequencer_draft.push((drum, step, velocity));
            self.preview_drum_with_velocity(drum, velocity);
            if self.recording {
                self.sync_sequencer_step_to_recording_clip(
                    drum,
                    step,
                    beats_per_bar,
                    steps,
                    Some(velocity),
                );
            }
        }

        let _ = track_idx;
    }

    pub fn clear_drum_step(
        &mut self,
        track_idx: usize,
        drum: crate::model::DrumKind,
        step: u8,
        beats_per_bar: f64,
        steps: u8,
    ) {
        self.sequencer_draft
            .retain(|(d, s, _)| !(*d == drum && *s == step));
        if self.recording {
            self.sync_sequencer_step_to_recording_clip(drum, step, beats_per_bar, steps, None);
        }
        let _ = track_idx;
    }

    pub fn clear_drum_pattern(&mut self, track_idx: usize) {
        self.sequencer_draft.clear();
        if self.recording {
            if let Some((ti, clip_idx)) = self.recording_clip {
                if ti == track_idx {
                    if let Some(clip) = self
                        .project
                        .tracks
                        .get_mut(track_idx)
                        .and_then(|t| t.clips.get_mut(clip_idx))
                    {
                        clip.notes.clear();
                        clip.recompute_bounds();
                    }
                }
            }
        }
    }

    pub fn toggle_drum_hit_at_beat(
        &mut self,
        track_idx: usize,
        drum: crate::model::DrumKind,
        beat: f64,
        accent: bool,
    ) {
        let beat = (beat * 4.0).round() / 4.0;
        let pitch = drum.midi_pitch();
        let clip_idx = self.drum_clip_idx(track_idx, beat);
        if let Some(track) = self.project.tracks.get_mut(track_idx) {
            let rel = beat
                - track
                    .clips
                    .get(clip_idx)
                    .map(|c| c.start_beat)
                    .unwrap_or(beat);
            if let Some(pos) = Self::find_drum_in_clip(track, clip_idx, pitch, rel) {
                if let Some(clip) = track.clips.get_mut(clip_idx) {
                    clip.notes.remove(pos);
                    clip.recompute_bounds();
                }
            } else {
                let velocity = if accent { 1.0 } else { 0.72 };
                let mut note = Note::new(pitch, rel.max(0.0), 0.25);
                note.velocity = velocity;
                track.add_note_to_clip(clip_idx, note);
                if let Ok(mut eng) = self.audio_engine.lock() {
                    eng.play_note_for_instrument(pitch, velocity, &InstrumentId::Drums, 1.0);
                }
            }
        }
    }

    pub fn delete_selected_note(&mut self) {
        if let Some((clip_idx, note_idx)) = self.selected_note {
            if let Some(track) = self.project.tracks.get_mut(self.current_track) {
                if let Some(clip) = track.clips.get_mut(clip_idx) {
                    if note_idx < clip.notes.len() {
                        clip.notes.remove(note_idx);
                        clip.recompute_bounds();
                        self.selected_note = None;
                    }
                }
            }
        }
    }

    pub fn section_select_click(&mut self, beat: f64) {
        if let Some(start) = self.section_select_drag {
            let (s, e) = if beat >= start {
                (start, beat)
            } else {
                (beat, start)
            };
            self.section_range = Some((s, e));
            self.section_select_drag = None;
        } else {
            self.section_select_drag = Some(beat);
        }
    }

    pub fn apply_section_range_drag(&mut self, start_beat: f64, end_beat: f64) {
        let (s, e) = if end_beat >= start_beat {
            (start_beat, end_beat)
        } else {
            (end_beat, start_beat)
        };
        if (e - s).abs() > 0.01 {
            self.section_range = Some((s.max(0.0), e));
        }
    }

    pub fn set_clip_loop_from_section(&mut self, track_idx: usize, clip_idx: usize) {
        if let Some((s, e)) = self.section_range {
            if let Some(clip) = self
                .project
                .tracks
                .get_mut(track_idx)
                .and_then(|t| t.clips.get_mut(clip_idx))
            {
                let rel_start = (s - clip.start_beat).max(0.0);
                let rel_end = (e - clip.start_beat).max(rel_start + 0.25);
                clip.trim_start = rel_start;
                clip.trim_end = (clip.source_length - rel_end).max(0.0);
                clip.content_length = clip.source_length;
                clip.loop_enabled = true;
                clip.loop_end_beat = clip.start_beat + clip.visible_length() * 2.0;
            }
        }
    }

    pub fn toggle_selected_clip_loop(&mut self) {
        if let (Some(track), Some(clip_idx)) = (
            self.project.tracks.get_mut(self.current_track),
            self.selected_clip,
        ) {
            if let Some(clip) = track.clips.get_mut(clip_idx) {
                clip.loop_enabled = !clip.loop_enabled;
                if clip.loop_enabled {
                    clip.loop_end_beat = clip.start_beat + clip.visible_length() * 2.0;
                }
            }
        }
    }

    pub fn set_track_loop_from_section(&mut self, track_idx: usize) {
        if let Some(clip_idx) = self.selected_clip {
            self.set_clip_loop_from_section(track_idx, clip_idx);
        }
    }

    pub fn save_drum_pattern(&mut self, track_idx: usize) {
        let beats_per_bar = self.project.beats_per_bar();
        let steps = SEQUENCER_STEPS;
        let name = if self.drum_pattern_name_input.trim().is_empty() {
            format!("Pattern {}", self.saved_drum_patterns.len() + 1)
        } else {
            self.drum_pattern_name_input.trim().to_string()
        };
        let notes: Vec<Note> = self
            .sequencer_draft
            .iter()
            .map(|(drum, step, velocity)| {
                let mut note = Note::new(
                    drum.midi_pitch(),
                    step_to_beat(*step, beats_per_bar, steps),
                    0.25,
                );
                note.velocity = *velocity;
                note
            })
            .collect();
        let pattern = SavedDrumPattern { name: name.clone(), notes };
        self.saved_drum_patterns.push(pattern);
        self.selected_pattern_index = self.saved_drum_patterns.len().saturating_sub(1);
        self.status_message = Some(format!("Saved drum pattern \"{}\"", name));
        self.drum_pattern_name_input =
            format!("Pattern {}", self.saved_drum_patterns.len() + 1);
        let _ = track_idx;
    }

    pub fn insert_drum_pattern(&mut self, track_idx: usize, pattern_idx: usize, at_beat: f64) {
        let Some(pattern) = self.saved_drum_patterns.get(pattern_idx).cloned() else {
            return;
        };
        let clip_idx = self.create_clip(track_idx, &pattern.name, at_beat);
        if let Some(track) = self.project.tracks.get_mut(track_idx) {
            for note in pattern.notes {
                track.add_note_to_clip(clip_idx, note);
            }
            self.status_message = Some(format!(
                "Inserted \"{}\" at beat {:.1}",
                pattern.name, at_beat
            ));
        }
    }
}

impl eframe::App for DAWApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_playback();
        self.update_sequencer_preview();
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.master_volume = self.audio.master_volume;
            eng.master_effects = self.audio.master_effects.clone();
        }
        self.handle_transport_shortcuts(ctx);
        ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            crate::ui::show_transport(self, ui);
        });

        egui::SidePanel::left("track_panel")
            .resizable(true)
            .min_width(220.0)
            .show(ctx, |ui| {
                crate::ui::show_track_panel(self, ui);
            });

        if self.show_composition {
            egui::SidePanel::right("composition_panel")
                .resizable(true)
                .min_width(280.0)
                .default_width(320.0)
                .show(ctx, |ui| {
                    crate::ui::show_composition_panel(self, ui);
                });
        }

        if self.show_instrument_designer {
            let mut show = self.show_instrument_designer;
            egui::Window::new("AI Instrument Designer")
                .open(&mut show)
                .default_size([400.0, 500.0])
                .show(ctx, |ui| {
                    crate::ui::show_instrument_designer(self, ui);
                });
            self.show_instrument_designer = show;
        }

        self.show_piano_window(ctx);

        if self.show_drum_sequencer {
            let mut show = self.show_drum_sequencer;
            egui::Window::new("Beat Sequencer")
                .open(&mut show)
                .default_size([620.0, 420.0])
                .show(ctx, |ui| {
                    crate::ui::show_drum_sequencer(self, ui);
                });
            if !show {
                self.close_drum_sequencer();
            } else {
                self.show_drum_sequencer = true;
            }
        }

        if self.show_dj_panel {
            let mut show = self.show_dj_panel;
            egui::Window::new("DJ Tools")
                .open(&mut show)
                .default_size([360.0, 480.0])
                .show(ctx, |ui| {
                    crate::ui::show_dj_panel(self, ui);
                });
            self.show_dj_panel = show;
        }

        if self.show_options {
            let mut show = self.show_options;
            egui::Window::new("Options")
                .open(&mut show)
                .default_size([320.0, 180.0])
                .show(ctx, |ui| {
                    crate::ui::show_options(self, ui, ctx);
                });
            self.show_options = show;
        }

        if self.show_user_guide {
            let mut show = self.show_user_guide;
            egui::Window::new("User Guide")
                .open(&mut show)
                .default_size([420.0, 520.0])
                .show(ctx, |ui| {
                    crate::ui::show_user_guide(ui);
                });
            self.show_user_guide = show;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                show_timeline_section(self, ui);
                ui.separator();
                crate::ui::show_piano_roll_detail(self, ui);
            });
        });
    }
}

fn show_timeline_section(app: &mut DAWApp, ui: &mut egui::Ui) {
    let available = ui.available_height() - crate::ui::constants::PIANO_ROLL_PANEL_HEIGHT - 20.0;
    egui::ScrollArea::vertical()
        .max_height(available.max(120.0))
        .show(ui, |ui| {
            crate::ui::show_multi_track_timeline(app, ui);
        });
}

impl DAWApp {
    pub fn show_piano_window(&mut self, ctx: &egui::Context) {
        crate::ui::show_piano_window(self, ctx);
    }
}
