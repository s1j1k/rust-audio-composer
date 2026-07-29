use crate::app::DAWApp;
use crate::music::theory::{is_black_key, pitch_to_name};
use crate::ui::theme;
use egui::{Color32, Key, Pos2, Rect, Sense, Ui, Vec2};
use std::time::{Duration, Instant};

const WHITE_KEY_WIDTH: f32 = 40.0;
const WHITE_KEY_HEIGHT: f32 = 160.0;
const BLACK_KEY_WIDTH: f32 = WHITE_KEY_WIDTH * 0.6;
const BLACK_KEY_HEIGHT: f32 = 100.0;

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
    if app.show_piano && !app.keyboard_mapping_seen {
        app.keyboard_mapping_minimized = false;
    }

    let mut show = app.show_piano;
    egui::Window::new("Virtual Piano")
        .open(&mut show)
        .default_size([620.0, 320.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Instrument: {}", app.current_instrument_name()));
                ui.separator();
                ui.label("Octaves:");
                ui.add(egui::Slider::new(&mut app.piano_keyboard_octaves, 1..=4));
                ui.label("Base:");
                let mut oct = (app.piano_keyboard_base / 12) as i32;
                if ui.add(egui::DragValue::new(&mut oct).range(0..=8)).changed() {
                    app.piano_keyboard_base = (oct * 12).clamp(0, 108) as u8;
                }
            });
            ui.label(
                egui::RichText::new(
                    "Click keys (lower = louder) · keyboard: hold longer/firmer for louder attack · \
                     scroll to shift octave · Shift = accent · Alt = soft",
                )
                .small()
                .color(Color32::GRAY),
            );

            if app.keyboard_mapping_minimized {
                if ui.button("ℹ Keyboard map").clicked() {
                    app.keyboard_mapping_minimized = false;
                }
            } else {
                draw_keyboard_mapping_panel(app, ui);
            }

            ui.separator();
            draw_virtual_piano_keys(app, ui);
            handle_keyboard_input(app, ctx);
        });
    app.show_piano = show;
}

fn draw_keyboard_mapping_panel(app: &mut DAWApp, ui: &mut Ui) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.strong("Computer keyboard map");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Minimize").clicked() {
                    app.keyboard_mapping_minimized = true;
                    app.keyboard_mapping_seen = true;
                }
            });
        });
        ui.label("Highlighted keys play notes while this window is open.");
        draw_qwerty_map(ui, app);
    });
}

fn draw_qwerty_map(ui: &mut Ui, app: &DAWApp) {
    let white_map = keyboard_white_pitches(app);
    let black_map = keyboard_black_pitches(app);

    let key_w = 28.0;
    let key_h = 26.0;

    ui.label(
        egui::RichText::new("White keys: Z X C V B N M , . /")
            .small()
            .color(Color32::LIGHT_BLUE),
    );
    ui.horizontal_wrapped(|ui| {
        for (key, pitch) in &white_map {
            draw_map_key(ui, key_label(*key), Some(*pitch), app, key_w, key_h);
        }
    });

    ui.label(
        egui::RichText::new("Black keys: S D G H J")
            .small()
            .color(Color32::LIGHT_BLUE),
    );
    ui.horizontal_wrapped(|ui| {
        for (key, pitch) in &black_map {
            draw_map_key(ui, key_label(*key), Some(*pitch), app, key_w, key_h);
        }
    });
}

fn draw_map_key(ui: &mut Ui, label: &str, pitch: Option<u8>, app: &DAWApp, w: f32, h: f32) {
    let active = pitch.map(|p| app.held_notes.contains(&p)).unwrap_or(false);
    let highlighted = pitch.is_some();
    let fill = if active {
        Color32::from_rgb(120, 180, 255)
    } else if highlighted {
        Color32::from_rgb(70, 110, 170)
    } else {
        Color32::from_rgb(45, 45, 52)
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, fill);
    painter.rect_stroke(rect, 3.0, egui::Stroke::new(1.0, Color32::from_gray(90)));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        theme::font(theme::FONT_MD),
        Color32::WHITE,
    );
    if let Some(p) = pitch {
        painter.text(
            rect.center_bottom() - Vec2::new(0.0, 2.0),
            egui::Align2::CENTER_BOTTOM,
            pitch_to_name(p),
            theme::font(theme::FONT_XS),
            Color32::from_rgb(200, 220, 255),
        );
    }
}

