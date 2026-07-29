use crate::app::{DAWApp, NoteDragMode, NoteDragState};
use crate::model::drum::DrumKind;
use crate::music::theory::{is_black_key, pitch_to_name};
use crate::ui::constants::{
    DRUM_KEY_LABEL_WIDTH, DRUM_ROW_HEIGHT, KEY_LABEL_WIDTH, PIANO_ROLL_PANEL_HEIGHT,
    PIANO_ROW_HEIGHT,
};
use crate::ui::grid::{
    self, beat_from_x, beat_width, draw_beat_grid_lines, draw_song_sections, draw_time_ruler,
    handle_zoom_scroll, show_zoom_controls, x_from_beat, RULER_HEIGHT,
};
use crate::ui::theme;
use egui::{Color32, CursorIcon, Rect, Sense, Ui, Vec2};

const PLAYHEAD_HIT_WIDTH: f32 = 14.0;
const NOTE_RESIZE_HANDLE: f32 = 8.0;
const MIN_NOTE_BEATS: f64 = 0.25;

pub fn show_piano_roll_detail(app: &mut DAWApp, ui: &mut Ui) {
    let is_drum = app
        .project
        .tracks
        .get(app.current_track)
        .map(|t| t.is_drum())
        .unwrap_or(false);

    ui.separator();
    ui.horizontal(|ui| {
        ui.heading(if is_drum { "Drum Roll" } else { "Piano Roll" });
        ui.label(format!(
            "— {}",
            app.project
                .tracks
                .get(app.current_track)
                .map(|t| t.name.as_str())
                .unwrap_or("No track")
        ));
        if app.recording {
            ui.colored_label(Color32::from_rgb(255, 60, 60), "⏺ Recording");
        }
    });

    show_zoom_controls(app, ui, "piano_roll_zoom");
    ui.label(
        egui::RichText::new(
            "Scroll on piano roll to shift pitch · Ctrl/⌘ + scroll to zoom · beats 1–4 in ruler",
        )
        .small()
        .color(Color32::GRAY),
    );

    if is_drum {
        show_drum_roll(app, ui);
    } else {
        ui.horizontal(|ui| {
            if app.selected_note.is_some() && ui.button("Delete note").clicked() {
                app.delete_selected_note();
            }
            if app.section_range.is_some() && ui.button("Clear note selection").clicked() {
                app.section_range = None;
            }
        });
        if let Some((s, e)) = app.section_range {
            ui.label(
                egui::RichText::new(format!("Note range selected: beat {:.2} – {:.2}", s, e))
                    .small()
                    .color(Color32::from_rgb(180, 220, 255)),
            );
        }
        show_melodic_roll(app, ui);
    }

    handle_note_drag(app, ui, is_drum);
    if !is_drum {
        handle_section_drag(app, ui);
        handle_delete_key(app, ui);
    }
}

fn piano_roll_panel_height() -> f32 {
    PIANO_ROLL_PANEL_HEIGHT - 30.0 - RULER_HEIGHT
}

fn visible_pitch_rows(height: f32) -> u8 {
    (height / PIANO_ROW_HEIGHT).floor().max(1.0) as u8
}

fn handle_pitch_scroll(app: &mut DAWApp, ui: &Ui, hovered: bool) {
    if !hovered {
        return;
    }
    let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
    if ctrl {
        return;
    }
    let scroll = ui.input(|i| i.smooth_scroll_delta.y + i.raw_scroll_delta.y);
    if scroll.abs() < 0.01 {
        return;
    }
    // Scroll up → higher pitches; scroll down → lower pitches.
    let semitones = -(scroll / 15.0).round() as i32;
    app.adjust_piano_roll_pitch(semitones);
}

fn show_melodic_roll(app: &mut DAWApp, ui: &mut Ui) {
    let total_beats = app.project.total_beats as f32;
    let bw = beat_width(app);
    let grid_width = total_beats * bw;
    let roll_low = app.piano_roll_low();
    let roll_high = app.piano_roll_high();
    let panel_height = piano_roll_panel_height();
    let visible_rows = visible_pitch_rows(panel_height);
    let grid_height = visible_rows as f32 * PIANO_ROW_HEIGHT;

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.allocate_exact_size(Vec2::new(KEY_LABEL_WIDTH, RULER_HEIGHT), Sense::hover());
            draw_melodic_key_labels(app, ui, panel_height, roll_low, roll_high);
        });
        ui.vertical(|ui| {
            draw_grid_ruler(app, ui, grid_width, total_beats);
            draw_melodic_grid(
                app,
                ui,
                grid_width,
                grid_height,
                panel_height,
                total_beats,
                roll_low,
                roll_high,
                visible_rows,
            );
        });
    });
}

