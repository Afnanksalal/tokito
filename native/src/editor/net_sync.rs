//! Live schematic connectivity: pin attachment + shared union-find graph.

use crate::canvas::{
    symbol_pin_world, Junction, NetLabel, NoConnect, PowerSymbol, Sym, WireSegment,
    PIN_ATTACH_RADIUS,
};
use egui::Pos2;
use tokito::connectivity::{
    rebuild_connectivity, ConnLabel, ConnPin, ConnPoint, ConnPower, ConnSegment, ConnectivityInput,
    LabelKind,
};
use tokito::models::NetLabelKind as DocLabelKind;

/// Snap wire endpoints to pins, sync anchors, rebuild topological nets.
pub fn refresh_connectivity(
    symbols: &[Sym],
    segments: &mut [WireSegment],
    junctions: &mut Vec<Junction>,
    net_labels: &[NetLabel],
    power_symbols: &[PowerSymbol],
    no_connects: &[NoConnect],
    buses: &[crate::canvas::BusSegment],
) -> std::collections::HashMap<(String, String), uuid::Uuid> {
    attach_orphan_endpoints_to_pins(symbols, segments);
    crate::canvas::sync_anchored_wire_endpoints(segments, symbols);
    super::junctions::ensure_junctions_at_crossings(segments, junctions);
    apply_connectivity_graph(
        symbols,
        segments,
        junctions,
        net_labels,
        power_symbols,
        no_connects,
        buses,
    )
}

/// Snap wire endpoints near symbol pins (used after segment drags).
pub fn attach_orphan_endpoints_to_pins(symbols: &[Sym], segments: &mut [WireSegment]) {
    for seg in segments.iter_mut() {
        if seg.start_pin.is_none() {
            if let Some(ep) = nearest_pin_endpoint(seg.start, symbols) {
                seg.start_pin = Some(ep.clone());
                if let Some(sym) = symbols.iter().find(|s| s.ref_des == ep.ref_des) {
                    seg.start = symbol_pin_world(sym, &ep.pin_name);
                }
            }
        }
        if seg.end_pin.is_none() {
            if let Some(ep) = nearest_pin_endpoint(seg.end, symbols) {
                seg.end_pin = Some(ep.clone());
                if let Some(sym) = symbols.iter().find(|s| s.ref_des == ep.ref_des) {
                    seg.end = symbol_pin_world(sym, &ep.pin_name);
                }
            }
        }
    }
}

fn nearest_pin_endpoint(pos: Pos2, symbols: &[Sym]) -> Option<crate::canvas::PinEndpoint> {
    let mut best: Option<(crate::canvas::PinEndpoint, f32)> = None;
    for sym in symbols {
        for (pin_name, _, _) in &sym.pin_layout {
            let pw = symbol_pin_world(sym, pin_name);
            let d = pw.distance(pos);
            if d <= PIN_ATTACH_RADIUS && best.as_ref().map(|(_, bd)| d < *bd).unwrap_or(true) {
                best = Some((
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
                let d = pw.distance(pos);
                if d <= PIN_ATTACH_RADIUS && best.as_ref().map(|(_, bd)| d < *bd).unwrap_or(true) {
                    best = Some((
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
    best.map(|(ep, _)| ep)
}

fn apply_connectivity_graph(
    symbols: &[Sym],
    segments: &mut [WireSegment],
    junctions: &[Junction],
    net_labels: &[NetLabel],
    power_symbols: &[PowerSymbol],
    no_connects: &[NoConnect],
    buses: &[crate::canvas::BusSegment],
) -> std::collections::HashMap<(String, String), uuid::Uuid> {
    let input = build_connectivity_input(
        symbols,
        segments,
        junctions,
        net_labels,
        power_symbols,
        no_connects,
        buses,
    );
    let result = rebuild_connectivity(&input);
    for (i, seg) in segments.iter_mut().enumerate() {
        if let Some((net_id, name)) = result.segment_nets.get(i) {
            seg.net_id = *net_id;
            seg.net = name.clone();
        }
    }
    result.pin_net
}

fn build_connectivity_input(
    symbols: &[Sym],
    segments: &[WireSegment],
    junctions: &[Junction],
    net_labels: &[NetLabel],
    power_symbols: &[PowerSymbol],
    no_connects: &[NoConnect],
    buses: &[crate::canvas::BusSegment],
) -> ConnectivityInput {
    let mut pins = Vec::new();
    for sym in symbols {
        for (pin_name, _, _) in &sym.pin_layout {
            let pw = symbol_pin_world(sym, pin_name);
            pins.push(ConnPin {
                ref_des: sym.ref_des.clone(),
                pin_name: pin_name.clone(),
                position: ConnPoint {
                    x: pw.x as f64,
                    y: pw.y as f64,
                },
            });
        }
    }

    let mut segments_in: Vec<ConnSegment> = segments
        .iter()
        .map(|s| ConnSegment {
            start: ConnPoint {
                x: s.start.x as f64,
                y: s.start.y as f64,
            },
            end: ConnPoint {
                x: s.end.x as f64,
                y: s.end.y as f64,
            },
            net_hint: (!s.net.trim().is_empty()).then(|| s.net.clone()),
        })
        .collect();
    for bus in buses {
        let hint = bus.name.as_deref().unwrap_or("").trim();
        let net_hint = if hint.is_empty() {
            None
        } else {
            Some(hint.to_string())
        };
        segments_in.push(ConnSegment {
            start: ConnPoint {
                x: bus.start.x as f64,
                y: bus.start.y as f64,
            },
            end: ConnPoint {
                x: bus.end.x as f64,
                y: bus.end.y as f64,
            },
            net_hint,
        });
    }

    ConnectivityInput {
        pins,
        segments: segments_in,
        junctions: junctions
            .iter()
            .map(|j| ConnPoint {
                x: j.pos.x as f64,
                y: j.pos.y as f64,
            })
            .collect(),
        labels: net_labels
            .iter()
            .map(|l| ConnLabel {
                position: ConnPoint {
                    x: l.pos.x as f64,
                    y: l.pos.y as f64,
                },
                name: l.name.clone(),
                kind: label_kind(l.kind),
            })
            .collect(),
        power: power_symbols
            .iter()
            .map(|p| ConnPower {
                position: ConnPoint {
                    x: p.pos.x as f64,
                    y: p.pos.y as f64,
                },
                name: p.name.clone(),
            })
            .collect(),
        no_connects: no_connects
            .iter()
            .map(|n| ConnPoint {
                x: n.pos.x as f64,
                y: n.pos.y as f64,
            })
            .collect(),
    }
}

fn label_kind(k: DocLabelKind) -> LabelKind {
    match k {
        DocLabelKind::Global => LabelKind::Global,
        DocLabelKind::Hierarchical => LabelKind::Hierarchical,
        DocLabelKind::Local | DocLabelKind::NetClassDirective => LabelKind::Local,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::WireSegment;
    use egui::Pos2;

    #[test]
    fn graph_unifies_touching_segments_by_net_id() {
        let symbols: Vec<Sym> = vec![];
        let mut segs = vec![
            WireSegment::new(Pos2::new(0.0, 0.0), Pos2::new(40.0, 0.0), "NET_A"),
            WireSegment::new(Pos2::new(40.0, 0.0), Pos2::new(80.0, 0.0), "NET_B"),
        ];
        refresh_connectivity(&symbols, &mut segs, &mut vec![], &[], &[], &[], &[]);
        assert_eq!(segs[0].net_id, segs[1].net_id);
        assert_eq!(segs[0].net, segs[1].net);
    }
}
