//! Egui application state and Tokito integration (DB, AI build, schematic ops).

use anyhow::Context;
use eframe::egui;
use egui::Pos2;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Receiver;
use tokito::models::{
    ErcViolation, PartSearchParams, ReplaceSchematic, SchematicDocument, SchematicEditBatch,
};
use tokito::router::AppState;
use tokito::store::intent;
use uuid::Uuid;

use crate::bootstrap::ensure_local_user;
use crate::canvas::{manhattan_bends, snap_world_pos, symbol_pin_world, PinEndpoint, Sym, Wire};
use crate::editor::PlaceSymbolRequest;
use crate::editor::{document, CanvasTool, SchematicEditor};
use crate::util::{guess_prefix, next_refdes};

type SchematicGenerationRx = Receiver<
    Result<
        (
            ReplaceSchematic,
            Vec<ErcViolation>,
            SchematicEditBatch,
            Vec<String>,
        ),
        String,
    >,
>;

pub mod studio_dock;

mod studio;

use studio_dock::StudioTab;

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
    pub package_name: Option<String>,
}

/// External catalog hit (LCSC / Nexar) before importing to local parts table.
#[derive(Clone)]
pub struct CatalogHit {
    pub mpn: String,
    pub manufacturer: Option<String>,
    pub description: Option<String>,
    pub package_name: Option<String>,
    pub footprint_hint: Option<String>,
    pub datasheet_url: Option<String>,
    pub distributor: String,
    pub sku: String,
    pub product_url: Option<String>,
}

#[derive(Clone, Copy)]
pub enum Route {
    Projects,
    Studio { design_id: Uuid },
}

/// Sub-view within [`Route::Projects`]: the projects launcher vs. one
/// project's designs. See `projects.rs`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProjectsView {
    /// The launcher — recent projects.
    Home,
    /// One project's designs (the project is `active_project_id`).
    Designs,
}

pub struct App {
    _embedded_db: tokito::db::EmbeddedPostgres,
    _project_embedded: Option<tokito::db::EmbeddedPostgres>,
    project_db_project_id: Option<Uuid>,
    db_url: String,
    global_pool: sqlx::PgPool,
    rt: tokio::runtime::Runtime,
    pool: sqlx::PgPool,
    state: AppState,
    db_status: tokito::db::DatabaseStatus,

    user_id: Uuid,

    route: Route,
    err: Option<String>,
    erc_note: Option<String>,

    designs: Vec<tokito::models::Design>,
    new_design_name: String,
    new_design_desc: String,

    design: Option<tokito::models::Design>,
    /// Unified place-browser query (symbols + parts catalog).
    place_query: String,
    place_scope: studio::PlaceScope,
    /// Full multi-sheet document cache while in studio.
    studio_document: Option<SchematicDocument>,
    studio_dirty: bool,
    parts_hits: Vec<PartRow>,
    catalog_hits: Vec<CatalogHit>,
    part_cache: HashMap<Uuid, String>, // part_id -> mpn

    editor: SchematicEditor,

    prompt: String,
    prompt_busy: bool,

    /// Refresh project list when switching back from Studio (and once on startup).
    projects_need_refresh: bool,

    /// Background schematic generation (never block egui thread).
    generation_rx: Option<SchematicGenerationRx>,
    build_cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,

    /// Dockable studio panels (`egui_dock`).
    dock_state: egui_dock::DockState<StudioTab>,

    /// Ring buffer of console / status lines for the Console tab.
    console_lines: Vec<String>,

    /// BOM cache for the BOM tab.
    bom_lines: Vec<tokito::models::BomLine>,
    bom_loaded_for: Option<Uuid>,
    bom_dirty: bool,

    /// Focus mode hides dock chrome and gives the schematic canvas the workspace.
    canvas_focus_mode: bool,
    properties_panel_open: bool,

    /// Projects launcher.
    projects_search: String,
    projects_sort: ProjectsSort,
    projects_pinned: HashSet<Uuid>,
    recent_design_ids: Vec<Uuid>,

    base_symbols: Option<crate::base_symbols::BaseSymbolLibrary>,

    /// ERC violations for Messages panel + navigation.
    erc_violations: Vec<ErcViolation>,

    /// Pending AI build edits awaiting user approval before they land on the canvas.
    pending_edit_batch: Option<SchematicEditBatch>,
    pending_edit_selected: Vec<bool>,

    /// AI provider and Firecrawl are configured in Settings (required to Build).
    ai_build_ready: bool,

    command_palette_open: bool,
    command_palette_query: String,
    /// Projects-screen quick switcher (⌘K) — jump to a project or design.
    projects_palette_open: bool,
    projects_palette_query: String,
    /// Type-to-filter text inside the project dropdown menu.
    project_menu_filter: String,

    /// Symbol import path typed in Place panel.
    symbol_import_path: String,

    mcad_viewer: crate::mcad_viewer::McadViewer,

    settings_file: tokito::settings::SettingsFile,
    ui_tokens: crate::ui::UiTokens,

    research_draft_title: String,
    research_draft_body: String,
    research_editing_id: Option<Uuid>,
    research_annotate_parent: Option<Uuid>,

    design_edit_name: String,
    design_edit_desc: String,
    design_edit_notes: String,

    toasts: crate::ui::toast::ToastStack,
    last_backup_label: String,
    bom_sort: crate::ui::table::SortState,
    erc_sort: crate::ui::table::SortState,
    research_sort: crate::ui::table::SortState,

    projects: Vec<tokito::models::Project>,
    active_project_id: Option<Uuid>,
    new_project_name: String,
    new_project_embedded_db: bool,
    new_design_form_open: bool,
    new_project_form_open: bool,
    projects_view: ProjectsView,
    renaming_project_id: Option<Uuid>,
    project_rename_name: String,
    renaming_design_id: Option<Uuid>,
    design_rename_name: String,
    projects_list_dirty: bool,

    build_warnings: Vec<String>,
    build_stage: String,
    build_bom_diff: Option<String>,
    design_save_debounce: f32,

    agent_query: String,
    agent_busy: bool,
    agent_last_message: String,
    agent_rx: Option<Receiver<Result<String, String>>>,

    /// Settings modal (replaces the old Settings dock tab) — see `studio/settings.rs`.
    show_settings: bool,
    settings_section: studio::SettingsSection,
    /// Set by a settings control on change; drives an auto-save this frame.
    settings_dirty: bool,
    /// Edit buffers for the numeric DB fields (parsed back into `settings_file`).
    db_port_buf: String,
    db_pgver_buf: String,

    /// Visible-design count per project, for the launcher cards.
    project_design_counts: HashMap<Uuid, i64>,
}