fn show_drum_roll(app: &mut DAWApp, ui: &mut Ui) {
    let total_beats = app.project.total_beats as f32;
    let bw = beat_width(app);
    let grid_width = total_beats * bw;
    let drums = DrumKind::all();
    let grid_height = drums.len() as f32 * DRUM_ROW_HEIGHT;
    let key_height = grid_height.min(PIANO_ROLL_PANEL_HEIGHT - 30.0 - RULER_HEIGHT);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.allocate_exact_size(Vec2::new(DRUM_KEY_LABEL_WIDTH, RULER_HEIGHT), Sense::hover());
            draw_drum_key_labels(app, ui, key_height, drums);
        });
        ui.vertical(|ui| {
            draw_grid_ruler(app, ui, grid_width, total_beats);
            draw_drum_grid(app, ui, grid_width, grid_height, total_beats, drums);
        });
    });
}

fn draw_grid_ruler(app: &DAWApp, ui: &mut Ui, grid_width: f32, total_beats: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(grid_width, RULER_HEIGHT), Sense::hover());
    let painter = ui.painter_at(rect);
    draw_time_ruler(app, &painter, rect, total_beats);
}

fn handle_delete_key(app: &mut DAWApp, ui: &mut Ui) {
    if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
        app.delete_selected_note();
    }
}

fn handle_note_drag(app: &mut DAWApp, ui: &mut Ui, is_drum: bool) {
    let Some(drag) = app.note_drag else {
        return;
    };
    let track_idx = app.current_track;
    let Some(pos) = ui.input(|i| i.pointer.latest_pos()) else {
        return;
    };

    if !ui.input(|i| i.pointer.primary_down()) {
        app.note_drag = None;
        return;
    }

    let Some(rect) = app.piano_roll_grid_rect else {
        return;
    };

    let beat = snap_beat(
        app,
        beat_from_x(app, pos.x, rect.min.x),
        ui.input(|i| i.modifiers.shift),
    );
    let row = ((pos.y - rect.min.y) / row_height(is_drum)).floor() as i32;

    let pitch = if is_drum {
        DrumKind::all()
            .get(row as usize)
            .map(|d| d.midi_pitch())
            .unwrap_or(36)
    } else {
        (app.piano_roll_high() as i32 - row)
            .clamp(app.piano_roll_low() as i32, app.piano_roll_high() as i32) as u8
    };

    let clip_start = app
        .project
        .tracks
        .get(track_idx)
        .and_then(|t| t.clips.get(drag.clip_index))
        .map(|c| c.start_beat)
        .unwrap_or(0.0);

    if let Some(track) = app.project.tracks.get_mut(track_idx) {
        if let Some(note) = track
            .clips
            .get_mut(drag.clip_index)
            .and_then(|c| c.notes.get_mut(drag.note_index))
        {
            match drag.mode {
                NoteDragMode::Move => {
                    let rel = (beat - drag.grab_beat - clip_start).max(0.0);
                    note.start_beat = rel;
                    note.pitch = pitch;
                }
                NoteDragMode::Resize if !is_drum => {
                    note.duration_beats =
                        (beat - clip_start - note.start_beat).max(MIN_NOTE_BEATS);
                }
                NoteDragMode::Resize => {}
            }
        }
        if let Some(clip) = track.clips.get_mut(drag.clip_index) {
            clip.recompute_bounds();
        }
    }
}

fn handle_section_drag(app: &mut DAWApp, ui: &mut Ui) {
    let Some((_origin_beat, start_pointer_beat)) = app.section_drag else {
        return;
    };
    if !ui.input(|i| i.pointer.primary_down()) {
        app.section_drag = None;
        app.section_note_indices = None;
        return;
    }
    let Some(rect) = app.piano_roll_grid_rect else {
        return;
    };
    let Some(pos) = ui.input(|i| i.pointer.latest_pos()) else {
        return;
    };
    let beat = beat_from_x(app, pos.x, rect.min.x);
    let delta = beat - start_pointer_beat;

    if let Some(clip_idx) = app.section_clip_index {
        if let Some(indices) = app.section_note_indices.clone() {
            if let Some(track) = app.project.tracks.get_mut(app.current_track) {
                if let Some(clip) = track.clips.get_mut(clip_idx) {
                    for &idx in &indices {
                        if let Some(note) = clip.notes.get_mut(idx) {
                            note.start_beat = app
                                .section_drag_origins
                                .get(&idx)
                                .map(|&b| (b + delta).max(0.0))
                                .unwrap_or(note.start_beat);
                        }
                    }
                    clip.recompute_bounds();
                }
            }
        }
    }
}

