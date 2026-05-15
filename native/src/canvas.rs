//! Schematic canvas types and viewport math (world ↔ screen, grid snap).

use egui::{Pos2, Vec2};
use uuid::Uuid;

/// Canonical schematic grid (aligned with LLM + schematic_gen guidance).
pub const GRID_PX: f32 = 40.0;
pub const CANVAS_UNDO_CAP: usize = 100;

#[inline]
pub fn snap_world_pos(p: Pos2) -> Pos2 {
    Pos2::new(
        (p.x / GRID_PX).round() * GRID_PX,
        (p.y / GRID_PX).round() * GRID_PX,
    )
}

#[derive(Clone)]
pub struct Sym {
    pub ref_des: String,
    pub part_id: Option<Uuid>,
    pub pos: Pos2, // world coords
    /// Stored as degrees (matches `SchematicInstanceInput.rotation`).
    pub rotation_deg: f32,
    pub pins: Vec<String>,
    pub footprint_ref: Option<String>,
    /// Library symbol id (e.g. `Device:R`).
    pub symbol_id: Option<String>,
    /// Pin name + local offset (mm) relative to symbol center before rotation.
    pub pin_layout: Vec<(String, f32, f32)>,
}

/// First-class orthogonal wire segment (CAD geometry).
#[derive(Clone)]
pub struct WireSegment {
    pub id: uuid::Uuid,
    pub start: Pos2,
    pub end: Pos2,
    pub net: String,
}

impl WireSegment {
    pub fn new(start: Pos2, end: Pos2, net: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            start,
            end,
            net: net.into(),
        }
    }
}

/// Legacy pin-to-pin wire (converted to segments on load).
#[derive(Clone)]
pub struct Wire {
    pub a: String,
    pub a_pin: String,
    pub b: String,
    pub b_pin: String,
    pub net: String,
    pub bends: Vec<Pos2>,
}

/// Build Manhattan segments between two world points.
pub fn manhattan_segments(start: Pos2, end: Pos2, net: impl Into<String>) -> Vec<WireSegment> {
    let net = net.into();
    if (start.x - end.x).abs() < f32::EPSILON || (start.y - end.y).abs() < f32::EPSILON {
        return vec![WireSegment::new(start, end, net)];
    }
    let mid = Pos2::new(end.x, start.y);
    vec![
        WireSegment::new(start, mid, net.clone()),
        WireSegment::new(mid, end, net),
    ]
}

/// Convert legacy wire + symbol positions into segments.
pub fn wire_to_segments(w: &Wire, symbols: &[Sym]) -> Vec<WireSegment> {
    let a = symbols.iter().find(|s| s.ref_des == w.a);
    let b = symbols.iter().find(|s| s.ref_des == w.b);
    match (a, b) {
        (Some(sa), Some(sb)) => {
            let pa = symbol_pin_world(sa, &w.a_pin);
            let pb = symbol_pin_world(sb, &w.b_pin);
            let points = route_points(pa, &w.bends, pb);
            let mut out = Vec::new();
            for pair in points.windows(2) {
                out.push(WireSegment::new(pair[0], pair[1], w.net.clone()));
            }
            out
        }
        _ => vec![],
    }
}

const PIN_ATTACH_RADIUS: f32 = 18.0;

/// Pin names that have a segment endpoint within attach radius.
pub fn display_pins_for_symbol(sym: &Sym, segments: &[WireSegment]) -> Vec<String> {
    let mut pins = if sym.pins.is_empty() {
        vec!["1".to_string(), "2".to_string()]
    } else {
        sym.pins.clone()
    };
    for pin_name in pins.clone() {
        let pw = symbol_pin_world(sym, &pin_name);
        let attached = segments.iter().any(|seg| {
            seg.start.distance(pw) <= PIN_ATTACH_RADIUS || seg.end.distance(pw) <= PIN_ATTACH_RADIUS
        });
        if !attached && !sym.pins.is_empty() {
            // keep declared pins
        }
    }
    for seg in segments {
        for pin_name in pins.clone() {
            let pw = symbol_pin_world(sym, &pin_name);
            if seg.start.distance(pw) <= PIN_ATTACH_RADIUS
                || seg.end.distance(pw) <= PIN_ATTACH_RADIUS
            {
                if !pins.contains(&pin_name) {
                    pins.push(pin_name);
                }
            }
        }
    }
    pins.sort();
    pins.dedup();
    pins
}

