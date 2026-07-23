use crate::app::DAWApp;
use crate::model::drum::{DrumKind, SEQUENCER_STEPS, step_to_beat};
use crate::model::note::Note;
use egui::{Color32, Ui, Vec2};

const CELL: f32 = 28.0;

pub fn show_drum_sequencer(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("🥁 Beat Sequencer");
    ui.label(
        "Click steps to toggle hits. Shift+click = accent (louder). Right-click removes. \
         Samples: FreePats Synthesizer Percussion (CC0).",
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
        }
        return;
    }

    let beats_per_bar = app.project.time_sig_numerator as f64;
    let steps = SEQUENCER_STEPS;
    let playhead_step = ((app.project.playhead_beat / beats_per_bar) * steps as f64).floor() as u8
        % steps;

    ui.horizontal(|ui| {
        ui.label(format!("Track: {}", app.project.tracks[track_idx].name));
        ui.separator();
        ui.label(format!("{} steps · {} BPM", steps, app.project.bpm as u32));
    });

    ui.add_space(4.0);

    let grid_width = CELL * steps as f32 + 100.0;
    let grid_height = CELL * DrumKind::all().len() as f32 + 24.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(grid_width, grid_height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let origin = rect.min + Vec2::new(90.0, 20.0);

    for s in 0..steps {
        let x = origin.x + s as f32 * CELL;
        let is_play = s == playhead_step && app.playing;
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
            egui::FontId::proportional(10.0),
            label_color,
        );
    }

    for (row, &drum) in DrumKind::all().iter().enumerate() {
        let y = origin.y + row as f32 * CELL;
        painter.text(
            egui::pos2(rect.min.x + 4.0, y + CELL * 0.5),
            egui::Align2::LEFT_CENTER,
            format!("{} {}", drum.icon(), drum.label()),
            egui::FontId::proportional(12.0),
            Color32::WHITE,
        );

        for step in 0..steps {
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(origin.x + step as f32 * CELL, y),
                Vec2::new(CELL - 2.0, CELL - 2.0),
            );
            let hit = drum_hit_at(&app.project.tracks[track_idx], drum, step, beats_per_bar, steps);
            let vel = hit.map(|n| n.velocity).unwrap_or(0.0);
            let active = hit.is_some();

            let base = if step == playhead_step {
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
        if ui.button("Preview step row").on_hover_text("Click a drum row label area to preview").clicked() {
            app.preview_drum(DrumKind::Kick);
        }
    });

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

fn drum_hit_at(
    track: &crate::model::Track,
    drum: DrumKind,
    step: u8,
    beats_per_bar: f64,
    steps: u8,
) -> Option<&Note> {
    let beat = step_to_beat(step, beats_per_bar, steps);
    let pitch = drum.midi_pitch();
    track.notes.iter().find(|n| {
        n.pitch == pitch && (n.start_beat - beat).abs() < 0.001
    })
}
