//! Net name resolution and highlighting helpers.

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::canvas::{NetLabel, WireSegment};

/// All segment indices on the same topological net (by `net_id`).
pub fn segment_indices_for_net_id(net_id: Uuid, segments: &[WireSegment]) -> Vec<usize> {
    segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.net_id == net_id)
        .map(|(i, _)| i)
        .collect()
}

/// All segment indices that share a display net name (legacy / label match).
pub fn segment_indices_for_net(
    net: &str,
    segments: &[WireSegment],
    labels: &[NetLabel],
) -> Vec<usize> {
    let net = net.trim();
    if net.is_empty() {
        return vec![];
    }
    let mut out: Vec<usize> = segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.net.trim() == net)
        .map(|(i, _)| i)
        .collect();
    let _ = labels.iter().any(|l| l.name.trim() == net);
    out.sort_unstable();
    out.dedup();
    out
}

/// Union of net names referenced on the canvas.
pub fn all_net_names(segments: &[WireSegment], labels: &[NetLabel]) -> Vec<String> {
    let mut names: HashSet<String> = HashSet::new();
    for s in segments {
        names.insert(s.net.trim().to_string());
    }
    for l in labels {
        names.insert(l.name.trim().to_string());
    }
    let mut v: Vec<_> = names.into_iter().filter(|n| !n.is_empty()).collect();
    v.sort();
    v
}

/// Component refdes list for design manager.
pub fn components_summary(
    symbols: &[crate::canvas::Sym],
    part_cache: &HashMap<uuid::Uuid, String>,
) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = symbols
        .iter()
        .map(|s| {
            let mpn = s
                .part_id
                .and_then(|id| part_cache.get(&id))
                .cloned()
                .unwrap_or_else(|| "—".to_string());
            (s.ref_des.clone(), mpn)
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

pub use segment_indices_for_net as wire_indices_for_net;
