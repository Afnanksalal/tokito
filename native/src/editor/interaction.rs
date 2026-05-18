//! Pointer/keyboard interaction for the schematic editor.

use egui::{Pos2, Rect, Response, Ui, Vec2};

use super::hit_test::{
    hover_at, pick_bus, pick_erc_marker, pick_junction, pick_net_label, pick_no_connect,
    pick_power_symbol, pick_text_item, pick_wire_segment, HoverState,
};
use super::state::SchematicEditor;
use super::tools::CanvasTool;
use super::wire_snap::{wire_snap_at, WireSnap};
use crate::canvas::{snap_symbol_pins_to_grid, symbol_hit_half_extents};
use crate::util;

pub struct InteractionResult {
    pub log: Option<String>,
}

impl InteractionResult {
    fn none() -> Self {
        Self { log: None }
    }
    fn msg(s: impl Into<String>) -> Self {
        Self {
            log: Some(s.into()),
        }
    }
}

pub fn handle(
    ui: &mut Ui,
    editor: &mut SchematicEditor,
    canvas_resp: &Response,
    rect: Rect,
    origin: Pos2,
) -> InteractionResult {
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let shift = ui.input(|i| i.modifiers.shift);
    let hover = pointer
        .map(|mp| {
            hover_at(
                mp,
                rect,
                origin,
                &editor.viewport,
                &editor.symbols,
                &editor.wire_segments,
            )
        })
        .unwrap_or(HoverState {
            symbol: None,
            pin: None,
        });
    editor.hovered_sym = hover.symbol;

    let mut clicked_any = false;
    let mut result = InteractionResult::none();

    // Marquee selection (Select tool, drag on empty canvas). Primary drag no longer pans — use
    // middle mouse or hold Space to pan (see editor::show).
    if matches!(editor.tool, CanvasTool::Select) {
        let box_active = editor.box_select_start.is_some();
        if canvas_resp.drag_started() {
            if let Some(mp) = pointer.filter(|p| rect.contains(*p)) {
                let on_symbol = symbol_hit(editor, mp, origin);
                let on_wire =
                    pick_wire_segment(mp, origin, &editor.viewport, &editor.wire_segments, 12.0)
                        .is_some();
                if !on_symbol && !on_wire {
                    editor.box_select_start =
                        Some(editor.snap_world(editor.viewport.screen_to_world(origin, mp)));
                    editor.box_select_current = editor.box_select_start;
                }
            }
        }
        if box_active || editor.box_select_start.is_some() {
            if let (Some(_), Some(mp)) = (editor.box_select_start, pointer) {
                editor.box_select_current =
                    Some(editor.snap_world(editor.viewport.screen_to_world(origin, mp)));
            }
        }
        if canvas_resp.drag_stopped() {
            if let (Some(a), Some(b)) = (editor.box_select_start, editor.box_select_current) {
                if a.distance(b) > 4.0 {
                    apply_box_select(editor, a, b, shift);
                    clicked_any = true;
                }
            }
            editor.box_select_start = None;
            editor.box_select_current = None;
        }
    }

    // Place symbol tool
    if matches!(editor.tool, CanvasTool::PlaceSymbol) {
        if canvas_resp.clicked() {
            if let Some(mp) = pointer.filter(|p| rect.contains(*p)) {
                let world = editor.snap_world(editor.viewport.screen_to_world(origin, mp));
                if let Some(req) = editor.place_request.clone() {
                    editor.before_edit();
                    let ref_des = util::next_refdes(&editor.symbols, &req.prefix);
                    let pin_names: Vec<String> = if req.pin_layout.is_empty() {
                        vec!["1".to_string(), "2".to_string()]
                    } else {
                        req.pin_layout.iter().map(|(n, _, _)| n.clone()).collect()
                    };
                    let mut sym = crate::canvas::Sym {
                        ref_des: ref_des.clone(),
                        part_id: req.part_id,
                        pos: world,
                        rotation_deg: 0.0,
                        pins: pin_names,
                        footprint_ref: None,
                        symbol_id: req.symbol_id.clone(),
                        pin_layout: req.pin_layout.clone(),
                        value: req.default_value.clone(),
                        fields: std::collections::BTreeMap::new(),
                    };
                    snap_symbol_pins_to_grid(&mut sym);
                    editor.symbols.push(sym);
                    editor.clear_selection();
                    editor.selected_syms.insert(ref_des);
                    editor.selected_sym = editor.selected_syms.iter().next().cloned();
                    clicked_any = true;
                    result = InteractionResult::msg("Placed symbol.");
                }
            }
        }
    }

    // Wire tool: pins, junctions, and wire endpoints only
    if matches!(editor.tool, CanvasTool::Wire) {
        if canvas_resp.clicked() {
            if let Some(mp) = pointer.filter(|p| rect.contains(*p)) {
                let net = editor.inherited_wire_net();
                if let Some(snap) = wire_snap_at(
                    mp,
                    origin,
                    &editor.viewport,
                    &editor.symbols,
                    &editor.wire_segments,
                    &editor.junctions,
                    &editor.wire_segments,
                ) {
                    clicked_any = true;
                    match snap {
                        WireSnap::Pin(pin) => {
                            if let Some(from) = editor.wire_drag_from.clone() {
                                if from != pin {
                                    editor.before_edit();
                                    editor.commit_wire_to_pin(from, pin, net);
                                    editor.refresh_wire_connectivity();
                                    result = InteractionResult::msg("Added wire.");
                                }
                            } else {
                                editor.before_edit();
                                editor.start_wire_from_pin(pin);
                                editor.clear_selection();
                            }
                        }
                        WireSnap::Junction(world) | WireSnap::WireEnd { world, .. } => {
                            if editor.wire_drag_from.is_some() {
                                let world = editor.snap_world(world);
                                editor.before_edit();
                                let start = editor
                                    .wire_chain_last
                                    .or_else(|| {
                                        editor
                                            .wire_drag_from
                                            .as_ref()
                                            .and_then(|p| editor.endpoint_world(p))
                                    })
                                    .unwrap_or(world);
                                editor.push_segments_between(start, world, net);
                                editor.wire_chain_last = Some(
                                    editor.wire_segments.last().map(|s| s.end).unwrap_or(world),
                                );
                                editor.refresh_wire_connectivity();
                            }
                        }
                    }
                }
            }
        }
    }

    // Symbol interactions (select, drag).
    for i in 0..editor.symbols.len() {
        let ref_des = editor.symbols[i].ref_des.clone();
        let p = editor
            .viewport
            .world_to_screen(origin, editor.symbols[i].pos);
        let z = editor.viewport.zoom;
        let (hx, hy) = symbol_hit_half_extents(&editor.symbols[i]);
        let size = Vec2::new(hx * 2.0 * z, hy * 2.0 * z);
        let r = Rect::from_center_size(p, size);
        let id = ui.id().with(("sym", ref_des.as_str()));

        let sense = match editor.tool {
            CanvasTool::Wire | CanvasTool::PlaceSymbol => egui::Sense::hover(),
            CanvasTool::Pan => egui::Sense::click(),
            _ => egui::Sense::click_and_drag(),
        };
        let sym_resp = ui.interact(r, id, sense);

        if matches!(editor.tool, CanvasTool::Select)
            && editor.selection_filter.allows_symbols()
            && sym_resp.clicked()
        {
            if !shift {
                editor.clear_selection();
            }
            editor.selected_syms.insert(ref_des.clone());
            editor.selected_sym = Some(ref_des.clone());
            editor.cancel_wire_tool();
            clicked_any = true;
        }

        if matches!(editor.tool, CanvasTool::Select) && sym_resp.drag_started() {
            editor.before_edit();
            if editor.selected_syms.contains(&ref_des) && editor.selected_syms.len() > 1 {
                editor.dragging_sym = None;
            } else {
                editor.dragging_sym = Some(ref_des.clone());
            }
        }
        if matches!(editor.tool, CanvasTool::Select) && sym_resp.dragged() {
            let delta = sym_resp.drag_delta() / editor.viewport.zoom;
            if editor.selected_syms.contains(&ref_des) && editor.selected_syms.len() > 1 {
                let refs: std::collections::HashSet<String> =
                    editor.selected_syms.iter().cloned().collect();
                for rd in &refs {
                    if let Some(idx) = editor.symbols.iter().position(|s| s.ref_des == *rd) {
                        editor.symbols[idx].pos += delta;
                    }
                }
                editor.push_wires_for_moved_symbols(&refs, delta);
                editor.sync_anchored_wire_endpoints();
            } else if editor.dragging_sym.as_deref() == Some(ref_des.as_str()) {
                editor.symbols[i].pos += delta;
                let mut refs = std::collections::HashSet::new();
                refs.insert(ref_des.clone());
                editor.push_wires_for_moved_symbols(&refs, delta);
                editor.sync_anchored_wire_endpoints();
            }
        }
        if sym_resp.drag_stopped() {
            if editor.selected_syms.contains(&ref_des) && editor.selected_syms.len() > 1 {
                let refs: Vec<String> = editor.selected_syms.iter().cloned().collect();
                for rd in refs {
                    if let Some(idx) = editor.symbols.iter().position(|s| s.ref_des == rd) {
                        let pos = editor.snap_world(editor.symbols[idx].pos);
                        editor.symbols[idx].pos = pos;
                    }
                }
            } else if editor.dragging_sym.as_deref() == Some(ref_des.as_str()) {
                editor.symbols[i].pos = editor.snap_world(editor.symbols[i].pos);
            }
            editor.dragging_sym = None;
            editor.sync_wires_after_symbol_move();
            editor.refresh_wire_connectivity();
        }
    }

    // Segment endpoint drag (select tool).
    if matches!(editor.tool, CanvasTool::Select) {
        if let Some(seg_idx) = editor.selected_segment {
            if let Some(seg) = editor.wire_segments.get(seg_idx).cloned() {
                for (is_end, pt) in [(false, seg.start), (true, seg.end)] {
                    let p = editor.viewport.world_to_screen(origin, pt);
                    let id = ui.id().with(("seg_ep", seg_idx, is_end));
                    let bend_rect = Rect::from_center_size(p, egui::vec2(14.0, 14.0));
                    let bend_resp = ui.interact(bend_rect, id, egui::Sense::click_and_drag());
                    if bend_resp.drag_started() {
                        editor.before_edit();
                        editor.dragging_segment_endpoint = Some((seg_idx, is_end));
                    }
                    if let Some((idx, end)) = editor.dragging_segment_endpoint {
                        if idx == seg_idx {
                            if let Some(mp) = pointer {
                                let world =
                                    editor.snap_world(editor.viewport.screen_to_world(origin, mp));
                                if let Some(s) = editor.wire_segments.get_mut(idx) {
                                    if end {
                                        s.end = world;
                                        s.end_pin = None;
                                    } else {
                                        s.start = world;
                                        s.start_pin = None;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if ui.input(|i| i.pointer.any_released()) {
            if let Some((idx, is_end)) = editor.dragging_segment_endpoint.take() {
                editor.finish_segment_endpoint_drag(idx, is_end);
            }
        }
    }

    // Placement tools on empty canvas click.
    if canvas_resp.clicked() && !clicked_any {
        if let Some(mp) = pointer.filter(|p| rect.contains(*p)) {
            let world = editor.snap_world(editor.viewport.screen_to_world(origin, mp));
            match editor.tool {
                CanvasTool::NetLabel => {
                    editor.before_edit();
                    let name = editor.new_wire_net.trim();
                    let rot = super::label_placement::wire_aligned_rotation(
                        world,
                        &editor.wire_segments,
                    );
                    editor.net_labels.push(crate::canvas::NetLabel {
                        name: if name.is_empty() {
                            "NET".to_string()
                        } else {
                            name.to_string()
                        },
                        pos: world,
                        rotation_deg: rot,
                        kind: editor.label_kind,
                    });
                    editor.clear_selection();
                    editor.selected_net_label = Some(editor.net_labels.len() - 1);
                    clicked_any = true;
                }
                CanvasTool::SheetPort => {
                    editor.before_edit();
                    let name = editor.new_wire_net.trim();
                    let name = if name.is_empty() {
                        format!("{}/NET", editor.active_sheet_id)
                    } else if name.contains('/') {
                        name.to_string()
                    } else {
                        format!("{}/{}", editor.active_sheet_id, name)
                    };
                    editor.net_labels.push(crate::canvas::NetLabel {
                        name,
                        pos: world,
                        kind: tokito::models::NetLabelKind::Hierarchical,
                        rotation_deg: 0.0,
                    });
                    editor.clear_selection();
                    editor.selected_net_label = Some(editor.net_labels.len() - 1);
                    clicked_any = true;
                }
                CanvasTool::Junction => {
                    editor.before_edit();
                    editor
                        .junctions
                        .push(crate::canvas::Junction { pos: world });
                    editor.clear_selection();
                    editor.selected_junction = Some(editor.junctions.len() - 1);
                    clicked_any = true;
                }
                CanvasTool::Power => {
                    editor.before_edit();
                    let name = editor.new_wire_net.trim();
                    editor.power_symbols.push(crate::canvas::PowerSymbol {
                        name: if name.is_empty() {
                            "VCC".to_string()
                        } else {
                            name.to_string()
                        },
                        pos: world,
                    });
                    editor.clear_selection();
                    editor.selected_power_symbol = Some(editor.power_symbols.len() - 1);
                    clicked_any = true;
                }
                CanvasTool::NoConnect => {
                    editor.before_edit();
                    editor
                        .no_connects
                        .push(crate::canvas::NoConnect { pos: world });
                    editor.clear_selection();
                    editor.selected_no_connect = Some(editor.no_connects.len() - 1);
                    clicked_any = true;
                }
                CanvasTool::Bus => {
                    editor.before_edit();
                    editor.buses.push(crate::canvas::BusSegment {
                        name: Some(editor.new_wire_net.trim().to_string())
                            .filter(|s| !s.is_empty()),
                        start: world,
                        end: world + Vec2::new(120.0, 0.0),
                    });
                    editor.clear_selection();
                    editor.selected_bus = Some(editor.buses.len() - 1);
                    clicked_any = true;
                }
                CanvasTool::Text => {
                    editor.before_edit();
                    editor.text_items.push(crate::canvas::TextItem {
                        text: "Note".to_string(),
                        pos: world,
                    });
                    editor.clear_selection();
                    editor.selected_text_item = Some(editor.text_items.len() - 1);
                    clicked_any = true;
                }
                _ => {}
            }
        }
    }

    // Pick wires and annotations.
    if canvas_resp.clicked() && !clicked_any {
        if let Some(mp) = pointer {
            let mut picked = false;
            if editor.selection_filter.allows_wires() {
                if let Some(si) =
                    pick_wire_segment(mp, origin, &editor.viewport, &editor.wire_segments, 12.0)
                {
                    if !shift {
                        editor.clear_selection();
                    }
                    editor.select_net_for_segment(si);
                    editor.cancel_wire_tool();
                    picked = true;
                }
            }
            if !picked && editor.selection_filter.allows_labels() {
                if let Some(i) = pick_net_label(mp, origin, &editor.viewport, &editor.net_labels) {
                    editor.clear_selection();
                    editor.selected_net_label = Some(i);
                    picked = true;
                }
            }
            if !picked && editor.selection_filter.allows_annotations() {
                if let Some(i) = pick_junction(mp, origin, &editor.viewport, &editor.junctions) {
                    editor.clear_selection();
                    editor.selected_junction = Some(i);
                    picked = true;
                }
            }
            if !picked && editor.selection_filter.allows_annotations() {
                if let Some(i) = pick_no_connect(mp, origin, &editor.viewport, &editor.no_connects)
                {
                    editor.clear_selection();
                    editor.selected_no_connect = Some(i);
                    picked = true;
                }
            }
            if !picked && editor.selection_filter.allows_labels() {
                if let Some(i) =
                    pick_power_symbol(mp, origin, &editor.viewport, &editor.power_symbols)
                {
                    editor.clear_selection();
                    editor.selected_power_symbol = Some(i);
                    picked = true;
                }
            }
            if !picked && editor.selection_filter.allows_wires() {
                if let Some(i) = pick_bus(mp, origin, &editor.viewport, &editor.buses) {
                    editor.clear_selection();
                    editor.selected_bus = Some(i);
                    picked = true;
                }
            }
            if !picked && editor.selection_filter.allows_annotations() {
                if let Some(i) = pick_text_item(mp, origin, &editor.viewport, &editor.text_items) {
                    editor.clear_selection();
                    editor.selected_text_item = Some(i);
                    picked = true;
                }
            }
            if !picked && editor.selection_filter.allows_annotations() {
                if let Some(i) = pick_erc_marker(mp, origin, &editor.viewport, &editor.erc_markers)
                {
                    editor.clear_selection();
                    editor.selected_erc_marker = Some(i);
                    editor.erc_marker_index = Some(i);
                    picked = true;
                }
            }
            if !picked && !shift {
                editor.clear_selection();
                if !matches!(editor.tool, CanvasTool::Wire) {
                    editor.cancel_wire_tool();
                }
            }
        }
    }

    result
}

fn symbol_hit(editor: &SchematicEditor, pointer: Pos2, origin: Pos2) -> bool {
    for sym in &editor.symbols {
        let center = editor.viewport.world_to_screen(origin, sym.pos);
        let z = editor.viewport.zoom;
        let (hx, hy) = symbol_hit_half_extents(sym);
        let size = Vec2::new(hx * 2.0 * z, hy * 2.0 * z);
        if Rect::from_center_size(center, size).contains(pointer) {
            return true;
        }
    }
    false
}

fn apply_box_select(editor: &mut SchematicEditor, a: Pos2, b: Pos2, add: bool) {
    use super::geometry::{segment_in_marquee, symbol_in_marquee};

    let r = Rect::from_two_pos(a, b);
    let enclosed = a.x <= b.x;
    if !add {
        editor.clear_selection();
    }
    if editor.selection_filter.allows_symbols() {
        for sym in &editor.symbols {
            if symbol_in_marquee(sym, r, enclosed) {
                editor.selected_syms.insert(sym.ref_des.clone());
                editor.selected_sym = Some(sym.ref_des.clone());
            }
        }
    }
    if editor.selection_filter.allows_wires() {
        for (i, seg) in editor.wire_segments.iter().enumerate() {
            if segment_in_marquee(seg, r, enclosed) {
                editor.selected_segments.insert(i);
                editor.selected_segment = Some(i);
            }
        }
        for (i, bus) in editor.buses.iter().enumerate() {
            let hit = if enclosed {
                r.contains(bus.start) && r.contains(bus.end)
            } else {
                segment_in_marquee(
                    &crate::canvas::WireSegment::new(bus.start, bus.end, ""),
                    r,
                    false,
                )
            };
            if hit {
                editor.selected_bus = Some(i);
            }
        }
    }
    if editor.selection_filter.allows_labels() {
        for (i, label) in editor.net_labels.iter().enumerate() {
            if r.contains(label.pos) {
                editor.selected_net_label = Some(i);
            }
        }
        for (i, pwr) in editor.power_symbols.iter().enumerate() {
            if r.contains(pwr.pos) {
                editor.selected_power_symbol = Some(i);
            }
        }
    }
    if editor.selection_filter.allows_annotations() {
        for (i, j) in editor.junctions.iter().enumerate() {
            if r.contains(j.pos) {
                editor.selected_junction = Some(i);
            }
        }
        for (i, nc) in editor.no_connects.iter().enumerate() {
            if r.contains(nc.pos) {
                editor.selected_no_connect = Some(i);
            }
        }
        for (i, t) in editor.text_items.iter().enumerate() {
            if r.contains(t.pos) {
                editor.selected_text_item = Some(i);
            }
        }
    }
}
