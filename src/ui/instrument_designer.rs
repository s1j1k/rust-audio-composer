//! Sound Designer: describe a timbre in plain words, hear it, then keep talking to shape it.

use crate::ai::instrument_gen::{self, describe_params, interpret, refine, Base};
use crate::app::DAWApp;
use crate::audio::note_render::{default_hold_secs, normalize, render_note, MAX_NOTE_SECS};
use crate::model::instrument::{
    CustomInstrument, DrumKit, Genre, GenrePreset, InstrumentId, InstrumentProfile, NoteSample,
    ProfileSample, SampleKind, SynthParams,
};
use crate::music::theory::{name_to_pitch, pitch_to_name};
use crate::ui::waveform_editor::{self, WaveEditorState};
use egui::{Color32, RichText, Ui};
use std::time::{Duration, Instant};

const PREVIEW_PROFILE: &str = "__preview__";

/// Undo depth. Each step holds a copy of the note's audio (a few hundred KB).
const MAX_UNDO: usize = 40;

/// One undo step: the settings and the audio as they were.
pub type Snapshot = (SynthParams, Option<NoteSample>);

const EXAMPLES: &[&str] = &[
    "dark gritty 808 with a long tail",
    "dusty lofi rhodes, warm and a bit wobbly",
    "bright UK garage organ stab",
    "huge detuned reese for jungle",
    "soft felt piano with a gentle attack",
    "lush cinematic strings that swell slowly",
    "glassy trap bell, short and crisp",
];

const QUICK: &[(&str, &str)] = &[
    ("Warmer", "warmer"),
    ("Brighter", "brighter"),
    ("Punchier", "punchier"),
    ("Softer attack", "soft attack"),
    ("Longer tail", "longer"),
    ("Shorter", "shorter"),
    ("Wider", "wider"),
    ("Grittier", "grittier"),
    ("Cleaner", "cleaner"),
    ("Vintage", "vintage"),
];

#[derive(Clone, Copy)]
enum PreviewEv {
    On(u8, f32),
    Off(u8),
}

pub struct DesignerState {
    pub description: String,
    pub refine_input: String,
    pub base_override: Option<Base>,
    pub current: Option<CustomInstrument>,
    pub undo: Vec<Snapshot>,
    pub history: Vec<(String, String)>,
    pub name: String,
    pub auto_preview: bool,
    /// Pitch new notes are rendered at.
    pub render_root: u8,
    /// How long the key is held when rendering; `None` picks from the envelope.
    pub render_hold: Option<f32>,
    pub wave: WaveEditorState,
    /// Settings when a fine-tune slider drag began (the undo point for that drag).
    drag_params_start: Option<SynthParams>,
    preview_queue: Vec<(Instant, PreviewEv)>,
    preview_instrument: Option<InstrumentId>,
}

impl Default for DesignerState {
    fn default() -> Self {
        Self {
            description: String::new(),
            refine_input: String::new(),
            base_override: None,
            current: None,
            undo: Vec::new(),
            history: Vec::new(),
            name: String::new(),
            auto_preview: true,
            render_root: 60,
            render_hold: None,
            wave: WaveEditorState::default(),
            drag_params_start: None,
            preview_queue: Vec::new(),
            preview_instrument: None,
        }
    }
}

/// Called every frame by the app: plays queued preview notes on time (so sustained
/// sounds like pads always get their note-off, even if the window is closed).
pub fn tick_preview(app: &mut DAWApp) {
    if app.designer.preview_queue.is_empty() {
        return;
    }
    let now = Instant::now();
    let Some(inst) = app.designer.preview_instrument.clone() else {
        app.designer.preview_queue.clear();
        return;
    };
    let due: Vec<PreviewEv> = app
        .designer
        .preview_queue
        .iter()
        .filter(|(t, _)| *t <= now)
        .map(|(_, e)| *e)
        .collect();
    app.designer.preview_queue.retain(|(t, _)| *t > now);
    if due.is_empty() {
        return;
    }
    if let Ok(mut eng) = app.audio_engine.lock() {
        for e in due {
            match e {
                PreviewEv::On(n, v) => eng.play_note_for_instrument(n, v, &inst, 1.0),
                PreviewEv::Off(n) => eng.stop_note(n),
            }
        }
    }
}

