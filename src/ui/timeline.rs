use crate::app::{ClipDragMode, ClipDragState, DAWApp};
use crate::ui::constants::{TRACK_HEADER_WIDTH, TRACK_ROW_HEIGHT};
use crate::ui::grid::{
    beat_from_x, beat_width, draw_beat_grid_lines, draw_song_sections, draw_time_ruler,
    handle_zoom_scroll, playhead_scrubber, section_color, show_zoom_controls, x_from_beat,
    RULER_HEIGHT,
};
use crate::model::automation::FxParam;
use crate::ui::dj_widgets;
use crate::ui::theme;
use egui::{Color32, CursorIcon, Rect, Sense, Ui, Vec2};
use egui_phosphor::regular as ph;

const CLIP_HANDLE_WIDTH: f32 = 8.0;
/// Height of an open automation lane under a track.
const LANE_HEIGHT: f32 = 66.0;
/// Lane points snap to this fraction of a beat.
const LANE_SNAP: f64 = 1.0 / 16.0;

/// Top offset (from the first row) and height of each track row, lane included.
fn row_layout(app: &DAWApp) -> Vec<(f32, bool)> {
    let mut y = 0.0;
    app.project
        .tracks
        .iter()
        .map(|t| {
            let top = y;
            let lane = t.lane_view.is_some();
            y += TRACK_ROW_HEIGHT + if lane { LANE_HEIGHT } else { 0.0 };
            (top, lane)
        })
        .collect()
}

