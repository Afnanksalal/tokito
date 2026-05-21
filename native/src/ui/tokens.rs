//! UI color and spacing tokens.

use egui::{Color32, Margin, Rounding, Stroke};

#[derive(Clone, Copy)]
pub struct UiTokens {
    pub bg_app: Color32,
    pub bg_panel: Color32,
    pub bg_elevated: Color32,
    /// Schematic sheet (always light gray).
    pub bg_canvas: Color32,
    pub bg_hover: Color32,
    pub bg_chip_selected: Color32,
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
    pub spacing_lg: f32,
    pub radius_sm: Rounding,
    pub radius_md: Rounding,
    pub radius_lg: Rounding,
    pub margin_panel: Margin,
    // Schematic sheet ink
    pub sym_ink: Color32,
    pub sym_ink_hover: Color32,
    pub sym_ink_selected: Color32,
    pub sym_outline: Color32,
    pub sym_sel_ring: Color32,
    pub canvas_grid_minor: Color32,
    pub canvas_grid_major: Color32,
    pub canvas_frame: Color32,
    pub wire: Color32,
    pub wire_highlight: Color32,
    pub wire_selected: Color32,
    pub label_ink: Color32,
    pub refdes_ink: Color32,
    pub pin_ink: Color32,
    pub pin_hot: Color32,
    pub selection: Color32,
    pub preview_bg: Color32,
}

impl Default for UiTokens {
    fn default() -> Self {
        Self {
            // Light theme — cool slate neutrals with more layering contrast so
            // panels read distinctly against the app background and inputs pop
            // against panels.
            bg_app: Color32::from_rgb(241, 244, 249),
            bg_panel: Color32::from_rgb(252, 253, 255),
            bg_elevated: Color32::from_rgb(255, 255, 255),
            bg_canvas: Color32::from_rgb(244, 247, 250),
            bg_hover: Color32::from_rgb(231, 238, 246),
            bg_chip_selected: Color32::from_rgb(214, 238, 232),
            stroke_subtle: Stroke::new(1.0, Color32::from_rgb(215, 222, 232)),
            stroke_focus: Stroke::new(1.4, Color32::from_rgb(20, 132, 118)),
            accent: Color32::from_rgb(20, 132, 118),
            accent_dim: Color32::from_rgba_unmultiplied(20, 132, 118, 46),
            text_primary: Color32::from_rgb(14, 20, 30),
            text_secondary: Color32::from_rgb(72, 84, 102),
            text_muted: Color32::from_rgb(124, 136, 154),
            danger: Color32::from_rgb(207, 67, 76),
            warning: Color32::from_rgb(189, 124, 36),
            spacing_xs: 4.0,
            spacing_sm: 10.0,
            spacing_md: 16.0,
            spacing_lg: 24.0,
            radius_sm: Rounding::same(6.0),
            radius_md: Rounding::same(8.0),
            radius_lg: Rounding::same(12.0),
            margin_panel: Margin::symmetric(16.0, 14.0),
            sym_ink: Color32::from_rgb(28, 32, 38),
            sym_ink_hover: Color32::from_rgb(20, 52, 92),
            sym_ink_selected: Color32::from_rgb(16, 20, 26),
            sym_outline: Color32::from_rgb(250, 251, 252),
            sym_sel_ring: Color32::from_rgb(224, 120, 32),
            canvas_grid_minor: Color32::from_rgba_unmultiplied(140, 148, 158, 28),
            canvas_grid_major: Color32::from_rgba_unmultiplied(120, 128, 140, 52),
            canvas_frame: Color32::from_rgb(168, 174, 184),
            wire: Color32::from_rgb(48, 94, 132),
            wire_highlight: Color32::from_rgb(20, 132, 118),
            wire_selected: Color32::from_rgb(224, 120, 32),
            label_ink: Color32::from_rgb(40, 72, 108),
            refdes_ink: Color32::from_rgb(48, 54, 62),
            pin_ink: Color32::from_rgb(72, 88, 108),
            pin_hot: Color32::from_rgb(224, 120, 32),
            selection: Color32::from_rgb(224, 120, 32),
            preview_bg: Color32::from_rgb(244, 245, 247),
        }
    }
}
