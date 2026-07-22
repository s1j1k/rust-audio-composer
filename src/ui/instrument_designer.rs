use crate::ai::{ai_summary, describe_to_instrument};
use crate::app::DAWApp;
use crate::model::instrument::{CustomInstrument, InstrumentPreset};
use egui::Ui;

pub fn show_instrument_designer(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("AI Instrument Designer");
    ui.label(
        "Describe the sound you want in plain language. The AI interprets your description \
         and creates a playable instrument you can save and use on any track.",
    );
    ui.separator();

    ui.label("Instrument Name:");
    ui.text_edit_singleline(&mut app.instrument_name_input);

    ui.add_space(4.0);
    ui.label("Describe the sound:");
    ui.text_edit_multiline(&mut app.instrument_description_input);

    ui.add_space(8.0);
    if ui.button("✨ Generate Instrument").clicked() {
        let name = if app.instrument_name_input.is_empty() {
            format!("Custom {}", app.custom_instruments.len() + 1)
        } else {
            app.instrument_name_input.clone()
        };

        let custom = describe_to_instrument(&app.instrument_description_input, &name);
        app.instrument_gen_result = Some(ai_summary(&app.instrument_description_input, &custom));
        app.pending_custom_instrument = Some(custom);
    }

    if let Some(result) = &app.instrument_gen_result {
        ui.add_space(8.0);
        ui.group(|ui| {
            ui.label(result);
        });
    }

    if let Some(custom) = app.pending_custom_instrument.clone() {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("💾 Save Instrument").clicked() {
                app.save_custom_instrument(custom);
            }
            if ui.button("🎹 Preview").clicked() {
                app.preview_custom_instrument();
            }
        });
    }

    ui.add_space(12.0);
    ui.heading("Saved Custom Instruments");
    if app.custom_instruments.is_empty() {
        ui.label("No custom instruments yet.");
    } else {
        for inst in app.custom_instruments.clone() {
            ui.horizontal(|ui| {
                ui.label(&inst.name);
                ui.label(
                    egui::RichText::new(&inst.description)
                        .italics()
                        .color(egui::Color32::GRAY),
                );
                if ui.button("Use").clicked() {
                    app.select_custom_instrument(&inst.name);
                }
            });
        }
    }

    ui.add_space(8.0);
    ui.collapsing("Future: Cloud AI Integration", |ui| {
        ui.label(
            "This module currently uses local keyword analysis to map descriptions to synth \
             parameters. A future version will connect to cloud LLM APIs for richer sound design. \
             Set OPENAI_API_KEY in your environment when that integration lands.",
        );
    });
}
