//! Schematic editor state: geometry, selection, undo, and viewport.

use egui::{Pos2, Rect};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::canvas::{
    junction_at_wire_crossing, manhattan_segments, snap_world_pos, BusSegment, CanvasSnapshot,
    Junction, NetLabel, NoConnect, PinEndpoint, PowerSymbol, Sym, TextItem, Viewport, Wire,
    WireSegment, CANVAS_UNDO_CAP,
};

use super::geometry::paste_nudge;
use super::tools::CanvasTool;

/// In-memory clipboard for canvas copy/cut/paste.
#[derive(Clone, Default)]
pub struct CanvasClipboard {
    pub symbols: Vec<Sym>,
    pub wire_segments: Vec<WireSegment>,
}

/// Pending symbol placement from library / parts catalog.
#[derive(Clone)]
pub struct PlaceSymbolRequest {
    pub prefix: String,
    pub part_id: Option<Uuid>,
    pub symbol_id: Option<String>,
    pub pin_layout: Vec<(String, f32, f32)>,
    pub default_value: String,
}

pub struct SchematicEditor {
    pub viewport: Viewport,
    pub symbols: Vec<Sym>,
    pub wire_segments: Vec<WireSegment>,
    pub net_labels: Vec<NetLabel>,
    pub junctions: Vec<Junction>,
    pub no_connects: Vec<NoConnect>,
    pub power_symbols: Vec<PowerSymbol>,
    pub text_items: Vec<TextItem>,
    pub buses: Vec<BusSegment>,

    pub sheets: Vec<SheetInfo>,
    pub active_sheet_id: String,

    /// ERC markers rendered on canvas (from last save / run).
    pub erc_markers: Vec<ErcMarkerOnCanvas>,
    /// Last connectivity rebuild: pin → net id (for live ERC).
    pub connectivity_pin_net: HashMap<(String, String), Uuid>,

    pub selected_syms: HashSet<String>,
    pub selected_segments: HashSet<usize>,
    pub selected_net_label: Option<usize>,
    pub selected_junction: Option<usize>,
    pub selected_no_connect: Option<usize>,
    pub selected_power_symbol: Option<usize>,
    pub selected_text_item: Option<usize>,
    pub selected_bus: Option<usize>,
    pub selected_erc_marker: Option<usize>,

    /// Primary selection for inspector (last clicked).
    pub selected_sym: Option<String>,
    pub selected_segment: Option<usize>,

    pub dragging_sym: Option<String>,
    pub dragging_segment_endpoint: Option<(usize, bool)>,
    pub wire_drag_from: Option<PinEndpoint>,
    pub wire_chain_last: Option<Pos2>,
    /// `wire_segments.len()` when the current wire chain started (in-progress clicks).
    pub wire_chain_segment_start: Option<usize>,

    pub box_select_start: Option<Pos2>,
    pub box_select_current: Option<Pos2>,

    pub tool: CanvasTool,
    pub show_grid: bool,
    pub snap_enabled: bool,
    pub new_wire_net: String,
    pub label_kind: tokito::models::NetLabelKind,
    pub hovered_sym: Option<String>,
    pub pending_zoom_fit: bool,
    pub screen_rect: Option<Rect>,
    pub cursor_world: Option<Pos2>,
    pub erc_marker_index: Option<usize>,
    pub place_request: Option<PlaceSymbolRequest>,
    pub selection_filter: SelectionFilter,

    /// Set each frame when the schematic canvas panel has keyboard focus.
    pub canvas_has_focus: bool,
    pub clipboard: Option<CanvasClipboard>,

    undo: Vec<CanvasSnapshot>,
    redo: Vec<CanvasSnapshot>,
}

#[derive(Clone)]
pub struct SheetInfo {
    pub id: String,
    pub name: String,
}