pub fn show_multi_track_timeline(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("Timeline");
    ui.label(
        "Clips group notes from each recording or edit — drag to move, trim edges to crop (recoverable), \
         drag the loop handle to repeat.",
    );
    ui.horizontal(|ui| {
        show_zoom_controls(app, ui, "timeline_zoom");
        ui.checkbox(&mut app.snap_clips_to_playhead, "Snap to playhead")
            .on_hover_text("Dragged clip edges stick to the playhead when close. Hold Alt to drag freely.");
    });
    show_song_sections_panel(app, ui);
    ui.add_space(4.0);

    let total_beats = app.project.total_beats as f32;
    let bw = beat_width(app);
    let grid_width = total_beats * bw;
    let track_count = app.project.tracks.len();
    let rows = row_layout(app);
    let total_height = rows
        .last()
        .map(|&(top, lane)| top + TRACK_ROW_HEIGHT + if lane { LANE_HEIGHT } else { 0.0 })
        .unwrap_or(0.0);
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
                let (top, lane) = rows[i];
                let row_rect = Rect::from_min_size(
                    egui::pos2(left_rect.min.x, left_rect.min.y + RULER_HEIGHT + top),
                    Vec2::new(TRACK_HEADER_WIDTH, TRACK_ROW_HEIGHT),
                );
                draw_track_header_in_rect(app, ui, row_rect, i);
                if lane {
                    let lane_rect = Rect::from_min_size(
                        row_rect.left_bottom(),
                        Vec2::new(TRACK_HEADER_WIDTH, LANE_HEIGHT),
                    );
                    draw_lane_header(app, ui, lane_rect, i);
                }
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
                    let (top, lane) = rows[i];
                    let row_rect = Rect::from_min_size(
                        egui::pos2(grid_rect.min.x, grid_rect.min.y + top),
                        Vec2::new(content_width, TRACK_ROW_HEIGHT),
                    );
                    if lane {
                        let lane_rect = Rect::from_min_size(
                            row_rect.left_bottom(),
                            Vec2::new(content_width, LANE_HEIGHT),
                        );
                        draw_lane(app, ui, &painter, lane_rect, i, grid_rect.min.x);
                    }

                    let selected = app.current_track == i;
                    // Opaque colours only: a translucent tint lets the light theme's white
                    // window background show through and washes out clips and loops.
                    let bg = if selected {
                        blend(Color32::from_rgb(38, 38, 44), app.project.tracks[i].color, 0.18)
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

                playhead_scrubber(app, ui, ruler_rect, grid_rect, "timeline_playhead");
                if app.clip_snapped && app.clip_drag.is_some() {
                    let x = x_from_beat(app, app.project.playhead_beat, grid_rect.min.x);
                    ui.painter_at(grid_rect).line_segment(
                        [egui::pos2(x, grid_rect.min.y), egui::pos2(x, grid_rect.max.y)],
                        egui::Stroke::new(3.0_f32, Color32::from_rgb(255, 210, 90)),
                    );
                }

                // Row dividers on the header column (same Y as grid rows).
                let header_painter = ui.painter_at(left_rect);
                for &(top, _) in rows.iter().skip(1) {
                    let y = left_rect.min.y + RULER_HEIGHT + top;
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

    // Add tracks right under the last one, where a new row will appear.
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button("+ Melodic track")
            .on_hover_text("Add a track for piano, bass, synths… below the others")
            .clicked()
        {
            app.add_new_track(false);
        }
        if ui
            .button("+ Drum track")
            .on_hover_text("Add a drum track below the others (opens the step sequencer)")
            .clicked()
        {
            app.add_new_track(true);
        }
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
        blend(Color32::from_rgb(34, 34, 40), track_color, 0.28)
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

    // Two lines: [● name ⏺ ..... M] then "instrument · n clips". Clipped to the row so a
    // long name can never spill into the next track.
    let inner = rect.shrink2(Vec2::new(8.0, 7.0));
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner), |ui| {
        ui.set_clip_rect(inner.intersect(ui.clip_rect()));
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
            ui.horizontal(|ui| {
                let icon = app.project.tracks[index].icon();
                let icon_resp = dj_widgets::icon_button(ui, icon, color, 18.0)
                    .on_hover_text("Track icon — click to change");
                let popup_id = ui.id().with(("icon_popup", index));
                if icon_resp.clicked() {
                    ui.memory_mut(|m| m.toggle_popup(popup_id));
                }
                egui::popup::popup_below_widget(
                    ui,
                    popup_id,
                    &icon_resp,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui| {
                        ui.set_min_width(240.0);
                        if let Some(picked) = dj_widgets::icon_grid(ui, icon, color) {
                            app.project.tracks[index].icon = Some(picked);
                            ui.memory_mut(|m| m.close_popup());
                        }
                    },
                );
                let name = ui.selectable_label(selected, egui::RichText::new(&track_name).strong());
                if name.clicked() {
                    app.select_track(index);
                }
                name.context_menu(|ui| track_context_menu(app, ui, index));
                if recording {
                    ui.colored_label(Color32::from_rgb(255, 60, 60), "⏺");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = egui::RichText::new("M").small().strong();
                    let mute = egui::Button::new(if muted {
                        label.color(Color32::BLACK)
                    } else {
                        label
                    })
                    .fill(if muted {
                        Color32::from_rgb(240, 190, 60)
                    } else {
                        Color32::from_rgb(55, 55, 64)
                    })
                    .min_size(Vec2::new(20.0, 16.0));
                    if ui.add(mute).on_hover_text("Mute track").clicked() {
                        muted = !muted;
                        app.project.tracks[index].muted = muted;
                    }
                    let open = app.project.tracks[index].lane_view.is_some();
                    let automated = app.project.tracks[index].has_automation();
                    let lane_btn = egui::Button::new(egui::RichText::new(ph::CHART_LINE).small().color(
                        if open {
                            Color32::BLACK
                        } else if automated {
                            dj_widgets::AUTOMATED
                        } else {
                            Color32::WHITE
                        },
                    ))
                    .fill(if open { dj_widgets::AUTOMATED } else { Color32::from_rgb(55, 55, 64) })
                    .min_size(Vec2::new(20.0, 16.0));
                    if ui
                        .add(lane_btn)
                        .on_hover_text("Effects over time: show / hide this track's automation lane")
                        .clicked()
                    {
                        let t = &mut app.project.tracks[index];
                        t.lane_view = if open {
                            None
                        } else {
                            // Open the first automated lane, else the filter.
                            Some(t.automation.iter().find(|l| !l.lane.is_empty()).map(|l| l.param).unwrap_or(FxParam::Filter))
                        };
                    }
                });
            });
            let clips = if clip_count == 1 { "1 clip".to_string() } else { format!("{clip_count} clips") };
            ui.label(
                egui::RichText::new(format!("{instrument_label} · {clips}"))
                    .small()
                    .color(Color32::GRAY),
            );
        });
    });

    if response.clicked() {
        app.select_track(index);
    }
    response.context_menu(|ui| track_context_menu(app, ui, index));
}

