use crate::app::DAWApp;
use crate::model::drum::{DrumKind, SEQUENCER_STEPS};
use crate::ui::theme;
use egui::{Color32, Ui, Vec2};

const CELL: f32 = 28.0;

pub fn show_drum_sequencer(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("🥁 Beat Sequencer");
    ui.label(
        "Pattern loops while you edit · steps update the preview immediately · \
         press Record to write changes into a clip on the timeline.",
    );
    ui.separator();

    let track_idx = app.current_track;
    if track_idx >= app.project.tracks.len() {
        ui.label("No track selected.");
        return;
    }

    if !app.project.tracks[track_idx].is_drum() {
        ui.label("Select or add a drum track to use the beat sequencer.");
        if ui.button("+ Add Drum Track").clicked() {
            app.project.add_drum_track();
            app.current_track = app.project.tracks.len() - 1;
            app.sync_instrument_to_track();
            app.refresh_sequencer_draft();
        }
        return;
    }

    if app.sequencer_draft_track != Some(track_idx) {
        app.refresh_sequencer_draft();
    }

    let beats_per_bar = app.project.beats_per_bar();
    let steps = SEQUENCER_STEPS;
    let playhead_step = if app.sequencer_running {
        ((app.sequencer_loop_beat / beats_per_bar) * steps as f64)
            .floor()
            .clamp(0.0, (steps - 1) as f64) as u8
    } else {
        ((app.project.playhead_beat / beats_per_bar) * steps as f64).floor() as u8 % steps
    };

    ui.horizontal(|ui| {
        ui.label(format!("Track: {}", app.project.tracks[track_idx].name));
        ui.separator();
        ui.label(format!("{} steps · {} BPM", steps, app.project.bpm as u32));
        ui.separator();
        let run_label = if app.sequencer_running {
            "⏸ Pause loop"
        } else {
            "▶ Run loop"
        };
        if ui.button(run_label).clicked() {
            app.toggle_sequencer_run();
        }
        if ui.button("↺ Reload bar").on_hover_text("Load pattern from playhead bar on timeline").clicked() {
            app.refresh_sequencer_draft();
        }
        if app.recording {
            ui.colored_label(Color32::from_rgb(255, 60, 60), "⏺ Recording to clip");
        } else {
            ui.label(
                egui::RichText::new("Preview only — not on timeline until Record")
                    .small()
                    .color(Color32::GRAY),
            );
        }
    });

    ui.add_space(4.0);

    let grid_width = CELL * steps as f32 + 100.0;
    let grid_height = CELL * DrumKind::all().len() as f32 + 24.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(grid_width, grid_height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let origin = rect.min + Vec2::new(90.0, 20.0);

    for s in 0..steps {
        let x = origin.x + s as f32 * CELL;
        let is_play = s == playhead_step && app.sequencer_running;
        let label_color = if is_play {
            Color32::from_rgb(255, 120, 80)
        } else if s % 4 == 0 {
            Color32::LIGHT_GRAY
        } else {
            Color32::GRAY
        };
        painter.text(
            egui::pos2(x + CELL * 0.5, rect.min.y + 8.0),
            egui::Align2::CENTER_CENTER,
            format!("{}", s + 1),
            theme::font(theme::FONT_MD),
            label_color,
        );
    }

    for (row, &drum) in DrumKind::all().iter().enumerate() {
        let y = origin.y + row as f32 * CELL;
        painter.text(
            egui::pos2(rect.min.x + 4.0, y + CELL * 0.5),
            egui::Align2::LEFT_CENTER,
            format!("{} {}", drum.icon(), drum.label()),
            theme::font(theme::FONT_LG),
            Color32::WHITE,
        );

        for step in 0..steps {
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(origin.x + step as f32 * CELL, y),
                Vec2::new(CELL - 2.0, CELL - 2.0),
            );
            let vel = app.sequencer_draft_hit(drum, step);
            let active = vel.is_some();
            let vel = vel.unwrap_or(0.0);

            let base = if step == playhead_step && app.sequencer_running {
                Color32::from_rgba_unmultiplied(255, 100, 60, 40)
            } else if step % 4 == 0 {
                Color32::from_rgb(45, 45, 55)
            } else {
                Color32::from_rgb(35, 35, 42)
            };
            painter.rect_filled(cell_rect, 3.0, base);

            if active {
                let intensity = (vel * 255.0) as u8;
                let color = Color32::from_rgb(
                    255,
                    (180.0 + vel * 75.0) as u8,
                    intensity / 2,
                );
                let pad = 4.0 + (1.0 - vel) * 4.0;
                painter.rect_filled(cell_rect.shrink(pad), 2.0, color);
            }

            let response = ui.interact(cell_rect, ui.id().with((drum, step)), egui::Sense::click());
            if response.clicked() {
                let accent = ui.input(|i| i.modifiers.shift);
                app.toggle_drum_step(track_idx, drum, step, beats_per_bar, steps, accent);
            }
            if response.secondary_clicked() {
                app.clear_drum_step(track_idx, drum, step, beats_per_bar, steps);
            }
        }
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Clear pattern").clicked() {
            app.clear_drum_pattern(track_idx);
        }
        if ui.button("Preview kick").clicked() {
            app.preview_drum(DrumKind::Kick);
        }
    });

    ui.separator();
    ui.heading("Pattern library");
    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut app.drum_pattern_name_input);
        if ui.button("💾 Save pattern").clicked() {
            app.save_drum_pattern(track_idx);
        }
    });

    if app.saved_drum_patterns.is_empty() {
        ui.label("No saved patterns yet — build a bar and save it.");
    } else {
        ui.horizontal(|ui| {
            ui.label("Insert:");
            let names: Vec<String> = app
                .saved_drum_patterns
                .iter()
                .map(|p| p.name.clone())
                .collect();
            egui::ComboBox::from_id_salt("drum_pattern_picker")
                .selected_text(
                    names
                        .get(app.selected_pattern_index)
                        .cloned()
                        .unwrap_or_else(|| "Select…".to_string()),
                )
                .show_ui(ui, |ui| {
                    for (i, name) in names.iter().enumerate() {
                        ui.selectable_value(&mut app.selected_pattern_index, i, name);
                    }
                });
            if ui
                .button("Insert at playhead")
                .on_hover_text("Paste saved pattern at current playhead bar")
                .clicked()
            {
                let beat = app.sequencer_bar_start();
                app.insert_drum_pattern(track_idx, app.selected_pattern_index, beat);
                app.refresh_sequencer_draft();
            }
            if ui.button("Insert at bar start").clicked() {
                let beat = app.sequencer_bar_start();
                app.insert_drum_pattern(track_idx, app.selected_pattern_index, beat);
                app.refresh_sequencer_draft();
            }
        });
        for (i, pat) in app.saved_drum_patterns.clone().iter().enumerate() {
            let pat_name = pat.name.clone();
            let hit_count = pat.notes.len();
            ui.horizontal(|ui| {
                ui.label(format!("{} — {} hits", pat_name, hit_count));
                if ui.button("Insert").clicked() {
                    app.insert_drum_pattern(track_idx, i, app.sequencer_bar_start());
                    app.refresh_sequencer_draft();
                }
            });
        }
    }

    ui.add_space(6.0);
    ui.collapsing("Drum sample credits (CC0)", |ui| {
        for src in crate::audio::all_drum_sources() {
            ui.horizontal(|ui| {
                ui.label(format!("{} {}", src.kind.icon(), src.kind.label()));
                ui.label(format!("({})", src.license));
            });
        }
        ui.hyperlink_to(
            "FreePats Synthesizer Percussion",
            "https://github.com/freepats/synthesizer-percussion",
        );
    });
}
