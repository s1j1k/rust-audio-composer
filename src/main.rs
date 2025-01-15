use eframe::egui;
// use egui::plot::{Line, Plot, PlotPoints};

struct DAWApp {
    playing: bool,
    current_track: usize,
    volume: f32,
}

impl Default for DAWApp {
    fn default() -> Self {
        Self {
            playing: false,
            current_track: 0,
            volume: 0.8,
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
                ui.button("⏹");
                ui.button("⏮");
                ui.button("⏭");
                
                ui.separator();
                
                // BPM control
                ui.label("BPM:");
                ui.add(egui::Slider::new(&mut 120.0_f32, 60.0..=200.0));
            });
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
            // Plot::new("waveform")
            //     .height(200.0)
            //     .show(ui, |plot_ui| {
            //         let points: PlotPoints = (0..100)
            //             .map(|i| {
            //                 let x = i as f64 / 10.0;
            //                 [x, (x * 5.0).sin()]
            //             })
            //             .collect();
            //         plot_ui.line(Line::new(points));
            //     });
            
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
        Box::new(|_cc| Box::new(DAWApp::default())),
    )
}