use egui::{FontFamily, FontId, TextStyle, Visuals};
use std::collections::BTreeMap;

/// Compact typography scale for labels drawn with `Painter`.
pub const FONT_XS: f32 = 7.5;
pub const FONT_SM: f32 = 8.5;
pub const FONT_MD: f32 = 9.5;
pub const FONT_LG: f32 = 10.5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiFontFamily {
    #[default]
    Code,
    SansSerif,
}

impl UiFontFamily {
    pub fn label(self) -> &'static str {
        match self {
            Self::Code => "Code (monospace)",
            Self::SansSerif => "Sans serif",
        }
    }

    fn proportional_stack(self) -> Vec<String> {
        match self {
            Self::Code => vec![
                "Hack".to_owned(),
                "Ubuntu-Light".to_owned(),
                "NotoEmoji-Regular".to_owned(),
                "emoji-icon-font".to_owned(),
            ],
            Self::SansSerif => vec![
                "Ubuntu-Light".to_owned(),
                "NotoEmoji-Regular".to_owned(),
                "emoji-icon-font".to_owned(),
            ],
        }
    }
}

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

/// Apply global fonts, text sizes, and spacing.
pub fn apply(ctx: &egui::Context, ui_font: UiFontFamily) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .families
        .insert(FontFamily::Proportional, ui_font.proportional_stack());

    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.text_styles = text_styles();
    style.spacing.item_spacing = egui::vec2(5.0, 3.0);
    style.spacing.button_padding = egui::vec2(5.0, 2.0);
    style.spacing.window_margin = egui::Margin::same(6.0);
    style.spacing.indent = 14.0;
    style.visuals = compact_visuals(style.visuals);

    ctx.set_style(style);
}

fn text_styles() -> BTreeMap<TextStyle, FontId> {
    BTreeMap::from([
        (TextStyle::Small, FontId::new(9.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(11.0, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(10.0, FontFamily::Monospace)),
    ])
}

fn compact_visuals(mut visuals: Visuals) -> Visuals {
    visuals.window_rounding = egui::Rounding::same(4.0);
    visuals.menu_rounding = egui::Rounding::same(3.0);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(2.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(2.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(2.0);
    visuals.widgets.active.rounding = egui::Rounding::same(2.0);
    visuals.widgets.open.rounding = egui::Rounding::same(2.0);
    visuals
}
