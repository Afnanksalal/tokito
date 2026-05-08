//! Egui application state and Tokito integration (DB, copilot, schematic ops).

use anyhow::Context;
use eframe::egui;
use egui::{Pos2, Rect, Sense, Stroke, Vec2};
use egui_dock::{DockArea, Style as DockStyle};
use sqlx::postgres::PgPoolOptions;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Receiver;
use tokito::models::{ErcViolation, PartSearchParams, Position, ReplaceSchematic};
use tokito::router::AppState;
use tokito::store::intent;
use uuid::Uuid;

use crate::bootstrap::ensure_local_user;
use crate::canvas::{snap_world_pos, CanvasSnapshot, Sym, Viewport, Wire, CANVAS_UNDO_CAP};
use crate::util::{guess_prefix, next_refdes};

type SchematicGenerationRx = Receiver<Result<(ReplaceSchematic, Vec<ErcViolation>), String>>;

pub mod studio_dock;

use studio_dock::StudioTab;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CanvasTool {
    #[default]
    Select,
    Wire,
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
    selected_sym: Option<String>,
    selected_wire: Option<usize>,
    dragging_sym: Option<String>,
    wire_drag_from: Option<String>,
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

    /// Projects launcher.
    projects_search: String,
    projects_sort: ProjectsSort,
    projects_pinned: HashSet<Uuid>,
    recent_design_ids: Vec<Uuid>,
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
            selected_sym: None,
            selected_wire: None,
            dragging_sym: None,
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
            projects_search: String::new(),
            projects_sort: ProjectsSort::default(),
            projects_pinned: HashSet::new(),
            recent_design_ids: vec![],
        })
    }

    fn snapshot_canvas(&self) -> CanvasSnapshot {
        CanvasSnapshot {
            symbols: self.symbols.clone(),
            wires: self.wires.clone(),
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

        match sch {
            Ok(sch) => {
                self.design = Some(design);
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
                    })
                    .collect();

                let net_id_to_name: HashMap<Uuid, String> =
                    sch.nets.iter().map(|n| (n.id, n.name.clone())).collect();
                let inst_id_to_ref: HashMap<Uuid, String> = sch
                    .instances
                    .iter()
                    .map(|i| (i.id, i.ref_des.clone()))
                    .collect();
                let mut by_net: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
                for p in sch.pins {
                    by_net.entry(p.net_id).or_default().push(p.instance_id);
                }
                let mut wires = vec![];
                for (net_id, insts) in by_net {
                    let net = net_id_to_name
                        .get(&net_id)
                        .cloned()
                        .unwrap_or_else(|| "NET".into());
                    let mut uniq: Vec<Uuid> = vec![];
                    for id in insts {
                        if !uniq.contains(&id) {
                            uniq.push(id);
                        }
                    }
                    for w in uniq.windows(2) {
                        if let (Some(a), Some(b)) =
                            (inst_id_to_ref.get(&w[0]), inst_id_to_ref.get(&w[1]))
                        {
                            wires.push(Wire {
                                a: a.clone(),
                                b: b.clone(),
                                net: net.clone(),
                            });
                        }
                    }
                }
                self.wires = wires;

                self.selected_sym = None;
                self.selected_wire = None;
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
        let body = self.graph_to_replace_schematic();
        let warns = tokito::services::schematic_validate::erc_light(&body);
        let res = self.rt.block_on(async {
            tokito::store::schematic::replace(&self.pool, design_id, body).await
        });
        match res {
            Ok(()) => {
                self.err = None;
                self.set_erc_note_from_slice(&warns);
                self.log_console("Saved schematic to board.".to_string());
            }
            Err(e) => {
                self.erc_note = None;
                self.set_err(e);
            }
        }
    }

    fn graph_to_replace_schematic(&self) -> ReplaceSchematic {
        let instances = self
            .symbols
            .iter()
            .map(|s| tokito::models::SchematicInstanceInput {
                id: None,
                ref_des: s.ref_des.clone(),
                part_id: s.part_id,
                position: Some(Position {
                    x: s.pos.x as f64,
                    y: s.pos.y as f64,
                }),
                rotation: f64::from(s.rotation_deg),
                meta: None,
            })
            .collect::<Vec<_>>();

        let net_names = self
            .wires
            .iter()
            .map(|w| w.net.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let nets = net_names
            .iter()
            .map(|n| tokito::models::SchematicNetInput {
                id: None,
                name: n.clone(),
            })
            .collect::<Vec<_>>();

        let mut pins = vec![];
        for (i, w) in self.wires.iter().enumerate() {
            pins.push(tokito::models::SchematicPinInput {
                instance_ref: w.a.clone(),
                pin_name: format!("w{}_a", i),
                net_name: w.net.clone(),
            });
            pins.push(tokito::models::SchematicPinInput {
                instance_ref: w.b.clone(),
                pin_name: format!("w{}_b", i),
                net_name: w.net.clone(),
            });
        }

        ReplaceSchematic {
            instances,
            nets,
            pins,
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
            })
            .collect();
        let mut by_net: HashMap<String, Vec<String>> = HashMap::new();
        for p in draft.pins {
            by_net.entry(p.net_name).or_default().push(p.instance_ref);
        }
        let mut wires = vec![];
        for (net, refs) in by_net {
            let mut uniq = vec![];
            for r in refs {
                if !uniq.contains(&r) {
                    uniq.push(r);
                }
            }
            for w in uniq.windows(2) {
                wires.push(Wire {
                    a: w[0].clone(),
                    b: w[1].clone(),
                    net: net.clone(),
                });
            }
        }
        self.wires = wires;
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
        }
    }
}

include!("impl_eframe.rs");
include!("impl_ui.rs");
