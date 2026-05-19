//! Propose BOM lines from placed schematic instances.

use crate::models::{BomLineInput, ReplaceSchematic};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProposedBomLine {
    pub part_id: Uuid,
    pub quantity: f64,
    pub notes: Option<String>,
}

/// Group schematic instances by `part_id` and count quantities.
pub fn propose_from_schematic(s: &ReplaceSchematic) -> Vec<ProposedBomLine> {
    let mut counts: HashMap<Uuid, u32> = HashMap::new();
    for inst in &s.instances {
        let Some(pid) = inst.part_id else {
            continue;
        };
        *counts.entry(pid).or_insert(0) += 1;
    }
    let mut out: Vec<ProposedBomLine> = counts
        .into_iter()
        .map(|(part_id, qty)| ProposedBomLine {
            part_id,
            quantity: qty as f64,
            notes: None,
        })
        .collect();
    out.sort_by_key(|a| a.part_id);
    out
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BomDiffSummary {
    pub in_sync: bool,
    pub db_line_count: usize,
    pub proposed_line_count: usize,
    pub quantity_delta: f64,
    pub message: String,
}

pub fn diff_summary(
    db_lines: &[crate::models::BomLine],
    schematic: &ReplaceSchematic,
) -> BomDiffSummary {
    let proposed = propose_from_schematic(schematic);
    let db_qty: f64 = db_lines.iter().map(|l| l.quantity).sum();
    let prop_qty: f64 = proposed.iter().map(|l| l.quantity).sum();
    let in_sync = db_lines.len() == proposed.len() && (db_qty - prop_qty).abs() < 0.01;
    let message = if in_sync {
        "BOM matches schematic instance counts".into()
    } else {
        format!(
            "BOM has {} line(s), schematic suggests {} (qty {} vs {})",
            db_lines.len(),
            proposed.len(),
            db_qty,
            prop_qty
        )
    };
    BomDiffSummary {
        in_sync,
        db_line_count: db_lines.len(),
        proposed_line_count: proposed.len(),
        quantity_delta: prop_qty - db_qty,
        message,
    }
}

pub fn to_bom_inputs(lines: &[ProposedBomLine]) -> Vec<BomLineInput> {
    lines
        .iter()
        .enumerate()
        .map(|(i, l)| BomLineInput {
            part_id: l.part_id,
            quantity: l.quantity,
            sort_order: i as i32,
            notes: l.notes.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ReplaceSchematic, SchematicInstanceInput};

    #[test]
    fn counts_instances_per_part() {
        let pid = Uuid::new_v4();
        let s = ReplaceSchematic {
            instances: vec![
                SchematicInstanceInput {
                    id: None,
                    ref_des: "R1".into(),
                    part_id: Some(pid),
                    position: None,
                    rotation: 0.0,
                    meta: None,
                },
                SchematicInstanceInput {
                    id: None,
                    ref_des: "R2".into(),
                    part_id: Some(pid),
                    position: None,
                    rotation: 0.0,
                    meta: None,
                },
            ],
            nets: vec![],
            pins: vec![],
        };
        let p = propose_from_schematic(&s);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].quantity, 2.0);
    }
}
