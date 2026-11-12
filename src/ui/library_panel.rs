//! Instrument Library window: your sample instruments, a Freesound browser for
//! single-note samples, and import from a folder of note samples.

use crate::app::DAWApp;
use crate::library::freesound::{self, FsSound, LicenseFilter, SearchOptions, SearchPage};
use crate::library::{self, notes, SampleCredit};
use crate::model::instrument::InstrumentId;
use egui::{Color32, RichText, Ui};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Mine,
    Freesound,
    Folder,
}

#[derive(Clone)]
pub struct FolderItem {
    path: PathBuf,
    name: String,
    midi: Option<u8>,
    /// "name" when read from the file name, "pitch" when measured, "" when unknown.
    found_by: &'static str,
    include: bool,
}

enum Job {
    Search(Result<SearchPage, String>),
    Pack(Result<Vec<FsSound>, String>),
    Preview(u64, Result<Vec<f32>, String>),
    Progress(String),
    Added(Result<String, String>),
    Scanned(Vec<FolderItem>),
}

pub struct LibraryUi {
    tab: Tab,
    api_key: String,
    key_loaded: bool,
    show_key: bool,
    query: String,
    license: LicenseFilter,
    detected_only: bool,
    page: u32,
    results: Vec<FsSound>,
    total: u64,
    showing_pack: Option<u64>,
    selected: HashSet<u64>,
    target: String,
    busy: Option<String>,
    message: Option<Result<String, String>>,
    previews: HashMap<u64, Arc<Vec<f32>>>,
    folder: Option<PathBuf>,
    folder_items: Vec<FolderItem>,
    octave_shift: i32,
    folder_author: String,
    folder_license: String,
    folder_url: String,
    confirm_delete: Option<String>,
    tx: Sender<Job>,
    rx: Receiver<Job>,
}

impl Default for LibraryUi {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            tab: Tab::Mine,
            api_key: String::new(),
            key_loaded: false,
            show_key: false,
            query: String::new(),
            // Default to sounds you may use in music you sell.
            license: LicenseFilter::CommercialOk,
            detected_only: true,
            page: 1,
            results: Vec::new(),
            total: 0,
            showing_pack: None,
            selected: HashSet::new(),
            target: String::new(),
            busy: None,
            message: None,
            previews: HashMap::new(),
            folder: None,
            folder_items: Vec::new(),
            octave_shift: 0,
            folder_author: String::new(),
            folder_license: String::new(),
            folder_url: String::new(),
            confirm_delete: None,
            tx,
            rx,
        }
    }
}

pub fn show_library_window(app: &mut DAWApp, ctx: &egui::Context) {
    if !app.show_library {
        return;
    }
    let mut open = true;
    egui::Window::new("Instrument Library")
        .open(&mut open)
        .default_size([620.0, 560.0])
        .show(ctx, |ui| show_library(app, ui));
    app.show_library = open;
}

fn show_library(app: &mut DAWApp, ui: &mut Ui) {
    poll_jobs(app);
    if !app.library_ui.key_loaded {
        app.library_ui.api_key = library::load_api_key();
        app.library_ui.key_loaded = true;
    }
    if app.library_ui.busy.is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }

    ui.label(
        RichText::new(
            "Build instruments from single recorded notes (no loops or beats). Each note you \
             add becomes a key; notes in between are filled by repitching the nearest sample. \
             Only sounds whose licence lets you use them in songs you release or sell can be \
             added.",
        )
        .small()
        .color(Color32::GRAY),
    );
    ui.horizontal(|ui| {
        let s = &mut app.library_ui.tab;
        ui.selectable_value(s, Tab::Mine, "My instruments");
        ui.selectable_value(s, Tab::Freesound, "Freesound");
        ui.selectable_value(s, Tab::Folder, "Import folder");
    });
    if let Some(msg) = &app.library_ui.busy {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(msg);
        });
    } else if let Some(m) = &app.library_ui.message {
        match m {
            Ok(t) => ui.label(RichText::new(t).color(Color32::LIGHT_GREEN)),
            Err(t) => ui.label(RichText::new(t).color(Color32::LIGHT_RED)),
        };
    }
    ui.separator();

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        match app.library_ui.tab {
            Tab::Mine => mine_tab(app, ui),
            Tab::Freesound => freesound_tab(app, ui),
            Tab::Folder => folder_tab(app, ui),
        }
    });
}

