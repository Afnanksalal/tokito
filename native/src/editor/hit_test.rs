//! Screen-space picking for symbols, pins, wires, and annotations.

use egui::{Pos2, Rect, Vec2};

use crate::canvas::Viewport;
use crate::canvas::{
    display_pins_for_symbol, symbol_hit_half_extents, symbol_pin_world, BusSegment, Junction,
    NetLabel, NoConnect, PinEndpoint, PowerSymbol, Sym, TextItem, WireSegment,
};
use crate::util;

/// Pin hit radius in screen pixels.
pub const PIN_HIT_RADIUS: f32 = 12.0;

pub struct HoverState {
    pub symbol: Option<String>,
    pub pin: Option<PinEndpoint>,
}

pub fn hover_at(
    pointer: Pos2,
    canvas_rect: Rect,
    origin: Pos2,
    viewport: &Viewport,
    symbols: &[Sym],
    segments: &[WireSegment],
) -> HoverState {
    if !canvas_rect.contains(pointer) {
        return HoverState {
            symbol: None,
            pin: None,
        };
    }

    let mut symbol = None;
    let mut pin = None;

    for s in symbols {
        let center = viewport.world_to_screen(origin, s.pos);
        let z = viewport.zoom;
        let (hx, hy) = symbol_hit_half_extents(s);
        let size = Vec2::new(hx * 2.0 * z, hy * 2.0 * z);
        let r = Rect::from_center_size(center, size);
        if !r.contains(pointer) {
            continue;
        }
        pin = pick_pin_on_symbol(pointer, origin, viewport, s, segments);
        if pin.is_some() {
            symbol = Some(s.ref_des.clone());
            break;
        }
        symbol = Some(s.ref_des.clone());
    }

    if pin.is_none() {
        for s in symbols {
            if let Some(p) = pick_pin_on_symbol(pointer, origin, viewport, s, segments) {
                symbol = Some(s.ref_des.clone());
                pin = Some(p);
                break;
            }
        }
    }

    HoverState { symbol, pin }
}

pub fn pick_pin_on_symbol(
    pointer: Pos2,
    origin: Pos2,
    viewport: &Viewport,
    sym: &Sym,
    segments: &[WireSegment],
) -> Option<PinEndpoint> {
    let mut best: Option<(PinEndpoint, f32)> = None;
    for pin_name in display_pins_for_symbol(sym, segments) {
        let pin_world = symbol_pin_world(sym, &pin_name);
        let pin_screen = viewport.world_to_screen(origin, pin_world);
        let d = pin_screen.distance(pointer);
        if d <= PIN_HIT_RADIUS && best.as_ref().map(|(_, bd)| d < *bd).unwrap_or(true) {
            best = Some((
                PinEndpoint {
                    ref_des: sym.ref_des.clone(),
                    pin_name,
                },
                d,
            ));
        }
    }
    best.map(|(pin, _)| pin)
}

pub fn pick_wire_segment(
    pointer: Pos2,
    origin: Pos2,
    viewport: &Viewport,
    segments: &[WireSegment],
    threshold: f32,
) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (i, seg) in segments.iter().enumerate() {
        let a = viewport.world_to_screen(origin, seg.start);
        let b = viewport.world_to_screen(origin, seg.end);
        let d = util::dist_point_to_segment_px(pointer, a, b);
        if d < threshold && best.as_ref().map(|(_, bd)| d < *bd).unwrap_or(true) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

pub fn pick_net_label(
    pointer: Pos2,
    origin: Pos2,
    viewport: &Viewport,
    labels: &[NetLabel],
) -> Option<usize> {
    for (i, label) in labels.iter().enumerate() {
        let p = viewport.world_to_screen(origin, label.pos);
        let width = (label.name.len() as f32 * 8.0).max(24.0);
        if Rect::from_min_size(p + Vec2::new(4.0, -16.0), Vec2::new(width, 22.0)).contains(pointer)
        {
            return Some(i);
        }
    }
    None
}

pub fn pick_junction(
    pointer: Pos2,
    origin: Pos2,
    viewport: &Viewport,
    junctions: &[Junction],
) -> Option<usize> {
    for (i, j) in junctions.iter().enumerate() {
        let p = viewport.world_to_screen(origin, j.pos);
        if p.distance(pointer) <= 10.0 {
            return Some(i);
        }
    }
    None
}

pub fn pick_no_connect(
    pointer: Pos2,
    origin: Pos2,
    viewport: &Viewport,
    items: &[NoConnect],
) -> Option<usize> {
    for (i, nc) in items.iter().enumerate() {
        let p = viewport.world_to_screen(origin, nc.pos);
        if p.distance(pointer) <= 10.0 {
            return Some(i);
        }
    }
    None
}

pub fn pick_power_symbol(
    pointer: Pos2,
    origin: Pos2,
    viewport: &Viewport,
    items: &[PowerSymbol],
) -> Option<usize> {
    for (i, pwr) in items.iter().enumerate() {
        let p = viewport.world_to_screen(origin, pwr.pos);
        if Rect::from_center_size(p + Vec2::new(0.0, -18.0), Vec2::new(48.0, 48.0))
            .contains(pointer)
        {
            return Some(i);
        }
    }
    None
}

pub fn pick_bus(
    pointer: Pos2,
    origin: Pos2,
    viewport: &Viewport,
    buses: &[BusSegment],
) -> Option<usize> {
    for (i, bus) in buses.iter().enumerate() {
        let a = viewport.world_to_screen(origin, bus.start);
        let b = viewport.world_to_screen(origin, bus.end);
        if util::dist_point_to_segment_px(pointer, a, b) <= 12.0 {
            return Some(i);
        }
    }
    None
}

pub fn pick_text_item(
    pointer: Pos2,
    origin: Pos2,
    viewport: &Viewport,
    items: &[TextItem],
) -> Option<usize> {
    for (i, text) in items.iter().enumerate() {
        let p = viewport.world_to_screen(origin, text.pos);
        let width = (text.text.len() as f32 * 8.0).max(36.0);
        if Rect::from_min_size(p, Vec2::new(width, 22.0)).contains(pointer) {
            return Some(i);
        }
    }
    None
}

pub fn pick_erc_marker(
    pointer: Pos2,
    origin: Pos2,
    viewport: &Viewport,
    markers: &[super::ErcMarkerOnCanvas],
) -> Option<usize> {
    for (i, m) in markers.iter().enumerate() {
        let p = viewport.world_to_screen(origin, m.position);
        if Rect::from_center_size(p, Vec2::new(24.0, 24.0)).contains(pointer) {
            return Some(i);
        }
    }
    None
}
