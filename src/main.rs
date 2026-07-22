mod ai;
mod app;
mod audio;
mod model;
mod music;
mod ui;

use app::DAWApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Rust Audio Composer"),
        ..Default::default()
    };

    eframe::run_native(
        "Rust Audio Composer",
        options,
        Box::new(|_cc| Ok(Box::new(DAWApp::default()))),
    )
}