/// Release anything still sounding from the last preview.
fn release_pending_preview(app: &mut DAWApp) {
    let pending_offs: Vec<u8> = app
        .designer
        .preview_queue
        .iter()
        .filter_map(|(_, e)| match e {
            PreviewEv::Off(n) => Some(*n),
            _ => None,
        })
        .collect();
    if let Ok(mut eng) = app.audio_engine.lock() {
        for n in pending_offs {
            eng.stop_note(n);
        }
    }
}

/// Queue a short phrase that shows off velocity: a soft hit then a hard one.
pub fn preview_phrase(app: &mut DAWApp, instrument: InstrumentId, bass: bool, drums: bool) {
    release_pending_preview(app);
    let t0 = Instant::now();
    let at = |s: f32| t0 + Duration::from_secs_f32(s);
    let mut q = Vec::new();
    let mut note = |start: f32, len: f32, n: u8, v: f32| {
        q.push((at(start), PreviewEv::On(n, v)));
        q.push((at(start + len), PreviewEv::Off(n)));
    };
    if drums {
        for (i, (n, v)) in [(36, 0.95), (42, 0.5), (38, 0.9), (42, 0.35), (36, 0.8), (36, 0.6), (38, 1.0), (46, 0.7)]
            .iter()
            .enumerate()
        {
            note(i as f32 * 0.18, 0.15, *n, *v);
        }
    } else if bass {
        note(0.0, 0.42, 36, 0.45);
        note(0.5, 0.42, 36, 0.95);
        note(1.0, 0.35, 43, 0.75);
        note(1.45, 0.7, 39, 0.9);
    } else {
        for n in [60, 64, 67, 71] {
            note(0.0, 0.9, n, 0.4);
        }
        for n in [62, 65, 69, 72] {
            note(1.05, 1.0, n, 0.95);
        }
    }
    app.designer.preview_queue = q;
    app.designer.preview_instrument = Some(instrument);
}

/// Point the hidden preview slot at the sound being designed (settings + audio).
fn load_preview_profile(app: &mut DAWApp) -> Option<bool> {
    let custom = app.designer.current.as_ref()?;
    let profile = InstrumentProfile {
        name: PREVIEW_PROFILE.to_string(),
        params: custom.params.clone(),
        sample: custom.sample.as_ref().map(ProfileSample::from_note),
    };
    let bass = Base::of(&custom.params).is_bass();
    if let Ok(mut eng) = app.audio_engine.lock() {
        eng.add_profile(profile);
    }
    Some(bass)
}

fn preview_current(app: &mut DAWApp) {
    if let Some(bass) = load_preview_profile(app) {
        preview_phrase(app, InstrumentId::Custom(PREVIEW_PROFILE.to_string()), bass, false);
    }
}

/// Play just the stored note at its own pitch: exactly the waveform on screen.
fn preview_note(app: &mut DAWApp) {
    let Some(sample) = app.designer.current.as_ref().and_then(|c| c.sample.as_ref()) else { return };
    let (pitch, len) = (sample.root_pitch, sample.duration_secs().min(MAX_NOTE_SECS));
    if load_preview_profile(app).is_none() {
        return;
    }
    release_pending_preview(app);
    let t0 = Instant::now();
    app.designer.preview_queue = vec![
        (t0, PreviewEv::On(pitch, 0.85)),
        (t0 + Duration::from_secs_f32(len), PreviewEv::Off(pitch)),
    ];
    app.designer.preview_instrument = Some(InstrumentId::Custom(PREVIEW_PROFILE.to_string()));
}

fn push_undo(app: &mut DAWApp) {
    if let Some(cur) = &app.designer.current {
        app.designer.undo.push((cur.params.clone(), cur.sample.clone()));
        if app.designer.undo.len() > MAX_UNDO {
            app.designer.undo.remove(0);
        }
    }
}

/// Render the current settings into the note the instrument plays (the waveform).
fn render_current(app: &mut DAWApp) {
    let sr = app.audio.sample_rate();
    let (root, hold) = (app.designer.render_root, app.designer.render_hold);
    if let Some(cur) = &mut app.designer.current {
        let hold = hold.unwrap_or_else(|| default_hold_secs(&cur.params));
        cur.sample = Some(render_note(&cur.params, root, hold, sr));
    }
    app.designer.wave.reset_view();
}

