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
}

#[derive(Clone)]
pub struct Wire {
    pub a: String,
    pub a_pin: String,
    pub b: String,
    pub b_pin: String,
    pub net: String,
    pub bends: Vec<Pos2>,
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
    pub wires: Vec<Wire>,
    pub net_labels: Vec<NetLabel>,
    pub junctions: Vec<Junction>,
    pub no_connects: Vec<NoConnect>,
    pub power_symbols: Vec<PowerSymbol>,
    pub text_items: Vec<TextItem>,
    pub buses: Vec<BusSegment>,
}

pub fn display_pins_for_symbol(sym: &Sym, wires: &[Wire]) -> Vec<String> {
    let mut pins = if sym.pins.is_empty() {
        vec!["1".to_string(), "2".to_string()]
    } else {
        sym.pins.clone()
    };
    for w in wires {
        if w.a == sym.ref_des && !pins.contains(&w.a_pin) {
            pins.push(w.a_pin.clone());
        }
        if w.b == sym.ref_des && !pins.contains(&w.b_pin) {
            pins.push(w.b_pin.clone());
        }
    }
    pins.sort();
    pins
}

pub fn symbol_pin_world(sym: &Sym, pin_name: &str) -> Pos2 {
    let right_side = matches!(
        pin_name.trim().to_ascii_lowercase().as_str(),
        "2" | "b" | "out" | "vout" | "sda" | "scl" | "tx" | "miso"
    ) || pin_name.ends_with("_b");
    let local = if right_side {
        Vec2::new(70.0, 0.0)
    } else {
        Vec2::new(-70.0, 0.0)
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
