use crate::app::DAWApp;
use crate::music::theory::{is_black_key, note_name};
use crate::ui::constants::{
    BEAT_WIDTH, KEY_LABEL_WIDTH, PIANO_ROLL_HIGH, PIANO_ROLL_LOW, PIANO_ROLL_PANEL_HEIGHT,
    PIANO_ROW_HEIGHT,
};
use egui::{Color32, Rect, Sense, Ui, Vec2};

pub fn show_piano_roll_detail(app: &mut DAWApp, ui: &mut Ui) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.heading("Piano Roll");
        ui.label(format!(
            "— {}",
            app.project
                .tracks
                .get(app.current_track)
                .map(|t| t.name.as_str())
                .unwrap_or("No track")
        ));
        if app.recording {
            ui.colored_label(Color32::from_rgb(255, 60, 60), "⏺ Recording to this track");
        }
    });

    let total_beats = app.project.total_beats as f32;
    let grid_width = total_beats * BEAT_WIDTH;
    let num_rows = (PIANO_ROLL_HIGH - PIANO_ROLL_LOW + 1) as f32;
    let grid_height = num_rows * PIANO_ROW_HEIGHT;

    ui.horizontal(|ui| {
        draw_key_labels(ui, grid_height.min(PIANO_ROLL_PANEL_HEIGHT - 30.0));
        draw_grid_and_notes(app, ui, grid_width, grid_height, total_beats);
    });
}

fn draw_key_labels(ui: &mut Ui, height: f32) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(KEY_LABEL_WIDTH, height),
        Sense::hover(),
    );
    let painter = ui.painter_at(rect);

    for pitch in (PIANO_ROLL_LOW..=PIANO_ROLL_HIGH).rev() {
        let row = (PIANO_ROLL_HIGH - pitch) as f32;
        let y = rect.min.y + row * PIANO_ROW_HEIGHT;
        if y >= rect.max.y {
            continue;
        }

        let key_rect = Rect::from_min_size(
            egui::pos2(rect.min.x, y),
            Vec2::new(KEY_LABEL_WIDTH, PIANO_ROW_HEIGHT),
        );

        let is_black = is_black_key(pitch);
        let fill = if is_black {
            Color32::from_rgb(40, 40, 45)
        } else {
            Color32::from_rgb(220, 220, 225)
        };

        painter.rect_filled(key_rect, 0.0, fill);
        painter.rect_stroke(key_rect, 0.0, egui::Stroke::new(0.5, Color32::GRAY));

        if !is_black {
            painter.text(
                key_rect.left_center() + Vec2::new(3.0, 0.0),
                egui::Align2::LEFT_CENTER,
                note_name(pitch),
                egui::FontId::proportional(9.0),
                Color32::DARK_GRAY,
            );
        }
    }
}

fn draw_grid_and_notes(
    app: &mut DAWApp,
    ui: &mut Ui,
    grid_width: f32,
    grid_height: f32,
    total_beats: f32,
) {
    let panel_height = PIANO_ROLL_PANEL_HEIGHT - 30.0;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .max_height(panel_height)
        .show(ui, |ui| {
            let (rect, response) = ui.allocate_exact_size(
                Vec2::new(grid_width, grid_height),
                Sense::click(),
            );
            let painter = ui.painter_at(rect);

            for beat in 0..=total_beats as i32 {
                let x = rect.min.x + beat as f32 * BEAT_WIDTH;
                let is_bar = beat % app.project.time_sig_numerator as i32 == 0;
                let color = if is_bar {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 60)
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, 25)
                };
                painter.line_segment(
                    [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                    egui::Stroke::new(if is_bar { 1.5 } else { 0.5 }, color),
                );
            }

            for pitch in (PIANO_ROLL_LOW..=PIANO_ROLL_HIGH).rev() {
                let row = (PIANO_ROLL_HIGH - pitch) as f32;
                let y = rect.min.y + row * PIANO_ROW_HEIGHT;
                let is_black = is_black_key(pitch);
                let row_color = if is_black {
                    Color32::from_rgb(30, 30, 35)
                } else {
                    Color32::from_rgb(45, 45, 50)
                };
                painter.rect_filled(
                    Rect::from_min_size(egui::pos2(rect.min.x, y), Vec2::new(grid_width, PIANO_ROW_HEIGHT)),
                    0.0,
                    row_color,
                );
            }

            if let Some(track) = app.project.tracks.get(app.current_track) {
                let color = Color32::from_rgb(track.color[0], track.color[1], track.color[2]);
                for note in &track.notes {
                    if note.pitch < PIANO_ROLL_LOW || note.pitch > PIANO_ROLL_HIGH {
                        continue;
                    }
                    let row = (PIANO_ROLL_HIGH - note.pitch) as f32;
                    let x = rect.min.x + note.start_beat as f32 * BEAT_WIDTH;
                    let w = (note.duration_beats as f32 * BEAT_WIDTH).max(4.0);
                    let y = rect.min.y + row * PIANO_ROW_HEIGHT + 1.0;
                    let note_rect = Rect::from_min_size(
                        egui::pos2(x, y),
                        Vec2::new(w, PIANO_ROW_HEIGHT - 2.0),
                    );
                    painter.rect_filled(note_rect, 2.0, color);
                    painter.rect_stroke(
                        note_rect,
                        2.0,
                        egui::Stroke::new(1.0, color.gamma_multiply(1.3)),
                    );
                }
            }

            let playhead_x = rect.min.x + app.project.playhead_beat as f32 * BEAT_WIDTH;
            painter.line_segment(
                [egui::pos2(playhead_x, rect.min.y), egui::pos2(playhead_x, rect.max.y)],
                egui::Stroke::new(2.0, Color32::from_rgb(255, 80, 80)),
            );

            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let beat = ((pos.x - rect.min.x) / BEAT_WIDTH).floor().max(0.0) as f64;
                    let row = ((pos.y - rect.min.y) / PIANO_ROW_HEIGHT).floor() as i32;
                    let pitch = PIANO_ROLL_HIGH - row as u8;
                    if pitch >= PIANO_ROLL_LOW && pitch <= PIANO_ROLL_HIGH {
                        app.set_playhead(beat);
                        app.add_note_at(pitch, beat, 1.0);
                    }
                }
            }
        });
}