/// Right-click menu on a track header.
fn track_context_menu(app: &mut DAWApp, ui: &mut Ui, index: usize) {
    if ui
        .add_enabled(app.project.tracks.len() > 1, egui::Button::new("Delete track"))
        .on_disabled_hover_text("Can't delete the only track")
        .clicked()
    {
        app.request_delete_track(index);
        ui.close_menu();
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
        let is_selected = app.current_track == track_index
            && (app.selected_clip == Some(clip_idx) || app.selected_clip_ids.contains(&clip.id));

        let vis_len = clip.visible_length();
        let x0 = x_from_beat(app, clip.start_beat, inner.min.x);
        let x1 = x_from_beat(app, clip.start_beat + vis_len, inner.min.x);
        let clip_rect = Rect::from_min_max(
            egui::pos2(x0, inner.min.y),
            egui::pos2(x1.max(x0 + 12.0), inner.max.y),
        );

        let is_recording = app.recording && app.recording_clip == Some((track_index, clip_idx));
        let fill = if is_selected {
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 200)
        } else {
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 140)
        };
        painter.rect_filled(clip_rect, 6.0, fill);
        painter.rect_stroke(
            clip_rect,
            6.0,
            if is_recording {
                egui::Stroke::new(2.5_f32, Color32::from_rgb(255, 70, 70))
            } else {
                egui::Stroke::new(
                    if is_selected { 2.0_f32 } else { 1.0_f32 },
                    if is_selected {
                        Color32::WHITE
                    } else if clip.loop_enabled {
                        Color32::from_rgb(120, 230, 160)
                    } else {
                        color.gamma_multiply(1.3)
                    },
                )
            },
        );

        let label = if is_recording {
            format!("● REC {}", if app.recording_overdub { "overdub" } else { "" })
        } else if clip.loop_enabled {
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
                // Each repeat is drawn as a dimmer copy of the bubble (opaque, so it reads
                // on light and dark themes) with the same mini piano roll inside.
                let one = (x1 - x0).max(4.0);
                let dim = blend(Color32::from_rgb(38, 38, 44), [color.r(), color.g(), color.b()], 0.45);
                let mut rx = x1;
                let mut n = 0;
                while rx < loop_x1 - 1.0 && n < 512 {
                    let rep = Rect::from_min_max(
                        egui::pos2(rx, inner.min.y + 1.0),
                        egui::pos2((rx + one).min(loop_x1), inner.max.y - 1.0),
                    );
                    painter.rect_filled(rep, 5.0, dim);
                    dashed_outline(painter, rep, Color32::from_rgb(140, 235, 170));
                    let full = Rect::from_min_size(rep.min, Vec2::new(one, rep.height()));
                    draw_clip_note_preview(&painter.with_clip_rect(rep), &clip, full, color);
                    rx += one;
                    n += 1;
                }
                painter.text(
                    egui::pos2(x1 + 6.0, inner.min.y + 4.0),
                    egui::Align2::LEFT_TOP,
                    "↻ loop",
                    theme::font(theme::FONT_XS),
                    Color32::from_rgb(160, 245, 190),
                );
            }
        }

        if app.current_track == track_index {
            interact_clip(app, ui, clip_rect, track_index, clip_idx, grid_origin_x, clip, bw);
        }
    }
}

/// Opaque mix of `base` toward an RGB colour (`t` = 0 → base, 1 → colour).
fn blend(base: Color32, rgb: [u8; 3], t: f32) -> Color32 {
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgb(mix(base.r(), rgb[0]), mix(base.g(), rgb[1]), mix(base.b(), rgb[2]))
}