fn poll_jobs(app: &mut DAWApp) {
    while let Ok(job) = app.library_ui.rx.try_recv() {
        let lib = &mut app.library_ui;
        match job {
            Job::Search(Ok(page)) => {
                lib.busy = None;
                lib.total = page.total;
                lib.results = page.sounds;
                lib.message = Some(Ok(format!("{} sounds found", lib.total)));
            }
            Job::Pack(Ok(sounds)) => {
                lib.busy = None;
                lib.total = sounds.len() as u64;
                lib.message = Some(Ok(format!(
                    "{} sounds in this pack that you may use in songs",
                    sounds.len()
                )));
                lib.results = sounds;
            }
            Job::Search(Err(e)) | Job::Pack(Err(e)) => {
                lib.busy = None;
                lib.message = Some(Err(e));
            }
            Job::Preview(id, Ok(data)) => {
                lib.busy = None;
                let data = Arc::new(data);
                lib.previews.insert(id, data.clone());
                if let Ok(mut eng) = app.audio_engine.lock() {
                    eng.audition(data);
                }
            }
            Job::Preview(_, Err(e)) => {
                lib.busy = None;
                lib.message = Some(Err(e));
            }
            Job::Progress(p) => lib.busy = Some(p),
            Job::Added(r) => {
                lib.busy = None;
                lib.selected.clear();
                if r.is_ok() {
                    lib.tab = Tab::Mine;
                }
                lib.message = Some(r);
            }
            Job::Scanned(items) => {
                lib.busy = None;
                let named = items.iter().filter(|i| i.midi.is_some()).count();
                lib.message = Some(Ok(format!(
                    "{} audio files, {} with a note",
                    items.len(),
                    named
                )));
                lib.folder_items = items;
            }
        }
    }
}

// ---- My instruments ----------------------------------------------------------------

fn mine_tab(app: &mut DAWApp, ui: &mut Ui) {
    let instruments = library::list();
    if instruments.is_empty() {
        ui.label("No sample instruments yet. Find notes on Freesound, or import a folder of note samples (for example a pack downloaded from soundpacks.com).");
        return;
    }
    let melodic_track = app
        .project
        .tracks
        .get(app.current_track)
        .map(|t| !t.is_drum())
        .unwrap_or(false);
    for inst in instruments {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("♫ {}", inst.name)).strong());
                ui.label(
                    RichText::new(format!("{} notes · {}", inst.zones.len(), inst.range_label()))
                        .small()
                        .color(Color32::GRAY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.library_ui.confirm_delete.as_deref() == Some(inst.name.as_str()) {
                        if ui.button(RichText::new("Really delete").color(Color32::LIGHT_RED)).clicked() {
                            app.library_ui.message = Some(
                                library::delete(&inst.name).map(|_| format!("Deleted {}", inst.name)),
                            );
                            app.library_ui.confirm_delete = None;
                        }
                        if ui.small_button("Cancel").clicked() {
                            app.library_ui.confirm_delete = None;
                        }
                    } else if ui
                        .small_button("Delete")
                        .on_hover_text("Removes it from disk. Tracks using it fall back to piano.")
                        .clicked()
                    {
                        app.library_ui.confirm_delete = Some(inst.name.clone());
                    }
                    if ui
                        .add_enabled(melodic_track, egui::Button::new("Use on track"))
                        .on_disabled_hover_text("Select a melodic track first")
                        .clicked()
                        && app.set_track_instrument(InstrumentId::Sampled(inst.name.clone()))
                    {
                        app.status_message = Some(format!("Track instrument: {}", inst.name));
                    }
                    if ui.small_button("Add notes").on_hover_text("Search Freesound to add more notes to this instrument").clicked() {
                        app.library_ui.target = inst.name.clone();
                        app.library_ui.tab = Tab::Freesound;
                    }
                });
            });

            // Which keys have a recording.
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for z in &inst.zones {
                    let tag = RichText::new(crate::music::pitch_to_name(z.midi)).small().monospace();
                    let resp = ui.small_button(tag).on_hover_text(format!(
                        "{}\nby {} · {}\nClick to hear · right-click to remove",
                        z.credit.title, z.credit.author, z.credit.license
                    ));
                    if resp.clicked() {
                        if let Ok(mut eng) = app.audio_engine.lock() {
                            eng.play_note_for_instrument(z.midi, 0.8, &InstrumentId::Sampled(inst.name.clone()), 1.0);
                        }
                        app.preview_releases.push((z.midi, std::time::Instant::now() + std::time::Duration::from_millis(900)));
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Remove this note").clicked() {
                            app.library_ui.message = Some(library::remove_zone(&inst.name, z.midi).map(|_| "Removed".to_string()));
                            ui.close_menu();
                        }
                    });
                }
            });

            let mut release = inst.release;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Release").small());
                if ui
                    .add(egui::Slider::new(&mut release, 0.02..=3.0).suffix(" s").logarithmic(true))
                    .on_hover_text("How long notes ring after you let go of the key")
                    .drag_stopped()
                {
                    let _ = library::set_release(&inst.name, release);
                }
            });

            let credits = inst.credits();
            let nc = credits.iter().any(|c| c.non_commercial());
            if nc {
                ui.label(
                    RichText::new("Contains NonCommercial samples: fine for personal music, not for songs you sell.")
                        .small()
                        .color(Color32::from_rgb(240, 190, 90)),
                );
            }
            egui::CollapsingHeader::new(RichText::new(format!("Credits ({})", credits.len())).small())
                .id_salt(("credits", &inst.name))
                .show(ui, |ui| {
                    for c in &credits {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(format!("{} by {}", c.title, c.author)).small());
                            ui.label(RichText::new(format!("({}, {})", c.license, c.source)).small().color(Color32::GRAY));
                            if !c.url.is_empty() {
                                ui.hyperlink_to(RichText::new("source").small(), &c.url);
                            }
                        });
                    }
                    if credits.iter().any(|c| c.needs_attribution()) {
                        ui.label(
                            RichText::new("Attribution licences: credit these authors when you publish music that uses them.")
                                .small()
                                .color(Color32::GRAY),
                        );
                    }
                });
        });
        ui.add_space(4.0);
    }
}

