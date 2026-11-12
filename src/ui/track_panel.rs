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

        ui.horizontal(|ui| {
            ui.checkbox(&mut app.project.tracks[i].muted, "Mute track");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let only_one = app.project.tracks.len() <= 1;
                let delete = ui
                    .add_enabled(
                        !only_one,
                        egui::Button::new(
                            egui::RichText::new("Delete track").color(egui::Color32::from_rgb(240, 110, 110)),
                        ),
                    )
                    .on_hover_text("Remove this track and its clips (also: right-click the track name)")
                    .on_disabled_hover_text("Can't delete the only track");
                if delete.clicked() {
                    app.request_delete_track(i);
                }
            });
        });

        ui.separator();
        let selected = app.selected_clip_indices();
        if selected.len() > 1 {
            // Several clips: the operations that make sense on a group.
            ui.label(format!("{} clips selected", selected.len()));
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button("Merge into one")
                    .on_hover_text("Combine them into a single clip you can loop as a whole")
                    .clicked()
                {
                    app.merge_selected_clips();
                }
                if ui.button("Copy").on_hover_text("Ctrl+C").clicked() {
                    if let Some(json) = app.copy_selected_clips() {
                        ui.ctx().copy_text(json);
                    }
                }
                if ui.button("Duplicate").on_hover_text("Ctrl+D: paste right after").clicked() {
                    app.duplicate_selected_clips();
                }
                if ui.button("Delete").clicked() {
                    app.delete_selected_clips();
                }
            });
            ui.label(
                egui::RichText::new("Ctrl+click or Shift+click bubbles to add or remove them.")
                    .small()
                    .color(egui::Color32::GRAY),
            );
        }
        ui.label("Selected clip");
        if selected.len() <= 1 {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(app.selected_clip.is_some(), egui::Button::new("Copy"))
                    .on_hover_text("Ctrl+C")
                    .clicked()
                {
                    if let Some(json) = app.copy_selected_clips() {
                        ui.ctx().copy_text(json);
                    }
                }
                if ui
                    .add_enabled(app.clip_clipboard.is_some(), egui::Button::new("Paste"))
                    .on_hover_text("Ctrl+V: paste at the playhead")
                    .clicked()
                {
                    app.paste_clips_at_playhead();
                }
                if ui
                    .add_enabled(app.selected_clip.is_some(), egui::Button::new("Duplicate"))
                    .on_hover_text("Ctrl+D: paste right after")
                    .clicked()
                {
                    app.duplicate_selected_clips();
                }
                if ui
                    .button("Select all")
                    .on_hover_text("Select every clip on this track (then Merge)")
                    .clicked()
                {
                    app.select_all_clips();
                }
            });
        }
        if let (Some(clip_idx), true) = (app.selected_clip, selected.len() <= 1) {
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
            if ui.button("▼ Octave").on_hover_text("Show lower notes").clicked() {
                app.adjust_piano_roll_pitch(-12);
            }
            if ui.button("▲ Octave").on_hover_text("Show higher notes").clicked() {
                app.adjust_piano_roll_pitch(12);
            }
            ui.label(format!(
                "{} – {}",
                crate::music::theory::pitch_to_name(app.piano_roll_low()),
                crate::music::theory::pitch_to_name(app.piano_roll_high()),
            ));
        });
        ui.label(
            egui::RichText::new("Scroll over the piano roll to move up and down by semitones.")
                .small()
                .color(egui::Color32::GRAY),
        );

        let current = app.project.tracks[i].instrument.clone();
        let drum_track = app.project.tracks[i].is_drum();
        ui.label(if drum_track { "Drum kit:" } else { "Instrument:" });
        let mut picked: Option<InstrumentId> = None;
        egui::ComboBox::from_id_salt("selected_track_instrument")
            .selected_text(format!("{} {}", current.icon(), current.label()))
            .height(420.0)
            .show_ui(ui, |ui| {
                let mut last_group = String::new();
                // Only kits on drum tracks, only melodic instruments on melodic tracks.
                for (group, id) in app
                    .instrument_choices()
                    .into_iter()
                    .filter(|(_, id)| id.is_drum_track() == drum_track)
                {
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
        if !drum_track
            && ui
                .small_button("Browse more instruments…")
                .on_hover_text("Build instruments from single-note samples on Freesound or in a folder")
                .clicked()
        {
            app.show_library = true;
        }
    }

    ui.add_space(12.0);
    ui.separator();
    ui.heading("Master");
    ui.add(egui::Slider::new(&mut app.audio.master_volume, 0.0..=1.0).text("Volume"));
    if ui.button("Open DJ Tools").clicked() {
        app.show_dj_panel = true;
    }
}

/// "Delete track?" confirmation, shown when the track still has notes (there's no undo).
pub fn show_delete_track_confirm(app: &mut DAWApp, ctx: &egui::Context) {
    let Some(index) = app.pending_track_delete else { return };
    let Some(track) = app.project.tracks.get(index) else {
        app.pending_track_delete = None;
        return;
    };
    let name = track.name.clone();
    let clips = track.clips.len();
    let notes: usize = track.clips.iter().map(|c| c.notes.len()).sum();

    let mut keep_open = true;
    egui::Window::new("Delete track?")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .open(&mut keep_open)
        .show(ctx, |ui| {
            ui.label(format!(
                "\"{name}\" has {clips} clip{} with {notes} note{}. You can undo this with Ctrl+Z.",
                if clips == 1 { "" } else { "s" },
                if notes == 1 { "" } else { "s" },
            ));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let delete = egui::Button::new(
                    egui::RichText::new("Delete track").color(egui::Color32::WHITE).strong(),
                )
                .fill(egui::Color32::from_rgb(190, 50, 50));
                if ui.add(delete).clicked() {
                    app.delete_track(index);
                }
                if ui.button("Cancel").clicked() {
                    app.pending_track_delete = None;
                }
            });
        });
    if !keep_open {
        app.pending_track_delete = None;
    }
}