fn dashed_outline(painter: &egui::Painter, r: Rect, color: Color32) {
    let stroke = egui::Stroke::new(1.0, color);
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    painter.extend(egui::Shape::dashed_line(&pts, stroke, 4.0, 3.0));
}

/// A miniature piano roll inside the bubble: x = time, y = pitch (high at the top),
/// width = note length, brightness = velocity. Pitch range fits the clip's own notes.
fn draw_clip_note_preview(
    painter: &egui::Painter,
    clip: &crate::model::NoteClip,
    clip_rect: Rect,
    _color: Color32,
) {
    let visible: Vec<&crate::model::Note> =
        clip.notes.iter().filter(|n| clip.note_is_visible(n)).collect();
    if visible.is_empty() {
        return;
    }
    // Below the name label, with a little padding.
    let area = Rect::from_min_max(
        egui::pos2(clip_rect.min.x + 3.0, clip_rect.min.y + 17.0),
        egui::pos2(clip_rect.max.x - 3.0, clip_rect.max.y - 3.0),
    );
    if area.height() < 4.0 || area.width() < 4.0 {
        return;
    }
    let lo = visible.iter().map(|n| n.pitch).min().unwrap_or(60) as f32;
    let hi = visible.iter().map(|n| n.pitch).max().unwrap_or(60) as f32;
    // At least an octave of range so a single repeated note sits mid-height.
    let span = (hi - lo).max(12.0);
    let mid = (hi + lo) * 0.5;
    let (lo, hi) = (mid - span * 0.5, mid + span * 0.5);
    let row_h = (area.height() / (span + 1.0)).clamp(1.5, 4.0);
    let vis_len = clip.visible_length().max(0.01) as f32;
    let painter = painter.with_clip_rect(area.intersect(painter.clip_rect()));

    for note in visible {
        let t0 = (note.start_beat - clip.trim_start) as f32 / vis_len;
        let t1 = (note.end_beat() - clip.trim_start) as f32 / vis_len;
        let x0 = area.min.x + t0 * area.width();
        let x1 = (area.min.x + t1 * area.width()).max(x0 + 2.0);
        let y_frac = (hi - note.pitch as f32) / (hi - lo).max(1.0);
        let y = area.min.y + y_frac * (area.height() - row_h);
        let alpha = (120.0 + 135.0 * note.velocity.clamp(0.0, 1.0)) as u8;
        painter.rect_filled(
            Rect::from_min_max(egui::pos2(x0, y), egui::pos2(x1, y + row_h)),
            1.0,
            Color32::from_rgba_unmultiplied(255, 255, 255, alpha),
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
    if body_resp.hovered() && app.clip_drag.is_none() {
        let melodic = app.project.tracks.get(track_index).map(|t| !t.is_drum()).unwrap_or(false);
        if let (true, Some(pos)) = (melodic, body_resp.hover_pos()) {
            let beat = beat_from_x(app, pos.x, grid_origin_x);
            let mut spans = Vec::new();
            crate::app::clip_spans(clip, app.project.total_beats, &mut spans);
            let chord = crate::music::chord_segments(&spans)
                .into_iter()
                .find(|(a, b, _)| beat >= *a && beat < *b);
            let text = match chord {
                Some((_, _, c)) => format!(
                    "{}: {}  ({})\nBass note: {}",
                    clip.name,
                    c.name,
                    c.tone_names(),
                    crate::music::NOTE_NAMES[c.root as usize]
                ),
                None => clip.name.clone(),
            };
            body_resp.clone().on_hover_text_at_pointer(text);
        }
    }
    if trim_start_resp.hovered() || trim_end_resp.hovered() || loop_resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }

    if body_resp.clicked() {
        let add = ui.input(|i| i.modifiers.command || i.modifiers.shift);
        if add {
            app.toggle_clip_selected(track_index, clip_idx);
        } else {
            app.select_single_clip(track_index, clip_idx);
        }
    }
    body_resp.context_menu(|ui| clip_context_menu(app, ui, track_index, clip_idx));

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

    // Magnet: an edge within ~10 px of the playhead lands exactly on it.
    let playhead = app.project.playhead_beat;
    let snap_on = app.snap_clips_to_playhead && !ui.input(|i| i.modifiers.alt);
    let reach = 10.0 / beat_width(app).max(1.0) as f64;
    let near = |b: f64| snap_on && (b - playhead).abs() <= reach;
    let mut snapped = false;

    if let Some(clip) = app
        .project
        .tracks
        .get_mut(track_idx)
        .and_then(|t| t.clips.get_mut(clip_idx))
    {
        match drag.mode {
            ClipDragMode::Move => {
                let mut new_start = (beat - drag.grab_beat).max(0.0);
                let len = clip.visible_length();
                if near(new_start) {
                    new_start = playhead;
                    snapped = true;
                } else if near(new_start + len) && playhead - len >= 0.0 {
                    new_start = playhead - len;
                    snapped = true;
                }
                let delta = new_start - clip.start_beat;
                clip.offset_on_timeline(delta);
            }
            ClipDragMode::TrimStart => {
                let rel = (beat - clip.start_beat).max(0.0);
                clip.set_trim_start(rel.min(clip.source_length - clip.trim_end - 0.25));
            }
            ClipDragMode::TrimEnd => {
                let beat = if near(beat) {
                    snapped = true;
                    playhead
                } else {
                    beat
                };
                let rel_end = (beat - clip.start_beat).max(clip.trim_start + 0.25);
                clip.set_trim_end((clip.source_length - rel_end).max(0.0));
            }
            ClipDragMode::ExtendLoop => {
                let beat = if near(beat) {
                    snapped = true;
                    playhead
                } else {
                    beat
                };
                clip.loop_enabled = true;
                clip.set_loop_end(beat.max(clip.start_beat + clip.visible_length()));
            }
        }
    }
    app.clip_snapped = snapped;
}

/// Right-click menu on a clip bubble. Right-clicking a clip outside the selection
/// selects just that clip first, like most editors.
fn clip_context_menu(app: &mut DAWApp, ui: &mut Ui, track_index: usize, clip_idx: usize) {
    let in_selection = app.current_track == track_index
        && app.selected_clip_indices().contains(&clip_idx);
    if !in_selection {
        app.select_single_clip(track_index, clip_idx);
    }
    let n = app.selected_clip_indices().len();
    let what = if n == 1 { "clip".to_string() } else { format!("{n} clips") };
    if ui.button(format!("Copy {what}  (Ctrl+C)")).clicked() {
        if let Some(json) = app.copy_selected_clips() {
            ui.ctx().copy_text(json);
        }
        ui.close_menu();
    }
    if ui
        .add_enabled(app.clip_clipboard.is_some(), egui::Button::new("Paste at playhead  (Ctrl+V)"))
        .clicked()
    {
        app.paste_clips_at_playhead();
        ui.close_menu();
    }
    if ui.button(format!("Duplicate {what}  (Ctrl+D)")).clicked() {
        app.duplicate_selected_clips();
        ui.close_menu();
    }
    if ui.button(format!("Move {what} to playhead")).clicked() {
        app.move_selected_clips_to_playhead();
        ui.close_menu();
    }
    if ui
        .add_enabled(n >= 2, egui::Button::new(format!("Merge {what} into one")))
        .on_disabled_hover_text("Ctrl+click or Shift+click more clips to merge them")
        .clicked()
    {
        app.merge_selected_clips();
        ui.close_menu();
    }
    if ui.button("Select all clips on track").clicked() {
        app.select_all_clips();
        ui.close_menu();
    }
    ui.separator();
    if ui.button(format!("Delete {what}")).clicked() {
        app.delete_selected_clips();
        ui.close_menu();
    }
}

/// Left-column controls for an open automation lane: which parameter, clear, close.
fn draw_lane_header(app: &mut DAWApp, ui: &mut Ui, rect: Rect, index: usize) {
    ui.painter_at(rect).rect_filled(rect, 0.0, Color32::from_rgb(28, 29, 34));
    let Some(param) = app.project.tracks[index].lane_view else { return };
    let inner = rect.shrink2(Vec2::new(8.0, 6.0));
    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)), |ui| {
        ui.set_clip_rect(inner.intersect(ui.clip_rect()));
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 3.0);
        let mut choice = param;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(ph::CHART_LINE).color(dj_widgets::AUTOMATED));
            egui::ComboBox::from_id_salt(("lane_param", index))
                .width(TRACK_HEADER_WIDTH - 44.0)
                .selected_text(egui::RichText::new(param.label()).small())
                .show_ui(ui, |ui| {
                    for &p in FxParam::all() {
                        let has = app.project.tracks[index].lane(p).is_some();
                        let label = if has { format!("{} ●", p.label()) } else { p.label().to_string() };
                        ui.selectable_value(&mut choice, p, label);
                    }
                });
        });
        ui.horizontal(|ui| {
            let has = app.project.tracks[index].lane(param).is_some();
            if ui
                .add_enabled(has, egui::Button::new(egui::RichText::new(format!("{} Clear", ph::TRASH)).small()))
                .on_hover_text("Remove every point on this lane (the knob keeps its current value)")
                .clicked()
            {
                app.dj_clear_automation(crate::dj::DjTarget::Track(index, param));
            }
            if ui
                .button(egui::RichText::new(format!("{} Hide", ph::X)).small())
                .clicked()
            {
                app.project.tracks[index].lane_view = None;
            }
        });
        if choice != param {
            app.project.tracks[index].lane_view = Some(choice);
        }
    });
}

