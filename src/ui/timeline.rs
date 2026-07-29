use crate::app::{ClipDragMode, ClipDragState, DAWApp};
use crate::ui::constants::{TRACK_HEADER_WIDTH, TRACK_ROW_HEIGHT};
use crate::ui::grid::{
    beat_from_x, beat_width, draw_beat_grid_lines, draw_song_sections, draw_time_ruler,
    handle_zoom_scroll, section_color, show_zoom_controls, x_from_beat, RULER_HEIGHT,
};
use crate::ui::theme;
use egui::{Color32, CursorIcon, Rect, Sense, Ui, Vec2};

const CLIP_HANDLE_WIDTH: f32 = 8.0;

pub fn show_multi_track_timeline(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("Timeline");
    ui.label(
        "Clips group notes from each recording or edit — drag to move, trim edges to crop (recoverable), \
         drag the loop handle to repeat.",
    );
    show_zoom_controls(app, ui, "timeline_zoom");
    show_song_sections_panel(app, ui);
    ui.add_space(4.0);

    let total_beats = app.project.total_beats as f32;
    let bw = beat_width(app);
    let grid_width = total_beats * bw;
    let track_count = app.project.tracks.len();
    let total_height = track_count as f32 * TRACK_ROW_HEIGHT;
    let body_height = RULER_HEIGHT + total_height;

    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        let (left_rect, _) = ui.allocate_exact_size(
            Vec2::new(TRACK_HEADER_WIDTH, body_height),
            Sense::hover(),
        );
        {
            let corner = Rect::from_min_size(left_rect.min, Vec2::new(TRACK_HEADER_WIDTH, RULER_HEIGHT));
            ui.painter_at(left_rect).rect_filled(
                corner,
                0.0,
                Color32::from_rgb(28, 28, 34),
            );
            for i in 0..track_count {
                let row_rect = Rect::from_min_size(
                    egui::pos2(
                        left_rect.min.x,
                        left_rect.min.y + RULER_HEIGHT + i as f32 * TRACK_ROW_HEIGHT,
                    ),
                    Vec2::new(TRACK_HEADER_WIDTH, TRACK_ROW_HEIGHT),
                );
                draw_track_header_in_rect(app, ui, row_rect, i);
            }
        }

        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .id_salt("timeline_grid_scroll")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                ui.set_min_height(body_height);

                let content_width = grid_width.max(ui.available_width());
                let (area_rect, area_response) = ui.allocate_exact_size(
                    Vec2::new(content_width, body_height),
                    Sense::click(),
                );
                handle_zoom_scroll(app, ui, area_response.hovered());

                let ruler_rect = Rect::from_min_size(area_rect.min, Vec2::new(content_width, RULER_HEIGHT));
                draw_time_ruler(app, &ui.painter_at(ruler_rect), ruler_rect, total_beats);

                let grid_rect = Rect::from_min_size(
                    egui::pos2(area_rect.min.x, area_rect.min.y + RULER_HEIGHT),
                    Vec2::new(content_width, total_height),
                );
                app.timeline_grid_origin_x = Some(grid_rect.min.x);

                let painter = ui.painter_at(grid_rect);
                draw_song_sections(app, &painter, grid_rect, true);
                draw_beat_grid_lines(app, &painter, grid_rect, total_beats);

                for i in 0..track_count {
                    let row_rect = Rect::from_min_size(
                        egui::pos2(
                            grid_rect.min.x,
                            grid_rect.min.y + i as f32 * TRACK_ROW_HEIGHT,
                        ),
                        Vec2::new(content_width, TRACK_ROW_HEIGHT),
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
                            egui::Stroke::new(
                                2.0_f32,
                                Color32::from_rgb(
                                    app.project.tracks[i].color[0],
                                    app.project.tracks[i].color[1],
                                    app.project.tracks[i].color[2],
                                ),
                            ),
                        );
                    }

                    draw_track_clips(app, ui, &painter, row_rect, i, grid_rect.min.x);

                    let row_response =
                        ui.interact(row_rect, ui.id().with(("track_row", i)), Sense::click());
                    if row_response.clicked() {
                        if let Some(pos) = row_response.interact_pointer_pos() {
                            app.select_track(i);
                            app.set_playhead(beat_from_x(app, pos.x, grid_rect.min.x));
                        }
                    }

                    if i > 0 {
                        painter.line_segment(
                            [
                                egui::pos2(row_rect.min.x, row_rect.min.y),
                                egui::pos2(row_rect.max.x, row_rect.min.y),
                            ],
                            egui::Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 25)),
                        );
                    }
                }

                let playhead_x = x_from_beat(app, app.project.playhead_beat, grid_rect.min.x);
                painter.line_segment(
                    [
                        egui::pos2(playhead_x, grid_rect.min.y),
                        egui::pos2(playhead_x, grid_rect.max.y),
                    ],
                    egui::Stroke::new(2.0_f32, Color32::from_rgb(255, 80, 80)),
                );

                // Row dividers on the header column (same Y as grid rows).
                let header_painter = ui.painter_at(left_rect);
                for i in 1..track_count {
                    let y = left_rect.min.y + RULER_HEIGHT + i as f32 * TRACK_ROW_HEIGHT;
                    header_painter.line_segment(
                        [
                            egui::pos2(left_rect.min.x, y),
                            egui::pos2(left_rect.max.x, y),
                        ],
                        egui::Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 25)),
                    );
                }
            });
    });

    if let Some(origin) = app.timeline_grid_origin_x {
        handle_clip_drag(app, ui, origin);
    }
}

