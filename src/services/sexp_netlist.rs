//! Tokito S-expression netlist export for external tooling.

use crate::models::SchematicView;
use std::collections::HashMap;

/// Connectivity netlist in Tokito S-expression form.
pub fn export(view: &SchematicView) -> String {
    let id_to_ref: HashMap<_, _> = view
        .instances
        .iter()
        .map(|i| (i.id, i.ref_des.trim().to_string()))
        .collect();
    let mut lines = vec![
        "(export (version D)".to_string(),
        "  (design".to_string(),
        "    (source \"tokito\")".to_string(),
        "    (date \"\")".to_string(),
        "    (tool \"tokito\")".to_string(),
        "    (sheet (number 1) (name /) (tstamps /))".to_string(),
        "    (components".to_string(),
    ];

    for inst in &view.instances {
        let value = inst
            .part_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| inst.ref_des.clone());
        lines.push(format!(
            "      (comp (ref {}) (value \"{}\") (footprint \"\"))",
            escape_sexp_atom(&inst.ref_des),
            escape_sexp_string(&value),
        ));
    }
    lines.push("    )".to_string());
    lines.push("    (nets".to_string());

    let mut by_net: HashMap<uuid::Uuid, Vec<String>> = HashMap::new();
    for p in &view.pins {
        let iref = id_to_ref
            .get(&p.instance_id)
            .cloned()
            .unwrap_or_else(|| "?".into());
        let node = format!(
            "(node (ref {}) (pin \"{}\"))",
            escape_sexp_atom(&iref),
            escape_sexp_string(&p.pin_name)
        );
        by_net.entry(p.net_id).or_default().push(node);
    }

    let mut net_idx = 0u32;
    for net in &view.nets {
        net_idx += 1;
        let nodes = by_net.remove(&net.id).unwrap_or_default();
        lines.push(format!(
            "      (net (code {}) (name \"{}\"))",
            net_idx,
            escape_sexp_string(net.name.trim()),
        ));
        for node in nodes {
            lines.push(format!("        {node}"));
        }
        lines.push("      )".to_string());
    }
    lines.push("    )".to_string());
    lines.push("  )".to_string());
    lines.push(")".to_string());
    lines.join("\n")
}

fn escape_sexp_atom(s: &str) -> String {
    if s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        s.to_string()
    } else {
        format!("\"{}\"", escape_sexp_string(s))
    }
}

fn escape_sexp_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{SchematicInstance, SchematicNet, SchematicPin, SchematicView};
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn netlist_contains_ref_and_net() {
        let u1 = Uuid::new_v4();
        let n1 = Uuid::new_v4();
        let design_id = Uuid::new_v4();
        let now = Utc::now();
        let view = SchematicView {
            instances: vec![SchematicInstance {
                id: u1,
                design_id,
                part_id: None,
                ref_des: "R1".into(),
                pos_x: Some(0.0),
                pos_y: Some(0.0),
                rotation: 0.0,
                meta: json!({}),
                created_at: now,
            }],
            nets: vec![SchematicNet {
                id: n1,
                design_id,
                name: "NET1".into(),
                created_at: now,
            }],
            pins: vec![SchematicPin {
                id: Uuid::new_v4(),
                instance_id: u1,
                pin_name: "1".into(),
                net_id: n1,
                created_at: now,
            }],
        };
        let out = export(&view);
        assert!(out.contains("(ref R1)"));
        assert!(out.contains("NET1"));
    }
}
