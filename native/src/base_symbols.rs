//! Bundled base symbol library for canvas rendering (`.tokito_sym` on disk).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use egui::{Color32, Painter, Pos2, Rect, Stroke};

use crate::canvas::{pin_pitch_world, sch_world_per_mm, Viewport, SCH_FIELD_FONT_MM};
use crate::component_value;
use crate::symbol_format::{Symbol, SymbolGraphic, SymbolLibFile, SymbolPin};
use crate::symbols_draw::{self, CompKind};

/// Paint a library symbol on the schematic canvas (mm-accurate, shared with pin layout).
pub struct CanvasSymbolPaint<'a> {
    pub painter: &'a Painter,
    pub viewport: &'a Viewport,
    pub origin: Pos2,
    pub sym_pos: Pos2,
    pub rot_deg: f32,
    pub ink: Color32,
    pub outline: Color32,
    pub stroke_px: f32,
}

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
        lib.resolve_extends();
        Ok(lib)
    }

    /// Merge `(extends "Parent")` graphics/pins and apply fallbacks for empty symbols.
    fn resolve_extends(&mut self) {
        for _ in 0..12 {
            let keys: Vec<String> = self.by_full_name.keys().cloned().collect();
            let mut changed = false;
            for key in keys {
                let empty = self
                    .by_full_name
                    .get(&key)
                    .is_some_and(|s| s.graphics.is_empty() && s.pins.is_empty());
                if !empty {
                    continue;
                }
                let Some(ext) = self.by_full_name.get(&key).and_then(|s| s.extends.clone()) else {
                    continue;
                };
                let Some(parent_key) = self.find_symbol_key(&ext).or_else(|| {
                    Self::fallback_template_for_key(&key).and_then(|tpl| self.find_symbol_key(tpl))
                }) else {
                    continue;
                };
                let Some(parent) = self.by_full_name.get(&parent_key).cloned() else {
                    continue;
                };
                if parent.graphics.is_empty() && parent.pins.is_empty() {
                    continue;
                }
                if let Some(sym) = self.by_full_name.get_mut(&key) {
                    sym.graphics = parent.graphics.clone();
                    sym.pins = parent.pins.clone();
                    if sym.properties.is_empty() {
                        sym.properties = parent.properties.clone();
                    }
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let keys: Vec<String> = self.by_full_name.keys().cloned().collect();
        for key in keys {
            let empty = self
                .by_full_name
                .get(&key)
                .is_some_and(|s| s.graphics.is_empty() && s.pins.is_empty());
            if !empty {
                continue;
            }
            let Some(tpl) = Self::fallback_template_for_key(&key) else {
                continue;
            };
            let Some(tpl_key) = self.find_symbol_key(tpl) else {
                continue;
            };
            let Some(src) = self.by_full_name.get(&tpl_key).cloned() else {
                continue;
            };
            if src.graphics.is_empty() && src.pins.is_empty() {
                continue;
            }
            if let Some(sym) = self.by_full_name.get_mut(&key) {
                sym.graphics = src.graphics;
                sym.pins = src.pins;
                if sym.properties.is_empty() {
                    sym.properties = src.properties.clone();
                }
            }
        }
    }

    fn find_symbol_key(&self, short_or_full: &str) -> Option<String> {
        if self.by_full_name.contains_key(short_or_full) {
            return Some(short_or_full.to_string());
        }
        if let Some(full) = self.by_short_name.get(short_or_full) {
            return Some(full.clone());
        }
        let suffix = format!(":{short_or_full}");
        self.by_full_name
            .keys()
            .find(|k| k.ends_with(&suffix))
            .cloned()
    }

    fn fallback_template_for_key(full_key: &str) -> Option<&'static str> {
        let lib = full_key.split(':').next().unwrap_or("");
        match lib {
            "Amplifier_Operational" | "Amplifier_Instrumentation" => Some("Interface:MCP6002"),
            "Diode" => Some("Device:D"),
            _ => None,
        }
    }

    pub fn contains(&self, full_name: &str) -> bool {
        self.by_full_name.contains_key(full_name)
    }

    pub fn symbol(&self, full_name: &str) -> Option<&Symbol> {
        self.by_full_name.get(full_name)
    }

    /// Default **Value** from library property or heuristic from symbol id.
    pub fn default_value_for(&self, full_name: &str) -> String {
        if let Some(sym) = self.symbol(full_name) {
            if let Some(prop) = sym.properties.iter().find(|p| p.name == "Value") {
                if !prop.default_text.is_empty() {
                    return prop.default_text.clone();
                }
            }
        }
        component_value::default_value_for_library_id(full_name)
    }

    pub fn pin_layout_for(&self, full_name: &str) -> Vec<(String, f32, f32)> {
        if let Some(s) = self.by_full_name.get(full_name) {
            if !s.pins.is_empty() {
                return library_pins_to_world(&s.pins);
            }
        }
        default_pin_layout_for_prefix(Self::refdes_prefix_for_library_id(full_name))
    }

    /// Refdes letter prefix for a library symbol id (`Device:R` → `R`, `Amplifier_Operational:LM358` → `U`).
    pub fn refdes_prefix_for_library_id(full_name: &str) -> &'static str {
        let (lib, short) = match full_name.split_once(':') {
            Some((l, s)) => (l, s),
            None => ("", full_name),
        };
        match lib {
            "Device" => match short.chars().next() {
                Some('R') => "R",
                Some('C') => "C",
                Some('L') => "L",
                Some('D') => "D",
                Some('Q') => "Q",
                Some('J') => "J",
                _ => "U",
            },
            "Connector" | "Connector_Generic" | "Connector_Conn" => "J",
            "Regulator_Linear" | "Regulator_Switching" | "Regulator_Controller" => "U",
            "Amplifier_Operational" | "Amplifier_Instrumentation" | "Amplifier_Current" => "U",
            "Transistor_BJT" | "Transistor_FET" | "Transistor_IGBT" => "Q",
            "Diode" | "LED" => "D",
            "Crystal" => "Y",
            "Switch" => "S",
            "Relay" => "K",
            "Transformer" => "T",
            "Battery" => "BT",
            "MCU" | "MCU_ST" | "MCU_Microchip" => "U",
            "Memory" | "Memory_EEPROM" | "Memory_Flash" => "U",
            _ => match short {
                "R" => "R",
                "C" => "C",
                "L" => "L",
                "D" | "D_Schottky" | "D_Zener" => "D",
                "BC547" | "Q_NPN" | "Q_PNP" => "Q",
                "Conn_01x02" | "Conn_01x04" => "J",
                _ => "U",
            },
        }
    }

    /// Find a library symbol whose full name contains `mpn` (case-insensitive).
    pub fn lookup_by_mpn(&self, mpn: &str) -> Option<String> {
        let q = mpn.trim().to_ascii_lowercase();
        if q.len() < 2 {
            return None;
        }
        let mut hits: Vec<String> = self
            .by_full_name
            .keys()
            .filter(|k| k.to_ascii_lowercase().contains(&q))
            .cloned()
            .collect();
        hits.sort_by_key(|k| {
            let prefer_device = k.starts_with("Device:") as u8;
            (prefer_device, k.len())
        });
        hits.into_iter().next()
    }

    /// Best bundled symbol for a refdes prefix (`R` → `Device:R`, etc.).
    pub fn default_for_prefix(&self, prefix: &str) -> Option<(String, Vec<(String, f32, f32)>)> {
        if prefix == "U" {
            let mut ic_keys: Vec<String> = self
                .by_full_name
                .keys()
                .filter(|k| {
                    k.starts_with("Amplifier_")
                        || k.starts_with("MCU_")
                        || k.starts_with("Regulator_")
                        || *k == "Device:U"
                })
                .cloned()
                .collect();
            ic_keys.sort();
            if let Some(key) = ic_keys.first() {
                let pins = self.pin_layout_for(key);
                return Some((key.clone(), pins));
            }
        }
        let candidates: &[&str] = match prefix {
            "R" => &["Device:R", "R"],
            "C" => &["Device:C", "C"],
            "L" => &["Device:L", "L"],
            "D" => &["Device:D", "D", "Device:D_Schottky"],
            "Q" => &["Transistor_BJT:BC547", "BC547", "Device:Q_NPN"],
            "J" => &["Connector:Conn_01x02", "Conn_01x02", "Connector:Conn_01x04"],
            "U" => &["Amplifier_Operational:LM358", "LM358", "Device:U", "U"],
            _ => &["Device:U", "U", "Conn_01x04"],
        };
        for key in candidates {
            if self.by_full_name.contains_key(*key) {
                let pins = self.pin_layout_for(key);
                return Some(((*key).to_string(), pins));
            }
            if let Some(full) = self.by_short_name.get(*key) {
                let pins = self.pin_layout_for(full);
                return Some((full.clone(), pins));
            }
        }
        None
    }

    /// Resolve symbol + pins for placement (library id, MPN, or refdes prefix).
    pub fn resolve_for_placement(
        &self,
        library_id: Option<&str>,
        mpn: Option<&str>,
        refdes_prefix: &str,
    ) -> (Option<String>, Vec<(String, f32, f32)>) {
        if let Some(id) = library_id.filter(|s| !s.is_empty()) {
            if self.contains(id) {
                return (Some(id.to_string()), self.pin_layout_for(id));
            }
        }
        if let Some(mpn) = mpn.filter(|s| !s.is_empty()) {
            if let Some(id) = self.lookup_by_mpn(mpn) {
                return (Some(id.clone()), self.pin_layout_for(&id));
            }
        }
        if let Some((id, pins)) = self.default_for_prefix(refdes_prefix) {
            return (Some(id), pins);
        }
        (None, vec![])
    }

    fn load_directory(&mut self, dir: &Path) -> anyhow::Result<()> {
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if !SymbolLibFile::is_library_path(path) {
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

    /// Re-run extends resolution after importing user symbols.
    pub fn finish_loading(&mut self) {
        self.resolve_extends();
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

    /// Paint on schematic canvas using library mm geometry (pins and body use the same scale).
    pub fn paint_named_on_canvas(
        &self,
        ctx: CanvasSymbolPaint<'_>,
        full_name: &str,
        fallback_kind: CompKind,
        pin_layout: &[(String, f32, f32)],
    ) {
        if let Some(sym) = self.by_full_name.get(full_name) {
            if paint_symbol_on_canvas(&ctx, sym) {
                return;
            }
        }
        paint_fallback_on_canvas(&ctx, fallback_kind);
        paint_pin_stubs_from_layout(&ctx, pin_layout);
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

/// Heuristic pin layout when library file has no pin records (e.g. unresolved `extends`).
fn default_pin_layout_for_prefix(prefix: &str) -> Vec<(String, f32, f32)> {
    let w = sch_world_per_mm();
    let pitch = pin_pitch_world();
    match prefix {
        "R" | "C" | "L" | "D" => vec![("1".into(), 0.0, -pitch), ("2".into(), 0.0, pitch)],
        "Q" => vec![
            ("B".into(), -5.08 * w, 0.0),
            ("E".into(), 0.0, -5.08 * w),
            ("C".into(), 5.08 * w, 0.0),
        ],
        "J" => vec![("1".into(), -2.54 * w, 0.0), ("2".into(), 2.54 * w, 0.0)],
        _ => dip8_pin_layout(),
    }
}

/// Typical 8-pin dual-inline pin positions in world units (mm × scale).
fn dip8_pin_layout() -> Vec<(String, f32, f32)> {
    let w = sch_world_per_mm();
    let xs = [-5.08_f32, 5.08];
    let ys = [1.905_f32, 0.635, -0.635, -1.905];
    let mut out = Vec::with_capacity(8);
    let mut n = 1;
    for &x in &xs {
        for &y in &ys {
            out.push((n.to_string(), x * w, y * w));
            n += 1;
        }
    }
    out
}

/// Library pins are stored in mm (Y flipped once at parse). Scale to world without reshaping.
fn library_pins_to_world(pins: &[SymbolPin]) -> Vec<(String, f32, f32)> {
    let wpm = sch_world_per_mm();
    pins.iter()
        .map(|p| (p.name.clone(), p.x * wpm, p.y * wpm))
        .collect()
}

fn rotate_offset(v: egui::Vec2, rot_deg: f32) -> egui::Vec2 {
    let turns = ((rot_deg / 90.0).round() as i32).rem_euclid(4);
    match turns {
        1 => egui::Vec2::new(-v.y, v.x),
        2 => egui::Vec2::new(-v.x, -v.y),
        3 => egui::Vec2::new(v.y, -v.x),
        _ => v,
    }
}

fn world_to_screen(ctx: &CanvasSymbolPaint<'_>, offset: egui::Vec2) -> Pos2 {
    let world = ctx.sym_pos + rotate_offset(offset, ctx.rot_deg);
    ctx.viewport.world_to_screen(ctx.origin, world)
}

/// Zoom level below which fields collapse to one label (avoids overlap when zoomed out).
const FIELD_LOD_ZOOM: f32 = 0.58;

/// Draw Reference and Value at library field positions.
pub fn paint_symbol_fields(
    ctx: &CanvasSymbolPaint<'_>,
    lib_sym: Option<&Symbol>,
    ref_des: &str,
    value: &str,
    field_ink: Color32,
    bounds_half: (f32, f32),
) {
    let wpm = sch_world_per_mm();
    let z = ctx.viewport.zoom;

    if z < FIELD_LOD_ZOOM {
        paint_symbol_fields_lod(ctx, ref_des, value, field_ink, bounds_half, wpm, z);
        return;
    }

    let draw_field = |x_mm: f32, y_mm: f32, font_h_mm: f32, text: &str, align: egui::Align2| {
        if text.is_empty() {
            return;
        }
        let local = rotate_offset(egui::Vec2::new(x_mm * wpm, -y_mm * wpm), ctx.rot_deg);
        let pos = world_to_screen(ctx, local);
        let font_px = schematic_field_font_px(font_h_mm, wpm, z);
        ctx.painter.text(
            pos,
            align,
            text,
            egui::FontId::monospace(font_px),
            field_ink,
        );
    };

    if let Some(sym) = lib_sym {
        let mut ref_screen = None;
        let mut val_screen = None;
        for prop in &sym.properties {
            if prop.hide {
                continue;
            }
            match prop.name.as_str() {
                "Reference" | "Value" => {}
                _ => continue,
            }
            let local = rotate_offset(
                egui::Vec2::new(prop.x_mm * wpm, -prop.y_mm * wpm),
                ctx.rot_deg,
            );
            let screen = world_to_screen(ctx, local);
            if prop.name == "Reference" {
                ref_screen = Some(screen);
            } else {
                val_screen = Some(screen);
            }
        }
        let crowded = match (ref_screen, val_screen) {
            (Some(a), Some(b)) => {
                a.distance(b) < schematic_field_font_px(SCH_FIELD_FONT_MM, wpm, z) * 2.2
            }
            _ => false,
        };
        if crowded {
            paint_symbol_fields_lod(ctx, ref_des, value, field_ink, bounds_half, wpm, z);
            return;
        }
        for prop in &sym.properties {
            if prop.hide {
                continue;
            }
            let text = match prop.name.as_str() {
                "Reference" => ref_des,
                "Value" => value,
                _ => continue,
            };
            let align = field_align_for_property(prop, ctx.rot_deg);
            draw_field(prop.x_mm, prop.y_mm, prop.font_h_mm, text, align);
        }
        return;
    }

    let pitch = pin_pitch_world();
    draw_field(
        0.0,
        pitch / wpm + 2.54,
        SCH_FIELD_FONT_MM,
        ref_des,
        egui::Align2::CENTER_BOTTOM,
    );
    draw_field(
        0.0,
        0.0,
        SCH_FIELD_FONT_MM,
        value,
        egui::Align2::CENTER_CENTER,
    );
}

fn schematic_field_font_px(font_h_mm: f32, wpm: f32, zoom: f32) -> f32 {
    (font_h_mm.max(0.9) * wpm * zoom).clamp(4.5, 22.0)
}

fn paint_symbol_fields_lod(
    ctx: &CanvasSymbolPaint<'_>,
    ref_des: &str,
    value: &str,
    field_ink: Color32,
    bounds_half: (f32, f32),
    wpm: f32,
    z: f32,
) {
    let above = rotate_offset(
        egui::Vec2::new(0.0, -(bounds_half.1 + 1.27 * wpm)),
        ctx.rot_deg,
    );
    let pos = world_to_screen(ctx, above);
    let font_px = schematic_field_font_px(SCH_FIELD_FONT_MM, wpm, z);
    let line = if value.is_empty() || value == ref_des {
        ref_des.to_string()
    } else {
        format!("{ref_des} · {value}")
    };
    ctx.painter.text(
        pos,
        egui::Align2::CENTER_BOTTOM,
        line,
        egui::FontId::monospace(font_px),
        field_ink,
    );
}

fn field_align_for_property(
    prop: &crate::symbol_format::SymbolProperty,
    sym_rot_deg: f32,
) -> egui::Align2 {
    match prop.name.as_str() {
        "Value" => egui::Align2::CENTER_CENTER,
        "Reference" => {
            let local = rotate_offset(egui::Vec2::new(prop.x_mm, -prop.y_mm), sym_rot_deg);
            if local.x.abs() >= local.y.abs() {
                if local.x > 0.05 {
                    egui::Align2::LEFT_CENTER
                } else if local.x < -0.05 {
                    egui::Align2::RIGHT_CENTER
                } else if local.y > 0.05 {
                    egui::Align2::CENTER_BOTTOM
                } else {
                    egui::Align2::CENTER_TOP
                }
            } else if local.y > 0.05 {
                egui::Align2::CENTER_BOTTOM
            } else {
                egui::Align2::CENTER_TOP
            }
        }
        _ => field_align(prop.rot_deg + sym_rot_deg),
    }
}

fn field_align(combined_rot_deg: f32) -> egui::Align2 {
    let r = ((combined_rot_deg / 90.0).round() as i32).rem_euclid(4);
    match r {
        1 => egui::Align2::LEFT_CENTER,
        2 => egui::Align2::CENTER_TOP,
        3 => egui::Align2::RIGHT_CENTER,
        _ => egui::Align2::CENTER_BOTTOM,
    }
}

fn paint_symbol_on_canvas(ctx: &CanvasSymbolPaint<'_>, sym: &Symbol) -> bool {
    if sym.graphics.is_empty() {
        return false;
    }
    let wpm = sch_world_per_mm();
    let outline = Stroke::new((ctx.stroke_px + 1.1).clamp(1.8, 4.5), ctx.outline);
    let stroke = Stroke::new(ctx.stroke_px.clamp(1.0, 3.5), ctx.ink);

    let xf = |x_mm: f32, y_mm: f32| -> Pos2 {
        world_to_screen(ctx, egui::Vec2::new(x_mm * wpm, -y_mm * wpm))
    };

    paint_pin_stubs_on_canvas(ctx, sym);

    for g in &sym.graphics {
        match g.kind.as_str() {
            "line" => {
                if let (Some(a), Some(b)) = (g.start, g.end) {
                    let seg = [xf(a[0] as f32, a[1] as f32), xf(b[0] as f32, b[1] as f32)];
                    ctx.painter.line_segment(seg, outline);
                    ctx.painter.line_segment(seg, stroke);
                }
            }
            "rectangle" => {
                if let (Some(a), Some(b)) = (g.start, g.end) {
                    let tl = xf(a[0].min(b[0]) as f32, a[1].max(b[1]) as f32);
                    let br = xf(a[0].max(b[0]) as f32, a[1].min(b[1]) as f32);
                    let rr = Rect::from_two_pos(tl, br);
                    ctx.painter.rect_stroke(rr, 0.0, outline);
                    ctx.painter.rect_stroke(rr, 0.0, stroke);
                }
            }
            "circle" => {
                if let (Some(c), Some(r)) = (g.center, g.radius) {
                    let pc = xf(c[0] as f32, c[1] as f32);
                    let rad = (r as f32).abs() * wpm * ctx.viewport.zoom;
                    ctx.painter.circle_stroke(pc, rad, outline);
                    ctx.painter.circle_stroke(pc, rad, stroke);
                }
            }
            _ => {}
        }
    }
    true
}

fn paint_pin_stubs_on_canvas(ctx: &CanvasSymbolPaint<'_>, sym: &Symbol) {
    if sym.pins.is_empty() {
        return;
    }
    let wpm = sch_world_per_mm();
    let stroke = Stroke::new(ctx.stroke_px.clamp(1.0, 3.0), ctx.ink);
    for pin in &sym.pins {
        let a = world_to_screen(ctx, egui::Vec2::new(pin.body_x * wpm, pin.body_y * wpm));
        let b = world_to_screen(ctx, egui::Vec2::new(pin.x * wpm, pin.y * wpm));
        ctx.painter.line_segment([a, b], stroke);
    }
}

pub(crate) fn paint_pin_stubs_from_layout(
    ctx: &CanvasSymbolPaint<'_>,
    pin_layout: &[(String, f32, f32)],
) {
    let stroke = Stroke::new(ctx.stroke_px.clamp(1.0, 3.0), ctx.ink);
    for (name, cx, cy) in pin_layout {
        let conn = egui::Vec2::new(*cx, *cy);
        if conn.length_sq() < 1e-3 {
            continue;
        }
        let dir = conn.normalized();
        let stub = (conn.length() - 1.27 * sch_world_per_mm()).max(conn.length() * 0.4);
        let body = conn - dir * stub;
        let a = world_to_screen(ctx, body);
        let b = world_to_screen(ctx, conn);
        let _ = name;
        ctx.painter.line_segment([a, b], stroke);
    }
}

pub(crate) fn paint_fallback_on_canvas(ctx: &CanvasSymbolPaint<'_>, kind: CompKind) {
    let wpm = sch_world_per_mm();
    let z = ctx.viewport.zoom;
    let pivot = ctx.viewport.world_to_screen(ctx.origin, ctx.sym_pos);
    let lw = 2.54 * wpm * z;
    let lh = 2.0 * wpm * z;
    symbols_draw::paint_symbol_body(
        ctx.painter,
        pivot,
        lw,
        lh,
        ctx.rot_deg,
        kind,
        ctx.ink,
        ctx.stroke_px,
    );
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

    #[test]
    fn refdes_prefix_for_opamp_is_u_not_l() {
        assert_eq!(
            BaseSymbolLibrary::refdes_prefix_for_library_id("Amplifier_Operational:LM358"),
            "U"
        );
        assert_eq!(
            BaseSymbolLibrary::refdes_prefix_for_library_id("Device:R"),
            "R"
        );
        assert_eq!(
            BaseSymbolLibrary::refdes_prefix_for_library_id("Connector:Conn_01x02"),
            "J"
        );
    }

    #[test]
    fn default_for_prefix_u_prefers_ic_family() {
        let lib = BaseSymbolLibrary::open().expect("bundled symbols");
        let (name, _) = lib.default_for_prefix("U").expect("U default");
        assert!(
            name.starts_with("Amplifier_")
                || name.starts_with("MCU_")
                || name.starts_with("Regulator_")
                || name == "Device:U",
            "unexpected U default: {name}"
        );
    }

    #[test]
    fn mcp6002_fallback_template_has_art() {
        let lib = BaseSymbolLibrary::open().expect("bundled symbols");
        let sym = lib.symbol("Interface:MCP6002").expect("Interface:MCP6002");
        assert!(
            !sym.graphics.is_empty() || !sym.pins.is_empty(),
            "MCP6002 template should parse graphics or pins"
        );
    }

    #[test]
    fn device_r_pins_at_body_edge() {
        let lib = BaseSymbolLibrary::open().expect("bundled symbols");
        let pins = lib.pin_layout_for("Device:R");
        assert_eq!(pins.len(), 2, "expected two pins");
        let pitch = crate::canvas::pin_pitch_world();
        let p1 = pins.iter().find(|(n, _, _)| n == "1").expect("pin 1");
        let p2 = pins.iter().find(|(n, _, _)| n == "2").expect("pin 2");
        let dy = (p2.2 - p1.2).abs();
        assert!(
            (pitch * 1.9..=pitch * 2.1).contains(&dy),
            "vertical R pins should be ~2× pitch apart, got {dy} vs {pitch}"
        );
    }

    #[test]
    fn lm358_resolves_graphics_or_pins() {
        let lib = BaseSymbolLibrary::open().expect("bundled symbols");
        let key = "Amplifier_Operational:LM358";
        assert!(lib.contains(key), "missing {key}");
        let pins = lib.pin_layout_for(key);
        assert!(
            !pins.is_empty(),
            "LM358 should have pins after extends/fallback resolution"
        );
        let sym = lib.symbol(key).expect("symbol");
        assert!(
            !sym.graphics.is_empty() || !sym.pins.is_empty(),
            "LM358 should have library graphics or pin records"
        );
    }
}