fn row_height(is_drum: bool) -> f32 {
    if is_drum {
        DRUM_ROW_HEIGHT
    } else {
        PIANO_ROW_HEIGHT
    }
}

fn snap_beat(app: &DAWApp, beat: f64, fine: bool) -> f64 {
    if fine {
        beat
    } else {
        let subdiv = grid::grid_subdivision_beats(app.timeline_zoom);
        (beat / subdiv).round() * subdiv
    }
}

fn drum_row_index(pitch: u8) -> Option<usize> {
    DrumKind::from_midi_pitch(pitch)
        .and_then(|d| DrumKind::all().iter().position(|&k| k == d))
}

fn draw_melodic_key_labels(app: &mut DAWApp, ui: &mut Ui, height: f32, roll_low: u8, roll_high: u8) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(KEY_LABEL_WIDTH, height), Sense::hover());
    handle_pitch_scroll(app, ui, response.hovered());
    let painter = ui.painter_at(rect);
    let rows = visible_pitch_rows(height);

    for row in 0..rows {
        let pitch = roll_high.saturating_sub(row);
        if pitch < roll_low {
            break;
        }

        let y = rect.min.y + row as f32 * PIANO_ROW_HEIGHT;
        let key_rect = Rect::from_min_size(
            egui::pos2(rect.min.x, y),
            Vec2::new(KEY_LABEL_WIDTH, PIANO_ROW_HEIGHT),
        );

        let is_black = is_black_key(pitch);
        let pressed = app.held_notes.contains(&pitch);
        let fill = if pressed {
            Color32::from_rgb(140, 170, 255)
        } else if is_black {
            Color32::from_rgb(28, 28, 34)
        } else {
            Color32::from_rgb(228, 228, 235)
        };

        painter.rect_filled(key_rect, 1.0, fill);
        painter.rect_stroke(key_rect, 1.0, egui::Stroke::new(0.5_f32, Color32::from_gray(120)));

        let label = pitch_to_name(pitch);
        let text_color = if is_black {
            Color32::from_rgb(240, 240, 250)
        } else {
            Color32::from_rgb(35, 35, 45)
        };
        painter.text(
            key_rect.left_center() + Vec2::new(3.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            theme::font(theme::FONT_SM),
            text_color,
        );

        let response = ui.interact(key_rect, ui.id().with(("melodic_key", pitch)), Sense::click());
        if response.is_pointer_button_down_on() {
            if !app.held_notes.contains(&pitch) {
                let vel = if ui.input(|i| i.modifiers.shift) {
                    1.0
                } else if ui.input(|i| i.modifiers.alt) {
                    0.4
                } else {
                    0.75
                };
                app.play_note_with_velocity(pitch, vel);
            }
        } else if app.held_notes.contains(&pitch) {
            app.release_note(pitch);
        }
    }
}

fn draw_drum_key_labels(app: &mut DAWApp, ui: &mut Ui, height: f32, drums: &[DrumKind]) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(DRUM_KEY_LABEL_WIDTH, height), Sense::hover());
    let painter = ui.painter_at(rect);

    for (row, &drum) in drums.iter().enumerate() {
        let y = rect.min.y + row as f32 * DRUM_ROW_HEIGHT;
        let key_rect = Rect::from_min_size(
            egui::pos2(rect.min.x, y),
            Vec2::new(DRUM_KEY_LABEL_WIDTH, DRUM_ROW_HEIGHT - 1.0),
        );

        let pitch = drum.midi_pitch();
        let pressed = app.held_notes.contains(&pitch);
        let fill = if pressed {
            Color32::from_rgb(255, 150, 100)
        } else {
            Color32::from_rgb(50, 45, 55)
        };

        painter.rect_filled(key_rect, 3.0, fill);
        painter.rect_stroke(key_rect, 3.0, egui::Stroke::new(1.0_f32, Color32::from_gray(100)));
        painter.text(
            key_rect.left_center() + Vec2::new(4.0, 0.0),
            egui::Align2::LEFT_CENTER,
            format!("{} {}", drum.icon(), drum.label()),
            theme::font(theme::FONT_LG),
            Color32::WHITE,
        );

        let response = ui.interact(key_rect, ui.id().with(("drum_key", drum)), Sense::click());
        if response.is_pointer_button_down_on() {
            if !app.held_notes.contains(&pitch) {
                app.held_notes.insert(pitch);
                app.preview_drum(drum);
                if app.recording {
                    let beat = app.project.playhead_beat;
                    let accent = ui.input(|i| i.modifiers.shift);
                    app.toggle_drum_hit_at_beat(app.current_track, drum, beat, accent);
                }
            }
        } else if app.held_notes.contains(&pitch) {
            app.held_notes.remove(&pitch);
        }
    }
}

