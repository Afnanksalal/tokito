//! Egui application state and Tokito integration (DB, copilot, schematic ops).

use anyhow::Context;
use eframe::egui;
use egui::{Pos2, Rect, Sense, Stroke, Vec2};
use egui_dock::{DockArea, Style as DockStyle};
use sqlx::postgres::PgPoolOptions;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Receiver;
use tokito::models::{
    DocumentBusSegment, DocumentJunction, DocumentNetLabel, DocumentNoConnect, DocumentPin,
    DocumentPoint, DocumentPowerSymbol, DocumentSymbol, DocumentTextItem, DocumentWireSegment,
    ElectricalPinType, ErcViolation, MirrorMode, NetLabelKind, PartSearchParams, PinOrientation,
    ReplaceSchematic, SchematicDocument,
};
use tokito::router::AppState;
use tokito::store::intent;
use uuid::Uuid;

use crate::bootstrap::ensure_local_user;
use crate::canvas::{
    display_pins_for_symbol, manhattan_bends, route_segments, snap_world_pos, symbol_pin_world,
    BusSegment, CanvasSnapshot, Junction, NetLabel, NoConnect, PinEndpoint, PowerSymbol, Sym,
    TextItem, Viewport, Wire, CANVAS_UNDO_CAP,
};
use crate::util::{guess_prefix, next_refdes};

type SchematicGenerationRx = Receiver<Result<(ReplaceSchematic, Vec<ErcViolation>), String>>;

pub mod studio_dock;

