//! Schematic canvas types and viewport math (world ↔ screen, grid snap).

use std::collections::BTreeMap;

use egui::{Pos2, Vec2};
use uuid::Uuid;

/// Canonical schematic grid (aligned with LLM + schematic_gen guidance).
pub const GRID_PX: f32 = 40.0;
pub const CANVAS_UNDO_CAP: usize = 100;

/// Standard schematic grid pitch in millimetres (50 mil — same as common ECAD editors).
pub const SCH_MM_PER_GRID: f32 = 1.27;
/// Default pin-to-body spacing in millimetres (150 mil).
pub const SCH_PIN_PITCH_MM: f32 = 3.81;

/// World units per millimetre of library geometry (pins and graphics share this).
#[inline]
pub fn sch_world_per_mm() -> f32 {
    GRID_PX / SCH_MM_PER_GRID
}

/// ISO A4 schematic page (matches common ECAD sheet size in mm).
pub const SHEET_WIDTH_MM: f32 = 297.0;
pub const SHEET_HEIGHT_MM: f32 = 210.0;

/// Sheet border size in world units (centered on origin).
#[inline]
pub fn sheet_size_world() -> (f32, f32) {
    let wpm = sch_world_per_mm();
    (SHEET_WIDTH_MM * wpm, SHEET_HEIGHT_MM * wpm)
}

#[inline]
pub fn sheet_bounds_world() -> egui::Rect {
    let (w, h) = sheet_size_world();
    egui::Rect::from_center_size(egui::Pos2::ZERO, egui::vec2(w, h))
}

/// World-space symbol body half-extents fallback when pins are unknown.
pub const SYM_HALF_W: f32 = 56.0;
pub const SYM_HALF_H: f32 = 25.0;
/// Default pin distance from symbol origin for generated fallbacks.
pub fn pin_pitch_world() -> f32 {
    SCH_PIN_PITCH_MM * sch_world_per_mm()
}
pub const PIN_EDGE_WORLD: f32 = SCH_PIN_PITCH_MM * (GRID_PX / SCH_MM_PER_GRID);

/// Default schematic field height in mm (50 mil).
pub const SCH_FIELD_FONT_MM: f32 = 1.27;
/// Pin connection dot radius on canvas (screen px, before zoom clamp).
pub const PIN_VIS_RADIUS: f32 = 2.25;
pub const PIN_VIS_RADIUS_HOVER: f32 = 3.25;
/// Extra hit/paint margin around symbol pin bounds (world units).
pub const SYMBOL_HIT_PAD: f32 = 6.0;

#[inline]
pub fn snap_world_pos(p: Pos2) -> Pos2 {
    Pos2::new(
        (p.x / GRID_PX).round() * GRID_PX,
        (p.y / GRID_PX).round() * GRID_PX,
    )
}

/// Half-size of symbol hit/paint bounds from pin layout (world units).
pub fn symbol_hit_half_extents(sym: &Sym) -> (f32, f32) {
    if sym.pin_layout.is_empty() {
        return (SYM_HALF_W, SYM_HALF_H);
    }
    let mut hx = 16.0_f32;
    let mut hy = 16.0_f32;
    for (_, x, y) in &sym.pin_layout {
        hx = hx.max(x.abs());
        hy = hy.max(y.abs());
    }
    (hx + SYMBOL_HIT_PAD, hy + SYMBOL_HIT_PAD)
}

/// After placement, nudge the symbol so its first pin sits on the schematic grid.
pub fn snap_symbol_pins_to_grid(sym: &mut Sym) {
    sym.pos = snap_world_pos(sym.pos);
    let Some((ref pin_name, _, _)) = sym.pin_layout.first() else {
        return;
    };
    let pin_world = symbol_pin_world(sym, pin_name);
    let snapped = snap_world_pos(pin_world);
    sym.pos += snapped - pin_world;
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
    /// Schematic **Value** field (e.g. `10k`, `100n`, `MCP6002`).
    pub value: String,
    /// Extra symbol fields (`Footprint`, `Datasheet`, …).
    pub fields: BTreeMap<String, String>,
}

impl Sym {
    pub fn new_placed(
        ref_des: String,
        symbol_id: Option<String>,
        value: String,
        pos: Pos2,
        pins: Vec<String>,
        pin_layout: Vec<(String, f32, f32)>,
    ) -> Self {
        Self {
            ref_des,
            part_id: None,
            pos,
            rotation_deg: 0.0,
            pins,
            footprint_ref: None,
            symbol_id,
            pin_layout,
            value,
            fields: BTreeMap::new(),
        }
    }
}

