//! KiCad-style annotation graphics (net labels, power symbols).

use egui::emath::Rot2;
use egui::{Color32, Painter, Pos2, Stroke, Vec2};
use tokito::models::NetLabelKind;

#[inline]
fn rot_point(anchor: Pos2, offset: Vec2, rotation_deg: f32) -> Pos2 {
    if rotation_deg.abs() < 0.5 {
        return anchor + offset;
    }
    let rot = Rot2::from_angle(rotation_deg.to_radians());
    anchor + rot * offset
}

pub fn paint_net_label(
    painter: &Painter,
    anchor: Pos2,
    name: &str,
    kind: NetLabelKind,
    rotation_deg: f32,
    color: Color32,
) {
    match kind {
        NetLabelKind::Hierarchical => {
            let stroke = Stroke::new(1.5, color);
            let corners = [
                anchor,
                rot_point(anchor, Vec2::new(28.0, 0.0), rotation_deg),
                rot_point(anchor, Vec2::new(28.0, 18.0), rotation_deg),
                rot_point(anchor, Vec2::new(0.0, 18.0), rotation_deg),
            ];
            for w in corners.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            painter.line_segment([corners[3], corners[0]], stroke);
            let text_at = rot_point(anchor, Vec2::new(4.0, 2.0), rotation_deg);
            painter.text(
                text_at,
                egui::Align2::LEFT_TOP,
                name,
                egui::FontId::monospace(10.0),
                color,
            );
        }
        NetLabelKind::Global => paint_global_net_label(painter, anchor, name, rotation_deg, color),
        NetLabelKind::Local | NetLabelKind::NetClassDirective => {
            paint_local_net_label(painter, anchor, name, rotation_deg, color)
        }
    }
}

fn paint_local_net_label(
    painter: &Painter,
    anchor: Pos2,
    name: &str,
    rotation_deg: f32,
    color: Color32,
) {
    let rot = Rot2::from_angle(rotation_deg.to_radians());
    let wire_dir = rot * Vec2::new(1.0, 0.0);
    let tip = anchor;
    let base = tip - wire_dir * 22.0;
    let apex = base + rot * Vec2::new(0.0, -10.0);
    let stroke = Stroke::new(1.5, color);
    painter.line_segment([tip, base], stroke);
    painter.line_segment([base, apex], stroke);
    painter.line_segment([apex, tip], stroke);
    painter.text(
        rot_point(tip, Vec2::new(6.0, -12.0), rotation_deg),
        egui::Align2::LEFT_BOTTOM,
        name,
        egui::FontId::proportional(11.0),
        color,
    );
}

fn paint_global_net_label(
    painter: &Painter,
    anchor: Pos2,
    name: &str,
    rotation_deg: f32,
    color: Color32,
) {
    let stroke = Stroke::new(1.5, color);
    let p0 = anchor;
    let p1 = rot_point(anchor, Vec2::new(-18.0, -8.0), rotation_deg);
    let p2 = rot_point(anchor, Vec2::new(-18.0, 8.0), rotation_deg);
    painter.line_segment([p0, p1], stroke);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p0], stroke);
    painter.line_segment(
        [p1, rot_point(p1, Vec2::new(-8.0, 0.0), rotation_deg)],
        stroke,
    );
    painter.text(
        rot_point(anchor, Vec2::new(8.0, 0.0), rotation_deg),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(11.0),
        color,
    );
}

/// Standard ground symbol (bars).
pub fn paint_ground_symbol(painter: &Painter, anchor: Pos2, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let top = anchor;
    let mid = anchor + Vec2::new(0.0, 10.0);
    painter.line_segment([top, mid], stroke);
    painter.line_segment(
        [mid + Vec2::new(-12.0, 0.0), mid + Vec2::new(12.0, 0.0)],
        stroke,
    );
    painter.line_segment(
        [mid + Vec2::new(-8.0, 5.0), mid + Vec2::new(8.0, 5.0)],
        stroke,
    );
    painter.line_segment(
        [mid + Vec2::new(-4.0, 10.0), mid + Vec2::new(4.0, 10.0)],
        stroke,
    );
}

/// VCC / power arrow symbol.
pub fn paint_power_symbol(painter: &Painter, anchor: Pos2, name: &str, color: Color32) {
    let stroke = Stroke::new(1.5, color);
    let tip = anchor;
    let base = anchor + Vec2::new(0.0, 16.0);
    painter.line_segment([tip, base], stroke);
    painter.line_segment(
        [base + Vec2::new(-8.0, 0.0), base + Vec2::new(8.0, 0.0)],
        stroke,
    );
    painter.text(
        tip + Vec2::new(0.0, -6.0),
        egui::Align2::CENTER_BOTTOM,
        name,
        egui::FontId::proportional(11.0),
        color,
    );
}

pub fn paint_power_by_name(painter: &Painter, anchor: Pos2, name: &str, color: Color32) {
    let lower = name.to_ascii_lowercase();
    if lower.contains("gnd") || lower == "vss" || lower == "ground" {
        paint_ground_symbol(painter, anchor, color);
        painter.text(
            anchor + Vec2::new(14.0, 8.0),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(10.0),
            color,
        );
    } else {
        paint_power_symbol(painter, anchor, name, color);
    }
}