// ---- Freesound -------------------------------------------------------------------------

fn freesound_tab(app: &mut DAWApp, ui: &mut Ui) {
    let lib = &mut app.library_ui;
    ui.collapsing("Freesound API key", |ui| {
        ui.label(
            RichText::new(
                "Searching Freesound needs your own free API key (Freesound doesn't allow \
                 apps to share one). Create one at freesound.org/apiv2/apply, then paste the \
                 key here. It's saved only on this computer.",
            )
            .small(),
        );
        ui.hyperlink_to("Get an API key", "https://freesound.org/apiv2/apply");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut lib.api_key).password(!lib.show_key).desired_width(280.0));
            ui.checkbox(&mut lib.show_key, "Show");
            if ui.button("Save key").clicked() {
                lib.message = Some(library::save_api_key(&lib.api_key).map(|_| "API key saved".to_string()));
            }
        });
    });
    if lib.api_key.trim().is_empty() {
        ui.label(RichText::new("Add your API key above to search.").color(Color32::from_rgb(240, 190, 90)));
    }

    let mut do_search = false;
    ui.horizontal(|ui| {
        let resp = ui.add(
            egui::TextEdit::singleline(&mut lib.query)
                .hint_text("e.g. cello, felt piano, kalimba, analog synth bass")
                .desired_width(300.0),
        );
        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            do_search = true;
        }
        if ui.button("Search").clicked() {
            do_search = true;
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Licence:").small())
            .on_hover_text("Only sounds you may use in songs you release or sell are shown");
        ui.selectable_value(&mut lib.license, LicenseFilter::CommercialOk, "OK in songs you sell")
            .on_hover_text("Creative Commons 0 or Attribution (credit the author)");
        ui.selectable_value(&mut lib.license, LicenseFilter::Cc0, "CC0 only")
            .on_hover_text("Public domain: no credit needed");
        ui.checkbox(&mut lib.detected_only, "Only sounds with a detected note");
    });
    if do_search && !lib.query.trim().is_empty() {
        lib.page = 1;
        start_search(lib);
    }

    if lib.results.is_empty() {
        return;
    }
    ui.separator();

    // Target instrument for "Add selected".
    let others: Vec<String> = app.custom_instruments.iter().map(|c| c.name.clone()).collect();
    let lib = &mut app.library_ui;
    ui.horizontal_wrapped(|ui| {
        ui.label("Add to instrument:");
        let names = library::names();
        egui::ComboBox::from_id_salt("library_target")
            .selected_text(if lib.target.is_empty() { "(new)".to_string() } else { lib.target.clone() })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut lib.target, String::new(), "(new)");
                for n in names {
                    ui.selectable_value(&mut lib.target, n.clone(), n);
                }
            });
        if lib.target.is_empty() || !library::names().contains(&lib.target) {
            ui.label(RichText::new("name:").small());
            let mut name = lib.target.clone();
            if ui.add(egui::TextEdit::singleline(&mut name).hint_text(lib.query.trim()).desired_width(140.0)).changed() {
                lib.target = name;
            }
        }
        let n = lib.selected.len();
        if ui
            .add_enabled(n > 0 && lib.busy.is_none(), egui::Button::new(format!("Add {n} selected")))
            .clicked()
        {
            let wanted = if lib.target.trim().is_empty() { lib.query.trim().to_string() } else { lib.target.clone() };
            let name = if library::names().contains(&wanted) { wanted } else { library::unique_name(&wanted, &others) };
            lib.target = name.clone();
            let sounds: Vec<FsSound> = lib.results.iter().filter(|s| lib.selected.contains(&s.id)).cloned().collect();
            start_add(lib, name, sounds);
        }
    });

    ui.horizontal(|ui| {
        if let Some(pack) = lib.showing_pack {
            ui.label(RichText::new(format!("Pack {pack}")).small());
            if ui.small_button("Back to search").clicked() {
                lib.showing_pack = None;
                start_search(lib);
            }
        } else {
            ui.label(RichText::new(format!("{} results · page {}", lib.total, lib.page)).small());
            if ui.add_enabled(lib.page > 1, egui::Button::new("◀").small()).clicked() {
                lib.page -= 1;
                start_search(lib);
            }
            if ui.add_enabled((lib.page as u64) * 40 < lib.total, egui::Button::new("▶").small()).clicked() {
                lib.page += 1;
                start_search(lib);
            }
        }
        if ui.small_button("Select all").clicked() {
            lib.selected = lib.results.iter().map(|s| s.id).collect();
        }
        if ui.small_button("Select none").clicked() {
            lib.selected.clear();
        }
    });

    let results = lib.results.clone();
    egui::Grid::new("fs_results").striped(true).num_columns(6).show(ui, |ui| {
        for s in &results {
            let mut on = app.library_ui.selected.contains(&s.id);
            if ui.checkbox(&mut on, "").changed() {
                if on {
                    app.library_ui.selected.insert(s.id);
                } else {
                    app.library_ui.selected.remove(&s.id);
                }
            }
            if ui.small_button("▶").on_hover_text("Preview").clicked() {
                preview(app, s);
            }
            let note = match s.note_midi {
                Some(m) if s.note_confidence >= 0.5 => crate::music::pitch_to_name(m),
                _ => notes::note_from_name(&s.name).map(crate::music::pitch_to_name).unwrap_or_else(|| "?".into()),
            };
            ui.label(RichText::new(note).monospace());
            ui.label(&s.name).on_hover_text(format!("by {} · {:.1} s", s.username, s.duration));
            license_badge(ui, &s.license);
            ui.horizontal(|ui| {
                if let Some(pack) = s.pack_id {
                    if ui.small_button("Pack").on_hover_text("Show every sound in this pack (often all the notes of one instrument)").clicked() {
                        let lib = &mut app.library_ui;
                        lib.showing_pack = Some(pack);
                        start_pack(lib, pack);
                    }
                }
                if !s.url.is_empty() {
                    ui.hyperlink_to(RichText::new("page").small(), &s.url);
                }
            });
            ui.end_row();
        }
    });
}