fn set_params(app: &mut DAWApp, params: SynthParams, said: String, did: String) {
    push_undo(app);
    if let Some(cur) = &mut app.designer.current {
        cur.params = params;
        cur.base_kind = cur.params.sample_kind;
    }
    render_current(app);
    app.designer.history.push((said, did));
    if app.designer.auto_preview {
        preview_current(app);
    }
}

fn create_from_description(app: &mut DAWApp) {
    let text = app.designer.description.trim().to_string();
    if text.is_empty() {
        return;
    }
    let it = interpret(&text, app.designer.base_override);
    let name = if app.designer.name.trim().is_empty() {
        unique_name(app, &it.suggested_name)
    } else {
        app.designer.name.trim().to_string()
    };
    app.designer.name = name.clone();
    let effects: Vec<String> = it.traits.iter().map(|t| t.effect.clone()).collect();
    app.designer.current = Some(CustomInstrument {
        name,
        description: text.clone(),
        base_kind: it.params.sample_kind,
        params: it.params,
        sample: None,
    });
    // Render bass sounds low so they're cleanest where they'll be played.
    app.designer.render_root = if it.base.is_bass() { 36 } else { 60 };
    app.designer.render_hold = None;
    render_current(app);
    app.designer.undo.clear();
    app.designer.history = vec![(
        text,
        format!(
            "Started from {} ({}){}",
            it.base.label(),
            it.base_reason,
            if effects.is_empty() {
                String::new()
            } else {
                format!(" → {}", effects.join(", "))
            }
        ),
    )];
    if app.designer.auto_preview {
        preview_current(app);
    }
}

fn unique_name(app: &DAWApp, wanted: &str) -> String {
    let taken = |n: &str| app.custom_instruments.iter().any(|c| c.name == n);
    if !taken(wanted) {
        return wanted.to_string();
    }
    (2..).map(|i| format!("{wanted} {i}")).find(|n| !taken(n)).unwrap()
}

fn base_picker(app: &mut DAWApp, ui: &mut Ui) {
    let label = match app.designer.base_override {
        None => "Auto (from your words)".to_string(),
        Some(b) => b.label().to_string(),
    };
    egui::ComboBox::from_id_salt("designer_base")
        .selected_text(label)
        .width(220.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut app.designer.base_override, None, "Auto (from your words)");
            for &genre in Genre::all() {
                ui.label(RichText::new(genre.label()).small().color(Color32::GRAY));
                for &p in GenrePreset::all().iter().filter(|p| p.genre() == genre) {
                    ui.selectable_value(
                        &mut app.designer.base_override,
                        Some(Base::Preset(p)),
                        format!("{} {}", p.icon(), p.label()),
                    );
                }
            }
            ui.label(RichText::new("Sampled").small().color(Color32::GRAY));
            for &k in SampleKind::all() {
                ui.selectable_value(
                    &mut app.designer.base_override,
                    Some(Base::Sample(k)),
                    format!("{} {}", k.icon(), k.label()),
                );
            }
        });
}

pub fn show_instrument_designer(app: &mut DAWApp, ui: &mut Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("✨ Sound Designer");
        ui.label(
            "Describe a sound the way you'd tell a producer friend — genre, instrument, feel. \
             Then keep talking to shape it.",
        );
        ui.add_space(6.0);

        let response = ui.add(
            egui::TextEdit::multiline(&mut app.designer.description)
                .hint_text("e.g. dark gritty 808 with a long tail")
                .desired_rows(2)
                .desired_width(f32::INFINITY),
        );
        let submit = response.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) && (i.modifiers.command || i.modifiers.ctrl));

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Try:").small().color(Color32::GRAY));
            for ex in EXAMPLES {
                if ui.small_button(*ex).clicked() {
                    app.designer.description = ex.to_string();
                    app.designer.name.clear();
                    create_from_description(app);
                }
            }
        });

        // Live read-back of what the words mean, before committing.
        if !app.designer.description.trim().is_empty() {
            let it = interpret(&app.designer.description, app.designer.base_override);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("I'm hearing:").color(Color32::GRAY));
                ui.label(
                    RichText::new(it.base.label())
                        .strong()
                        .color(Color32::from_rgb(140, 190, 255)),
                )
                .on_hover_text(it.base_reason.clone());
                for t in &it.traits {
                    let color = if t.strength > 0.0 {
                        Color32::from_rgb(150, 220, 160)
                    } else {
                        Color32::from_rgb(230, 170, 120)
                    };
                    ui.label(RichText::new(format!("· {}", t.effect)).color(color))
                        .on_hover_text(format!("from “{}”", t.word));
                }
            });
        }

        ui.horizontal(|ui| {
            ui.label("Built on:");
            base_picker(app, ui);
            ui.checkbox(&mut app.designer.auto_preview, "Auto-preview");
        });

        let create = ui
            .add(egui::Button::new(RichText::new("✨ Create sound").strong()).min_size(egui::vec2(160.0, 28.0)))
            .on_hover_text("Ctrl+Enter")
            .clicked();
        if create || submit {
            app.designer.name.clear();
            create_from_description(app);
        }

        if app.designer.current.is_some() {
            ui.add_space(8.0);
            show_current(app, ui);
        }

        ui.add_space(12.0);
        show_library(app, ui);
    });
}

