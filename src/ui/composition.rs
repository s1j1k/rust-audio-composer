use crate::app::DAWApp;
use crate::model::instrument::Genre;
use crate::music::genre_guide::{guide, GenreGuide};
use crate::music::theory::{circle_of_fifths, NOTE_NAMES};
use crate::music::{detect_key, resolve_chord, suggest_progressions, Key, KeyMode};
use crate::ui::theme;
use egui::{Color32, RichText, Ui, Vec2};

pub fn show_composition_panel(app: &mut DAWApp, ui: &mut Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| composition_contents(app, ui));
}

fn composition_contents(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("Composition Assistant");
    ui.separator();

    let pitches: Vec<u8> = app
        .project
        .tracks
        .iter()
        .filter(|t| !t.is_drum())
        .flat_map(|t| t.flat_notes())
        .map(|n| n.pitch)
        .collect();
    app.detected_key = detect_key(&pitches);

    show_key_picker(app, ui);
    let key = app.composition_key.clone().or_else(|| app.detected_key.clone());

    ui.add_space(8.0);
    show_genre_picker(app, ui);
    let genre_guide = app.composition_genre.map(guide);
    if let Some(gd) = &genre_guide {
        show_genre_tips(app, ui, gd);
    }

    ui.add_space(10.0);
    ui.strong("Chord progressions");
    match &key {
        Some(key) => {
            ui.label(
                RichText::new(
                    "Click chords one at a time to add them a bar each at the playhead, \
                     or \"Add all\" for one clip you can loop.",
                )
                .small()
                .color(Color32::GRAY),
            );
            if let Some(gd) = &genre_guide {
                for (name, chords, desc) in gd.progressions {
                    let chords: Vec<String> = chords.iter().map(|c| c.to_string()).collect();
                    progression_card(app, ui, &format!("{} · {name}", gd.genre.label()), desc, &chords, key);
                }
            }
            for prog in suggest_progressions(key) {
                progression_card(app, ui, &prog.name, &prog.description, &prog.chords, key);
            }
        }
        None => {
            ui.label("Choose a key above (or add some notes) to see progressions.");
        }
    }

    ui.add_space(12.0);
    show_circle_of_fifths(app, ui);

    ui.add_space(8.0);
    ui.collapsing("Learn: what is the circle of fifths?", |ui| {
        ui.label(
            "The circle of fifths arranges all 12 keys so each step clockwise adds one sharp, \
             and each step counter-clockwise adds one flat. Keys next to each other on the circle \
             sound harmonious together. Each major key shares its notes with a relative minor \
             (C and Am, G and Em), shown together in each box.",
        );
    });
}

fn show_key_picker(app: &mut DAWApp, ui: &mut Ui) {
    ui.strong("Key");
    let detected = app.detected_key.as_ref().map(|k| key_label(k));
    ui.horizontal(|ui| {
        let auto = app.composition_key.is_none();
        let auto_text = match &detected {
            Some(name) => format!("Detected: {name}"),
            None => "Detect from notes".to_string(),
        };
        if ui
            .selectable_label(auto, auto_text)
            .on_hover_text("Use the key that best fits the notes on your melodic tracks")
            .clicked()
        {
            app.composition_key = None;
        }
        if ui.selectable_label(!auto, "Choose").clicked() && auto {
            app.composition_key = Some(
                app.detected_key
                    .clone()
                    .unwrap_or(Key { root: 9, mode: KeyMode::Minor }),
            );
        }
    });

    if let Some(mut key) = app.composition_key.clone() {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("composition_key_root")
                .width(60.0)
                .selected_text(NOTE_NAMES[key.root as usize])
                .show_ui(ui, |ui| {
                    for (pc, name) in NOTE_NAMES.iter().enumerate() {
                        ui.selectable_value(&mut key.root, pc as u8, *name);
                    }
                });
            ui.selectable_value(&mut key.mode, KeyMode::Major, "Major");
            ui.selectable_value(&mut key.mode, KeyMode::Minor, "Minor");
        });
        ui.label(
            RichText::new(format!("Scale: {}", scale_names(&key)))
                .small()
                .color(Color32::GRAY),
        );
        app.composition_key = Some(key);
    } else if detected.is_none() {
        ui.label(
            RichText::new("No melodic notes yet: pick \"Choose\" to set a key yourself.")
                .small()
                .color(Color32::GRAY),
        );
    }
}

