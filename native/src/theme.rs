//! CAD-style dark visuals + bundled proportional / monospace fonts.

use crate::ui::UiTokens;
use egui::{Color32, FontData, FontDefinitions, FontFamily, Rounding, Stroke, Visuals};

pub fn apply(ctx: &egui::Context) {
    setup_fonts(ctx);

    let tokens = UiTokens::default();
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(tokens.text_primary);

    visuals.window_fill = tokens.bg_panel;
    visuals.panel_fill = Color32::from_rgb(26, 28, 34);
    visuals.extreme_bg_color = tokens.bg_app;
    visuals.faint_bg_color = Color32::from_rgb(34, 37, 45);
    visuals.widgets.noninteractive.bg_fill = tokens.bg_elevated;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(140, 144, 152));
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(38, 41, 50);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(34, 37, 45);
    visuals.widgets.hovered.bg_fill = tokens.bg_hover;
    visuals.widgets.active.bg_fill = Color32::from_rgb(52, 58, 72);
    visuals.widgets.open.bg_fill = visuals.widgets.active.bg_fill;

    visuals.selection.bg_fill = tokens.accent_dim;
    visuals.selection.stroke = Stroke::new(1.0, tokens.accent);

    visuals.window_rounding = tokens.radius_md;
    visuals.menu_rounding = Rounding::same(8.0);

    ctx.set_visuals(visuals);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(10.0);
    ctx.set_style(style);
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "inter_ui".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/InterVar.ttf")),
    );
    fonts.font_data.insert(
        "jetbrains_mono".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter_ui".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "jetbrains_mono".to_owned());

    ctx.set_fonts(fonts);
}