fn show_current(app: &mut DAWApp, ui: &mut Ui) {
    ui.group(|ui| {
        let base_label = app
            .designer
            .current
            .as_ref()
            .map(|c| c.base_label())
            .unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.add(egui::TextEdit::singleline(&mut app.designer.name).desired_width(200.0));
            ui.label(RichText::new(format!("built on {base_label}")).small().color(Color32::GRAY));
        });
        if let Some(cur) = &mut app.designer.current {
            cur.name = app.designer.name.clone();
            ui.label(RichText::new(describe_params(&cur.params)).monospace().small());
        }

        ui.add_space(4.0);
        show_waveform(app, ui);
        if app.designer.current.as_ref().and_then(|c| c.sample.as_ref()).is_some_and(|s| s.edited) {
            ui.label(
                RichText::new(
                    "Shaping or fine-tuning re-renders the note and replaces your hand edits (Undo brings them back).",
                )
                .small()
                .color(Color32::from_rgb(230, 190, 120)),
            );
        }

        ui.add_space(4.0);
        ui.label("Shape it:");
        let mut quick = None;
        ui.horizontal_wrapped(|ui| {
            for (label, words) in QUICK {
                if ui.button(*label).clicked() {
                    quick = Some((label.to_string(), words.to_string()));
                }
            }
        });
        if let Some((label, words)) = quick {
            if let Some(cur) = &app.designer.current {
                let (p, notes) = refine(&cur.params, &words);
                set_params(app, p, label, notes.join(", "));
            }
        }

        ui.horizontal(|ui| {
            let r = ui.add(
                egui::TextEdit::singleline(&mut app.designer.refine_input)
                    .hint_text("…or say what to change: “a bit darker and less boomy”")
                    .desired_width(300.0),
            );
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.button("Apply").clicked() || enter) && !app.designer.refine_input.trim().is_empty() {
                let said = app.designer.refine_input.trim().to_string();
                if let Some(cur) = &app.designer.current {
                    let (p, notes) = refine(&cur.params, &said);
                    set_params(app, p, said, notes.join(", "));
                }
                app.designer.refine_input.clear();
            }
        });

        if !app.designer.history.is_empty() {
            ui.add_space(4.0);
            for (said, did) in app.designer.history.iter().rev().take(6).rev() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("you: {said}")).color(Color32::from_rgb(170, 170, 190)));
                    ui.label(RichText::new(format!("→ {did}")).color(Color32::from_rgb(150, 220, 160)));
                });
            }
        }

        ui.collapsing("Fine-tune by hand", |ui| {
            let mut commit = false;
            let mut started = false;
            let mut before = None;
            if let Some(cur) = &mut app.designer.current {
                before = Some(cur.params.clone());
                let p = &mut cur.params;
                // A drag commits when released; a click or keyboard nudge commits at once.
                let mut track = |r: egui::Response| {
                    started |= r.drag_started();
                    commit |= r.drag_stopped() || (r.changed() && !r.dragged());
                };
                track(ui.add(egui::Slider::new(&mut p.filter_cutoff, 0.05..=1.0).text("Tone (dark ↔ bright)")));
                track(ui.add(egui::Slider::new(&mut p.brightness, 0.0..=1.0).text("Edge / bite")));
                track(ui.add(egui::Slider::new(&mut p.attack, 0.0005..=2.0).logarithmic(true).text("Attack (s)")));
                track(ui.add(egui::Slider::new(&mut p.decay, 0.02..=6.0).logarithmic(true).text("Decay (s)")));
                track(ui.add(egui::Slider::new(&mut p.sustain, 0.0..=1.0).text("Sustain")));
                track(ui.add(egui::Slider::new(&mut p.release, 0.02..=6.0).logarithmic(true).text("Tail / release (s)")));
                track(ui.add(egui::Slider::new(&mut p.detune, 0.0..=0.04).text("Width / detune")));
                track(ui.add(egui::Slider::new(&mut p.drive, 0.0..=1.0).text("Grit / drive")));
                instrument_gen::clamp_params(p);
            }
            if started && app.designer.drag_params_start.is_none() {
                app.designer.drag_params_start = before.clone();
            }
            if commit {
                let start = app.designer.drag_params_start.take().or(before);
                if let (Some(start), Some(cur)) = (start, &app.designer.current) {
                    app.designer.undo.push((start, cur.sample.clone()));
                }
                render_current(app);
                if app.designer.auto_preview {
                    preview_current(app);
                }
            }
        });

        ui.horizontal(|ui| {
            if ui.button("▶ Preview").clicked() {
                preview_current(app);
            }
            if ui
                .add_enabled(!app.designer.undo.is_empty(), egui::Button::new("↶ Undo"))
                .clicked()
            {
                if let (Some((params, sample)), Some(cur)) = (app.designer.undo.pop(), app.designer.current.as_mut()) {
                    cur.params = params;
                    cur.sample = sample;
                    if let Some(s) = &cur.sample {
                        app.designer.render_root = s.root_pitch;
                    }
                    app.designer.wave.reset_view();
                    app.designer.history.push(("undo".into(), "back one step".into()));
                    if app.designer.auto_preview {
                        preview_current(app);
                    }
                }
            }
            if ui.button(RichText::new("💾 Save to library").strong()).clicked() {
                if let Some(mut cur) = app.designer.current.clone() {
                    let name = if cur.name.trim().is_empty() {
                        unique_name(app, "My Sound")
                    } else if app.custom_instruments.iter().any(|c| c.name == cur.name) {
                        cur.name.clone()
                    } else {
                        cur.name.clone()
                    };
                    cur.name = name;
                    // Saving with an existing name updates that instrument.
                    app.custom_instruments.retain(|c| c.name != cur.name);
                    app.save_custom_instrument(cur);
                    app.designer.current = None;
                    app.designer.history.clear();
                    app.designer.name.clear();
                }
            }
            if ui.button("Start over").clicked() {
                app.designer.current = None;
                app.designer.history.clear();
                app.designer.undo.clear();
                app.designer.name.clear();
            }
        });
    });
}

