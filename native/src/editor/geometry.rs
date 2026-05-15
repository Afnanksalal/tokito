//! World-space bounds and routing helpers for the schematic canvas.

use egui::{Pos2, Rect};

use crate::canvas::CanvasSnapshot;

/// Padding around content when fitting the viewport (world units).
pub const FIT_PADDING: f32 = 48.0;

/// Minimum world extent when the sheet is empty (fit still centers a usable view).
pub const EMPTY_SHEET_HALF: f32 = 200.0;

/// Union bounding box of all schematic geometry on the canvas.
pub fn content_bounds(snap: &CanvasSnapshot) -> Option<Rect> {
    let mut r = Rect::NOTHING;
    let mut any = false;

    for s in &snap.symbols {
        let half = egui::Vec2::new(78.0, 36.0);
        r = r.union(Rect::from_center_size(s.pos, half * 2.0));
        any = true;
    }

    for seg in &snap.wire_segments {
        r = r.union(Rect::from_two_pos(seg.start, seg.end));
        any = true;
    }

    for label in &snap.net_labels {
        r = r.union(Rect::from_center_size(label.pos, egui::vec2(48.0, 24.0)));
        any = true;
    }

    for j in &snap.junctions {
        r = r.union(Rect::from_center_size(j.pos, egui::vec2(12.0, 12.0)));
        any = true;
    }

    for nc in &snap.no_connects {
        r = r.union(Rect::from_center_size(nc.pos, egui::vec2(16.0, 16.0)));
        any = true;
    }

    for p in &snap.power_symbols {
        r = r.union(Rect::from_center_size(
            p.pos + egui::vec2(0.0, -18.0),
            egui::vec2(40.0, 48.0),
        ));
        any = true;
    }

    for t in &snap.text_items {
        let w = (t.text.len() as f32 * 7.0).clamp(24.0, 400.0);
        r = r.union(Rect::from_min_size(t.pos, egui::vec2(w, 22.0)));
        any = true;
    }

    for b in &snap.buses {
        r = r.union(Rect::from_two_pos(b.start, b.end));
        any = true;
    }

    if any {
        Some(r.expand(FIT_PADDING))
    } else {
        None
    }
}

/// Default bounds when opening an empty sheet (centered at origin).
pub fn empty_sheet_bounds() -> Rect {
    Rect::from_center_size(
        Pos2::ZERO,
        egui::vec2(EMPTY_SHEET_HALF * 2.0, EMPTY_SHEET_HALF * 2.0),
    )
}
