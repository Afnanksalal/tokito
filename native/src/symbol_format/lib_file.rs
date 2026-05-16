//! Tokito symbol library file (`.tokito_sym`).

use std::path::Path;

use super::sexpr::{list_head, ParseError, Sexpr};

#[derive(Debug, Clone)]
pub struct SymbolLibFile {
    symbols: Vec<Symbol>,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    /// Parent symbol short name from `(extends "…")` in the library file.
    pub extends: Option<String>,
    pub graphics: Vec<SymbolGraphic>,
    /// Pin name + local position (library units, Y up).
    pub pins: Vec<SymbolPin>,
    /// Schematic fields (`Reference`, `Value`, …) with default text and placement.
    pub properties: Vec<SymbolProperty>,
}

/// Symbol field (Reference, Value, Footprint, …).
#[derive(Debug, Clone)]
pub struct SymbolProperty {
    pub name: String,
    pub default_text: String,
    pub x_mm: f32,
    pub y_mm: f32,
    pub rot_deg: f32,
    pub hide: bool,
    pub font_h_mm: f32,
    pub font_w_mm: f32,
}

#[derive(Debug, Clone)]
pub struct SymbolPin {
    pub name: String,
    /// Wire connection point (schematic coords, Y-up).
    pub x: f32,
    pub y: f32,
    /// Inner end of pin stub at the symbol body (schematic coords).
    pub body_x: f32,
    pub body_y: f32,
}

#[derive(Debug, Clone)]
pub struct SymbolGraphic {
    pub kind: String,
    pub start: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub center: Option<[f64; 2]>,
    pub radius: Option<f64>,
}

impl SymbolLibFile {
    pub const EXTENSION: &'static str = "tokito_sym";
    /// External symbol library file extension (import converts to `.tokito_sym`).
    pub const IMPORT_SYM_EXT: &str = "kicad_sym";

    pub fn is_library_path(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == Self::EXTENSION || e == Self::IMPORT_SYM_EXT)
    }

    pub fn read(path: &Path) -> Result<Self, ParseError> {
        let text = std::fs::read_to_string(path).map_err(|e| ParseError {
            message: format!("read {}: {e}", path.display()),
        })?;
        Self::parse(&text)
    }

    /// Rewrite an imported library root tag to Tokito format for storage.
    pub fn normalize_to_tokito_format(text: &str) -> String {
        let trimmed = text.trim_start();
        if trimmed.starts_with("(kicad_symbol_lib") {
            trimmed.replacen("(kicad_symbol_lib", "(tokito_symbol_lib", 1)
        } else {
            text.to_string()
        }
    }

    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let root = super::sexpr::parse(text)?;
        let mut symbols = Vec::new();

        let Sexpr::List(lib_items) = &root else {
            return Err(ParseError {
                message: "expected symbol library root list".into(),
            });
        };

        if let Some((head, _)) = lib_items.first().and_then(|i| list_head(i)) {
            if head != "tokito_symbol_lib" && head != "kicad_symbol_lib" {
                return Err(ParseError {
                    message: format!("unsupported library root '{head}'"),
                });
            }
        }

        for item in lib_items {
            let Some((head, tail)) = list_head(item) else {
                continue;
            };
            if head != "symbol" {
                continue;
            }
            let Some(name) = tail.first().and_then(|s| s.as_atom()) else {
                continue;
            };
            let extends = extract_extends(item);
            let graphics = collect_graphics(item);
            let pins = collect_pins(item);
            let properties = collect_properties(item);
            symbols.push(Symbol {
                name: name.to_string(),
                extends,
                graphics,
                pins,
                properties,
            });
        }

        if symbols.is_empty() {
            return Err(ParseError {
                message: "no symbols found in library file".into(),
            });
        }

        Ok(Self { symbols })
    }

    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }
}

fn extract_extends(symbol_node: &Sexpr) -> Option<String> {
    let Sexpr::List(items) = symbol_node else {
        return None;
    };
    for child in items {
        if let Some((head, tail)) = list_head(child) {
            if head == "extends" {
                return tail
                    .first()
                    .and_then(|s| s.as_atom())
                    .map(|s| s.to_string());
            }
        }
    }
    None
}

fn collect_pins(node: &Sexpr) -> Vec<SymbolPin> {
    let mut out = Vec::new();
    walk_pins(node, &mut out);
    out
}