fn show_library(app: &mut DAWApp, ui: &mut Ui) {
    if !app.custom_instruments.is_empty() {
        ui.heading("My instruments");
        for inst in app.custom_instruments.clone() {
            ui.horizontal(|ui| {
                if let Some(s) = &inst.sample {
                    waveform_editor::thumbnail(ui, &s.data, egui::vec2(72.0, 18.0)).on_hover_text(format!(
                        "One {} note, {:.2} s{}",
                        pitch_to_name(s.root_pitch),
                        s.duration_secs(),
                        if s.edited { ", hand-edited" } else { "" }
                    ));
                }
                ui.label(RichText::new(&inst.name).strong());
                ui.label(
                    RichText::new(format!("on {}", inst.base_label()))
                        .small()
                        .color(Color32::GRAY),
                )
                .on_hover_text(&inst.description);
                if ui.small_button("Use").clicked() {
                    app.select_custom_instrument(&inst.name);
                }
                if ui.small_button("▶").clicked() {
                    let bass = Base::of(&inst.params).is_bass();
                    preview_phrase(app, InstrumentId::Custom(inst.name.clone()), bass, false);
                }
                if ui.small_button("Remix").on_hover_text("Load into the designer").clicked() {
                    app.designer.description = inst.description.clone();
                    app.designer.name = inst.name.clone();
                    app.designer.current = Some(inst.clone());
                    app.designer.history = vec![("remix".into(), format!("loaded {}", inst.name))];
                    app.designer.undo.clear();
                    app.designer.render_hold = None;
                    match &inst.sample {
                        Some(s) => app.designer.render_root = s.root_pitch,
                        // Made before notes were stored as audio: render one now.
                        None => render_current(app),
                    }
                    app.designer.wave.reset_view();
                }
            });
        }
        ui.add_space(8.0);
    }

    ui.collapsing("Built-in instruments", |ui| {
        for &genre in Genre::all() {
            ui.label(RichText::new(genre.label()).strong());
            ui.horizontal_wrapped(|ui| {
                for &p in GenrePreset::all().iter().filter(|p| p.genre() == genre) {
                    instrument_chip(app, ui, InstrumentId::Preset(p), p.is_bass(), false);
                }
                for &k in DrumKit::all().iter().filter(|k| k.genre() == genre) {
                    instrument_chip(app, ui, InstrumentId::Kit(k), false, true);
                }
                if genre == Genre::Classical {
                    instrument_chip(app, ui, InstrumentId::Piano, false, false);
                    instrument_chip(app, ui, InstrumentId::Harp, false, false);
                }
            });
        }
        ui.label(RichText::new("Sampled (FreePats / VCSL, CC0)").strong());
        ui.horizontal_wrapped(|ui| {
            for &k in SampleKind::all() {
                if !matches!(k, SampleKind::Piano | SampleKind::Harp) {
                    instrument_chip(app, ui, InstrumentId::from_sample_kind(k), k == SampleKind::Bass, false);
                }
            }
            instrument_chip(app, ui, InstrumentId::Drums, false, true);
        });
        let sr = app.audio.sample_rate();
        for kind in [SampleKind::Piano, SampleKind::Harp] {
            if let Some(status) = crate::audio::multisample::status(kind, sr) {
                ui.label(RichText::new(status).small().color(Color32::GRAY));
            }
        }
    });

    ui.collapsing("Sample credits (CC0)", |ui| {
        ui.label(
            "Sampled instruments use FreePats and the Versilian Community Sample Library \
             (both CC0 — fine for commercial use). Genre presets and kits are synthesized \
             by this app. See CREDITS.md for full attribution.",
        );
        ui.hyperlink_to("FreePats project", "https://freepats.zenvoid.org/");
        ui.hyperlink_to("VCSL", "https://github.com/sgossner/VCSL");
        for source in crate::audio::all_sources() {
            ui.group(|ui| {
                ui.label(RichText::new(source.kind.label()).strong());
                ui.label(format!("License: {}", source.license));
                ui.label(source.attribution);
                ui.hyperlink_to("Source", source.project_url);
            });
        }
    });
}