use studio_dock::StudioTab;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CanvasTool {
    #[default]
    Select,
    Wire,
    NetLabel,
    Power,
    Junction,
    NoConnect,
    Bus,
    Text,
    Pan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ProjectsSort {
    #[default]
    UpdatedDesc,
    UpdatedAsc,
    NameAsc,
    NameDesc,
}

#[derive(Clone)]
pub struct PartRow {
    pub id: Uuid,
    pub mpn: String,
    pub description: Option<String>,
}

#[derive(Clone, Copy)]
pub enum Route {
    Projects,
    Studio { design_id: Uuid },
}

pub struct App {
    rt: tokio::runtime::Runtime,
    pool: sqlx::PgPool,
    state: AppState,

    user_id: Uuid,

    route: Route,
    err: Option<String>,
    erc_note: Option<String>,

    designs: Vec<tokito::models::Design>,
    new_design_name: String,
    new_design_desc: String,

    design: Option<tokito::models::Design>,
    parts_query: String,
    parts_hits: Vec<PartRow>,
    part_cache: HashMap<Uuid, String>, // part_id -> mpn

    viewport: Viewport,
    symbols: Vec<Sym>,
    wires: Vec<Wire>,
    net_labels: Vec<NetLabel>,
    junctions: Vec<Junction>,
    no_connects: Vec<NoConnect>,
    power_symbols: Vec<PowerSymbol>,
    text_items: Vec<TextItem>,
    buses: Vec<BusSegment>,
    selected_sym: Option<String>,
    selected_wire: Option<usize>,
    selected_wire_bend: Option<usize>,
    selected_net_label: Option<usize>,
    selected_junction: Option<usize>,
    selected_no_connect: Option<usize>,
    selected_power_symbol: Option<usize>,
    selected_text_item: Option<usize>,
    selected_bus: Option<usize>,
    dragging_sym: Option<String>,
    dragging_wire_bend: Option<(usize, usize)>,
    wire_drag_from: Option<PinEndpoint>,
    /// Last canvas panel rect in screen space (for placing parts in view).
    canvas_screen_rect: Option<Rect>,

    prompt: String,
    prompt_busy: bool,

    canvas_undo: Vec<CanvasSnapshot>,
    canvas_redo: Vec<CanvasSnapshot>,

    /// Refresh project list when switching back from Studio (and once on startup).
    projects_need_refresh: bool,

    /// Background schematic generation (never block egui thread).
    generation_rx: Option<SchematicGenerationRx>,

    /// Dockable studio panels (`egui_dock`).
    dock_state: egui_dock::DockState<StudioTab>,

    /// Ring buffer of console / status lines for the Console tab.
    console_lines: Vec<String>,

    /// BOM cache for the BOM tab.
    bom_lines: Vec<tokito::models::BomLine>,
    bom_loaded_for: Option<Uuid>,
    bom_dirty: bool,

    /// Canvas tool mode (CAD toolbar).
    canvas_tool: CanvasTool,
    show_grid: bool,
    snap_enabled: bool,
    /// Default net name for new wires.
    new_wire_net: String,
    /// Symbol under pointer (hover highlight).
    canvas_hovered_sym: Option<String>,
    /// Zoom-to-fit after layout (Home / toolbar).
    pending_zoom_fit: bool,
    /// Focus mode hides dock chrome and gives the schematic canvas the workspace.
    canvas_focus_mode: bool,

    /// Projects launcher.
    projects_search: String,
    projects_sort: ProjectsSort,
    projects_pinned: HashSet<Uuid>,
    recent_design_ids: Vec<Uuid>,

    /// Optional external symbol provider (KiCad `.kicad_sym`).
    kicad_symbols: Option<crate::kicad_symbols::KicadSymbolLibrary>,
}

impl App {
    pub fn try_new() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "tokito=info,tower_http=warn".into()),
            )
            .init();

        let cfg = tokito::config::load()?;
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("tokio runtime")?;
        let pool = rt
            .block_on(async {
                PgPoolOptions::new()
                    .max_connections(cfg.db_max_connections)
                    .connect(&cfg.database_url)
                    .await
            })
            .context("connect database")?;
        rt.block_on(async { sqlx::migrate!("../migrations").run(&pool).await })
            .context("migrations")?;

        let state = AppState::try_new(pool.clone(), &cfg)?;

        let user_id = rt.block_on(async { ensure_local_user(&pool).await })?;

        let kicad_symbols = std::env::var("TOKITO_KICAD_SYM_LIB")
            .ok()
            .and_then(|p| crate::kicad_symbols::KicadSymbolLibrary::try_load(p).ok());

        Ok(Self {
            rt,
            pool,
            state,
            user_id,
            route: Route::Projects,
            err: None,
            erc_note: None,
            designs: vec![],
            new_design_name: "New design".to_string(),
            new_design_desc: "".to_string(),
            design: None,
            parts_query: "".to_string(),
            parts_hits: vec![],
            part_cache: HashMap::new(),
            viewport: Viewport {
                pan: egui::Vec2::new(40.0, 40.0),
                zoom: 1.0,
            },
            symbols: vec![],
            wires: vec![],
            net_labels: vec![],
            junctions: vec![],
            no_connects: vec![],
            power_symbols: vec![],
            text_items: vec![],
            buses: vec![],
            selected_sym: None,
            selected_wire: None,
            selected_wire_bend: None,
            selected_net_label: None,
            selected_junction: None,
            selected_no_connect: None,
            selected_power_symbol: None,
            selected_text_item: None,
            selected_bus: None,
            dragging_sym: None,
            dragging_wire_bend: None,
            wire_drag_from: None,
            canvas_screen_rect: None,
            prompt: "".to_string(),
            prompt_busy: false,
            canvas_undo: vec![],
            canvas_redo: vec![],
            projects_need_refresh: true,
            generation_rx: None,
            dock_state: egui_dock::DockState::new(StudioTab::ALL.into_iter().collect()),
            console_lines: vec![],
            bom_lines: vec![],
            bom_loaded_for: None,
            bom_dirty: true,
            canvas_tool: CanvasTool::default(),
            show_grid: true,
            snap_enabled: true,
            new_wire_net: "NET".to_string(),
            canvas_hovered_sym: None,
            pending_zoom_fit: false,
            canvas_focus_mode: false,
            projects_search: String::new(),
            projects_sort: ProjectsSort::default(),
            projects_pinned: HashSet::new(),
            recent_design_ids: vec![],
            kicad_symbols,
        })
    }

    fn snapshot_canvas(&self) -> CanvasSnapshot {
        CanvasSnapshot {
            symbols: self.symbols.clone(),
            wires: self.wires.clone(),
            net_labels: self.net_labels.clone(),
            junctions: self.junctions.clone(),
            no_connects: self.no_connects.clone(),
            power_symbols: self.power_symbols.clone(),
            text_items: self.text_items.clone(),
            buses: self.buses.clone(),
        }
    }

    fn before_canvas_edit(&mut self) {
        self.canvas_undo.push(self.snapshot_canvas());
        while self.canvas_undo.len() > CANVAS_UNDO_CAP {
            self.canvas_undo.remove(0);
        }
        self.canvas_redo.clear();
    }

    fn undo_canvas(&mut self) {
        if self.canvas_undo.is_empty() {
            return;
        }
        let prev = self.canvas_undo.pop().unwrap();
        let cur = self.snapshot_canvas();
        self.canvas_redo.push(cur);
        self.symbols = prev.symbols;
        self.wires = prev.wires;
        self.net_labels = prev.net_labels;
        self.junctions = prev.junctions;
        self.no_connects = prev.no_connects;
        self.power_symbols = prev.power_symbols;
        self.text_items = prev.text_items;
        self.buses = prev.buses;
    }

    fn redo_canvas(&mut self) {
        if self.canvas_redo.is_empty() {
            return;
        }
        let next = self.canvas_redo.pop().unwrap();
        let cur = self.snapshot_canvas();
        self.canvas_undo.push(cur);
        self.symbols = next.symbols;
        self.wires = next.wires;
        self.net_labels = next.net_labels;
        self.junctions = next.junctions;
        self.no_connects = next.no_connects;
        self.power_symbols = next.power_symbols;
        self.text_items = next.text_items;
        self.buses = next.buses;
    }

    fn clear_canvas_selection(&mut self) {
        self.selected_sym = None;
        self.selected_wire = None;
        self.selected_wire_bend = None;
        self.selected_net_label = None;
        self.selected_junction = None;
        self.selected_no_connect = None;
        self.selected_power_symbol = None;
        self.selected_text_item = None;
        self.selected_bus = None;
    }

    fn apply_document_to_canvas(&mut self, doc: SchematicDocument) {
        self.symbols = doc
            .symbols
            .iter()
            .map(|s| Sym {
                ref_des: s.ref_des.clone(),
                part_id: s.part_id,
                pos: snap_world_pos(Pos2::new(s.position.x as f32, s.position.y as f32)),
                rotation_deg: s.rotation as f32,
                pins: s.pins.iter().map(|p| p.name.clone()).collect(),
            })
            .collect();

        let mut symbols_by_id: HashMap<Uuid, String> = HashMap::new();
        for symbol in &doc.symbols {
            symbols_by_id.insert(symbol.id, symbol.ref_des.clone());
        }

        self.wires = doc
            .wire_segments
            .chunks(2)
            .enumerate()
            .filter_map(|(idx, chunk)| {
                let start = chunk.first()?.start;
                let end = chunk.last()?.end;
                let a =
                    self.nearest_symbol_pin_to_world(Pos2::new(start.x as f32, start.y as f32))?;
                let b = self.nearest_symbol_pin_to_world(Pos2::new(end.x as f32, end.y as f32))?;
                if a == b {
                    return None;
                }
                Some(Wire {
                    a: a.ref_des,
                    a_pin: a.pin_name,
                    b: b.ref_des,
                    b_pin: b.pin_name,
                    net: chunk
                        .first()
                        .and_then(|s| s.net_name.clone())
                        .unwrap_or_else(|| format!("N${}", idx + 1)),
                    bends: chunk
                        .iter()
                        .map(|s| Pos2::new(s.end.x as f32, s.end.y as f32))
                        .take(chunk.len().saturating_sub(1))
                        .collect(),
                })
            })
            .collect();

        self.net_labels = doc
            .net_labels
            .into_iter()
            .map(|l| NetLabel {
                name: l.name,
                pos: Pos2::new(l.position.x as f32, l.position.y as f32),
            })
            .collect();
        self.junctions = doc
            .junctions
            .into_iter()
            .map(|j| Junction {
                pos: Pos2::new(j.position.x as f32, j.position.y as f32),
            })
            .collect();
        self.no_connects = doc
            .no_connects
            .into_iter()
            .map(|n| NoConnect {
                pos: Pos2::new(n.position.x as f32, n.position.y as f32),
            })
            .collect();
        self.power_symbols = doc
            .power_symbols
            .into_iter()
            .map(|p| PowerSymbol {
                name: p.name,
                pos: Pos2::new(p.position.x as f32, p.position.y as f32),
            })
            .collect();
        self.text_items = doc
            .text_items
            .into_iter()
            .map(|t| TextItem {
                text: t.text,
                pos: Pos2::new(t.position.x as f32, t.position.y as f32),
            })
            .collect();
        self.buses = doc
            .buses
            .into_iter()
            .map(|b| BusSegment {
                name: b.name,
                start: Pos2::new(b.start.x as f32, b.start.y as f32),
                end: Pos2::new(b.end.x as f32, b.end.y as f32),
            })
            .collect();
    }

    fn nearest_symbol_pin_to_world(&self, world: Pos2) -> Option<PinEndpoint> {
        let mut best: Option<(PinEndpoint, f32)> = None;
        for sym in &self.symbols {
            for pin_name in display_pins_for_symbol(sym, &self.wires) {
                let pin_world = symbol_pin_world(sym, &pin_name);
                let dist = pin_world.distance(world);
                if dist <= 18.0 && best.as_ref().map(|(_, d)| dist < *d).unwrap_or(true) {
                    best = Some((
                        PinEndpoint {
                            ref_des: sym.ref_des.clone(),
                            pin_name,
                        },
                        dist,
                    ));
                }
            }
        }
        best.map(|(pin, _)| pin)
    }

    fn endpoint_world(&self, endpoint: &PinEndpoint) -> Option<Pos2> {
        let sym = self
            .symbols
            .iter()
            .find(|s| s.ref_des == endpoint.ref_des)?;
        Some(symbol_pin_world(sym, &endpoint.pin_name))
    }

    fn graph_to_document(&self) -> SchematicDocument {
        let mut doc = SchematicDocument::empty();
        doc.symbols = self
            .symbols
            .iter()
            .map(|s| {
                let pin_names = display_pins_for_symbol(s, &self.wires);
                DocumentSymbol {
                    id: Uuid::new_v4(),
                    sheet_id: tokito::models::DEFAULT_SHEET_ID.to_string(),
                    part_id: s.part_id,
                    symbol_id: Some("tokito:generic".to_string()),
                    ref_des: s.ref_des.clone(),
                    value: self
                        .part_cache
                        .get(&s.part_id.unwrap_or_else(Uuid::nil))
                        .cloned(),
                    position: DocumentPoint {
                        x: s.pos.x as f64,
                        y: s.pos.y as f64,
                    },
                    rotation: s.rotation_deg as f64,
                    mirror: MirrorMode::None,
                    fields: Default::default(),
                    footprint_ref: None,
                    pins: pin_names
                        .into_iter()
                        .map(|pin_name| {
                            let right_side = matches!(
                                pin_name.trim().to_ascii_lowercase().as_str(),
                                "2" | "b" | "out" | "vout" | "sda" | "scl" | "tx" | "miso"
                            ) || pin_name.ends_with("_b");
                            DocumentPin {
                                number: Some(pin_name.clone()),
                                name: pin_name,
                                electrical_type: ElectricalPinType::Unspecified,
                                offset: DocumentPoint {
                                    x: if right_side { 70.0 } else { -70.0 },
                                    y: 0.0,
                                },
                                orientation: if right_side {
                                    PinOrientation::Right
                                } else {
                                    PinOrientation::Left
                                },
                                visible: true,
                            }
                        })
                        .collect(),
                }
            })
            .collect();

        for w in &self.wires {
            let route = match (
                self.endpoint_world(&PinEndpoint {
                    ref_des: w.a.clone(),
                    pin_name: w.a_pin.clone(),
                }),
                self.endpoint_world(&PinEndpoint {
                    ref_des: w.b.clone(),
                    pin_name: w.b_pin.clone(),
                }),
            ) {
                (Some(a), Some(b)) => route_segments(a, &w.bends, b),
                _ => vec![],
            };
            for (start, end) in route {
                doc.wire_segments.push(DocumentWireSegment {
                    id: Uuid::new_v4(),
                    sheet_id: tokito::models::DEFAULT_SHEET_ID.to_string(),
                    start: DocumentPoint {
                        x: start.x as f64,
                        y: start.y as f64,
                    },
                    end: DocumentPoint {
                        x: end.x as f64,
                        y: end.y as f64,
                    },
                    net_name: Some(w.net.clone()),
                });
            }
        }

        doc.net_labels = self
            .net_labels
            .iter()
            .map(|l| DocumentNetLabel {
                id: Uuid::new_v4(),
                sheet_id: tokito::models::DEFAULT_SHEET_ID.to_string(),
                name: l.name.clone(),
                kind: NetLabelKind::Local,
                position: DocumentPoint {
                    x: l.pos.x as f64,
                    y: l.pos.y as f64,
                },
                orientation: PinOrientation::Right,
            })
            .collect();
        doc.junctions = self
            .junctions
            .iter()
            .map(|j| DocumentJunction {
                id: Uuid::new_v4(),
                sheet_id: tokito::models::DEFAULT_SHEET_ID.to_string(),
                position: DocumentPoint {
                    x: j.pos.x as f64,
                    y: j.pos.y as f64,
                },
            })
            .collect();
        doc.no_connects = self
            .no_connects
            .iter()
            .map(|n| DocumentNoConnect {
                id: Uuid::new_v4(),
                sheet_id: tokito::models::DEFAULT_SHEET_ID.to_string(),
                position: DocumentPoint {
                    x: n.pos.x as f64,
                    y: n.pos.y as f64,
                },
            })
            .collect();
        doc.power_symbols = self
            .power_symbols
            .iter()
            .map(|p| DocumentPowerSymbol {
                id: Uuid::new_v4(),
                sheet_id: tokito::models::DEFAULT_SHEET_ID.to_string(),
                name: p.name.clone(),
                position: DocumentPoint {
                    x: p.pos.x as f64,
                    y: p.pos.y as f64,
                },
            })
            .collect();
        doc.text_items = self
            .text_items
            .iter()
            .map(|t| DocumentTextItem {
                id: Uuid::new_v4(),
                sheet_id: tokito::models::DEFAULT_SHEET_ID.to_string(),
                text: t.text.clone(),
                position: DocumentPoint {
                    x: t.pos.x as f64,
                    y: t.pos.y as f64,
                },
                rotation: 0.0,
            })
            .collect();
        doc.buses = self
            .buses
            .iter()
            .map(|b| DocumentBusSegment {
                id: Uuid::new_v4(),
                sheet_id: tokito::models::DEFAULT_SHEET_ID.to_string(),
                start: DocumentPoint {
                    x: b.start.x as f64,
                    y: b.start.y as f64,
                },
                end: DocumentPoint {
                    x: b.end.x as f64,
                    y: b.end.y as f64,
                },
                name: b.name.clone(),
            })
            .collect();
        doc
    }

    fn load_prompt_after_open(&mut self, design_id: Uuid) {
        self.canvas_undo.clear();
        self.canvas_redo.clear();
        let pool = self.pool.clone();
        let res = self
            .rt
            .block_on(async move { intent::get(&pool, design_id).await });
        match res {
            Ok(Some(row)) => {
                self.prompt = row.goal_text;
                self.err = None;
            }
            Ok(None) => {
                self.prompt.clear();
                self.err = None;
            }
            Err(e) => self.set_err(e),
        }
    }

    fn log_console(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        self.console_lines.push(msg);
        const MAX: usize = 250;
        if self.console_lines.len() > MAX {
            let drain = self.console_lines.len() - MAX;
            self.console_lines.drain(0..drain);
        }
    }

    fn set_err(&mut self, e: impl std::fmt::Display) {
        let s = e.to_string();
        self.log_console(format!("[error] {s}"));
        self.err = Some(s);
    }

    fn push_recent_design(&mut self, id: Uuid) {
        self.recent_design_ids.retain(|x| *x != id);
        self.recent_design_ids.insert(0, id);
        self.recent_design_ids.truncate(24);
    }

    fn refresh_bom(&mut self, design_id: Uuid) {
        let res = self
            .rt
            .block_on(async { tokito::store::bom::list_for_design(&self.pool, design_id).await });
        match res {
            Ok(lines) => {
                let missing: Vec<Uuid> = lines
                    .iter()
                    .map(|l| l.part_id)
                    .filter(|id| !self.part_cache.contains_key(id))
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect();
                if !missing.is_empty() {
                    let map_res = self.rt.block_on(async {
                        tokito::store::parts::get_by_ids(&self.pool, &missing).await
                    });
                    if let Ok(map) = map_res {
                        for (id, p) in map {
                            self.part_cache.insert(id, p.mpn);
                        }
                    }
                }
                self.bom_lines = lines;
                self.bom_loaded_for = Some(design_id);
                self.bom_dirty = false;
            }
            Err(e) => self.set_err(e),
        }
    }

    fn set_erc_note_from_slice(&mut self, w: &[tokito::models::ErcViolation]) {
        if w.is_empty() {
            self.erc_note = None;
            return;
        }
        let head: Vec<String> = w
            .iter()
            .take(4)
            .map(|v| format!("{}: {}", v.code, v.message))
            .collect();
        let mut s = format!("ERC advisory ({}): {}", w.len(), head.join(" · "));
        if w.len() > 4 {
            s.push_str(&format!(" (+{} more)", w.len() - 4));
        }
        self.erc_note = Some(s);
    }

    fn reload_projects(&mut self) {
        self.err = None;
        let user_id = self.user_id;
        let res = self.rt.block_on(async {
            tokito::store::designs::list_for_user(&self.pool, user_id, 100).await
        });
        match res {
            Ok(rows) => self.designs = rows,
            Err(e) => self.set_err(e),
        }
    }

    fn open_design(&mut self, design_id: Uuid) {
        self.err = None;
        let user_id = self.user_id;
        let res = self.rt.block_on(async {
            tokito::store::designs::assert_visible(&self.pool, design_id, user_id).await
        });
        let design = match res {
            Ok(d) => d,
            Err(e) => {
                self.set_err(e);
                return;
            }
        };

        let sch = self
            .rt
            .block_on(async { tokito::store::schematic::get_view(&self.pool, design_id).await });
        let stored_doc = self
            .rt
            .block_on(async { tokito::store::schematic_document::get(&self.pool, design_id).await })
            .ok()
            .flatten();

        match sch {
            Ok(sch) => {
                self.design = Some(design);
                if let Some(doc) = stored_doc {
                    self.apply_document_to_canvas(doc);
                } else {
                    self.net_labels.clear();
                    self.junctions.clear();
                    self.no_connects.clear();
                    self.power_symbols.clear();
                    self.text_items.clear();
                    self.buses.clear();
                    self.symbols = sch
                        .instances
                        .iter()
                        .map(|i| Sym {
                            ref_des: i.ref_des.clone(),
                            part_id: i.part_id,
                            pos: snap_world_pos(Pos2::new(
                                i.pos_x.unwrap_or(120.0) as f32,
                                i.pos_y.unwrap_or(120.0) as f32,
                            )),
                            rotation_deg: i.rotation as f32,
                            pins: vec!["1".to_string(), "2".to_string()],
                        })
                        .collect();

                    let net_id_to_name: HashMap<Uuid, String> =
                        sch.nets.iter().map(|n| (n.id, n.name.clone())).collect();
                    let inst_id_to_ref: HashMap<Uuid, String> = sch
                        .instances
                        .iter()
                        .map(|i| (i.id, i.ref_des.clone()))
                        .collect();
                    let mut by_net: HashMap<Uuid, Vec<(Uuid, String)>> = HashMap::new();
                    for p in sch.pins {
                        by_net
                            .entry(p.net_id)
                            .or_default()
                            .push((p.instance_id, p.pin_name));
                    }
                    let mut wires = vec![];
                    for (net_id, inst_pins) in by_net {
                        let net = net_id_to_name
                            .get(&net_id)
                            .cloned()
                            .unwrap_or_else(|| "NET".into());
                        let mut uniq: Vec<(Uuid, String)> = vec![];
                        for pair in inst_pins {
                            if !uniq.iter().any(|(id, _)| *id == pair.0) {
                                uniq.push(pair);
                            }
                        }
                        for w in uniq.windows(2) {
                            if let (Some(a), Some(b)) =
                                (inst_id_to_ref.get(&w[0].0), inst_id_to_ref.get(&w[1].0))
                            {
                                let a_pin = w[0].1.clone();
                                let b_pin = w[1].1.clone();
                                let a_sym = self.symbols.iter().find(|s| s.ref_des == *a);
                                let b_sym = self.symbols.iter().find(|s| s.ref_des == *b);
                                let bends = match (a_sym, b_sym) {
                                    (Some(sa), Some(sb)) => manhattan_bends(
                                        symbol_pin_world(sa, &a_pin),
                                        symbol_pin_world(sb, &b_pin),
                                    ),
                                    _ => vec![],
                                };
                                wires.push(Wire {
                                    a: a.clone(),
                                    a_pin,
                                    b: b.clone(),
                                    b_pin,
                                    net: net.clone(),
                                    bends,
                                });
                            }
                        }
                    }
                    self.wires = wires;
                    self.net_labels.clear();
                    self.junctions.clear();
                    self.no_connects.clear();
                }

                self.clear_canvas_selection();
                self.viewport = Viewport {
                    pan: egui::Vec2::new(40.0, 40.0),
                    zoom: 1.0,
                };
                self.load_prompt_after_open(design_id);
                self.push_recent_design(design_id);
                self.bom_dirty = true;
                self.log_console(format!(
                    "Opened schematic · {}",
                    self.design
                        .as_ref()
                        .map(|d| d.name.as_str())
                        .unwrap_or("design")
                ));
                self.route = Route::Studio { design_id };
            }
            Err(e) => self.set_err(e),
        }
    }

    fn save_schematic(&mut self, design_id: Uuid) {
        let document = self.graph_to_document();
        let (body, document_diagnostics) = document.to_replace_schematic();
        let warns = tokito::services::schematic_validate::erc_light(&body);
        let res = self.rt.block_on(async {
            tokito::store::schematic::replace(&self.pool, design_id, body).await?;
            tokito::store::schematic_document::upsert(&self.pool, design_id, &document).await
        });
        match res {
            Ok(()) => {
                self.err = None;
                self.set_erc_note_from_slice(&warns);
                for diagnostic in document_diagnostics {
                    self.log_console(format!(
                        "[document] {}: {}",
                        diagnostic.code, diagnostic.message
                    ));
                }
                self.log_console("Saved schematic to board.".to_string());
            }
            Err(e) => {
                self.erc_note = None;
                self.set_err(e);
            }
        }
    }

    pub(crate) fn poll_async_jobs(&mut self, ctx: &egui::Context) {
        if let Some(rx) = self.generation_rx.take() {
            match rx.try_recv() {
                Ok(Ok((draft, erc))) => {
                    self.prompt_busy = false;
                    self.generation_rx = None;
                    self.apply_generated_schematic(draft, &erc);
                }
                Ok(Err(msg)) => {
                    self.prompt_busy = false;
                    self.generation_rx = None;
                    let mut m = msg;
                    if m.contains("API_KEY") || m.contains("api key") || m.contains("xAI") {
                        m.push_str(" — set TOKITO_XAI_API_KEY in .env and restart.");
                    }
                    if m.contains("Firecrawl") || m.contains("firecrawl") {
                        m.push_str(" — set TOKITO_FIRECRAWL_API_KEY in .env and restart.");
                    }
                    self.erc_note = None;
                    self.set_err(m);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.generation_rx = Some(rx);
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.prompt_busy = false;
                    self.generation_rx = None;
                    self.set_err("Schematic generation stopped unexpectedly.");
                }
            }
        }

        if self.prompt_busy {
            ctx.request_repaint();
        }
    }

    fn apply_generated_schematic(
        &mut self,
        draft: ReplaceSchematic,
        erc_warnings: &[ErcViolation],
    ) {
        self.before_canvas_edit();
        self.symbols = draft
            .instances
            .iter()
            .map(|i| Sym {
                ref_des: i.ref_des.clone(),
                part_id: i.part_id,
                pos: snap_world_pos(Pos2::new(
                    i.position.as_ref().map(|p| p.x).unwrap_or(120.0) as f32,
                    i.position.as_ref().map(|p| p.y).unwrap_or(120.0) as f32,
                )),
                rotation_deg: i.rotation as f32,
                pins: draft
                    .pins
                    .iter()
                    .filter(|p| p.instance_ref == i.ref_des)
                    .map(|p| p.pin_name.clone())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect(),
            })
            .collect();
        let mut by_net: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for p in draft.pins {
            by_net
                .entry(p.net_name)
                .or_default()
                .push((p.instance_ref, p.pin_name));
        }
        let mut wires = vec![];
        for (net, refs_pins) in by_net {
            let mut uniq = vec![];
            for pair in refs_pins {
                if !uniq.iter().any(|(r, _): &(String, String)| r == &pair.0) {
                    uniq.push(pair);
                }
            }
            for w in uniq.windows(2) {
                let a_sym = self.symbols.iter().find(|s| s.ref_des == w[0].0);
                let b_sym = self.symbols.iter().find(|s| s.ref_des == w[1].0);
                let bends = match (a_sym, b_sym) {
                    (Some(sa), Some(sb)) => manhattan_bends(
                        symbol_pin_world(sa, &w[0].1),
                        symbol_pin_world(sb, &w[1].1),
                    ),
                    _ => vec![],
                };
                wires.push(Wire {
                    a: w[0].0.clone(),
                    a_pin: w[0].1.clone(),
                    b: w[1].0.clone(),
                    b_pin: w[1].1.clone(),
                    net: net.clone(),
                    bends,
                });
            }
        }
        self.wires = wires;
        self.net_labels.clear();
        self.junctions.clear();
        self.no_connects.clear();
        self.err = None;
        self.set_erc_note_from_slice(erc_warnings);
        self.bom_dirty = true;
        self.log_console("Applied generated schematic draft.".to_string());
    }

    fn run_prompt_draft(&mut self, design_id: Uuid, ctx: &egui::Context) {
        if self.generation_rx.is_some() {
            return;
        }
        let trimmed = self.prompt.trim().to_string();
        if trimmed.is_empty() {
            self.set_err("Describe what you want the schematic to do, then generate.");
            return;
        }

        self.prompt_busy = true;
        self.err = None;
        self.erc_note = None;

        let (tx, rx) = std::sync::mpsc::channel();
        let prompt = trimmed;
        let user_id = self.user_id;
        let state = self.state.clone();
        let pool = self.pool.clone();
        let repaint = ctx.clone();

        self.rt.spawn(async move {
            let outcome: Result<(ReplaceSchematic, Vec<ErcViolation>), String> =
                tokito::services::design_pipeline::build_design_from_prompt(
                    &state, &pool, user_id, design_id, &prompt,
                )
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(outcome);
            repaint.request_repaint();
        });

        self.generation_rx = Some(rx);
        ctx.request_repaint();
    }

    fn search_parts(&mut self) {
        let q = self.parts_query.trim().to_string();
        let res = self.rt.block_on(async {
            tokito::store::parts::search(
                &self.pool,
                PartSearchParams {
                    q: if q.is_empty() { None } else { Some(q) },
                    limit: Some(50),
                },
            )
            .await
        });
        match res {
            Ok(rows) => {
                self.parts_hits = rows
                    .into_iter()
                    .map(|p| PartRow {
                        id: p.id,
                        mpn: p.mpn,
                        description: p.description,
                    })
                    .collect();
            }
            Err(e) => self.set_err(e),
        }
    }

    fn drop_part_as_symbol(&mut self, part: &PartRow) {
        self.before_canvas_edit();
        let prefix = guess_prefix(&part.mpn);
        let ref_des = next_refdes(&self.symbols, prefix);
        self.part_cache.insert(part.id, part.mpn.clone());
        let pos = if let Some(rect) = self.canvas_screen_rect {
            let origin = rect.min;
            let p = self.viewport.screen_to_world(origin, rect.center());
            if self.snap_enabled {
                snap_world_pos(p)
            } else {
                p
            }
        } else if self.snap_enabled {
            snap_world_pos(Pos2::new(240.0, 240.0))
        } else {
            Pos2::new(240.0, 240.0)
        };
        self.symbols.push(Sym {
            ref_des,
            part_id: Some(part.id),
            pos,
            rotation_deg: 0.0,
            pins: vec!["1".to_string(), "2".to_string()],
        });
    }

    fn delete_selected(&mut self) {
        if let Some(ref_des) = self.selected_sym.take() {
            self.before_canvas_edit();
            self.symbols.retain(|s| s.ref_des != ref_des);
            self.wires.retain(|w| w.a != ref_des && w.b != ref_des);
            return;
        }
        if let Some(i) = self.selected_wire.take() {
            self.before_canvas_edit();
            if i < self.wires.len() {
                self.wires.remove(i);
            }
            return;
        }
        if let Some(i) = self.selected_net_label.take() {
            self.before_canvas_edit();
            if i < self.net_labels.len() {
                self.net_labels.remove(i);
            }
            return;
        }
        if let Some(i) = self.selected_junction.take() {
            self.before_canvas_edit();
            if i < self.junctions.len() {
                self.junctions.remove(i);
            }
            return;
        }
        if let Some(i) = self.selected_no_connect.take() {
            self.before_canvas_edit();
            if i < self.no_connects.len() {
                self.no_connects.remove(i);
            }
            return;
        }
        if let Some(i) = self.selected_power_symbol.take() {
            self.before_canvas_edit();
            if i < self.power_symbols.len() {
                self.power_symbols.remove(i);
            }
            return;
        }
        if let Some(i) = self.selected_text_item.take() {
            self.before_canvas_edit();
            if i < self.text_items.len() {
                self.text_items.remove(i);
            }
            return;
        }
        if let Some(i) = self.selected_bus.take() {
            self.before_canvas_edit();
            if i < self.buses.len() {
                self.buses.remove(i);
            }
        }
    }
}

include!("impl_eframe.rs");
include!("impl_ui.rs");
