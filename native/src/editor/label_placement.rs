//! Net label placement helpers (wire-aligned rotation).

use egui::Pos2;

use crate::canvas::WireSegment;

const SNAP_DEG: [f32; 4] = [0.0, 90.0, 180.0, 270.0];

/// Snap label rotation to nearest cardinal angle aligned with the wire under `world`.
pub fn wire_aligned_rotation(world: Pos2, segments: &[WireSegment]) -> f32 {
    let mut best_dist = f32::MAX;
    let mut best_angle = 0.0_f32;
    for seg in segments {
        let d = point_segment_distance(world, seg.start, seg.end);
        if d < best_dist {
            best_dist = d;
            let delta = seg.end - seg.start;
            if delta.length_sq() > 1e-6 {
                best_angle = delta.y.atan2(delta.x).to_degrees();
            }
        }
    }
    if best_dist > crate::canvas::GRID_PX * 1.5 {
        return 0.0;
    }
    let mut nearest = SNAP_DEG[0];
    let mut min_diff = f32::MAX;
    for a in SNAP_DEG {
        let diff = (best_angle - a).abs();
        let diff = diff.min(360.0 - diff);
        if diff < min_diff {
            min_diff = diff;
            nearest = a;
        }
    }
    nearest
}

fn point_segment_distance(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_sq();
    if len_sq < 1e-6 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let proj = a + ab * t;
    p.distance(proj)
}