/// First-class orthogonal wire segment (CAD geometry).
#[derive(Clone)]
pub struct WireSegment {
    pub id: uuid::Uuid,
    pub start: Pos2,
    pub end: Pos2,
    /// Topological net identity (stable across connected copper).
    pub net_id: uuid::Uuid,
    /// Human-readable net name (from labels / power / hints).
    pub net: String,
    /// Pin-attached start; position follows symbol when set.
    pub start_pin: Option<PinEndpoint>,
    /// Pin-attached end; position follows symbol when set.
    pub end_pin: Option<PinEndpoint>,
}

impl WireSegment {
    pub fn new(start: Pos2, end: Pos2, net: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            start,
            end,
            net_id: uuid::Uuid::new_v4(),
            net: net.into(),
            start_pin: None,
            end_pin: None,
        }
    }
}

/// Update segment endpoints that are anchored to symbol pins.
pub fn sync_anchored_wire_endpoints(segments: &mut [WireSegment], symbols: &[Sym]) {
    for seg in segments.iter_mut() {
        if let Some(pin) = &seg.start_pin {
            if let Some(sym) = symbols.iter().find(|s| s.ref_des == pin.ref_des) {
                seg.start = symbol_pin_world(sym, &pin.pin_name);
            }
        }
        if let Some(pin) = &seg.end_pin {
            if let Some(sym) = symbols.iter().find(|s| s.ref_des == pin.ref_des) {
                seg.end = symbol_pin_world(sym, &pin.pin_name);
            }
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

const ORTHO_EPS: f32 = 0.5;

/// True when a segment is strictly horizontal or vertical.
pub fn segment_is_orthogonal(seg: &WireSegment) -> bool {
    (seg.start.x - seg.end.x).abs() <= ORTHO_EPS || (seg.start.y - seg.end.y).abs() <= ORTHO_EPS
}

/// Snap wire vertex to schematic grid.
#[inline]
pub fn snap_wire_vertex(p: Pos2) -> Pos2 {
    snap_world_pos(p)
}

/// Build Manhattan (H/V only) segments between two points; endpoints snap to grid.
pub fn manhattan_segments(start: Pos2, end: Pos2, net: impl Into<String>) -> Vec<WireSegment> {
    let net = net.into();
    let start = snap_wire_vertex(start);
    let end = snap_wire_vertex(end);
    if start.distance(end) <= ORTHO_EPS {
        return vec![];
    }
    if (start.x - end.x).abs() <= ORTHO_EPS || (start.y - end.y).abs() <= ORTHO_EPS {
        return vec![WireSegment::new(start, end, net)];
    }
    // Prefer exiting horizontally first when x separation dominates (common for pin stubs).
    let mid = if (start.x - end.x).abs() >= (start.y - end.y).abs() {
        Pos2::new(end.x, start.y)
    } else {
        Pos2::new(start.x, end.y)
    };
    let mid = snap_wire_vertex(mid);
    vec![
        WireSegment::new(start, mid, net.clone()),
        WireSegment::new(mid, end, net),
    ]
}

/// Split any non-orthogonal segment into H/V Manhattan pieces.
pub fn orthogonalize_segments(segments: &[WireSegment]) -> Vec<WireSegment> {
    let mut out = Vec::new();
    for seg in segments {
        if segment_is_orthogonal(seg) {
            out.push(seg.clone());
        } else {
            let pieces = manhattan_segments(seg.start, seg.end, seg.net.clone());
            let n = pieces.len();
            for (i, mut s) in pieces.into_iter().enumerate() {
                if i == 0 {
                    s.start_pin = seg.start_pin.clone();
                }
                if i + 1 == n {
                    s.end_pin = seg.end_pin.clone();
                }
                out.push(s);
            }
        }
    }
    out
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
                out.extend(manhattan_segments(pair[0], pair[1], w.net.clone()));
            }
            if let Some(first) = out.first_mut() {
                first.start_pin = Some(PinEndpoint {
                    ref_des: w.a.clone(),
                    pin_name: w.a_pin.clone(),
                });
            }
            if let Some(last) = out.last_mut() {
                last.end_pin = Some(PinEndpoint {
                    ref_des: w.b.clone(),
                    pin_name: w.b_pin.clone(),
                });
            }
            out
        }
        _ => vec![],
    }
}