fn draw_melodic_grid(
    app: &mut DAWApp,
    ui: &mut Ui,
    grid_width: f32,
    grid_height: f32,
    panel_height: f32,
    total_beats: f32,
    roll_low: u8,
    roll_high: u8,
    visible_rows: u8,
) {
    let section_range = app.section_range;
    let selected = app.selected_note;
    let note_drag = app.note_drag;

    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .max_height(panel_height)
        .show(ui, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(grid_width, grid_height), Sense::click());
            app.piano_roll_grid_rect = Some(rect);
            handle_pitch_scroll(app, ui, response.hovered());
            handle_zoom_scroll(app, ui, response.hovered());

            let painter = ui.painter_at(rect);

            draw_song_sections(app, &painter, rect, false);
            draw_beat_grid_lines(app, &painter, rect, total_beats);
            draw_melodic_pitch_rows(&painter, rect, grid_width, roll_low, roll_high, visible_rows);

            draw_section_overlay(app, ui, &painter, rect, section_range);
            draw_melodic_notes(
                app,
                ui,
                &painter,
                rect,
                selected,
                note_drag,
                section_range,
                roll_low,
                roll_high,
                visible_rows,
            );
            draw_playhead(app, ui, &painter, rect, &response, false);
        });
}

fn draw_drum_grid(
    app: &mut DAWApp,
    ui: &mut Ui,
    grid_width: f32,
    grid_height: f32,
    total_beats: f32,
    drums: &[DrumKind],
) {
    let panel_height = PIANO_ROLL_PANEL_HEIGHT - 30.0 - RULER_HEIGHT;
    let selected = app.selected_note;
    let note_drag = app.note_drag;
    let track_idx = app.current_track;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .max_height(panel_height)
        .show(ui, |ui| {
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(grid_width, grid_height), Sense::click());
            app.piano_roll_grid_rect = Some(rect);
            handle_zoom_scroll(app, ui, response.hovered());

            let painter = ui.painter_at(rect);

            draw_song_sections(app, &painter, rect, false);
            draw_beat_grid_lines(app, &painter, rect, total_beats);

            for (row, _drum) in drums.iter().enumerate() {
                let y = rect.min.y + row as f32 * DRUM_ROW_HEIGHT;
                let row_color = if row % 2 == 0 {
                    Color32::from_rgb(38, 36, 44)
                } else {
                    Color32::from_rgb(32, 30, 38)
                };
                painter.rect_filled(
                    Rect::from_min_size(
                        egui::pos2(rect.min.x, y),
                        Vec2::new(grid_width, DRUM_ROW_HEIGHT),
                    ),
                    0.0,
                    row_color,
                );
            }

            let track_color = app
                .project
                .tracks
                .get(track_idx)
                .map(|t| t.color)
                .unwrap_or([255, 140, 80]);

            let clip_indices: Vec<usize> = if let Some(ci) = app.selected_clip {
                vec![ci]
            } else {
                app.project
                    .tracks
                    .get(track_idx)
                    .map(|t| (0..t.clips.len()).collect())
                    .unwrap_or_default()
            };

            let bw = beat_width(app);
            for clip_idx in clip_indices {
                let Some(clip) = app
                    .project
                    .tracks
                    .get(track_idx)
                    .and_then(|t| t.clips.get(clip_idx))
                    .cloned()
                else {
                    continue;
                };
                for (note_idx, note) in clip.notes.iter().enumerate() {
                    let Some(row) = drum_row_index(note.pitch) else {
                        continue;
                    };

                    let mut color =
                        Color32::from_rgb(track_color[0], track_color[1], track_color[2]);
                    if selected == Some((clip_idx, note_idx))
                        || note_drag
                            .map(|d| d.clip_index == clip_idx && d.note_index == note_idx)
                            .unwrap_or(false)
                    {
                        color = color.gamma_multiply(1.35);
                    }

                    let abs_start = clip.start_beat + note.start_beat;
                    let x = x_from_beat(app, abs_start, rect.min.x);
                    let w = (note.duration_beats as f32 * bw).max(8.0);
                    let y = rect.min.y + row as f32 * DRUM_ROW_HEIGHT + 2.0;
                    let note_rect = Rect::from_min_size(
                        egui::pos2(x, y),
                        Vec2::new(w, DRUM_ROW_HEIGHT - 4.0),
                    );

                    let intensity = (note.velocity * 255.0) as u8;
                    let hit_color = Color32::from_rgb(
                        255,
                        (160.0 + note.velocity * 95.0) as u8,
                        intensity / 2,
                    );
                    painter.rect_filled(note_rect, 3.0, hit_color);
                    painter.rect_stroke(note_rect, 3.0, egui::Stroke::new(1.0_f32, color));

                    let body_resp = ui.interact(
                        note_rect,
                        ui.id().with(("drum_hit", clip_idx, note_idx)),
                        Sense::click_and_drag(),
                    );
                    if body_resp.hovered() || body_resp.dragged() {
                        ui.ctx().set_cursor_icon(CursorIcon::Grab);
                    }
                    if body_resp.clicked() {
                        app.selected_clip = Some(clip_idx);
                        app.selected_note = Some((clip_idx, note_idx));
                    }
                    if body_resp.drag_started() {
                        app.selected_clip = Some(clip_idx);
                        app.selected_note = Some((clip_idx, note_idx));
                        if let Some(pos) = body_resp.interact_pointer_pos() {
                            let abs_beat = beat_from_x(app, pos.x, rect.min.x);
                            app.note_drag = Some(NoteDragState {
                                clip_index: clip_idx,
                                note_index: note_idx,
                                mode: NoteDragMode::Move,
                                grab_beat: abs_beat - (clip.start_beat + note.start_beat),
                                grab_pitch: note.pitch as i32,
                            });
                        }
                    }
                }
            }

            draw_playhead(app, ui, &painter, rect, &response, true);
        });
}

