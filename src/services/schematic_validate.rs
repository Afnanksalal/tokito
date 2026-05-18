//! Topology validation (hard errors) + light ERC (warnings) for `ReplaceSchematic`.

use crate::error::{AppError, AppResult};
use crate::models::{
    ElectricalPinType, ErcSeverity, ErcViolation, NetLabelKind, ReplaceSchematic,
    SchematicDocument, SchematicValidationReport,
};
use std::collections::{HashMap, HashSet};

/// Structural checks before persist or copilot accept.
pub fn validate_topology(s: &ReplaceSchematic) -> AppResult<()> {
    let mut refs = HashSet::new();
    for inst in &s.instances {
        let r = inst.ref_des.trim();
        if r.is_empty() {
            return Err(AppError::BadRequest("empty ref_des".into()));
        }
        if !refs.insert(r.to_string()) {
            return Err(AppError::BadRequest(format!("duplicate ref_des {r}")));
        }
    }
    let mut nets = HashSet::new();
    for n in &s.nets {
        let name = n.name.trim();
        if name.is_empty() {
            return Err(AppError::BadRequest("empty net name".into()));
        }
        if !nets.insert(name.to_string()) {
            return Err(AppError::BadRequest(format!("duplicate net {name}")));
        }
    }

    let mut pin_keys = HashSet::new();
    for p in &s.pins {
        let iref = p.instance_ref.trim();
        let nname = p.net_name.trim();
        if !refs.contains(iref) {
            return Err(AppError::BadRequest(format!(
                "pin references unknown instance {}",
                p.instance_ref
            )));
        }
        if !nets.contains(nname) {
            return Err(AppError::BadRequest(format!(
                "pin references unknown net {}",
                p.net_name
            )));
        }
        if p.pin_name.trim().is_empty() {
            return Err(AppError::BadRequest("empty pin_name".into()));
        }
        let pk = (iref.to_string(), p.pin_name.trim().to_string());
        if !pin_keys.insert(pk) {
            return Err(AppError::BadRequest(format!(
                "duplicate pin_name '{}' on instance {}",
                p.pin_name.trim(),
                p.instance_ref
            )));
        }
    }

    Ok(())
}

fn net_is_gnd_like(name: &str) -> bool {
    let x = name.trim().to_ascii_lowercase();
    x.contains("gnd") || x == "vss" || x == "vee" || x == "ground"
}

/// Heuristic ERC; advisory only.
pub fn erc_light(s: &ReplaceSchematic) -> Vec<ErcViolation> {
    let mut out = Vec::new();

    let mut pins_per_net: HashMap<String, usize> = HashMap::new();
    let mut pins_per_inst: HashMap<String, usize> = HashMap::new();
    for inst in &s.instances {
        pins_per_inst
            .entry(inst.ref_des.trim().to_string())
            .or_insert(0);
    }
    for p in &s.pins {
        let nn = p.net_name.trim().to_string();
        *pins_per_net.entry(nn).or_insert(0) += 1;
        let ir = p.instance_ref.trim().to_string();
        *pins_per_inst.entry(ir).or_insert(0) += 1;
    }

    for (net, n) in &pins_per_net {
        if *n == 1 {
            out.push(ErcViolation {
                code: "ERC_SINGLE_PIN_NET".into(),
                severity: ErcSeverity::Warning,
                message: format!("Net '{net}' has only one pin connection (floating stub?)"),
                detail: None,
                instance_ref: None,
                net_name: Some(net.clone()),
                pin_name: None,
            });
        }
    }

    for net in &s.nets {
        let name = net.name.trim();
        if !pins_per_net.contains_key(name) {
            out.push(ErcViolation {
                code: "ERC_UNUSED_NET".into(),
                severity: ErcSeverity::Info,
                message: format!("Net '{name}' is declared but has no pins"),
                detail: None,
                instance_ref: None,
                net_name: Some(name.to_string()),
                pin_name: None,
            });
        }
    }

    for (ref_des, count) in &pins_per_inst {
        if *count == 0 {
            out.push(ErcViolation {
                code: "ERC_INSTANCE_NO_PINS".into(),
                severity: ErcSeverity::Warning,
                message: format!("Symbol '{ref_des}' has no pin records"),
                detail: Some(
                    "Add schematic_pins tying pins to nets, or remove the instance.".into(),
                ),
                instance_ref: Some(ref_des.clone()),
                net_name: None,
                pin_name: None,
            });
        }
    }

    let gnd_like: Vec<&str> = s
        .nets
        .iter()
        .map(|n| n.name.trim())
        .filter(|n| net_is_gnd_like(n))
        .collect();
    if gnd_like.len() > 1 {
        out.push(ErcViolation {
            code: "ERC_MULTI_GROUND_NET".into(),
            severity: ErcSeverity::Warning,
            message: format!(
                "Multiple ground-like nets defined ({}) — verify single star ground discipline.",
                gnd_like.join(", ")
            ),
            detail: None,
            instance_ref: None,
            net_name: None,
            pin_name: None,
        });
    }

    for inst in &s.instances {
        if inst.part_id.is_none() {
            out.push(ErcViolation {
                code: "ERC_MISSING_PART_ID".into(),
                severity: ErcSeverity::Info,
                message: format!("'{0}' has no catalog part_id", inst.ref_des.trim()),
                detail: Some("Link a BOM line or assign part_id for procurement.".into()),
                instance_ref: Some(inst.ref_des.trim().to_string()),
                net_name: None,
                pin_name: None,
            });
        }
    }

    out.sort_by(|a, b| a.code.cmp(&b.code));
    out
}