pub(crate) const PIN_ATTACH_RADIUS: f32 = 18.0;

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
    pub rotation_deg: f32,
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
            Vec2::new(pin_pitch_world(), 0.0)
        } else {
            Vec2::new(-pin_pitch_world(), 0.0)
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
    let mut out = Vec::new();
    let mut cur = snap_wire_vertex(a);
    for bend in bends {
        let next = snap_wire_vertex(*bend);
        for seg in manhattan_segments(cur, next, "") {
            out.push((seg.start, seg.end));
        }
        cur = next;
    }
    let end = snap_wire_vertex(b);
    for seg in manhattan_segments(cur, end, "") {
        out.push((seg.start, seg.end));
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;

    #[test]
    fn orthogonalize_splits_diagonal_segment() {
        let diag = WireSegment::new(Pos2::new(0.0, 0.0), Pos2::new(80.0, 80.0), "N");
        assert!(!segment_is_orthogonal(&diag));
        let fixed = orthogonalize_segments(&[diag]);
        assert!(fixed.len() >= 2);
        assert!(fixed.iter().all(segment_is_orthogonal));
    }

    #[test]
    fn sync_anchored_wire_endpoints_follows_symbol_move() {
        let mut sym = Sym {
            ref_des: "R1".into(),
            part_id: None,
            pos: Pos2::new(0.0, 0.0),
            rotation_deg: 0.0,
            pins: vec!["1".into(), "2".into()],
            footprint_ref: None,
            symbol_id: Some("Device:R".into()),
            pin_layout: vec![
                ("1".into(), -pin_pitch_world(), 0.0),
                ("2".into(), pin_pitch_world(), 0.0),
            ],
            value: "R".into(),
            fields: BTreeMap::new(),
        };
        let pin2 = symbol_pin_world(&sym, "2");
        let mut segments = vec![WireSegment {
            id: Uuid::new_v4(),
            start: Pos2::new(-pin_pitch_world(), 0.0),
            end: pin2,
            net_id: Uuid::new_v4(),
            net: "NET1".into(),
            start_pin: None,
            end_pin: Some(PinEndpoint {
                ref_des: "R1".into(),
                pin_name: "2".into(),
            }),
        }];
        sym.pos = Pos2::new(200.0, 100.0);
        sync_anchored_wire_endpoints(&mut segments, &[sym.clone()]);
        let expected = symbol_pin_world(&sym, "2");
        assert!(
            segments[0].end.distance(expected) < 0.01,
            "end should follow pin after move"
        );
    }

    #[test]
    fn wire_to_segments_sets_pin_anchors() {
        let sym_a = Sym {
            ref_des: "R1".into(),
            part_id: None,
            pos: Pos2::ZERO,
            rotation_deg: 0.0,
            pins: vec!["1".into(), "2".into()],
            footprint_ref: None,
            symbol_id: None,
            pin_layout: vec![
                ("1".into(), -pin_pitch_world(), 0.0),
                ("2".into(), pin_pitch_world(), 0.0),
            ],
            value: "10k".into(),
            fields: BTreeMap::new(),
        };
        let sym_b = Sym {
            ref_des: "C1".into(),
            part_id: None,
            pos: Pos2::new(200.0, 0.0),
            rotation_deg: 0.0,
            pins: vec!["1".into(), "2".into()],
            footprint_ref: None,
            symbol_id: None,
            pin_layout: vec![
                ("1".into(), -pin_pitch_world(), 0.0),
                ("2".into(), pin_pitch_world(), 0.0),
            ],
            value: "100n".into(),
            fields: BTreeMap::new(),
        };
        let w = Wire {
            a: "R1".into(),
            a_pin: "2".into(),
            b: "C1".into(),
            b_pin: "1".into(),
            net: "N1".into(),
            bends: vec![],
        };
        let segs = wire_to_segments(&w, &[sym_a, sym_b]);
        assert!(!segs.is_empty());
        assert_eq!(
            segs.first().unwrap().start_pin.as_ref().unwrap().ref_des,
            "R1"
        );
        assert_eq!(segs.last().unwrap().end_pin.as_ref().unwrap().ref_des, "C1");
    }
}
