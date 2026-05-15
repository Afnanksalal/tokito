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
    pub graphics: Vec<SymbolGraphic>,
    /// Pin name + local position (library units, Y up).
    pub pins: Vec<SymbolPin>,
}

#[derive(Debug, Clone)]
pub struct SymbolPin {
    pub name: String,
    pub x: f32,
    pub y: f32,
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

    pub fn read(path: &Path) -> Result<Self, ParseError> {
        let text = std::fs::read_to_string(path).map_err(|e| ParseError {
            message: format!("read {}: {e}", path.display()),
        })?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let root = super::sexpr::parse(text)?;
        let mut symbols = Vec::new();

        let Sexpr::List(lib_items) = &root else {
            return Err(ParseError {
                message: "expected symbol library root list".into(),
            });
        };

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
            let graphics = collect_graphics(item);
            let pins = collect_pins(item);
            if graphics.is_empty() && pins.is_empty() {
                continue;
            }
            symbols.push(Symbol {
                name: name.to_string(),
                graphics,
                pins,
            });
        }

        if symbols.is_empty() {
            return Err(ParseError {
                message: "no symbols with graphics found".into(),
            });
        }

        Ok(Self { symbols })
    }

    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }
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
            let mut number = String::new();
            if let Sexpr::List(items) = node {
                for child in items {
                    if let Some((h, t)) = list_head(child) {
                        if h == "at" {
                            let x: f64 = t
                                .first()
                                .and_then(|s| s.as_atom())
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(0.0);
                            let y: f64 = t
                                .get(1)
                                .and_then(|s| s.as_atom())
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(0.0);
                            at = Some([x, y]);
                        }
                        if h == "number" {
                            if let Some(n) = t.first().and_then(|s| s.as_atom()) {
                                number = n.to_string();
                            }
                        }
                    }
                }
            }
            if let Some([x, y]) = at {
                let name = if number.is_empty() {
                    format!("{}", out.len() + 1)
                } else {
                    number
                };
                out.push(SymbolPin {
                    name,
                    x: x as f32,
                    y: -(y as f32),
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
    fn parses_resistor_graphics() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../assets/base-symbols/Device/R.tokito_sym");
        if !path.exists() {
            return;
        }
        let lib = SymbolLibFile::read(&path).expect("parse R");
        assert!(!lib.symbols().is_empty());
        assert!(lib.symbols()[0]
            .graphics
            .iter()
            .any(|g| g.kind == "rectangle"));
    }
}
