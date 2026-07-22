use crate::app::DAWApp;
use crate::music::theory::{circle_of_fifths, pitch_to_name};
use crate::music::{detect_key, suggest_progressions, Key};
use egui::{Pos2, Ui, Vec2};

pub fn show_composition_panel(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("Composition Assistant");
    ui.separator();

    let pitches: Vec<u8> = app
        .project
        .tracks
        .iter()
        .flat_map(|t| t.notes.iter().map(|n| n.pitch))
        .collect();

    ui.label("Detected Key");
    if let Some(key) = detect_key(&pitches) {
        ui.heading(&key.name());
        app.detected_key = Some(key.clone());

        ui.add_space(8.0);
        ui.label("Suggested Chord Progressions");
        ui.separator();

        for prog in suggest_progressions(&key) {
            ui.group(|ui| {
                ui.label(egui::RichText::new(&prog.name).strong());
                ui.label(&prog.description);
                ui.horizontal_wrapped(|ui| {
                    for (i, chord) in prog.chords.iter().enumerate() {
                        if i > 0 {
                            ui.label("→");
                        }
                        if ui.button(chord).clicked() {
                            app.insert_chord_from_roman(chord, &key);
                        }
                    }
                });
            });
            ui.add_space(4.0);
        }
    } else {
        ui.label("Add notes to your tracks to detect a key.");
        app.detected_key = None;
    }

    ui.add_space(12.0);
    ui.heading("Circle of Fifths");
    ui.label("Click a key to learn its relative minor/major relationship.");
    ui.add_space(4.0);

    let (rect, _response) = ui.allocate_exact_size(Vec2::new(280.0, 280.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter_at(rect);
        let center = rect.center();
        let radius = rect.width().min(rect.height()) * 0.4;

        painter.circle_stroke(center, radius, egui::Stroke::new(1.5, egui::Color32::GRAY));

        let highlighted = app
            .detected_key
            .as_ref()
            .map(|k| k.name())
            .unwrap_or_default();

        for entry in circle_of_fifths() {
            let angle = (entry.angle_deg - 90.0).to_radians();
            let pos = center + Vec2::new(angle.cos(), angle.sin()) * radius;

            let is_major_match = highlighted.starts_with(entry.major);
            let is_minor_match = highlighted.starts_with(
                entry.minor.trim_end_matches('m'),
            ) && highlighted.contains('m');

            let color = if is_major_match || is_minor_match {
                egui::Color32::from_rgb(100, 200, 255)
            } else {
                egui::Color32::from_rgb(180, 180, 200)
            };

            let label = format!("{}\n{}", entry.major, entry.minor);
            let text_rect = egui::Rect::from_center_size(pos, Vec2::new(40.0, 30.0));
            painter.rect_filled(text_rect, 4.0, color);
            painter.text(
                pos,
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );

            if ui.put(text_rect, egui::Label::new("")).clicked() {
                app.composition_hint = Some(format!(
                    "{} major is the relative major of {} minor. \
                     They share the same key signature — a core idea of the circle of fifths.",
                    entry.major, entry.minor
                ));
            }
        }
    }

    if let Some(hint) = &app.composition_hint {
        ui.add_space(8.0);
        ui.group(|ui| {
            ui.label(hint);
        });
    }

    ui.add_space(8.0);
    ui.collapsing("Learn: What is the Circle of Fifths?", |ui| {
        ui.label(
            "The circle of fifths arranges all 12 keys so each step clockwise adds one sharp, \
             and each step counter-clockwise adds one flat. Keys next to each other on the circle \
             sound harmonious together. Relative major/minor pairs (like C and Am) share the same \
             notes and sit opposite each other in the circle.",
        );
    });
}
