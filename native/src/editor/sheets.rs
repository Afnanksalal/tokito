//! Multi-sheet document merge: flush active sheet into `SchematicDocument`, hydrate editor.

use egui::Pos2;
use uuid::Uuid;

use crate::canvas::{
    snap_world_pos, BusSegment, Junction, NetLabel, NoConnect, PowerSymbol, Sym, TextItem,
    WireSegment,
};
use crate::editor::{ErcMarkerOnCanvas, SchematicEditor, SheetInfo};
use tokito::models::{
    DocumentBusSegment, DocumentErcMarker, DocumentJunction, DocumentNetLabel, DocumentNoConnect,
    DocumentPin, DocumentPoint, DocumentPowerSymbol, DocumentSymbol, DocumentTextItem,
    DocumentWireSegment, ElectricalPinType, MirrorMode, PinOrientation, SchematicDocument,
    SchematicSheet,
};

/// Write the active sheet from `editor` into `doc`, replacing prior geometry for that sheet id.
pub fn flush_active_sheet(
    editor: &SchematicEditor,
    doc: &mut SchematicDocument,
    part_cache: &std::collections::HashMap<Uuid, String>,
) {
    let sheet_id = editor.active_sheet_id.clone();

    doc.sheets = editor
        .sheets
        .iter()
        .map(|s| SchematicSheet {
            id: s.id.clone(),
            name: s.name.clone(),
            path: s.id.clone(),
            page_size: tokito::models::PageSize {
                width: 2970.0,
                height: 2100.0,
            },
            grid: 40.0,
            title_block: Default::default(),
        })
        .collect();

    doc.symbols.retain(|s| s.sheet_id != sheet_id);
    doc.wire_segments.retain(|w| w.sheet_id != sheet_id);
    doc.net_labels.retain(|l| l.sheet_id != sheet_id);
    doc.junctions.retain(|j| j.sheet_id != sheet_id);
    doc.no_connects.retain(|n| n.sheet_id != sheet_id);
    doc.power_symbols.retain(|p| p.sheet_id != sheet_id);
    doc.text_items.retain(|t| t.sheet_id != sheet_id);
    doc.buses.retain(|b| b.sheet_id != sheet_id);
    doc.erc_markers.retain(|m| m.sheet_id != sheet_id);

    doc.symbols.extend(
        editor
            .symbols
            .iter()
            .map(|s| symbol_to_document(s, &sheet_id, part_cache)),
    );
    doc.wire_segments
        .extend(editor.wire_segments.iter().map(|seg| DocumentWireSegment {
            id: seg.id,
            sheet_id: sheet_id.clone(),
            start: DocumentPoint {
                x: seg.start.x as f64,
                y: seg.start.y as f64,
            },
            end: DocumentPoint {
                x: seg.end.x as f64,
                y: seg.end.y as f64,
            },
            net_name: Some(seg.net.clone()),
        }));
    doc.net_labels
        .extend(editor.net_labels.iter().map(|l| DocumentNetLabel {
            id: Uuid::new_v4(),
            sheet_id: sheet_id.clone(),
            name: l.name.clone(),
            kind: l.kind,
            position: DocumentPoint {
                x: l.pos.x as f64,
                y: l.pos.y as f64,
            },
            orientation: PinOrientation::Right,
        }));
    doc.junctions
        .extend(editor.junctions.iter().map(|j| DocumentJunction {
            id: Uuid::new_v4(),
            sheet_id: sheet_id.clone(),
            position: DocumentPoint {
                x: j.pos.x as f64,
                y: j.pos.y as f64,
            },
        }));
    doc.no_connects
        .extend(editor.no_connects.iter().map(|n| DocumentNoConnect {
            id: Uuid::new_v4(),
            sheet_id: sheet_id.clone(),
            position: DocumentPoint {
                x: n.pos.x as f64,
                y: n.pos.y as f64,
            },
        }));
    doc.power_symbols
        .extend(editor.power_symbols.iter().map(|p| DocumentPowerSymbol {
            id: Uuid::new_v4(),
            sheet_id: sheet_id.clone(),
            name: p.name.clone(),
            position: DocumentPoint {
                x: p.pos.x as f64,
                y: p.pos.y as f64,
            },
        }));
    doc.text_items
        .extend(editor.text_items.iter().map(|t| DocumentTextItem {
            id: Uuid::new_v4(),
            sheet_id: sheet_id.clone(),
            text: t.text.clone(),
            position: DocumentPoint {
                x: t.pos.x as f64,
                y: t.pos.y as f64,
            },
            rotation: 0.0,
        }));
    doc.buses
        .extend(editor.buses.iter().map(|b| DocumentBusSegment {
            id: Uuid::new_v4(),
            sheet_id: sheet_id.clone(),
            name: b.name.clone(),
            start: DocumentPoint {
                x: b.start.x as f64,
                y: b.start.y as f64,
            },
            end: DocumentPoint {
                x: b.end.x as f64,
                y: b.end.y as f64,
            },
        }));
    doc.erc_markers
        .extend(editor.erc_markers.iter().map(|m| DocumentErcMarker {
            id: Uuid::new_v4(),
            sheet_id: sheet_id.clone(),
            code: m.code.clone(),
            message: m.message.clone(),
            severity: m.severity.clone(),
            position: DocumentPoint {
                x: m.position.x as f64,
                y: m.position.y as f64,
            },
        }));
}

