use crate::app::DAWApp;
use crate::ui::theme;
use egui::{Color32, Painter, Rect, Ui};

pub const RULER_HEIGHT: f32 = 22.0;
pub const MIN_ZOOM: f32 = 0.12;
pub const MAX_ZOOM: f32 = 6.0;

pub fn beat_width(app: &DAWApp) -> f32 {
    crate::ui::constants::BEAT_WIDTH * app.timeline_zoom
}

pub fn beat_from_x(app: &DAWApp, x: f32, origin_x: f32) -> f64 {
    ((x - origin_x) / beat_width(app)).max(0.0) as f64
}

pub fn x_from_beat(app: &DAWApp, beat: f64, origin_x: f32) -> f32 {
    origin_x + beat as f32 * beat_width(app)
}

pub fn grid_subdivision_beats(zoom: f32) -> f64 {
    if zoom >= 4.0 {
        0.125
    } else if zoom >= 2.0 {
        0.25
    } else if zoom >= 1.0 {
        0.5
    } else {
        1.0
    }
}

pub fn show_zoom_controls(app: &mut DAWApp, ui: &mut Ui, id: &str) {
    ui.push_id(id, |ui| {
        ui.horizontal(|ui| {
            ui.label("Zoom:");
            if ui.button("−").clicked() {
                app.adjust_timeline_zoom(-0.15);
            }
            ui.add(
                egui::Slider::new(&mut app.timeline_zoom, MIN_ZOOM..=MAX_ZOOM).logarithmic(true),
            );
            if ui.button("+").clicked() {
                app.adjust_timeline_zoom(0.15);
            }
            ui.label(format!("{:.0}%", app.timeline_zoom * 100.0));
        });
    });
}

pub fn handle_zoom_scroll(app: &mut DAWApp, ui: &Ui, hovered: bool) {
    if !hovered {
        return;
    }
    let scroll = ui.input(|i| i.smooth_scroll_delta.y + i.raw_scroll_delta.y);
    if scroll.abs() < 0.01 {
        return;
    }
    let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
    if ctrl {
        let factor = if scroll > 0.0 { 1.12 } else { 1.0 / 1.12 };
        app.timeline_zoom = (app.timeline_zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    }
}

pub fn draw_time_ruler(app: &DAWApp, painter: &Painter, rect: Rect, total_beats: f32) {
    painter.rect_filled(rect, 0.0, Color32::from_rgb(28, 28, 34));

    let beats_per_bar = app.project.time_sig_numerator as i32;
    let bw = beat_width(app);
    let show_sub_beats = bw >= 14.0;

    for beat in 0..=(total_beats as i32) {
        let x = rect.min.x + beat as f32 * bw;
        if x > rect.max.x + 1.0 {
            break;
        }
        let is_bar = beat % beats_per_bar == 0;
        if is_bar {
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 90)),
            );
            let bar_num = beat / beats_per_bar + 1;
            painter.text(
                egui::pos2(x + 4.0, rect.min.y + 2.0),
                egui::Align2::LEFT_TOP,
                format!("Bar {bar_num}"),
                theme::font(theme::FONT_SM),
                Color32::from_rgb(210, 210, 230),
            );
        } else if show_sub_beats {
            painter.line_segment(
                [
                    egui::pos2(x, rect.min.y + 10.0),
                    egui::pos2(x, rect.max.y),
                ],
                egui::Stroke::new(0.5_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 35)),
            );
        }

        if show_sub_beats && !is_bar {
            let beat_in_bar = (beat % beats_per_bar) + 1;
            painter.text(
                egui::pos2(x + 2.0, rect.min.y + 11.0),
                egui::Align2::LEFT_TOP,
                beat_in_bar.to_string(),
                theme::font(theme::FONT_XS),
                Color32::from_rgb(130, 130, 150),
            );
        } else if is_bar && bw >= 28.0 {
            for b in 1..beats_per_bar {
                let bx = rect.min.x + (beat + b) as f32 * bw;
                if bx > rect.max.x {
                    break;
                }
                painter.text(
                    egui::pos2(bx + 2.0, rect.min.y + 11.0),
                    egui::Align2::LEFT_TOP,
                    (b + 1).to_string(),
                    theme::font(theme::FONT_XS),
                    Color32::from_rgb(130, 130, 150),
                );
            }
        }
    }
}

