//! CAD-style dark palette + spacing scale (dense UI).

use egui::{Color32, Margin, Rounding, Stroke};

#[derive(Clone, Copy)]
#[allow(dead_code)] // reserved for wider theme parity / inspector polish
pub struct UiTokens {
    pub bg_app: Color32,
    pub bg_panel: Color32,
    pub bg_elevated: Color32,
    pub bg_canvas: Color32,
    pub bg_hover: Color32,
    pub stroke_subtle: Stroke,
    pub stroke_focus: Stroke,
    pub accent: Color32,
    pub accent_dim: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub spacing_xs: f32,
    pub spacing_sm: f32,
    pub spacing_md: f32,
    pub radius_sm: Rounding,
    pub radius_md: Rounding,
    pub margin_panel: Margin,
}

impl Default for UiTokens {
    fn default() -> Self {
        Self {
            bg_app: Color32::from_rgb(18, 19, 23),
            bg_panel: Color32::from_rgb(22, 24, 28),
            bg_elevated: Color32::from_rgb(30, 33, 40),
            bg_canvas: Color32::from_rgb(14, 15, 19),
            bg_hover: Color32::from_rgb(46, 50, 62),
            stroke_subtle: Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 28)),
            stroke_focus: Stroke::new(1.0, Color32::from_rgb(110, 155, 235)),
            accent: Color32::from_rgb(88, 149, 255),
            accent_dim: Color32::from_rgba_unmultiplied(88, 149, 255, 90),
            text_primary: Color32::from_rgb(235, 237, 240),
            text_secondary: Color32::from_rgb(175, 180, 190),
            text_muted: Color32::from_rgb(120, 125, 135),
            danger: Color32::from_rgb(255, 120, 120),
            warning: Color32::from_rgb(210, 190, 110),
            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 12.0,
            radius_sm: Rounding::same(6.0),
            radius_md: Rounding::same(10.0),
            margin_panel: Margin::same(10.0),
        }
    }
}