fn license_badge(ui: &mut Ui, license: &str) {
    let (text, color) = match license {
        "Creative Commons 0" => ("CC0", Color32::from_rgb(110, 200, 130)),
        "Attribution" => ("CC BY", Color32::from_rgb(120, 170, 240)),
        other => (other, Color32::GRAY),
    };
    ui.label(RichText::new(text).small().strong().color(color)).on_hover_text(match text {
        "CC0" => "Public domain: use anywhere, no credit needed",
        "CC BY" => "Use anywhere, including songs you sell; credit the author",
        _ => "Check this licence before using the sound",
    });
}

fn start_search(lib: &mut LibraryUi) {
    let key = lib.api_key.clone();
    let opts = SearchOptions {
        query: lib.query.clone(),
        license: lib.license,
        detected_note_only: lib.detected_only,
        page: lib.page,
    };
    lib.busy = Some("Searching Freesound…".to_string());
    lib.selected.clear();
    let tx = lib.tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(Job::Search(freesound::search(&key, &opts)));
    });
}

fn start_pack(lib: &mut LibraryUi, pack: u64) {
    let key = lib.api_key.clone();
    lib.busy = Some("Loading pack…".to_string());
    lib.selected.clear();
    let tx = lib.tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(Job::Pack(freesound::pack_sounds(&key, pack)));
    });
}

