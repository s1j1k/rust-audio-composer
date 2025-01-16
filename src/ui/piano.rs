use super::super::DAWApp;
use egui::{Context, Ui};

// TODO move constants to separate file / location
const WHITE_NOTES: [&str; 7] = ["C", "D", "E", "F", "G", "A", "B"];
const BLACK_NOTES: [&str; 5] = ["C#", "D#", "F#", "G#", "A#"];

const WHITE_KEY_WIDTH: f32 = 40.0;
const WHITE_KEY_HEIGHT: f32 = 160.0;
const BLACK_KEY_WIDTH: f32 = 0.6 * WHITE_KEY_WIDTH;
const BLACK_KEY_HEIGHT: f32 = 100.0;

const BLACK_KEY_SPACING: [f32; 5] = [
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH / 2.0,       // C#
    WHITE_KEY_WIDTH / 2.0 - BLACK_KEY_WIDTH / 2.0, // D#
    // minus the previous offset + black_key_width / 2
    WHITE_KEY_WIDTH - (WHITE_KEY_WIDTH / 2.0 - BLACK_KEY_WIDTH / 2.0) / 2.0 + BLACK_KEY_WIDTH / 2.0, // F#
    WHITE_KEY_WIDTH / 2.0 - BLACK_KEY_WIDTH / 2.0, // G#
    WHITE_KEY_WIDTH / 2.0 - BLACK_KEY_WIDTH / 2.0, // A#
];

impl DAWApp {
    pub fn show_piano_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Virtual Piano")
            .open(&mut self.show_piano)
            .default_size([400.0, 200.0])
            .show(ctx, |ui| {
                // Create the horizontal for white keys and adjust position
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(0.0, 10.0), // Adjust the y value for vertical positioning
                        egui::vec2(WHITE_KEY_WIDTH * WHITE_NOTES.len() as f32, WHITE_KEY_HEIGHT),
                    ),
                    |ui: &mut Ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0; // Remove horizontal spacing

                            for (i, note) in WHITE_NOTES.iter().enumerate() {
                                let key_response = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new(*note)
                                            .color(egui::Color32::from_rgb(80, 80, 80)), // Dark gray text
                                    )
                                    .min_size(egui::vec2(WHITE_KEY_WIDTH, WHITE_KEY_HEIGHT))
                                    .fill(egui::Color32::WHITE)
                                    .stroke(egui::Stroke::new(0.1, egui::Color32::LIGHT_GRAY)),
                                );

                                if key_response.clicked() {
                                    println!("Played white note: {}", note);
                                }
                            }
                        })
                        .response
                    },
                );

                // Create the horizontal for black keys and adjust position
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(0.0, 10.0), // Adjust the y value for vertical positioning (same as above)
                        egui::vec2(BLACK_KEY_WIDTH * BLACK_NOTES.len() as f32, BLACK_KEY_HEIGHT),
                    ),
                    |ui: &mut Ui| {
                        ui.horizontal(|ui| {
                            for (i, note) in BLACK_NOTES.iter().enumerate() {
                                // Add custom space before each black key to align it with the corresponding white key
                                ui.add_space(BLACK_KEY_SPACING[i]);

                                let response = ui.add(
                                    egui::Button::new(egui::RichText::new(*note))
                                        .min_size(egui::vec2(BLACK_KEY_WIDTH, BLACK_KEY_HEIGHT))
                                        .fill(egui::Color32::BLACK)
                                        .stroke(egui::Stroke::new(0.5, egui::Color32::WHITE)),
                                );

                                if response.clicked() {
                                    println!("Played black note: {}", note);
                                }
                            }
                        })
                        .response
                    },
                );
            });
    }
}