fn draw_section_overlay(
    app: &mut DAWApp,
    ui: &mut Ui,
    painter: &egui::Painter,
    rect: Rect,
    section_range: Option<(f64, f64)>,
) {
    let Some((s, e)) = section_range else {
        return;
    };
    let x0 = x_from_beat(app, s, rect.min.x);
    let x1 = x_from_beat(app, e, rect.min.x);
    painter.rect_filled(
        Rect::from_min_max(egui::pos2(x0, rect.min.y), egui::pos2(x1, rect.max.y)),
        0.0,
        Color32::from_rgba_unmultiplied(100, 180, 255, 35),
    );
    let section_resp = ui.interact(
        Rect::from_min_max(egui::pos2(x0, rect.min.y), egui::pos2(x1, rect.min.y + 12.0)),
        ui.id().with("section_drag_handle"),
        Sense::click_and_drag(),
    );
    if section_resp.drag_started() {
        let clip_idx = app.selected_clip.unwrap_or(0);
        if let Some(track) = app.project.tracks.get(app.current_track) {
            if let Some(clip) = track.clips.get(clip_idx) {
                let rel_s = s - clip.start_beat;
                let rel_e = e - clip.start_beat;
                let indices = clip.notes_in_visible_range(rel_s, rel_e);
                let mut origins = std::collections::HashMap::new();
                for &idx in &indices {
                    if let Some(n) = clip.notes.get(idx) {
                        origins.insert(idx, n.start_beat);
                    }
                }
                app.section_clip_index = Some(clip_idx);
                app.section_note_indices = Some(indices);
                app.section_drag_origins = origins;
                if let Some(pos) = section_resp.interact_pointer_pos() {
                    app.section_drag = Some((s, beat_from_x(app, pos.x, rect.min.x)));
                }
            }
        }
    }
}

