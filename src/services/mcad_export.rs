//! MCAD handoff JSON from a schematic document.

use crate::models::SchematicDocument;
use serde_json::json;

pub fn document_handoff_json(document: &SchematicDocument, design_name: &str) -> String {
    let placements: Vec<serde_json::Value> = document
        .symbols
        .iter()
        .filter_map(|s| {
            let fp = s.footprint_ref.as_deref().filter(|f| !f.is_empty())?;
            Some(json!({
                "ref_des": s.ref_des,
                "footprint": fp,
                "part_id": s.part_id,
                "symbol_id": s.symbol_id,
                "x_mm": s.position.x,
                "y_mm": s.position.y,
                "rotation_deg": s.rotation,
                "sheet_id": s.sheet_id,
            }))
        })
        .collect();
    let payload = json!({
        "format": "tokito_mcad_handoff_v1",
        "design": design_name,
        "sheet_count": document.sheets.len(),
        "placements": placements,
    });
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
}
