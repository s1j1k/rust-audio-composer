use crate::app::DAWApp;
use crate::music::theory::{is_black_key, pitch_to_name};
use egui::{Color32, Key, Vec2};

const WHITE_KEY_WIDTH: f32 = 32.0;
const WHITE_KEY_HEIGHT: f32 = 120.0;
const BLACK_KEY_WIDTH: f32 = 20.0;
const BLACK_KEY_HEIGHT: f32 = 72.0;
const BLACK_KEY_OFFSETS: [f32; 5] = [
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH * 0.5,
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH,
    WHITE_KEY_WIDTH * 2.0 - BLACK_KEY_WIDTH,
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH,
    WHITE_KEY_WIDTH - BLACK_KEY_WIDTH,
];

const PIANO_START: u8 = 48;
const PIANO_END: u8 = 72;
const BLACK_NOTES: [u8; 5] = [49, 51, 54, 56, 58];

pub fn show_piano_window(app: &mut DAWApp, ctx: &egui::Context) {
    let mut show = app.show_piano;
    egui::Window::new("Virtual Piano")
        .open(&mut show)
        .default_size([520.0, 200.0])
        .show(ctx, |ui| {
            ui.label("Click keys or use your keyboard (Z-M = white keys, S/D/G/H/J = black keys).");
            ui.label(format!("Instrument: {}", app.current_instrument_name()));

            let white_pitches: Vec<u8> = (PIANO_START..=PIANO_END)
                .filter(|p| !is_black_key(*p))
                .collect();

            let total_width = white_pitches.len() as f32 * WHITE_KEY_WIDTH;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for pitch in &white_pitches {
                    let pressed = app.held_notes.contains(pitch);
                    let fill = if pressed {
                        Color32::from_rgb(180, 200, 255)
                    } else {
                        Color32::WHITE
                    };

                    let response = ui.add(
                        egui::Button::new(pitch_to_name(*pitch))
                            .min_size(Vec2::new(WHITE_KEY_WIDTH, WHITE_KEY_HEIGHT))
                            .fill(fill)
                            .stroke(egui::Stroke::new(1.0, Color32::GRAY)),
                    );

                    if response.is_pointer_button_down_on() {
                        app.play_note(*pitch);
                    } else if app.held_notes.contains(pitch) {
                        app.release_note(*pitch);
                    }
                }
            });

            ui.allocate_new_ui(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    egui::pos2(ui.min_rect().min.x, ui.min_rect().min.y - BLACK_KEY_HEIGHT),
                    egui::vec2(total_width, BLACK_KEY_HEIGHT),
                )),
                |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        for (i, pitch) in BLACK_NOTES.iter().enumerate() {
                            ui.add_space(BLACK_KEY_OFFSETS[i]);
                            let pressed = app.held_notes.contains(pitch);
                            let fill = if pressed {
                                Color32::from_rgb(80, 80, 120)
                            } else {
                                Color32::BLACK
                            };

                            let response = ui.add(
                                egui::Button::new("")
                                    .min_size(Vec2::new(BLACK_KEY_WIDTH, BLACK_KEY_HEIGHT))
                                    .fill(fill),
                            );

                            if response.is_pointer_button_down_on() {
                                app.play_note(*pitch);
                            } else if app.held_notes.contains(pitch) {
                                app.release_note(*pitch);
                            }
                        }
                    });
                },
            );

            handle_keyboard_input(app, ctx);
        });
    app.show_piano = show;
}

fn handle_keyboard_input(app: &mut DAWApp, ctx: &egui::Context) {
    let white_map = [
        (Key::Z, 48),
        (Key::X, 50),
        (Key::C, 52),
        (Key::V, 53),
        (Key::B, 55),
        (Key::N, 57),
        (Key::M, 59),
        (Key::Comma, 60),
        (Key::Period, 62),
        (Key::Slash, 64),
    ];
    let black_map = [
        (Key::S, 49),
        (Key::D, 51),
        (Key::G, 56),
        (Key::H, 58),
        (Key::J, 60),
    ];

    ctx.input(|i| {
        for (key, pitch) in white_map.iter().chain(black_map.iter()) {
            if i.key_pressed(*key) {
                app.play_note(*pitch);
            }
            if i.key_released(*key) {
                app.release_note(*pitch);
            }
        }
    });
}