fn key_label(key: Key) -> &'static str {
    match key {
        Key::Z => "Z",
        Key::X => "X",
        Key::C => "C",
        Key::V => "V",
        Key::B => "B",
        Key::N => "N",
        Key::M => "M",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Slash => "/",
        Key::S => "S",
        Key::D => "D",
        Key::G => "G",
        Key::H => "H",
        Key::J => "J",
        _ => "?",
    }
}

fn keyboard_white_pitches(app: &DAWApp) -> Vec<(Key, u8)> {
    let start = app.piano_keyboard_base;
    let end = app.piano_keyboard_end();
    let keys = [
        Key::Z, Key::X, Key::C, Key::V, Key::B, Key::N, Key::M, Key::Comma, Key::Period, Key::Slash,
    ];
    let white: Vec<u8> = (start..=end).filter(|p| !is_black_key(*p)).collect();
    keys.iter()
        .zip(white.iter())
        .map(|(&k, &p)| (k, p))
        .collect()
}

fn keyboard_black_pitches(app: &DAWApp) -> Vec<(Key, u8)> {
    let start = app.piano_keyboard_base;
    let end = app.piano_keyboard_end();
    let keys = [Key::S, Key::D, Key::G, Key::H, Key::J];
    let black: Vec<u8> = (start..=end).filter(|p| is_black_key(*p)).collect();
    keys.iter()
        .zip(black.iter())
        .map(|(&k, &p)| (k, p))
        .collect()
}

fn draw_virtual_piano_keys(app: &mut DAWApp, ui: &mut Ui) {
    let start = app.piano_keyboard_base;
    let end = app.piano_keyboard_end();

    let white_pitches: Vec<u8> = (start..=end).filter(|p| !is_black_key(*p)).collect();
    let total_width = white_pitches.len() as f32 * WHITE_KEY_WIDTH;

    let (area, response) =
        ui.allocate_exact_size(Vec2::new(total_width, WHITE_KEY_HEIGHT + 8.0), Sense::hover());

    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y + i.raw_scroll_delta.y);
        if scroll.abs() > 0.01 {
            let delta = if scroll > 0.0 { -12 } else { 12 };
            app.piano_keyboard_base =
                (app.piano_keyboard_base as i32 + delta).clamp(0, 108) as u8;
        }
    }

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
            theme::font(theme::FONT_MD),
            Color32::DARK_GRAY,
        );

        let key_response = ui.interact(key_rect, ui.id().with(("w", wp)), Sense::click());
        if key_response.is_pointer_button_down_on() {
            let vel = velocity_from_key_rect(key_rect, key_response.interact_pointer_pos());
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
            if bp <= end {
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

                let key_response = ui.interact(key_rect, ui.id().with(("b", bp)), Sense::click());
                if key_response.is_pointer_button_down_on() {
                    let vel = velocity_from_key_rect(key_rect, key_response.interact_pointer_pos());
                    app.play_note_with_velocity(bp, vel);
                } else if app.held_notes.contains(&bp) {
                    app.release_note(bp);
                }
            }
        }
        x += WHITE_KEY_WIDTH;
    }
}

fn keyboard_velocity_modifier(ctx: &egui::Context) -> f32 {
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

/// Maps hold duration to velocity, mirroring the mouse key's vertical position range.
fn keyboard_velocity_from_hold(ctx: &egui::Context, hold: Duration) -> f32 {
    let accent = keyboard_velocity_modifier(ctx);
    let hold_ms = hold.as_secs_f32() * 1000.0;
    // Quick firm press reaches full level in ~120 ms; light taps stay softer.
    let pressure = (1.0 - (-hold_ms / 80.0).exp()).clamp(0.0, 1.0);
    let dynamic = 0.35 + pressure * 0.65;
    (accent * dynamic).clamp(0.05, 1.0)
}

fn handle_keyboard_input(app: &mut DAWApp, ctx: &egui::Context) {
    let white_map = keyboard_white_pitches(app);
    let black_map = keyboard_black_pitches(app);
    let now = Instant::now();

    ctx.input(|i| {
        for (key, pitch) in white_map.iter().chain(black_map.iter()) {
            if i.key_down(*key) {
                if !app.held_notes.contains(pitch) {
                    app.play_note_with_velocity(*pitch, keyboard_velocity_from_hold(ctx, Duration::ZERO));
                } else if let Some(state) = app.held_note_details.get(pitch) {
                    let hold = now.duration_since(state.press_start);
                    let vel = keyboard_velocity_from_hold(ctx, hold);
                    if (vel - state.peak_velocity).abs() > 0.02 {
                        app.update_held_note_velocity(*pitch, vel);
                    }
                }
            } else if app.held_notes.contains(pitch) {
                app.release_note(*pitch);
            }
        }
    });
}
