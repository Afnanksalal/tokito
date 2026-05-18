//! Reusable widgets: buttons and CAD toolbar icon controls.

use crate::ui::tokens::UiTokens;
use egui::{Pos2, Rect, RichText, Stroke, Ui, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolIcon {
    Select,
    Place,
    Wire,
    NetLabel,
    Power,
    Junction,
    NoConnect,
    Bus,
    Text,
    Pan,
    Focus,
    ZoomFit,
    Grid,
    Snap,
}

pub fn primary_button(ui: &mut Ui, tokens: &UiTokens, label: impl Into<String>) -> egui::Response {
    let label = label.into();
    let width = fitted_button_width(ui, &label, 112.0, 220.0);
    ui.add_sized(
        [width, 36.0],
        egui::Button::new(RichText::new(label).strong().color(egui::Color32::WHITE))
            .fill(tokens.accent),
    )
}

pub fn primary_button_full(
    ui: &mut Ui,
    tokens: &UiTokens,
    label: impl Into<String>,
) -> egui::Response {
    let available = ui.available_width().max(0.0);
    let width = available.min(360.0).max(available.min(112.0));
    ui.add_sized(
        [width, 38.0],
        egui::Button::new(
            RichText::new(label.into())
                .strong()
                .color(egui::Color32::WHITE),
        )
        .fill(tokens.accent),
    )
}

pub fn secondary_button(
    ui: &mut Ui,
    tokens: &UiTokens,
    label: impl Into<String>,
) -> egui::Response {
    let label = label.into();
    let width = fitted_button_width(ui, &label, 96.0, 220.0);
    ui.add_sized(
        [width, 34.0],
        egui::Button::new(RichText::new(label).color(tokens.text_primary))
            .fill(tokens.bg_elevated)
            .stroke(tokens.stroke_subtle),
    )
}

pub fn cad_tool_button(
    ui: &mut Ui,
    tokens: &UiTokens,
    icon: ToolIcon,
    selected: bool,
    tooltip: &str,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(38.0, 36.0), egui::Sense::click());
    let fill = if selected {
        tokens.bg_chip_selected
    } else if response.hovered() {
        tokens.bg_hover
    } else {
        tokens.bg_elevated
    };
    let stroke = if selected {
        Stroke::new(1.2, tokens.accent)
    } else {
        tokens.stroke_subtle
    };
    ui.painter().rect_filled(rect, 7.0, fill);
    ui.painter().rect_stroke(rect.shrink(0.5), 7.0, stroke);

    let ink = if selected {
        tokens.text_primary
    } else {
        tokens.text_secondary
    };
    paint_tool_icon(ui.painter(), rect, icon, ink);
    response.on_hover_text(tooltip).clicked()
}

fn fitted_button_width(ui: &Ui, label: &str, min: f32, max: f32) -> f32 {
    let available = ui.available_width().max(0.0);
    let estimated = label.chars().count() as f32 * 7.2 + 34.0;
    let desired = estimated.clamp(min, max);
    if available < min {
        available
    } else {
        desired.min(available)
    }
}

pub fn toolbar_icon_btn(ui: &mut Ui, tokens: &UiTokens, tooltip: &str, icon: ToolIcon) -> bool {
    cad_tool_button(ui, tokens, icon, false, tooltip)
}

