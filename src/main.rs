mod ai;
mod app;
mod audio;
mod history;
mod library;
mod model;
mod music;
mod remote;
mod ui;

use app::DAWApp;

fn main() -> Result<(), eframe::Error> {
    // Window / taskbar icon: a cursive "Ch" (assets/icon.png). See CREDITS.md.
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_title("ChorDAW");
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "ChorDAW",
        options,
        Box::new(|cc| {
            ui::theme::apply(&cc.egui_ctx, ui::theme::UiFontFamily::default());
            cc.egui_ctx.set_zoom_factor(ui::theme::DEFAULT_UI_SCALE);
            Ok(Box::new(DAWApp::default()))
        }),
    )
}
