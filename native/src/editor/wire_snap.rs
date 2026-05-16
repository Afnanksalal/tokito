//! Wire-tool snap targets (pins, junctions, existing copper).

use egui::Pos2;

use super::hit_test::{pick_junction, pick_wire_segment, PIN_HIT_RADIUS};
use crate::canvas::{symbol_pin_world, Junction, PinEndpoint, Sym, Viewport, WireSegment};

/// Where the wire tool may start or bend.
#[derive(Debug, Clone, PartialEq)]
pub enum WireSnap {
    Pin(PinEndpoint),
    Junction(Pos2),
    WireEnd {
        world: Pos2,
        segment_index: usize,
        is_end: bool,
    },
}

/// Nearest legal wire attachment at `pointer` (screen space).
pub fn wire_snap_at(
    pointer: Pos2,
    origin: Pos2,
    viewport: &Viewport,
    symbols: &[Sym],
    segments: &[WireSegment],
    junctions: &[Junction],
    wire_segments_for_pins: &[WireSegment],
) -> Option<WireSnap> {
    if let Some(pin) = pick_pin_at(pointer, origin, viewport, symbols, wire_segments_for_pins) {
        return Some(WireSnap::Pin(pin));
    }
    if let Some(i) = pick_junction(pointer, origin, viewport, junctions) {
        return Some(WireSnap::Junction(junctions[i].pos));
    }
    if let Some(i) = pick_wire_segment(pointer, origin, viewport, segments, 12.0) {
        let seg = &segments[i];
        let a = viewport.world_to_screen(origin, seg.start);
        let b = viewport.world_to_screen(origin, seg.end);
        let da = pointer.distance(a);
        let db = pointer.distance(b);
        if da <= 14.0 || db <= 14.0 {
            let (world, is_end) = if da <= db {
                (seg.start, true)
            } else {
                (seg.end, false)
            };
            return Some(WireSnap::WireEnd {
                world,
                segment_index: i,
                is_end,
            });
        }
    }
    None
}

fn pick_pin_at(
    pointer: Pos2,
    origin: Pos2,
    viewport: &Viewport,
    symbols: &[Sym],
    wire_segments: &[WireSegment],
) -> Option<PinEndpoint> {
    let mut best: Option<(PinEndpoint, f32)> = None;
    for sym in symbols {
        if let Some(pin) =
            super::hit_test::pick_pin_on_symbol(pointer, origin, viewport, sym, wire_segments)
        {
            let pin_world = symbol_pin_world(sym, &pin.pin_name);
            let d = viewport
                .world_to_screen(origin, pin_world)
                .distance(pointer);
            if d <= PIN_HIT_RADIUS && best.as_ref().map(|(_, bd)| d < *bd).unwrap_or(true) {
                best = Some((pin, d));
            }
        }
    }
    best.map(|(p, _)| p)
}
