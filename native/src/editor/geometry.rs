//! World-space bounds and routing helpers for the schematic canvas.

use egui::{Pos2, Rect, Vec2};

use crate::canvas::{symbol_hit_half_extents, GRID_PX};
use crate::canvas::{CanvasSnapshot, Sym, WireSegment};

/// Padding around content when fitting the viewport (world units).
pub const FIT_PADDING: f32 = 48.0;

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
    crate::canvas::sheet_bounds_world()
}

/// Axis-aligned bounds of a symbol in world space (for hit testing / marquee).
pub fn symbol_bounds_world(sym: &Sym) -> Rect {
    let (hx, hy) = symbol_hit_half_extents(sym);
    Rect::from_center_size(sym.pos, Vec2::new(hx * 2.0, hy * 2.0))
}

/// Whether a symbol is inside a marquee (enclosed = full bounds inside, else intersects).
pub fn symbol_in_marquee(sym: &Sym, marquee: Rect, enclosed: bool) -> bool {
    let b = symbol_bounds_world(sym);
    if enclosed {
        marquee.contains(b.min) && marquee.contains(b.max)
    } else {
        marquee.intersects(b)
    }
}

/// Whether a wire segment is inside / touched by a marquee.
pub fn segment_in_marquee(seg: &WireSegment, marquee: Rect, enclosed: bool) -> bool {
    if enclosed {
        return marquee.contains(seg.start) && marquee.contains(seg.end);
    }
    marquee.contains(seg.start)
        || marquee.contains(seg.end)
        || segment_intersects_rect(seg.start, seg.end, marquee)
}

fn segment_intersects_rect(a: Pos2, b: Pos2, r: Rect) -> bool {
    if r.contains(a) || r.contains(b) {
        return true;
    }
    let edges = [
        (r.left_top(), r.right_top()),
        (r.right_top(), r.right_bottom()),
        (r.right_bottom(), r.left_bottom()),
        (r.left_bottom(), r.left_top()),
    ];
    edges.iter().any(|&(p, q)| segments_intersect(a, b, p, q))
}

fn segments_intersect(a: Pos2, b: Pos2, c: Pos2, d: Pos2) -> bool {
    fn cross(o: Pos2, a: Pos2, b: Pos2) -> f32 {
        (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
    }
    let d1 = cross(c, d, a);
    let d2 = cross(c, d, b);
    let d3 = cross(a, b, c);
    let d4 = cross(a, b, d);
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    false
}

/// Default paste nudge (one grid step).
pub fn paste_nudge() -> Vec2 {
    Vec2::new(GRID_PX * 2.0, GRID_PX * 2.0)
}
