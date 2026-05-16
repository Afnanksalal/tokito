//! Live ERC markers on the canvas (light checks from editor connectivity).

use std::collections::HashMap;

use egui::Pos2;
use tokito::models::{
    ReplaceSchematic, SchematicInstanceInput, SchematicNetInput, SchematicPinInput,
};
use tokito::services::schematic_validate::erc_light;

use crate::canvas::{symbol_pin_world, Sym, PIN_ATTACH_RADIUS};

use super::state::{ErcMarkerOnCanvas, SchematicEditor};

/// Rebuild advisory ERC markers from the current sheet geometry and connectivity graph.
pub fn refresh_live_erc_markers(editor: &mut SchematicEditor) {
    let replace = editor_to_replace(editor);
    let violations = erc_light(&replace);
    editor.erc_markers = violations
        .iter()
        .map(|v| violation_to_canvas_marker(v, editor))
        .collect();
}

fn editor_to_replace(editor: &SchematicEditor) -> ReplaceSchematic {
    let mut net_id_to_name: HashMap<uuid::Uuid, String> = HashMap::new();
    for seg in &editor.wire_segments {
        net_id_to_name
            .entry(seg.net_id)
            .or_insert_with(|| seg.net.clone());
    }
    let mut nets: Vec<SchematicNetInput> = net_id_to_name
        .values()
        .map(|name| SchematicNetInput {
            id: None,
            name: name.clone(),
        })
        .collect();
    for label in &editor.net_labels {
        let name = label.name.trim();
        if !name.is_empty() && !nets.iter().any(|n| n.name == name) {
            nets.push(SchematicNetInput {
                id: None,
                name: name.to_string(),
            });
        }
    }
    nets.sort_by(|a, b| a.name.cmp(&b.name));

    let instances: Vec<SchematicInstanceInput> = editor
        .symbols
        .iter()
        .map(|s| SchematicInstanceInput {
            id: None,
            part_id: s.part_id,
            ref_des: s.ref_des.clone(),
            position: Some(tokito::models::Position {
                x: s.pos.x as f64,
                y: s.pos.y as f64,
            }),
            rotation: s.rotation_deg as f64,
            meta: s
                .symbol_id
                .as_ref()
                .map(|id| serde_json::json!({ "symbol_id": id, "value": s.value })),
        })
        .collect();

    let mut pins = Vec::new();
    for ((ref_des, pin_name), net_id) in &editor.connectivity_pin_net {
        let net_name = net_id_to_name
            .get(net_id)
            .cloned()
            .unwrap_or_else(|| "NET".to_string());
        pins.push(SchematicPinInput {
            instance_ref: ref_des.clone(),
            pin_name: pin_name.clone(),
            net_name,
        });
    }

    ReplaceSchematic {
        instances,
        nets,
        pins,
    }
}

/// Map one ERC violation to a canvas marker at a sensible position.
pub fn violation_to_canvas_marker(
    v: &tokito::models::ErcViolation,
    editor: &SchematicEditor,
) -> ErcMarkerOnCanvas {
    let position = marker_position(v, editor);
    ErcMarkerOnCanvas {
        code: v.code.clone(),
        message: v.message.clone(),
        severity: format!("{:?}", v.severity).to_ascii_lowercase(),
        position,
        instance_ref: v.instance_ref.clone(),
        net_name: v.net_name.clone(),
    }
}

fn marker_position(v: &tokito::models::ErcViolation, editor: &SchematicEditor) -> Pos2 {
    if let Some(refdes) = &v.instance_ref {
        if let Some(sym) = editor.symbols.iter().find(|s| s.ref_des == *refdes) {
            if let Some(pin) = &v.pin_name {
                return symbol_pin_world(sym, pin);
            }
            return sym.pos;
        }
    }
    if let Some(net) = &v.net_name {
        if let Some(seg) = editor
            .wire_segments
            .iter()
            .find(|s| s.net.eq_ignore_ascii_case(net))
        {
            return Pos2::new(
                (seg.start.x + seg.end.x) * 0.5,
                (seg.start.y + seg.end.y) * 0.5,
            );
        }
        if let Some(label) = editor.net_labels.iter().find(|l| l.name == *net) {
            return label.pos;
        }
    }
    editor
        .screen_rect
        .map(|r| Pos2::new(r.min.x + 80.0, r.min.y + 80.0))
        .unwrap_or(Pos2::new(120.0, 80.0))
}

/// Snap a dragged wire endpoint to the nearest pin, if within attach radius.
pub fn snap_segment_endpoint_to_pin(
    world: Pos2,
    symbols: &[Sym],
) -> Option<(Pos2, crate::canvas::PinEndpoint)> {
    let mut best: Option<(Pos2, crate::canvas::PinEndpoint, f32)> = None;
    for sym in symbols {
        for (pin_name, _, _) in &sym.pin_layout {
            let pw = symbol_pin_world(sym, pin_name);
            let d = pw.distance(world);
            if d <= PIN_ATTACH_RADIUS && best.as_ref().map(|(_, _, bd)| d < *bd).unwrap_or(true) {
                best = Some((
                    pw,
                    crate::canvas::PinEndpoint {
                        ref_des: sym.ref_des.clone(),
                        pin_name: pin_name.clone(),
                    },
                    d,
                ));
            }
        }
        if sym.pin_layout.is_empty() {
            for pin_name in &sym.pins {
                let pw = symbol_pin_world(sym, pin_name);
                let d = pw.distance(world);
                if d <= PIN_ATTACH_RADIUS && best.as_ref().map(|(_, _, bd)| d < *bd).unwrap_or(true)
                {
                    best = Some((
                        pw,
                        crate::canvas::PinEndpoint {
                            ref_des: sym.ref_des.clone(),
                            pin_name: pin_name.clone(),
                        },
                        d,
                    ));
                }
            }
        }
    }
    best.map(|(p, ep, _)| (p, ep))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_editor_has_no_erc_markers() {
        let mut ed = SchematicEditor::default();
        refresh_live_erc_markers(&mut ed);
        assert!(ed.erc_markers.is_empty());
    }
}