/// Deeper ERC using document geometry and pin electrical classes.
pub fn erc_deep(s: &ReplaceSchematic, doc: &SchematicDocument) -> Vec<ErcViolation> {
    let mut out = Vec::new();

    let mut pin_types: HashMap<(String, String), ElectricalPinType> = HashMap::new();
    for sym in &doc.symbols {
        for pin in &sym.pins {
            pin_types.insert((sym.ref_des.clone(), pin.name.clone()), pin.electrical_type);
        }
    }

    let mut drivers_per_net: HashMap<String, usize> = HashMap::new();
    let mut outputs_per_net: HashMap<String, usize> = HashMap::new();
    let mut power_in_per_net: HashMap<String, usize> = HashMap::new();

    for p in &s.pins {
        let net = p.net_name.trim().to_string();
        let key = (
            p.instance_ref.trim().to_string(),
            p.pin_name.trim().to_string(),
        );
        let et = pin_types.get(&key).copied().unwrap_or_default();
        match et {
            ElectricalPinType::PowerOut
            | ElectricalPinType::Output
            | ElectricalPinType::OpenCollector
            | ElectricalPinType::OpenEmitter => {
                *drivers_per_net.entry(net.clone()).or_insert(0) += 1;
            }
            ElectricalPinType::PowerIn => {
                *power_in_per_net.entry(net.clone()).or_insert(0) += 1;
            }
            _ => {}
        }
        if matches!(
            et,
            ElectricalPinType::Output | ElectricalPinType::PowerOut | ElectricalPinType::TriState
        ) {
            *outputs_per_net.entry(net).or_insert(0) += 1;
        }
    }

    for pwr in &doc.power_symbols {
        let net = pwr.name.trim().to_string();
        *drivers_per_net.entry(net.clone()).or_insert(0) += 1;
    }

    for (net, count) in &power_in_per_net {
        let drivers = drivers_per_net.get(net).copied().unwrap_or(0)
            + doc
                .power_symbols
                .iter()
                .filter(|p| p.name.trim() == net.as_str())
                .count();
        if drivers == 0 && *count > 0 {
            out.push(ErcViolation {
                code: "ERC_POWER_IN_NO_DRIVER".into(),
                severity: ErcSeverity::Warning,
                message: format!(
                    "Power input on net '{net}' has no driver (power symbol or output)"
                ),
                detail: None,
                instance_ref: None,
                net_name: Some(net.clone()),
                pin_name: None,
            });
        }
    }

    for (net, count) in &outputs_per_net {
        if *count > 1 {
            out.push(ErcViolation {
                code: "ERC_OUTPUT_CONFLICT".into(),
                severity: ErcSeverity::Warning,
                message: format!("Net '{net}' has multiple output drivers ({count})"),
                detail: None,
                instance_ref: None,
                net_name: Some(net.clone()),
                pin_name: None,
            });
        }
    }

    for label in &doc.net_labels {
        let pos = label.position;
        let attached = doc.wire_segments.iter().any(|seg| {
            point_on_segment(
                pos.x,
                pos.y,
                seg.start.x,
                seg.start.y,
                seg.end.x,
                seg.end.y,
                8.0,
            )
        });
        if !attached && label.kind == NetLabelKind::Local {
            out.push(ErcViolation {
                code: "ERC_DANGLING_LABEL".into(),
                severity: ErcSeverity::Info,
                message: format!("Label '{}' is not attached to a wire segment", label.name),
                detail: None,
                instance_ref: None,
                net_name: Some(label.name.clone()),
                pin_name: None,
            });
        }
    }

    for sym in &doc.symbols {
        if sym.footprint_ref.as_deref().unwrap_or("").trim().is_empty() && sym.part_id.is_some() {
            out.push(ErcViolation {
                code: "ERC_MISSING_FOOTPRINT".into(),
                severity: ErcSeverity::Info,
                message: format!("'{0}' has no footprint assigned", sym.ref_des),
                detail: None,
                instance_ref: Some(sym.ref_des.clone()),
                net_name: None,
                pin_name: None,
            });
        }
    }

    out.sort_by(|a, b| a.code.cmp(&b.code));
    out
}