#[derive(Clone)]
pub struct ErcMarkerOnCanvas {
    pub code: String,
    pub message: String,
    pub severity: String,
    pub position: Pos2,
    pub instance_ref: Option<String>,
    pub net_name: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionFilter {
    #[default]
    All,
    Symbols,
    Wires,
    Labels,
    Annotations,
}

impl SelectionFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Symbols => "Symbols",
            Self::Wires => "Wires",
            Self::Labels => "Labels",
            Self::Annotations => "Annotations",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::All => Self::Symbols,
            Self::Symbols => Self::Wires,
            Self::Wires => Self::Labels,
            Self::Labels => Self::Annotations,
            Self::Annotations => Self::All,
        }
    }

    pub fn allows_symbols(self) -> bool {
        matches!(self, Self::All | Self::Symbols)
    }

    pub fn allows_wires(self) -> bool {
        matches!(self, Self::All | Self::Wires)
    }

    pub fn allows_labels(self) -> bool {
        matches!(self, Self::All | Self::Labels)
    }

    pub fn allows_annotations(self) -> bool {
        matches!(self, Self::All | Self::Annotations)
    }
}

impl Default for SchematicEditor {
    fn default() -> Self {
        Self {
            viewport: Viewport {
                pan: egui::Vec2::new(40.0, 40.0),
                zoom: 1.0,
            },
            symbols: vec![],
            wire_segments: vec![],
            net_labels: vec![],
            junctions: vec![],
            no_connects: vec![],
            power_symbols: vec![],
            text_items: vec![],
            buses: vec![],
            sheets: vec![SheetInfo {
                id: tokito::models::DEFAULT_SHEET_ID.to_string(),
                name: "Root".to_string(),
            }],
            active_sheet_id: tokito::models::DEFAULT_SHEET_ID.to_string(),
            erc_markers: vec![],
            connectivity_pin_net: HashMap::new(),
            selected_syms: HashSet::new(),
            selected_segments: HashSet::new(),
            selected_net_label: None,
            selected_junction: None,
            selected_no_connect: None,
            selected_power_symbol: None,
            selected_text_item: None,
            selected_bus: None,
            selected_erc_marker: None,
            selected_sym: None,
            selected_segment: None,
            dragging_sym: None,
            dragging_segment_endpoint: None,
            wire_drag_from: None,
            wire_chain_last: None,
            wire_chain_segment_start: None,
            box_select_start: None,
            box_select_current: None,
            tool: CanvasTool::default(),
            show_grid: true,
            snap_enabled: true,
            new_wire_net: "NET".to_string(),
            label_kind: tokito::models::NetLabelKind::Local,
            hovered_sym: None,
            pending_zoom_fit: true,
            screen_rect: None,
            cursor_world: None,
            erc_marker_index: None,
            place_request: None,
            selection_filter: SelectionFilter::default(),
            canvas_has_focus: false,
            clipboard: None,
            undo: vec![],
            redo: vec![],
        }
    }
}

impl SchematicEditor {
    pub fn snapshot(&self) -> CanvasSnapshot {
        CanvasSnapshot {
            symbols: self.symbols.clone(),
            wire_segments: self.wire_segments.clone(),
            net_labels: self.net_labels.clone(),
            junctions: self.junctions.clone(),
            no_connects: self.no_connects.clone(),
            power_symbols: self.power_symbols.clone(),
            text_items: self.text_items.clone(),
            buses: self.buses.clone(),
        }
    }

    pub fn restore_snapshot(&mut self, snap: CanvasSnapshot) {
        self.symbols = snap.symbols;
        self.wire_segments = snap.wire_segments;
        self.net_labels = snap.net_labels;
        self.junctions = snap.junctions;
        self.no_connects = snap.no_connects;
        self.power_symbols = snap.power_symbols;
        self.text_items = snap.text_items;
        self.buses = snap.buses;
    }

    pub fn load_legacy_wires(&mut self, wires: Vec<Wire>) {
        self.wire_segments.clear();
        for w in wires {
            self.wire_segments
                .extend(crate::canvas::wire_to_segments(&w, &self.symbols));
        }
    }

