use crate::app::DAWApp;
use crate::ui::constants::{BEAT_WIDTH, TRACK_HEADER_WIDTH, TRACK_ROW_HEIGHT, PIANO_ROLL_LOW, PIANO_ROLL_HIGH};
use egui::{Color32, Rect, Sense, Ui, Vec2};

pub fn show_multi_track_timeline(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("Timeline");
    ui.label("Each row is a track — click a row to select it, click the grid to set the playhead.");
    ui.add_space(4.0);

    let total_beats = app.project.total_beats as f32;
    let grid_width = total_beats * BEAT_WIDTH;
    let track_count = app.project.tracks.len();
    let total_height = track_count as f32 * TRACK_ROW_HEIGHT;

    ui.horizontal(|ui| {
        // Fixed track headers
        ui.vertical(|ui| {
            ui.allocate_exact_size(Vec2::new(TRACK_HEADER_WIDTH, 20.0), Sense::hover());
            for i in 0..track_count {
                draw_track_header(app, ui, i);
            }
        });

        // Scrollable timeline grid
        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(grid_width.max(ui.available_width()), total_height),
                    Sense::click(),
                );
                let painter = ui.painter_at(rect);

                draw_beat_grid(app, &painter, rect, total_beats);

                for i in 0..track_count {
                    let row_y = rect.min.y + i as f32 * TRACK_ROW_HEIGHT;
                    let row_rect = Rect::from_min_size(
                        egui::pos2(rect.min.x, row_y),
                        Vec2::new(grid_width, TRACK_ROW_HEIGHT),
                    );

                    let selected = app.current_track == i;
                    let bg = if selected {
                        Color32::from_rgba_unmultiplied(
                            app.project.tracks[i].color[0],
                            app.project.tracks[i].color[1],
                            app.project.tracks[i].color[2],
                            30,
                        )
                    } else if i % 2 == 0 {
                        Color32::from_rgb(35, 35, 40)
                    } else {
                        Color32::from_rgb(40, 40, 46)
                    };
                    painter.rect_filled(row_rect, 0.0, bg);

                    if selected {
                        painter.rect_stroke(
                            row_rect,
                            0.0,
                            egui::Stroke::new(2.0, Color32::from_rgb(
                                app.project.tracks[i].color[0],
                                app.project.tracks[i].color[1],
                                app.project.tracks[i].color[2],
                            )),
                        );
                    }

                    draw_track_overview(app, &painter, row_rect, i);

                    let row_response = ui.interact(row_rect, ui.id().with(("track_row", i)), Sense::click());
                    if row_response.clicked() {
                        if let Some(pos) = row_response.interact_pointer_pos() {
                            app.select_track(i);
                            let beat = ((pos.x - rect.min.x) / BEAT_WIDTH).max(0.0) as f64;
                            app.set_playhead(beat);
                        }
                    }
                }

                let playhead_x = rect.min.x + app.project.playhead_beat as f32 * BEAT_WIDTH;
                painter.line_segment(
                    [
                        egui::pos2(playhead_x, rect.min.y),
                        egui::pos2(playhead_x, rect.max.y),
                    ],
                    egui::Stroke::new(2.0, Color32::from_rgb(255, 80, 80)),
                );
            });
    });
}

fn draw_track_header(app: &mut DAWApp, ui: &mut Ui, index: usize) {
    let selected = app.current_track == index;
    let track_name = app.project.tracks[index].name.clone();
    let track_color = app.project.tracks[index].color;
    let instrument_label = app.project.tracks[index].instrument.label();
    let mut muted = app.project.tracks[index].muted;
    let recording = app.recording && selected;
    let color = Color32::from_rgb(track_color[0], track_color[1], track_color[2]);

    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(TRACK_HEADER_WIDTH, TRACK_ROW_HEIGHT),
        Sense::click(),
    );

    let bg = if selected {
        Color32::from_rgba_unmultiplied(track_color[0], track_color[1], track_color[2], 50)
    } else {
        Color32::from_rgb(32, 32, 38)
    };
    ui.painter_at(rect).rect_filled(rect, 0.0, bg);

    if selected {
        ui.painter_at(rect).rect_stroke(rect, 0.0, egui::Stroke::new(2.0, color));
    }

    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(rect.shrink(6.0)),
        |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(color, "●");
                    if ui.selectable_label(selected, &track_name).clicked() {
                        app.select_track(index);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut muted, "M").changed() {
                        app.project.tracks[index].muted = muted;
                    }
                    if recording {
                        ui.colored_label(Color32::from_rgb(255, 60, 60), "⏺");
                    }
                });
                ui.label(
                    egui::RichText::new(instrument_label)
                        .small()
                        .color(Color32::GRAY),
                );
            });
        },
    );

    if response.clicked() {
        app.select_track(index);
    }
}

fn draw_beat_grid(app: &DAWApp, painter: &egui::Painter, rect: Rect, total_beats: f32) {
    for beat in 0..=total_beats as i32 {
        let x = rect.min.x + beat as f32 * BEAT_WIDTH;
        let is_bar = beat % app.project.time_sig_numerator as i32 == 0;
        let color = if is_bar {
            Color32::from_rgba_unmultiplied(255, 255, 255, 50)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 20)
        };
        painter.line_segment(
            [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
            egui::Stroke::new(if is_bar { 1.5 } else { 0.5 }, color),
        );
    }

    for i in 0..app.project.tracks.len() {
        let y = rect.min.y + i as f32 * TRACK_ROW_HEIGHT;
        painter.line_segment(
            [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
            egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 30)),
        );
    }
}

fn draw_track_overview(app: &DAWApp, painter: &egui::Painter, row_rect: Rect, track_index: usize) {
    let track = &app.project.tracks[track_index];
    let color = Color32::from_rgb(track.color[0], track.color[1], track.color[2]);
    let pitch_range = (PIANO_ROLL_HIGH - PIANO_ROLL_LOW) as f32;
    let inner = row_rect.shrink2(Vec2::new(2.0, 4.0));

    for note in &track.notes {
        let x = inner.min.x + note.start_beat as f32 * BEAT_WIDTH;
        let w = (note.duration_beats as f32 * BEAT_WIDTH).max(3.0);

        let norm = (note.pitch.saturating_sub(PIANO_ROLL_LOW)) as f32 / pitch_range;
        let bar_h = 8.0;
        let y = inner.max.y - norm * (inner.height() - bar_h) - bar_h;

        let note_rect = Rect::from_min_size(egui::pos2(x, y), Vec2::new(w, bar_h));
        painter.rect_filled(note_rect, 2.0, color);
        painter.rect_stroke(
            note_rect,
            2.0,
            egui::Stroke::new(0.5, color.gamma_multiply(1.4)),
        );
    }

    if track.notes.is_empty() {
        painter.text(
            inner.center(),
            egui::Align2::CENTER_CENTER,
            "empty — select track, set playhead, then record",
            egui::FontId::proportional(11.0),
            Color32::from_rgba_unmultiplied(180, 180, 180, 80),
        );
    }
}
