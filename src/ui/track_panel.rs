use crate::app::DAWApp;
use crate::model::instrument::InstrumentId;
use egui::Ui;

pub fn show_track_panel(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("Track Controls");
    ui.separator();

    if let Some(track) = app.project.tracks.get(app.current_track) {
        let track_name = track.name.clone();
        let track_color = track.color;
        ui.horizontal(|ui| {
            ui.colored_label(
                egui::Color32::from_rgb(track_color[0], track_color[1], track_color[2]),
                "●",
            );
            ui.heading(&track_name);
        });
        ui.label(format!(
            "Track {} of {} — select rows in the timeline",
            app.current_track + 1,
            app.project.tracks.len()
        ));
    }

    ui.add_space(8.0);

    if app.current_track < app.project.tracks.len() {
        let i = app.current_track;

        ui.horizontal(|ui| {
            ui.label("Volume");
            ui.add(egui::Slider::new(&mut app.project.tracks[i].volume, 0.0..=1.0));
        });

        ui.checkbox(&mut app.project.tracks[i].muted, "Mute track");

        ui.separator();
        ui.label("Selected clip");
        if let Some(clip_idx) = app.selected_clip {
            if let Some(clip) = app.project.tracks[i].clips.get_mut(clip_idx) {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut clip.name);
                });
                let mut loop_on = clip.loop_enabled;
                if ui.checkbox(&mut loop_on, "↻ Loop clip").changed() {
                    clip.loop_enabled = loop_on;
                    if loop_on {
                        clip.loop_end_beat = clip.start_beat + clip.visible_length() * 2.0;
                    }
                }
                if clip.loop_enabled {
                    let min_loop = clip.start_beat + clip.visible_length();
                    let max_loop = app.project.total_beats;
                    let mut loop_end = clip.loop_end_beat;
                    ui.horizontal(|ui| {
                        ui.label("Loop until beat:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut loop_end)
                                    .speed(0.25)
                                    .range(min_loop..=max_loop),
                            )
                            .changed()
                        {
                            clip.loop_end_beat = loop_end;
                        }
                    });
                }
                ui.label(format!(
                    "Trim: {:.2} – {:.2} beats (source {:.2} — drag edges to recover hidden notes)",
                    clip.trim_start,
                    clip.visible_content_end(),
                    clip.source_length,
                ));
                if app.section_range.is_some() && ui.button("Use section as clip loop").clicked() {
                    app.set_clip_loop_from_section(i, clip_idx);
                }
                if ui.button("Delete clip").clicked() {
                    app.project.tracks[i].remove_clip(clip_idx);
                    app.selected_clip = None;
                    app.selected_note = None;
                }
            }
        } else {
            ui.label(
                egui::RichText::new("Click a clip bubble on the timeline to edit it.")
                    .small()
                    .color(egui::Color32::GRAY),
            );
        }

        ui.separator();
        ui.label("Piano roll view");
        ui.horizontal(|ui| {
            ui.label("Octaves:");
            ui.add(egui::Slider::new(&mut app.piano_roll_octave_count, 2..=5));
        });
        ui.horizontal(|ui| {
            ui.label("Base octave:");
            let mut oct = (app.piano_roll_octave_base / 12) as i32;
            if ui.add(egui::DragValue::new(&mut oct).range(0..=8)).changed() {
                app.piano_roll_octave_base = (oct * 12).clamp(0, 108) as u8;
            }
        });
        ui.label(
            egui::RichText::new("Scroll on piano keys to shift octave up/down.")
                .small()
                .color(egui::Color32::GRAY),
        );

        let current = app.project.tracks[i].instrument.clone();
        ui.label("Instrument:");
        let mut picked: Option<InstrumentId> = None;
        egui::ComboBox::from_id_salt("selected_track_instrument")
            .selected_text(format!("{} {}", current.icon(), current.label()))
            .height(420.0)
            .show_ui(ui, |ui| {
                let mut last_group = String::new();
                for (group, id) in app.instrument_choices() {
                    if group != last_group {
                        if !last_group.is_empty() {
                            ui.separator();
                        }
                        ui.label(egui::RichText::new(&group).small().color(egui::Color32::GRAY));
                        last_group = group;
                    }
                    if ui
                        .selectable_label(current == id, format!("{} {}", id.icon(), id.label()))
                        .clicked()
                    {
                        picked = Some(id);
                    }
                }
            });
        if let Some(id) = picked {
            app.set_track_instrument(id);
        }
    }

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui.button("+ Melodic Track").clicked() {
            app.project.add_track();
        }
        if ui.button("+ Drum Track").clicked() {
            app.project.add_drum_track();
            app.current_track = app.project.tracks.len() - 1;
            app.sync_instrument_to_track();
            app.show_drum_sequencer = true;
            app.open_drum_sequencer();
        }
    });

    ui.separator();
    ui.heading("Master");
    ui.add(egui::Slider::new(&mut app.audio.master_volume, 0.0..=1.0).text("Volume"));
    if ui.button("Open DJ Tools").clicked() {
        app.show_dj_panel = true;
    }
}