fn preview(app: &mut DAWApp, s: &FsSound) {
    if let Some(data) = app.library_ui.previews.get(&s.id).cloned() {
        if let Ok(mut eng) = app.audio_engine.lock() {
            eng.audition(data);
        }
        return;
    }
    let key = app.library_ui.api_key.clone();
    let url = s.preview_url.clone();
    let id = s.id;
    let rate = app.audio.sample_rate();
    app.library_ui.busy = Some(format!("Loading preview of {}…", s.name));
    let tx = app.library_ui.tx.clone();
    std::thread::spawn(move || {
        let r = freesound::download_preview(&key, &url)
            .and_then(|b| library::decode_to_mono(b, ext_of(&url)))
            .map(|(d, r)| crate::audio::samples::resample(&d, r, rate));
        let _ = tx.send(Job::Preview(id, r));
    });
}

fn ext_of(url: &str) -> Option<&'static str> {
    if url.ends_with(".mp3") {
        Some("mp3")
    } else if url.ends_with(".ogg") {
        Some("ogg")
    } else {
        None
    }
}

/// Download each sound's preview, work out its note, and add it to `name`.
fn start_add(lib: &mut LibraryUi, name: String, sounds: Vec<FsSound>) {
    let key = lib.api_key.clone();
    lib.busy = Some(format!("Adding {} sounds…", sounds.len()));
    let tx = lib.tx.clone();
    std::thread::spawn(move || {
        let total = sounds.len();
        let (mut added, mut skipped) = (Vec::new(), Vec::new());
        for (i, s) in sounds.iter().enumerate() {
            let _ = tx.send(Job::Progress(format!("Adding {} ({}/{total})…", s.name, i + 1)));
            let result = freesound::download_preview(&key, &s.preview_url)
                .and_then(|b| library::decode_to_mono(b, ext_of(&s.preview_url)))
                .and_then(|(data, rate)| {
                    let midi = s
                        .note_midi
                        .filter(|_| s.note_confidence >= 0.5)
                        .or_else(|| notes::note_from_name(&s.name))
                        .or_else(|| notes::detect_midi(&data, rate))
                        .ok_or("no clear note")?;
                    let credit = SampleCredit {
                        title: s.name.clone(),
                        author: s.username.clone(),
                        license: s.license.clone(),
                        url: s.url.clone(),
                        source: "Freesound".to_string(),
                    };
                    library::add_sample(&name, midi, data, rate, credit).map(|_| midi)
                });
            match result {
                Ok(m) => added.push(crate::music::pitch_to_name(m)),
                Err(e) => skipped.push(format!("{} ({e})", s.name)),
            }
            // Be gentle with Freesound.
            std::thread::sleep(std::time::Duration::from_millis(150));
        }
        let msg = if added.is_empty() {
            Err(format!("Nothing added: {}", skipped.join(", ")))
        } else if skipped.is_empty() {
            Ok(format!("Added {} to {name}", added.join(" ")))
        } else {
            Ok(format!("Added {} to {name}; skipped {}", added.join(" "), skipped.join(", ")))
        };
        let _ = tx.send(Job::Added(msg));
    });
}

