//! Hardware-style DJ controls drawn with egui's painter: rotary knobs, channel faders, a
//! crossfader, LED VU meters and a vinyl platter. All colours are opaque (translucent
//! fills wash out on the light theme).
//!
//! Every control: drag to change (Shift = fine), double-click to reset, scroll wheel to
//! nudge, hover for the exact value.

use crate::model::track_icon::TrackIcon;
use crate::ui::theme;
use egui::{pos2, vec2, Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui};
use std::f32::consts::{PI, TAU};

pub const PANEL_BG: Color32 = Color32::from_rgb(24, 25, 30);
pub const STRIP_BG: Color32 = Color32::from_rgb(33, 34, 40);
pub const STRIP_EDGE: Color32 = Color32::from_rgb(52, 54, 62);
pub const LABEL: Color32 = Color32::from_rgb(150, 154, 166);
pub const AUTOMATED: Color32 = Color32::from_rgb(255, 170, 60);
pub const WRITING: Color32 = Color32::from_rgb(255, 70, 70);

/// How a control relates to automation, for its little status light.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutoState {
    #[default]
    None,
    /// Follows a curve on the timeline.
    Automated,
    /// Your moves are being recorded right now.
    Writing,
}

impl AutoState {
    fn color(self) -> Option<Color32> {
        match self {
            Self::None => None,
            Self::Automated => Some(AUTOMATED),
            Self::Writing => Some(WRITING),
        }
    }
}

/// Knob sweep: 270° from bottom-left (7 o'clock) to bottom-right (5 o'clock).
const SWEEP: f32 = 1.5 * PI;
const START: f32 = 0.75 * PI;

fn angle_for(t: f32) -> f32 {
    START + SWEEP * t.clamp(0.0, 1.0)
}

fn polar(c: Pos2, r: f32, a: f32) -> Pos2 {
    pos2(c.x + r * a.cos(), c.y + r * a.sin())
}

fn arc(painter: &Painter, c: Pos2, r: f32, a0: f32, a1: f32, stroke: Stroke) {
    let (a0, a1) = if a0 <= a1 { (a0, a1) } else { (a1, a0) };
    let n = (((a1 - a0) / 0.08).ceil() as usize).max(2);
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| polar(c, r, a0 + (a1 - a0) * i as f32 / n as f32))
        .collect();
    painter.add(Shape::line(pts, stroke));
}

/// Mix two opaque colours.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

/// Shared drag / reset / scroll handling. Returns true if the value changed.
fn drag_value(
    ui: &Ui,
    resp: &Response,
    value: &mut f32,
    (lo, hi): (f32, f32),
    neutral: f32,
    pixels_for_full_range: f32,
    axis: Pos2Axis,
) -> bool {
    let before = *value;
    let fine = ui.input(|i| i.modifiers.shift);
    if resp.double_clicked() {
        *value = neutral;
    } else if resp.dragged() {
        let d = resp.drag_delta();
        let px = match axis {
            Pos2Axis::Knob => d.x - d.y,
            Pos2Axis::Vertical => -d.y,
            Pos2Axis::Horizontal => d.x,
        };
        let scale = if fine { 0.2 } else { 1.0 };
        *value += px / pixels_for_full_range * (hi - lo) * scale;
    }
    if resp.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            *value += scroll.signum() * (hi - lo) * if fine { 0.005 } else { 0.03 };
        }
    }
    // Centre detent: snap to neutral when passing close to it.
    if (*value - neutral).abs() < (hi - lo) * 0.012 && resp.dragged() {
        *value = neutral;
    }
    *value = value.clamp(lo, hi);
    (*value - before).abs() > f32::EPSILON
}

#[derive(Clone, Copy)]
enum Pos2Axis {
    Knob,
    Vertical,
    Horizontal,
}

