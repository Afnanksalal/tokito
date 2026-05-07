//! Topology validation (hard errors) + light ERC (warnings) for `ReplaceSchematic`.

use crate::error::{AppError, AppResult};
use crate::models::{ErcSeverity, ErcViolation, ReplaceSchematic, SchematicValidationReport};
use std::collections::{HashMap, HashSet};

/// Structural checks required before DB persist or AI accept.
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

/// Heuristic ERC — never blocks persist; surface in API/native UI.
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
        });
    }

    out.sort_by(|a, b| a.code.cmp(&b.code));
    out
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
}
