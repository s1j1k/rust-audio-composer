use crate::app::DAWApp;
use egui::Ui;

pub fn show_dj_panel(app: &mut DAWApp, ui: &mut Ui) {
    ui.heading("🎛 DJ Tools");
    ui.label(
        "Mix effects for the whole song and individual tracks — distance, underwater muffling, \
         reverb, and phaser swirl (GarageBand-style).",
    );
    ui.separator();

    ui.heading("Master (whole mix)");
    show_effects_sliders(
        ui,
        &mut app.audio.master_effects.distance,
        &mut app.audio.master_effects.reverb,
        &mut app.audio.master_effects.phaser,
        &mut app.audio.master_effects.phaser_rate,
        &mut app.audio.master_effects.lowpass,
    );

    ui.add_space(12.0);
    ui.heading("Selected track");

    let idx = app.current_track;
    if idx < app.project.tracks.len() {
        let track_name = app.project.tracks[idx].name.clone();
        ui.label(format!(
            "{} {}",
            app.project.tracks[idx].instrument.icon(),
            track_name
        ));

        let fx = &mut app.project.tracks[idx].effects;
        show_effects_sliders(
            ui,
            &mut fx.distance,
            &mut fx.reverb,
            &mut fx.phaser,
            &mut fx.phaser_rate,
            &mut fx.lowpass,
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Reset track FX").clicked() {
                app.project.tracks[idx].effects = Default::default();
            }
            if ui.button("Reset master FX").clicked() {
                app.audio.master_effects = Default::default();
            }
        });
    } else {
        ui.label("Select a track to edit its effects.");
    }

    ui.add_space(8.0);
    ui.collapsing("Effect guide", |ui| {
        ui.label("Distance — pushes sound far away / underwater (auto low-pass + reverb).");
        ui.label("Reverb — space and depth on top of distance.");
        ui.label("Phaser — swirling phase movement (fade in/out character).");
        ui.label("Brightness — manual low-pass; lower = more muffled.");
    });
}

fn show_effects_sliders(
    ui: &mut Ui,
    distance: &mut f32,
    reverb: &mut f32,
    phaser: &mut f32,
    phaser_rate: &mut f32,
    lowpass: &mut f32,
) {
    ui.add(
        egui::Slider::new(distance, 0.0..=1.0)
            .text("Distance / underwater")
            .smart_aim(false),
    );
    ui.add(egui::Slider::new(reverb, 0.0..=1.0).text("Reverb"));
    ui.add(egui::Slider::new(phaser, 0.0..=1.0).text("Phaser swirl"));
    ui.add(
        egui::Slider::new(phaser_rate, 0.05..=3.0)
            .text("Phaser speed (Hz)")
            .logarithmic(true),
    );
    ui.add(
        egui::Slider::new(lowpass, 0.0..=1.0)
            .text("Brightness (low-pass)")
            .smart_aim(false),
    );
}
