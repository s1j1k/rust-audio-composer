use crate::audio::engine::VelocityCurve;
use crate::audio::export::export_project_wav;
use crate::audio::AudioEngine;
use crate::remote::protocol::InstrumentChoice;
use crate::remote::{RemoteEvent, RemoteServer};
use crate::model::drum::{beat_to_step, step_to_beat, DrumKind, SEQUENCER_STEPS};
use crate::model::instrument::{
    CustomInstrument, DrumKit, Genre, GenrePreset, InstrumentId, InstrumentProfile, SampleKind,
};
use crate::model::{Note, NoteClip, Project, SavedDrumPattern, SavedProject};
use crate::music::{resolve_chord, Key};

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

/// Copied clips. Serialized to the system clipboard as JSON so Ctrl+V works (egui only
/// reports a paste when the OS clipboard has text) and clips can move between windows.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClipClipboard {
    /// Marks the JSON as ours, so pasting ordinary text does nothing.
    pub format: String,
    /// Copied from a drum track (drum hits only paste onto drum tracks, and vice versa).
    pub drum: bool,
    pub clips: Vec<NoteClip>,
}

const CLIP_CLIPBOARD_FORMAT: &str = "rust-audio-composer/clips@1";

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

    /// Virtual piano popped out into its own window.
    pub show_piano: bool,
    /// The bottom panel shows the virtual piano (instead of the piano roll / drum views).
    pub bottom_keyboard: bool,
    /// Notes started by the computer keyboard (so key-up only releases those, never a note
    /// held with the mouse on the same pitch).
    pub keyboard_held_notes: HashSet<u8>,
    pub show_composition: bool,
    pub show_instrument_designer: bool,
    /// True while the beat sequencer is on screen (docked or popped out). Kept in sync
    /// each frame by `sync_sequencer_visibility`.
    pub show_drum_sequencer: bool,
    /// On drum tracks the bottom panel shows the step sequencer (true) or the drum roll.
    pub drum_view_sequencer: bool,
    /// The sequencer is in its own floating window instead of the bottom panel.
    pub sequencer_popped_out: bool,
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
    /// Key chosen by hand in the Composition Assistant (None = use the detected key).
    pub composition_key: Option<Key>,
    /// Genre whose tips are shown in the Composition Assistant.
    pub composition_genre: Option<crate::model::instrument::Genre>,
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
    /// Extra clips (by id, on the current track) added with Ctrl/Shift+click, for merge,
    /// copy and duplicate. `selected_clip` is always part of the selection too.
    pub selected_clip_ids: HashSet<u64>,
    /// Last copied clips, also put on the system clipboard as JSON (see `ClipClipboard`).
    pub clip_clipboard: Option<ClipClipboard>,
    pub note_drag: Option<NoteDragState>,
    pub clip_drag: Option<ClipDragState>,
    pub next_clip_id: u64,
    pub recording_clip: Option<(usize, usize)>,
    /// Track waiting for the user to confirm deletion (it has clips, and there's no undo).
    pub pending_track_delete: Option<usize>,
    /// When the current take started (for the REC timer) and whether it is overdubbing
    /// into a bubble that already existed.
    pub recording_since: Option<Instant>,
    pub recording_overdub: bool,
    /// Recording started the transport, so stopping the take also stops playback.
    pub recording_started_transport: bool,
    /// Count-in before a take: the beat where recording will begin. The playhead pre-rolls
    /// one bar up to it with metronome clicks.
    pub count_in_target: Option<f64>,
    count_in_last_click: i64,
    /// The count-in runs on its own clock: the playhead can't go below beat 0 (and the
    /// transport's beat field clamps it), so when recording near the start the playhead
    /// waits at 0 until the virtual pre-roll position reaches it.
    count_in_started: Instant,
    pub section_range: Option<(f64, f64)>,
    pub section_select_drag: Option<f64>,
    pub section_drag: Option<(f64, f64)>,
    pub section_note_indices: Option<Vec<usize>>,
    pub section_drag_origins: HashMap<usize, f64>,
    pub section_clip_index: Option<usize>,
    pub piano_roll_grid_rect: Option<egui::Rect>,
    pub timeline_grid_origin_x: Option<f32>,
    /// Highest pitch shown in the melodic piano roll; the roll shows
    /// `visible_pitch_count()` rows downward from here.
    pub piano_roll_top: u8,
    /// Leftover scroll (pixels) that hasn't added up to a whole semitone yet.
    pub piano_roll_scroll_accum: f32,
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

    /// Live touch -> velocity mapping for the on-screen piano and remote devices.
    pub velocity_curve: VelocityCurve,
    pub show_remote: bool,
    pub remote: Option<RemoteServer>,
    pub remote_error: Option<String>,
    /// Notes currently held on remote devices (kept apart from `held_notes`, which the
    /// on-screen piano releases whenever the mouse isn't on a key).
    pub remote_held: HashMap<u8, HeldNoteState>,
    pub remote_chain: bool,
    pub remote_chain_start: u8,
    pub designer: crate::ui::instrument_designer::DesignerState,
}