fn point_on_segment(px: f64, py: f64, x1: f64, y1: f64, x2: f64, y2: f64, tol: f64) -> bool {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-9 {
        return (px - x1).hypot(py - y1) <= tol;
    }
    let t = ((px - x1) * dx + (py - y1) * dy) / len2;
    let t = t.clamp(0.0, 1.0);
    let qx = x1 + t * dx;
    let qy = y1 + t * dy;
    (px - qx).hypot(py - qy) <= tol
}

/// Light + deep ERC combined.
pub fn erc_full(s: &ReplaceSchematic, doc: &SchematicDocument) -> Vec<ErcViolation> {
    erc_full_with_options(s, doc, false)
}

pub fn erc_full_with_options(
    s: &ReplaceSchematic,
    doc: &SchematicDocument,
    strict: bool,
) -> Vec<ErcViolation> {
    let mut out = erc_light(s);
    out.extend(erc_deep_with_options(s, doc, strict));
    out.sort_by(|a, b| a.code.cmp(&b.code));
    out.dedup_by(|a, b| a.code == b.code && a.message == b.message);
    out
}

pub fn has_blocking_erc(violations: &[ErcViolation]) -> bool {
    violations.iter().any(|v| v.severity == ErcSeverity::Error)
}

fn erc_deep_with_options(
    s: &ReplaceSchematic,
    doc: &SchematicDocument,
    strict: bool,
) -> Vec<ErcViolation> {
    let mut out = erc_deep(s, doc);
    if strict {
        for v in &mut out {
            if v.code == "ERC_OUTPUT_CONFLICT" {
                v.severity = ErcSeverity::Error;
            }
        }
    }
    out
}

/// Map ERC violations to document markers for canvas display / persistence.
pub fn violations_to_erc_markers(
    violations: &[ErcViolation],
    sheet_id: &str,
    default_pos: (f64, f64),
) -> Vec<crate::models::DocumentErcMarker> {
    use crate::models::{DocumentErcMarker, DocumentPoint};
    violations
        .iter()
        .map(|v| DocumentErcMarker {
            id: uuid::Uuid::new_v4(),
            sheet_id: sheet_id.to_string(),
            severity: format!("{:?}", v.severity).to_ascii_lowercase(),
            code: v.code.clone(),
            message: v.message.clone(),
            position: DocumentPoint {
                x: default_pos.0,
                y: default_pos.1,
            },
            instance_ref: v.instance_ref.clone(),
            net_name: v.net_name.clone(),
        })
        .collect()
}

pub fn validation_report(s: &ReplaceSchematic) -> SchematicValidationReport {
    match validate_topology(s) {
        Ok(()) => SchematicValidationReport {
            topology_ok: true,
            topology_error: None,
            erc_warnings: erc_light(s),
        },
        Err(e) => SchematicValidationReport {
            topology_ok: false,
            topology_error: Some(e.to_string()),
            erc_warnings: vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Position, ReplaceSchematic, SchematicInstanceInput, SchematicNetInput, SchematicPinInput,
    };

    fn minimal_ok() -> ReplaceSchematic {
        ReplaceSchematic {
            instances: vec![SchematicInstanceInput {
                id: None,
                part_id: None,
                ref_des: "U1".into(),
                position: Some(Position { x: 0.0, y: 0.0 }),
                rotation: 0.0,
                meta: None,
            }],
            nets: vec![SchematicNetInput {
                id: None,
                name: "GND".into(),
            }],
            pins: vec![SchematicPinInput {
                instance_ref: "U1".into(),
                pin_name: "g".into(),
                net_name: "GND".into(),
            }],
        }
    }

    #[test]
    fn topology_accepts_minimal() {
        validate_topology(&minimal_ok()).unwrap();
    }

    #[test]
    fn topology_rejects_duplicate_refdes() {
        let mut s = minimal_ok();
        s.instances.push(SchematicInstanceInput {
            id: None,
            part_id: None,
            ref_des: "U1".into(),
            position: None,
            rotation: 0.0,
            meta: None,
        });
        assert!(validate_topology(&s).is_err());
    }

    #[test]
    fn topology_rejects_unknown_net_on_pin() {
        let mut s = minimal_ok();
        s.pins[0].net_name = "MISSING".into();
        assert!(validate_topology(&s).is_err());
    }

    #[test]
    fn erc_flags_single_pin_net() {
        let w = erc_light(&minimal_ok());
        assert!(w.iter().any(|v| v.code == "ERC_SINGLE_PIN_NET"));
    }

    #[test]
    fn erc_markers_from_violations() {
        let v = erc_light(&minimal_ok());
        let markers = violations_to_erc_markers(&v, "root", (0.0, 0.0));
        assert_eq!(markers.len(), v.len());
    }
}