fn draw_melodic_notes(
    app: &mut DAWApp,
    ui: &mut Ui,
    painter: &egui::Painter,
    rect: Rect,
    selected: Option<(usize, usize)>,
    note_drag: Option<NoteDragState>,
    section_range: Option<(f64, f64)>,
    roll_low: u8,
    roll_high: u8,
    visible_rows: u8,
) {
    let track_color = app.project.tracks[app.current_track].color;
    let clip_indices: Vec<usize> = if let Some(ci) = app.selected_clip {
        vec![ci]
    } else {
        (0..app.project.tracks[app.current_track].clips.len()).collect()
    };
    let bw = beat_width(app);

    for clip_idx in clip_indices {
        let Some(clip) = app
            .project
            .tracks
            .get(app.current_track)
            .and_then(|t| t.clips.get(clip_idx))
            .cloned()
        else {
            continue;
        };

        for (note_idx, note) in clip.notes.iter().enumerate() {
            if note.pitch < roll_low || note.pitch > roll_high {
                continue;
            }
            let row = roll_high.saturating_sub(note.pitch);
            if row >= visible_rows {
                continue;
            }
            if !clip.note_is_visible(note) {
                continue;
            }

            let abs_start = clip.start_beat + note.start_beat;

            let mut color = Color32::from_rgb(track_color[0], track_color[1], track_color[2]);
            if selected == Some((clip_idx, note_idx))
                || note_drag
                    .map(|d| d.clip_index == clip_idx && d.note_index == note_idx)
                    .unwrap_or(false)
            {
                color = color.gamma_multiply(1.35);
            }
            if app.selected_clip == Some(clip_idx) {
                color = color.gamma_multiply(1.1);
            }
            if let Some((s, e)) = section_range {
                if abs_start >= s && abs_start < e {
                    color = Color32::from_rgb(
                        (track_color[0] as f32 * 0.7 + 100.0) as u8,
                        (track_color[1] as f32 * 0.7 + 180.0) as u8,
                        (track_color[2] as f32 * 0.7 + 255.0) as u8,
                    );
                }
            }

            let row = roll_high.saturating_sub(note.pitch) as f32;
            let x = x_from_beat(app, abs_start, rect.min.x);
            let w = (note.duration_beats as f32 * bw).max(6.0);
            let y = rect.min.y + row * PIANO_ROW_HEIGHT + 1.0;
            let note_rect =
                Rect::from_min_size(egui::pos2(x, y), Vec2::new(w, PIANO_ROW_HEIGHT - 2.0));

            painter.rect_filled(note_rect, 2.0, color);
            painter.rect_stroke(note_rect, 2.0, egui::Stroke::new(1.0_f32, color.gamma_multiply(1.3)));

            let resize_rect = Rect::from_min_max(
                egui::pos2(note_rect.max.x - NOTE_RESIZE_HANDLE, note_rect.min.y),
                note_rect.max,
            );
            let body_rect =
                Rect::from_min_max(note_rect.min, egui::pos2(resize_rect.min.x, note_rect.max.y));

            let resize_resp = ui.interact(
                resize_rect,
                ui.id().with(("note_resize", clip_idx, note_idx)),
                Sense::click_and_drag(),
            );
            let body_resp = ui.interact(
                body_rect,
                ui.id().with(("note_body", clip_idx, note_idx)),
                Sense::click_and_drag(),
            );

            if resize_resp.hovered() || resize_resp.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            } else if body_resp.hovered() || body_resp.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }

            if body_resp.clicked() {
                app.selected_clip = Some(clip_idx);
                app.selected_note = Some((clip_idx, note_idx));
            }
            if resize_resp.drag_started() {
                app.selected_clip = Some(clip_idx);
                app.selected_note = Some((clip_idx, note_idx));
                app.note_drag = Some(NoteDragState {
                    clip_index: clip_idx,
                    note_index: note_idx,
                    mode: NoteDragMode::Resize,
                    grab_beat: 0.0,
                    grab_pitch: note.pitch as i32,
                });
            }
            if body_resp.drag_started() && !ui.input(|i| i.modifiers.alt) {
                app.selected_clip = Some(clip_idx);
                app.selected_note = Some((clip_idx, note_idx));
                if let Some(pos) = body_resp.interact_pointer_pos() {
                    let abs_beat = beat_from_x(app, pos.x, rect.min.x);
                    app.note_drag = Some(NoteDragState {
                        clip_index: clip_idx,
                        note_index: note_idx,
                        mode: NoteDragMode::Move,
                        grab_beat: abs_beat - (clip.start_beat + note.start_beat),
                        grab_pitch: note.pitch as i32,
                    });
                }
            }
        }
    }
}

