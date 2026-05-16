//! Junction detection at wire crossings (T / + intersections).

use egui::Pos2;

use crate::canvas::{Junction, WireSegment};

/// Insert junction dots where orthogonal segments cross (not at shared endpoints).
pub fn ensure_junctions_at_crossings(segments: &[WireSegment], junctions: &mut Vec<Junction>) {
    for i in 0..segments.len() {
        for j in (i + 1)..segments.len() {
            if let Some(p) = segment_intersection(&segments[i], &segments[j]) {
                if segments[i].start.distance(p) > 4.0
                    && segments[i].end.distance(p) > 4.0
                    && segments[j].start.distance(p) > 4.0
                    && segments[j].end.distance(p) > 4.0
                    && !junctions.iter().any(|j| j.pos.distance(p) <= 6.0)
                {
                    junctions.push(Junction { pos: p });
                }
            }
        }
    }
}

fn segment_intersection(a: &WireSegment, b: &WireSegment) -> Option<Pos2> {
    let hor_a = (a.start.y - a.end.y).abs() < 0.5;
    let hor_b = (b.start.y - b.end.y).abs() < 0.5;
    if hor_a == hor_b {
        return None;
    }
    let (h, v) = if hor_a { (a, b) } else { (b, a) };
    let x = v.start.x;
    let y = h.start.y;
    let hx_min = h.start.x.min(h.end.x) - 0.5;
    let hx_max = h.start.x.max(h.end.x) + 0.5;
    let vy_min = v.start.y.min(v.end.y) - 0.5;
    let vy_max = v.start.y.max(v.end.y) + 0.5;
    if x >= hx_min && x <= hx_max && y >= vy_min && y <= vy_max {
        Some(Pos2::new(x, y))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_t_crossing() {
        let segs = vec![
            WireSegment::new(Pos2::new(0.0, 40.0), Pos2::new(80.0, 40.0), "N"),
            WireSegment::new(Pos2::new(40.0, 0.0), Pos2::new(40.0, 80.0), "N"),
        ];
        let mut junctions = vec![];
        ensure_junctions_at_crossings(&segs, &mut junctions);
        assert_eq!(junctions.len(), 1);
        assert!((junctions[0].pos.x - 40.0).abs() < 1.0);
        assert!((junctions[0].pos.y - 40.0).abs() < 1.0);
    }
}