fn show_genre_picker(app: &mut DAWApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.strong("Genre tips");
        egui::ComboBox::from_id_salt("composition_genre")
            .selected_text(app.composition_genre.map(|g| g.label()).unwrap_or("None"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.composition_genre, None, "None");
                for &g in Genre::all() {
                    ui.selectable_value(&mut app.composition_genre, Some(g), g.label());
                }
            });
    });
}

fn show_genre_tips(app: &mut DAWApp, ui: &mut Ui, gd: &GenreGuide) {
    let (lo, typical, hi) = gd.bpm;
    ui.group(|ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Tempo {lo:.0}–{hi:.0} BPM, typically {typical:.0}"));
            let current = app.project.bpm;
            if (current - typical).abs() > 0.5 {
                if ui
                    .small_button(format!("Set {typical:.0} BPM"))
                    .on_hover_text(format!("Currently {current:.0} BPM"))
                    .clicked()
                {
                    app.project.bpm = typical;
                }
            } else {
                ui.label(RichText::new("✔ set").small().color(Color32::LIGHT_GREEN));
            }
        });
        ui.label(RichText::new(gd.feel).small());
        ui.label(RichText::new(gd.suggested_key).small().color(Color32::GRAY));
        bullet_section(ui, "Structure", gd.structure, true);
        bullet_section(ui, "Drums", gd.drums, false);
        bullet_section(ui, "Harmony & bass", gd.harmony, false);
        if let Some((title, url)) = gd.source {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Source:").small().color(Color32::GRAY));
                ui.hyperlink_to(RichText::new(title).small(), url);
            });
        }
    });
}

fn bullet_section(ui: &mut Ui, title: &str, lines: &[&str], open: bool) {
    egui::CollapsingHeader::new(title)
        .id_salt(("genre_tips", title))
        .default_open(open)
        .show(ui, |ui| {
            for line in lines {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("•").small());
                    ui.label(RichText::new(*line).small());
                });
            }
        });
}

fn progression_card(app: &mut DAWApp, ui: &mut Ui, name: &str, description: &str, chords: &[String], key: &Key) {
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(name).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button(format!("Add all ({} bars)", chords.len()))
                    .on_hover_text("One clip at the playhead, one bar per chord")
                    .clicked()
                {
                    app.insert_progression(name, chords, key);
                }
            });
        });
        ui.label(RichText::new(description).small().color(Color32::GRAY));
        ui.horizontal_wrapped(|ui| {
            for (i, symbol) in chords.iter().enumerate() {
                if i > 0 {
                    ui.label(RichText::new("→").color(Color32::GRAY));
                }
                let chord_name = resolve_chord(symbol, key).map(|c| c.name()).unwrap_or_default();
                let text = RichText::new(format!("{symbol}  {chord_name}"));
                if ui
                    .button(text)
                    .on_hover_text("Add this chord for one bar at the playhead")
                    .clicked()
                {
                    app.insert_chord_from_roman(symbol, key);
                }
            }
        });
    });
    ui.add_space(4.0);
}

