use crate::app::DAWApp;
use crate::model::instrument::{InstrumentId, SampleKind};
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
        let profile_names = app.audio.profile_names();

        ui.horizontal(|ui| {
            ui.label("Volume");
            ui.add(egui::Slider::new(&mut app.project.tracks[i].volume, 0.0..=1.0));
        });

        ui.checkbox(&mut app.project.tracks[i].muted, "Mute track");

        let current_label = app.project.tracks[i].instrument.label();
        ui.label("Instrument:");
        egui::ComboBox::from_id_salt("selected_track_instrument")
            .selected_text(&current_label)
            .show_ui(ui, |ui| {
                for &kind in SampleKind::all() {
                    let label = kind.label();
                    if ui.selectable_label(current_label == label, label).clicked() {
                        app.project.tracks[i].instrument = InstrumentId::from_sample_kind(kind);
                        if app.current_track == i {
                            app.sync_instrument_to_track();
                        }
                    }
                }
                ui.separator();
                for name in &profile_names {
                    if SampleKind::all().iter().any(|k| k.label() == name) {
                        continue;
                    }
                    if ui.selectable_label(current_label == *name, name).clicked() {
                        app.project.tracks[i].instrument = InstrumentId::Custom(name.clone());
                        if app.current_track == i {
                            app.sync_instrument_to_track();
                        }
                    }
                }
            });
    }

    ui.add_space(12.0);
    if ui.button("+ Add Track").clicked() {
        app.project.add_track();
    }

    ui.separator();
    ui.heading("Master");
    ui.add(egui::Slider::new(&mut app.audio.master_volume, 0.0..=1.0).text("Volume"));
}
