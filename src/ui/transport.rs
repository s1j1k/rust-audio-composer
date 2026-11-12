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

        ui.separator();
        if ui
            .add_enabled(app.can_undo(), egui::Button::new("Undo"))
            .on_hover_text("Undo the last edit (Ctrl+Z)")
            .clicked()
        {
            app.undo();
        }
        if ui
            .add_enabled(app.can_redo(), egui::Button::new("Redo"))
            .on_hover_text("Redo (Ctrl+Y or Ctrl+Shift+Z)")
            .clicked()
        {
            app.redo();
        }
        ui.separator();

        if let Some(left) = app.count_in_beats_left() {
            let label = egui::RichText::new(format!("Count-in {left}…  (cancel)"))
                .color(egui::Color32::WHITE)
                .strong();
            if ui
                .add(egui::Button::new(label).fill(egui::Color32::from_rgb(200, 130, 20)))
                .on_hover_text("Recording starts when the count-in reaches the playhead. Click or Space to cancel.")
                .clicked()
            {
                app.finish_recording();
            }
        } else if app.recording {
            // Unmistakable while a take is running: a solid red Stop button with a timer.
            let secs = app.recording_since.map(|t| t.elapsed().as_secs()).unwrap_or(0);
            let mode = if app.recording_overdub { "overdub" } else { "new take" };
            let blink = (ui.input(|i| i.time) * 2.0) as i64 % 2 == 0;
            let label = egui::RichText::new(format!(
                "{} Stop recording  {}:{:02}",
                if blink { "⏹" } else { "■" },
                secs / 60,
                secs % 60
            ))
            .color(egui::Color32::WHITE)
            .strong();
            if ui
                .add(egui::Button::new(label).fill(egui::Color32::from_rgb(200, 40, 40)))
                .on_hover_text(format!("Recording ({mode}) — click or press Space to stop"))
                .clicked()
            {
                app.finish_recording();
            }
        } else if ui
            .button("⏺ Record")
            .on_hover_text(
                "Start recording at the playhead (the playhead rolls). Inside an existing \
                 bubble = overdub into it; on empty space = new bubble. Space stops.",
            )
            .clicked()
        {
            app.start_recording();
        }

        if ui
            .selectable_label(app.show_piano || app.bottom_keyboard, "🎹 Piano")
            .on_hover_text("Virtual piano in the bottom panel (Pop out there for a separate window)")
            .clicked()
        {
            if app.show_piano {
                app.show_piano = false; // close the pop-out
            } else {
                app.bottom_keyboard = !app.bottom_keyboard;
            }
            app.sync_sequencer_visibility();
        }

        if ui
            .selectable_label(app.show_remote, "📱 Remote")
            .on_hover_text("Play from your iPhone / iPad over Wi-Fi")
            .clicked()
        {
            app.show_remote = !app.show_remote;
            if app.show_remote {
                app.start_remote();
            }
        }

        if ui
            .selectable_label(app.show_composition, "🎼 Compose")
            .clicked()
        {
            app.show_composition = !app.show_composition;
        }

        if ui
            .selectable_label(app.show_instrument_designer, "✨ Sound Designer")
            .clicked()
        {
            app.show_instrument_designer = !app.show_instrument_designer;
        }

        if ui
            .selectable_label(app.show_drum_sequencer, "Drums")
            .on_hover_text("Step sequencer for the selected drum track (shown in the bottom panel)")
            .clicked()
        {
            if app.show_drum_sequencer {
                app.close_drum_sequencer();
            } else {
                app.open_drum_sequencer();
            }
        }

        if ui
            .selectable_label(app.show_dj_panel, "DJ")
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
