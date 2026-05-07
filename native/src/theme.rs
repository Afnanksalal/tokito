//! Calm SaaS-style dark visuals for egui.

use egui::{Color32, Rounding, Stroke, Visuals};

pub fn apply(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(Color32::from_rgb(235, 237, 240));

    let bg = Color32::from_rgb(22, 24, 28);
    let elevated = Color32::from_rgb(30, 33, 40);
    let accent = Color32::from_rgb(88, 149, 255);

    visuals.window_fill = bg;
    visuals.panel_fill = Color32::from_rgb(26, 28, 34);
    visuals.extreme_bg_color = Color32::from_rgb(18, 19, 23);
    visuals.faint_bg_color = Color32::from_rgb(34, 37, 45);
    visuals.widgets.noninteractive.bg_fill = elevated;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(140, 144, 152));
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(38, 41, 50);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(34, 37, 45);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(46, 50, 62);
    visuals.widgets.active.bg_fill = Color32::from_rgb(52, 58, 72);
    visuals.widgets.open.bg_fill = visuals.widgets.active.bg_fill;

    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(88, 149, 255, 55);
    visuals.selection.stroke = Stroke::new(1.0, accent);

    visuals.window_rounding = Rounding::same(10.0);
    visuals.menu_rounding = Rounding::same(8.0);

    ctx.set_visuals(visuals);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    ctx.set_style(style);
}
