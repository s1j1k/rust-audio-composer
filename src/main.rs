use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
mod ui;

struct DAWApp {
    playing: bool,
    current_track: usize,
    volume: f32,
    show_piano: bool
}

impl Default for DAWApp {
    fn default() -> Self {
        Self {
            playing: false,
            current_track: 0,
            volume: 0.8,
            show_piano: false,
        }
    }
}


impl eframe::App for DAWApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // Transport controls
            ui.horizontal(|ui| {
                if ui.button(if self.playing { "⏸" } else { "▶" }).clicked() {
                    self.playing = !self.playing;
                }

                // TODO add functionality
                ui.button("⏹");

                // ui.button("⏮");
                // ui.button("⏭");

                // TODO add record button
                // TODO enable emoji symbol button
                ui.button("⏺");
                // TODO make a virtual piano spawn

                // ui.button("🎹");
                if ui.button("🎹").clicked() {
                    self.show_piano = !self.show_piano;
                }

                // TODO add instruments/midi selection
                // TODO add piano input button
                // TODO add manual piano input thing
                
                ui.separator();
                
                // BPM control
                ui.label("BPM:");
                ui.add(egui::Slider::new(&mut 120.0_f32, 60.0..=200.0));
            });

            self.show_piano_window(ctx);
        });

        egui::SidePanel::left("track_panel")
            .resizable(true)
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Tracks");
                ui.separator();
                
                for i in 0..4 {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.current_track, i, format!("Track {}", i + 1));
                        ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0));
                    });
                }
                
                ui.separator();
                if ui.button("Add Track").clicked() {
                    // Add track logic
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Waveform display
            Plot::new("waveform")
                .height(200.0)
                .show(ui,
                    |plot_ui| {
                    let points: PlotPoints = (0..100)
                        .map(|i| {
                            let x = i as f64 / 10.0;
                            [x, (x * 5.0).sin()]
                        })
                        .collect();
                    plot_ui.line(Line::new(points));
                });
            
            // Piano roll or timeline
            ui.separator();
            ui.heading("Piano Roll");
            // Add piano roll implementation
        });
    }
}


fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        // initial_window_size: Some(egui::vec2(1200.0, 800.0)),
        ..Default::default()
    };
    
    eframe::run_native(
        "Audio Composer",
        options,
        Box::new(|_cc| Ok(Box::new(DAWApp::default()))),
    )
}

// TODO add piano on-screen input