/// A rotary knob. `bipolar` knobs (EQ, filter) light their ring from the top centre.
#[allow(clippy::too_many_arguments)]
pub fn knob(
    ui: &mut Ui,
    value: &mut f32,
    range: (f32, f32),
    neutral: f32,
    diameter: f32,
    label: &str,
    accent: Color32,
    auto: AutoState,
    hover: impl FnOnce(f32) -> String,
) -> Response {
    let label_h = if label.is_empty() { 0.0 } else { 12.0 };
    let (rect, mut resp) = ui.allocate_exact_size(vec2(diameter + 6.0, diameter + label_h), Sense::click_and_drag());
    if drag_value(ui, &resp, value, range, neutral, 180.0, Pos2Axis::Knob) {
        resp.mark_changed();
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }

    let painter = ui.painter_at(rect.expand(2.0));
    let c = pos2(rect.center().x, rect.min.y + diameter * 0.5);
    let r = diameter * 0.5;
    let t = (*value - range.0) / (range.1 - range.0);
    let bipolar = range.0 < 0.0;
    let hot = resp.hovered() || resp.dragged();

    // Value ring (behind the cap).
    arc(&painter, c, r - 1.5, START, START + SWEEP, Stroke::new(3.0, Color32::from_rgb(18, 18, 22)));
    let ring = auto.color().unwrap_or(accent);
    let from = if bipolar { angle_for(0.5) } else { START };
    if (angle_for(t) - from).abs() > 0.01 {
        arc(&painter, c, r - 1.5, from, angle_for(t), Stroke::new(3.0, ring));
    }
    // Scale ticks just outside the ring.
    for i in 0..=10 {
        let a = angle_for(i as f32 / 10.0);
        let long = i == 0 || i == 10 || (bipolar && i == 5);
        painter.line_segment(
            [polar(c, r + 0.5, a), polar(c, r + if long { 3.5 } else { 2.0 }, a)],
            Stroke::new(1.0, Color32::from_rgb(90, 92, 102)),
        );
    }
    // Cap: dark skirt, knurled edge, lighter top.
    let cap_r = r - 5.0;
    painter.circle_filled(c + vec2(0.0, 1.5), cap_r + 0.5, Color32::from_rgb(10, 10, 12));
    painter.circle_filled(c, cap_r, Color32::from_rgb(46, 47, 54));
    let grips = 18;
    for i in 0..grips {
        let a = i as f32 / grips as f32 * TAU;
        painter.line_segment(
            [polar(c, cap_r - 2.5, a), polar(c, cap_r, a)],
            Stroke::new(1.0, Color32::from_rgb(30, 30, 36)),
        );
    }
    let top = if hot { Color32::from_rgb(78, 80, 90) } else { Color32::from_rgb(64, 66, 75) };
    painter.circle_filled(c, cap_r - 3.0, top);
    painter.circle_stroke(c, cap_r - 3.0, Stroke::new(1.0, Color32::from_rgb(92, 94, 104)));
    // Pointer.
    let a = angle_for(t);
    painter.line_segment(
        [polar(c, cap_r * 0.25, a), polar(c, cap_r - 3.5, a)],
        Stroke::new(2.2, Color32::from_rgb(240, 240, 245)),
    );
    // Automation light.
    if let Some(col) = auto.color() {
        painter.circle_filled(pos2(rect.max.x - 3.0, rect.min.y + 3.0), 2.6, col);
    }
    if !label.is_empty() {
        painter.text(
            pos2(c.x, rect.max.y - 5.5),
            egui::Align2::CENTER_CENTER,
            label,
            theme::font(theme::FONT_XS),
            if hot { Color32::WHITE } else { LABEL },
        );
    }
    let v = *value;
    resp.on_hover_text(format!(
        "{}\nDrag to turn · Shift = fine · double-click = reset{}",
        hover(v),
        match auto {
            AutoState::None => "",
            AutoState::Automated => "\nAutomated: follows the curve on the timeline",
            AutoState::Writing => "\nRecording your moves",
        }
    ))
}

