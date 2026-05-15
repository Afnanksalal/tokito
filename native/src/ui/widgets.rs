//! Reusable widgets: buttons and CAD toolbar icon controls.

use crate::ui::tokens::UiTokens;
use egui::{Pos2, Rect, RichText, Stroke, Ui, Vec2, Widget};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolIcon {
    Select,
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
    ui.add_sized(
        [ui.available_width(), 36.0],
        egui::Button::new(RichText::new(label).strong().color(egui::Color32::WHITE))
            .fill(tokens.accent),
    )
}

pub fn secondary_button(
    ui: &mut Ui,
    tokens: &UiTokens,
    label: impl Into<String>,
) -> egui::Response {
    egui::Button::new(RichText::new(label.into()).color(tokens.text_primary))
        .fill(tokens.bg_elevated)
        .stroke(tokens.stroke_subtle)
        .ui(ui)
}

pub fn cad_tool_button(
    ui: &mut Ui,
    tokens: &UiTokens,
    icon: ToolIcon,
    selected: bool,
    tooltip: &str,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(34.0, 32.0), egui::Sense::click());
    let fill = if selected {
        egui::Color32::from_rgb(46, 65, 88)
    } else if response.hovered() {
        tokens.bg_hover
    } else {
        tokens.bg_panel
    };
    let stroke = if selected {
        Stroke::new(1.2, tokens.accent)
    } else {
        Stroke::new(
            1.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 26),
        )
    };
    ui.painter().rect_filled(rect, 5.0, fill);
    ui.painter().rect_stroke(rect.shrink(0.5), 5.0, stroke);

    let ink = if selected {
        egui::Color32::from_rgb(238, 243, 250)
    } else {
        tokens.text_secondary
    };
    paint_tool_icon(ui.painter(), rect, icon, ink);
    response.on_hover_text(tooltip).clicked()
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