/// Load one sheet from `doc` into live editor geometry (clears prior editor content).
pub fn hydrate_active_sheet(editor: &mut SchematicEditor, doc: &SchematicDocument, sheet_id: &str) {
    editor.active_sheet_id = sheet_id.to_string();
    editor.sheets = doc
        .sheets
        .iter()
        .map(|s| SheetInfo {
            id: s.id.clone(),
            name: s.name.clone(),
        })
        .collect();
    if editor.sheets.is_empty() {
        editor.sheets.push(SheetInfo {
            id: sheet_id.to_string(),
            name: "Root".to_string(),
        });
    }

    editor.symbols = doc
        .symbols
        .iter()
        .filter(|s| s.sheet_id == sheet_id)
        .map(document_symbol_to_sym)
        .collect();
    editor.wire_segments = doc
        .wire_segments
        .iter()
        .filter(|w| w.sheet_id == sheet_id)
        .map(|w| WireSegment {
            id: w.id,
            start: Pos2::new(w.start.x as f32, w.start.y as f32),
            end: Pos2::new(w.end.x as f32, w.end.y as f32),
            net: w.net_name.clone().unwrap_or_else(|| "NET".to_string()),
        })
        .collect();
    editor.net_labels = doc
        .net_labels
        .iter()
        .filter(|l| l.sheet_id == sheet_id)
        .map(|l| NetLabel {
            name: l.name.clone(),
            pos: Pos2::new(l.position.x as f32, l.position.y as f32),
            kind: l.kind,
        })
        .collect();
    editor.junctions = doc
        .junctions
        .iter()
        .filter(|j| j.sheet_id == sheet_id)
        .map(|j| Junction {
            pos: Pos2::new(j.position.x as f32, j.position.y as f32),
        })
        .collect();
    editor.no_connects = doc
        .no_connects
        .iter()
        .filter(|n| n.sheet_id == sheet_id)
        .map(|n| NoConnect {
            pos: Pos2::new(n.position.x as f32, n.position.y as f32),
        })
        .collect();
    editor.power_symbols = doc
        .power_symbols
        .iter()
        .filter(|p| p.sheet_id == sheet_id)
        .map(|p| PowerSymbol {
            name: p.name.clone(),
            pos: Pos2::new(p.position.x as f32, p.position.y as f32),
        })
        .collect();
    editor.text_items = doc
        .text_items
        .iter()
        .filter(|t| t.sheet_id == sheet_id)
        .map(|t| TextItem {
            text: t.text.clone(),
            pos: Pos2::new(t.position.x as f32, t.position.y as f32),
        })
        .collect();
    editor.buses = doc
        .buses
        .iter()
        .filter(|b| b.sheet_id == sheet_id)
        .map(|b| BusSegment {
            name: b.name.clone(),
            start: Pos2::new(b.start.x as f32, b.start.y as f32),
            end: Pos2::new(b.end.x as f32, b.end.y as f32),
        })
        .collect();
    editor.erc_markers = doc
        .erc_markers
        .iter()
        .filter(|m| m.sheet_id == sheet_id)
        .map(|m| ErcMarkerOnCanvas {
            code: m.code.clone(),
            message: m.message.clone(),
            severity: m.severity.clone(),
            position: Pos2::new(m.position.x as f32, m.position.y as f32),
            instance_ref: None,
            net_name: None,
        })
        .collect();
    editor.clear_selection();
    editor.clear_history();
}

