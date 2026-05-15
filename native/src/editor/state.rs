//! Schematic editor state: geometry, selection, undo, and viewport.

use egui::{Pos2, Rect};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::canvas::{
    junction_at_wire_crossing, manhattan_segments, snap_world_pos, BusSegment, CanvasSnapshot,
    Junction, NetLabel, NoConnect, PinEndpoint, PowerSymbol, Sym, TextItem, Viewport, Wire,
    WireSegment, CANVAS_UNDO_CAP,
};

use super::tools::CanvasTool;

/// Pending symbol placement from library / parts catalog.
#[derive(Clone)]
pub struct PlaceSymbolRequest {
    pub prefix: String,
    pub part_id: Option<Uuid>,
    pub symbol_id: Option<String>,
    pub pin_layout: Vec<(String, f32, f32)>,
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

    /// Multi-sheet support (Milestone 6).
    pub sheets: Vec<SheetInfo>,
    pub active_sheet_id: String,

    /// ERC markers rendered on canvas (from last save / run).
    pub erc_markers: Vec<ErcMarkerOnCanvas>,

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

    /// Select every wire segment (and label) on the same net name as `segment_index`.
    pub fn select_net_for_segment(&mut self, segment_index: usize) {
        let Some(net) = self.wire_segments.get(segment_index).map(|s| s.net.clone()) else {
            return;
        };
        self.clear_selection();
        for i in crate::editor::connectivity::segment_indices_for_net(
            &net,
            &self.wire_segments,
            &self.net_labels,
        ) {
            self.selected_segments.insert(i);
        }
        self.selected_segment = Some(segment_index);
        for (i, label) in self.net_labels.iter().enumerate() {
            if label.name.trim() == net.trim() {
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

    pub fn push_wire_between(&mut self, a: PinEndpoint, b: PinEndpoint, net: String) {
        let (Some(pa), Some(pb)) = (self.endpoint_world(&a), self.endpoint_world(&b)) else {
            return;
        };
        self.push_segments_between(pa, pb, net);
    }

    pub fn push_segments_between(&mut self, start: Pos2, end: Pos2, net: String) {
        for seg in manhattan_segments(start, end, net.clone()) {
            self.maybe_add_junction(seg.start);
            self.maybe_add_junction(seg.end);
            self.wire_segments.push(seg);
        }
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
