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
            bg_app: Color32::from_rgb(17, 18, 22),
            bg_panel: Color32::from_rgb(24, 26, 31),
            bg_elevated: Color32::from_rgb(33, 36, 43),
            bg_canvas: Color32::from_rgb(12, 15, 18),
            bg_hover: Color32::from_rgb(48, 52, 61),
            stroke_subtle: Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 24)),
            stroke_focus: Stroke::new(1.0, Color32::from_rgb(72, 128, 196)),
            accent: Color32::from_rgb(56, 116, 190),
            accent_dim: Color32::from_rgba_unmultiplied(56, 116, 190, 88),
            text_primary: Color32::from_rgb(236, 238, 240),
            text_secondary: Color32::from_rgb(184, 188, 195),
            text_muted: Color32::from_rgb(132, 138, 148),
            danger: Color32::from_rgb(230, 95, 95),
            warning: Color32::from_rgb(218, 174, 78),
            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 12.0,
            radius_sm: Rounding::same(4.0),
            radius_md: Rounding::same(8.0),
            margin_panel: Margin::same(12.0),
        }
    }
}
