use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};
use std::sync::Arc;

pub const PRIMARY: Color32 = Color32::from_rgb(0x67, 0x50, 0xA4);
pub const ON_PRIMARY: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const PRIMARY_CONTAINER: Color32 = Color32::from_rgb(0xEA, 0xDD, 0xFF);
pub const ON_PRIMARY_CONTAINER: Color32 = Color32::from_rgb(0x21, 0x00, 0x5D);
pub const SECONDARY_CONTAINER: Color32 = Color32::from_rgb(0xE8, 0xDE, 0xF8);
pub const ON_SECONDARY_CONTAINER: Color32 = Color32::from_rgb(0x1D, 0x19, 0x2B);
pub const ERROR: Color32 = Color32::from_rgb(0xB3, 0x26, 0x1E);
pub const ERROR_CONTAINER: Color32 = Color32::from_rgb(0xF9, 0xDE, 0xDC);
pub const ON_ERROR_CONTAINER: Color32 = Color32::from_rgb(0x41, 0x0E, 0x0B);
pub const OK_CONTAINER: Color32 = Color32::from_rgb(0xCF, 0xF0, 0xD2);
pub const ON_OK_CONTAINER: Color32 = Color32::from_rgb(0x0C, 0x3D, 0x18);
pub const SURFACE: Color32 = Color32::from_rgb(0xFE, 0xF7, 0xFF);
pub const SURFACE_LOW: Color32 = Color32::from_rgb(0xF7, 0xF2, 0xFA);
pub const SURFACE_CONTAINER: Color32 = Color32::from_rgb(0xF3, 0xED, 0xF7);
pub const SURFACE_HIGH: Color32 = Color32::from_rgb(0xEC, 0xE6, 0xF0);
pub const SURFACE_HIGHEST: Color32 = Color32::from_rgb(0xE6, 0xE0, 0xE9);
pub const SURFACE_LOWEST: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const ON_SURFACE: Color32 = Color32::from_rgb(0x1D, 0x1B, 0x20);
pub const ON_SURFACE_VARIANT: Color32 = Color32::from_rgb(0x49, 0x45, 0x4F);
pub const OUTLINE: Color32 = Color32::from_rgb(0x79, 0x74, 0x7E);
pub const OUTLINE_VARIANT: Color32 = Color32::from_rgb(0xCA, 0xC4, 0xD0);
pub const INVERSE_SURFACE: Color32 = Color32::from_rgb(0x32, 0x2F, 0x35);
pub const ON_INVERSE_SURFACE: Color32 = Color32::from_rgb(0xF5, 0xEF, 0xF7);

pub const RADIUS_L: f32 = 24.0;

pub fn bold() -> FontFamily {
    FontFamily::Name(Arc::from("bold"))
}

pub fn heading(text: &str, size: f32) -> egui::RichText {
    egui::RichText::new(text).font(FontId::new(size, bold()))
}

pub fn body(text: &str, size: f32) -> egui::RichText {
    egui::RichText::new(text).font(FontId::new(size, FontFamily::Proportional))
}

pub fn subtle(text: &str, size: f32) -> egui::RichText {
    egui::RichText::new(text)
        .font(FontId::new(size, FontFamily::Proportional))
        .color(ON_SURFACE_VARIANT)
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let regular = read_font(&[
        "C:/Windows/Fonts/segoeui.ttf",
        "C:/Windows/Fonts/SegoeUI.ttf",
    ]);
    let semibold = read_font(&[
        "C:/Windows/Fonts/seguisb.ttf",
        "C:/Windows/Fonts/segoeuib.ttf",
    ]);

    let regular_name = "neurofix-sans".to_owned();
    let bold_name = "neurofix-bold".to_owned();

    if let Some(bytes) = regular {
        fonts
            .font_data
            .insert(regular_name.clone(), FontData::from_owned(bytes));
        let mut proportional = vec![regular_name.clone()];
        proportional.extend(
            fonts
                .families
                .get(&FontFamily::Proportional)
                .cloned()
                .unwrap_or_default(),
        );
        fonts
            .families
            .insert(FontFamily::Proportional, proportional);
    }

    if let Some(bytes) = semibold {
        fonts
            .font_data
            .insert(bold_name.clone(), FontData::from_owned(bytes));
        fonts.families.insert(
            bold(),
            vec![bold_name, regular_name.clone(), "Ubuntu-Light".to_owned()],
        );
    } else if fonts.font_data.contains_key(&regular_name) {
        fonts
            .families
            .insert(bold(), vec![regular_name, "Ubuntu-Light".to_owned()]);
    }

    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::light();
    style.visuals.panel_fill = SURFACE;
    style.visuals.window_fill = SURFACE;
    style.visuals.extreme_bg_color = SURFACE_HIGH;
    style.visuals.faint_bg_color = SURFACE_LOW;
    style.visuals.override_text_color = Some(ON_SURFACE);
    style.visuals.selection.bg_fill = PRIMARY_CONTAINER;
    style.visuals.selection.stroke = Stroke::new(1.0_f32, ON_PRIMARY_CONTAINER);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, OUTLINE_VARIANT);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, OUTLINE_VARIANT);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, PRIMARY);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, PRIMARY);
    style.visuals.window_rounding = RADIUS_L.into();
    style.visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 28.0,
        spread: 0.0,
        color: Color32::from_black_alpha(28),
    };
    style.visuals.popup_shadow = style.visuals.window_shadow;
    style.spacing.item_spacing = egui::vec2(12.0, 12.0);
    style.spacing.button_padding = egui::vec2(18.0, 10.0);
    style.spacing.menu_margin = egui::Margin::same(8.0);
    style.text_styles = [
        (TextStyle::Heading, FontId::new(28.0, bold())),
        (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.5, bold())),
        (
            TextStyle::Small,
            FontId::new(12.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        ),
    ]
    .into();

    ctx.set_style(style);
}

fn read_font(paths: &[&str]) -> Option<Vec<u8>> {
    for path in paths {
        if let Ok(bytes) = std::fs::read(path) {
            return Some(bytes);
        }
    }
    None
}
