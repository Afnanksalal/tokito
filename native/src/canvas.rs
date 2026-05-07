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
}

#[derive(Clone)]
pub struct Wire {
    pub a: String,
    pub b: String,
    pub net: String,
}

#[derive(Clone)]
pub struct CanvasSnapshot {
    pub symbols: Vec<Sym>,
    pub wires: Vec<Wire>,
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