fn show_song_sections_panel(app: &mut DAWApp, ui: &mut Ui) {
    ui.collapsing("Song sections (A, B, C…)", |ui| {
        ui.label(
            egui::RichText::new(
                "Define structural sections with a label and bar count. Sections are placed \
                 end-to-end and shown as colored regions on the timeline.",
            )
            .small()
            .color(Color32::GRAY),
        );
        ui.horizontal(|ui| {
            ui.label("Label:");
            ui.text_edit_singleline(&mut app.new_section_label);
            ui.label("Bars:");
            ui.add(egui::DragValue::new(&mut app.new_section_bars).range(1..=256));
            if ui.button("+ Add section").clicked() {
                app.add_song_section();
            }
            if app.selected_song_section.is_some() && ui.button("Remove selected").clicked() {
                app.remove_selected_song_section();
            }
        });

        if app.project.sections.is_empty() {
            ui.label(
                egui::RichText::new("No sections yet — add A (8 bars), B (18 bars), etc.")
                    .small()
                    .color(Color32::GRAY),
            );
        } else {
            let beats_per_bar = app.project.beats_per_bar();
            let section_labels: Vec<(usize, String, Color32)> = app
                .project
                .sections
                .iter()
                .enumerate()
                .map(|(idx, section)| {
                    (
                        idx,
                        format!(
                            "{} · {} bars (beat {:.0}–{:.0})",
                            section.label,
                            section.bar_count,
                            section.start_beat,
                            section.end_beat(beats_per_bar),
                        ),
                        section_color(idx),
                    )
                })
                .collect();
            let mut jump_to = None;
            ui.horizontal_wrapped(|ui| {
                for (idx, label, color) in section_labels {
                    let selected = app.selected_song_section == Some(idx);
                    if ui
                        .selectable_label(selected, label)
                        .on_hover_text("Click to jump playhead to this section")
                        .clicked()
                    {
                        jump_to = Some(idx);
                    }
                    ui.colored_label(color, "■");
                }
            });
            if let Some(idx) = jump_to {
                app.jump_to_song_section(idx);
            }
        }
    });
}

