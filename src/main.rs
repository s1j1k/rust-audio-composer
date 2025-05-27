use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::sync::{Arc, Mutex};

mod ui;
mod audio;
use audio::oscillator::Oscillator;

struct DAWApp {
    playing: bool,
    current_track: usize,
    volume: f32,
    show_piano: bool,
    oscillator: Oscillator,
    audio_stream: Option<Stream>,
}

impl Default for DAWApp {
    fn default() -> Self {
        let mut app = Self {
            playing: false,
            current_track: 0,
            volume: 0.8,
            show_piano: false,
            oscillator: Oscillator::default(),
            audio_stream: None,
        };

        // Setup audio stream
        if let Ok(stream) = app.setup_audio_stream() {
            app.audio_stream = Some(stream);
        }

        app
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
                // Stop button
                if ui.button("⏹").clicked() {
                    self.oscillator.stop();
                }

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

                // FIXME remove -for testing only
                // Frequency control
                // ui.label("Frequency:");
                // let mut frequency = self.oscillator.frequency();
                // if ui.add(egui::Slider::new(&mut frequency, 20.0..=20000.0)).changed() {
                //     self.oscillator.set_frequency(frequency);
                // }
                // // Amplitude control
                // ui.label("Amplitude:");
                // let mut amplitude = self.oscillator.amplitude();
                // if ui.add(egui::Slider::new(&mut amplitude, 0.0..=1.0)).changed() {
                //     self.oscillator.set_amplitude(amplitude);
                // }

                // TODO add instruments/midi selection
                // TODO add piano input button
                // TODO add manual piano input thing

                // TODO add piano roll

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
            Plot::new("waveform").height(200.0).show(ui, |plot_ui| {
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

// TODO move to other file
impl DAWApp {
    fn setup_audio_stream(&mut self) -> Result<Stream, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");

        let config = device.default_output_config()?;
        let sample_rate = config.sample_rate().0 as f32;

        let oscillator = Arc::new(Mutex::new(Oscillator::new(sample_rate)));
        let osc_clone = oscillator.clone();

        let stream = device.build_output_stream(
            &config.config(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(mut osc) = osc_clone.lock() {
                    for sample in data.iter_mut() {
                        *sample = osc.next_sample() * 100.0; // FIXME try to multiple by volume
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            Some(std::time::Duration::from_secs(1000000000)), // Add timeout configuration
        )?;

        stream.play()?;
        Ok(stream)
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