impl App {
    pub fn try_new() -> anyhow::Result<Self> {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "tokito=info,tower_http=warn".into()),
            )
            .init();

        let settings_file = tokito::config_provider::load_settings_merged();
        let cfg = settings_file.to_config()?;
        let ai_build_ready = cfg.llm.is_some() && cfg.firecrawl.is_some();
        let ui_tokens = crate::theme::tokens_for(&settings_file.general.theme);
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("tokio runtime")?;
        let settings_migrated = settings_file.general.settings_migrated_from_env;
        let (pool, embedded) = rt.block_on(tokito::db::connect(&cfg)).context("database")?;
        let db_url = embedded.database_url();
        let global_pool = pool.clone();
        rt.block_on(tokito::store::projects::ensure_default_workspace(&pool))?;

        let state = AppState::try_new(pool.clone(), &cfg)?;

        let user_id = rt.block_on(async { ensure_local_user(&pool).await })?;

        let base_symbols = crate::base_symbols::BaseSymbolLibrary::open().ok();

        let mut app = Self {
            _embedded_db: embedded,
            _project_embedded: None,
            project_db_project_id: None,
            db_url,
            global_pool,
            rt,
            pool,
            state,
            db_status: tokito::db::DatabaseStatus::Ready,
            user_id,
            route: Route::Projects,
            err: None,
            erc_note: None,
            designs: vec![],
            new_design_name: "New design".to_string(),
            new_design_desc: "".to_string(),
            design: None,
            place_query: String::new(),
            place_scope: studio::PlaceScope::default(),
            studio_document: None,
            studio_dirty: false,
            parts_hits: vec![],
            catalog_hits: vec![],
            part_cache: HashMap::new(),
            editor: SchematicEditor::default(),
            prompt: "".to_string(),
            prompt_busy: false,
            projects_need_refresh: true,
            generation_rx: None,
            build_cancel_flag: None,
            dock_state: studio_dock::default_studio_dock(),
            console_lines: vec![],
            bom_lines: vec![],
            bom_loaded_for: None,
            bom_dirty: true,
            canvas_focus_mode: false,
            properties_panel_open: false,
            projects_search: String::new(),
            projects_sort: ProjectsSort::default(),
            projects_pinned: HashSet::new(),
            recent_design_ids: vec![],
            base_symbols,
            erc_violations: vec![],
            pending_edit_batch: None,
            pending_edit_selected: vec![],
            ai_build_ready,
            command_palette_open: false,
            command_palette_query: String::new(),
            projects_palette_open: false,
            projects_palette_query: String::new(),
            project_menu_filter: String::new(),
            symbol_import_path: String::new(),
            mcad_viewer: crate::mcad_viewer::McadViewer::default(),
            settings_file,
            ui_tokens,
            research_draft_title: String::new(),
            research_draft_body: String::new(),
            research_editing_id: None,
            research_annotate_parent: None,
            design_edit_name: String::new(),
            design_edit_desc: String::new(),
            design_edit_notes: String::new(),
            toasts: crate::ui::toast::ToastStack::default(),
            last_backup_label: "never".into(),
            bom_sort: crate::ui::table::SortState::default(),
            erc_sort: crate::ui::table::SortState::default(),
            research_sort: crate::ui::table::SortState::default(),
            projects: vec![],
            active_project_id: Some(tokito::store::projects::default_project_id()),
            new_project_name: String::new(),
            new_project_embedded_db: false,
            new_design_form_open: false,
            new_project_form_open: false,
            projects_view: ProjectsView::Home,
            renaming_project_id: None,
            project_rename_name: String::new(),
            renaming_design_id: None,
            design_rename_name: String::new(),
            projects_list_dirty: true,
            build_warnings: vec![],
            build_stage: String::new(),
            build_bom_diff: None,
            design_save_debounce: 0.0,
            agent_query: String::new(),
            agent_busy: false,
            agent_last_message: String::new(),
            agent_rx: None,
            show_settings: false,
            settings_section: studio::SettingsSection::default(),
            settings_dirty: false,
            db_port_buf: String::new(),
            db_pgver_buf: String::new(),
            project_design_counts: HashMap::new(),
        };
        if settings_migrated {
            app.toasts.push(
                "Imported legacy settings into settings.toml",
                crate::ui::toast::ToastKind::Info,
            );
        }
        Ok(app)
    }

    /// Open the settings modal, syncing the numeric edit buffers from disk state.
    pub(crate) fn open_settings(&mut self) {
        self.db_port_buf = self.settings_file.database.embedded_port.to_string();
        self.db_pgver_buf = self.settings_file.database.pg_embed_version.to_string();
        self.show_settings = true;
    }

    pub(crate) fn autosave_design_info(&mut self, design_id: Uuid) {
        let Some(design) = self.design.clone() else {
            return;
        };
        if design.id != design_id {
            return;
        }
        let name = self.design_edit_name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let desc = self.design_edit_desc.trim().to_string();
        let notes = self.design_edit_notes.trim().to_string();
        let patch = tokito::models::PatchDesign {
            name: Some(name),
            description: if desc.is_empty() { None } else { Some(desc) },
            notes: if notes.is_empty() { None } else { Some(notes) },
        };
        match self
            .rt
            .block_on(async { tokito::store::designs::patch(&self.pool, design_id, patch).await })
        {
            Ok(row) => {
                self.design = Some(row);
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }

    pub(crate) fn shutdown_database(&mut self) {
        self.rt.block_on(self._embedded_db.graceful_stop());
        if let Some(ref mut emb) = self._project_embedded {
            self.rt.block_on(emb.graceful_stop());
        }
    }

    fn disconnect_project_db(&mut self) {
        if let Some(mut emb) = self._project_embedded.take() {
            self.rt.block_on(emb.graceful_stop());
        }
        self.project_db_project_id = None;
        self.pool = self.global_pool.clone();
        self.db_url = self._embedded_db.database_url();
        if let Ok(cfg) = self.settings_file.to_config() {
            if let Ok(st) = AppState::try_new(self.pool.clone(), &cfg) {
                self.state = st;
            }
        }
    }

    fn connect_project_db_for_project(&mut self, project_id: Uuid) {
        if self.project_db_project_id == Some(project_id) {
            return;
        }
        self.disconnect_project_db();
        let project = match self
            .rt
            .block_on(tokito::store::projects::get(&self.global_pool, project_id))
        {
            Ok(project) => project,
            Err(e) => {
                self.set_err(e.to_string());
                return;
            }
        };
        let ws = std::path::PathBuf::from(&project.workspace_path);
        let meta = tokito::store::projects::read_toml_for_workspace(&ws);
        if !meta.uses_embedded_db() {
            return;
        }
        let cfg = match self.settings_file.to_config() {
            Ok(c) => c,
            Err(e) => {
                self.set_err(e.to_string());
                return;
            }
        };
        let res = self
            .rt
            .block_on(tokito::db::connect_project_embedded(&ws, &cfg));
        match res {
            Ok((pool, emb)) => {
                let seed = self.rt.block_on(async {
                    let user =
                        tokito::store::account::find_user_by_id(&self.global_pool, self.user_id)
                            .await?
                            .ok_or_else(|| {
                                tokito::error::AppError::Unauthorized("local user not found".into())
                            })?;
                    tokito::store::account::upsert_user_seed(&pool, &user).await?;
                    tokito::store::projects::upsert_existing(&pool, &project).await
                });
                if let Err(e) = seed {
                    self.set_err(e.to_string());
                    return;
                }
                self.db_url = emb.database_url();
                self._project_embedded = Some(emb);
                self.project_db_project_id = Some(project_id);
                self.pool = pool.clone();
                if let Ok(st) = AppState::try_new(pool, &cfg) {
                    self.state = st;
                }
                self.db_status = tokito::db::DatabaseStatus::Ready;
                self.log_console(format!(
                    "Using per-project database at {}",
                    tokito::project_toml::ProjectToml::embedded_data_dir(&ws).display()
                ));
            }
            Err(e) => {
                self.db_status = tokito::db::DatabaseStatus::Error;
                self.set_err(format!("Project database failed: {e}"));
            }
        }
    }

    fn connect_project_db_for_design(&mut self, design_id: Uuid) {
        self.disconnect_project_db();
        let ws = self.rt.block_on(async {
            tokito::store::projects::workspace_path_for_design(&self.global_pool, design_id).await
        });
        let Ok(ws) = ws else { return };
        let meta = tokito::store::projects::read_toml_for_workspace(&ws);
        if !meta.uses_embedded_db() {
            return;
        }
        let cfg = match self.settings_file.to_config() {
            Ok(c) => c,
            Err(e) => {
                self.set_err(e.to_string());
                return;
            }
        };
        let res = self
            .rt
            .block_on(tokito::db::connect_project_embedded(&ws, &cfg));
        match res {
            Ok((pool, emb)) => {
                self._project_embedded = Some(emb);
                self.pool = pool.clone();
                if let Ok(st) = AppState::try_new(pool, &cfg) {
                    self.state = st;
                }
                self.db_status = tokito::db::DatabaseStatus::Ready;
                self.log_console(format!(
                    "Using per-project database at {}",
                    tokito::project_toml::ProjectToml::embedded_data_dir(&ws).display()
                ));
            }
            Err(e) => {
                self.db_status = tokito::db::DatabaseStatus::Error;
                self.set_err(format!("Project database failed: {e}"));
            }
        }
    }

    pub(crate) fn toast_ok(&mut self, msg: impl Into<String>) {
        self.toasts.push(msg, crate::ui::toast::ToastKind::Success);
    }

    pub(crate) fn refresh_projects(&mut self) {
        let res = self
            .rt
            .block_on(tokito::store::projects::list(&self.global_pool, 64));
        match res {
            Ok(rows) => {
                self.projects = rows;
                self.projects_list_dirty = false;
            }
            Err(e) => self.set_err(e.to_string()),
        }
        let user_id = self.user_id;
        if let Ok(counts) = self.rt.block_on(tokito::store::designs::count_by_project(
            &self.global_pool,
            user_id,
        )) {
            self.project_design_counts = counts;
        }
    }

    pub(crate) fn erc_strict(&self) -> bool {
        true
    }

    pub(crate) fn export_blocked_by_erc(&self) -> bool {
        self.erc_strict()
            && tokito::services::schematic_validate::has_blocking_erc(&self.erc_violations)
    }

    pub(crate) fn backup_current_design(&mut self, design_id: Uuid) {
        let name = self
            .design
            .as_ref()
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "design".into());
        let document = self.graph_to_document();
        let (replace, _) = document.to_replace_schematic();
        let view = tokito::models::SchematicView::from_replace(design_id, &replace);
        let ws = self
            .rt
            .block_on(tokito::store::projects::workspace_path_for_design(
                &self.pool, design_id,
            ))
            .unwrap_or_else(|_| tokito::paths::project_dir("default"));
        let csv = self
            .rt
            .block_on(async { tokito::store::bom::csv_export(&self.pool, design_id).await })
            .unwrap_or_default();
        let pg_ver = self.settings_file.database.pg_embed_version;
        match self
            .rt
            .block_on(tokito::services::backup::write_design_backup_with_db_async(
                &ws,
                &name,
                &document,
                &view,
                &csv,
                Some(&self.db_url),
                pg_ver,
            )) {
            Ok(dir) => {
                self.last_backup_label = time_format_now();
                self.log_console(format!("Backup saved to {}", dir.display()));
                self.toast_ok("Design backup saved");
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }

    pub(crate) fn export_dir_for_design(&self, design_id: Uuid) -> std::path::PathBuf {
        let ws = self
            .rt
            .block_on(tokito::store::projects::workspace_path_for_design(
                &self.pool, design_id,
            ))
            .unwrap_or_else(|_| tokito::paths::project_dir("default"));
        tokito::paths::project_exports_dir(&ws)
    }

    pub(crate) fn open_path_after_export(&self, path: &std::path::Path) {
        let _ = open::that(path);
    }

    pub(crate) fn bom_sync_status(&mut self, _design_id: Uuid) -> String {
        let doc = self.graph_to_document();
        let (replace, _) = doc.to_replace_schematic();
        let proposed = tokito::services::bom_sync::propose_from_schematic(&replace);
        if proposed.is_empty() && self.bom_lines.is_empty() {
            return "No BOM".into();
        }
        let current_qty: f64 = self.bom_lines.iter().map(|l| l.quantity).sum();
        let proposed_qty: f64 = proposed.iter().map(|l| l.quantity).sum();
        if (current_qty - proposed_qty).abs() < 0.01 && self.bom_lines.len() == proposed.len() {
            "In sync".into()
        } else {
            format!(
                "{} line(s) differ from schematic",
                proposed.len().abs_diff(self.bom_lines.len()).max(1)
            )
        }
    }

    pub(crate) fn add_empty_bom_line(&mut self, design_id: Uuid) {
        let res = self.rt.block_on(async {
            let parts = tokito::store::parts::search(
                &self.pool,
                tokito::models::PartSearchParams {
                    q: Some("generic".into()),
                    limit: Some(1),
                },
            )
            .await?;
            let part_id = parts
                .first()
                .map(|p| p.id)
                .ok_or_else(|| tokito::error::AppError::BadRequest("no parts in catalog".into()))?;
            tokito::store::bom::append_lines(
                &self.pool,
                design_id,
                &[tokito::models::BomLineInput {
                    part_id,
                    quantity: 1.0,
                    sort_order: 0,
                    notes: None,
                }],
            )
            .await
        });
        match res {
            Ok(_) => {
                self.bom_dirty = true;
                self.toast_ok("BOM line added");
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }

    pub(crate) fn sync_bom_from_schematic(&mut self, design_id: Uuid) {
        let doc = self.graph_to_document();
        let (replace, _) = doc.to_replace_schematic();
        let proposed = tokito::services::bom_sync::propose_from_schematic(&replace);
        if proposed.is_empty() {
            self.set_err("No parts with part_id on schematic to sync");
            return;
        }
        let inputs = tokito::services::bom_sync::to_bom_inputs(&proposed);
        let body = tokito::models::ReplaceBom { lines: inputs };
        let res = self.rt.block_on(async {
            tokito::store::bom::replace_validated(&self.pool, design_id, body).await
        });
        match res {
            Ok(_) => {
                self.bom_dirty = true;
                self.log_console("BOM synced from schematic.");
            }
            Err(e) => self.set_err(e),
        }
    }

    pub(crate) fn suggest_erc_fixes(&mut self) {
        let doc = self.graph_to_document();
        let batch = tokito::services::erc_fixes::propose_fixes(&doc, &self.erc_violations);
        if batch.ops.is_empty() {
            self.log_console("No automatic ERC fixes available.".to_string());
            return;
        }
        let n = batch.ops.len();
        self.pending_edit_batch = Some(batch);
        self.pending_edit_selected = vec![true; n];
        self.log_console("ERC fix suggestions ready in Build; review and apply.".to_string());
    }

    pub(crate) fn import_symbol_library_folder(&mut self) {
        let path = self.symbol_import_path.trim();
        if path.is_empty() {
            self.set_err("Enter a folder path containing .tokito_sym or .kicad_sym files.");
            return;
        }
        match crate::symbol_library::import_folder(std::path::Path::new(path)) {
            Ok(n) => {
                self.base_symbols = crate::base_symbols::BaseSymbolLibrary::open().ok();
                self.err = None;
                self.log_console(format!(
                    "Imported {n} symbols; restart search or reload Place panel."
                ));
            }
            Err(e) => self.set_err(format!("Import failed: {e}")),
        }
    }

    fn before_canvas_edit(&mut self) {
        self.editor.before_edit();
        self.studio_dirty = true;
        self.mcad_viewer.invalidate();
    }

    pub(crate) fn flush_studio_document(&mut self) {
        let doc = self
            .studio_document
            .get_or_insert_with(SchematicDocument::empty);
        document::export_document(&self.editor, &self.part_cache, doc);
    }

    pub(crate) fn switch_active_sheet(&mut self, sheet_id: String) {
        if self.editor.active_sheet_id == sheet_id {
            return;
        }
        self.flush_studio_document();
        self.editor.active_sheet_id = sheet_id.clone();
        if let Some(doc) = self.studio_document.clone() {
            crate::editor::sheets::hydrate_active_sheet(&mut self.editor, &doc, &sheet_id);
            self.enrich_symbols_from_library();
        }
        self.studio_dirty = true;
    }

    fn graph_to_document(&mut self) -> SchematicDocument {
        self.flush_studio_document();
        self.studio_document
            .clone()
            .unwrap_or_else(SchematicDocument::empty)
    }

    fn undo_canvas(&mut self) {
        self.editor.undo();
    }

    fn redo_canvas(&mut self) {
        self.editor.redo();
    }

    fn clear_canvas_selection(&mut self) {
        self.editor.clear_selection();
    }

    fn apply_document_to_canvas(&mut self, doc: SchematicDocument) {
        self.studio_document = Some(doc.clone());
        document::load_document(&mut self.editor, doc);
        self.enrich_symbols_from_library();
        self.studio_dirty = false;
    }

    fn load_prompt_after_open(&mut self, design_id: Uuid) {
        self.editor.clear_history();
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
        let mut s = format!("ERC advisory ({}): {}", w.len(), head.join(" | "));
        if w.len() > 4 {
            s.push_str(&format!(" (+{} more)", w.len() - 4));
        }
        self.erc_note = Some(s);
    }

    fn reload_projects(&mut self) {
        self.err = None;
        if self.projects_list_dirty {
            self.refresh_projects();
        }
        if let Some(pid) = self.active_project_id {
            self.connect_project_db_for_project(pid);
        } else {
            self.disconnect_project_db();
        }
        let user_id = self.user_id;
        let res = self.rt.block_on(async {
            if let Some(pid) = self.active_project_id {
                tokito::store::designs::list_for_project(&self.pool, pid, user_id, 100).await
            } else {
                tokito::store::designs::list_for_user(&self.pool, user_id, 100).await
            }
        });
        match res {
            Ok(rows) => self.designs = rows,
            Err(e) => self.set_err(e),
        }
    }

    pub(crate) fn hovered_net_name(&self, ctx: &egui::Context) -> Option<String> {
        self.editor.hovered_net_at_pointer(ctx)
    }

    fn open_design(&mut self, design_id: Uuid) {
        self.err = None;
        if let Some(pid) = self.active_project_id {
            self.connect_project_db_for_project(pid);
        } else {
            self.connect_project_db_for_design(design_id);
        }
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
                self.design_edit_name = design.name.clone();
                self.design_edit_desc = design.description.clone().unwrap_or_default();
                self.design_edit_notes = design.notes.clone().unwrap_or_default();
                self.design = Some(design);
                if let Some(doc) = stored_doc {
                    self.apply_document_to_canvas(doc);
                } else {
                    self.studio_document = None;
                    self.editor.net_labels.clear();
                    self.editor.junctions.clear();
                    self.editor.no_connects.clear();
                    self.editor.power_symbols.clear();
                    self.editor.text_items.clear();
                    self.editor.buses.clear();
                    self.editor.symbols = sch
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
                            footprint_ref: None,
                            symbol_id: None,
                            pin_layout: vec![],
                            value: String::new(),
                            fields: Default::default(),
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
                                let a_sym = self.editor.symbols.iter().find(|s| s.ref_des == *a);
                                let b_sym = self.editor.symbols.iter().find(|s| s.ref_des == *b);
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
                    self.editor.load_legacy_wires(wires);
                    self.enrich_symbols_from_library();
                    self.editor.net_labels.clear();
                    self.editor.junctions.clear();
                    self.editor.no_connects.clear();
                    let mut doc = SchematicDocument::empty();
                    crate::editor::sheets::flush_active_sheet(
                        &self.editor,
                        &mut doc,
                        &self.part_cache,
                    );
                    self.studio_document = Some(doc);
                    self.studio_dirty = false;
                }

                self.clear_canvas_selection();
                self.editor.reset_view();
                self.load_prompt_after_open(design_id);
                self.push_recent_design(design_id);
                self.bom_dirty = true;
                self.log_console(format!(
                    "Opened schematic | {}",
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

    pub(crate) fn export_schematic_file(&mut self, kind: &str) {
        if self.export_blocked_by_erc() {
            self.set_err(
                "Export blocked: fix ERC errors before exporting (ERC strict mode is on).",
            );
            return;
        }
        let safe_name = self
            .design
            .as_ref()
            .map(|d| d.name.as_str())
            .unwrap_or("design")
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>();
        let document = self.graph_to_document();
        let design_id = match self.route {
            Route::Studio { design_id } => design_id,
            _ => return,
        };
        let design_title = self
            .design
            .as_ref()
            .map(|d| d.name.as_str())
            .unwrap_or(&safe_name);
        let export_dir = self.export_dir_for_design(design_id);
        let _ = std::fs::create_dir_all(&export_dir);
        let ext = match kind {
            "svg" => "svg",
            "netlist" => "txt",
            "sexp_netlist" => "net",
            "pdf" => "pdf",
            "pdf_pack" => "pdf",
            "bom_csv" => "csv",
            "mcad" => "json",
            _ => "bin",
        };
        let dated = tokito::services::export_service::dated_filename(&safe_name, ext);
        let picked = rfd::FileDialog::new()
            .set_directory(&export_dir)
            .set_file_name(&dated)
            .save_file();
        let Some(target) = picked else {
            self.log_console("Export canceled.");
            return;
        };
        let result = match kind {
            "svg" => {
                let svg =
                    tokito::services::svg_export::document_to_svg_titled(&document, design_title);
                std::fs::write(&target, svg).map(|_| target)
            }
            "netlist" | "sexp_netlist" => {
                let (replace, _) = document.to_replace_schematic();
                let view = tokito::models::SchematicView::from_replace(design_id, &replace);
                let text = if kind == "sexp_netlist" {
                    tokito::services::sexp_netlist::export(&view)
                } else {
                    tokito::services::netlist::connectivity_text(&view)
                };
                std::fs::write(&target, text).map(|_| target)
            }
            "pdf" => {
                let pdf =
                    tokito::services::pdf_export::document_to_pdf_titled(&document, design_title);
                std::fs::write(&target, pdf).map(|_| target)
            }
            "pdf_pack" => {
                let csv = self.rt.block_on(async {
                    tokito::store::bom::csv_export(&self.pool, design_id).await
                });
                let erc = self.erc_violations.clone();
                match csv {
                    Ok(bom_csv) => {
                        let pdf = tokito::services::pdf_export::document_to_pdf_pack(
                            &document,
                            design_title,
                            &bom_csv,
                            &erc,
                        );
                        std::fs::write(&target, pdf).map(|_| target)
                    }
                    Err(e) => Err(std::io::Error::other(e.to_string())),
                }
            }
            "bom_csv" => {
                let csv = self.rt.block_on(async {
                    tokito::store::bom::csv_export(&self.pool, design_id).await
                });
                match csv {
                    Ok(text) => std::fs::write(&target, text).map(|_| target),
                    Err(e) => Err(std::io::Error::other(e.to_string())),
                }
            }
            "bundle" => {
                let (replace, _) = document.to_replace_schematic();
                let view = tokito::models::SchematicView::from_replace(design_id, &replace);
                match self
                    .rt
                    .block_on(async { tokito::store::bom::csv_export(&self.pool, design_id).await })
                {
                    Ok(csv) => {
                        let mcad = tokito::services::mcad_export::document_handoff_json(
                            &document,
                            design_title,
                        );
                        match tokito::services::export_bundle::write_design_exports_zip(
                            &export_dir,
                            &safe_name,
                            &document,
                            &view,
                            &csv,
                            Some(&mcad),
                        ) {
                            Ok(w) => Ok(w.zip_path.unwrap_or_else(|| {
                                export_dir.join(format!("{safe_name}_bundle.zip"))
                            })),
                            Err(e) => Err(std::io::Error::other(e.to_string())),
                        }
                    }
                    Err(e) => Err(std::io::Error::other(e.to_string())),
                }
            }
            "mcad" => {
                let json =
                    tokito::services::mcad_export::document_handoff_json(&document, design_title);
                std::fs::write(&target, json).map(|_| target)
            }
            _ => return,
        };
        match result {
            Ok(path) => {
                self.err = None;
                self.log_console(format!("Exported {}", path.display()));
                self.toast_ok(format!(
                    "Exported {}",
                    path.file_name().and_then(|s| s.to_str()).unwrap_or("file")
                ));
                self.open_path_after_export(path.as_path());
                crate::util::reveal_in_folder(path.as_path());
            }
            Err(e) => self.set_err(format!("Export failed: {e}")),
        }
    }

    fn save_schematic(&mut self, design_id: Uuid) {
        let mut document = self.graph_to_document();
        self.studio_dirty = false;
        let (body, document_diagnostics) = document.to_replace_schematic();
        let warns = tokito::services::schematic_validate::erc_full_with_options(
            &body,
            &document,
            self.erc_strict(),
        );
        self.erc_violations = warns.clone();
        document.erc_markers = tokito::services::schematic_validate::violations_to_erc_markers(
            &warns,
            tokito::models::DEFAULT_SHEET_ID,
            (120.0, 80.0),
        );
        self.editor.erc_markers = document
            .erc_markers
            .iter()
            .map(|m| crate::editor::ErcMarkerOnCanvas {
                code: m.code.clone(),
                message: m.message.clone(),
                severity: m.severity.clone(),
                position: Pos2::new(m.position.x as f32, m.position.y as f32),
                instance_ref: m.instance_ref.clone(),
                net_name: m.net_name.clone(),
            })
            .collect();
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
                Ok(Ok((_draft, erc, batch, warnings))) => {
                    self.prompt_busy = false;
                    self.generation_rx = None;
                    let cancelled = self
                        .build_cancel_flag
                        .as_ref()
                        .is_some_and(|f| f.load(std::sync::atomic::Ordering::Relaxed));
                    self.build_cancel_flag = None;
                    if cancelled {
                        self.log_console("Build cancelled.".to_string());
                        return;
                    }
                    self.pending_edit_batch = Some(batch);
                    self.pending_edit_selected = vec![true];
                    self.erc_violations = erc.clone();
                    self.build_warnings = warnings.clone();
                    self.set_erc_note_from_slice(&erc);
                    for w in warnings {
                        self.log_console(format!("Build: {w}"));
                    }
                    self.build_stage.clear();
                    if let Route::Studio { design_id } = self.route {
                        if let Ok(v) =
                            self.rt
                                .block_on(tokito::services::design_pipeline::reconcile_bom(
                                    &self.pool, design_id,
                                ))
                        {
                            if let Some(msg) = v.get("message").and_then(|m| m.as_str()) {
                                self.build_bom_diff = Some(msg.to_string());
                            }
                        }
                    }
                    self.log_console("Build complete; review proposed changes in the Build tab.");
                }
                Ok(Err(msg)) => {
                    self.prompt_busy = false;
                    self.generation_rx = None;
                    let mut m = msg;
                    if m.contains("API_KEY") || m.contains("api key") || m.contains("AI") {
                        m.push_str(&format!("; {}", tokito::user_messages::AI_NOT_CONFIGURED));
                    }
                    if m.contains("Firecrawl") || m.contains("firecrawl") {
                        m.push_str(&format!(
                            "; {}",
                            tokito::user_messages::FIRECRAWL_NOT_CONFIGURED
                        ));
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
                    let cancelled = self
                        .build_cancel_flag
                        .as_ref()
                        .is_some_and(|f| f.load(std::sync::atomic::Ordering::Relaxed));
                    self.build_cancel_flag = None;
                    if cancelled {
                        self.log_console("Build cancelled.".to_string());
                    } else {
                        self.set_err("Schematic generation stopped unexpectedly.");
                    }
                }
            }
        }

        if let Some(rx) = self.agent_rx.take() {
            match rx.try_recv() {
                Ok(Ok(text)) => {
                    self.agent_busy = false;
                    self.agent_last_message = text.clone();
                    self.log_console(format!("Agent: {text}"));
                }
                Ok(Err(e)) => {
                    self.agent_busy = false;
                    self.set_err(e);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.agent_rx = Some(rx);
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.agent_busy = false;
                }
            }
        }

        if self.prompt_busy || self.agent_busy {
            ctx.request_repaint();
        }
    }

    pub(crate) fn apply_pending_edit_batch(&mut self) {
        let Some(batch) = self.pending_edit_batch.take() else {
            return;
        };
        let selected = self.pending_edit_selected.clone();
        let erc = self.erc_violations.clone();
        let mut applied = 0usize;
        let mut any = false;
        for (_op, on) in batch.ops.iter().zip(selected.iter()) {
            if *on {
                any = true;
                break;
            }
        }
        if !any {
            self.pending_edit_selected.clear();
            self.log_console("No build edits selected.".to_string());
            return;
        }
        self.before_canvas_edit();
        for (op, on) in batch.ops.into_iter().zip(selected) {
            if !on {
                continue;
            }
            match op {
                tokito::models::SchematicEditOp::ReplaceSchematic { schematic, .. } => {
                    self.apply_generated_schematic(schematic, &erc, false);
                    applied += 1;
                }
                tokito::models::SchematicEditOp::AddInstance {
                    ref_des,
                    part_id,
                    position,
                    rotation,
                    ..
                } => {
                    let prefix: String = ref_des
                        .chars()
                        .take_while(|c| c.is_ascii_alphabetic())
                        .collect();
                    let prefix = if prefix.is_empty() {
                        "U".to_string()
                    } else {
                        prefix
                    };
                    let (symbol_id, pin_layout) =
                        self.resolve_symbol_for_instance(part_id, &prefix, None);
                    let pins: Vec<String> = if pin_layout.is_empty() {
                        vec!["1".into(), "2".into()]
                    } else {
                        pin_layout.iter().map(|(n, _, _)| n.clone()).collect()
                    };
                    let value = self.placement_value_for(
                        symbol_id.as_deref(),
                        &prefix,
                        part_id.and_then(|id| self.part_cache.get(&id).map(String::as_str)),
                    );
                    self.editor.symbols.push(Sym {
                        ref_des: ref_des.clone(),
                        part_id,
                        pos: snap_world_pos(Pos2::new(position.x as f32, position.y as f32)),
                        rotation_deg: rotation as f32,
                        pins,
                        footprint_ref: None,
                        symbol_id,
                        pin_layout,
                        value,
                        fields: Default::default(),
                    });
                    applied += 1;
                }
                tokito::models::SchematicEditOp::RemoveInstance { ref_des, .. } => {
                    self.editor.symbols.retain(|s| s.ref_des != ref_des);
                    applied += 1;
                }
                tokito::models::SchematicEditOp::ConnectPins { net_name, pins, .. } => {
                    for w in pins.windows(2) {
                        let a = PinEndpoint {
                            ref_des: w[0].0.clone(),
                            pin_name: w[0].1.clone(),
                        };
                        let b = PinEndpoint {
                            ref_des: w[1].0.clone(),
                            pin_name: w[1].1.clone(),
                        };
                        self.editor.push_wire_between(a, b, net_name.clone());
                    }
                    applied += 1;
                }
                tokito::models::SchematicEditOp::SetInstanceField {
                    ref_des,
                    field,
                    value,
                    ..
                } => {
                    if let Some(sym) = self
                        .editor
                        .symbols
                        .iter_mut()
                        .find(|s| s.ref_des == ref_des)
                    {
                        if field.eq_ignore_ascii_case("footprint") {
                            sym.footprint_ref = Some(value);
                            applied += 1;
                        } else if field.eq_ignore_ascii_case("value") {
                            sym.value = value;
                            applied += 1;
                        }
                    }
                }
            }
        }
        self.pending_edit_selected.clear();
        self.studio_dirty = true;
        self.log_console(format!(
            "Applied {applied} build edit(s); Undo reverts the batch."
        ));
    }

    fn resolve_symbol_for_instance(
        &self,
        part_id: Option<Uuid>,
        prefix: &str,
        meta_symbol_id: Option<&str>,
    ) -> (Option<String>, Vec<(String, f32, f32)>) {
        let Some(lib) = self.base_symbols.as_ref() else {
            return (None, vec![]);
        };
        let mpn = part_id.and_then(|id| self.part_cache.get(&id).map(String::as_str));
        lib.resolve_for_placement(meta_symbol_id, mpn, prefix)
    }

    /// Attach library graphics and pin layouts to symbols missing metadata (e.g. after AI build).
    fn enrich_symbols_from_library(&mut self) {
        let Some(lib) = self.base_symbols.as_ref() else {
            return;
        };
        for sym in &mut self.editor.symbols {
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
            let mpn = sym
                .part_id
                .and_then(|id| self.part_cache.get(&id).map(|s| s.as_str()));
            let (symbol_id, pin_layout) =
                lib.resolve_for_placement(sym.symbol_id.as_deref(), mpn, &prefix);
            if sym.symbol_id.is_none() {
                sym.symbol_id = symbol_id;
            }
            let layout_tiny = sym.pin_layout.is_empty()
                || sym
                    .pin_layout
                    .iter()
                    .map(|(_, x, y)| x.abs().max(y.abs()))
                    .fold(0.0_f32, f32::max)
                    < 20.0;
            if sym.symbol_id.is_some() || layout_tiny {
                sym.pin_layout = pin_layout.clone();
            }
            if sym.pins.is_empty() && !sym.pin_layout.is_empty() {
                sym.pins = sym.pin_layout.iter().map(|(n, _, _)| n.clone()).collect();
            }
        }
        let lib = self.base_symbols.as_ref();
        for sym in &mut self.editor.symbols {
            if sym.value.is_empty() {
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
                let mpn = sym
                    .part_id
                    .and_then(|id| self.part_cache.get(&id).map(|s| s.as_str()));
                sym.value =
                    placement_value_for_symbols(lib, sym.symbol_id.as_deref(), &prefix, mpn);
            }
        }
    }

    fn placement_value_for(
        &self,
        symbol_id: Option<&str>,
        prefix: &str,
        part_mpn: Option<&str>,
    ) -> String {
        placement_value_for_symbols(self.base_symbols.as_ref(), symbol_id, prefix, part_mpn)
    }

    pub(crate) fn cut_selection(&mut self) {
        if self.editor.copy_selection() && self.editor.delete_selected() {
            self.after_canvas_geometry_change();
        }
    }

    pub(crate) fn paste_selection(&mut self) {
        let n = self.editor.paste_clipboard();
        if n > 0 {
            self.after_canvas_geometry_change();
            self.log_console(format!("Pasted {n} item(s)."));
        }
    }

    pub(crate) fn duplicate_selection(&mut self) {
        if self.editor.selected_syms.is_empty() {
            if let Some(rd) = self.editor.selected_sym.clone() {
                self.editor.selected_syms.insert(rd);
            } else {
                return;
            }
        }
        self.before_canvas_edit();
        let n = self
            .editor
            .duplicate_selected_symbols(egui::Vec2::new(crate::canvas::GRID_PX * 2.0, 0.0));
        if n > 0 {
            self.after_canvas_geometry_change();
            self.log_console(format!("Duplicated {n} symbol(s)."));
        }
    }

    pub(crate) fn place_generic_symbol(&mut self, prefix: &str) {
        let (symbol_id, pin_layout) = self
            .base_symbols
            .as_ref()
            .and_then(|lib| lib.default_for_prefix(prefix))
            .map(|(id, pins)| (Some(id), pins))
            .unwrap_or((None, vec![]));
        let default_value = symbol_id
            .as_deref()
            .and_then(|id| {
                self.base_symbols
                    .as_ref()
                    .map(|lib| lib.default_value_for(id))
            })
            .unwrap_or_else(|| crate::component_value::default_value_for_library_id(prefix));
        self.editor.place_request = Some(PlaceSymbolRequest {
            prefix: prefix.to_string(),
            part_id: None,
            symbol_id,
            pin_layout,
            default_value,
        });
        self.editor.tool = CanvasTool::PlaceSymbol;
    }

    pub(crate) fn discard_pending_edit_batch(&mut self) {
        self.pending_edit_batch = None;
        self.pending_edit_selected.clear();
        self.log_console("Discarded proposed build edits.");
    }

    fn apply_generated_schematic(
        &mut self,
        draft: ReplaceSchematic,
        erc_warnings: &[ErcViolation],
        record_undo: bool,
    ) {
        if record_undo {
            self.before_canvas_edit();
        }
        self.editor.symbols = draft
            .instances
            .iter()
            .map(|i| {
                let prefix: String = i
                    .ref_des
                    .chars()
                    .take_while(|c| c.is_ascii_alphabetic())
                    .collect();
                let prefix = if prefix.is_empty() {
                    "U".to_string()
                } else {
                    prefix
                };
                let meta_sym = i.meta.as_ref().and_then(|m| {
                    m.get("symbol_id")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                });
                let meta_value = i
                    .meta
                    .as_ref()
                    .and_then(|m| m.get("value").and_then(|v| v.as_str()).map(String::from));
                let (symbol_id, pin_layout) =
                    self.resolve_symbol_for_instance(i.part_id, &prefix, meta_sym.as_deref());
                Sym {
                    ref_des: i.ref_des.clone(),
                    part_id: i.part_id,
                    pos: snap_world_pos(Pos2::new(
                        i.position.as_ref().map(|p| p.x).unwrap_or(120.0) as f32,
                        i.position.as_ref().map(|p| p.y).unwrap_or(120.0) as f32,
                    )),
                    rotation_deg: i.rotation as f32,
                    pins: if pin_layout.is_empty() {
                        draft
                            .pins
                            .iter()
                            .filter(|p| p.instance_ref == i.ref_des)
                            .map(|p| p.pin_name.clone())
                            .collect::<HashSet<_>>()
                            .into_iter()
                            .collect()
                    } else {
                        pin_layout.iter().map(|(n, _, _)| n.clone()).collect()
                    },
                    footprint_ref: None,
                    symbol_id,
                    pin_layout,
                    value: meta_value.unwrap_or_default(),
                    fields: Default::default(),
                }
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
                let a_sym = self.editor.symbols.iter().find(|s| s.ref_des == w[0].0);
                let b_sym = self.editor.symbols.iter().find(|s| s.ref_des == w[1].0);
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
        self.editor.load_legacy_wires(wires);
        self.enrich_symbols_from_library();
        self.editor.sync_anchored_wire_endpoints();
        self.editor.net_labels.clear();
        self.editor.junctions.clear();
        self.editor.no_connects.clear();
        self.editor.request_zoom_fit();
        self.err = None;
        self.set_erc_note_from_slice(erc_warnings);
        self.bom_dirty = true;
        self.log_console("Applied generated schematic draft.".to_string());
    }

    pub(crate) fn cancel_build(&mut self) {
        if let Some(flag) = &self.build_cancel_flag {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.generation_rx = None;
        self.prompt_busy = false;
        self.log_console("Build cancelled.".to_string());
    }

    fn run_prompt_draft(&mut self, design_id: Uuid, ctx: &egui::Context) {
        if self.generation_rx.is_some() {
            return;
        }
        if !self.ai_build_ready {
            self.set_err("AI build requires an AI provider key and Firecrawl key in Settings.");
            return;
        }
        let trimmed = self.prompt.trim().to_string();
        if trimmed.is_empty() {
            self.set_err("Describe what this board should do, then click Build schematic.");
            return;
        }

        if let Some(flag) = &self.build_cancel_flag {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.build_cancel_flag = Some(cancel.clone());

        self.prompt_busy = true;
        self.build_stage = "Planning...".into();
        self.build_warnings.clear();
        self.build_bom_diff = None;
        self.err = None;
        self.erc_note = None;

        let (tx, rx) = std::sync::mpsc::channel();
        let prompt = trimmed;
        let user_id = self.user_id;
        let state = self.state.clone();
        let pool = self.pool.clone();
        let repaint = ctx.clone();
        let stage = std::sync::Arc::new(std::sync::Mutex::new("Planning...".to_string()));
        let stage_cb = stage.clone();
        self.rt.spawn(async move {
            {
                let mut g = stage_cb.lock().unwrap();
                *g = "Research (Firecrawl)...".into();
            }
            let outcome = tokito::services::design_pipeline::build_design_from_prompt(
                &state,
                &pool,
                user_id,
                design_id,
                &prompt,
                true,
                Some(cancel.clone()),
            )
            .await
            .map(|out| {
                let batch = SchematicEditBatch::from_replace(
                    out.schematic.clone(),
                    "Replace schematic from AI build",
                );
                (out.schematic, out.erc_warnings, batch, out.warnings)
            })
            .map_err(|e| e.to_string());
            if !cancel.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = tx.send(outcome);
            }
            repaint.request_repaint();
        });

        self.generation_rx = Some(rx);
        ctx.request_repaint();
    }

    pub(crate) fn search_parts_catalog(&mut self) {
        let q = self.place_query.trim().to_string();
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.rt.block_on(async {
                tokito::store::parts::search(
                    &self.pool,
                    PartSearchParams {
                        q: if q.is_empty() { None } else { Some(q) },
                        limit: Some(50),
                    },
                )
                .await
            })
        }));
        match res {
            Ok(Ok(rows)) => {
                self.parts_hits = rows
                    .into_iter()
                    .map(|p| PartRow {
                        id: p.id,
                        mpn: p.mpn,
                        description: p.description,
                        package_name: p.package_name,
                    })
                    .collect();
            }
            Ok(Err(e)) => self.set_err(e),
            Err(_) => self.set_err("Component search failed unexpectedly"),
        }
    }

    pub(crate) fn lcsc_catalog_enabled(&self) -> bool {
        true
    }

    pub(crate) fn search_catalog(&mut self) {
        let q = self.place_query.trim().to_string();
        if q.is_empty() {
            self.catalog_hits.clear();
            return;
        }
        if self.state.nexar.is_none() && !self.lcsc_catalog_enabled() {
            self.catalog_hits.clear();
            return;
        }
        let state = self.state.clone();
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.rt.block_on(async move {
                tokito::services::catalog_search::search(&state, &q, 30).await
            })
        }));
        match res {
            Ok(Ok(hits)) => {
                self.catalog_hits = hits
                    .into_iter()
                    .map(|h| CatalogHit {
                        mpn: h.mpn,
                        manufacturer: h.manufacturer,
                        description: h.description,
                        package_name: h.package_name,
                        footprint_hint: h.footprint_hint,
                        datasheet_url: h.datasheet_url,
                        distributor: h.distributor,
                        sku: h.sku,
                        product_url: h.product_url,
                    })
                    .collect();
                let sources = match (self.lcsc_catalog_enabled(), self.state.nexar.is_some()) {
                    (true, true) => "LCSC + Nexar",
                    (true, false) => "LCSC",
                    (false, true) => "Nexar",
                    (false, false) => "none",
                };
                self.log_console(format!(
                    "Catalog: {} hit(s) ({sources})",
                    self.catalog_hits.len()
                ));
            }
            Ok(Err(e)) => self.set_err(e),
            Err(_) => self.set_err("Catalog search failed unexpectedly"),
        }
    }

    pub(crate) fn place_catalog_hit(&mut self, hit: &CatalogHit) {
        self.before_canvas_edit();
        let pool = self.pool.clone();
        let hit_clone = CatalogHit {
            mpn: hit.mpn.clone(),
            manufacturer: hit.manufacturer.clone(),
            description: hit.description.clone(),
            package_name: hit.package_name.clone(),
            footprint_hint: hit.footprint_hint.clone(),
            datasheet_url: hit.datasheet_url.clone(),
            distributor: hit.distributor.clone(),
            sku: hit.sku.clone(),
            product_url: hit.product_url.clone(),
        };
        let catalog_hit = tokito::models::CatalogPartHit {
            mpn: hit_clone.mpn.clone(),
            manufacturer: hit_clone.manufacturer.clone(),
            description: hit_clone.description.clone(),
            package_name: hit_clone.package_name.clone(),
            footprint_hint: hit_clone.footprint_hint.clone(),
            datasheet_url: hit_clone.datasheet_url.clone(),
            distributor: hit_clone.distributor.clone(),
            sku: hit_clone.sku.clone(),
            product_url: hit_clone.product_url.clone(),
            stock_qty: None,
            unit_price_cents: None,
            currency: None,
        };
        let part_id = match self.rt.block_on(async move {
            tokito::services::catalog_part::ensure_part_from_catalog_hit(&pool, &catalog_hit).await
        }) {
            Ok(part) => {
                self.part_cache.insert(part.id, part.mpn.clone());
                self.bom_dirty = true;
                Some(part.id)
            }
            Err(e) => {
                self.log_console(format!("Catalog part not saved: {e}"));
                None
            }
        };
        let (symbol_id, pin_layout) = self
            .base_symbols
            .as_ref()
            .map(|lib| lib.resolve_for_placement(None, Some(&hit.mpn), guess_prefix(&hit.mpn)))
            .unwrap_or((None, vec![]));
        let prefix = symbol_id
            .as_deref()
            .map(crate::base_symbols::BaseSymbolLibrary::refdes_prefix_for_library_id)
            .unwrap_or_else(|| guess_prefix(&hit.mpn));
        let ref_des = next_refdes(&self.editor.symbols, prefix);
        let pins: Vec<String> = if pin_layout.is_empty() {
            vec!["1".into(), "2".into()]
        } else {
            pin_layout.iter().map(|(n, _, _)| n.clone()).collect()
        };
        let footprint_ref = hit.footprint_hint.clone().or_else(|| {
            hit.package_name
                .as_ref()
                .map(|p| tokito::services::footprint_map::hint_from_package(p))
        });
        let pos = self
            .editor
            .screen_center_world()
            .unwrap_or_else(|| self.editor.snap_world(Pos2::new(240.0, 240.0)));
        let value = self.placement_value_for(symbol_id.as_deref(), &prefix, Some(&hit.mpn));
        self.editor.symbols.push(Sym {
            ref_des: ref_des.clone(),
            part_id,
            pos,
            rotation_deg: 0.0,
            pins,
            footprint_ref,
            symbol_id,
            pin_layout,
            value,
            fields: Default::default(),
        });
        self.editor.tool = CanvasTool::Select;
        self.mcad_viewer.invalidate();
        if let (Some(pid), Route::Studio { design_id }) = (part_id, self.route) {
            let _ = self.rt.block_on(async {
                tokito::store::bom::append_lines(
                    &self.pool,
                    design_id,
                    &[tokito::models::BomLineInput {
                        part_id: pid,
                        quantity: 1.0,
                        sort_order: 0,
                        notes: None,
                    }],
                )
                .await
            });
            self.bom_dirty = true;
        }
        self.log_console(format!(
            "Placed {} ({}{})",
            ref_des,
            hit.distributor,
            if part_id.is_some() {
                "; part saved to library"
            } else {
                ""
            }
        ));
    }

    fn drop_part_as_symbol(&mut self, part: &PartRow) {
        self.before_canvas_edit();
        let prefix = guess_prefix(&part.mpn);
        let ref_des = next_refdes(&self.editor.symbols, prefix);
        self.part_cache.insert(part.id, part.mpn.clone());
        let pos = self
            .editor
            .screen_center_world()
            .unwrap_or_else(|| self.editor.snap_world(Pos2::new(240.0, 240.0)));
        let (symbol_id, pin_layout) = self.resolve_symbol_for_instance(Some(part.id), prefix, None);
        let pins: Vec<String> = if pin_layout.is_empty() {
            vec!["1".to_string(), "2".to_string()]
        } else {
            pin_layout.iter().map(|(n, _, _)| n.clone()).collect()
        };
        let footprint_ref = part
            .package_name
            .as_ref()
            .map(|p| tokito::services::footprint_map::hint_from_package(p));
        let value = self.placement_value_for(symbol_id.as_deref(), prefix, Some(part.mpn.as_str()));
        self.editor.symbols.push(Sym {
            ref_des,
            part_id: Some(part.id),
            pos,
            rotation_deg: 0.0,
            pins,
            footprint_ref,
            symbol_id,
            pin_layout,
            value,
            fields: Default::default(),
        });
        self.mcad_viewer.invalidate();
    }

    fn delete_selected(&mut self) {
        if self.editor.delete_selected() {
            self.after_canvas_geometry_change();
        }
    }

    fn after_canvas_geometry_change(&mut self) {
        self.editor.refresh_wire_connectivity();
        self.editor.selected_erc_marker = None;
        self.editor.erc_marker_index = None;
        self.erc_violations.clear();
        self.erc_note = None;
        self.studio_dirty = true;
        self.mcad_viewer.invalidate();
    }
}

fn placement_value_for_symbols(
    base_symbols: Option<&crate::base_symbols::BaseSymbolLibrary>,
    symbol_id: Option<&str>,
    prefix: &str,
    part_mpn: Option<&str>,
) -> String {
    let from_lib = symbol_id.and_then(|id| base_symbols.map(|lib| lib.default_value_for(id)));
    match prefix {
        "R" | "C" | "L" | "D" => from_lib.unwrap_or_else(|| {
            crate::component_value::default_value_for_library_id(symbol_id.unwrap_or("Device:R"))
        }),
        _ => part_mpn
            .map(|s| s.to_string())
            .or(from_lib)
            .unwrap_or_else(|| {
                crate::component_value::default_value_for_library_id(symbol_id.unwrap_or(""))
            }),
    }
}

fn time_format_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("t{secs}")
}

include!("impl_eframe.rs");
