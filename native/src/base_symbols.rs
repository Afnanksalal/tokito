//! Bundled base symbol library for canvas rendering (`.tokito_sym` on disk).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use egui::{Color32, Painter, Pos2, Rect, Stroke};

use crate::symbol_format::{Symbol, SymbolGraphic, SymbolLibFile};
use crate::symbols_draw::{self, CompKind};

#[derive(Clone, Copy)]
pub struct SymbolPaintSpec {
    pub pivot: Pos2,
    pub lw: f32,
    pub lh: f32,
    pub rot_deg: f32,
    pub kind: CompKind,
    pub ink: Color32,
    pub stroke_px: f32,
    pub outline: Color32,
}

impl SymbolPaintSpec {
    pub fn new(
        pivot: Pos2,
        lw: f32,
        lh: f32,
        rot_deg: f32,
        kind: CompKind,
        ink: Color32,
        stroke_px: f32,
        outline: Color32,
    ) -> Self {
        Self {
            pivot,
            lw,
            lh,
            rot_deg,
            kind,
            ink,
            stroke_px,
            outline,
        }
    }
}

pub struct BaseSymbolLibrary {
    by_full_name: HashMap<String, Symbol>,
    by_short_name: HashMap<String, String>,
}

impl BaseSymbolLibrary {
    pub fn open() -> anyhow::Result<Self> {
        let dir = bundled_dir();
        let mut lib = Self {
            by_full_name: HashMap::new(),
            by_short_name: HashMap::new(),
        };
        if !dir.is_dir() {
            anyhow::bail!("missing bundled symbols at {}", dir.display());
        }
        lib.load_directory(&dir)?;
        let user = crate::symbol_library::user_symbols_root();
        if user.is_dir() {
            let _ = lib.load_directory(&user);
        }
        if lib.by_full_name.is_empty() {
            anyhow::bail!("no symbols found in {}", dir.display());
        }
        Ok(lib)
    }

    pub fn contains(&self, full_name: &str) -> bool {
        self.by_full_name.contains_key(full_name)
    }

    pub fn pin_layout_for(&self, full_name: &str) -> Vec<(String, f32, f32)> {
        self.by_full_name
            .get(full_name)
            .map(|s| s.pins.iter().map(|p| (p.name.clone(), p.x, p.y)).collect())
            .unwrap_or_default()
    }

    /// Best bundled symbol for a refdes prefix (`R` → `Device:R`, etc.).
    pub fn default_for_prefix(&self, prefix: &str) -> Option<(String, Vec<(String, f32, f32)>)> {
        let short = match prefix {
            "R" => "R",
            "C" => "C",
            "L" => "L",
            "D" => "D",
            "Q" => "BC547",
            "J" => "Conn_01x02",
            _ => "Conn_01x04",
        };
        let full = self.by_short_name.get(short)?.clone();
        let pins = self.pin_layout_for(&full);
        Some((full, pins))
    }

