use crate::app::DAWApp;
use crate::music::theory::{is_black_key, pitch_to_name};
use crate::ui::theme;
use egui::{Color32, Key, Pos2, Rect, Response, Sense, Ui, Vec2};
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

/// Mouse play on a key: strike position sets velocity; dragging down on a held key adds
/// pressure (aftertouch), like leaning into a real key.
fn press_key(app: &mut DAWApp, ui: &Ui, rect: Rect, resp: &Response, pitch: u8) {
    let id = egui::Id::new(("piano_press", pitch));
    if resp.is_pointer_button_down_on() {
        let pos = resp.interact_pointer_pos();
        if !app.held_notes.contains(&pitch) {
            let raw = velocity_from_key_rect(rect, pos);
            let vel = app.velocity_curve.apply(raw);
            app.play_note_with_velocity(pitch, vel);
            if let Some(p) = pos {
                ui.ctx().data_mut(|d| d.insert_temp(id, p.y));
            }
        } else if let (Some(p), Some(y0)) = (pos, ui.ctx().data(|d| d.get_temp::<f32>(id))) {
            let pressure = ((p.y - y0) / 50.0).clamp(0.0, 1.0);
            if let Ok(mut eng) = app.audio_engine.lock() {
                eng.set_note_pressure(pitch, pressure);
            }
        }
    } else if app.held_notes.contains(&pitch) && !app.keyboard_held_notes.contains(&pitch) {
        // Mouse let go. (Notes held on the computer keyboard are released by their key.)
        app.release_note(pitch);
    }
}

/// HSL (h in degrees, s/l in 0..1) to an sRGB colour — same colour model as the phone UI.
fn hsl(h: f32, s: f32, l: f32) -> Color32 {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let f = |v: f32| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(f(r), f(g), f(b))
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// Who is holding a key and how hard: local (mouse / computer keyboard) or a phone.
fn key_hold(app: &DAWApp, pitch: u8) -> Option<(f32, bool)> {
    if app.held_notes.contains(&pitch) {
        let v = app.held_note_details.get(&pitch).map(|s| s.peak_velocity).unwrap_or(0.75);
        Some((v, false))
    } else {
        app.remote_held.get(&pitch).map(|s| (s.peak_velocity, true))
    }
}

/// The pressed colour, matching the phone: blue for local playing (green for a phone),
/// deeper for harder hits.
fn pressed_color(vel: f32, remote: bool, black: bool) -> Color32 {
    let v = vel.clamp(0.0, 1.0);
    let hue = if remote { 140.0 } else { 215.0 };
    if black {
        hsl(hue, 0.70, 0.22 + v * 0.30)
    } else {
        hsl(hue, 0.85, 0.88 - v * 0.30)
    }
}

/// Floating window, only when the piano is popped out. Closing it docks it back into the
/// bottom panel.
pub fn show_piano_window(app: &mut DAWApp, ctx: &egui::Context) {
    if !app.show_piano {
        return;
    }
    let mut show = true;
    egui::Window::new("Virtual Piano")
        .open(&mut show)
        .default_size([720.0, 300.0])
        .show(ctx, |ui| {
            show_piano_body(app, ui, false);
        });
    if !show {
        app.show_piano = false;
        app.bottom_keyboard = true;
    }
}

/// The virtual piano: controls, optional computer-keyboard map, and the keys. Used docked
/// in the bottom panel (`docked = true`) and in the pop-out window.
pub fn show_piano_body(app: &mut DAWApp, ui: &mut Ui, docked: bool) {
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
        ui.separator();
        let label = if app.keyboard_mapping_minimized { "Show keyboard map" } else { "Hide keyboard map" };
        if ui.small_button(label).clicked() {
            app.keyboard_mapping_minimized = !app.keyboard_mapping_minimized;
            app.keyboard_mapping_seen = true;
        }
    });
    ui.label(
        egui::RichText::new(
            "Click keys (lower = louder) · drag down on a held key for pressure · computer keys \
             Z–/ and S D G H J play (letters shown on the keys) · Shift = accent · Alt = soft · \
             scroll to shift octave · green = played from a phone",
        )
        .small()
        .color(Color32::GRAY),
    );
    if !app.keyboard_mapping_minimized {
        draw_keyboard_mapping_panel(app, ui);
    }
    ui.add_space(4.0);
    let max_h = if docked { (ui.available_height() - 6.0).clamp(90.0, 170.0) } else { WHITE_KEY_HEIGHT };
    draw_virtual_piano_keys(app, ui, max_h);
}

/// Computer-keyboard playing, active whenever the virtual piano is on screen (docked or
/// popped out). Called once per frame from the app.
pub fn piano_keyboard_input(app: &mut DAWApp, ctx: &egui::Context) {
    handle_keyboard_input(app, ctx);
}