/// A vertical channel fader (value 0..`max`).
pub fn fader(
    ui: &mut Ui,
    value: &mut f32,
    max: f32,
    neutral: f32,
    size: egui::Vec2,
    accent: Color32,
    auto: AutoState,
) -> Response {
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let travel = rect.height() - 18.0;
    if drag_value(ui, &resp, value, (0.0, max), neutral, travel, Pos2Axis::Vertical) {
        resp.mark_changed();
    }
    let painter = ui.painter_at(rect);
    let cx = rect.center().x;
    let y_for = |v: f32| rect.max.y - 9.0 - travel * (v / max).clamp(0.0, 1.0);
    // Scale.
    for i in 0..=10 {
        let y = y_for(max * i as f32 / 10.0);
        let w = if i % 5 == 0 { 7.0 } else { 4.0 };
        painter.line_segment(
            [pos2(rect.min.x + 2.0, y), pos2(rect.min.x + 2.0 + w, y)],
            Stroke::new(1.0, Color32::from_rgb(80, 82, 92)),
        );
    }
    // Slot.
    let slot = Rect::from_center_size(pos2(cx, rect.center().y), vec2(4.0, travel + 4.0));
    painter.rect_filled(slot, 2.0, Color32::from_rgb(10, 10, 12));
    let lit = Rect::from_min_max(pos2(slot.min.x, y_for(*value)), slot.max);
    painter.rect_filled(lit, 2.0, mix(Color32::from_rgb(10, 10, 12), auto.color().unwrap_or(accent), 0.55));
    // Cap.
    let y = y_for(*value);
    let cap = Rect::from_center_size(pos2(cx, y), vec2((rect.width() - 6.0).min(30.0), 16.0));
    let hot = resp.hovered() || resp.dragged();
    painter.rect_filled(cap.translate(vec2(0.0, 1.5)), 2.0, Color32::from_rgb(8, 8, 10));
    painter.rect_filled(cap, 2.0, if hot { Color32::from_rgb(96, 98, 108) } else { Color32::from_rgb(78, 80, 90) });
    for dy in [-4.0, -2.0, 2.0, 4.0] {
        painter.line_segment(
            [pos2(cap.min.x + 3.0, y + dy), pos2(cap.max.x - 3.0, y + dy)],
            Stroke::new(1.0, Color32::from_rgb(58, 60, 68)),
        );
    }
    painter.line_segment(
        [pos2(cap.min.x + 2.0, y), pos2(cap.max.x - 2.0, y)],
        Stroke::new(1.5, Color32::from_rgb(235, 235, 240)),
    );
    if let Some(col) = auto.color() {
        painter.circle_filled(pos2(rect.max.x - 3.0, rect.min.y + 3.0), 2.6, col);
    }
    let v = *value;
    resp.on_hover_text(format!(
        "Level {:.0}%\nDrag · Shift = fine · double-click = reset",
        v / max * 100.0
    ))
}

/// The crossfader (-1 = A … +1 = B).
pub fn crossfader(ui: &mut Ui, value: &mut f32, width: f32, auto: AutoState) -> Response {
    let (rect, mut resp) = ui.allocate_exact_size(vec2(width, 34.0), Sense::click_and_drag());
    let travel = rect.width() - 40.0;
    if drag_value(ui, &resp, value, (-1.0, 1.0), 0.0, travel, Pos2Axis::Horizontal) {
        resp.mark_changed();
    }
    let painter = ui.painter_at(rect);
    let cy = rect.center().y;
    let x_for = |v: f32| rect.min.x + 20.0 + travel * ((v + 1.0) * 0.5);
    for (txt, x) in [("A", rect.min.x + 7.0), ("B", rect.max.x - 7.0)] {
        painter.text(pos2(x, cy), egui::Align2::CENTER_CENTER, txt, theme::font(theme::FONT_LG), LABEL);
    }
    for i in 0..=8 {
        let x = x_for(-1.0 + i as f32 / 4.0);
        let h = if i == 4 { 8.0 } else { 4.0 };
        painter.line_segment([pos2(x, rect.min.y), pos2(x, rect.min.y + h)], Stroke::new(1.0, Color32::from_rgb(80, 82, 92)));
        painter.line_segment([pos2(x, rect.max.y - h), pos2(x, rect.max.y)], Stroke::new(1.0, Color32::from_rgb(80, 82, 92)));
    }
    let slot = Rect::from_center_size(pos2(rect.center().x, cy), vec2(travel + 4.0, 4.0));
    painter.rect_filled(slot, 2.0, Color32::from_rgb(10, 10, 12));
    let x = x_for(*value);
    let cap = Rect::from_center_size(pos2(x, cy), vec2(18.0, 26.0));
    let hot = resp.hovered() || resp.dragged();
    painter.rect_filled(cap.translate(vec2(0.0, 1.5)), 2.0, Color32::from_rgb(8, 8, 10));
    painter.rect_filled(cap, 2.0, if hot { Color32::from_rgb(96, 98, 108) } else { Color32::from_rgb(78, 80, 90) });
    for dx in [-5.0, -3.0, 3.0, 5.0] {
        painter.line_segment([pos2(x + dx, cap.min.y + 3.0), pos2(x + dx, cap.max.y - 3.0)], Stroke::new(1.0, Color32::from_rgb(58, 60, 68)));
    }
    painter.line_segment(
        [pos2(x, cap.min.y + 2.0), pos2(x, cap.max.y - 2.0)],
        Stroke::new(1.5, auto.color().unwrap_or(Color32::from_rgb(235, 235, 240))),
    );
    let v = *value;
    resp.on_hover_text(format!(
        "Crossfader: {}\nLeft plays channels on A, right plays B, centre plays both.\nDouble-click = centre",
        if v.abs() < 0.02 { "centre".to_string() } else if v < 0.0 { format!("{:.0}% toward A", -v * 100.0) } else { format!("{:.0}% toward B", v * 100.0) }
    ))
}