    fn load_directory(&mut self, dir: &Path) -> anyhow::Result<()> {
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if !path
                .to_str()
                .is_some_and(|p| p.ends_with(&format!(".{}", SymbolLibFile::EXTENSION)))
            {
                continue;
            }
            let lib_prefix = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .filter(|n| *n != "base-symbols" && !n.ends_with(".symdir"));
            self.load_file(path, lib_prefix)?;
        }
        Ok(())
    }

    fn load_file(&mut self, path: &Path, lib_prefix: Option<&str>) -> anyhow::Result<()> {
        let Ok(doc) = SymbolLibFile::read(path) else {
            return Ok(());
        };
        for s in doc.symbols() {
            let short = s.name.clone();
            let full = match lib_prefix {
                Some(lib) => format!("{lib}:{short}"),
                None => short.clone(),
            };
            self.by_short_name
                .entry(short)
                .or_insert_with(|| full.clone());
            self.by_full_name.insert(full, s.clone());
        }
        Ok(())
    }

    pub fn symbol_count(&self) -> usize {
        self.by_full_name.len()
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<String> {
        let q = query.trim().to_ascii_lowercase();
        if q.is_empty() {
            return self.by_full_name.keys().take(limit).cloned().collect();
        }
        let mut hits: Vec<String> = self
            .by_full_name
            .keys()
            .filter(|n| n.to_ascii_lowercase().contains(&q))
            .cloned()
            .collect();
        hits.sort();
        hits.truncate(limit);
        hits
    }

    pub fn resolve_symbol_name_for_kind(&self, kind: CompKind) -> Option<String> {
        let candidates: &[&str] = match kind {
            CompKind::Resistor => &["Device:R", "R"],
            CompKind::Capacitor => &["Device:C", "C"],
            CompKind::Inductor => &["Device:L", "L"],
            CompKind::Diode => &["Device:D", "D", "Device:D_Schottky", "D_Schottky"],
            CompKind::Transistor => &["Transistor_BJT:BC547", "BC547"],
            CompKind::IC => &["Device:U", "U", "Amplifier_Operational:LM358"],
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

    pub fn paint_named_or_fallback(
        &self,
        painter: &Painter,
        spec: SymbolPaintSpec,
        full_name: &str,
    ) {
        if let Some(sym) = self.by_full_name.get(full_name) {
            if paint_symbol_graphics(painter, spec, sym) {
                return;
            }
        }
        symbols_draw::paint_symbol_body(
            painter,
            spec.pivot,
            spec.lw,
            spec.lh,
            spec.rot_deg,
            spec.kind,
            spec.ink,
            spec.stroke_px,
        );
    }

    pub fn paint_kind_or_fallback(&self, painter: &Painter, spec: SymbolPaintSpec) {
        let Some(sym_name) = self.resolve_symbol_name_for_kind(spec.kind) else {
            symbols_draw::paint_symbol_body(
                painter,
                spec.pivot,
                spec.lw,
                spec.lh,
                spec.rot_deg,
                spec.kind,
                spec.ink,
                spec.stroke_px,
            );
            return;
        };
        self.paint_named_or_fallback(painter, spec, &sym_name);
    }
}

pub fn bundled_dir() -> PathBuf {
    crate::paths::bundled_symbols_dir()
}

fn paint_symbol_graphics(painter: &Painter, spec: SymbolPaintSpec, sym: &Symbol) -> bool {
    if sym.graphics.is_empty() {
        return false;
    }

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

    let sx = (spec.lw * 1.6) / w;
    let sy = (spec.lh * 1.6) / h;
    let s = sx.min(sy);
    let center = Pos2::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5);

    let outline_w = (spec.stroke_px + 1.1).clamp(1.8, 4.5);
    let outline = Stroke::new(outline_w, spec.outline);
    let stroke = Stroke::new(spec.stroke_px.clamp(1.0, 3.5), spec.ink);

    let xf = |p: Pos2| -> Pos2 {
        let px = (p.x - center.x) * s;
        let py = -(p.y - center.y) * s;
        symbols_draw::xf_public(Pos2::new(px, py), spec.pivot, spec.rot_deg)
    };

    for g in &sym.graphics {
        match g.kind.as_str() {
            "line" => {
                if let (Some(a), Some(b)) = (g.start, g.end) {
                    let a = Pos2::new(a[0] as f32, a[1] as f32);
                    let b = Pos2::new(b[0] as f32, b[1] as f32);
                    let seg = [xf(a), xf(b)];
                    painter.line_segment(seg, outline);
                    painter.line_segment(seg, stroke);
                }
            }
            "rectangle" => {
                if let (Some(a), Some(b)) = (g.start, g.end) {
                    let a = Pos2::new(a[0] as f32, a[1] as f32);
                    let b = Pos2::new(b[0] as f32, b[1] as f32);
                    let tl = xf(Pos2::new(a.x.min(b.x), a.y.min(b.y)));
                    let br = xf(Pos2::new(a.x.max(b.x), a.y.max(b.y)));
                    let rr = Rect::from_two_pos(tl, br);
                    painter.rect_stroke(rr, 0.0, outline);
                    painter.rect_stroke(rr, 0.0, stroke);
                }
            }
            "circle" => {
                if let (Some(c), Some(r)) = (g.center, g.radius) {
                    let c = Pos2::new(c[0] as f32, c[1] as f32);
                    let r = (r as f32).abs() * s;
                    let pc = xf(c);
                    painter.circle_stroke(pc, r, outline);
                    painter.circle_stroke(pc, r, stroke);
                } else if let (Some(c), Some(a)) = (g.center, g.start) {
                    let c = Pos2::new(c[0] as f32, c[1] as f32);
                    let a = Pos2::new(a[0] as f32, a[1] as f32);
                    let r = ((a.x - c.x).hypot(a.y - c.y) * s).abs();
                    let pc = xf(c);
                    painter.circle_stroke(pc, r, outline);
                    painter.circle_stroke(pc, r, stroke);
                }
            }
            "arc" => {
                if let (Some(c), Some(a), Some(b)) = (g.center, g.start, g.end) {
                    let c = Pos2::new(c[0] as f32, c[1] as f32);
                    let a = Pos2::new(a[0] as f32, a[1] as f32);
                    let b = Pos2::new(b[0] as f32, b[1] as f32);
                    paint_arc_approx(painter, xf, c, a, b, outline);
                    paint_arc_approx(painter, xf, c, a, b, stroke);
                }
            }
            _ => {}
        }
    }

    true
}

fn bounds_for_graphic(g: &SymbolGraphic) -> Option<(Pos2, Pos2)> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_base_symbols_load() {
        let lib = BaseSymbolLibrary::open().expect("bundled symbols");
        assert!(lib.symbol_count() > 10);
        assert!(lib.contains("Device:R"));
    }
}