fn instrument_chip(app: &mut DAWApp, ui: &mut Ui, id: InstrumentId, bass: bool, drums: bool) {
    let label = format!("{} {}", id.icon(), id.label());
    let resp = ui
        .button(label)
        .on_hover_text("Click to preview · double-click to use on the selected track");
    if resp.double_clicked() {
        if app.set_track_instrument(id.clone()) {
            app.status_message = Some(format!("Track instrument: {}", id.label()));
        }
    } else if resp.clicked() {
        preview_phrase(app, id, bass, drums);
    }
}

/// The waveform panel: note pitch and length, the editor, and play/render/import/export.
fn show_waveform(app: &mut DAWApp, ui: &mut Ui) {
    let mut render = false;
    let mut play = false;
    let mut import = false;
    let mut export = false;
    let mut edited = None;
    {
        let d = &mut app.designer;
        let Some(cur) = d.current.as_mut() else { return };
        ui.label(RichText::new("🎵 Waveform: one note, played back on every key").strong());
        let Some(sample) = cur.sample.as_mut() else {
            if ui.button("Render as a note to see its waveform").clicked() {
                render = true;
            }
            if render {
                push_undo(app);
                render_current(app);
            }
            return;
        };

        ui.horizontal_wrapped(|ui| {
            ui.label("Note");
            let tip = if sample.edited {
                "The pitch this audio is. Set it to the note you actually hear so every key plays in tune."
            } else {
                "The pitch the note is rendered at. Render close to where you'll play it (e.g. C2 for a bass) for the cleanest sound."
            };
            let r = ui
                .add(
                    egui::DragValue::new(&mut sample.root_pitch)
                        .range(24..=96)
                        .custom_formatter(|v, _| pitch_to_name(v as u8))
                        .custom_parser(|t| name_to_pitch(t, 4).map(|p| p as f64)),
                )
                .on_hover_text(tip);
            if (r.drag_stopped() || (r.changed() && !r.dragged())) && !sample.edited {
                d.render_root = sample.root_pitch;
                render = true;
            } else if r.changed() {
                d.render_root = sample.root_pitch;
            }

            let mut hold = d.render_hold.unwrap_or_else(|| default_hold_secs(&cur.params));
            let r = ui
                .add(egui::Slider::new(&mut hold, 0.05..=6.0).logarithmic(true).suffix(" s").text("key held"))
                .on_hover_text("How long the key is held while rendering, before the release tail.");
            if r.changed() {
                d.render_hold = Some(hold);
            }
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                render = true;
            }
            ui.label(
                RichText::new(format!(
                    "{:.2} s · {} Hz{}",
                    sample.duration_secs(),
                    sample.sample_rate,
                    if sample.edited { " · hand-edited" } else { "" }
                ))
                .small()
                .color(Color32::GRAY),
            );
        });

        let resp = waveform_editor::show(ui, &mut d.wave, &mut sample.data, sample.sample_rate);
        if let Some(before) = resp.before {
            let old = NoteSample {
                root_pitch: sample.root_pitch,
                sample_rate: sample.sample_rate,
                data: before,
                edited: sample.edited,
            };
            d.undo.push((cur.params.clone(), Some(old)));
            if d.undo.len() > MAX_UNDO {
                d.undo.remove(0);
            }
            sample.edited = true;
        }
        if resp.committed {
            edited = Some(());
        }

        ui.horizontal_wrapped(|ui| {
            play = ui.button("▶ Play note").on_hover_text("Hear exactly this waveform at its own pitch").clicked();
            render = ui
                .button("⟳ Re-render from settings")
                .on_hover_text("Rebuild the note from the sound settings (replaces hand edits; Undo brings them back)")
                .clicked()
                || render;
            import = ui
                .button("📂 Import WAV…")
                .on_hover_text("Use your own one-note recording. Only import audio you're allowed to use in a commercial song.")
                .clicked();
            export = ui.button("💾 Export WAV…").clicked();
        });
    }

    if render {
        push_undo(app);
        render_current(app);
        if app.designer.auto_preview {
            preview_note(app);
        }
    }
    if (edited.is_some() && app.designer.auto_preview) || play {
        preview_note(app);
    }
    if import {
        import_note(app);
    }
    if export {
        export_note(app);
    }
}

