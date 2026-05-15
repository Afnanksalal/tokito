//! Pointer/keyboard interaction for the schematic editor.

use egui::{Pos2, Rect, Response, Ui, Vec2};

use super::hit_test::{
    hover_at, pick_bus, pick_erc_marker, pick_junction, pick_net_label, pick_no_connect,
    pick_pin_on_symbol, pick_power_symbol, pick_text_item, pick_wire_segment, HoverState,
    PIN_HIT_RADIUS,
};
use super::state::SchematicEditor;
use super::tools::CanvasTool;
use crate::canvas::{manhattan_segments, symbol_pin_world, PinEndpoint};
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

    // Box select (select tool, empty canvas drag).
    if matches!(editor.tool, CanvasTool::Select) {
        if canvas_resp.drag_started() {
            if let Some(mp) = pointer {
                if !symbol_hit(editor, mp, origin)
                    && pick_wire_segment(mp, origin, &editor.viewport, &editor.wire_segments, 12.0)
                        .is_none()
                {
                    editor.box_select_start =
                        Some(editor.snap_world(editor.viewport.screen_to_world(origin, mp)));
                    editor.box_select_current = editor.box_select_start;
                }
            }
        }
        if canvas_resp.dragged() {
            if let (Some(_), Some(mp)) = (editor.box_select_start, pointer) {
                editor.box_select_current =
                    Some(editor.snap_world(editor.viewport.screen_to_world(origin, mp)));
            }
        }
        if canvas_resp.drag_stopped() {
            if let (Some(a), Some(b)) = (editor.box_select_start, editor.box_select_current) {
                apply_box_select(editor, a, b, shift);
                clicked_any = true;
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
                    editor.symbols.push(crate::canvas::Sym {
                        ref_des: ref_des.clone(),
                        part_id: req.part_id,
                        pos: world,
                        rotation_deg: 0.0,
                        pins: pin_names,
                        footprint_ref: None,
                        symbol_id: req.symbol_id.clone(),
                        pin_layout: req.pin_layout.clone(),
                    });
                    editor.clear_selection();
                    editor.selected_syms.insert(ref_des);
                    editor.selected_sym = editor.selected_syms.iter().next().cloned();
                    clicked_any = true;
                    result = InteractionResult::msg("Placed symbol.");
                }
            }
        }
    }

    // Wire tool: pin-first, chain segments
    if matches!(editor.tool, CanvasTool::Wire) {
        if canvas_resp.clicked() {
            if let Some(mp) = pointer.filter(|p| rect.contains(*p)) {
                let net = editor.new_wire_net.trim().to_string();
                let net = if net.is_empty() {
                    "NET".to_string()
                } else {
                    net
                };
                if let Some(pin) = pick_pin_at(editor, mp, origin) {
                    clicked_any = true;
                    if let Some(from) = editor.wire_drag_from.take() {
                        if from != pin {
                            editor.before_edit();
                            editor.push_wire_between(from, pin, net);
                            editor.wire_chain_last = None;
                            result = InteractionResult::msg("Added wire.");
                        }
                    } else {
                        editor.wire_drag_from = Some(pin.clone());
                        editor.wire_chain_last = editor.endpoint_world(&pin);
                        editor.clear_selection();
                    }
                } else if editor.wire_drag_from.is_some() {
                    let world = editor.snap_world(editor.viewport.screen_to_world(origin, mp));
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
                    for seg in manhattan_segments(start, world, net.clone()) {
                        editor.maybe_add_junction(seg.start);
                        editor.maybe_add_junction(seg.end);
                        editor.wire_segments.push(seg);
                    }
                    editor.wire_chain_last = Some(world);
                    clicked_any = true;
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
        let size = Vec2::new(140.0 * editor.viewport.zoom, 62.0 * editor.viewport.zoom);
        let r = Rect::from_center_size(p, size);
        let id = ui.id().with(("sym", ref_des.as_str()));

        let sense = if matches!(editor.tool, CanvasTool::Pan) {
            egui::Sense::click()
        } else {
            egui::Sense::click_and_drag()
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
            editor.wire_drag_from = None;
            clicked_any = true;
        }

        if matches!(editor.tool, CanvasTool::Select) && sym_resp.drag_started() {
            editor.before_edit();
            editor.dragging_sym = Some(ref_des.clone());
        }
        if matches!(editor.tool, CanvasTool::Select)
            && sym_resp.dragged()
            && editor.dragging_sym.as_deref() == Some(ref_des.as_str())
        {
            let delta = sym_resp.drag_delta() / editor.viewport.zoom;
            editor.symbols[i].pos += delta;
        }
        if sym_resp.drag_stopped() {
            editor.dragging_sym = None;
            editor.symbols[i].pos = editor.snap_world(editor.symbols[i].pos);
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
                                    } else {
                                        s.start = world;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if ui.input(|i| i.pointer.any_released()) {
            editor.dragging_segment_endpoint = None;
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
                    editor.net_labels.push(crate::canvas::NetLabel {
                        name: if name.is_empty() {
                            "NET".to_string()
                        } else {
                            name.to_string()
                        },
                        pos: world,
                        kind: editor.label_kind,
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
                    editor.wire_drag_from = None;
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
                    editor.wire_drag_from = None;
                    editor.wire_chain_last = None;
                }
            }
        }
    }

    result
}

fn symbol_hit(editor: &SchematicEditor, pointer: Pos2, origin: Pos2) -> bool {
    for sym in &editor.symbols {
        let center = editor.viewport.world_to_screen(origin, sym.pos);
        let size = Vec2::new(140.0 * editor.viewport.zoom, 62.0 * editor.viewport.zoom);
        if Rect::from_center_size(center, size).contains(pointer) {
            return true;
        }
    }
    false
}

fn apply_box_select(editor: &mut SchematicEditor, a: Pos2, b: Pos2, add: bool) {
    let r = Rect::from_two_pos(a, b);
    let enclosed = a.x <= b.x;
    if !add {
        editor.clear_selection();
    }
    for sym in &editor.symbols {
        let hit = if enclosed {
            r.contains(sym.pos)
        } else {
            r.intersects(Rect::from_center_size(sym.pos, Vec2::splat(80.0)))
        };
        if hit {
            editor.selected_syms.insert(sym.ref_des.clone());
            editor.selected_sym = Some(sym.ref_des.clone());
        }
    }
    for (i, seg) in editor.wire_segments.iter().enumerate() {
        let hit = if enclosed {
            r.contains(seg.start) && r.contains(seg.end)
        } else {
            util::dist_point_to_segment_px(
                Pos2::new(r.center().x, r.center().y),
                seg.start,
                seg.end,
            ) < 80.0
                || r.contains(seg.start)
                || r.contains(seg.end)
        };
        if hit {
            editor.selected_segments.insert(i);
            editor.selected_segment = Some(i);
        }
    }
}

fn pick_pin_at(editor: &SchematicEditor, pointer: Pos2, origin: Pos2) -> Option<PinEndpoint> {
    let mut best: Option<(PinEndpoint, f32)> = None;
    for sym in &editor.symbols {
        if let Some(pin) = pick_pin_on_symbol(
            pointer,
            origin,
            &editor.viewport,
            sym,
            &editor.wire_segments,
        ) {
            let pin_world = symbol_pin_world(sym, &pin.pin_name);
            let d = editor
                .viewport
                .world_to_screen(origin, pin_world)
                .distance(pointer);
            if d <= PIN_HIT_RADIUS && best.as_ref().map(|(_, bd)| d < *bd).unwrap_or(true) {
                best = Some((pin, d));
            }
        }
    }
    best.map(|(p, _)| p)
}