fn show_circle_of_fifths(app: &mut DAWApp, ui: &mut Ui) {
    ui.strong("Circle of fifths");
    ui.label(
        RichText::new("Click a major (top) or minor (bottom) key to use it.")
            .small()
            .color(Color32::GRAY),
    );
    ui.add_space(4.0);

    // Allocate the whole graphic first; the labels are hit-tested with `interact` inside
    // it. (Placing widgets with `ui.put` here used to move the layout cursor back up into
    // the circle, so everything after it was drawn on top of the graphic.)
    let size = ui.available_width().clamp(200.0, 300.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let box_size = Vec2::new(44.0, 34.0);
    let radius = size * 0.5 - box_size.x * 0.5 - 4.0;
    painter.circle_stroke(center, radius, egui::Stroke::new(1.5_f32, Color32::from_gray(90)));

    let active = app.composition_key.clone().or_else(|| app.detected_key.clone());
    let active_name = active.as_ref().map(|k| k.name()).unwrap_or_default();

    for entry in circle_of_fifths() {
        let angle = (entry.angle_deg - 90.0).to_radians();
        let pos = center + Vec2::new(angle.cos(), angle.sin()) * radius;
        let box_rect = egui::Rect::from_center_size(pos, box_size);
        let (major_rect, minor_rect) = box_rect.split_top_bottom_at_fraction(0.5);

        let major_key = parse_key(entry.major, KeyMode::Major);
        let minor_key = parse_key(entry.minor.trim_end_matches('m'), KeyMode::Minor);
        let major_on = major_key.as_ref().map(|k| k.name() == active_name).unwrap_or(false);
        let minor_on = minor_key.as_ref().map(|k| k.name() == active_name).unwrap_or(false);

        let base = Color32::from_rgb(52, 52, 64);
        let hi = Color32::from_rgb(70, 140, 220);
        painter.rect_filled(box_rect, 4.0, if major_on || minor_on { blend(base, hi) } else { base });
        if major_on {
            painter.rect_filled(major_rect, 4.0, hi);
        }
        if minor_on {
            painter.rect_filled(minor_rect, 4.0, hi);
        }
        painter.text(
            major_rect.center(),
            egui::Align2::CENTER_CENTER,
            entry.major,
            theme::font(theme::FONT_LG),
            Color32::WHITE,
        );
        painter.text(
            minor_rect.center(),
            egui::Align2::CENTER_CENTER,
            entry.minor,
            theme::font(theme::FONT_SM),
            Color32::from_gray(210),
        );

        for (half, key, label) in [(major_rect, major_key, entry.major), (minor_rect, minor_key, entry.minor)] {
            let resp = ui
                .interact(half, ui.id().with(("cof", label)), egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                if let Some(key) = key {
                    app.composition_key = Some(key);
                }
                app.composition_hint = Some(format!(
                    "{} major and {} minor are relatives: they share the same notes and key \
                     signature. Neighbours on the circle ({}) differ by just one note, so they \
                     mix well in a progression.",
                    entry.major,
                    entry.minor.trim_end_matches('m'),
                    neighbours(entry.major)
                ));
            }
        }
    }

    if let Some(hint) = &app.composition_hint {
        ui.add_space(6.0);
        ui.group(|ui| {
            ui.label(hint);
        });
    }
}

/// "C" / "F#" / "Db" (flat names from the circle) → key.
fn parse_key(name: &str, mode: KeyMode) -> Option<Key> {
    let sharp = match name {
        "Db" => "C#",
        "Eb" => "D#",
        "Gb" => "F#",
        "Ab" => "G#",
        "Bb" => "A#",
        other => other,
    };
    NOTE_NAMES
        .iter()
        .position(|&n| n == sharp)
        .map(|pc| Key { root: pc as u8, mode })
}

fn neighbours(major: &str) -> String {
    let circle = circle_of_fifths();
    let i = circle.iter().position(|e| e.major == major).unwrap_or(0);
    let n = circle.len();
    format!("{} and {}", circle[(i + n - 1) % n].major, circle[(i + 1) % n].major)
}

fn key_label(key: &Key) -> String {
    let root = NOTE_NAMES[key.root as usize];
    match key.mode {
        KeyMode::Major => format!("{root} major"),
        KeyMode::Minor => format!("{root} minor"),
    }
}

fn scale_names(key: &Key) -> String {
    key.scale_pitches()
        .iter()
        .map(|&pc| NOTE_NAMES[pc as usize])
        .collect::<Vec<_>>()
        .join(" ")
}

fn blend(a: Color32, b: Color32) -> Color32 {
    let m = |x: u8, y: u8| ((x as u16 + y as u16) / 2) as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}