fn draw_melodic_pitch_rows(
    painter: &egui::Painter,
    rect: Rect,
    grid_width: f32,
    roll_low: u8,
    roll_high: u8,
    visible_rows: u8,
) {
    for row in 0..visible_rows {
        let pitch = roll_high.saturating_sub(row);
        if pitch < roll_low {
            break;
        }
        let y = rect.min.y + row as f32 * PIANO_ROW_HEIGHT;
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
}

fn draw_playhead(
    app: &mut DAWApp,
    ui: &mut Ui,
    painter: &egui::Painter,
    rect: Rect,
    response: &egui::Response,
    is_drum: bool,
) {
    let playhead_x = x_from_beat(app, app.project.playhead_beat, rect.min.x);
    let playhead_rect = Rect::from_min_max(
        egui::pos2(playhead_x - PLAYHEAD_HIT_WIDTH * 0.5, rect.min.y),
        egui::pos2(playhead_x + PLAYHEAD_HIT_WIDTH * 0.5, rect.max.y),
    );

    let ph_response = ui.interact(
        playhead_rect,
        ui.id().with("piano_roll_playhead"),
        Sense::click_and_drag(),
    );

    if ph_response.hovered() || ph_response.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }

    let handle_color = if ph_response.hovered() || ph_response.dragged() {
        Color32::from_rgb(255, 120, 120)
    } else {
        Color32::from_rgb(255, 80, 80)
    };

    painter.line_segment(
        [egui::pos2(playhead_x, rect.min.y), egui::pos2(playhead_x, rect.max.y)],
        egui::Stroke::new(
            if ph_response.hovered() || ph_response.dragged() {
                3.0_f32
            } else {
                2.0_f32
            },
            handle_color,
        ),
    );

    if ph_response.dragged() || ph_response.clicked() {
        if let Some(pos) = ph_response.interact_pointer_pos() {
            app.set_playhead(beat_from_x(app, pos.x, rect.min.x));
        }
    }

    if response.clicked() && !ph_response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let beat = snap_beat(
                app,
                beat_from_x(app, pos.x, rect.min.x),
                ui.input(|i| i.modifiers.shift),
            );
            if is_drum {
                let row = ((pos.y - rect.min.y) / DRUM_ROW_HEIGHT).floor() as i32;
                if let Some(&drum) = DrumKind::all().get(row as usize) {
                    let accent = ui.input(|i| i.modifiers.shift);
                    app.toggle_drum_hit_at_beat(app.current_track, drum, beat, accent);
                }
            } else {
                app.set_playhead(beat.floor().max(0.0));
            }
        }
    }

    if !is_drum {
        if response.drag_started() && ui.input(|i| i.modifiers.alt) {
            if let Some(pos) = response.interact_pointer_pos() {
                app.section_select_drag = Some(beat_from_x(app, pos.x, rect.min.x));
            }
        }
        if response.dragged() {
            if let (Some(start), Some(pos)) =
                (app.section_select_drag, response.interact_pointer_pos())
            {
                let end = beat_from_x(app, pos.x, rect.min.x);
                app.apply_section_range_drag(start, end);
            }
        }
        if response.drag_stopped() {
            app.section_select_drag = None;
        }

        if response.double_clicked() && !ph_response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let beat = beat_from_x(app, pos.x, rect.min.x).floor().max(0.0);
                let row = ((pos.y - rect.min.y) / PIANO_ROW_HEIGHT).floor() as i32;
                let roll_high = app.piano_roll_high();
                let roll_low = app.piano_roll_low();
                let pitch = roll_high.saturating_sub(row as u8);
                if pitch >= roll_low && pitch <= roll_high {
                    app.set_playhead(beat);
                    app.add_note_at(pitch, beat, 1.0);
                }
            }
        }
    } else if response.double_clicked() && !ph_response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let beat = snap_beat(
                app,
                beat_from_x(app, pos.x, rect.min.x),
                ui.input(|i| i.modifiers.shift),
            );
            let row = ((pos.y - rect.min.y) / DRUM_ROW_HEIGHT).floor() as i32;
            if let Some(&drum) = DrumKind::all().get(row as usize) {
                let accent = ui.input(|i| i.modifiers.shift);
                app.toggle_drum_hit_at_beat(app.current_track, drum, beat, accent);
            }
        }
    }
}