/// If endpoint touches an existing segment (not at same point), insert a junction.
pub fn junction_at_wire_crossing(
    pos: Pos2,
    segments: &[WireSegment],
    junctions: &[Junction],
) -> bool {
    let on_junction = junctions.iter().any(|j| j.pos.distance(pos) <= 6.0);
    if on_junction {
        return false;
    }
    for seg in segments {
        if crate::util::dist_point_to_segment_px(pos, seg.start, seg.end) <= 4.0
            && pos.distance(seg.start) > 4.0
            && pos.distance(seg.end) > 4.0
        {
            return true;
        }
    }
    false
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinEndpoint {
    pub ref_des: String,
    pub pin_name: String,
}

#[derive(Clone)]
pub struct NetLabel {
    pub name: String,
    pub pos: Pos2,
    pub kind: tokito::models::NetLabelKind,
}

#[derive(Clone)]
pub struct Junction {
    pub pos: Pos2,
}

#[derive(Clone)]
pub struct NoConnect {
    pub pos: Pos2,
}

#[derive(Clone)]
pub struct PowerSymbol {
    pub name: String,
    pub pos: Pos2,
}

#[derive(Clone)]
pub struct TextItem {
    pub text: String,
    pub pos: Pos2,
}

#[derive(Clone)]
pub struct BusSegment {
    pub name: Option<String>,
    pub start: Pos2,
    pub end: Pos2,
}

#[derive(Clone)]
pub struct CanvasSnapshot {
    pub symbols: Vec<Sym>,
    pub wire_segments: Vec<WireSegment>,
    pub net_labels: Vec<NetLabel>,
    pub junctions: Vec<Junction>,
    pub no_connects: Vec<NoConnect>,
    pub power_symbols: Vec<PowerSymbol>,
    pub text_items: Vec<TextItem>,
    pub buses: Vec<BusSegment>,
}

pub fn symbol_pin_world(sym: &Sym, pin_name: &str) -> Pos2 {
    let local = if let Some((_, x, y)) = sym.pin_layout.iter().find(|(n, _, _)| n == pin_name) {
        Vec2::new(*x, *y)
    } else {
        let right_side = matches!(
            pin_name.trim().to_ascii_lowercase().as_str(),
            "2" | "b" | "out" | "vout" | "sda" | "scl" | "tx" | "miso"
        ) || pin_name.ends_with("_b");
        if right_side {
            Vec2::new(70.0, 0.0)
        } else {
            Vec2::new(-70.0, 0.0)
        }
    };
    let turns = ((sym.rotation_deg / 90.0).round() as i32).rem_euclid(4);
    let rotated = match turns {
        1 => Vec2::new(-local.y, local.x),
        2 => Vec2::new(-local.x, -local.y),
        3 => Vec2::new(local.y, -local.x),
        _ => local,
    };
    sym.pos + rotated
}

pub fn route_points(a: Pos2, bends: &[Pos2], b: Pos2) -> Vec<Pos2> {
    let mut points = Vec::with_capacity(bends.len() + 2);
    points.push(a);
    points.extend_from_slice(bends);
    points.push(b);
    points
}

pub fn route_segments(a: Pos2, bends: &[Pos2], b: Pos2) -> Vec<(Pos2, Pos2)> {
    route_points(a, bends, b)
        .windows(2)
        .map(|w| (w[0], w[1]))
        .collect()
}

pub fn manhattan_bends(a: Pos2, b: Pos2) -> Vec<Pos2> {
    if (a.x - b.x).abs() < f32::EPSILON || (a.y - b.y).abs() < f32::EPSILON {
        vec![]
    } else {
        vec![Pos2::new(b.x, a.y)]
    }
}

#[derive(Default)]
pub struct Viewport {
    pub pan: Vec2,
    pub zoom: f32,
}

impl Viewport {
    pub fn world_to_screen(&self, origin: Pos2, w: Pos2) -> Pos2 {
        origin + self.pan + (w.to_vec2() * self.zoom)
    }
    pub fn screen_to_world(&self, origin: Pos2, s: Pos2) -> Pos2 {
        let v = (s - origin - self.pan) / self.zoom;
        Pos2::new(v.x, v.y)
    }
}