// ---- Folder import -----------------------------------------------------------------------

const AUDIO_EXTS: &[&str] = &["wav", "flac", "ogg", "mp3", "aif", "aiff"];

fn folder_tab(app: &mut DAWApp, ui: &mut Ui) {
    let others: Vec<String> = app.custom_instruments.iter().map(|c| c.name.clone()).collect();
    let lib = &mut app.library_ui;
    ui.label(
        RichText::new(
            "For packs you've downloaded (soundpacks.com, VCSL, Philharmonia, …): choose the \
             folder of single-note samples. Notes are read from file names like \"Cello_A3.wav\" \
             or measured from the sound. Check each file's note before importing.",
        )
        .small(),
    );
    ui.horizontal(|ui| {
        if ui.button("Choose folder…").clicked() {
            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                if lib.target.is_empty() {
                    lib.target = dir.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                }
                lib.folder = Some(dir.clone());
                lib.folder_items.clear();
                lib.busy = Some("Reading folder…".to_string());
                let tx = lib.tx.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(Job::Scanned(scan_folder(&dir, &tx)));
                });
            }
        }
        if let Some(dir) = &lib.folder {
            ui.label(RichText::new(dir.display().to_string()).small().color(Color32::GRAY));
        }
    });
    if lib.folder_items.is_empty() {
        return;
    }

    ui.horizontal_wrapped(|ui| {
        ui.label("Instrument name:");
        ui.add(egui::TextEdit::singleline(&mut lib.target).desired_width(160.0));
        ui.label("Octave fix:");
        if ui
            .add(egui::DragValue::new(&mut lib.octave_shift).range(-2..=2))
            .on_hover_text("Some packs call middle C \"C3\" instead of \"C4\". If notes sound an octave off, set +1 or −1.")
            .changed()
        {}
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Credit:").small());
        ui.add(egui::TextEdit::singleline(&mut lib.folder_author).hint_text("author").desired_width(110.0));
        egui::ComboBox::from_id_salt("folder_licence")
            .selected_text(if lib.folder_license.is_empty() { "choose licence…" } else { lib.folder_license.as_str() })
            .show_ui(ui, |ui| {
                for l in library::LICENCES_FOR_SONGS {
                    ui.selectable_value(&mut lib.folder_license, l.to_string(), *l);
                }
            })
            .response
            .on_hover_text(
                "Check the pack's licence or readme. Only licences that let you use the sounds \
                 in songs you release or sell can be imported; packs marked NonCommercial or \
                 \"personal use only\" can't.",
            );
        ui.add(egui::TextEdit::singleline(&mut lib.folder_url).hint_text("source URL").desired_width(150.0));
    });

    let shift = lib.octave_shift * 12;
    let shifted = |item: &FolderItem| -> Option<u8> {
        let m = item.midi? as i32 + if item.found_by == "name" { shift } else { 0 };
        (0..=127).contains(&m).then_some(m as u8)
    };
    let chosen = lib.folder_items.iter().filter(|i| i.include && shifted(i).is_some()).count();
    let mut dupes = HashSet::new();
    let mut seen = HashSet::new();
    for i in lib.folder_items.iter().filter(|i| i.include) {
        if let Some(m) = shifted(i) {
            if !seen.insert(m) {
                dupes.insert(m);
            }
        }
    }
    if !dupes.is_empty() {
        ui.label(
            RichText::new("Some notes appear more than once (velocity layers or alternate takes?). Untick the extras; the last one wins.")
                .small()
                .color(Color32::from_rgb(240, 190, 90)),
        );
    }
    if lib.folder_license.is_empty() {
        ui.label(
            RichText::new("Choose the pack's licence to import (only song-safe licences are allowed).")
                .small()
                .color(Color32::from_rgb(240, 190, 90)),
        );
    }
    if ui
        .add_enabled(
            chosen > 0 && lib.busy.is_none() && !lib.folder_license.is_empty(),
            egui::Button::new(format!("Import {chosen} notes")),
        )
        .clicked()
    {
        let name = if library::names().contains(&lib.target) {
            lib.target.clone()
        } else {
            library::unique_name(&lib.target, &others)
        };
        lib.target = name.clone();
        let items: Vec<(PathBuf, String, u8)> = lib
            .folder_items
            .iter()
            .filter(|i| i.include)
            .filter_map(|i| Some((i.path.clone(), i.name.clone(), shifted(i)?)))
            .collect();
        let credit = SampleCredit {
            title: String::new(),
            author: lib.folder_author.trim().to_string(),
            license: lib.folder_license.clone(),
            url: lib.folder_url.trim().to_string(),
            source: "Folder import".to_string(),
        };
        lib.busy = Some(format!("Importing {} notes…", items.len()));
        let tx = lib.tx.clone();
        std::thread::spawn(move || {
            let total = items.len();
            let (mut ok, mut bad) = (0, Vec::new());
            for (i, (path, title, midi)) in items.into_iter().enumerate() {
                let _ = tx.send(Job::Progress(format!("Importing {title} ({}/{total})…", i + 1)));
                let c = SampleCredit { title: title.clone(), ..credit.clone() };
                match library::decode_file(&path).and_then(|(d, r)| library::add_sample(&name, midi, d, r, c)) {
                    Ok(()) => ok += 1,
                    Err(e) => bad.push(format!("{title} ({e})")),
                }
            }
            let msg = if ok == 0 {
                Err(format!("Nothing imported: {}", bad.join(", ")))
            } else if bad.is_empty() {
                Ok(format!("Imported {ok} notes into {name}"))
            } else {
                Ok(format!("Imported {ok} notes into {name}; skipped {}", bad.join(", ")))
            };
            let _ = tx.send(Job::Added(msg));
        });
    }

    egui::Grid::new("folder_items").striped(true).num_columns(4).show(ui, |ui| {
        for item in lib.folder_items.iter_mut() {
            ui.checkbox(&mut item.include, "");
            let mut midi = item.midi.unwrap_or(60) as i32;
            let label = match item.midi {
                Some(_) => {
                    let m = (midi + if item.found_by == "name" { shift } else { 0 }).clamp(0, 127) as u8;
                    crate::music::pitch_to_name(m)
                }
                None => "?".to_string(),
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{label:>4}")).monospace());
                if ui
                    .add(egui::DragValue::new(&mut midi).range(0..=127).prefix("MIDI "))
                    .on_hover_text("Drag to correct the note")
                    .changed()
                {
                    item.midi = Some(midi as u8);
                    item.found_by = "manual";
                    item.include = true;
                }
            });
            ui.label(&item.name);
            ui.label(
                RichText::new(match item.found_by {
                    "name" => "from name",
                    "pitch" => "measured",
                    "manual" => "set by you",
                    _ => "unknown",
                })
                .small()
                .color(Color32::GRAY),
            );
            ui.end_row();
        }
    });
}

