use egui::{self, Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, Style, TextStyle, Visuals};
use egui::style::WidgetVisuals;

pub const BG_DEEP: Color32 = Color32::from_rgb(13, 15, 18);
pub const BG_PANEL: Color32 = Color32::from_rgb(20, 23, 28);
pub const BG_SURFACE: Color32 = Color32::from_rgb(28, 32, 38);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(36, 41, 48);
pub const BG_INPUT: Color32 = Color32::from_rgb(16, 19, 24);

pub const ACCENT: Color32 = Color32::from_rgb(0, 212, 170);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0, 140, 112);
pub const ACCENT_GLOW: Color32 = Color32::from_rgb(0, 255, 204);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(210, 215, 220);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(120, 130, 145);
pub const TEXT_DIM: Color32 = Color32::from_rgb(70, 78, 90);

pub const RED: Color32 = Color32::from_rgb(255, 82, 82);
pub const RED_DIM: Color32 = Color32::from_rgb(180, 50, 50);
pub const ORANGE: Color32 = Color32::from_rgb(255, 170, 50);
pub const GREEN: Color32 = Color32::from_rgb(80, 220, 120);
pub const YELLOW: Color32 = Color32::from_rgb(240, 220, 80);
pub const BLUE: Color32 = Color32::from_rgb(80, 160, 255);

pub const SIDEBAR_W: f32 = 180.0;
pub const SECTION_ROUNDING: CornerRadius = CornerRadius::same(3);

pub fn apply(ctx: &egui::Context) {
    let mut style = Style::default();

    style.visuals = Visuals {
        dark_mode: true,
        override_text_color: Some(TEXT_PRIMARY),
        panel_fill: BG_PANEL,
        window_fill: BG_PANEL,
        faint_bg_color: BG_SURFACE,
        extreme_bg_color: BG_INPUT,
        window_corner_radius: CornerRadius::same(4),
        window_stroke: Stroke::new(1.0, Color32::from_rgb(40, 45, 55)),
        widgets: egui::style::Widgets {
            noninteractive: WidgetVisuals {
                bg_fill: BG_SURFACE,
                weak_bg_fill: BG_SURFACE,
                bg_stroke: Stroke::new(0.0, Color32::TRANSPARENT),
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke::new(1.0, TEXT_SECONDARY),
                expansion: 0.0,
            },
            inactive: WidgetVisuals {
                bg_fill: BG_ELEVATED,
                weak_bg_fill: BG_ELEVATED,
                bg_stroke: Stroke::new(1.0, Color32::from_rgb(50, 56, 65)),
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke::new(1.0, TEXT_PRIMARY),
                expansion: 0.0,
            },
            hovered: WidgetVisuals {
                bg_fill: Color32::from_rgb(44, 50, 60),
                weak_bg_fill: Color32::from_rgb(44, 50, 60),
                bg_stroke: Stroke::new(1.0, ACCENT_DIM),
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke::new(1.0, ACCENT),
                expansion: 1.0,
            },
            active: WidgetVisuals {
                bg_fill: Color32::from_rgb(0, 60, 48),
                weak_bg_fill: Color32::from_rgb(0, 60, 48),
                bg_stroke: Stroke::new(1.5, ACCENT),
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke::new(1.5, ACCENT_GLOW),
                expansion: 0.0,
            },
            open: WidgetVisuals {
                bg_fill: BG_ELEVATED,
                weak_bg_fill: BG_ELEVATED,
                bg_stroke: Stroke::new(1.0, ACCENT_DIM),
                corner_radius: CornerRadius::same(2),
                fg_stroke: Stroke::new(1.0, ACCENT),
                expansion: 0.0,
            },
        },
        selection: egui::style::Selection {
            bg_fill: Color32::from_rgb(0, 70, 56),
            stroke: Stroke::new(1.0, ACCENT),
        },
        slider_trailing_fill: true,
        ..Visuals::dark()
    };

    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 4.0);
    style.spacing.slider_width = 200.0;

    style.text_styles.insert(TextStyle::Heading, FontId::new(16.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Small, FontId::new(11.0, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(12.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace));

    ctx.set_style(style);
}

pub fn section_frame() -> egui::Frame {
    egui::Frame::default()
        .fill(BG_SURFACE)
        .corner_radius(SECTION_ROUNDING)
        .inner_margin(Margin::same(12))
        .stroke(Stroke::new(1.0, Color32::from_rgb(35, 40, 48)))
}

pub fn sidebar_frame() -> egui::Frame {
    egui::Frame::default()
        .fill(BG_DEEP)
        .inner_margin(Margin::symmetric(8, 12))
}

pub fn label_dim(text: &str) -> egui::RichText {
    egui::RichText::new(text).color(TEXT_SECONDARY).size(11.0)
}

pub fn label_mono(text: &str) -> egui::RichText {
    egui::RichText::new(text).color(TEXT_PRIMARY).family(FontFamily::Monospace).size(12.0)
}

pub fn section_heading(text: &str) -> egui::RichText {
    egui::RichText::new(text).color(ACCENT).size(13.0).strong()
}
