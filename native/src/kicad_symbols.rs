//! Optional KiCad symbol library provider (.kicad_sym) for rendering symbols on the canvas.
//!
//! This is intentionally "best-effort": if the library can't be loaded or a symbol isn't found,
//! we fall back to Tokito's built-in stroke symbols.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use egui::{Color32, Painter, Pos2, Rect, Stroke};

use crate::symbols_draw::{self, CompKind};

pub struct KicadSymbolLibrary {
    #[allow(dead_code)]
    pub path: PathBuf,
    by_full_name: HashMap<String, kiutils_kicad::Symbol>,
    by_short_name: HashMap<String, String>, // short -> full
}

impl KicadSymbolLibrary {
    pub fn try_load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let doc = kiutils_kicad::SymbolLibFile::read(&path)?;

        let mut by_full_name = HashMap::new();
        let mut by_short_name = HashMap::new();

        for s in doc.ast().symbols.iter() {
            let Some(name) = s.name.clone() else {
                continue;
            };
            by_short_name
                .entry(short_name(&name).to_string())
                .or_insert_with(|| name.clone());
            by_full_name.insert(name, s.clone());
        }

        Ok(Self {
            path,
            by_full_name,
            by_short_name,
        })
    }

    pub fn resolve_symbol_name_for_kind(&self, kind: CompKind) -> Option<String> {
        // KiCad defaults usually use `Device:*` for basic passives.
        // We attempt full name first, then a short-name match.
        let candidates: &[&str] = match kind {
            CompKind::Resistor => &["Device:R", "R"],
            CompKind::Capacitor => &["Device:C", "C"],
            CompKind::Inductor => &["Device:L", "L"],
            CompKind::Diode => &["Device:D", "D", "Device:D_Schottky", "D_Schottky"],
            CompKind::Transistor => &[
                "Device:Q_NPN_EBC",
                "Q_NPN_EBC",
                "Device:Q_NPN_BCE",
                "Q_NPN_BCE",
                "Device:Q_PNP_EBC",
                "Q_PNP_EBC",
            ],
            CompKind::IC => &["Device:U", "U"],
            CompKind::Generic => &["Device:Generic", "Generic"],
        };

        for c in candidates {
            if self.by_full_name.contains_key(*c) {
                return Some((*c).to_string());
            }
            if let Some(full) = self.by_short_name.get(*c) {
                return Some(full.clone());
            }
        }
        None
    }

    pub fn paint_kind_or_fallback(
        &self,
        painter: &Painter,
        pivot: Pos2,
        lw: f32,
        lh: f32,
        rot_deg: f32,
        kind: CompKind,
        ink: Color32,
        stroke_px: f32,
    ) {
        let Some(sym_name) = self.resolve_symbol_name_for_kind(kind) else {
            symbols_draw::paint_symbol_body(painter, pivot, lw, lh, rot_deg, kind, ink, stroke_px);
            return;
        };
        let Some(sym) = self.by_full_name.get(&sym_name) else {
            symbols_draw::paint_symbol_body(painter, pivot, lw, lh, rot_deg, kind, ink, stroke_px);
            return;
        };

        if !paint_kicad_symbol_graphics(painter, pivot, lw, lh, rot_deg, sym, ink, stroke_px) {
            symbols_draw::paint_symbol_body(painter, pivot, lw, lh, rot_deg, kind, ink, stroke_px);
        }
    }
}

fn short_name(full: &str) -> &str {
    full.rsplit(':').next().unwrap_or(full)
}

