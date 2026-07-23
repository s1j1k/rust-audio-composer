use crate::app::DAWApp;
use crate::music::theory::{is_black_key, pitch_to_name};
use egui::{Color32, Key, Pos2, Rect, Sense, Ui, Vec2};

const WHITE_KEY_WIDTH: f32 = 40.0;
const WHITE_KEY_HEIGHT: f32 = 160.0;
const BLACK_KEY_WIDTH: f32 = WHITE_KEY_WIDTH * 0.6;
const BLACK_KEY_HEIGHT: f32 = 100.0;

const PIANO_START: u8 = 48;
const PIANO_END: u8 = 72;

fn white_key_has_sharp(pitch: u8) -> bool {
    matches!(pitch % 12, 0 | 2 | 5 | 7 | 9)
}

fn velocity_from_key_rect(rect: Rect, pos: Option<Pos2>) -> f32 {
    pos.map(|p| {
        let t = ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
        0.35 + (1.0 - t) * 0.65
    })
    .unwrap_or(0.75)
}

pub fn show_piano_window(app: &mut DAWApp, ctx: &egui::Context) {
    let mut show = app.show_piano;
    egui::Window::new("Virtual Piano")
        .open(&mut show)
        .default_size([560.0, 220.0])
        .show(ctx, |ui| {
            ui.label(
                "Click keys (lower = louder) or use keyboard. Shift = accent, Alt = soft. \
                 Z–M white keys, S/D/G/H/J black keys.",
            );
            ui.label(format!("Instrument: {}", app.current_instrument_name()));

            let white_pitches: Vec<u8> = (PIANO_START..=PIANO_END)
                .filter(|p| !is_black_key(*p))
                .collect();
            let total_width = white_pitches.len() as f32 * WHITE_KEY_WIDTH;

            let (area, _) =
                ui.allocate_exact_size(Vec2::new(total_width, WHITE_KEY_HEIGHT + 8.0), Sense::click());
            let painter = ui.painter_at(area);
            let origin = area.min;

            let mut x = origin.x;
            for &wp in &white_pitches {
                let key_rect = Rect::from_min_size(
                    egui::pos2(x, origin.y),
                    Vec2::new(WHITE_KEY_WIDTH, WHITE_KEY_HEIGHT),
                );
                let pressed = app.held_notes.contains(&wp);
                let fill = if pressed {
                    Color32::from_rgb(180, 200, 255)
                } else {
                    Color32::WHITE
                };
                painter.rect_filled(key_rect, 2.0, fill);
                painter.rect_stroke(key_rect, 2.0, egui::Stroke::new(1.0_f32, Color32::GRAY));
                painter.text(
                    key_rect.center_bottom() - Vec2::new(0.0, 10.0),
                    egui::Align2::CENTER_BOTTOM,
                    pitch_to_name(wp),
                    egui::FontId::proportional(10.0),
                    Color32::DARK_GRAY,
                );

                let response = ui.interact(key_rect, ui.id().with(("w", wp)), Sense::click());
                if response.is_pointer_button_down_on() {
                    let vel = velocity_from_key_rect(key_rect, response.interact_pointer_pos());
                    app.play_note_with_velocity(wp, vel);
                } else if app.held_notes.contains(&wp) {
                    app.release_note(wp);
                }
                x += WHITE_KEY_WIDTH;
            }

            x = origin.x;
            for &wp in &white_pitches {
                if white_key_has_sharp(wp) {
                    let bp = wp + 1;
                    if bp <= PIANO_END {
                        let bx = x + WHITE_KEY_WIDTH - BLACK_KEY_WIDTH * 0.5;
                        let key_rect = Rect::from_min_size(
                            egui::pos2(bx, origin.y),
                            Vec2::new(BLACK_KEY_WIDTH, BLACK_KEY_HEIGHT),
                        );
                        let pressed = app.held_notes.contains(&bp);
                        let fill = if pressed {
                            Color32::from_rgb(80, 80, 120)
                        } else {
                            Color32::BLACK
                        };
                        painter.rect_filled(key_rect, 2.0, fill);

                        let response = ui.interact(key_rect, ui.id().with(("b", bp)), Sense::click());
                        if response.is_pointer_button_down_on() {
                            let vel = velocity_from_key_rect(key_rect, response.interact_pointer_pos());
                            app.play_note_with_velocity(bp, vel);
                        } else if app.held_notes.contains(&bp) {
                            app.release_note(bp);
                        }
                    }
                }
                x += WHITE_KEY_WIDTH;
            }

            handle_keyboard_input(app, ctx);
        });
    app.show_piano = show;
}

fn keyboard_velocity(ctx: &egui::Context) -> f32 {
    ctx.input(|i| {
        if i.modifiers.shift {
            1.0
        } else if i.modifiers.alt {
            0.4
        } else {
            0.75
        }
    })
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

    let vel = keyboard_velocity(ctx);
    ctx.input(|i| {
        for (key, pitch) in white_map.iter().chain(black_map.iter()) {
            if i.key_pressed(*key) {
                app.play_note_with_velocity(*pitch, vel);
            }
            if i.key_released(*key) {
                app.release_note(*pitch);
            }
        }
    });
}