impl Default for DAWApp {
    fn default() -> Self {
        // Build the engine at the device's real sample rate from the start, so samples are
        // decoded once at the right rate (and pitch).
        let sample_rate = probe_output_sample_rate().unwrap_or(44_100.0);
        let mut app = Self {
            project: Project::default(),
            audio: AudioEngine::new(sample_rate),
            audio_stream: None,
            audio_engine: Arc::new(Mutex::new(AudioEngine::new(sample_rate))),

            playing: false,
            recording: false,
            current_track: 0,

            show_piano: false,
            bottom_keyboard: false,
            keyboard_held_notes: HashSet::new(),
            show_composition: true,
            show_instrument_designer: false,
            show_drum_sequencer: false,
            drum_view_sequencer: true,
            sequencer_popped_out: false,
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
            composition_key: None,
            composition_genre: None,
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
            selected_clip_ids: HashSet::new(),
            clip_clipboard: None,
            note_drag: None,
            clip_drag: None,
            next_clip_id: 1,
            pending_track_delete: None,
            recording_clip: None,
            recording_since: None,
            recording_overdub: false,
            recording_started_transport: false,
            count_in_target: None,
            count_in_last_click: -1,
            count_in_started: Instant::now(),
            section_range: None,
            section_select_drag: None,
            section_drag: None,
            section_note_indices: None,
            section_drag_origins: HashMap::new(),
            section_clip_index: None,
            piano_roll_grid_rect: None,
            timeline_grid_origin_x: None,
            piano_roll_top: 72, // C5: the view starts around middle C
            piano_roll_scroll_accum: 0.0,
            piano_keyboard_octaves: 2,
            piano_keyboard_base: 48,
            // Keys show their computer-key letters, so the big map starts hidden.
            keyboard_mapping_minimized: true,
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

            velocity_curve: VelocityCurve::Natural,
            show_remote: false,
            remote: None,
            remote_error: None,
            remote_held: HashMap::new(),
            remote_chain: false,
            remote_chain_start: 36,
            designer: Default::default(),
        };

        if let Ok(stream) = app.setup_audio_stream() {
            app.audio_stream = Some(stream);
        }

        app.sync_instrument_to_track();
        app.migrate_tracks();
        // `cargo run --release -- --remote` opens the phone keyboard server at launch.
        if std::env::args().any(|a| a == "--remote") {
            app.show_remote = true;
            app.start_remote();
        }
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
        let channels = config.channels().max(1) as usize;

        let same_rate = self
            .audio_engine
            .lock()
            .map(|e| (e.sample_rate() - sample_rate).abs() < 1.0)
            .unwrap_or(false);
        if !same_rate {
            self.audio_engine = Arc::new(Mutex::new(AudioEngine::new(sample_rate)));
            self.audio = AudioEngine::new(sample_rate);
        }
        let engine = self.audio_engine.clone();

        let stream = device.build_output_stream(
            &config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                match engine.lock() {
                    Ok(mut eng) => {
                        // One engine sample per *frame*, copied to every channel. Writing a
                        // new sample per channel would run the engine at 2x speed in stereo.
                        for frame in data.chunks_mut(channels) {
                            let s = eng.next_sample();
                            for out in frame.iter_mut() {
                                *out = s;
                            }
                        }
                    }
                    Err(_) => data.fill(0.0),
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

    /// Lowest pitch visible in the melodic piano roll.
    pub fn piano_roll_low(&self) -> u8 {
        let rows = crate::ui::piano_roll_visible_pitch_count() as i32;
        (self.piano_roll_high() as i32 - rows + 1).max(0) as u8
    }

    /// Highest pitch visible in the melodic piano roll.
    pub fn piano_roll_high(&self) -> u8 {
        self.piano_roll_top.min(127)
    }

    /// Scroll the piano roll view by `semitones` (negative = down toward the bass).
    /// The view can reach the whole MIDI range, 0 (C-1) to 127 (G9).
    pub fn adjust_piano_roll_pitch(&mut self, semitones: i32) {
        if semitones == 0 {
            return;
        }
        let rows = crate::ui::piano_roll_visible_pitch_count() as i32;
        let min_top = (rows - 1).clamp(0, 127);
        self.piano_roll_top = (self.piano_roll_top as i32 + semitones).clamp(min_top, 127) as u8;
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
            // `add_clip` keeps clips sorted by start time, so the new clip is not
            // necessarily last: look it up by id.
            track.clips.iter().position(|c| c.id == id).unwrap_or(0)
        } else {
            0
        }
    }

    /// The bubble on `track_idx` whose visible span contains `beat`, if any.
    fn clip_at_beat(&self, track_idx: usize, beat: f64) -> Option<usize> {
        self.project.tracks.get(track_idx)?.clips.iter().position(|c| {
            beat >= c.start_beat - 0.001 && beat < c.start_beat + c.visible_length()
        })
    }

    // ---- Clip selection, copy / paste / duplicate / merge -------------------------

    /// Indices (on the current track) of every selected clip, in timeline order.
    pub fn selected_clip_indices(&self) -> Vec<usize> {
        let Some(track) = self.project.tracks.get(self.current_track) else {
            return Vec::new();
        };
        track
            .clips
            .iter()
            .enumerate()
            .filter(|(i, c)| self.selected_clip == Some(*i) || self.selected_clip_ids.contains(&c.id))
            .map(|(i, _)| i)
            .collect()
    }

    /// Plain click on a bubble: select just that clip.
    pub fn select_single_clip(&mut self, track_idx: usize, clip_idx: usize) {
        self.select_track(track_idx);
        self.selected_clip_ids.clear();
        self.selected_clip = Some(clip_idx);
    }

    /// Ctrl/Shift+click on a bubble: add it to (or remove it from) the selection.
    /// Clicking a bubble on another track starts a new selection there.
    pub fn toggle_clip_selected(&mut self, track_idx: usize, clip_idx: usize) {
        if track_idx != self.current_track {
            self.select_single_clip(track_idx, clip_idx);
            return;
        }
        let ids: Vec<u64> = self.project.tracks[track_idx].clips.iter().map(|c| c.id).collect();
        let Some(&id) = ids.get(clip_idx) else { return };
        // Fold the primary selection into the set so the set holds everything.
        if let Some(primary) = self.selected_clip.and_then(|i| ids.get(i)) {
            self.selected_clip_ids.insert(*primary);
        }
        if !self.selected_clip_ids.remove(&id) {
            self.selected_clip_ids.insert(id);
        }
        self.selected_clip = ids.iter().position(|i| self.selected_clip_ids.contains(i));
    }

    /// Select every clip on the current track.
    pub fn select_all_clips(&mut self) {
        if let Some(track) = self.project.tracks.get(self.current_track) {
            self.selected_clip_ids = track.clips.iter().map(|c| c.id).collect();
            self.selected_clip = if track.clips.is_empty() { None } else { Some(0) };
        }
    }

    fn clip_ops_blocked(&mut self) -> bool {
        if self.recording || self.count_in_target.is_some() {
            self.status_error = Some("Stop recording first.".to_string());
            return true;
        }
        false
    }

    fn selection_payload(&self) -> Option<ClipClipboard> {
        let idxs = self.selected_clip_indices();
        let track = self.project.tracks.get(self.current_track)?;
        if idxs.is_empty() {
            return None;
        }
        Some(ClipClipboard {
            format: CLIP_CLIPBOARD_FORMAT.to_string(),
            drum: track.is_drum(),
            clips: idxs.iter().map(|&i| track.clips[i].clone()).collect(),
        })
    }

    /// Copy the selected clips. Returns the JSON for the system clipboard.
    pub fn copy_selected_clips(&mut self) -> Option<String> {
        let payload = self.selection_payload()?;
        let n = payload.clips.len();
        let json = serde_json::to_string(&payload).ok();
        self.clip_clipboard = Some(payload);
        self.status_error = None;
        self.status_message = Some(format!(
            "Copied {n} clip{} (Ctrl+V pastes at the playhead)",
            if n == 1 { "" } else { "s" }
        ));
        json
    }

    /// Paste text from the system clipboard, if it holds clips copied from this app.
    pub fn paste_clipboard_text(&mut self, text: &str) -> bool {
        match serde_json::from_str::<ClipClipboard>(text) {
            Ok(payload) if payload.format == CLIP_CLIPBOARD_FORMAT => {
                self.clip_clipboard = Some(payload);
                self.paste_clips_at_playhead();
                true
            }
            _ => false,
        }
    }

    /// Paste the copied clips onto the selected track, starting at the playhead.
    pub fn paste_clips_at_playhead(&mut self) {
        let beat = self.project.playhead_beat;
        if let Some(payload) = self.clip_clipboard.clone() {
            self.paste_clips_at(&payload, beat);
        }
    }

    /// Paste `payload` so its earliest clip starts at `at_beat`, keeping the spacing
    /// between clips. The pasted clips become the selection.
    fn paste_clips_at(&mut self, payload: &ClipClipboard, at_beat: f64) {
        if self.clip_ops_blocked() || payload.clips.is_empty() {
            return;
        }
        let track_idx = self.current_track;
        let Some(track) = self.project.tracks.get(track_idx) else { return };
        if track.is_drum() != payload.drum {
            self.status_message = None;
            self.status_error = Some(if payload.drum {
                "Those are drum clips: select a drum track to paste them.".to_string()
            } else {
                "Those are melodic clips: select a melodic track to paste them.".to_string()
            });
            return;
        }
        let first = payload.clips.iter().map(|c| c.start_beat).fold(f64::MAX, f64::min);
        let delta = at_beat.max(0.0) - first;
        let mut new_ids = HashSet::new();
        let mut end = 0.0_f64;
        for clip in &payload.clips {
            let mut c = clip.clone();
            c.id = self.next_clip_id;
            self.next_clip_id += 1;
            c.start_beat = (c.start_beat + delta).max(0.0);
            c.loop_end_beat = (c.loop_end_beat + delta).max(c.start_beat + c.visible_length());
            end = end.max(c.timeline_end());
            new_ids.insert(c.id);
            self.project.tracks[track_idx].add_clip(c);
        }
        self.extend_project_to(end);
        let clips = &self.project.tracks[track_idx].clips;
        self.selected_clip = clips.iter().position(|c| new_ids.contains(&c.id));
        self.selected_clip_ids = new_ids;
        self.selected_note = None;
        self.status_error = None;
        let n = payload.clips.len();
        self.status_message = Some(format!(
            "Pasted {n} clip{} at bar {}",
            if n == 1 { "" } else { "s" },
            (at_beat / self.project.beats_per_bar().max(1.0)).floor() as i64 + 1
        ));
    }

    /// Copy the selected clips and paste them straight after the selection (Ctrl+D).
    pub fn duplicate_selected_clips(&mut self) {
        let Some(payload) = self.selection_payload() else { return };
        let start = payload.clips.iter().map(|c| c.start_beat).fold(f64::MAX, f64::min);
        let end = payload.clips.iter().map(|c| c.timeline_end()).fold(0.0, f64::max);
        // Land on a bar line so a duplicated phrase stays in time.
        let bar = self.project.beats_per_bar().max(1.0);
        let at = start + ((end - start) / bar).ceil() * bar;
        self.paste_clips_at(&payload, at);
    }

    /// Combine the selected clips into one clip that plays exactly what they played
    /// (loop repeats included), so the whole phrase can be looped or moved as one.
    pub fn merge_selected_clips(&mut self) {
        if self.clip_ops_blocked() {
            return;
        }
        let idxs = self.selected_clip_indices();
        if idxs.len() < 2 {
            self.status_message = None;
            self.status_error =
                Some("Select two or more clips to merge (Ctrl+click or Shift+click them).".to_string());
            return;
        }
        let track_idx = self.current_track;
        let bar = self.project.beats_per_bar().max(1.0);
        let clips: Vec<NoteClip> =
            idxs.iter().map(|&i| self.project.tracks[track_idx].clips[i].clone()).collect();
        let merged = merge_clips(self.next_clip_id, &clips, bar);
        self.next_clip_id += 1;
        let id = merged.id;
        let n = clips.len();

        let track = &mut self.project.tracks[track_idx];
        for &i in idxs.iter().rev() {
            track.remove_clip(i);
        }
        track.add_clip(merged);
        self.selected_clip = track.clips.iter().position(|c| c.id == id);
        self.selected_clip_ids.clear();
        self.selected_note = None;
        self.clip_drag = None;
        self.stop_playback_notes();
        self.status_error = None;
        self.status_message = Some(format!(
            "Merged {n} clips into one. Turn on \"Loop clip\" to repeat the whole phrase."
        ));
    }

    /// Delete every selected clip on the current track.
    pub fn delete_selected_clips(&mut self) {
        if self.clip_ops_blocked() {
            return;
        }
        let idxs = self.selected_clip_indices();
        if let Some(track) = self.project.tracks.get_mut(self.current_track) {
            for &i in idxs.iter().rev() {
                track.remove_clip(i);
            }
        }
        self.selected_clip = None;
        self.selected_clip_ids.clear();
        self.selected_note = None;
        self.clip_drag = None;
        self.stop_playback_notes();
    }

    /// Make sure the song is at least `end_beat` long (rounded up to a whole bar).
    fn extend_project_to(&mut self, end_beat: f64) {
        if end_beat > self.project.total_beats {
            let bar = self.project.beats_per_bar().max(1.0);
            self.project.total_beats = (end_beat / bar).ceil() * bar;
        }
    }

    /// Recording target: overdub into the bubble under the playhead, or start a new one.
    fn ensure_recording_clip(&mut self) -> Option<(usize, usize)> {
        let track_idx = self.current_track;
        if self.recording_clip.is_none() {
            let beat = self.project.playhead_beat;
            let (clip_idx, overdub) = match self.clip_at_beat(track_idx, beat) {
                Some(idx) => (idx, true),
                None => (self.create_clip(track_idx, "Recording…", beat), false),
            };
            self.recording_clip = Some((track_idx, clip_idx));
            self.recording_overdub = overdub;
            self.selected_clip = Some(clip_idx);
        }
        self.recording_clip
    }

    pub fn toggle_recording(&mut self) {
        if self.recording || self.count_in_target.is_some() {
            self.finish_recording();
        } else {
            self.start_recording();
        }
    }

    /// Press Record while stopped: one bar of count-in. The playhead jumps back a bar and
    /// rolls (with clicks) up to where you were; recording starts exactly there.
    /// While already playing, recording punches in immediately.
    pub fn start_recording(&mut self) {
        if self.recording || self.count_in_target.is_some() {
            return;
        }
        if self.playing || self.sequencer_context_active() {
            self.begin_take();
            return;
        }
        let target = self.project.playhead_beat;
        let bar = self.project.beats_per_bar();
        self.count_in_target = Some(target);
        self.count_in_last_click = -1;
        self.count_in_started = Instant::now();
        self.recording_started_transport = true;
        self.stop_playback_notes();
        self.project.playhead_beat = (target - bar).max(0.0);
        // Roll right away if there's a full bar before the target; otherwise wait at 0.
        self.playing = target - bar >= 0.0;
        self.playback_start = Instant::now();
        self.last_frame = Instant::now();
    }

    /// Clicks during the count-in and starts the take when the playhead arrives.
    fn update_count_in(&mut self) {
        let Some(target) = self.count_in_target else { return };
        let bar = self.project.beats_per_bar();
        let elapsed = self.count_in_started.elapsed().as_secs_f64() * self.project.beats_per_second();
        let virtual_beat = target - bar + elapsed;
        if virtual_beat < 0.0 {
            // Pre-roll before the song start: hold the playhead at 0.
            self.playing = false;
            self.project.playhead_beat = 0.0;
        } else if !self.playing {
            self.project.playhead_beat = virtual_beat;
            self.playing = true;
            self.last_frame = Instant::now();
        }
        let beat_idx = elapsed.floor() as i64;
        if beat_idx > self.count_in_last_click && (beat_idx as f64) < bar {
            self.count_in_last_click = beat_idx;
            let accent = beat_idx == 0;
            if let Ok(mut eng) = self.audio_engine.lock() {
                eng.synth.drum_hit(
                    crate::model::drum::DrumKind::ClosedHat,
                    if accent { 1.0 } else { 0.55 },
                );
            }
        }
        if virtual_beat >= target - 0.001 {
            self.count_in_target = None;
            // Anchor the take exactly on the target beat, not a frame late.
            let now = self.project.playhead_beat;
            self.project.playhead_beat = target;
            let started_transport = self.recording_started_transport;
            self.begin_take();
            self.recording_started_transport = started_transport;
            self.project.playhead_beat = now;
        }
    }

    /// Count-in beats left (for the transport display).
    pub fn count_in_beats_left(&self) -> Option<u32> {
        self.count_in_target.map(|_| {
            let elapsed =
                self.count_in_started.elapsed().as_secs_f64() * self.project.beats_per_second();
            (self.project.beats_per_bar() - elapsed).ceil().max(1.0) as u32
        })
    }

    /// Start writing notes now: overdub the bubble under the playhead or make a new one.
    fn begin_take(&mut self) {
        self.recording = true;
        self.recording_clip = None;
        self.recording_since = Some(Instant::now());
        if self.sequencer_context_active() {
            self.ensure_sequencer_recording_clip();
        } else {
            self.ensure_recording_clip();
        }
        self.recording_started_transport = !self.playing;
        if !self.playing {
            self.toggle_playback();
        }
    }

    pub fn finish_recording(&mut self) {
        if let Some(target) = self.count_in_target.take() {
            // Cancelled during the count-in: nothing was recorded; go back to where we were.
            self.playing = false;
            self.recording_started_transport = false;
            self.stop_playback_notes();
            self.project.playhead_beat = target;
            return;
        }
        if !self.recording {
            return;
        }
        // Notes still held: give them their final length now, and detach them from the
        // take so releasing the key later doesn't edit the finished bubble.
        let mut closing: Vec<(u8, HeldNoteState)> = Vec::new();
        for (&pitch, state) in self
            .held_note_details
            .iter_mut()
            .chain(self.remote_held.iter_mut())
        {
            if state.recording_anchor.is_some() {
                closing.push((pitch, *state));
                state.recording_anchor = None;
            }
        }
        for (pitch, state) in closing {
            self.finish_recorded_note(pitch, state);
        }
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
        self.recording_since = None;
        self.recording_overdub = false;
        if self.recording_started_transport && self.playing {
            self.toggle_playback();
        }
        self.recording_started_transport = false;
    }

    /// While recording: stretch the bubble to the playhead and grow held notes, so the
    /// take is visible as it is played.
    fn update_recording_live(&mut self) {
        if !self.recording {
            return;
        }
        let beat = self.project.playhead_beat;
        let Some((track_idx, clip_idx)) = self.recording_clip else { return };
        let held: Vec<(u8, (usize, usize, f64))> = self
            .held_note_details
            .iter()
            .chain(self.remote_held.iter())
            .filter_map(|(&p, s)| s.recording_anchor.map(|a| (p, a)))
            .collect();
        let Some(clip) = self
            .project
            .tracks
            .get_mut(track_idx)
            .and_then(|t| t.clips.get_mut(clip_idx))
        else {
            return;
        };
        let rel_now = beat - clip.start_beat;
        if rel_now > 0.0 {
            let bar = 4.0_f64;
            let needed = (rel_now / bar).ceil() * bar;
            if needed > clip.source_length {
                clip.source_length = needed;
                clip.content_length = needed;
                if !clip.loop_enabled {
                    clip.loop_end_beat = clip.start_beat + clip.visible_length();
                }
            }
        }
        for (pitch, (ti, ci, rel_start)) in held {
            if ti != track_idx || ci != clip_idx {
                continue;
            }
            if let Some(note) = clip
                .notes
                .iter_mut()
                .find(|n| n.pitch == pitch && (n.start_beat - rel_start).abs() < 0.001)
            {
                note.duration_beats = (rel_now - rel_start).max(0.125);
            }
        }
    }

    /// Scroll the piano roll's pitch window so `pitch` is on screen (the roll draws
    /// `visible_pitch_count()` rows downward from `piano_roll_high()`).
    pub fn reveal_pitch_in_roll(&mut self, pitch: u8) {
        let high = self.piano_roll_high() as i32;
        let low = high - crate::ui::piano_roll_visible_pitch_count() as i32 + 1;
        let p = pitch as i32;
        if p > high {
            self.adjust_piano_roll_pitch(p - high + 1);
        } else if p < low {
            self.adjust_piano_roll_pitch(p - low - 1);
        }
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
        self.selected_clip_ids.clear();
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
            let state = self.start_recorded_note(pitch, vel, Instant::now());
            self.held_note_details.insert(pitch, state);
        }
    }

    /// Bookkeeping for a note that has started sounding: records it at the playhead when
    /// recording. `at` is when the note actually started (remote notes arrive a frame late).
    fn start_recorded_note(&mut self, pitch: u8, vel: f32, at: Instant) -> HeldNoteState {
        let melodic = self
            .project
            .tracks
            .get(self.current_track)
            .map(|t| !t.is_drum())
            .unwrap_or(false);
        if melodic {
            self.reveal_pitch_in_roll(pitch);
        }
        let mut recording_anchor = None;
        if self.recording {
            let lag = at.elapsed().as_secs_f64() * self.project.beats_per_second();
            let beat = (self.project.playhead_beat - if self.playing { lag } else { 0.0 }).max(0.0);
            if let Some((track_idx, clip_idx)) = self.ensure_recording_clip() {
                if track_idx == self.current_track {
                    if let Some(clip) = self
                        .project
                        .tracks
                        .get(track_idx)
                        .and_then(|t| t.clips.get(clip_idx))
                    {
                        let rel = (beat - clip.start_beat).max(0.0);
                        let abs = clip.start_beat + rel;
                        self.add_note_to_clip_relative(track_idx, clip_idx, pitch, rel, 0.125, vel);
                        // It is already sounding live; playback must not fire it again.
                        self.playback_events_done
                            .insert(playback_event_key(track_idx, clip_idx, abs, pitch));
                        recording_anchor = Some((track_idx, clip_idx, rel));
                    }
                }
            }
        }
        HeldNoteState {
            press_start: at,
            peak_velocity: vel,
            recording_anchor,
        }
    }

    fn finish_recorded_note(&mut self, pitch: u8, state: HeldNoteState) {
        if let Some((track_idx, clip_idx, rel_start)) = state.recording_anchor {
            let hold_secs = state.press_start.elapsed().as_secs_f64();
            let mut duration = (hold_secs * self.project.beats_per_second()).clamp(0.125, 16.0);
            // While the transport rolls, the playhead is the truth (matches the live view).
            if self.playing {
                if let Some(clip) = self.project.tracks.get(track_idx).and_then(|t| t.clips.get(clip_idx)) {
                    let by_playhead = self.project.playhead_beat - (clip.start_beat + rel_start);
                    if by_playhead > 0.0 {
                        duration = by_playhead.clamp(0.125, 64.0);
                    }
                }
            }
            if let Some(track) = self.project.tracks.get_mut(track_idx) {
                if let Some(clip) = track.clips.get_mut(clip_idx) {
                    for note in &mut clip.notes {
                        if note.pitch == pitch && (note.start_beat - rel_start).abs() < 0.001 {
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
            self.finish_recorded_note(pitch, state);
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

    /// Delete a track straight away if it's empty, otherwise ask for confirmation first.
    pub fn request_delete_track(&mut self, index: usize) {
        let Some(track) = self.project.tracks.get(index) else { return };
        if track.clips.iter().all(|c| c.notes.is_empty()) {
            self.delete_track(index);
        } else {
            self.pending_track_delete = Some(index);
        }
    }

    /// Remove a track and fix up everything that refers to tracks by index.
    /// The last remaining track can't be deleted (the editors assume one exists).
    pub fn delete_track(&mut self, index: usize) {
        self.pending_track_delete = None;
        if index >= self.project.tracks.len() {
            return;
        }
        if self.project.tracks.len() <= 1 {
            self.status_error = Some("Can't delete the only track.".to_string());
            return;
        }
        // Close any take and silence playback first: held notes, the recording clip and
        // the played-events set all hold track indices.
        self.finish_recording();
        self.stop_playback_notes();
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.stop_all();
        }

        let name = self.project.tracks.remove(index).name;

        self.selected_clip = None;
        self.selected_clip_ids.clear();
        self.selected_note = None;
        self.note_drag = None;
        self.clip_drag = None;
        self.section_note_indices = None;
        self.section_drag_origins.clear();
        self.section_clip_index = None;
        match self.sequencer_draft_track {
            Some(t) if t == index => {
                self.sequencer_draft_track = None;
                self.sequencer_draft.clear();
            }
            Some(t) if t > index => self.sequencer_draft_track = Some(t - 1),
            _ => {}
        }
        if self.current_track > index || self.current_track >= self.project.tracks.len() {
            self.current_track = self.current_track.saturating_sub(1);
        }
        self.sync_instrument_to_track();
        self.sync_sequencer_visibility();
        self.status_error = None;
        self.status_message = Some(format!("Deleted track \"{name}\""));
    }

    pub fn select_track(&mut self, index: usize) {
        if index < self.project.tracks.len() {
            if index != self.current_track {
                self.selected_clip_ids.clear();
            }
            self.current_track = index;
            self.sync_instrument_to_track();
        }
    }

    pub fn set_playhead(&mut self, beat: f64) {
        let beat = beat.clamp(0.0, self.project.total_beats);
        if (beat - self.project.playhead_beat).abs() > 1e-6 {
            // Moving the playhead: silence notes from the old spot and let notes at the
            // new spot trigger again.
            self.stop_playback_notes();
        }
        self.project.playhead_beat = beat;
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
        // Read keys first, act after: never call back into egui (or do real work) while
        // `ctx.input` holds the context lock.
        let typing = ctx.wants_keyboard_input();
        let (home, end) = ctx.input(|i| (i.key_pressed(egui::Key::Home), i.key_pressed(egui::Key::End)));
        // Consume Space (this runs before any panel) so a focused button can't also
        // "click" on it — otherwise one press would stop *and* restart a take.
        let space = !typing
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space));
        if home {
            self.go_to_start();
        }
        if end {
            self.go_to_end();
        }
        // Clip shortcuts. egui turns Ctrl+C / Ctrl+V into Copy / Paste events (a paste
        // only when the OS clipboard has text, which copying clips guarantees).
        if !typing {
            let (copy, paste, dup) = ctx.input(|i| {
                let copy = i.events.iter().any(|e| matches!(e, egui::Event::Copy));
                let paste = i.events.iter().find_map(|e| match e {
                    egui::Event::Paste(text) => Some(text.clone()),
                    _ => None,
                });
                (copy, paste, i.modifiers.command && i.key_pressed(egui::Key::D))
            });
            if copy {
                if let Some(json) = self.copy_selected_clips() {
                    ctx.copy_text(json);
                }
            }
            if let Some(text) = paste {
                self.paste_clipboard_text(&text);
            }
            if dup {
                self.duplicate_selected_clips();
            }
        }
        if space {
            // Space = stop the take if recording, otherwise play/pause.
            if self.recording {
                self.finish_recording();
            } else {
                self.toggle_playback();
            }
        }
    }

    pub fn toggle_playback(&mut self) {
        if self.count_in_target.is_some() {
            self.finish_recording(); // cancels the count-in
            return;
        }
        if self.playing && self.recording {
            // Pausing ends the take (and then pauses, via finish_recording).
            self.recording_started_transport = true;
            self.finish_recording();
            return;
        }
        self.playing = !self.playing;
        if self.playing {
            self.playback_start = Instant::now();
            self.last_frame = Instant::now();
        } else {
            self.stop_playback_notes();
        }
    }

    pub fn stop_playback(&mut self) {
        self.finish_recording();
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
        // Notes being played live right now are already sounding: don't let playback
        // re-trigger them as the playhead passes their (just recorded) start.
        let live: Vec<(usize, usize, f64, u8)> = self
            .held_note_details
            .iter()
            .chain(self.remote_held.iter())
            .filter_map(|(&p, s)| s.recording_anchor.map(|(t, c, r)| (t, c, r, p)))
            .collect();
        for (track_idx, track) in self.project.tracks.iter().enumerate() {
            if track.muted {
                continue;
            }
            let is_drum = track.is_drum();
            for (clip_idx, clip) in track.clips.iter().enumerate() {
                for (note_idx, note) in clip.notes.iter().enumerate() {
                    if live.iter().any(|&(t, c, r, p)| {
                        t == track_idx && c == clip_idx && p == note.pitch && (r - note.start_beat).abs() < 0.001
                    }) {
                        continue;
                    }
                    let note_end = clip.note_absolute_end(note);
                    let tolerance = 0.05;

                    for trigger_beat in track.note_trigger_times(clip, note, self.project.total_beats) {
                        let event_key = playback_event_key(track_idx, clip_idx, trigger_beat, note.pitch);

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
        let id = InstrumentId::from_sample_kind(kind);
        if !self.instrument_fits_current_track(&id) {
            return;
        }
        let name = kind.label();
        self.selected_instrument = name.to_string();
        if let Some(track) = self.project.tracks.get_mut(self.current_track) {
            track.instrument = id;
        }
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.set_profile(name);
        }
        self.audio.set_profile(name);
    }

    pub fn select_custom_instrument(&mut self, name: &str) {
        if !self.instrument_fits_current_track(&InstrumentId::Custom(name.to_string())) {
            return;
        }
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

    /// Chords go on melodic tracks only; explain otherwise.
    fn composition_target_ok(&mut self) -> bool {
        let melodic = self
            .project
            .tracks
            .get(self.current_track)
            .map(|t| !t.is_drum())
            .unwrap_or(false);
        if !melodic {
            self.status_message = None;
            self.status_error = Some("Select a melodic track to add chords.".to_string());
        }
        melodic && !self.clip_ops_blocked()
    }

    /// Add one chord, a whole bar long, at the playhead and move the playhead to the next
    /// bar, so clicking chords one after another builds a progression in order.
    pub fn insert_chord_from_roman(&mut self, symbol: &str, key: &Key) {
        let Some(chord) = resolve_chord(symbol, key) else { return };
        if !self.composition_target_ok() {
            return;
        }
        let bar = self.project.beats_per_bar().max(1.0);
        let at = self.project.playhead_beat;
        let voicing = chord.voicing();
        self.add_note_batch_clip(voicing.iter().map(|&p| (p, 0.0, bar, 0.8)).collect());
        self.name_selected_clip(chord.name());
        self.extend_project_to(at + bar);
        for &p in &voicing {
            self.play_note_at_preview(p, 0.7);
        }
        self.set_playhead(at + bar);
        self.status_error = None;
        self.status_message = Some(format!("Added {} ({symbol}) in {}", chord.name(), key.name()));
    }

    /// Add a whole progression (one bar per chord) as a single clip at the playhead,
    /// ready to loop.
    pub fn insert_progression(&mut self, name: &str, symbols: &[String], key: &Key) {
        let chords: Vec<_> = symbols.iter().filter_map(|s| resolve_chord(s, key)).collect();
        if chords.is_empty() || !self.composition_target_ok() {
            return;
        }
        let bar = self.project.beats_per_bar().max(1.0);
        let at = self.project.playhead_beat;
        let mut batch = Vec::new();
        for (i, chord) in chords.iter().enumerate() {
            for p in chord.voicing() {
                batch.push((p, i as f64 * bar, bar, 0.8));
            }
        }
        self.add_note_batch_clip(batch);
        self.name_selected_clip(format!("{name} in {}", key.name()));
        self.extend_project_to(at + chords.len() as f64 * bar);
        self.status_error = None;
        self.status_message = Some(format!(
            "Added {name} ({} bars). Turn on \"Loop clip\" in Track Controls to repeat it.",
            chords.len()
        ));
    }

    fn name_selected_clip(&mut self, name: String) {
        if let Some(ci) = self.selected_clip {
            if let Some(clip) = self.project.tracks.get_mut(self.current_track).and_then(|t| t.clips.get_mut(ci)) {
                clip.name = name;
            }
        }
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
        let kit = self.drum_instrument_for(self.current_track);
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.play_note_for_instrument(pitch, velocity, &kit, 1.0);
        }
    }

    /// The drum kit a track plays (genre kit or the FreePats kit).
    fn drum_instrument_for(&self, track_idx: usize) -> InstrumentId {
        self.project
            .tracks
            .get(track_idx)
            .map(|t| t.instrument.clone())
            .filter(|i| i.is_drum_track())
            .unwrap_or(InstrumentId::Drums)
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

    /// Show the step sequencer for drums: selects a drum track (adding one if there is
    /// none) and switches the bottom panel to the sequencer view.
    pub fn open_drum_sequencer(&mut self) {
        let on_drums = self
            .project
            .tracks
            .get(self.current_track)
            .map(|t| t.is_drum())
            .unwrap_or(false);
        if !on_drums {
            match self.project.tracks.iter().position(|t| t.is_drum()) {
                Some(i) => self.select_track(i),
                None => {
                    self.project.add_drum_track();
                    self.current_track = self.project.tracks.len() - 1;
                    self.sync_instrument_to_track();
                }
            }
        }
        self.drum_view_sequencer = true;
        self.sync_sequencer_visibility();
    }

    pub fn close_drum_sequencer(&mut self) {
        self.sequencer_popped_out = false;
        self.drum_view_sequencer = false;
        self.sync_sequencer_visibility();
    }

    /// The sequencer is visible when a drum track is selected and its view is the
    /// sequencer (docked) or it's popped out. Entering/leaving refreshes or stops it.
    pub fn sync_sequencer_visibility(&mut self) {
        let on_drums = self
            .project
            .tracks
            .get(self.current_track)
            .map(|t| t.is_drum())
            .unwrap_or(false);
        let docked = self.drum_view_sequencer && !self.bottom_keyboard;
        let visible = on_drums && (docked || self.sequencer_popped_out);
        if visible && !self.show_drum_sequencer {
            self.show_drum_sequencer = true;
            // Don't start looping on its own just because a drum track was selected.
            self.sequencer_running = false;
            self.refresh_sequencer_draft();
        } else if !visible && self.show_drum_sequencer {
            self.show_drum_sequencer = false;
            self.sequencer_running = false;
        }
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
        let kit = self.drum_instrument_for(track_idx);
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
                    eng.play_note_for_instrument(pitch, velocity, &kit, 1.0);
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
        self.sync_sequencer_visibility();
        self.update_playback();
        self.update_count_in();
        self.update_recording_live();
        self.update_sequencer_preview();
        let (live_instrument, live_filter) = self.current_track_playback_info();
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.master_volume = self.audio.master_volume;
            eng.master_effects = self.audio.master_effects.clone();
            eng.live_instrument = live_instrument;
            eng.live_filter = live_filter;
            eng.velocity_curve = self.velocity_curve;
        }
        self.process_remote_events();
        crate::ui::instrument_designer::tick_preview(self);
        self.handle_transport_shortcuts(ctx);
        if self.bottom_keyboard || self.show_piano {
            crate::ui::piano_keyboard_input(self, ctx);
        } else if !self.keyboard_held_notes.is_empty() {
            // Piano hidden mid-press: don't leave notes hanging.
            for pitch in std::mem::take(&mut self.keyboard_held_notes) {
                self.release_note(pitch);
            }
        }
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
        crate::ui::show_delete_track_confirm(self, ctx);

        if self.show_remote {
            let mut show = self.show_remote;
            egui::Window::new("📱 Remote Keyboard")
                .open(&mut show)
                .default_size([460.0, 560.0])
                .show(ctx, |ui| {
                    crate::ui::show_remote_panel(self, ui);
                });
            self.show_remote = show;
        }

        // Only when popped out; normally the sequencer is docked in the bottom panel.
        if self.sequencer_popped_out && self.show_drum_sequencer {
            let mut show = true;
            egui::Window::new("Beat Sequencer")
                .open(&mut show)
                .default_size([620.0, 420.0])
                .show(ctx, |ui| {
                    crate::ui::show_drum_sequencer(self, ui);
                });
            if !show {
                // Closing the window docks it back into the bottom panel.
                self.sequencer_popped_out = false;
                self.drum_view_sequencer = true;
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
    // Reserve the whole bottom panel: its header rows (title, zoom, hint ≈ 100 px) plus the
    // roll itself, so the lowest piano-roll rows don't run off the bottom of the window.
    let available = ui.available_height() - crate::ui::constants::PIANO_ROLL_PANEL_HEIGHT - 100.0;
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


/// Identifies one scheduled note-on during playback, so each fires once per pass.
fn playback_event_key(track_idx: usize, clip_idx: usize, trigger_beat: f64, pitch: u8) -> u32 {
    ((track_idx as u32) << 22)
        | ((clip_idx as u32) << 16)
        | (((trigger_beat * 100.0) as u32) & 0xFFFF)
        | ((pitch as u32) << 8)
}

/// Ask the default output device for its sample rate without opening a stream.
fn probe_output_sample_rate() -> Option<f32> {
    let device = cpal::default_host().default_output_device()?;
    let config = device.default_output_config().ok()?;
    Some(config.sample_rate().0 as f32)
}

impl DAWApp {
    /// Every instrument the user can pick, grouped for menus (desktop and phone).
    pub fn instrument_choices(&self) -> Vec<(String, InstrumentId)> {
        let mut out = Vec::new();
        for &genre in Genre::all() {
            for &p in GenrePreset::all().iter().filter(|p| p.genre() == genre) {
                out.push((genre.label().to_string(), InstrumentId::Preset(p)));
            }
            for &k in DrumKit::all().iter().filter(|k| k.genre() == genre) {
                out.push((genre.label().to_string(), InstrumentId::Kit(k)));
            }
            if genre == Genre::Classical {
                out.push((genre.label().to_string(), InstrumentId::Piano));
                out.push((genre.label().to_string(), InstrumentId::Harp));
            }
        }
        for &kind in SampleKind::all() {
            if matches!(kind, SampleKind::Piano | SampleKind::Harp) {
                continue;
            }
            out.push(("Sampled".to_string(), InstrumentId::from_sample_kind(kind)));
        }
        out.push(("Sampled".to_string(), InstrumentId::Drums));
        for c in &self.custom_instruments {
            out.push(("My Instruments".to_string(), InstrumentId::Custom(c.name.clone())));
        }
        out
    }

    /// Instruments that can go on track `idx`: drum tracks take only drum kits and melodic
    /// tracks only melodic instruments. Swapping across would reinterpret the notes (a drum
    /// pattern becomes random pitches) and swap the step sequencer for the piano roll.
    pub fn instrument_fits_track(&self, idx: usize, instrument: &InstrumentId) -> bool {
        self.project
            .tracks
            .get(idx)
            .map(|t| t.is_drum() == instrument.is_drum_track())
            .unwrap_or(false)
    }

    /// Like [`Self::instrument_fits_track`] for the selected track; explains a refusal in the
    /// status bar.
    fn instrument_fits_current_track(&mut self, instrument: &InstrumentId) -> bool {
        if self.instrument_fits_track(self.current_track, instrument) {
            return true;
        }
        let drum = self
            .project
            .tracks
            .get(self.current_track)
            .map(|t| t.is_drum())
            .unwrap_or(false);
        self.status_message = None;
        self.status_error = Some(if drum {
            format!(
                "Drum tracks can only use drum kits. Add a melodic track for {}.",
                instrument.label()
            )
        } else {
            format!(
                "{} is a drum kit. Add a drum track to use it.",
                instrument.label()
            )
        });
        false
    }

    /// Put an instrument on the selected track (from any picker, or a remote device).
    /// Returns false (and leaves the track alone) if the instrument is the wrong kind.
    pub fn set_track_instrument(&mut self, instrument: InstrumentId) -> bool {
        if !self.instrument_fits_current_track(&instrument) {
            return false;
        }
        if let Some(track) = self.project.tracks.get_mut(self.current_track) {
            track.instrument = instrument;
        }
        self.status_error = None;
        self.sync_instrument_to_track();
        true
    }

    pub fn start_remote(&mut self) {
        if self.remote.is_some() {
            return;
        }
        match RemoteServer::start(
            self.audio_engine.clone(),
            crate::remote::DEFAULT_HTTP_PORT,
            crate::remote::DEFAULT_HTTPS_PORT,
        ) {
            Ok(server) => {
                server.set_curve(self.velocity_curve);
                server.set_chain(self.remote_chain, self.remote_chain_start);
                self.remote = Some(server);
                self.remote_error = None;
            }
            Err(e) => self.remote_error = Some(e),
        }
    }

    pub fn stop_remote(&mut self) {
        if let Some(server) = self.remote.take() {
            server.stop();
        }
        let held: Vec<u8> = self.remote_held.keys().copied().collect();
        for pitch in held {
            self.remote_note_off(pitch);
        }
    }

    fn remote_note_off(&mut self, pitch: u8) {
        if let Some(state) = self.remote_held.remove(&pitch) {
            self.finish_recorded_note(pitch, state);
        }
    }

    /// Notes from phones were already played by the network thread; here we only record
    /// them, light up keys, and keep menus in sync.
    fn process_remote_events(&mut self) {
        let Some(server) = &self.remote else { return };
        let events = server.poll_events();
        let choices: Vec<(String, InstrumentId)> = self
            .instrument_choices()
            .into_iter()
            .filter(|(_, id)| self.instrument_fits_track(self.current_track, id))
            .collect();
        let current = self.current_instrument_name();
        if let Some(server) = &self.remote {
            server.set_instruments(
                choices
                    .iter()
                    .map(|(group, id)| InstrumentChoice {
                        group: group.clone(),
                        name: id.label(),
                    })
                    .collect(),
                &current,
            );
            server.set_curve(self.velocity_curve);
            let (state, beats) = match self.count_in_beats_left() {
                Some(b) => ("count", b),
                None if self.recording => ("rec", 0),
                None => ("off", 0),
            };
            server.set_record_state(state, beats);
        }
        for event in events {
            match event {
                RemoteEvent::NoteOn { pitch, velocity, at, .. } => {
                    if let Some(old) = self.remote_held.remove(&pitch) {
                        self.finish_recorded_note(pitch, old);
                    }
                    let state = self.start_recorded_note(pitch, velocity, at);
                    self.remote_held.insert(pitch, state);
                }
                RemoteEvent::NoteOff { pitch, .. } => self.remote_note_off(pitch),
                RemoteEvent::SelectInstrument(label) => {
                    let id = InstrumentId::from_label(&label);
                    let known = choices.iter().any(|(_, c)| *c == id);
                    if known && self.set_track_instrument(id) {
                        self.status_message = Some(format!("Remote: switched to {}", label));
                    }
                }
                RemoteEvent::SetCurve(curve) => self.velocity_curve = curve,
                RemoteEvent::Record(on) => {
                    if on {
                        self.start_recording();
                        self.status_message = Some("Remote: recording (1-bar count-in)".into());
                    } else {
                        self.finish_recording();
                        self.status_message = Some("Remote: recording stopped".into());
                    }
                }
                RemoteEvent::DevicesChanged => {}
            }
        }
    }
}

/// One clip holding every note `clips` would play, at the same times. Loop repeats are
/// written out as notes and gaps between clips are kept as rests. The clip runs from the
/// first clip's start to the last clip's end, rounded up to whole bars of `bar` beats.
pub fn merge_clips(id: u64, clips: &[NoteClip], bar: f64) -> NoteClip {
    let start = clips.iter().map(|c| c.start_beat).fold(f64::MAX, f64::min).max(0.0);
    let end = clips.iter().map(|c| c.timeline_end()).fold(start, f64::max);
    let name = match clips.first() {
        Some(c) if clips.len() > 1 => format!("{} +{}", c.name, clips.len() - 1),
        Some(c) => c.name.clone(),
        None => "Merged".to_string(),
    };
    let mut merged = NoteClip::new(id, name, start);
    for clip in clips {
        for note in &clip.notes {
            for t in clip.note_trigger_times(note, f64::INFINITY) {
                let mut n = note.clone();
                n.start_beat = t - start;
                merged.notes.push(n);
            }
        }
    }
    merged.notes.sort_by(|a, b| {
        a.start_beat
            .partial_cmp(&b.start_beat)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.pitch.cmp(&b.pitch))
    });
    let bar = bar.max(1.0);
    merged.source_length = (((end - start) / bar).ceil() * bar).max(bar);
    merged.content_length = merged.source_length;
    merged.recompute_bounds();
    merged.loop_end_beat = merged.start_beat + merged.visible_length();
    merged
}

#[cfg(test)]
mod clip_merge_tests {
    use super::merge_clips;
    use crate::model::{Note, NoteClip};

    fn clip(id: u64, start: f64, notes: &[(u8, f64)]) -> NoteClip {
        let mut c = NoteClip::new(id, format!("C{id}"), start);
        for &(p, b) in notes {
            c.add_note(Note::new(p, b, 1.0));
        }
        c
    }

    #[test]
    fn merge_keeps_absolute_times_and_gaps() {
        // Chord in bar 1, chord in bar 3 (bar 2 empty).
        let a = clip(1, 0.0, &[(60, 0.0), (64, 0.0)]);
        let b = clip(2, 8.0, &[(65, 0.0), (69, 0.0)]);
        let m = merge_clips(10, &[a, b], 4.0);
        assert_eq!(m.start_beat, 0.0);
        let got: Vec<(u8, f64)> = m.notes.iter().map(|n| (n.pitch, n.start_beat)).collect();
        assert_eq!(got, vec![(60, 0.0), (64, 0.0), (65, 8.0), (69, 8.0)]);
        assert_eq!(m.visible_length(), 12.0);
    }

    #[test]
    fn merge_writes_out_loop_repeats() {
        let mut a = clip(1, 4.0, &[(60, 0.0)]);
        a.loop_enabled = true;
        a.loop_end_beat = 12.0; // a 4-beat clip played twice
        let b = clip(2, 12.0, &[(67, 1.0)]);
        let m = merge_clips(10, &[a, b], 4.0);
        assert_eq!(m.start_beat, 4.0);
        let got: Vec<f64> = m.notes.iter().map(|n| n.start_beat).collect();
        assert_eq!(got, vec![0.0, 4.0, 9.0]);
        assert_eq!(m.visible_length(), 12.0);
    }
}