fn paint_kicad_symbol_graphics(
    painter: &Painter,
    pivot: Pos2,
    lw: f32,
    lh: f32,
    rot_deg: f32,
    sym: &kiutils_kicad::Symbol,
    ink: Color32,
    stroke_px: f32,
) -> bool {
    // If there are no graphics, there is nothing to render.
    if sym.graphics.is_empty() {
        return false;
    }

    // Collect bounds from supported primitives.
    let mut min = Pos2::new(f32::INFINITY, f32::INFINITY);
    let mut max = Pos2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut any = false;

    for g in &sym.graphics {
        if let Some((gmin, gmax)) = bounds_for_graphic(g) {
            min.x = min.x.min(gmin.x);
            min.y = min.y.min(gmin.y);
            max.x = max.x.max(gmax.x);
            max.y = max.y.max(gmax.y);
            any = true;
        }
    }

    if !any {
        return false;
    }

    let w = (max.x - min.x).max(1.0);
    let h = (max.y - min.y).max(1.0);

    // Fit into the target box. KiCad units are arbitrary here; we just normalize.
    // Flip Y because KiCad's +Y is typically "up", while egui's +Y is "down".
    let sx = (lw * 1.6) / w;
    let sy = (lh * 1.6) / h;
    let s = sx.min(sy);
    let center = Pos2::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5);

    let stroke = Stroke::new(stroke_px.clamp(1.0, 3.5), ink);

    let xf = |p: Pos2| -> Pos2 {
        let px = (p.x - center.x) as f32 * s;
        let py = -((p.y - center.y) as f32) * s;
        symbols_draw::xf_public(Pos2::new(px, py), pivot, rot_deg)
    };

    for g in &sym.graphics {
        match g.token.as_str() {
            "line" => {
                if let (Some(a), Some(b)) = (g.start, g.end) {
                    let a = Pos2::new(a[0] as f32, a[1] as f32);
                    let b = Pos2::new(b[0] as f32, b[1] as f32);
                    painter.line_segment([xf(a), xf(b)], stroke);
                }
            }
            "rectangle" => {
                if let (Some(a), Some(b)) = (g.start, g.end) {
                    let a = Pos2::new(a[0] as f32, a[1] as f32);
                    let b = Pos2::new(b[0] as f32, b[1] as f32);
                    let tl = xf(Pos2::new(a.x.min(b.x), a.y.min(b.y)));
                    let br = xf(Pos2::new(a.x.max(b.x), a.y.max(b.y)));
                    painter.rect_stroke(Rect::from_two_pos(tl, br), 0.0, stroke);
                }
            }
            "circle" => {
                if let (Some(c), Some(r)) = (g.center, g.radius) {
                    let c = Pos2::new(c[0] as f32, c[1] as f32);
                    let r = (r as f32).abs() * s;
                    painter.circle_stroke(xf(c), r, stroke);
                } else if let (Some(c), Some(a)) = (g.center, g.start) {
                    // Some files encode radius via center+start.
                    let c = Pos2::new(c[0] as f32, c[1] as f32);
                    let a = Pos2::new(a[0] as f32, a[1] as f32);
                    let r = ((a.x - c.x).hypot(a.y - c.y) * s).abs();
                    painter.circle_stroke(xf(c), r, stroke);
                }
            }
            "arc" => {
                if let (Some(c), Some(a), Some(b)) = (g.center, g.start, g.end) {
                    let c = Pos2::new(c[0] as f32, c[1] as f32);
                    let a = Pos2::new(a[0] as f32, a[1] as f32);
                    let b = Pos2::new(b[0] as f32, b[1] as f32);
                    paint_arc_approx(painter, xf, c, a, b, stroke);
                }
            }
            _ => {}
        }
    }

    true
}

fn bounds_for_graphic(g: &kiutils_kicad::SymGraphic) -> Option<(Pos2, Pos2)> {
    let mut min = Pos2::new(f32::INFINITY, f32::INFINITY);
    let mut max = Pos2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
    let mut any = false;

    let mut push = |p: Pos2| {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
        any = true;
    };

    if let Some(s) = g.start {
        push(Pos2::new(s[0] as f32, s[1] as f32));
    }
    if let Some(e) = g.end {
        push(Pos2::new(e[0] as f32, e[1] as f32));
    }
    if let Some(c) = g.center {
        let c = Pos2::new(c[0] as f32, c[1] as f32);
        push(c);
        if let Some(r) = g.radius {
            let r = r as f32;
            push(Pos2::new(c.x - r, c.y - r));
            push(Pos2::new(c.x + r, c.y + r));
        }
    }

    if any {
        Some((min, max))
    } else {
        None
    }
}

fn paint_arc_approx(
    painter: &Painter,
    xf: impl Fn(Pos2) -> Pos2,
    center: Pos2,
    start: Pos2,
    end: Pos2,
    stroke: Stroke,
) {
    let a0 = (start.y - center.y).atan2(start.x - center.x);
    let a1 = (end.y - center.y).atan2(end.x - center.x);
    let r = (start.x - center.x).hypot(start.y - center.y).max(0.001);

    // Shortest sweep approximation.
    let mut da = a1 - a0;
    while da > std::f32::consts::PI {
        da -= std::f32::consts::TAU;
    }
    while da < -std::f32::consts::PI {
        da += std::f32::consts::TAU;
    }

    let steps = ((da.abs() * 18.0).clamp(8.0, 64.0)) as usize;
    let mut prev = start;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let a = a0 + da * t;
        let p = Pos2::new(center.x + r * a.cos(), center.y + r * a.sin());
        painter.line_segment([xf(prev), xf(p)], stroke);
        prev = p;
    }
}

