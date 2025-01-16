use egui::Context;
use super::super::DAWApp;

// TODO move constants to separate file / location
const WHITE_NOTES: [&str; 7] = ["C", "D", "E", "F", "G", "A", "B"];

impl DAWApp {
    pub fn show_piano_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Virtual Piano")
            .open(&mut self.show_piano)
            .default_size([400.0, 200.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Remove horizontal spacing between buttons
                    ui.spacing_mut().item_spacing.x = 0.0;

                    let white_key_width = 40.0;
                    let white_key_height = 160.0;

                    for (i, note) in WHITE_NOTES.iter().enumerate() {
                        // Draw the white keys
                        let response = ui.add(egui::Button::new(
                            egui::RichText::new(*note).color(egui::Color32::from_rgb(80, 80, 80)), // Dark gray text
                        )
                        .min_size(egui::vec2(white_key_width, white_key_height))
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(0.1, egui::Color32::LIGHT_GRAY)));

                        if response.clicked() {
                            println!("Played white note: {}", note);
                        }
                    }
                });
            });
    }
}
