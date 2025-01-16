use super::super::DAWApp;
use egui::Context;

// TODO move constants to separate file / location
const WHITE_NOTES: [&str; 7] = ["C", "D", "E", "F", "G", "A", "B"];
const BLACK_NOTES: [&str; 5] = ["C#", "D#", "F#", "G#", "A#"];

impl DAWApp {
    pub fn show_piano_window(&mut self, ctx: &egui::Context) {
        let white_key_width = 40.0;
        let white_key_height = 160.0;
        let black_key_width = 30.0;
        let black_key_height = 100.0;

        egui::Window::new("Virtual Piano")
            .open(&mut self.show_piano)
            .default_size([400.0, 200.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Remove horizontal spacing between buttons
                    ui.spacing_mut().item_spacing.x = 0.0;

                    // Draw white keys
                    for (i, note) in WHITE_NOTES.iter().enumerate() {
                        let response = ui.add(
                            egui::Button::new(
                                egui::RichText::new(*note)
                                    .color(egui::Color32::from_rgb(80, 80, 80)), // Dark gray text
                            )
                            .min_size(egui::vec2(white_key_width, white_key_height))
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(0.1, egui::Color32::LIGHT_GRAY)),
                        );

                        if response.clicked() {
                            println!("Played white note: {}", note);
                        }
                    }
                });

                ui.horizontal(|ui| {
                    // Align black keys on top of white keys
                    ui.spacing_mut().item_spacing.x = 0.0;

                    for (i, note) in BLACK_NOTES.iter().enumerate() {
                        // Calculate the position of black keys
                        let black_key_offset = match i {
                            0 => 0.7, // C#
                            1 => 1.7, // D#
                            2 => 3.7, // F#
                            3 => 4.7, // G#
                            4 => 5.7, // A#
                            _ => unreachable!(),
                        };

                        // Overlay the black keys on the white keys
                        // let response = ui.add(
                        //     egui::Button::new(egui::RichText::new(*note))
                        //         .min_size(egui::vec2(black_key_width, black_key_height))
                        //         .fill(egui::Color32::BLACK)
                        //         .stroke(egui::Stroke::new(0.5, egui::Color32::WHITE))
                        //         .translate(egui::vec2(white_key_width * black_key_offset, 0.0)),
                        // );

                    // Position the black keys using `ui.put()`
                    let response = ui.put(
                        egui::Rect::from_min_size(
                            egui::pos2(white_key_width * black_key_offset, 0.0),
                            egui::vec2(black_key_width, black_key_height),
                        ),
                        egui::Button::new(egui::RichText::new(*note))
                            .fill(egui::Color32::BLACK)
                            .stroke(egui::Stroke::new(0.5, egui::Color32::WHITE)),
                    );

                        if response.clicked() {
                            println!("Played black note: {}", note);
                        }
                    }
                });
            });
    }
}