fn document_symbol_to_sym(s: &DocumentSymbol) -> Sym {
    let pins: Vec<String> = s
        .pins
        .iter()
        .map(|p| {
            p.number
                .clone()
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| p.name.clone())
        })
        .collect();
    let pin_layout = s
        .pins
        .iter()
        .map(|p| {
            let name = p
                .number
                .clone()
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| p.name.clone());
            (name, p.offset.x as f32, p.offset.y as f32)
        })
        .collect();
    Sym {
        ref_des: s.ref_des.clone(),
        part_id: s.part_id,
        pos: snap_world_pos(Pos2::new(s.position.x as f32, s.position.y as f32)),
        rotation_deg: s.rotation as f32,
        pins,
        footprint_ref: s.footprint_ref.clone(),
        symbol_id: s.symbol_id.clone(),
        pin_layout,
    }
}

fn symbol_to_document(
    s: &Sym,
    sheet_id: &str,
    part_cache: &std::collections::HashMap<Uuid, String>,
) -> DocumentSymbol {
    let pins: Vec<DocumentPin> = if s.pin_layout.is_empty() {
        s.pins
            .iter()
            .map(|pin_name| default_document_pin(pin_name))
            .collect()
    } else {
        s.pin_layout
            .iter()
            .map(|(name, x, y)| DocumentPin {
                number: Some(name.clone()),
                name: name.clone(),
                electrical_type: ElectricalPinType::Unspecified,
                offset: DocumentPoint {
                    x: *x as f64,
                    y: *y as f64,
                },
                orientation: PinOrientation::Right,
                visible: true,
            })
            .collect()
    };

    DocumentSymbol {
        id: Uuid::new_v4(),
        sheet_id: sheet_id.to_string(),
        part_id: s.part_id,
        symbol_id: s
            .symbol_id
            .clone()
            .or_else(|| Some("tokito:generic".to_string())),
        ref_des: s.ref_des.clone(),
        value: s.part_id.and_then(|id| part_cache.get(&id)).cloned(),
        position: DocumentPoint {
            x: s.pos.x as f64,
            y: s.pos.y as f64,
        },
        rotation: s.rotation_deg as f64,
        mirror: MirrorMode::None,
        fields: Default::default(),
        footprint_ref: s.footprint_ref.clone(),
        pins,
    }
}

fn default_document_pin(pin_name: &str) -> DocumentPin {
    let right_side = matches!(
        pin_name.trim().to_ascii_lowercase().as_str(),
        "2" | "b" | "out" | "vout" | "sda" | "scl" | "tx" | "miso"
    ) || pin_name.ends_with("_b");
    DocumentPin {
        number: Some(pin_name.to_string()),
        name: pin_name.to_string(),
        electrical_type: ElectricalPinType::Unspecified,
        offset: DocumentPoint {
            x: if right_side { 70.0 } else { -70.0 },
            y: 0.0,
        },
        orientation: if right_side {
            PinOrientation::Right
        } else {
            PinOrientation::Left
        },
        visible: true,
    }
}