pub fn draw_beat_grid_lines(app: &DAWApp, painter: &Painter, rect: Rect, total_beats: f32) {
    let beats_per_bar = app.project.time_sig_numerator as i32;
    let bw = beat_width(app);
    let subdiv = grid_subdivision_beats(app.timeline_zoom);
    let show_subdiv = subdiv < 1.0 && bw * subdiv as f32 >= 6.0;

    if show_subdiv {
        let max_units = (total_beats as f64 / subdiv) as i32 + 2;
        for unit in 0..=max_units {
            let beat = unit as f64 * subdiv;
            if beat > total_beats as f64 + 0.001 {
                break;
            }
            let x = rect.min.x + beat as f32 * bw;
            let is_bar = (beat.round() as i32) % beats_per_bar == 0 && (beat.fract() < 0.001);
            let is_beat = (beat * 4.0).fract() < 0.001;
            let color = if is_bar {
                Color32::from_rgba_unmultiplied(255, 255, 255, 55)
            } else if is_beat {
                Color32::from_rgba_unmultiplied(255, 255, 255, 30)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 12)
            };
            let width = if is_bar { 1.5_f32 } else if is_beat { 0.75_f32 } else { 0.5_f32 };
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(width, color),
            );
        }
    } else {
        for beat in 0..=total_beats as i32 {
            let x = rect.min.x + beat as f32 * bw;
            let is_bar = beat % beats_per_bar == 0;
            let color = if is_bar {
                Color32::from_rgba_unmultiplied(255, 255, 255, 60)
            } else if bw >= 8.0 {
                Color32::from_rgba_unmultiplied(255, 255, 255, 25)
            } else {
                continue;
            };
            let width = if is_bar { 1.5_f32 } else { 0.5_f32 };
            painter.line_segment(
                [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                egui::Stroke::new(width, color),
            );
        }
    }
}

const SECTION_COLORS: [(u8, u8, u8); 6] = [
    (80, 120, 200),
    (120, 180, 100),
    (200, 130, 80),
    (180, 100, 180),
    (200, 180, 80),
    (100, 180, 180),
];

pub fn section_color(index: usize) -> Color32 {
    let (r, g, b) = SECTION_COLORS[index % SECTION_COLORS.len()];
    Color32::from_rgb(r, g, b)
}

pub fn draw_song_sections(
    app: &DAWApp,
    painter: &Painter,
    rect: Rect,
    show_labels: bool,
) {
    let beats_per_bar = app.project.beats_per_bar();
    for (idx, section) in app.project.sections.iter().enumerate() {
        let x0 = x_from_beat(app, section.start_beat, rect.min.x);
        let x1 = x_from_beat(app, section.end_beat(beats_per_bar), rect.min.x);
        let color = section_color(idx);
        painter.rect_filled(
            Rect::from_min_max(egui::pos2(x0, rect.min.y), egui::pos2(x1, rect.max.y)),
            0.0,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 22),
        );
        if show_labels && x1 - x0 > 24.0 {
            let label = format!("{} ({} bars)", section.label, section.bar_count);
            painter.text(
                egui::pos2(x0 + 4.0, rect.min.y + 2.0),
                egui::Align2::LEFT_TOP,
                label,
                theme::font(theme::FONT_SM),
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 200),
            );
        }
        painter.line_segment(
            [egui::pos2(x0, rect.min.y), egui::pos2(x0, rect.max.y)],
            egui::Stroke::new(1.5_f32, color),
        );
    }
}