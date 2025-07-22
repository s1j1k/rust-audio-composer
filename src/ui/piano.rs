use super::super::DAWApp;
use egui::Ui;

// Constants for piano keys
const WHITE_NOTES: [&str; 7] = ["C", "D", "E", "F", "G", "A", "B"];
const BLACK_NOTES: [&str; 5] = ["C#", "D#", "F#", "G#", "A#"];

const ALL_NOTES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

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

// Offsets for black keys relative to the previous black key
const BLACK_KEY_OFFSETS_REV: [f32; 5] = [
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH * 0.5, // A#
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH,       // G#
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH,       // F#
    WHITE_KEY_WIDTH * 2.0 - BLACK_KEY_WIDTH * 0.75, // D#
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH,       // C#
];

// FIXME take oscillator as param instead?
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
                                // TODO add this to a separate function / file
                                // TODO allow different instruments to be selected
                                // TODO pass this command back to another file
                                println!("Played white note: {}", note); // TODO remove comment
                                // let frequency = note_to_frequency(*note);
                                // self.oscillator.set_frequency(frequency);
                                if let Some(freq) = note_to_frequency(*note) {
                                    if let Ok(mut osc) = self.oscillator.lock() {
                                        osc.set_frequency(freq);
                                        // println!("Playing note at {} Hz", freq);
                                    }
                                }
                                // self.oscillator.play();
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
                                        println!("Played black note: {}", note); // TODO remove
                                        // TODO cleanup
                                        // let frequency = note_to_frequency(*note);
                                        // self.oscillator.set_frequency(frequency);
                                        // self.oscillator.play();

                                        if let Some(freq) = note_to_frequency(*note) {
                                            if let Ok(mut osc) = self.oscillator.lock() {
                                                osc.set_frequency(freq);
                                                // println!("Playing note at {} Hz", freq);
                                            }
                                        }
                                    }
                                }
                            });
                        },
                    );
                });
            });
    }

    pub fn show_piano_roll(&mut self, ui: &mut Ui) {
        // TODO add middle C octave piano roll
        // FIXME update so that it is vertical 
        // FIXME manipulate an actual track and record the notes pressed to the piano roll
        // TODO add a cursor and separate into vertical beats
        // TODO add a section separated by beats
        // Create a horizontal layout for the white and black keys

        // FIXME change to available height
        let available_width = ui.available_height();
        ui.horizontal(|ui| {
            // White keys
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0; // No spacing between keys
                for note in WHITE_NOTES.iter().rev() {
                    let response = ui.add(
                        egui::Button::new(
                            // FIXME align this text to the middle
                            egui::RichText::new(*note)
                                .color(egui::Color32::from_rgb(80, 80, 80)), // Dark gray text
                        )
                        .min_size(egui::vec2(WHITE_KEY_HEIGHT, WHITE_KEY_WIDTH))
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY)),
                    );

                    if response.clicked() {
                        // TODO add this to a separate function / file
                        // TODO allow different instruments to be selected
                        // TODO pass this command back to another file
                        println!("Played white note: {}", note); // TODO remove comment
                        // let frequency = note_to_frequency(*note);
                        // self.oscillator.set_frequency(frequency);
                        if let Some(freq) = note_to_frequency(*note) {
                            if let Ok(mut osc) = self.oscillator.lock() {
                                osc.set_frequency(freq);
                                // println!("Playing note at {} Hz", freq);
                            }
                        }
                        // self.oscillator.play();
                    }
                }
            });

            // Black keys (overlaid on white keys)
            // FIXME change to like a grid layout to make it responsive to clicks
            ui.allocate_new_ui(
                    egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                        egui::pos2(ui.min_rect().min.x, ui.min_rect().min.y - 1.0), // Move up by 1 pixel
                        egui::vec2(available_width, BLACK_KEY_HEIGHT),
                    )),
                    |ui| {
                        // NOTE builds from top to bottom
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0; // No spacing
                            for (i, note) in BLACK_NOTES.iter().rev().enumerate() {
                                let space_to_add = BLACK_KEY_OFFSETS_REV[i];

                                // Add space for alignment
                                ui.add_space(space_to_add);

                                // Add black key button
                                let response = ui.add(
                                    egui::Button::new(
                                        egui::RichText::new(*note).color(egui::Color32::WHITE), // White text
                                    )
                                    .min_size(egui::vec2(BLACK_KEY_HEIGHT, BLACK_KEY_WIDTH))
                                    .fill(egui::Color32::BLACK)
                                );

                                if response.clicked() {
                                    println!("Played black note: {}", note); // TODO remove
                                    // TODO cleanup
                                    // let frequency = note_to_frequency(*note);
                                    // self.oscillator.set_frequency(frequency);
                                    // self.oscillator.play();

                                    if let Some(freq) = note_to_frequency(*note) {
                                        if let Ok(mut osc) = self.oscillator.lock() {
                                            osc.set_frequency(freq);
                                            // println!("Playing note at {} Hz", freq);
                                        }
                                    }
                                }
                            }
                        });
                    });
        
            // TODO enclose this and piano roll with a vertical bar showing which beat we're up to 
            // FIXME add this
            // This is the Track Editor part
            // vertical bars for each semitone
            ui.vertical(|ui| {
                for note in WHITE_NOTES.iter().rev() {
                    // Add a full width horizontal bar 
                    // TODO make it different color for sharps/natural notes
                    let response = ui.add(
                        egui::Button::new(
                            // FIXME align this text to the middle
                            // FIXME remove text on this part
                            egui::RichText::new(*note)
                                .color(egui::Color32::from_rgb(80, 80, 80)), // Dark gray text
                        )
                        // FIXME set the width to the width of the window it's contained in
                        .min_size(egui::vec2(100.0, WHITE_KEY_WIDTH * 0.5))
                        .fill(egui::Color32::DARK_GRAY)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY)),
                    );

                    if response.clicked() {
                        // TODO add this to a separate function / file
                        // TODO allow different instruments to be selected
                        // TODO pass this command back to another file
                        println!("Played note: {}", note); // TODO remove comment
                        if let Some(freq) = note_to_frequency(*note) {
                            if let Ok(mut osc) = self.oscillator.lock() {
                                osc.set_frequency(freq);
                            }
                        }
                    }

                    // TODO control onclick action - play note based on duration
                    // TODO add default duration
                    
                 }
            });

            // TODO overlay transparent vertical lines to indicate the beats
        
        });
    }

}

// FIXME this is simplified and does not allow full range of notes
fn note_to_frequency(note: &str) -> Option<f32> {
    match note {
        "C" => Some(261.63),
        "C#" => Some(277.18),
        "D" => Some(293.66),
        "D#" => Some(311.13),
        "E" => Some(329.63),
        "F" => Some(349.23),
        "F#" => Some(369.99),
        "G" => Some(392.00),
        "G#" => Some(415.30),
        "A" => Some(440.00),
        "A#" => Some(466.16),
        "B" => Some(493.88),
        _ => None,
    }
}