fn draw_keyboard_mapping_panel(app: &mut DAWApp, ui: &mut Ui) {
    ui.group(|ui| {
        ui.strong("Computer keyboard map");
        ui.label("These keys play notes while the virtual piano is showing.");
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
        theme::mono(theme::FONT_MD),
        Color32::WHITE,
    );
    if let Some(p) = pitch {
        painter.text(
            rect.center_bottom() - Vec2::new(0.0, 2.0),
            egui::Align2::CENTER_BOTTOM,
            pitch_to_name(p),
            theme::mono(theme::FONT_XS),
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

/// `(press animation 0..1, velocity, from a phone?)` for a key. The press eases in fast
/// (should feel instant) and fades out a little slower; the last velocity/source is
/// remembered so the release fade keeps its colour.
fn key_look(app: &DAWApp, ctx: &egui::Context, pitch: u8) -> (f32, f32, bool) {
    let look_id = egui::Id::new(("vkey_look", pitch));
    let hold = key_hold(app, pitch);
    if let Some(h) = hold {
        ctx.data_mut(|d| d.insert_temp(look_id, h));
    }
    let (v, remote) = ctx.data(|d| d.get_temp::<(f32, bool)>(look_id)).unwrap_or((0.7, false));
    let held = hold.is_some();
    let anim = ctx.animate_bool_with_time(
        egui::Id::new(("vkey_anim", pitch)),
        held,
        if held { 0.04 } else { 0.18 },
    );
    (anim, v, remote)
}

/// One key's look: `anim` is 0 (up) → 1 (fully pressed), eased by egui so presses snap
/// in and releases fade out, like the phone keyboard.
fn paint_key(
    painter: &egui::Painter,
    rect: Rect,
    black: bool,
    anim: f32,
    vel: f32,
    remote: bool,
) {
    let base = if black { Color32::from_rgb(22, 22, 26) } else { Color32::from_rgb(250, 250, 252) };
    let fill = lerp_color(base, pressed_color(vel, remote, black), anim);
    // Pressed keys sink a little: shorter by a few px with a shadow at the top edge.
    let sink = 3.0 * anim;
    let r = Rect::from_min_max(rect.min + Vec2::new(0.0, sink), rect.max);
    let rounding = egui::Rounding { nw: 0.0, ne: 0.0, sw: 4.0, se: 4.0 };
    painter.rect_filled(r, rounding, fill);
    if anim > 0.01 {
        let shadow = Rect::from_min_size(r.min, Vec2::new(r.width(), 6.0));
        painter.rect_filled(shadow, 0.0, Color32::from_black_alpha((70.0 * anim) as u8));
    }
    if !black {
        painter.rect_stroke(r, rounding, egui::Stroke::new(1.0, Color32::from_gray(150)));
    } else if anim < 0.99 {
        // A sheen on raised black keys.
        let sheen = Rect::from_min_max(
            egui::pos2(r.min.x + 3.0, r.max.y - 10.0),
            egui::pos2(r.max.x - 3.0, r.max.y - 4.0),
        );
        painter.rect_filled(sheen, 2.0, Color32::from_white_alpha((28.0 * (1.0 - anim)) as u8));
    }
}

fn draw_virtual_piano_keys(app: &mut DAWApp, ui: &mut Ui, max_height: f32) {
    let start = app.piano_keyboard_base;
    let end = app.piano_keyboard_end();

    let white_pitches: Vec<u8> = (start..=end).filter(|p| !is_black_key(*p)).collect();
    // Fill the available width (docked) within comfortable key sizes.
    let key_w = (ui.available_width() / white_pitches.len().max(1) as f32).clamp(22.0, 56.0);
    let key_h = max_height.min(WHITE_KEY_HEIGHT * key_w / WHITE_KEY_WIDTH).max(80.0);
    let black_w = key_w * 0.6;
    let black_h = key_h * (BLACK_KEY_HEIGHT / WHITE_KEY_HEIGHT);
    let total_width = white_pitches.len() as f32 * key_w;

    // Computer-keyboard letters, drawn on the keys they play.
    let letters: std::collections::HashMap<u8, &'static str> = keyboard_white_pitches(app)
        .into_iter()
        .chain(keyboard_black_pitches(app))
        .map(|(k, p)| (p, key_label(k)))
        .collect();

    let (area, response) =
        ui.allocate_exact_size(Vec2::new(total_width, key_h + 4.0), Sense::hover());

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

    let ctx = ui.ctx().clone();
    let look = |app: &DAWApp, pitch: u8| key_look(app, &ctx, pitch);

    let mut x = origin.x;
    for &wp in &white_pitches {
        let key_rect = Rect::from_min_size(egui::pos2(x, origin.y), Vec2::new(key_w, key_h));
        let (anim, vel, remote) = look(app, wp);
        paint_key(&painter, key_rect, false, anim, vel, remote);
        let text_color = lerp_color(Color32::from_gray(110), Color32::from_gray(40), anim);
        painter.text(
            key_rect.center_bottom() - Vec2::new(0.0, 6.0),
            egui::Align2::CENTER_BOTTOM,
            pitch_to_name(wp),
            theme::mono(theme::FONT_SM),
            text_color,
        );
        if let Some(letter) = letters.get(&wp) {
            painter.text(
                key_rect.center_bottom() - Vec2::new(0.0, 22.0),
                egui::Align2::CENTER_BOTTOM,
                *letter,
                theme::mono(theme::FONT_MD),
                lerp_color(Color32::from_rgb(70, 110, 180), Color32::from_rgb(15, 35, 80), anim),
            );
        }

        let key_response = ui.interact(key_rect, ui.id().with(("w", wp)), Sense::click_and_drag());
        press_key(app, ui, key_rect, &key_response, wp);
        x += key_w;
    }

    x = origin.x;
    for &wp in &white_pitches {
        if white_key_has_sharp(wp) {
            let bp = wp + 1;
            if bp <= end {
                let bx = x + key_w - black_w * 0.5;
                let key_rect =
                    Rect::from_min_size(egui::pos2(bx, origin.y), Vec2::new(black_w, black_h));
                let (anim, vel, remote) = look(app, bp);
                paint_key(&painter, key_rect, true, anim, vel, remote);
                if let Some(letter) = letters.get(&bp) {
                    painter.text(
                        key_rect.center_bottom() - Vec2::new(0.0, 14.0),
                        egui::Align2::CENTER_BOTTOM,
                        *letter,
                        theme::mono(theme::FONT_SM),
                        Color32::from_rgb(150, 185, 240),
                    );
                }

                let key_response =
                    ui.interact(key_rect, ui.id().with(("b", bp)), Sense::click_and_drag());
                press_key(app, ui, key_rect, &key_response, bp);
            }
        }
        x += key_w;
    }
}

fn keyboard_velocity_modifier(modifiers: egui::Modifiers) -> f32 {
    if modifiers.shift {
        1.0
    } else if modifiers.alt {
        0.4
    } else {
        0.75
    }
}

/// Maps hold duration to velocity, mirroring the mouse key's vertical position range.
fn keyboard_velocity_from_hold(accent: f32, hold: Duration) -> f32 {
    let hold_ms = hold.as_secs_f32() * 1000.0;
    // Quick firm press reaches full level in ~120 ms; light taps stay softer.
    let pressure = (1.0 - (-hold_ms / 80.0).exp()).clamp(0.0, 1.0);
    let dynamic = 0.35 + pressure * 0.65;
    (accent * dynamic).clamp(0.05, 1.0)
}

fn handle_keyboard_input(app: &mut DAWApp, ctx: &egui::Context) {
    let mapped: Vec<(Key, u8)> = keyboard_white_pitches(app)
        .into_iter()
        .chain(keyboard_black_pitches(app))
        .collect();
    let now = Instant::now();

    // Read everything in one short `ctx.input` call and act *after* it returns.
    // `ctx.input` holds egui's context lock; calling back into `ctx` (or doing slow work)
    // inside the closure re-locks it on the same thread and freezes the whole window.
    let typing = ctx.wants_keyboard_input();
    let (accent, down): (f32, Vec<bool>) = ctx.input(|i| {
        (
            keyboard_velocity_modifier(i.modifiers),
            // Ctrl/Cmd + key is a shortcut (Ctrl+D duplicate, ...), never a note.
            mapped
                .iter()
                .map(|(k, _)| !typing && !i.modifiers.command && i.key_down(*k))
                .collect(),
        )
    });

    for (&(_, pitch), is_down) in mapped.iter().zip(down) {
        let ours = app.keyboard_held_notes.contains(&pitch);
        if is_down {
            if !app.held_notes.contains(&pitch) {
                app.play_note_with_velocity(pitch, keyboard_velocity_from_hold(accent, Duration::ZERO));
                app.keyboard_held_notes.insert(pitch);
            } else if ours {
                if let Some(state) = app.held_note_details.get(&pitch) {
                    let hold = now.duration_since(state.press_start);
                    let vel = keyboard_velocity_from_hold(accent, hold);
                    if (vel - state.peak_velocity).abs() > 0.02 {
                        app.update_held_note_velocity(pitch, vel);
                    }
                }
            }
        } else if ours {
            // Only release what the computer keyboard started; a mouse-held note on the
            // same pitch must keep sounding.
            app.keyboard_held_notes.remove(&pitch);
            app.release_note(pitch);
        }
    }
}