/// LED-segment level meter. `level` is a linear peak (1.0 = full scale).
pub fn vu_meter(ui: &mut Ui, level: f32, size: egui::Vec2) -> Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    paint_vu(ui.painter(), rect, level);
    let db = 20.0 * level.max(1e-5).log10();
    resp.on_hover_text(if level < 1e-4 { "silent".to_string() } else { format!("{db:.1} dB") })
}

/// Meter segments light from -48 dB to +3 dB; red above -3 dB.
pub fn paint_vu(painter: &Painter, rect: Rect, level: f32) {
    let segments = ((rect.height() / 5.0) as usize).clamp(6, 24);
    let db = 20.0 * level.max(1e-6).log10();
    let lit = ((db + 48.0) / 51.0).clamp(0.0, 1.0);
    let seg_h = rect.height() / segments as f32;
    painter.rect_filled(rect, 1.0, Color32::from_rgb(12, 12, 14));
    for i in 0..segments {
        let frac = (i as f32 + 0.5) / segments as f32;
        let seg_db = -48.0 + frac * 51.0;
        let on = frac <= lit;
        let base = if seg_db > -3.0 {
            Color32::from_rgb(255, 60, 50)
        } else if seg_db > -12.0 {
            Color32::from_rgb(250, 200, 50)
        } else {
            Color32::from_rgb(70, 220, 110)
        };
        let col = if on { base } else { mix(Color32::from_rgb(12, 12, 14), base, 0.16) };
        let y1 = rect.max.y - i as f32 * seg_h - 1.0;
        let r = Rect::from_min_max(pos2(rect.min.x + 1.0, y1 - seg_h + 1.5), pos2(rect.max.x - 1.0, y1));
        painter.rect_filled(r, 0.5, col);
    }
}

/// Result of a frame of platter interaction.
pub struct PlatterInput {
    /// A hand is on the platter (drag in progress).
    pub held: bool,
    /// How far it was turned this frame, in revolutions (+ = clockwise).
    pub turns: f32,
}

