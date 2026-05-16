//! Move wire endpoints and junctions when symbols drag.

use std::collections::HashSet;

use egui::Vec2;

use crate::canvas::{Junction, WireSegment};

const JUNCTION_EPS: f32 = 6.0;

/// Move junctions and unanchored wire ends attached to pins on moved symbols.
pub fn push_wires_for_symbol_delta(
    segments: &mut [WireSegment],
    junctions: &mut [Junction],
    moved_refs: &HashSet<String>,
    delta: Vec2,
) {
    if delta == Vec2::ZERO || moved_refs.is_empty() {
        return;
    }

    for seg in segments.iter_mut() {
        let start_moved = seg
            .start_pin
            .as_ref()
            .is_some_and(|p| moved_refs.contains(&p.ref_des));
        let end_moved = seg
            .end_pin
            .as_ref()
            .is_some_and(|p| moved_refs.contains(&p.ref_des));

        if start_moved && seg.end_pin.is_none() {
            seg.end += delta;
            translate_junction_at(junctions, seg.end - delta, delta);
        }
        if end_moved && seg.start_pin.is_none() {
            seg.start += delta;
            translate_junction_at(junctions, seg.start - delta, delta);
        }

        if start_moved {
            translate_junction_near(junctions, seg.end, delta);
        }
        if end_moved {
            translate_junction_near(junctions, seg.start, delta);
        }
    }
}

fn translate_junction_near(junctions: &mut [Junction], pos: egui::Pos2, delta: Vec2) {
    for j in junctions.iter_mut() {
        if j.pos.distance(pos) <= JUNCTION_EPS {
            j.pos += delta;
        }
    }
}

fn translate_junction_at(junctions: &mut [Junction], old_pos: egui::Pos2, delta: Vec2) {
    for j in junctions.iter_mut() {
        if j.pos.distance(old_pos) <= JUNCTION_EPS {
            j.pos += delta;
        }
    }
}