fn paint_tool_icon(painter: &egui::Painter, rect: Rect, icon: ToolIcon, ink: egui::Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) / 24.0;
    let p = |x: f32, y: f32| Pos2::new(c.x + x * s, c.y + y * s);
    let stroke = Stroke::new(1.65, ink);

    match icon {
        ToolIcon::Place => {
            painter.rect_stroke(
                Rect::from_center_size(c, egui::vec2(14.0, 10.0) * s),
                0.0,
                stroke,
            );
            painter.line_segment([p(0.0, -10.0), p(0.0, 2.0)], stroke);
            painter.line_segment([p(-5.0, -3.0), p(5.0, -3.0)], stroke);
        }
        ToolIcon::Select => {
            painter.line_segment([p(-6.0, -8.0), p(-2.0, 8.0)], stroke);
            painter.line_segment([p(-6.0, -8.0), p(6.0, 1.0)], stroke);
            painter.line_segment([p(-2.0, 8.0), p(1.0, 3.0)], stroke);
            painter.line_segment([p(1.0, 3.0), p(6.0, 1.0)], stroke);
        }
        ToolIcon::Wire => {
            painter.line_segment([p(-8.0, 5.0), p(-2.0, 5.0)], stroke);
            painter.line_segment([p(-2.0, 5.0), p(-2.0, -5.0)], stroke);
            painter.line_segment([p(-2.0, -5.0), p(8.0, -5.0)], stroke);
            painter.circle_filled(p(-8.0, 5.0), 2.2, ink);
            painter.circle_filled(p(8.0, -5.0), 2.2, ink);
        }
        ToolIcon::NetLabel => {
            let pts = [
                p(-8.0, -5.0),
                p(2.0, -5.0),
                p(8.0, 0.0),
                p(2.0, 5.0),
                p(-8.0, 5.0),
            ];
            for pair in pts.windows(2) {
                painter.line_segment([pair[0], pair[1]], stroke);
            }
            painter.line_segment([pts[4], pts[0]], stroke);
            painter.line_segment([p(-5.0, 2.0), p(-5.0, -2.0)], stroke);
            painter.line_segment([p(-5.0, -2.0), p(-1.0, 2.0)], stroke);
            painter.line_segment([p(-1.0, 2.0), p(-1.0, -2.0)], stroke);
        }
        ToolIcon::Junction => {
            painter.line_segment([p(-8.0, 0.0), p(8.0, 0.0)], stroke);
            painter.line_segment([p(0.0, -8.0), p(0.0, 8.0)], stroke);
            painter.circle_filled(c, 4.0, ink);
        }
        ToolIcon::Power => {
            painter.line_segment([p(0.0, 8.0), p(0.0, -2.0)], stroke);
            painter.line_segment([p(-7.0, -2.0), p(7.0, -2.0)], stroke);
            painter.line_segment([p(-4.5, -6.0), p(4.5, -6.0)], stroke);
            painter.line_segment([p(-2.0, -10.0), p(2.0, -10.0)], stroke);
        }
        ToolIcon::NoConnect => {
            painter.line_segment([p(-7.0, -7.0), p(7.0, 7.0)], stroke);
            painter.line_segment([p(-7.0, 7.0), p(7.0, -7.0)], stroke);
            painter.line_segment([p(-10.0, 0.0), p(-5.0, 0.0)], stroke);
        }
        ToolIcon::Bus => {
            painter.line_segment([p(-9.0, 7.0), p(9.0, -7.0)], Stroke::new(3.0, ink));
            painter.line_segment([p(-9.0, -2.0), p(-2.0, -2.0)], stroke);
            painter.line_segment([p(2.0, 2.0), p(9.0, 2.0)], stroke);
        }
        ToolIcon::Text => {
            painter.line_segment([p(-8.0, -7.0), p(8.0, -7.0)], stroke);
            painter.line_segment([p(0.0, -7.0), p(0.0, 8.0)], stroke);
            painter.line_segment([p(-4.0, 8.0), p(4.0, 8.0)], stroke);
        }
        ToolIcon::Pan => {
            painter.circle_stroke(c, 6.5, stroke);
            painter.line_segment([p(0.0, -10.0), p(0.0, 10.0)], stroke);
            painter.line_segment([p(-10.0, 0.0), p(10.0, 0.0)], stroke);
        }
        ToolIcon::Focus => {
            painter.line_segment([p(-9.0, -4.0), p(-9.0, -9.0)], stroke);
            painter.line_segment([p(-9.0, -9.0), p(-4.0, -9.0)], stroke);
            painter.line_segment([p(9.0, -4.0), p(9.0, -9.0)], stroke);
            painter.line_segment([p(9.0, -9.0), p(4.0, -9.0)], stroke);
            painter.line_segment([p(-9.0, 4.0), p(-9.0, 9.0)], stroke);
            painter.line_segment([p(-9.0, 9.0), p(-4.0, 9.0)], stroke);
            painter.line_segment([p(9.0, 4.0), p(9.0, 9.0)], stroke);
            painter.line_segment([p(9.0, 9.0), p(4.0, 9.0)], stroke);
        }
        ToolIcon::ZoomFit => {
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::new(15.0 * s, 12.0 * s)),
                1.0,
                stroke,
            );
            painter.line_segment([p(-10.0, -8.0), p(-5.0, -8.0)], stroke);
            painter.line_segment([p(-10.0, -8.0), p(-10.0, -3.0)], stroke);
            painter.line_segment([p(10.0, 8.0), p(5.0, 8.0)], stroke);
            painter.line_segment([p(10.0, 8.0), p(10.0, 3.0)], stroke);
        }
        ToolIcon::Grid => {
            for x in [-6.0, 0.0, 6.0] {
                painter.line_segment([p(x, -8.0), p(x, 8.0)], stroke);
            }
            for y in [-6.0, 0.0, 6.0] {
                painter.line_segment([p(-8.0, y), p(8.0, y)], stroke);
            }
        }
        ToolIcon::Snap => {
            painter.circle_stroke(c, 7.0, stroke);
            painter.line_segment([p(-8.0, 0.0), p(8.0, 0.0)], stroke);
            painter.line_segment([p(0.0, -8.0), p(0.0, 8.0)], stroke);
            painter.circle_filled(c, 2.5, ink);
        }
    }
}
