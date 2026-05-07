//! Text connectivity export from stored schematic views.

use crate::models::SchematicView;
use std::collections::HashMap;

/// Simple SPICE-ish adjacency list: one line per `(net, ref_des.pin_name)`.
pub fn connectivity_text(view: &SchematicView) -> String {
    let id_to_ref: HashMap<_, _> = view
        .instances
        .iter()
        .map(|i| (i.id, i.ref_des.trim().to_string()))
        .collect();
    let net_id_to_name: HashMap<_, _> = view
        .nets
        .iter()
        .map(|n| (n.id, n.name.trim().to_string()))
        .collect();

    let mut rows: Vec<(String, String)> = Vec::new();
    for p in &view.pins {
        let net = net_id_to_name
            .get(&p.net_id)
            .cloned()
            .unwrap_or_else(|| "?NET".into());
        let iref = id_to_ref
            .get(&p.instance_id)
            .cloned()
            .unwrap_or_else(|| "?REF".into());
        rows.push((net, format!("{}.{}", iref, p.pin_name.trim())));
    }
    rows.sort();

    let mut lines = vec![
        "* Tokito connectivity export".to_string(),
        "* Lines: NET  REFDES.PINNAME".to_string(),
    ];
    for (net, pinref) in rows {
        lines.push(format!("{net}  {pinref}"));
    }
    lines.push(String::new());
    lines.join("\n")
}
