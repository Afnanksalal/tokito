//! Propose reviewable schematic edits from ERC violations (native copilot apply flow).

use crate::models::{
    EditProvenance, ErcViolation, SchematicDocument, SchematicEditBatch, SchematicEditOp,
};

/// Build a batch of suggested fixes for the current document and ERC list.
pub fn propose_fixes(doc: &SchematicDocument, violations: &[ErcViolation]) -> SchematicEditBatch {
    let mut ops = Vec::new();
    let mut op_provenance = Vec::new();

    for v in violations {
        match v.code.as_str() {
            "ERC_MISSING_FOOTPRINT" => {
                let Some(refdes) = &v.instance_ref else {
                    continue;
                };
                let Some(sym) = doc.symbols.iter().find(|s| s.ref_des == *refdes) else {
                    continue;
                };
                let fp = guess_footprint(sym);
                ops.push(SchematicEditOp::SetInstanceField {
                    ref_des: refdes.clone(),
                    field: "footprint".into(),
                    value: fp.clone(),
                    summary: format!("Assign footprint '{fp}' to {refdes}"),
                });
                op_provenance.push(EditProvenance::ErcFix {
                    code: v.code.clone(),
                });
            }
            "ERC_UNCONNECTED_PIN" => {
                if let (Some(refdes), Some(pin)) = (&v.instance_ref, &v.pin_name) {
                    let net = format!("NC_{refdes}_{pin}");
                    ops.push(SchematicEditOp::ConnectPins {
                        net_name: net,
                        pins: vec![(refdes.clone(), pin.clone())],
                        summary: format!("Isolate unconnected pin {refdes}.{pin}"),
                    });
                    op_provenance.push(EditProvenance::ErcFix {
                        code: v.code.clone(),
                    });
                }
            }
            "ERC_DANGLING_LABEL" | "ERC_UNCONNECTED_LABEL" => {
                if let Some(net) = &v.net_name {
                    if let Some(label) = doc.net_labels.iter().find(|l| l.name == *net) {
                        if let Some(sym) = doc.symbols.first() {
                            ops.push(SchematicEditOp::ConnectPins {
                                net_name: net.clone(),
                                pins: vec![(sym.ref_des.clone(), "1".into())],
                                summary: format!(
                                    "Connect label {net} to {}/1 (review)",
                                    sym.ref_des
                                ),
                            });
                            op_provenance.push(EditProvenance::ErcFix {
                                code: v.code.clone(),
                            });
                        }
                        let _ = label;
                    }
                }
            }
            _ => {}
        }
    }

    SchematicEditBatch {
        ops,
        op_provenance,
        provenance: vec![EditProvenance::UserRequest],
    }
}

fn guess_footprint(sym: &crate::models::DocumentSymbol) -> String {
    if let Some(fp) = sym.footprint_ref.as_deref() {
        if !fp.trim().is_empty() {
            return fp.trim().to_string();
        }
    }
    let prefix: String = sym
        .ref_des
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    match prefix.as_str() {
        "R" => "R_0805_2012Metric".into(),
        "C" => "C_0805_2012Metric".into(),
        "L" => "L_0805_2012Metric".into(),
        "D" => "D_SOD-123".into(),
        "Q" => "SOT-23".into(),
        "J" => "PinHeader_1x02_P2.54mm_Vertical".into(),
        _ => "Generic_2P_5.0x5.0mm".into(),
    }
}

/// Merge batch-level provenance with per-op tags for the native UI.
pub fn provenance_label(p: &EditProvenance) -> &'static str {
    match p {
        EditProvenance::ModelInference => "AI model",
        EditProvenance::BomLine { .. } => "BOM",
        EditProvenance::ResearchArtifact { .. } => "Research",
        EditProvenance::UserRequest => "You",
        EditProvenance::ErcFix { .. } => "ERC fix",
    }
}