fn import_note(app: &mut DAWApp) {
    use crate::audio::sample_sources::SampleFileFormat;
    let Some(path) = rfd::FileDialog::new().add_filter("Audio", &["wav", "flac"]).pick_file() else { return };
    let format = match path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()) {
        Some(e) if e == "flac" => SampleFileFormat::Flac,
        _ => SampleFileFormat::Wav,
    };
    let decoded = std::fs::read(&path).ok().and_then(|b| crate::audio::samples::decode_sample_raw(&b, format));
    let Some((mut data, rate)) = decoded.filter(|(d, _)| !d.is_empty()) else {
        app.status_error = Some(format!("Couldn't read {} as WAV/FLAC audio", path.display()));
        return;
    };
    data.truncate((MAX_NOTE_SECS * rate) as usize);
    normalize(&mut data, 0.95);
    push_undo(app);
    let root = app.designer.render_root;
    let file = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    if let Some(cur) = &mut app.designer.current {
        cur.sample = Some(NoteSample { root_pitch: root, sample_rate: rate as u32, data, edited: true });
    }
    app.designer.wave.reset_view();
    app.designer.history.push((
        format!("import {file}"),
        format!("using your recording; set Note to the pitch you hear (now {})", pitch_to_name(root)),
    ));
    if app.designer.auto_preview {
        preview_note(app);
    }
}

fn export_note(app: &mut DAWApp) {
    let Some(cur) = &app.designer.current else { return };
    let Some(sample) = &cur.sample else { return };
    let name = format!("{} {}.wav", cur.name.trim(), pitch_to_name(sample.root_pitch));
    let Some(path) = rfd::FileDialog::new().add_filter("WAV Audio", &["wav"]).set_file_name(name).save_file() else {
        return;
    };
    match crate::audio::samples::save_wav_f32(&path, &sample.data, sample.sample_rate) {
        Ok(()) => app.status_message = Some(format!("Exported note to {}", path.display())),
        Err(e) => app.status_error = Some(e),
    }
}
