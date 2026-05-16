//! Keep wire paths orthogonal when symbols move (rebuild Manhattan routes between pins).

use std::collections::{HashMap, HashSet};

use egui::Pos2;

use crate::canvas::{manhattan_segments, symbol_pin_world, PinEndpoint, Sym, WireSegment};

const JOIN_EPS: f32 = 4.0;

#[derive(Clone, PartialEq, Eq, Hash)]
struct PinKey {
    ref_des: String,
    pin_name: String,
}

impl From<&PinEndpoint> for PinKey {
    fn from(p: &PinEndpoint) -> Self {
        Self {
            ref_des: p.ref_des.clone(),
            pin_name: p.pin_name.clone(),
        }
    }
}

fn points_join(a: Pos2, b: Pos2) -> bool {
    a.distance(b) <= JOIN_EPS
}

fn shares_endpoint(a: &WireSegment, b: &WireSegment) -> bool {
    points_join(a.start, b.start)
        || points_join(a.start, b.end)
        || points_join(a.end, b.start)
        || points_join(a.end, b.end)
}

fn endpoint_world(symbols: &[Sym], ep: &PinEndpoint) -> Option<Pos2> {
    let sym = symbols.iter().find(|s| s.ref_des == ep.ref_des)?;
    Some(symbol_pin_world(sym, &ep.pin_name))
}

fn collect_component(
    start: usize,
    segments: &[WireSegment],
    visited: &mut [bool],
) -> (Vec<usize>, HashMap<PinKey, PinEndpoint>, String) {
    let mut stack = vec![start];
    let mut component = Vec::new();
    let mut pin_keys: HashMap<PinKey, PinEndpoint> = HashMap::new();
    let mut net = String::new();

    while let Some(i) = stack.pop() {
        if visited[i] {
            continue;
        }
        visited[i] = true;
        component.push(i);
        let seg = &segments[i];
        if net.is_empty() {
            net = seg.net.clone();
        }
        if let Some(p) = &seg.start_pin {
            pin_keys.insert(PinKey::from(p), p.clone());
        }
        if let Some(p) = &seg.end_pin {
            pin_keys.insert(PinKey::from(p), p.clone());
        }
        for j in 0..segments.len() {
            if visited[j] {
                continue;
            }
            if shares_endpoint(&segments[i], &segments[j]) {
                stack.push(j);
            }
        }
    }
    (component, pin_keys, net)
}

struct RerouteJob {
    remove: Vec<usize>,
    new_segs: Vec<WireSegment>,
}

/// Rebuild every wire chain that connects exactly two symbol pins as H/V segments only.
pub fn reroute_pin_connected_chains(segments: &mut Vec<WireSegment>, symbols: &[Sym]) {
    let n = segments.len();
    if n == 0 {
        return;
    }
    let mut visited = vec![false; n];
    let mut jobs: Vec<RerouteJob> = Vec::new();

    for start in 0..n {
        if visited[start] {
            continue;
        }
        let (component, pin_keys, net) = collect_component(start, segments, &mut visited);
        if pin_keys.len() != 2 || component.is_empty() {
            continue;
        }

        let mut pins: Vec<PinEndpoint> = pin_keys.into_values().collect();
        pins.sort_by(|a, b| {
            (a.ref_des.as_str(), a.pin_name.as_str())
                .cmp(&(b.ref_des.as_str(), b.pin_name.as_str()))
        });

        let Some(pa) = endpoint_world(symbols, &pins[0]) else {
            continue;
        };
        let Some(pb) = endpoint_world(symbols, &pins[1]) else {
            continue;
        };

        let net_id = segments
            .get(component[0])
            .map(|s| s.net_id)
            .unwrap_or_else(uuid::Uuid::new_v4);
        let mut new_segs = manhattan_segments(pa, pb, net.clone());
        if new_segs.is_empty() {
            continue;
        }
        for s in &mut new_segs {
            s.net_id = net_id;
        }
        if let Some(first) = new_segs.first_mut() {
            first.start_pin = Some(pins[0].clone());
        }
        if let Some(last) = new_segs.last_mut() {
            last.end_pin = Some(pins[1].clone());
        }

        jobs.push(RerouteJob {
            remove: component,
            new_segs,
        });
    }

    if jobs.is_empty() {
        return;
    }

    let mut remove_all: HashSet<usize> = HashSet::new();
    for job in &jobs {
        remove_all.extend(job.remove.iter().copied());
    }
    let mut kept: Vec<WireSegment> = segments
        .iter()
        .enumerate()
        .filter(|(i, _)| !remove_all.contains(i))
        .map(|(_, s)| s.clone())
        .collect();
    for job in jobs {
        kept.extend(job.new_segs);
    }
    *segments = kept;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::{pin_pitch_world, WireSegment};

    #[test]
    fn reroute_replaces_diagonal_two_pin_chain() {
        let pitch = pin_pitch_world();
        let mut symbols = vec![
            Sym {
                ref_des: "R1".into(),
                part_id: None,
                pos: Pos2::ZERO,
                rotation_deg: 0.0,
                pins: vec!["1".into()],
                footprint_ref: None,
                symbol_id: None,
                pin_layout: vec![("1".into(), 0.0, -pitch)],
                value: String::new(),
                fields: Default::default(),
            },
            Sym {
                ref_des: "D1".into(),
                part_id: None,
                pos: Pos2::new(120.0, 0.0),
                rotation_deg: 0.0,
                pins: vec!["1".into()],
                footprint_ref: None,
                symbol_id: None,
                pin_layout: vec![("1".into(), -pitch, 0.0)],
                value: String::new(),
                fields: Default::default(),
            },
        ];
        let pa = symbol_pin_world(&symbols[0], "1");
        let pb = symbol_pin_world(&symbols[1], "1");
        let corner = Pos2::new(pb.x, pa.y);
        let shared_net = uuid::Uuid::new_v4();
        let mut segments = vec![
            WireSegment {
                id: uuid::Uuid::new_v4(),
                start: pa,
                end: corner,
                net_id: shared_net,
                net: "N".into(),
                start_pin: Some(PinEndpoint {
                    ref_des: "R1".into(),
                    pin_name: "1".into(),
                }),
                end_pin: None,
            },
            WireSegment {
                id: uuid::Uuid::new_v4(),
                start: corner,
                end: pb,
                net_id: shared_net,
                net: "N".into(),
                start_pin: None,
                end_pin: Some(PinEndpoint {
                    ref_des: "D1".into(),
                    pin_name: "1".into(),
                }),
            },
        ];
        symbols[0].pos = Pos2::new(80.0, 0.0);
        crate::canvas::sync_anchored_wire_endpoints(&mut segments, &symbols);
        reroute_pin_connected_chains(&mut segments, &symbols);
        assert!(
            segments.len() >= 2,
            "expected Manhattan chain, got {}",
            segments.len()
        );
        for seg in &segments {
            let dx = (seg.start.x - seg.end.x).abs();
            let dy = (seg.start.y - seg.end.y).abs();
            assert!(
                dx < 0.01 || dy < 0.01,
                "segment must be horizontal or vertical"
            );
        }
    }
}