/// List audio files (this folder and one level down) and work out each file's note.
fn scan_folder(dir: &Path, tx: &Sender<Job>) -> Vec<FolderItem> {
    let mut files = Vec::new();
    let mut push_dir = |d: &Path, files: &mut Vec<PathBuf>| {
        if let Ok(rd) = std::fs::read_dir(d) {
            for e in rd.flatten() {
                files.push(e.path());
            }
        }
    };
    push_dir(dir, &mut files);
    let subdirs: Vec<PathBuf> = files.iter().filter(|p| p.is_dir()).cloned().collect();
    for d in subdirs {
        push_dir(&d, &mut files);
    }
    let mut audio: Vec<PathBuf> = files
        .into_iter()
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .map(|e| AUDIO_EXTS.contains(&e.to_ascii_lowercase().as_str()))
                .unwrap_or(false)
        })
        .collect();
    audio.sort();
    audio.truncate(400);
    let total = audio.len();
    audio
        .into_iter()
        .enumerate()
        .map(|(i, path)| {
            let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let (midi, found_by) = match notes::note_from_name(&name) {
                Some(m) => (Some(m), "name"),
                None => {
                    let _ = tx.send(Job::Progress(format!("Measuring pitch of {name} ({}/{total})…", i + 1)));
                    match library::decode_file(&path).ok().and_then(|(d, r)| notes::detect_midi(&d, r)) {
                        Some(m) => (Some(m), "pitch"),
                        None => (None, ""),
                    }
                }
            };
            FolderItem { path, name, midi, found_by, include: midi.is_some() }
        })
        .collect()
}
