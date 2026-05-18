//! CAD visuals + bundled fonts; light/dark themes from settings.

use crate::ui::UiTokens;
use egui::{Color32, FontData, FontDefinitions, FontFamily, Rounding, Stroke, Visuals};

pub fn apply(ctx: &egui::Context) {
    let theme = tokito::settings::merge_from_env(tokito::settings::load_file())
        .general
        .theme;
    apply_with_theme(ctx, &theme);
}

pub fn apply_with_theme(ctx: &egui::Context, theme: &str) {
    setup_fonts(ctx);
    let t = tokens_for(theme);
    apply_tokens(ctx, &t, theme);
}

pub fn effective_theme(theme: &str) -> String {
    if theme == "system" {
        match dark_light::detect() {
            Ok(mode) => match mode {
                dark_light::Mode::Dark => "dark".to_string(),
                dark_light::Mode::Light | dark_light::Mode::Unspecified => "light".to_string(),
            },
            Err(_) => "light".to_string(),
        }
    } else {
        theme.to_string()
    }
}

pub fn tokens_for(theme: &str) -> UiTokens {
    let theme = effective_theme(theme);
    match theme.as_str() {
        "dark" => UiTokens {
            bg_app: Color32::from_rgb(19, 22, 26),
            bg_panel: Color32::from_rgb(27, 31, 36),
            bg_elevated: Color32::from_rgb(35, 40, 47),
            bg_canvas: Color32::from_rgb(32, 37, 43),
            bg_hover: Color32::from_rgb(43, 50, 58),
            bg_chip_selected: Color32::from_rgb(28, 66, 61),
            stroke_subtle: Stroke::new(1.0, Color32::from_rgb(62, 70, 80)),
            stroke_focus: Stroke::new(1.5, Color32::from_rgb(55, 172, 152)),
            accent: Color32::from_rgb(55, 172, 152),
            accent_dim: Color32::from_rgba_unmultiplied(55, 172, 152, 52),
            text_primary: Color32::from_rgb(238, 241, 245),
            text_secondary: Color32::from_rgb(196, 204, 214),
            text_muted: Color32::from_rgb(139, 150, 164),
            danger: Color32::from_rgb(238, 92, 104),
            warning: Color32::from_rgb(226, 164, 70),
            spacing_xs: 4.0,
            spacing_sm: 10.0,
            spacing_md: 16.0,
            radius_sm: Rounding::same(6.0),
            radius_md: Rounding::same(8.0),
            margin_panel: egui::Margin::symmetric(14.0, 12.0),
            sym_ink: Color32::from_rgb(228, 232, 238),
            sym_ink_hover: Color32::from_rgb(160, 200, 240),
            sym_ink_selected: Color32::from_rgb(255, 255, 255),
            sym_outline: Color32::from_rgb(48, 52, 58),
            sym_sel_ring: Color32::from_rgb(240, 140, 48),
            canvas_grid_minor: Color32::from_rgba_unmultiplied(120, 128, 140, 36),
            canvas_grid_major: Color32::from_rgba_unmultiplied(100, 108, 120, 64),
            canvas_frame: Color32::from_rgb(88, 94, 104),
            wire: Color32::from_rgb(120, 180, 220),
            wire_highlight: Color32::from_rgb(55, 172, 152),
            wire_selected: Color32::from_rgb(240, 180, 80),
            label_ink: Color32::from_rgb(180, 220, 255),
            refdes_ink: Color32::from_rgb(200, 204, 212),
            pin_ink: Color32::from_rgb(140, 200, 255),
            pin_hot: Color32::from_rgb(255, 200, 80),
            selection: Color32::from_rgb(240, 160, 64),
            preview_bg: Color32::from_rgba_unmultiplied(80, 120, 180, 40),
        },
        _ => UiTokens::default(),
    }
}

fn apply_tokens(ctx: &egui::Context, t: &UiTokens, theme: &str) {
    let theme = effective_theme(theme);
    let dark = theme == "dark";
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
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
    visuals.menu_rounding = t.radius_sm;
    visuals.window_shadow = egui::epaint::Shadow::NONE;
    ctx.set_visuals(visuals);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.window_margin = egui::Margin::same(10.0);
    style.spacing.indent = 16.0;
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
