use crate::app::DAWApp;
use egui::Ui;

pub fn show_transport(app: &mut DAWApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        if ui
            .button(if app.playing { "⏸ Pause" } else { "▶ Play" })
            .clicked()
        {
            app.toggle_playback();
        }

        if ui.button("⏹ Stop").clicked() {
            app.stop_playback();
        }

        if ui.button("⏮").on_hover_text("Go to start (Home)").clicked() {
            app.go_to_start();
        }

        if ui.button("⏭").on_hover_text("Go to end (End)").clicked() {
            app.go_to_end();
        }

        let record_label = if app.recording {
            "⏺ Recording"
        } else {
            "⏺ Record"
        };
        if ui
            .selectable_label(app.recording, record_label)
            .clicked()
        {
            app.toggle_recording();
        }

        if ui
            .selectable_label(app.show_piano, "🎹 Piano")
            .clicked()
        {
            app.show_piano = !app.show_piano;
        }

        if ui
            .selectable_label(app.show_composition, "🎼 Compose")
            .clicked()
        {
            app.show_composition = !app.show_composition;
        }

        if ui
            .selectable_label(app.show_instrument_designer, "✨ AI Instrument")
            .clicked()
        {
            app.show_instrument_designer = !app.show_instrument_designer;
        }

        if ui
            .selectable_label(app.show_drum_sequencer, "🥁 Drums")
            .clicked()
        {
            if app.show_drum_sequencer {
                app.close_drum_sequencer();
            } else {
                app.open_drum_sequencer();
            }
        }

        if ui
            .selectable_label(app.show_dj_panel, "🎛 DJ")
            .clicked()
        {
            app.show_dj_panel = !app.show_dj_panel;
        }

        if ui
            .selectable_label(app.show_options, "⚙ Options")
            .clicked()
        {
            app.show_options = !app.show_options;
        }

        if ui
            .selectable_label(app.show_user_guide, "📖 Guide")
            .clicked()
        {
            app.show_user_guide = !app.show_user_guide;
        }

        ui.separator();

        ui.label("BPM:");
        if ui
            .add(egui::Slider::new(&mut app.project.bpm, 40.0..=240.0).fixed_decimals(0))
            .changed()
        {
            app.project.bpm = app.project.bpm.round();
        }

        ui.separator();

        ui.label("Time Sig:");
        ui.add(egui::DragValue::new(&mut app.project.time_sig_numerator).range(1..=12));
        ui.label("/");
        let mut denom = app.project.time_sig_denominator as i32;
        egui::ComboBox::from_id_salt("time_sig_denom")
            .selected_text(format!("{}", app.project.time_sig_denominator))
            .show_ui(ui, |ui| {
                for d in [2, 4, 8, 16] {
                    ui.selectable_value(&mut denom, d, format!("{}", d));
                }
            });
        app.project.time_sig_denominator = denom as u8;

        ui.separator();

        ui.label(format!(
            "Beat: {:.1} / {:.0}",
            app.project.playhead_beat, app.project.total_beats
        ));

        ui.separator();

        if ui.button("💾 Save Project").clicked() {
            app.save_project_dialog();
        }
        if ui.button("📂 Load Project").clicked() {
            app.load_project_dialog();
        }
        if ui.button("🎵 Export WAV").clicked() {
            app.export_wav_dialog();
        }

        if let Some(ref msg) = app.status_message {
            ui.label(egui::RichText::new(msg).color(egui::Color32::LIGHT_GREEN));
        }
        if let Some(ref err) = app.status_error {
            ui.label(egui::RichText::new(err).color(egui::Color32::LIGHT_RED));
        }
    });
}
