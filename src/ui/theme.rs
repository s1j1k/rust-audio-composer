use egui::{FontFamily, FontId, TextStyle, Visuals};
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// Compact typography scale for labels drawn with `Painter`.
pub const FONT_XS: f32 = 8.5;
pub const FONT_SM: f32 = 9.5;
pub const FONT_MD: f32 = 10.5;
pub const FONT_LG: f32 = 11.5;

/// Default interface zoom. Slightly under 100% keeps the layout airy on high-DPI laptops,
/// where Windows display scaling (often 150%) already enlarges everything.
pub const DEFAULT_UI_SCALE: f32 = 0.9;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiFontFamily {
    Code,
    #[default]
    SansSerif,
}

impl UiFontFamily {
    pub fn label(self) -> &'static str {
        match self {
            Self::Code => "Code (monospace)",
            Self::SansSerif => "Sans serif",
        }
    }

    fn proportional_stack(self, system_sans: bool) -> Vec<String> {
        let mut stack = Vec::new();
        match self {
            Self::Code => stack.push("Hack".to_owned()),
            Self::SansSerif => {
                if system_sans {
                    stack.push(SYSTEM_SANS.to_owned());
                }
            }
        }
        stack.extend(
            ["Ubuntu-Light", "NotoEmoji-Regular", "emoji-icon-font"]
                .iter()
                .map(|s| s.to_string()),
        );
        stack
    }
}

const SYSTEM_SANS: &str = "system-sans";
const PHOSPHOR: &str = "phosphor";

/// The platform's own UI typeface (Segoe UI, Helvetica/SF, Noto/DejaVu Sans), read once from
/// disk. egui's bundled Ubuntu-Light is the fallback, so a missing file is harmless.
fn system_sans_font() -> Option<&'static [u8]> {
    static FONT: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    FONT.get_or_init(|| {
        let candidates: &[&str] = &[
            r"C:\Windows\Fonts\segoeui.ttf",
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
        ];
        candidates.iter().find_map(|p| std::fs::read(p).ok())
    })
    .as_deref()
}

/// Proportional (sans) font for painter-drawn labels.
pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

/// Monospace font, used where fixed-width glyphs read better (piano key labels).
pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

/// Apply global fonts, text sizes, and spacing.
pub fn apply(ctx: &egui::Context, ui_font: UiFontFamily) {
    let mut fonts = egui::FontDefinitions::default();
    let system = system_sans_font();
    if let Some(bytes) = system {
        fonts.font_data.insert(
            SYSTEM_SANS.to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(bytes)),
        );
    }
    // Phosphor icons (MIT) for track icons and DJ controls; glyphs live in the
    // private-use area so they never shadow normal text.
    fonts.font_data.insert(
        PHOSPHOR.to_owned(),
        std::sync::Arc::new(egui_phosphor::Variant::Regular.font_data()),
    );
    let mut stack = ui_font.proportional_stack(system.is_some());
    stack.insert(1.min(stack.len()), PHOSPHOR.to_owned());
    fonts.families.insert(FontFamily::Proportional, stack);

    ctx.set_fonts(fonts);

    // One dark theme everywhere. egui follows the OS light/dark setting by default, but
    // the timeline, piano roll and sequencer paint their own dark colours, so a light
    // OS theme left half the app light and half dark. A light theme can come later.
    ctx.set_theme(egui::Theme::Dark);

    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.text_styles = text_styles();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(6.0, 3.0);
    style.spacing.window_margin = egui::Margin::same(8.0);
    style.spacing.indent = 14.0;
    style.visuals = studio_visuals(compact_visuals(style.visuals));

    ctx.set_style_of(egui::Theme::Dark, style);
}

fn text_styles() -> BTreeMap<TextStyle, FontId> {
    BTreeMap::from([
        (TextStyle::Small, FontId::new(10.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Heading, FontId::new(14.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(11.0, FontFamily::Monospace)),
    ])
}

/// Dark visuals tinted to match the hand-painted editors (track rows are ~rgb(32,32,38)),
/// so panels, windows and editors read as one surface instead of grey vs blue-grey.
fn studio_visuals(mut visuals: Visuals) -> Visuals {
    use egui::Color32;
    visuals.panel_fill = Color32::from_rgb(28, 28, 34);
    visuals.window_fill = Color32::from_rgb(33, 33, 40);
    visuals.extreme_bg_color = Color32::from_rgb(20, 20, 25);
    visuals.faint_bg_color = Color32::from_rgb(36, 36, 43);
    visuals.code_bg_color = Color32::from_rgb(38, 38, 46);
    visuals.window_stroke.color = Color32::from_rgb(58, 58, 68);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(33, 33, 40);
    visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(33, 33, 40);
    visuals.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(52, 52, 62);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(52, 52, 62);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(46, 46, 56);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(64, 64, 76);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(60, 60, 72);
    visuals.widgets.active.bg_fill = Color32::from_rgb(74, 74, 88);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(70, 70, 84);
    visuals
}

fn compact_visuals(mut visuals: Visuals) -> Visuals {
    visuals.window_rounding = egui::Rounding::same(6.0);
    visuals.menu_rounding = egui::Rounding::same(4.0);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(3.0);
    visuals.widgets.inactive.rounding = egui::Rounding::same(3.0);
    visuals.widgets.hovered.rounding = egui::Rounding::same(3.0);
    visuals.widgets.active.rounding = egui::Rounding::same(3.0);
    visuals.widgets.open.rounding = egui::Rounding::same(3.0);
    visuals
}