fn walk_pins(node: &Sexpr, out: &mut Vec<SymbolPin>) {
    if let Some((head, _tail)) = list_head(node) {
        if head == "pin" {
            let mut at: Option<[f64; 2]> = None;
            let mut angle_deg: f64 = 0.0;
            let mut length: f32 = 1.27;
            let mut number = String::new();
            if let Sexpr::List(items) = node {
                for child in items {
                    if let Some((h, t)) = list_head(child) {
                        if h == "at" {
                            at = point_from_sexpr_tail(t);
                            if let Some(a) = t.get(2).and_then(|s| s.as_atom()) {
                                angle_deg = a.parse().unwrap_or(0.0);
                            }
                        }
                        if h == "number" {
                            if let Some(n) = t.first().and_then(|s| s.as_atom()) {
                                number = n.to_string();
                            }
                        }
                        if h == "length" {
                            if let Some(n) = t.first().and_then(|s| s.as_atom()) {
                                length = n.parse().unwrap_or(1.27) as f32;
                            }
                        }
                    } else if let Some(p) = child.as_point() {
                        at = Some(p);
                    }
                }
            }
            if let Some([x, y]) = at {
                let name = if number.is_empty() {
                    format!("{}", out.len() + 1)
                } else {
                    number
                };
                let (bx, by, cx, cy) = pin_connection_and_body(x, y, angle_deg, f64::from(length));
                out.push(SymbolPin {
                    name,
                    x: cx as f32,
                    y: cy as f32,
                    body_x: bx as f32,
                    body_y: by as f32,
                });
            }
        }
    }
    if let Sexpr::List(items) = node {
        for child in items {
            walk_pins(child, out);
        }
    }
}

/// Pin connection at `(x,y)`; stub toward body along `angle` by `length`.
fn pin_connection_and_body(x: f64, y: f64, angle_deg: f64, length: f64) -> (f64, f64, f64, f64) {
    let rad = angle_deg.to_radians();
    let bx = x + length * rad.cos();
    let by = y + length * rad.sin();
    // Schematic Y-up; file coordinates are Y-down.
    (bx, -by, x, -y)
}

fn point_from_sexpr_tail(tail: &[Sexpr]) -> Option<[f64; 2]> {
    let x: f64 = tail
        .first()
        .and_then(|s| s.as_atom())
        .and_then(|s| s.parse().ok())?;
    let y: f64 = tail
        .get(1)
        .and_then(|s| s.as_atom())
        .and_then(|s| s.parse().ok())?;
    Some([x, y])
}

fn collect_properties(node: &Sexpr) -> Vec<SymbolProperty> {
    let mut out = Vec::new();
    walk_properties(node, &mut out);
    out
}

fn walk_properties(node: &Sexpr, out: &mut Vec<SymbolProperty>) {
    if let Some((head, tail)) = list_head(node) {
        if head == "property" {
            if let Some(prop) = property_from_list(node, tail) {
                out.push(prop);
            }
        }
    }
    if let Sexpr::List(items) = node {
        for child in items {
            walk_properties(child, out);
        }
    }
}

