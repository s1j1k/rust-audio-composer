use crate::app::DAWApp;
use crate::ui::hints::{self, text};
use crate::ui::theme::{self, UiFontFamily};
use egui::{Context, Ui};

pub fn show_options(app: &mut DAWApp, ui: &mut Ui, ctx: &Context) {
    ui.heading("Options");
    ui.separator();

    ui.horizontal(|ui| {
        ui.label("Interface font");
        hints::bubble(
            ui,
            "options_font",
            "Code uses a monospace face for a technical look. Sans serif uses the default proportional UI font.",
        );
    });
    let prev_font = app.ui_font;
    egui::ComboBox::from_id_salt("ui_font_family")
        .selected_text(app.ui_font.label())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut app.ui_font, UiFontFamily::Code, UiFontFamily::Code.label());
            ui.selectable_value(
                &mut app.ui_font,
                UiFontFamily::SansSerif,
                UiFontFamily::SansSerif.label(),
            );
        });

    if app.ui_font != prev_font {
        theme::apply(ctx, app.ui_font);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Interface size");
        hints::bubble(
            ui,
            "options_ui_scale",
            "Scales all text and controls. Ctrl + / Ctrl − / Ctrl 0 also work anywhere.",
        );
    });
    let mut scale = ctx.zoom_factor();
    ui.horizontal(|ui| {
        if ui
            .add(egui::Slider::new(&mut scale, 0.7..=1.5).custom_formatter(|v, _| format!("{:.0}%", v * 100.0)))
            .changed()
        {
            ctx.set_zoom_factor(scale);
        }
        if ui.small_button("Reset").clicked() {
            ctx.set_zoom_factor(theme::DEFAULT_UI_SCALE);
        }
    });

    ui.add_space(8.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Inline help");
        hints::bubble(ui, "options_inline_help", text::UI_HINTS);
    });
    ui.checkbox(&mut app.ui_hints_expanded, "Show help text under section headers");

    ui.add_space(4.0);
    if ui.button("Open User Guide").clicked() {
        app.show_user_guide = true;
    }
}