/// A turntable platter with a spinning record. `rotation` is the record's angle in
/// revolutions; `label` / `color` describe what's on it.
pub fn platter(ui: &mut Ui, diameter: f32, rotation: f64, label: &str, color: Color32) -> (Response, PlatterInput) {
    let (rect, resp) = ui.allocate_exact_size(vec2(diameter, diameter), Sense::click_and_drag());
    let c = rect.center();
    let r = diameter * 0.5;
    let mut input = PlatterInput { held: resp.is_pointer_button_down_on() || resp.dragged(), turns: 0.0 };
    if resp.dragged() {
        if let Some(p) = resp.interact_pointer_pos() {
            let prev = p - resp.drag_delta();
            let a0 = (prev - c).angle();
            let a1 = (p - c).angle();
            let mut d = a1 - a0;
            if d > PI {
                d -= TAU;
            } else if d < -PI {
                d += TAU;
            }
            // Very close to the spindle the angle is meaningless.
            if (p - c).length() > r * 0.12 {
                input.turns = d / TAU;
            }
        }
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(if resp.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
    }

    let painter = ui.painter_at(rect.expand(1.0));
    // Plinth shadow + metal platter rim with strobe dots.
    painter.circle_filled(c + vec2(0.0, 2.0), r, Color32::from_rgb(8, 8, 10));
    painter.circle_filled(c, r, Color32::from_rgb(118, 120, 128));
    painter.circle_filled(c, r - 2.0, Color32::from_rgb(92, 94, 102));
    let angle = (rotation.fract() as f32) * TAU;
    let dots = 48;
    for i in 0..dots {
        let a = angle + i as f32 / dots as f32 * TAU;
        let col = if i % 2 == 0 { Color32::from_rgb(200, 202, 210) } else { Color32::from_rgb(70, 72, 80) };
        painter.circle_filled(polar(c, r - 4.5, a), (r * 0.025).max(1.0), col);
    }
    // Record.
    let rec_r = r - 9.0;
    painter.circle_filled(c, rec_r, Color32::from_rgb(16, 16, 18));
    let grooves = ((rec_r - rec_r * 0.38) / 3.0) as usize;
    for g in 0..grooves {
        let gr = rec_r * 0.38 + g as f32 * 3.0 + 1.5;
        let shade = if g % 3 == 0 { 30 } else { 24 };
        painter.circle_stroke(c, gr, Stroke::new(1.0, Color32::from_rgb(shade, shade, shade + 4)));
    }
    // Light sheen that turns with the record so motion is obvious.
    arc(&painter, c, rec_r * 0.72, angle - 0.5, angle + 0.5, Stroke::new(rec_r * 0.4, Color32::from_rgb(34, 34, 40)));
    // Centre label in the track's colour, with a cue mark that turns with it.
    let lab_r = rec_r * 0.36;
    painter.circle_filled(c, lab_r, color);
    painter.circle_stroke(c, lab_r, Stroke::new(1.0, mix(color, Color32::BLACK, 0.4)));
    let tip = polar(c, rec_r - 2.0, angle - PI * 0.5);
    painter.line_segment([polar(c, lab_r, angle - PI * 0.5), tip], Stroke::new(2.0, Color32::from_rgb(240, 240, 245)));
    painter.circle_filled(c, (r * 0.035).max(2.0), Color32::from_rgb(200, 202, 210));
    if !label.is_empty() {
        let text_col = if color.r() as u32 + color.g() as u32 + color.b() as u32 > 380 {
            Color32::from_rgb(20, 20, 24)
        } else {
            Color32::WHITE
        };
        let short: String = label.chars().take(12).collect();
        painter.text(c + vec2(0.0, lab_r * 0.45), egui::Align2::CENTER_CENTER, short, theme::font(theme::FONT_SM), text_col);
    }
    (resp, input)
}

/// Draw a track icon centred in `rect`.
pub fn paint_track_icon(painter: &Painter, rect: Rect, icon: TrackIcon, color: Color32) {
    if icon == TrackIcon::Drums {
        // Phosphor has no drum; use the app's own vector drum.
        crate::ui::drum_icons::paint_drum_icon(painter, rect.shrink(rect.width() * 0.08), crate::model::DrumKind::Snare, color);
        return;
    }
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon.glyph(),
        egui::FontId::proportional(rect.height() * 0.86),
        color,
    );
}

/// A clickable icon square; returns the response (open a picker on click).
pub fn icon_button(ui: &mut Ui, icon: TrackIcon, color: Color32, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let painter = ui.painter_at(rect);
    let bg = if resp.hovered() { Color32::from_rgb(64, 66, 76) } else { Color32::from_rgb(46, 47, 55) };
    painter.rect_filled(rect, 4.0, bg);
    paint_track_icon(&painter, rect.shrink(3.0), icon, color);
    resp
}

/// Grid of every icon; returns the one clicked.
pub fn icon_grid(ui: &mut Ui, current: TrackIcon, color: Color32) -> Option<TrackIcon> {
    let mut picked = None;
    ui.label(egui::RichText::new("Track icon").strong());
    egui::Grid::new("icon_grid").spacing(vec2(3.0, 3.0)).show(ui, |ui| {
        for (i, &icon) in TrackIcon::all().iter().enumerate() {
            let (rect, resp) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
            let p = ui.painter_at(rect);
            let selected = icon == current;
            let bg = if selected {
                mix(Color32::from_rgb(40, 40, 48), color, 0.45)
            } else if resp.hovered() {
                Color32::from_rgb(64, 66, 76)
            } else {
                Color32::from_rgb(40, 40, 48)
            };
            p.rect_filled(rect, 4.0, bg);
            paint_track_icon(&p, rect.shrink(4.0), icon, if selected { Color32::WHITE } else { color });
            if resp.on_hover_text(icon.label()).clicked() {
                picked = Some(icon);
            }
            if i % 8 == 7 {
                ui.end_row();
            }
        }
    });
    ui.label(
        egui::RichText::new("Icons: Phosphor (MIT) · custom hand-drawn icons are planned")
            .small()
            .color(LABEL),
    );
    picked
}