/// An automation lane: the curve over time for one parameter. Click to add a point,
/// drag to move it, double- or right-click to delete it.
fn draw_lane(
    app: &mut DAWApp,
    ui: &mut Ui,
    painter: &egui::Painter,
    rect: Rect,
    index: usize,
    origin_x: f32,
) {
    let Some(param) = app.project.tracks[index].lane_view else { return };
    let color = {
        let c = app.project.tracks[index].color;
        Color32::from_rgb(c[0], c[1], c[2])
    };
    painter.rect_filled(rect, 0.0, Color32::from_rgb(26, 27, 32));
    painter.line_segment(
        [rect.left_top(), rect.right_top()],
        egui::Stroke::new(1.0, Color32::from_rgb(50, 52, 60)),
    );
    let (lo, hi) = param.range();
    let area = rect.shrink2(Vec2::new(0.0, 6.0));
    let y_for = |v: f32| area.max.y - (v - lo) / (hi - lo) * area.height();
    let v_for = |y: f32| param.clamp(lo + (area.max.y - y) / area.height() * (hi - lo));
    if param.bipolar() || param == FxParam::Volume {
        let y = y_for(param.neutral());
        painter.extend(egui::Shape::dashed_line(
            &[egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
            egui::Stroke::new(1.0, Color32::from_rgb(60, 62, 72)),
            4.0,
            4.0,
        ));
    }

    let points: Vec<(f64, f32)> = app.project.tracks[index]
        .lane(param)
        .map(|l| l.points.iter().map(|p| (p.beat, p.value)).collect())
        .unwrap_or_default();
    let bw = beat_width(app);
    let pos_of = |b: f64, v: f32| egui::pos2(origin_x + b as f32 * bw, y_for(v));
    let beat_at = |x: f32| ((x - origin_x) / bw).max(0.0) as f64;

    if points.is_empty() {
        let v = app.project.tracks[index].static_value(param);
        painter.line_segment(
            [egui::pos2(rect.min.x, y_for(v)), egui::pos2(rect.max.x, y_for(v))],
            egui::Stroke::new(1.0, Color32::from_rgb(110, 112, 124)),
        );
        painter.text(
            egui::pos2(rect.min.x + 8.0, rect.min.y + 6.0),
            egui::Align2::LEFT_TOP,
            format!(
                "{} — fixed at {}. Click to add points, or arm WRITE in DJ mode and turn the knob while playing.",
                param.label(),
                param.describe(v)
            ),
            theme::font(theme::FONT_SM),
            Color32::from_rgb(140, 142, 154),
        );
    } else {
        let curve = dj_widgets::mix(Color32::from_rgb(26, 27, 32), color, 0.9);
        let mut line = vec![egui::pos2(rect.min.x, y_for(points[0].1))];
        line.extend(points.iter().map(|&(b, v)| pos_of(b, v)));
        line.push(egui::pos2(rect.max.x, y_for(points[points.len() - 1].1)));
        // Filled area under the curve (opaque, darker) for readability.
        for pair in line.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if b.x - a.x < 0.5 {
                continue;
            }
            painter.add(egui::Shape::convex_polygon(
                vec![a, b, egui::pos2(b.x, area.max.y), egui::pos2(a.x, area.max.y)],
                dj_widgets::mix(Color32::from_rgb(26, 27, 32), color, 0.18),
                egui::Stroke::NONE,
            ));
        }
        painter.add(egui::Shape::line(line, egui::Stroke::new(2.0, curve)));
    }

    // Interaction.
    let id = ui.id().with(("lane", index));
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
    let near = |p: egui::Pos2| -> Option<usize> {
        points
            .iter()
            .enumerate()
            .map(|(i, &(b, v))| (i, pos_of(b, v).distance(p)))
            .filter(|(_, d)| *d < 8.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
    };
    let hovered_point = pointer.and_then(near);
    let drag_key = id.with("drag");
    let mut dragging: Option<usize> = ui.data(|d| d.get_temp(drag_key)).flatten();
    let snap = |b: f64| ((b / LANE_SNAP).round() * LANE_SNAP).max(0.0);

    if resp.drag_started() {
        if let Some(p) = pointer {
            dragging = Some(match near(p) {
                Some(i) => i,
                None => app.project.tracks[index]
                    .lane_mut(param)
                    .set_point(snap(beat_at(p.x)), v_for(p.y)),
            });
        }
    }
    if resp.dragged() {
        if let (Some(i), Some(p)) = (dragging, pointer) {
            let beat = snap(beat_at(p.x));
            app.project.tracks[index].lane_mut(param).move_point(i, beat, v_for(p.y));
        }
    }
    if resp.drag_stopped() {
        dragging = None;
    }
    ui.data_mut(|d| d.insert_temp(drag_key, dragging));

    let delete = resp.double_clicked() || resp.secondary_clicked();
    if delete {
        if let Some(i) = hovered_point {
            let lane = app.project.tracks[index].lane_mut(param);
            lane.remove_point(i);
            if lane.is_empty() {
                app.project.tracks[index].clear_lane(param);
            }
        }
    } else if resp.clicked() && hovered_point.is_none() {
        if let Some(p) = pointer {
            app.project.tracks[index]
                .lane_mut(param)
                .set_point(snap(beat_at(p.x)), v_for(p.y));
        }
    }

    // Points (drawn after edits so they track the cursor this frame).
    if let Some(lane) = app.project.tracks[index].lane(param) {
        for (i, p) in lane.points.iter().enumerate() {
            let hot = hovered_point == Some(i) || dragging == Some(i);
            let c = pos_of(p.beat, p.value);
            painter.circle_filled(c, if hot { 5.5 } else { 3.5 }, if hot { Color32::WHITE } else { color });
            painter.circle_stroke(c, if hot { 5.5 } else { 3.5 }, egui::Stroke::new(1.0, Color32::from_rgb(16, 16, 20)));
        }
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(if hovered_point.is_some() { CursorIcon::Grab } else { CursorIcon::Crosshair });
        if let Some(p) = pointer {
            let beat = beat_at(p.x);
            resp.on_hover_text(format!(
                "{} at beat {:.2}: {}\nClick = add point · drag = move · double/right-click = delete",
                param.label(),
                beat,
                param.describe(app.project.tracks[index].value_at(param, beat))
            ));
        }
    }
}