fn property_from_list(node: &Sexpr, tail: &[Sexpr]) -> Option<SymbolProperty> {
    let name = tail.first().and_then(|s| s.as_atom())?.to_string();
    let default_text = tail.get(1).and_then(|s| s.as_atom())?.to_string();
    let mut x_mm = 0.0_f32;
    let mut y_mm = 0.0_f32;
    let mut rot_deg = 0.0_f32;
    let mut hide = false;
    let mut font_h_mm = 1.27_f32;
    let mut font_w_mm = 1.27_f32;
    if let Sexpr::List(items) = node {
        for child in items {
            if let Some((h, t)) = list_head(child) {
                match h {
                    "at" => {
                        if let Some([x, y]) = point_from_sexpr_tail(t) {
                            x_mm = x as f32;
                            y_mm = y as f32;
                            rot_deg = t
                                .get(2)
                                .and_then(|s| s.as_atom())
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(0.0) as f32;
                        }
                    }
                    "hide" => {
                        hide = t.first().and_then(|s| s.as_atom()) == Some("yes");
                    }
                    "effects" => {
                        if let Some((fh, fw)) = font_size_from_effects(child) {
                            font_h_mm = fh;
                            font_w_mm = fw;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Some(SymbolProperty {
        name,
        default_text,
        x_mm,
        y_mm,
        rot_deg,
        hide,
        font_h_mm,
        font_w_mm,
    })
}

fn font_size_from_effects(effects: &Sexpr) -> Option<(f32, f32)> {
    let Sexpr::List(items) = effects else {
        return None;
    };
    for child in items {
        if let Some((h, t)) = list_head(child) {
            if h == "font" {
                let h_mm = t
                    .first()
                    .and_then(|s| s.as_atom())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1.27) as f32;
                let w_mm = t
                    .get(1)
                    .and_then(|s| s.as_atom())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(h_mm as f64) as f32;
                return Some((h_mm, w_mm));
            }
        }
    }
    None
}

fn collect_graphics(node: &Sexpr) -> Vec<SymbolGraphic> {
    let mut out = Vec::new();
    walk_graphics(node, &mut out);
    out
}

fn walk_graphics(node: &Sexpr, out: &mut Vec<SymbolGraphic>) {
    if let Some((head, _)) = list_head(node) {
        match head {
            "line" | "rectangle" | "circle" | "arc" => {
                if let Some(g) = graphic_from_list(node, head) {
                    out.push(g);
                }
            }
            "polyline" => out.extend(polyline_segments(node)),
            _ => {}
        }
    }
    if let Sexpr::List(items) = node {
        for child in items {
            walk_graphics(child, out);
        }
    }
}

fn polyline_segments(node: &Sexpr) -> Vec<SymbolGraphic> {
    let Some(pts) = node.child_list("pts") else {
        return Vec::new();
    };
    let Sexpr::List(items) = pts else {
        return Vec::new();
    };
    let mut points: Vec<[f64; 2]> = Vec::new();
    for child in items {
        if let Some((h, tail)) = list_head(child) {
            if h == "xy" {
                let Some(x) = tail
                    .first()
                    .and_then(|s| s.as_atom())
                    .and_then(|s| s.parse().ok())
                else {
                    continue;
                };
                let Some(y) = tail
                    .get(1)
                    .and_then(|s| s.as_atom())
                    .and_then(|s| s.parse().ok())
                else {
                    continue;
                };
                points.push([x, y]);
            }
        }
    }
    let mut out = Vec::new();
    for w in points.windows(2) {
        out.push(SymbolGraphic {
            kind: "line".into(),
            start: Some(w[0]),
            end: Some(w[1]),
            center: None,
            radius: None,
        });
    }
    out
}

fn graphic_from_list(node: &Sexpr, kind: &str) -> Option<SymbolGraphic> {
    let mut start = None;
    let mut end = None;
    let mut center = None;
    let mut radius = None;

    if let Sexpr::List(items) = node {
        for child in items {
            if let Some((h, _)) = list_head(child) {
                match h {
                    "start" => start = child.as_point(),
                    "end" => end = child.as_point(),
                    "center" => center = child.as_point(),
                    "radius" => radius = child.as_f64(),
                    _ => {}
                }
            }
        }
    }

    Some(SymbolGraphic {
        kind: kind.to_string(),
        start,
        end,
        center,
        radius,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mcp6002_pins_and_graphics() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../assets/base-symbols/Interface/MCP6002.tokito_sym");
        if !path.exists() {
            return;
        }
        let lib = SymbolLibFile::read(&path).expect("parse MCP6002");
        let sym = lib
            .symbols()
            .iter()
            .find(|s| s.name == "MCP6002")
            .expect("sym");
        assert!(
            !sym.graphics.is_empty() || !sym.pins.is_empty(),
            "graphics={} pins={}",
            sym.graphics.len(),
            sym.pins.len()
        );
    }

    #[test]
    fn parses_import_symbol_lib_root() {
        let text = r#"(kicad_symbol_lib
	(version 20220914)
	(symbol "Device:R"
		(pin passive line
			(at 0 3.81 270)
			(length 1.27)
			(name "2" (effects (font (size 1.27 1.27))))
			(number "2" (effects (font (size 1.27 1.27))))
		)
		(pin passive line
			(at 0 -3.81 90)
			(length 1.27)
			(name "1" (effects (font (size 1.27 1.27))))
			(number "1" (effects (font (size 1.27 1.27))))
		)
		(property "Reference" "R"
			(at 2.032 0 90)
			(effects (font (size 1.27 1.27)))
		)
	)
)"#;
        let lib = SymbolLibFile::parse(text).expect("import root");
        assert_eq!(lib.symbols().len(), 1);
        assert_eq!(lib.symbols()[0].pins.len(), 2);
    }

    #[test]
    fn parses_resistor_graphics() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../assets/base-symbols/Device/R.tokito_sym");
        if !path.exists() {
            return;
        }
        let lib = SymbolLibFile::read(&path).expect("parse R");
        assert!(!lib.symbols().is_empty());
        let sym = &lib.symbols()[0];
        assert!(sym.graphics.iter().any(|g| g.kind == "rectangle"));
        let value = sym
            .properties
            .iter()
            .find(|p| p.name == "Value")
            .expect("Value property");
        assert_eq!(value.default_text, "R");
        assert!((value.y_mm - 0.0).abs() < 0.01);
    }
}
