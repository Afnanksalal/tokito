//! Rebuild electrical connectivity on a single schematic sheet.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use uuid::Uuid;

use super::disjoint_set::{DisjointSet, PointKey};
use super::net_name::sanitize_net_name;
use super::point::point_key_xy;

/// Net label scope (subset of editor / document kinds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    Local,
    Global,
    Hierarchical,
}

#[derive(Debug, Clone)]
pub struct ConnPoint {
    pub x: f64,
    pub y: f64,
}

impl ConnPoint {
    pub fn key(&self) -> PointKey {
        point_key_xy(self.x, self.y)
    }
}

#[derive(Debug, Clone)]
pub struct ConnPin {
    pub ref_des: String,
    pub pin_name: String,
    pub position: ConnPoint,
}

#[derive(Debug, Clone)]
pub struct ConnSegment {
    pub start: ConnPoint,
    pub end: ConnPoint,
    pub net_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConnLabel {
    pub position: ConnPoint,
    pub name: String,
    pub kind: LabelKind,
}

#[derive(Debug, Clone)]
pub struct ConnPower {
    pub position: ConnPoint,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct ConnectivityInput {
    pub pins: Vec<ConnPin>,
    pub segments: Vec<ConnSegment>,
    pub junctions: Vec<ConnPoint>,
    pub labels: Vec<ConnLabel>,
    pub power: Vec<ConnPower>,
    pub no_connects: Vec<ConnPoint>,
}

#[derive(Debug, Clone)]
pub struct ConnectivityResult {
    /// Stable net id per union-find root (v5 UUID from root key).
    pub net_id_for_root: HashMap<PointKey, Uuid>,
    /// Display name per net id.
    pub display_name: HashMap<Uuid, String>,
    /// Per wire segment: (net_id, display_name).
    pub segment_nets: Vec<(Uuid, String)>,
    /// Pin refdes+name → net id when electrically connected and not no-connect.
    pub pin_net: HashMap<(String, String), Uuid>,
}

const NET_ID_NAMESPACE: Uuid = Uuid::from_u128(0x6b7a5e20_3f1c_4e5a_9c8d_1a2b3c4d5e6f);

fn net_id_for_root(root: PointKey) -> Uuid {
    let name = format!("tokito-net:{}:{}", root.0, root.1);
    Uuid::new_v5(&NET_ID_NAMESPACE, name.as_bytes())
}

fn pick_display_name(candidates: &BTreeSet<String>, auto_index: usize) -> String {
    if candidates.is_empty() {
        return format!("Net_{auto_index}");
    }
    // Prefer power-style short names, then user labels, then generic NET*.
    let mut v: Vec<_> = candidates.iter().cloned().collect();
    v.sort_by(|a, b| {
        let score = |s: &str| {
            let u = s.to_ascii_uppercase();
            if u == "GND" || u == "VCC" || u.starts_with('+') || u.starts_with('-') {
                0
            } else if !u.starts_with("NET") {
                1
            } else {
                2
            }
        };
        score(a).cmp(&score(b)).then_with(|| a.len().cmp(&b.len()))
    });
    sanitize_net_name(v.first().unwrap())
}

/// Union-find connectivity for one sheet; assign stable net ids and display names.
pub fn rebuild_connectivity(input: &ConnectivityInput) -> ConnectivityResult {
    let mut dsu = DisjointSet::new();
    let no_connect: BTreeSet<PointKey> = input.no_connects.iter().map(|p| p.key()).collect();

    let mut pin_at: BTreeMap<PointKey, Vec<(String, String)>> = BTreeMap::new();
    for pin in &input.pins {
        let k = pin.position.key();
        dsu.make(k);
        pin_at
            .entry(k)
            .or_default()
            .push((pin.ref_des.clone(), pin.pin_name.clone()));
    }

    for j in &input.junctions {
        dsu.make(j.key());
    }
    for label in &input.labels {
        dsu.make(label.position.key());
    }
    for pwr in &input.power {
        dsu.make(pwr.position.key());
    }

    for seg in &input.segments {
        let ks = seg.start.key();
        let ke = seg.end.key();
        dsu.make(ks);
        dsu.make(ke);
        dsu.union(ks, ke);
        for j in &input.junctions {
            let kj = j.key();
            if point_on_segment(j, &seg.start, &seg.end) {
                dsu.union(ks, kj);
                dsu.union(ke, kj);
            }
        }
        for label in &input.labels {
            let kl = label.position.key();
            if kl == ks || kl == ke || point_on_segment(&label.position, &seg.start, &seg.end) {
                dsu.union(ks, kl);
            }
        }
        for pwr in &input.power {
            let kp = pwr.position.key();
            if kp == ks || kp == ke || point_on_segment(&pwr.position, &seg.start, &seg.end) {
                dsu.union(ks, kp);
            }
        }
    }

    // Labels and power at exact positions tie to pins/wires via shared keys above.
    apply_global_label_links(&input.labels, &mut dsu);
    apply_hierarchical_label_links(&input.labels, &mut dsu);

    let mut names_by_root: BTreeMap<PointKey, BTreeSet<String>> = BTreeMap::new();
    for seg in &input.segments {
        if let Some(hint) = seg.net_hint.as_deref().filter(|s| !s.trim().is_empty()) {
            let root = dsu.find(seg.start.key());
            names_by_root
                .entry(root)
                .or_default()
                .insert(sanitize_net_name(hint));
        }
    }
    for label in &input.labels {
        let root = dsu.find(label.position.key());
        names_by_root
            .entry(root)
            .or_default()
            .insert(sanitize_net_name(&label.name));
    }
    for pwr in &input.power {
        let root = dsu.find(pwr.position.key());
        names_by_root
            .entry(root)
            .or_default()
            .insert(sanitize_net_name(&pwr.name));
    }

    let mut root_to_id: HashMap<PointKey, Uuid> = HashMap::new();
    let mut display_name: HashMap<Uuid, String> = HashMap::new();
    let mut auto_idx = 0usize;

    let mut roots: BTreeSet<PointKey> = BTreeSet::new();
    for seg in &input.segments {
        roots.insert(dsu.find(seg.start.key()));
    }
    for pin in &input.pins {
        roots.insert(dsu.find(pin.position.key()));
    }
    for label in &input.labels {
        roots.insert(dsu.find(label.position.key()));
    }
    for pwr in &input.power {
        roots.insert(dsu.find(pwr.position.key()));
    }

    for root in roots {
        let id = net_id_for_root(root);
        root_to_id.insert(root, id);
        let candidates = names_by_root.get(&root).cloned().unwrap_or_default();
        auto_idx += 1;
        display_name.insert(id, pick_display_name(&candidates, auto_idx));
    }

    let mut segment_nets = Vec::with_capacity(input.segments.len());
    for seg in &input.segments {
        let root = dsu.find(seg.start.key());
        let id = *root_to_id
            .entry(root)
            .or_insert_with(|| net_id_for_root(root));
        let name = display_name
            .get(&id)
            .cloned()
            .unwrap_or_else(|| "NET".to_string());
        segment_nets.push((id, name));
    }

    let mut pin_net = HashMap::new();
    for (k, pins) in pin_at {
        if no_connect.contains(&k) {
            continue;
        }
        let root = dsu.find(k);
        if let Some(&id) = root_to_id.get(&root) {
            for (rd, pn) in pins {
                pin_net.insert((rd, pn), id);
            }
        }
    }

    ConnectivityResult {
        net_id_for_root: root_to_id,
        display_name,
        segment_nets,
        pin_net,
    }
}

/// `child_sheet/local_net` on parent sheet unions with local labels named `local_net`.
fn parse_hierarchical_label(name: &str) -> Option<(String, String)> {
    let (sheet, net) = name.trim().split_once('/')?;
    let sheet = sheet.trim();
    let net = net.trim();
    if sheet.is_empty() || net.is_empty() {
        return None;
    }
    Some((sheet.to_string(), net.to_string()))
}

fn apply_hierarchical_label_links(labels: &[ConnLabel], dsu: &mut DisjointSet) {
    let mut ports_by_target: BTreeMap<(String, String), Vec<PointKey>> = BTreeMap::new();
    let mut local_by_name: BTreeMap<String, Vec<PointKey>> = BTreeMap::new();

    for label in labels {
        let pt = label.position.key();
        dsu.make(pt);
        match label.kind {
            LabelKind::Hierarchical => {
                if let Some((sheet, net)) = parse_hierarchical_label(&label.name) {
                    ports_by_target.entry((sheet, net)).or_default().push(pt);
                }
            }
            LabelKind::Local => {
                local_by_name
                    .entry(sanitize_net_name(&label.name))
                    .or_default()
                    .push(pt);
            }
            LabelKind::Global => {}
        }
    }

    for ((_sheet, net), ports) in ports_by_target {
        let Some(&first) = ports.first() else {
            continue;
        };
        for &p in ports.iter().skip(1) {
            dsu.union(first, p);
        }
        if let Some(locals) = local_by_name.get(&net) {
            for &lp in locals {
                dsu.union(first, lp);
            }
        }
    }
}

fn apply_global_label_links(labels: &[ConnLabel], dsu: &mut DisjointSet) {
    let mut global_by_name: BTreeMap<String, Vec<PointKey>> = BTreeMap::new();
    for label in labels {
        if label.kind != LabelKind::Global {
            continue;
        }
        let key = sanitize_net_name(&label.name);
        if key == "NET" {
            continue;
        }
        let pt = label.position.key();
        dsu.make(pt);
        global_by_name.entry(key).or_default().push(pt);
    }
    for points in global_by_name.values() {
        if let Some(&first) = points.first() {
            for &p in &points[1..] {
                dsu.union(first, p);
            }
        }
    }
}

fn point_on_segment(p: &ConnPoint, start: &ConnPoint, end: &ConnPoint) -> bool {
    let cross = (p.y - start.y) * (end.x - start.x) - (p.x - start.x) * (end.y - start.y);
    if cross.abs() > 0.01 {
        return false;
    }
    let min_x = start.x.min(end.x) - 0.01;
    let max_x = start.x.max(end.x) + 0.01;
    let min_y = start.y.min(end.y) - 0.01;
    let max_y = start.y.max(end.y) + 0.01;
    p.x >= min_x && p.x <= max_x && p.y >= min_y && p.y <= max_y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_segments_share_net_id_and_name() {
        let input = ConnectivityInput {
            segments: vec![
                ConnSegment {
                    start: ConnPoint { x: 0.0, y: 0.0 },
                    end: ConnPoint { x: 40.0, y: 0.0 },
                    net_hint: Some("CLK".into()),
                },
                ConnSegment {
                    start: ConnPoint { x: 40.0, y: 0.0 },
                    end: ConnPoint { x: 80.0, y: 0.0 },
                    net_hint: Some("DATA".into()),
                },
            ],
            ..Default::default()
        };
        let r = rebuild_connectivity(&input);
        assert_eq!(r.segment_nets[0].0, r.segment_nets[1].0);
        assert_eq!(r.segment_nets[0].1, r.segment_nets[1].1);
    }

    #[test]
    fn label_names_the_net() {
        let input = ConnectivityInput {
            segments: vec![ConnSegment {
                start: ConnPoint { x: 0.0, y: 0.0 },
                end: ConnPoint { x: 40.0, y: 0.0 },
                net_hint: None,
            }],
            labels: vec![ConnLabel {
                position: ConnPoint { x: 20.0, y: 0.0 },
                name: "VCC".into(),
                kind: LabelKind::Local,
            }],
            ..Default::default()
        };
        let r = rebuild_connectivity(&input);
        assert_eq!(r.segment_nets[0].1, "VCC");
    }

    #[test]
    fn hierarchical_port_links_local_label() {
        let input = ConnectivityInput {
            segments: vec![ConnSegment {
                start: ConnPoint { x: 0.0, y: 0.0 },
                end: ConnPoint { x: 40.0, y: 0.0 },
                net_hint: None,
            }],
            labels: vec![
                ConnLabel {
                    position: ConnPoint { x: 20.0, y: 0.0 },
                    name: "child/SIGNAL".into(),
                    kind: LabelKind::Hierarchical,
                },
                ConnLabel {
                    position: ConnPoint { x: 20.0, y: 0.0 },
                    name: "SIGNAL".into(),
                    kind: LabelKind::Local,
                },
            ],
            ..Default::default()
        };
        let r = rebuild_connectivity(&input);
        assert_eq!(r.segment_nets[0].1, "SIGNAL");
    }

    #[test]
    fn pin_gets_net_when_wired() {
        let input = ConnectivityInput {
            pins: vec![ConnPin {
                ref_des: "R1".into(),
                pin_name: "1".into(),
                position: ConnPoint { x: 0.0, y: 0.0 },
            }],
            segments: vec![ConnSegment {
                start: ConnPoint { x: 0.0, y: 0.0 },
                end: ConnPoint { x: 40.0, y: 0.0 },
                net_hint: Some("N1".into()),
            }],
            ..Default::default()
        };
        let r = rebuild_connectivity(&input);
        assert!(r.pin_net.contains_key(&("R1".into(), "1".into())));
    }

    #[test]
    fn no_connect_excludes_pin() {
        let input = ConnectivityInput {
            pins: vec![ConnPin {
                ref_des: "U1".into(),
                pin_name: "NC".into(),
                position: ConnPoint { x: 0.0, y: 0.0 },
            }],
            segments: vec![ConnSegment {
                start: ConnPoint { x: 0.0, y: 0.0 },
                end: ConnPoint { x: 40.0, y: 0.0 },
                net_hint: Some("X".into()),
            }],
            no_connects: vec![ConnPoint { x: 0.0, y: 0.0 }],
            ..Default::default()
        };
        let r = rebuild_connectivity(&input);
        assert!(!r.pin_net.contains_key(&("U1".into(), "NC".into())));
    }

    #[test]
    fn global_labels_merge_across_positions() {
        let input = ConnectivityInput {
            segments: vec![
                ConnSegment {
                    start: ConnPoint { x: 0.0, y: 0.0 },
                    end: ConnPoint { x: 40.0, y: 0.0 },
                    net_hint: None,
                },
                ConnSegment {
                    start: ConnPoint { x: 200.0, y: 0.0 },
                    end: ConnPoint { x: 240.0, y: 0.0 },
                    net_hint: None,
                },
            ],
            labels: vec![
                ConnLabel {
                    position: ConnPoint { x: 20.0, y: 0.0 },
                    name: "GND".into(),
                    kind: LabelKind::Global,
                },
                ConnLabel {
                    position: ConnPoint { x: 220.0, y: 0.0 },
                    name: "GND".into(),
                    kind: LabelKind::Global,
                },
            ],
            ..Default::default()
        };
        let r = rebuild_connectivity(&input);
        assert_eq!(r.segment_nets[0].0, r.segment_nets[1].0);
        assert_eq!(r.segment_nets[0].1, "GND");
    }
}
