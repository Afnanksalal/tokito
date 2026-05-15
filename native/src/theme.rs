//! Light gray CAD visuals + bundled proportional / monospace fonts.

use crate::ui::UiTokens;
use egui::{FontData, FontDefinitions, FontFamily, Rounding, Stroke, Visuals};

pub fn apply(ctx: &egui::Context) {
    setup_fonts(ctx);

    let t = UiTokens::default();
    let mut visuals = Visuals::light();
    visuals.override_text_color = Some(t.text_primary);

    visuals.window_fill = t.bg_panel;
    visuals.panel_fill = t.bg_panel;
    visuals.extreme_bg_color = t.bg_app;
    visuals.faint_bg_color = t.bg_elevated;
    visuals.widgets.noninteractive.bg_fill = t.bg_elevated;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.text_muted);
    visuals.widgets.inactive.bg_fill = t.bg_elevated;
    visuals.widgets.inactive.weak_bg_fill = t.bg_panel;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text_secondary);
    visuals.widgets.hovered.bg_fill = t.bg_hover;
    visuals.widgets.active.bg_fill = t.bg_chip_selected;
    visuals.widgets.open.bg_fill = visuals.widgets.active.bg_fill;

    visuals.selection.bg_fill = t.accent_dim;
    visuals.selection.stroke = Stroke::new(1.0, t.accent);

    visuals.window_rounding = t.radius_md;
    visuals.menu_rounding = Rounding::same(6.0);
    visuals.window_shadow = egui::epaint::Shadow::NONE;

    ctx.set_visuals(visuals);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.window_margin = egui::Margin::same(8.0);
    style.spacing.indent = 14.0;
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
