use crate::app::DAWApp;
use crate::model::instrument::InstrumentId;
use egui::Ui;

pub fn show_track_panel(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("Tracks");
    ui.separator();

    let preset_names = app.audio.preset_names();
    let track_count = app.project.tracks.len();

    for i in 0..track_count {
        let selected = app.current_track == i;
        let track_name = app.project.tracks[i].name.clone();
        let track_color = app.project.tracks[i].color;

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(track_color[0], track_color[1], track_color[2]),
                    "●",
                );
                if ui.selectable_label(selected, &track_name).clicked() {
                    app.current_track = i;
                    app.sync_instrument_to_track();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Vol");
                ui.add(egui::Slider::new(&mut app.project.tracks[i].volume, 0.0..=1.0));
            });

            ui.horizontal(|ui| {
                ui.checkbox(&mut app.project.tracks[i].muted, "Mute");
            });

            let current_label = app.project.tracks[i].instrument.label();
            ui.horizontal(|ui| {
                ui.label("Instrument:");
                egui::ComboBox::from_id_salt(format!("inst_{}", i))
                    .selected_text(&current_label)
                    .show_ui(ui, |ui| {
                        for preset in ["Piano", "Guitar", "Bass", "Strings"] {
                            if ui.selectable_label(current_label == preset, preset).clicked() {
                                app.project.tracks[i].instrument = match preset {
                                    "Guitar" => InstrumentId::Guitar,
                                    "Bass" => InstrumentId::Bass,
                                    "Strings" => InstrumentId::Strings,
                                    _ => InstrumentId::Piano,
                                };
                                if app.current_track == i {
                                    app.sync_instrument_to_track();
                                }
                            }
                        }
                        for name in &preset_names {
                            if ["Piano", "Guitar", "Bass", "Strings"].contains(&name.as_str()) {
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
            });
        });
        ui.add_space(4.0);
    }

    if ui.button("+ Add Track").clicked() {
        app.project.add_track();
    }

    ui.separator();
    ui.heading("Master");
    ui.add(egui::Slider::new(&mut app.audio.master_volume, 0.0..=1.0).text("Volume"));
}
