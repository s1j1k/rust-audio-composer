use super::super::DAWApp;

// Constants for piano keys
const WHITE_NOTES: [&str; 7] = ["C", "D", "E", "F", "G", "A", "B"];
const BLACK_NOTES: [&str; 5] = ["C#", "D#", "F#", "G#", "A#"];

const WHITE_KEY_WIDTH: f32 = 40.0;
const WHITE_KEY_HEIGHT: f32 = 160.0;
const BLACK_KEY_WIDTH: f32 = 0.6 * WHITE_KEY_WIDTH;
const BLACK_KEY_HEIGHT: f32 = 100.0;

// Offsets for black keys relative to the previous black key
const BLACK_KEY_OFFSETS: [f32; 5] = [
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH * 0.5, // C#
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH,       // D#
    WHITE_KEY_WIDTH * 2.0 - BLACK_KEY_WIDTH, // F#
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH,       // G#
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH,       // A#
];

impl DAWApp {
    pub fn show_piano_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Virtual Piano")
            .open(&mut self.show_piano)
            .default_size([400.0, 200.0])
            .show(ctx, |ui| {
                let available_width = ui.available_width();

                // Create a vertical layout for the white and black keys
                ui.vertical(|ui| {
                    // White keys
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0; // No spacing between keys
                        for note in WHITE_NOTES.iter() {
                            let response = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(*note)
                                        .color(egui::Color32::from_rgb(80, 80, 80)), // Dark gray text
                                )
                                .min_size(egui::vec2(WHITE_KEY_WIDTH, WHITE_KEY_HEIGHT))
                                .fill(egui::Color32::WHITE)
                                .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY)),
                            );

                            if response.clicked() {
                                println!("Played white note: {}", note);
                            }
                        }
                    });

                    // Black keys (overlaid on white keys)
                    ui.allocate_new_ui(
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                            egui::pos2(ui.min_rect().min.x, ui.min_rect().min.y - 1.0), // Move up by 1 pixel
                            egui::vec2(available_width, BLACK_KEY_HEIGHT),
                        )),
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0; // No spacing
                                for (i, note) in BLACK_NOTES.iter().enumerate() {
                                    let space_to_add = BLACK_KEY_OFFSETS[i];

                                    // Add space for alignment
                                    ui.add_space(space_to_add);

                                    // Add black key button
                                    let response = ui.add(
                                        egui::Button::new(
                                            egui::RichText::new(*note).color(egui::Color32::WHITE), // White text
                                        )
                                        .min_size(egui::vec2(BLACK_KEY_WIDTH, BLACK_KEY_HEIGHT))
                                        .fill(egui::Color32::BLACK)
                                    );

                                    if response.clicked() {
                                        println!("Played black note: {}", note);
                                    }
                                }
                            });
                        },
                    );
                });
            });
    }
}
