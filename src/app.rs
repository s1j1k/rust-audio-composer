use crate::audio::export::export_project_wav;
use crate::audio::AudioEngine;
use crate::model::instrument::{CustomInstrument, InstrumentId, InstrumentProfile, SampleKind};
use crate::model::{Note, Project, SavedProject};
use crate::music::Key;
use crate::music::theory::NOTE_NAMES;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Instant;

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

    pub held_notes: HashSet<u8>,
    pub playback_notes_active: HashSet<u8>,
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

            held_notes: HashSet::new(),
            playback_notes_active: HashSet::new(),
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
        };

        if let Ok(stream) = app.setup_audio_stream() {
            app.audio_stream = Some(stream);
        }

        app.sync_instrument_to_track();
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
        if !self.held_notes.contains(&pitch) {
            self.held_notes.insert(pitch);
            let velocity = 0.8;
            if let Ok(mut eng) = self.audio_engine.lock() {
                eng.play_note(pitch, velocity);
            }

            if self.recording {
                let beat = self.project.playhead_beat;
                let note = Note::new(pitch, beat, 1.0);
                if let Some(track) = self.project.tracks.get_mut(self.current_track) {
                    track.add_note(note);
                }
            }
        }
    }

    pub fn release_note(&mut self, pitch: u8) {
        self.held_notes.remove(&pitch);
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.stop_note(pitch);
        }
    }

    pub fn add_note_at(&mut self, pitch: u8, start_beat: f64, duration: f64) {
        let note = Note::new(pitch, start_beat, duration);
        if let Some(track) = self.project.tracks.get_mut(self.current_track) {
            track.add_note(note);
        }
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.play_note(pitch, 0.8);
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
        self.project.playhead_beat = 0.0;
        self.stop_playback_notes();
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
        for (_track_idx, track) in self.project.tracks.iter().enumerate() {
            if track.muted {
                continue;
            }
            for note in &track.notes {
                let note_start = note.start_beat;
                let note_end = note.end_beat();
                let tolerance = 0.05;

                if (beat - note_start).abs() < tolerance
                    && !self.playback_notes_active.contains(&note.pitch)
                {
                    if let Ok(mut eng) = self.audio_engine.lock() {
                        let inst_name = track.instrument.label();
                        eng.set_profile(&inst_name);
                        eng.play_note(note.pitch, note.velocity);
                    }
                    self.playback_notes_active.insert(note.pitch);
                }

                if beat >= note_end && self.playback_notes_active.contains(&note.pitch) {
                    if let Ok(mut eng) = self.audio_engine.lock() {
                        eng.stop_note(note.pitch);
                    }
                    self.playback_notes_active.remove(&note.pitch);
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
                self.audio.master_volume,
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
        self.audio.master_volume = saved.master_volume;
        self.current_track = 0;
        if self.current_track >= self.project.tracks.len() && !self.project.tracks.is_empty() {
            self.current_track = self.project.tracks.len() - 1;
        }

        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.master_volume = saved.master_volume;
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
                    for offset in [0u8, 4, 7] {
                        self.add_note_at(base + offset - (base % 12) + (pc as u8), self.project.playhead_beat, 2.0);
                    }
                    return;
                }
                &[0, 4, 7]
            }
        };

        let beat = self.project.playhead_beat;
        for &interval in intervals {
            let pc = (root_pc + interval) % 12;
            let pitch = 60 + pc;
            self.add_note_at(pitch, beat, 2.0);
        }
    }
}

impl eframe::App for DAWApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_playback();
        if let Ok(mut eng) = self.audio_engine.lock() {
            eng.master_volume = self.audio.master_volume;
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
