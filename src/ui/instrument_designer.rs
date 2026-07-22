use crate::ai::{ai_summary, describe_to_instrument};
use crate::app::DAWApp;
use crate::model::instrument::SampleKind;
use egui::Ui;

pub fn show_instrument_designer(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("AI Instrument Designer");
    ui.label(
        "Pick a base instrument sample, describe how you want it to sound, \
         then generate and save to your library.",
    );
    ui.separator();

    ui.label("Base instrument (waveform sample):");
    egui::ComboBox::from_id_salt("ai_base_instrument")
        .selected_text(app.ai_base_kind.label())
        .show_ui(ui, |ui| {
            for &kind in SampleKind::all() {
                ui.selectable_value(&mut app.ai_base_kind, kind, kind.label());
            }
        });

    ui.add_space(6.0);
    ui.label("Instrument Name:");
    ui.text_edit_singleline(&mut app.instrument_name_input);

    ui.add_space(4.0);
    ui.label("Describe the sound:");
    ui.text_edit_multiline(&mut app.instrument_description_input);

    ui.horizontal(|ui| {
        if ui.button("🎹 Preview Base").clicked() {
            app.preview_base_instrument();
        }
        if ui.button("✨ Generate Instrument").clicked() {
            let name = if app.instrument_name_input.is_empty() {
                format!("Custom {}", app.custom_instruments.len() + 1)
            } else {
                app.instrument_name_input.clone()
            };
            let custom = describe_to_instrument(
                &app.instrument_description_input,
                &name,
                app.ai_base_kind,
            );
            app.instrument_gen_result =
                Some(ai_summary(&app.instrument_description_input, &custom));
            app.pending_custom_instrument = Some(custom);
        }
    });

    if let Some(result) = &app.instrument_gen_result {
        ui.add_space(8.0);
        ui.group(|ui| {
            ui.label(result);
        });
    }

    if let Some(custom) = app.pending_custom_instrument.clone() {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("💾 Save to Library").clicked() {
                app.save_custom_instrument(custom);
            }
            if ui.button("🎹 Preview Custom").clicked() {
                app.preview_custom_instrument();
            }
        });
    }

    ui.add_space(12.0);
    ui.heading("Instrument Library");
    ui.label("Built-in samples (cached as WAV in ~/.rust-audio-composer/samples/)");
    for &kind in SampleKind::all() {
        ui.horizontal(|ui| {
            ui.label(kind.label());
            if ui.button("Use").clicked() {
                app.select_builtin_instrument(kind);
            }
            if ui.button("Preview").clicked() {
                app.preview_builtin(kind);
            }
        });
    }

    if !app.custom_instruments.is_empty() {
        ui.separator();
        ui.label("Custom instruments");
        for inst in app.custom_instruments.clone() {
            ui.horizontal(|ui| {
                ui.label(&inst.name);
                ui.label(
                    egui::RichText::new(format!("({} base)", inst.base_kind.label()))
                        .small()
                        .color(egui::Color32::GRAY),
                );
                if ui.button("Use").clicked() {
                    app.select_custom_instrument(&inst.name);
                }
            });
        }
    }
}