    pub fn before_edit(&mut self) {
        self.undo.push(self.snapshot());
        while self.undo.len() > CANVAS_UNDO_CAP {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            let cur = self.snapshot();
            self.redo.push(cur);
            self.restore_snapshot(prev);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            let cur = self.snapshot();
            self.undo.push(cur);
            self.restore_snapshot(next);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_syms.clear();
        self.selected_segments.clear();
        self.selected_sym = None;
        self.selected_segment = None;
        self.selected_net_label = None;
        self.selected_junction = None;
        self.selected_no_connect = None;
        self.selected_power_symbol = None;
        self.selected_text_item = None;
        self.selected_bus = None;
        self.selected_erc_marker = None;
    }

    /// Marquee / Ctrl+A: select all schematic items on the active sheet.
    pub fn select_all(&mut self) {
        self.clear_selection();
        if self.selection_filter.allows_symbols() {
            for sym in &self.symbols {
                self.selected_syms.insert(sym.ref_des.clone());
            }
            self.selected_sym = self.symbols.first().map(|s| s.ref_des.clone());
        }
        if self.selection_filter.allows_wires() {
            for i in 0..self.wire_segments.len() {
                self.selected_segments.insert(i);
            }
            self.selected_segment = self.selected_segments.iter().copied().next();
        }
        if self.selection_filter.allows_labels() {
            if !self.net_labels.is_empty() {
                self.selected_net_label = Some(0);
            }
            if !self.power_symbols.is_empty() {
                self.selected_power_symbol = Some(0);
            }
        }
        if self.selection_filter.allows_annotations() {
            if !self.junctions.is_empty() {
                self.selected_junction = Some(0);
            }
            if !self.no_connects.is_empty() {
                self.selected_no_connect = Some(0);
            }
            if !self.text_items.is_empty() {
                self.selected_text_item = Some(0);
            }
        }
        if self.selection_filter.allows_wires() && !self.buses.is_empty() {
            self.selected_bus = Some(0);
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selection_count() > 0
    }

    pub fn copy_selection(&mut self) -> bool {
        let symbols: Vec<Sym> = self
            .symbols
            .iter()
            .filter(|s| self.selected_syms.contains(&s.ref_des))
            .cloned()
            .collect();
        let wire_segments: Vec<WireSegment> = self
            .wire_segments
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected_segments.contains(i))
            .map(|(_, s)| s.clone())
            .collect();
        if symbols.is_empty() && wire_segments.is_empty() {
            return false;
        }
        self.clipboard = Some(CanvasClipboard {
            symbols,
            wire_segments,
        });
        true
    }

    pub fn paste_clipboard(&mut self) -> usize {
        let Some(cb) = self.clipboard.clone() else {
            return 0;
        };
        if cb.symbols.is_empty() && cb.wire_segments.is_empty() {
            return 0;
        }
        self.before_edit();
        let nudge = paste_nudge();
        let mut ref_map: HashMap<String, String> = HashMap::new();
        let mut pasted = 0usize;

        for sym in cb.symbols {
            let prefix: String = sym
                .ref_des
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .collect();
            let prefix = if prefix.is_empty() {
                "U".to_string()
            } else {
                prefix
            };
            let new_ref = crate::util::next_refdes(&self.symbols, &prefix);
            ref_map.insert(sym.ref_des.clone(), new_ref.clone());
            let mut dup = sym;
            dup.ref_des = new_ref.clone();
            dup.pos = snap_world_pos(dup.pos + nudge);
            self.selected_syms.insert(new_ref.clone());
            self.symbols.push(dup);
            pasted += 1;
        }
        self.selected_sym = self
            .symbols
            .iter()
            .find(|s| self.selected_syms.contains(&s.ref_des))
            .map(|s| s.ref_des.clone());

        for mut seg in cb.wire_segments {
            seg.id = Uuid::new_v4();
            if let Some(ep) = seg.start_pin.as_mut() {
                if let Some(nr) = ref_map.get(&ep.ref_des) {
                    ep.ref_des = nr.clone();
                } else {
                    seg.start_pin = None;
                }
            }
            if let Some(ep) = seg.end_pin.as_mut() {
                if let Some(nr) = ref_map.get(&ep.ref_des) {
                    ep.ref_des = nr.clone();
                } else {
                    seg.end_pin = None;
                }
            }
            if seg.start_pin.is_none() {
                seg.start += nudge;
            }
            if seg.end_pin.is_none() {
                seg.end += nudge;
            }
            let idx = self.wire_segments.len();
            self.wire_segments.push(seg);
            self.selected_segments.insert(idx);
            pasted += 1;
        }
        self.refresh_wire_connectivity();
        pasted
    }

    pub fn selection_count(&self) -> usize {
        self.selected_syms.len()
            + self.selected_segments.len()
            + usize::from(self.selected_net_label.is_some())
            + usize::from(self.selected_junction.is_some())
            + usize::from(self.selected_no_connect.is_some())
            + usize::from(self.selected_power_symbol.is_some())
            + usize::from(self.selected_text_item.is_some())
            + usize::from(self.selected_bus.is_some())
            + usize::from(self.selected_erc_marker.is_some())
    }

    pub fn request_zoom_fit(&mut self) {
        self.pending_zoom_fit = true;
    }

    pub fn apply_zoom_fit_if_pending(&mut self, view_rect: Rect, origin: Pos2) {
        if !self.pending_zoom_fit {
            return;
        }
        self.pending_zoom_fit = false;
        self.viewport
            .fit_content(view_rect, origin, &self.snapshot());
    }

    pub fn snap_world(&self, p: Pos2) -> Pos2 {
        if self.snap_enabled {
            snap_world_pos(p)
        } else {
            p
        }
    }

    pub fn screen_center_world(&self) -> Option<Pos2> {
        let rect = self.screen_rect?;
        let origin = rect.min;
        let p = self.viewport.screen_to_world(origin, rect.center());
        Some(self.snap_world(p))
    }

    pub fn endpoint_world(&self, endpoint: &PinEndpoint) -> Option<Pos2> {
        let sym = self
            .symbols
            .iter()
            .find(|s| s.ref_des == endpoint.ref_des)?;
        Some(crate::canvas::symbol_pin_world(sym, &endpoint.pin_name))
    }

    /// Select every wire segment (and label) on the same electrical net as `segment_index`.
    pub fn select_net_for_segment(&mut self, segment_index: usize) {
        let Some(seg) = self.wire_segments.get(segment_index) else {
            return;
        };
        let net_id = seg.net_id;
        let net_name = seg.net.clone();
        self.clear_selection();
        for i in
            crate::editor::connectivity::segment_indices_for_net_id(net_id, &self.wire_segments)
        {
            self.selected_segments.insert(i);
        }
        self.selected_segment = Some(segment_index);
        for (i, label) in self.net_labels.iter().enumerate() {
            if label.name.trim() == net_name.trim() {
                self.selected_net_label = Some(i);
            }
        }
    }

    pub fn highlighted_net(&self) -> Option<String> {
        self.selected_segment
            .and_then(|i| self.wire_segments.get(i).map(|s| s.net.clone()))
            .or_else(|| {
                self.selected_net_label
                    .and_then(|i| self.net_labels.get(i).map(|l| l.name.clone()))
            })
    }

    pub fn inherited_wire_net(&self) -> String {
        if let Some(from) = &self.wire_drag_from {
            for seg in &self.wire_segments {
                if seg.start_pin.as_ref() == Some(from) || seg.end_pin.as_ref() == Some(from) {
                    return seg.net.clone();
                }
            }
        }
        if let Some(start) = self.wire_chain_segment_start {
            for seg in self.wire_segments[start..].iter() {
                if !seg.net.is_empty() {
                    return seg.net.clone();
                }
            }
        }
        let n = self.new_wire_net.trim();
        if n.is_empty() {
            "NET".to_string()
        } else {
            n.to_string()
        }
    }

    fn net_id_for_wire_route(&self, a: &PinEndpoint, b: &PinEndpoint) -> uuid::Uuid {
        self.wire_segments
            .iter()
            .find(|s| {
                s.start_pin.as_ref().is_some_and(|p| p == a)
                    || s.end_pin.as_ref().is_some_and(|p| p == a)
                    || s.start_pin.as_ref().is_some_and(|p| p == b)
                    || s.end_pin.as_ref().is_some_and(|p| p == b)
            })
            .map(|s| s.net_id)
            .unwrap_or_else(uuid::Uuid::new_v4)
    }

    pub fn push_wire_between(&mut self, a: PinEndpoint, b: PinEndpoint, net: String) {
        let (Some(pa), Some(pb)) = (self.endpoint_world(&a), self.endpoint_world(&b)) else {
            return;
        };
        let net_id = self.net_id_for_wire_route(&a, &b);
        self.remove_segments_between_pins(&a, &b, net_id);
        let mut segs = manhattan_segments(pa, pb, net.clone());
        for seg in &mut segs {
            seg.net_id = net_id;
            seg.net = net.clone();
        }
        if let Some(first) = segs.first_mut() {
            first.start_pin = Some(a.clone());
        }
        if let Some(last) = segs.last_mut() {
            last.end_pin = Some(b.clone());
        }
        self.push_wire_segments(segs);
    }

    /// Drop in-progress chain clicks and connect two pins with a clean Manhattan route.
    pub fn commit_wire_to_pin(&mut self, from: PinEndpoint, to: PinEndpoint, net: String) {
        self.discard_wire_chain();
        self.wire_chain_last = self.endpoint_world(&to);
        let continue_from = to.clone();
        self.push_wire_between(from, to, net);
        self.wire_drag_from = Some(continue_from);
        self.wire_chain_segment_start = Some(self.wire_segments.len());
    }

    pub fn start_wire_from_pin(&mut self, pin: PinEndpoint) {
        self.wire_drag_from = Some(pin.clone());
        self.wire_chain_last = self.endpoint_world(&pin);
        self.wire_chain_segment_start = Some(self.wire_segments.len());
    }

    pub fn discard_wire_chain(&mut self) {
        if let Some(start) = self.wire_chain_segment_start {
            if start < self.wire_segments.len() {
                self.wire_segments.truncate(start);
            }
        }
        self.wire_chain_segment_start = None;
        self.wire_chain_last = None;
    }

    pub fn cancel_wire_tool(&mut self) {
        self.discard_wire_chain();
        self.wire_drag_from = None;
    }

    pub fn push_segments_between(&mut self, start: Pos2, end: Pos2, net: String) {
        let net_id = self
            .wire_segments
            .last()
            .map(|s| s.net_id)
            .unwrap_or_else(uuid::Uuid::new_v4);
        let mut segs = manhattan_segments(start, end, net.clone());
        for seg in &mut segs {
            seg.net_id = net_id;
            seg.net = net.clone();
        }
        self.push_wire_segments(segs);
    }

    fn push_wire_segments(&mut self, segs: Vec<WireSegment>) {
        for seg in segs {
            self.maybe_add_junction(seg.start);
            self.maybe_add_junction(seg.end);
            self.wire_segments.push(seg);
        }
        self.sanitize_wires();
    }

    fn sanitize_wires(&mut self) {
        self.wire_segments = crate::canvas::orthogonalize_segments(&self.wire_segments);
    }

    fn remove_segments_between_pins(&mut self, a: &PinEndpoint, b: &PinEndpoint, net_id: Uuid) {
        self.wire_segments.retain(|seg| {
            if seg.net_id != net_id {
                return true;
            }
            let has_a = seg.start_pin.as_ref().is_some_and(|p| p == a)
                || seg.end_pin.as_ref().is_some_and(|p| p == a);
            let has_b = seg.start_pin.as_ref().is_some_and(|p| p == b)
                || seg.end_pin.as_ref().is_some_and(|p| p == b);
            !(has_a && has_b)
        });
    }

    /// Recompute world positions for wire endpoints attached to symbol pins.
    pub fn sync_anchored_wire_endpoints(&mut self) {
        crate::canvas::sync_anchored_wire_endpoints(&mut self.wire_segments, &self.symbols);
    }

    pub fn reroute_anchored_wires(&mut self) {
        super::wire_reroute::reroute_pin_connected_chains(&mut self.wire_segments, &self.symbols);
    }

    pub fn refresh_wire_connectivity(&mut self) {
        self.connectivity_pin_net = super::net_sync::refresh_connectivity(
            &self.symbols,
            &mut self.wire_segments,
            &mut self.junctions,
            &self.net_labels,
            &self.power_symbols,
            &self.no_connects,
            &self.buses,
        );
        super::live_erc::refresh_live_erc_markers(self);
    }

    /// After dragging a wire endpoint: snap to pin, orthogonalize, rebuild nets.
    pub fn finish_segment_endpoint_drag(&mut self, seg_idx: usize, is_end: bool) {
        if let Some(seg) = self.wire_segments.get_mut(seg_idx) {
            let world = if is_end { seg.end } else { seg.start };
            if let Some((pw, ep)) =
                super::live_erc::snap_segment_endpoint_to_pin(world, &self.symbols)
            {
                if is_end {
                    seg.end = pw;
                    seg.end_pin = Some(ep);
                } else {
                    seg.start = pw;
                    seg.start_pin = Some(ep);
                }
            }
        }
        super::net_sync::attach_orphan_endpoints_to_pins(&self.symbols, &mut self.wire_segments);
        self.wire_segments = crate::canvas::orthogonalize_segments(&self.wire_segments);
        self.refresh_wire_connectivity();
    }

    pub fn sync_wires_after_symbol_move(&mut self) {
        self.sync_anchored_wire_endpoints();
        self.sanitize_wires();
        self.reroute_anchored_wires();
        self.sanitize_wires();
    }

    /// Move wire ends and junctions attached to dragged symbols.
    pub fn push_wires_for_moved_symbols(
        &mut self,
        moved_refs: &std::collections::HashSet<String>,
        delta: egui::Vec2,
    ) {
        super::wire_push::push_wires_for_symbol_delta(
            &mut self.wire_segments,
            &mut self.junctions,
            moved_refs,
            delta,
        );
    }

    pub fn maybe_add_junction(&mut self, pos: Pos2) {
        if junction_at_wire_crossing(pos, &self.wire_segments, &self.junctions) {
            self.junctions.push(Junction { pos });
        }
    }

    pub fn delete_selected(&mut self) {
        if !self.selected_syms.is_empty() {
            self.before_edit();
            let refs: Vec<_> = self.selected_syms.iter().cloned().collect();
            for rd in &refs {
                self.symbols.retain(|s| s.ref_des != *rd);
            }
            let symbols = self.symbols.clone();
            let segments_snapshot = self.wire_segments.clone();
            self.wire_segments.retain(|seg| {
                !refs.iter().any(|r| {
                    endpoint_near_symbol_pin_static(seg.start, r, &symbols, &segments_snapshot)
                        || endpoint_near_symbol_pin_static(seg.end, r, &symbols, &segments_snapshot)
                })
            });
            self.clear_selection();
            return;
        }
        if !self.selected_segments.is_empty() {
            self.before_edit();
            let indices: HashSet<usize> = self.selected_segments.clone();
            self.wire_segments = self
                .wire_segments
                .iter()
                .enumerate()
                .filter(|(i, _)| !indices.contains(i))
                .map(|(_, s)| s.clone())
                .collect();
            self.clear_selection();
            return;
        }
        if let Some(i) = self.selected_net_label.take() {
            self.before_edit();
            if i < self.net_labels.len() {
                self.net_labels.remove(i);
            }
            return;
        }
        if let Some(i) = self.selected_junction.take() {
            self.before_edit();
            if i < self.junctions.len() {
                self.junctions.remove(i);
            }
            return;
        }
        if let Some(i) = self.selected_no_connect.take() {
            self.before_edit();
            if i < self.no_connects.len() {
                self.no_connects.remove(i);
            }
            return;
        }
        if let Some(i) = self.selected_power_symbol.take() {
            self.before_edit();
            if i < self.power_symbols.len() {
                self.power_symbols.remove(i);
            }
            return;
        }
        if let Some(i) = self.selected_text_item.take() {
            self.before_edit();
            if i < self.text_items.len() {
                self.text_items.remove(i);
            }
            return;
        }
        if let Some(i) = self.selected_bus.take() {
            self.before_edit();
            if i < self.buses.len() {
                self.buses.remove(i);
            }
        }
    }

    pub fn reset_view(&mut self) {
        self.viewport = Viewport {
            pan: egui::Vec2::new(40.0, 40.0),
            zoom: 1.0,
        };
        self.pending_zoom_fit = true;
    }

    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn rotate_selected_symbols(&mut self, delta_deg: f32) {
        if self.selected_syms.is_empty() {
            if let Some(rd) = self.selected_sym.clone() {
                self.selected_syms.insert(rd);
            } else {
                return;
            }
        }
        self.before_edit();
        for sym in &mut self.symbols {
            if self.selected_syms.contains(&sym.ref_des) {
                sym.rotation_deg = (sym.rotation_deg + delta_deg).rem_euclid(360.0);
            }
        }
        self.sync_wires_after_symbol_move();
        self.refresh_wire_connectivity();
    }

    pub fn mirror_selected_symbols_x(&mut self) {
        if self.selected_syms.is_empty() {
            return;
        }
        self.before_edit();
        for sym in &mut self.symbols {
            if self.selected_syms.contains(&sym.ref_des) {
                sym.rotation_deg = (360.0 - sym.rotation_deg).rem_euclid(360.0);
            }
        }
        self.sync_wires_after_symbol_move();
        self.refresh_wire_connectivity();
    }

    /// Clone selected symbols with new refdes, offset by `delta_world` (grid units).
    pub fn duplicate_selected_symbols(&mut self, delta_world: egui::Vec2) -> usize {
        let to_copy: Vec<crate::canvas::Sym> = self
            .symbols
            .iter()
            .filter(|s| self.selected_syms.contains(&s.ref_des))
            .cloned()
            .collect();
        if to_copy.is_empty() {
            return 0;
        }
        let mut new_refs = Vec::new();
        for sym in to_copy {
            let prefix: String = sym
                .ref_des
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .collect();
            let prefix = if prefix.is_empty() {
                "U".to_string()
            } else {
                prefix
            };
            let ref_des = crate::util::next_refdes(&self.symbols, &prefix);
            let mut dup = sym;
            dup.ref_des = ref_des.clone();
            dup.pos = crate::canvas::snap_world_pos(dup.pos + delta_world);
            new_refs.push(ref_des);
            self.symbols.push(dup);
        }
        self.clear_selection();
        for r in &new_refs {
            self.selected_syms.insert(r.clone());
        }
        self.selected_sym = new_refs.first().cloned();
        new_refs.len()
    }
}

fn endpoint_near_symbol_pin_static(
    pos: Pos2,
    ref_des: &str,
    symbols: &[Sym],
    segments: &[WireSegment],
) -> bool {
    let Some(sym) = symbols.iter().find(|s| s.ref_des == ref_des) else {
        return false;
    };
    for pin in crate::canvas::display_pins_for_symbol(sym, segments) {
        if crate::canvas::symbol_pin_world(sym, &pin).distance(pos) <= 18.0 {
            return true;
        }
    }
    false
}

pub type PartCache<'a> = &'a HashMap<Uuid, String>;