fn draw_track_header_in_rect(app: &mut DAWApp, ui: &mut Ui, rect: Rect, index: usize) {
    let selected = app.current_track == index;
    let track_name = app.project.tracks[index].name.clone();
    let track_color = app.project.tracks[index].color;
    let instrument_label = app.project.tracks[index].instrument.label();
    let mut muted = app.project.tracks[index].muted;
    let recording = app.recording && selected;
    let color = Color32::from_rgb(track_color[0], track_color[1], track_color[2]);
    let clip_count = app.project.tracks[index].clips.len();

    let bg = if selected {
        Color32::from_rgba_unmultiplied(track_color[0], track_color[1], track_color[2], 50)
    } else if index % 2 == 0 {
        Color32::from_rgb(32, 32, 38)
    } else {
        Color32::from_rgb(36, 36, 42)
    };
    ui.painter_at(rect).rect_filled(rect, 0.0, bg);

    if selected {
        ui.painter_at(rect)
            .rect_stroke(rect, 0.0, egui::Stroke::new(2.0_f32, color));
    }

    let response = ui.interact(rect, ui.id().with(("track_header", index)), Sense::click());

    ui.allocate_new_ui(
        egui::UiBuilder::new().max_rect(rect.shrink(6.0)),
        |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
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
                    egui::RichText::new(format!("{} · {} clip(s)", instrument_label, clip_count))
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

fn draw_track_clips(
    app: &mut DAWApp,
    ui: &mut Ui,
    painter: &egui::Painter,
    row_rect: Rect,
    track_index: usize,
    grid_origin_x: f32,
) {
    let track_color = app.project.tracks[track_index].color;
    let clips: Vec<crate::model::NoteClip> = app.project.tracks[track_index].clips.clone();
    let inner = row_rect.shrink2(Vec2::new(2.0, 6.0));
    let bw = beat_width(app);

    if clips.is_empty() {
        painter.text(
            inner.center(),
            egui::Align2::CENTER_CENTER,
            "empty — record or draw notes to create a clip",
            theme::font(theme::FONT_MD),
            Color32::from_rgba_unmultiplied(180, 180, 180, 80),
        );
        return;
    }

    let color = Color32::from_rgb(track_color[0], track_color[1], track_color[2]);

    for (clip_idx, clip) in clips.iter().enumerate() {
        let is_selected =
            app.current_track == track_index && app.selected_clip == Some(clip_idx);

        let vis_len = clip.visible_length();
        let x0 = x_from_beat(app, clip.start_beat, inner.min.x);
        let x1 = x_from_beat(app, clip.start_beat + vis_len, inner.min.x);
        let clip_rect = Rect::from_min_max(
            egui::pos2(x0, inner.min.y),
            egui::pos2(x1.max(x0 + 12.0), inner.max.y),
        );

        let fill = if is_selected {
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 200)
        } else {
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 140)
        };
        painter.rect_filled(clip_rect, 6.0, fill);
        painter.rect_stroke(
            clip_rect,
            6.0,
            egui::Stroke::new(
                if is_selected { 2.0_f32 } else { 1.0_f32 },
                if clip.loop_enabled {
                    Color32::from_rgb(120, 230, 160)
                } else {
                    color.gamma_multiply(1.3)
                },
            ),
        );

        let label = if clip.loop_enabled {
            format!("↻ {}", clip.name)
        } else {
            clip.name.clone()
        };
        painter.text(
            clip_rect.left_top() + Vec2::new(6.0, 4.0),
            egui::Align2::LEFT_TOP,
            label,
            theme::font(theme::FONT_MD),
            Color32::WHITE,
        );

        draw_clip_note_preview(painter, &clip, clip_rect, color);

        if clip.loop_enabled {
            let loop_x1 = x_from_beat(app, clip.loop_end_beat, inner.min.x);
            if loop_x1 > x1 + 4.0 {
                let loop_rect = Rect::from_min_max(
                    egui::pos2(x1, inner.min.y + 2.0),
                    egui::pos2(loop_x1, inner.max.y - 2.0),
                );
                painter.rect_filled(
                    loop_rect,
                    4.0,
                    Color32::from_rgba_unmultiplied(80, 200, 120, 35),
                );
                painter.text(
                    loop_rect.left_top() + Vec2::new(4.0, 2.0),
                    egui::Align2::LEFT_TOP,
                    "loop",
                    theme::font(theme::FONT_XS),
                    Color32::from_rgb(120, 230, 160),
                );
            }
        }

        if app.current_track == track_index {
            interact_clip(app, ui, clip_rect, track_index, clip_idx, grid_origin_x, clip, bw);
        }
    }
}

fn draw_clip_note_preview(
    painter: &egui::Painter,
    clip: &crate::model::NoteClip,
    clip_rect: Rect,
    color: Color32,
) {
    if clip.notes.is_empty() {
        return;
    }
    let vis_len = clip.visible_length().max(0.01);
    for note in &clip.notes {
        if !clip.note_is_visible(note) {
            continue;
        }
        let rel = (note.start_beat - clip.trim_start) / vis_len;
        let nx = clip_rect.min.x + rel as f32 * clip_rect.width();
        let bar_h = 4.0;
        let ny = clip_rect.center().y - bar_h * 0.5;
        painter.rect_filled(
            Rect::from_min_size(egui::pos2(nx, ny), Vec2::new(3.0, bar_h)),
            1.0,
            color.gamma_multiply(0.7),
        );
    }
}

fn interact_clip(
    app: &mut DAWApp,
    ui: &mut Ui,
    clip_rect: Rect,
    track_index: usize,
    clip_idx: usize,
    grid_origin_x: f32,
    clip: &crate::model::NoteClip,
    bw: f32,
) {
    let trim_start_handle = Rect::from_center_size(
        egui::pos2(clip_rect.min.x + 4.0, clip_rect.center().y),
        egui::vec2(CLIP_HANDLE_WIDTH, clip_rect.height() - 6.0),
    );
    let trim_end_handle = Rect::from_center_size(
        egui::pos2(clip_rect.max.x - 4.0, clip_rect.center().y),
        egui::vec2(CLIP_HANDLE_WIDTH, clip_rect.height() - 6.0),
    );
    let loop_handle = Rect::from_center_size(
        egui::pos2(
            clip_rect.min.x + (clip.loop_end_beat - clip.start_beat) as f32 * bw,
            clip_rect.center().y,
        ),
        egui::vec2(CLIP_HANDLE_WIDTH, clip_rect.height() - 6.0),
    );

    let body_rect = clip_rect.shrink2(egui::vec2(CLIP_HANDLE_WIDTH + 2.0, 0.0));

    let body_resp = ui.interact(
        body_rect,
        ui.id().with(("clip_body", track_index, clip_idx)),
        Sense::click_and_drag(),
    );
    let trim_start_resp = ui.interact(
        trim_start_handle,
        ui.id().with(("clip_trim_start", track_index, clip_idx)),
        Sense::drag(),
    );
    let trim_end_resp = ui.interact(
        trim_end_handle,
        ui.id().with(("clip_trim_end", track_index, clip_idx)),
        Sense::drag(),
    );
    let loop_resp = ui.interact(
        loop_handle,
        ui.id().with(("clip_loop_extend", track_index, clip_idx)),
        Sense::drag(),
    );

    if body_resp.hovered() || body_resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
    if trim_start_resp.hovered() || trim_end_resp.hovered() || loop_resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }

    if body_resp.clicked() {
        app.select_track(track_index);
        app.selected_clip = Some(clip_idx);
    }

    if trim_start_resp.drag_started() {
        app.selected_clip = Some(clip_idx);
        if let Some(pos) = trim_start_resp.interact_pointer_pos() {
            app.clip_drag = Some(ClipDragState {
                track_index,
                clip_index: clip_idx,
                mode: ClipDragMode::TrimStart,
                grab_beat: beat_from_x(app, pos.x, grid_origin_x),
                orig_start: clip.start_beat,
                orig_trim_start: clip.trim_start,
                orig_trim_end: clip.trim_end,
                orig_loop_end: clip.loop_end_beat,
            });
        }
    }
    if trim_end_resp.drag_started() {
        app.selected_clip = Some(clip_idx);
        if let Some(pos) = trim_end_resp.interact_pointer_pos() {
            app.clip_drag = Some(ClipDragState {
                track_index,
                clip_index: clip_idx,
                mode: ClipDragMode::TrimEnd,
                grab_beat: beat_from_x(app, pos.x, grid_origin_x),
                orig_start: clip.start_beat,
                orig_trim_start: clip.trim_start,
                orig_trim_end: clip.trim_end,
                orig_loop_end: clip.loop_end_beat,
            });
        }
    }
    if loop_resp.drag_started() {
        app.selected_clip = Some(clip_idx);
        if let Some(pos) = loop_resp.interact_pointer_pos() {
            app.clip_drag = Some(ClipDragState {
                track_index,
                clip_index: clip_idx,
                mode: ClipDragMode::ExtendLoop,
                grab_beat: beat_from_x(app, pos.x, grid_origin_x),
                orig_start: clip.start_beat,
                orig_trim_start: clip.trim_start,
                orig_trim_end: clip.trim_end,
                orig_loop_end: clip.loop_end_beat,
            });
        }
    }
    if body_resp.drag_started() {
        app.selected_clip = Some(clip_idx);
        if let Some(pos) = body_resp.interact_pointer_pos() {
            app.clip_drag = Some(ClipDragState {
                track_index,
                clip_index: clip_idx,
                mode: ClipDragMode::Move,
                grab_beat: beat_from_x(app, pos.x, grid_origin_x) - clip.start_beat,
                orig_start: clip.start_beat,
                orig_trim_start: clip.trim_start,
                orig_trim_end: clip.trim_end,
                orig_loop_end: clip.loop_end_beat,
            });
        }
    }
}

pub fn handle_clip_drag(app: &mut DAWApp, ui: &mut Ui, grid_origin_x: f32) {
    let Some(drag) = app.clip_drag else {
        return;
    };
    if !ui.input(|i| i.pointer.primary_down()) {
        app.clip_drag = None;
        return;
    }
    let Some(pos) = ui.input(|i| i.pointer.latest_pos()) else {
        return;
    };

    let beat = beat_from_x(app, pos.x, grid_origin_x);
    let track_idx = drag.track_index;
    let clip_idx = drag.clip_index;

    if let Some(clip) = app
        .project
        .tracks
        .get_mut(track_idx)
        .and_then(|t| t.clips.get_mut(clip_idx))
    {
        match drag.mode {
            ClipDragMode::Move => {
                let new_start = (beat - drag.grab_beat).max(0.0);
                let delta = new_start - drag.orig_start;
                clip.offset_on_timeline(delta);
            }
            ClipDragMode::TrimStart => {
                let rel = (beat - clip.start_beat).max(0.0);
                clip.set_trim_start(rel.min(clip.source_length - clip.trim_end - 0.25));
            }
            ClipDragMode::TrimEnd => {
                let rel_end = (beat - clip.start_beat).max(clip.trim_start + 0.25);
                clip.set_trim_end((clip.source_length - rel_end).max(0.0));
            }
            ClipDragMode::ExtendLoop => {
                clip.loop_enabled = true;
                clip.set_loop_end(beat.max(clip.start_beat + clip.visible_length()));
            }
        }
    }
}
