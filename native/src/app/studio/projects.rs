//! The projects experience — a two-level launcher built on `tokito_ui`.
//!
//! [`ProjectsView::Home`] lists recent projects; clicking one enters
//! [`ProjectsView::Designs`], that project's designs. A design opens the
//! studio. Ported from the locked Figma design lock.

use crate::app::{App, ProjectsSort, ProjectsView};
use tokito::models::{CreateProject, Design, PatchDesign, PatchProject};
use tokito_ui::components as c;
use tokito_ui::{icons, Tokens};

/// Max width of the centred content column.
const COLUMN_MAX: f32 = 1040.0;
/// Project / design card footprint.
const CARD_H: f32 = 130.0;
/// Gap between grid cards.
const GRID_GAP: f32 = 16.0;

impl App {
    pub(crate) fn ui_projects(&mut self, ctx: &egui::Context) {
        let theme = crate::theme::effective_theme(&self.settings_file.general.theme);
        let t = Tokens::from_name(&theme);

        // ---- top bar -------------------------------------------------------
        egui::TopBottomPanel::top("projects_topbar")
            .exact_height(50.0)
            .frame(
                egui::Frame::none()
                    .fill(t.bg_chrome)
                    .inner_margin(egui::Margin::symmetric(18.0, 0.0)),
            )
            .show(ctx, |ui| {
                ui.painter().hline(
                    ui.max_rect().x_range(),
                    ui.max_rect().bottom(),
                    egui::Stroke::new(1.0, t.border_soft),
                );
                ui.horizontal_centered(|ui| {
                    render_brand(ui, &t);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let glyph = if t.dark {
                            icons::ph::SUN
                        } else {
                            icons::ph::MOON
                        };
                        if c::icon_button(ui, &t, glyph, 32.0)
                            .on_hover_text("Toggle theme")
                            .clicked()
                        {
                            self.toggle_theme(ctx);
                        }
                    });
                });
            });

        // ---- content -------------------------------------------------------
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(t.bg))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let avail = ui.available_width();
                        let pad = ((avail - COLUMN_MAX) / 2.0).max(28.0);
                        let col_w = (avail - pad * 2.0).max(280.0);
                        ui.horizontal(|ui| {
                            ui.add_space(pad);
                            ui.allocate_ui_with_layout(
                                egui::vec2(col_w, 0.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.add_space(34.0);
                                    match self.projects_view {
                                        ProjectsView::Home => {
                                            self.render_projects_home(ui, &t, col_w)
                                        }
                                        ProjectsView::Designs => {
                                            self.render_designs_view(ui, &t, col_w)
                                        }
                                    }
                                    ui.add_space(60.0);
                                },
                            );
                        });
                    });
            });

        // Overlays: create-project / create-design modals.
        if self.new_project_form_open {
            let mut open = true;
            c::modal(ctx, &t, &mut open, "New project", 420.0, |ui| {
                self.new_project_modal_body(ui, &t);
            });
            if !open {
                self.new_project_form_open = false;
            }
        }
        if self.new_design_form_open {
            let mut open = true;
            c::modal(ctx, &t, &mut open, "New design", 420.0, |ui| {
                self.new_design_modal_body(ui, &t);
            });
            if !open {
                self.new_design_form_open = false;
            }
        }
        if let Some(pid) = self.renaming_project_id {
            let mut open = true;
            c::modal(ctx, &t, &mut open, "Rename project", 380.0, |ui| {
                self.rename_modal_body(ui, &t, RenameTarget::Project(pid));
            });
            if !open {
                self.renaming_project_id = None;
            }
        }
        if let Some(did) = self.renaming_design_id {
            let mut open = true;
            c::modal(ctx, &t, &mut open, "Rename design", 380.0, |ui| {
                self.rename_modal_body(ui, &t, RenameTarget::Design(did));
            });
            if !open {
                self.renaming_design_id = None;
            }
        }
        // The ⌘K quick switcher is driven from `impl_eframe` (key handling +
        // `show_projects_palette`), so it is not rendered again here.
    }

    // =======================================================================
    // Projects home
    // =======================================================================

    fn render_projects_home(&mut self, ui: &mut egui::Ui, t: &Tokens, col_w: f32) {
        c::page_header(
            ui,
            t,
            "Projects",
            "Open a recent project, or create a new one.",
        );
        ui.add_space(28.0);

        // 3-up grid: a new-project tile, then project cards.
        let card_w = ((col_w - GRID_GAP * 2.0) / 3.0).floor();
        let projects = self.projects.clone();
        let mut act: Option<(uuid::Uuid, ProjectAction)> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(GRID_GAP, GRID_GAP);

            if c::new_tile(ui, t, "New project", None, egui::vec2(card_w, CARD_H)).clicked() {
                self.new_project_form_open = true;
                self.new_project_name.clear();
            }

            for p in &projects {
                let a = project_card(ui, t, &p.name, &p.updated_at, p.id, card_w);
                if !matches!(a, ProjectAction::None) {
                    act = Some((p.id, a));
                }
            }
        });

        if let Some((pid, action)) = act {
            match action {
                ProjectAction::Enter => {
                    self.active_project_id = Some(pid);
                    self.projects_view = ProjectsView::Designs;
                    self.projects_search.clear();
                    self.reload_projects();
                    ui.ctx().request_repaint();
                }
                ProjectAction::Rename => {
                    self.project_rename_name = projects
                        .iter()
                        .find(|p| p.id == pid)
                        .map(|p| p.name.clone())
                        .unwrap_or_default();
                    self.renaming_project_id = Some(pid);
                }
                ProjectAction::Export => self.export_active_project_zip(pid),
                ProjectAction::None => {}
            }
        }
    }

    // =======================================================================
    // Designs view (one project)
    // =======================================================================

    fn render_designs_view(&mut self, ui: &mut egui::Ui, t: &Tokens, col_w: f32) {
        // back to the projects launcher
        let back = ui.add(
            egui::Label::new(icons::icon_text(
                icons::ph::ARROW_LEFT,
                15.0,
                "Projects",
                13.0,
                t.text_2,
            ))
            .sense(egui::Sense::click()),
        );
        if back.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if back.clicked() {
            self.projects_view = ProjectsView::Home;
            self.projects_list_dirty = true;
            self.refresh_projects();
            ui.ctx().request_repaint();
            return;
        }
        ui.add_space(14.0);

        let project_name = self
            .active_project_id
            .and_then(|pid| self.projects.iter().find(|p| p.id == pid))
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Project".to_string());
        let count = self.designs.len();
        let badge_text = format!("{} design{}", count, if count == 1 { "" } else { "s" });
        // Title + count badge on one line.
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&project_name)
                    .text_style(egui::TextStyle::Heading)
                    .strong()
                    .color(t.text),
            );
            ui.add_space(10.0);
            c::badge(ui, t, &badge_text);
        });
        ui.add_space(26.0);

        // section header row: "Designs" + search + sort
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Designs")
                    .text_style(egui::TextStyle::Name("h2".into()))
                    .strong()
                    .color(t.text),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                self.render_projects_sort_combo(ui);
                ui.add_space(8.0);
                let _ = c::search_field(
                    ui,
                    t,
                    "designs_search",
                    &mut self.projects_search,
                    "Search designs",
                    230.0,
                );
            });
        });
        ui.add_space(16.0);

        let rows = self.filtered_project_designs();
        let pinned = self.projects_pinned.clone();
        let card_w = ((col_w - GRID_GAP * 2.0) / 3.0).floor();
        let mut act: Option<(uuid::Uuid, DesignAction)> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(GRID_GAP, GRID_GAP);

            if c::new_tile(
                ui,
                t,
                "New design",
                Some("blank · or AI-drafted"),
                egui::vec2(card_w, CARD_H),
            )
            .clicked()
            {
                self.new_design_form_open = true;
                if self.new_design_name.trim().is_empty() {
                    self.new_design_name = "New design".to_string();
                }
            }

            for d in &rows {
                let a = design_card(ui, t, d, card_w, pinned.contains(&d.id));
                if !matches!(a, DesignAction::None) {
                    act = Some((d.id, a));
                }
            }
        });

        if rows.is_empty() && self.designs.is_empty() && !self.new_design_form_open {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("No designs yet — create one above.")
                    .size(13.0)
                    .color(t.text_3),
            );
        }

        if let Some((did, action)) = act {
            match action {
                DesignAction::Open => self.open_design(did),
                DesignAction::Rename => {
                    self.design_rename_name = rows
                        .iter()
                        .find(|d| d.id == did)
                        .map(|d| d.name.clone())
                        .unwrap_or_default();
                    self.renaming_design_id = Some(did);
                }
                DesignAction::TogglePin => {
                    if !self.projects_pinned.remove(&did) {
                        self.projects_pinned.insert(did);
                    }
                }
                DesignAction::None => {}
            }
        }
    }

    // =======================================================================
    // Inline forms
    // =======================================================================

    fn new_project_modal_body(&mut self, ui: &mut egui::Ui, t: &Tokens) {
        let w = ui.available_width();
        field_label(ui, t, "Name");
        c::text_input(
            ui,
            t,
            "new_project_name",
            &mut self.new_project_name,
            "Untitled project",
            w,
        );
        ui.add_space(14.0);
        c::toggle(
            ui,
            t,
            &mut self.new_project_embedded_db,
            "Isolated database",
        );
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("Stores this project's data in its own local database.")
                .size(12.0)
                .color(t.text_3),
        );
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            if c::text_button(ui, t, c::ButtonKind::Primary, "Create project", 34.0).clicked() {
                self.create_project();
                ui.ctx().request_repaint();
            }
            ui.add_space(8.0);
            if c::text_button(ui, t, c::ButtonKind::Secondary, "Cancel", 34.0).clicked() {
                self.new_project_form_open = false;
            }
        });
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Already have a project archive?")
                    .size(12.0)
                    .color(t.text_3),
            );
            ui.add_space(6.0);
            if c::text_button(ui, t, c::ButtonKind::Secondary, "Import .zip", 30.0).clicked() {
                self.new_project_form_open = false;
                self.import_project_zip();
            }
        });
    }

    fn new_design_modal_body(&mut self, ui: &mut egui::Ui, t: &Tokens) {
        let w = ui.available_width();
        field_label(ui, t, "Name");
        c::text_input(
            ui,
            t,
            "new_design_name",
            &mut self.new_design_name,
            "New design",
            w,
        );
        ui.add_space(12.0);
        field_label(ui, t, "Description");
        c::text_input(
            ui,
            t,
            "new_design_desc",
            &mut self.new_design_desc,
            "Optional",
            w,
        );
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            if c::text_button(ui, t, c::ButtonKind::Primary, "Create design", 34.0).clicked() {
                self.create_design();
            }
            ui.add_space(8.0);
            if c::text_button(ui, t, c::ButtonKind::Secondary, "Cancel", 34.0).clicked() {
                self.new_design_form_open = false;
            }
        });
    }

    fn rename_modal_body(&mut self, ui: &mut egui::Ui, t: &Tokens, target: RenameTarget) {
        let w = ui.available_width();
        field_label(ui, t, "Name");
        match target {
            RenameTarget::Project(_) => {
                c::text_input(
                    ui,
                    t,
                    "project_rename",
                    &mut self.project_rename_name,
                    "Project name",
                    w,
                );
            }
            RenameTarget::Design(_) => {
                c::text_input(
                    ui,
                    t,
                    "design_rename",
                    &mut self.design_rename_name,
                    "Design name",
                    w,
                );
            }
        }
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            if c::text_button(ui, t, c::ButtonKind::Primary, "Save", 34.0).clicked() {
                match target {
                    RenameTarget::Project(pid) => self.rename_active_project(pid),
                    RenameTarget::Design(did) => self.rename_design_from_projects(did),
                }
            }
            ui.add_space(8.0);
            if c::text_button(ui, t, c::ButtonKind::Secondary, "Cancel", 34.0).clicked() {
                match target {
                    RenameTarget::Project(_) => self.renaming_project_id = None,
                    RenameTarget::Design(_) => self.renaming_design_id = None,
                }
            }
        });
    }

    // =======================================================================
    // Data operations (UI-agnostic)
    // =======================================================================

    fn toggle_theme(&mut self, ctx: &egui::Context) {
        let resolved = crate::theme::effective_theme(&self.settings_file.general.theme);
        let next = if resolved == "dark" { "light" } else { "dark" };
        self.settings_file.general.theme = next.to_string();
        self.ui_tokens = crate::theme::tokens_for(next);
        crate::theme::apply_with_theme(ctx, next);
        let _ = tokito::settings::save_file(&self.settings_file);
    }

    fn create_project(&mut self) {
        let name = self.new_project_name.trim().to_string();
        if name.is_empty() {
            self.set_err("Project name is required");
            return;
        }
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.rt.block_on(async {
                tokito::store::projects::create(&self.global_pool, CreateProject { name }).await
            })
        }));
        match res {
            Ok(Ok(p)) => {
                self.new_project_name.clear();
                if self.new_project_embedded_db {
                    let ws = std::path::PathBuf::from(&p.workspace_path);
                    let mut meta = tokito::project_toml::read(&ws).unwrap_or_default();
                    meta.id = Some(p.id);
                    meta.name = p.name.clone();
                    meta.slug = p.slug.clone();
                    meta.database.mode = "embedded".into();
                    let _ = tokito::project_toml::write(&ws, &meta);
                }
                self.active_project_id = Some(p.id);
                self.projects_list_dirty = true;
                self.new_project_form_open = false;
                self.refresh_projects();
                self.reload_projects();
                self.toast_ok("Project created");
            }
            Ok(Err(e)) => self.set_err(e.to_string()),
            Err(_) => self.set_err("Project creation failed unexpectedly"),
        }
    }

    fn create_design(&mut self) {
        let name = self.new_design_name.trim().to_string();
        if name.is_empty() {
            self.set_err("Design name is required");
            return;
        }
        let desc = self.new_design_desc.trim().to_string();
        let project_id = self.active_project_id;
        if let Some(pid) = project_id {
            self.connect_project_db_for_project(pid);
        }
        let res = self.rt.block_on(async {
            tokito::store::designs::create(
                &self.pool,
                tokito::models::CreateDesign {
                    name,
                    description: if desc.is_empty() { None } else { Some(desc) },
                    project_id,
                },
                self.user_id,
            )
            .await
        });
        match res {
            Ok(d) => {
                self.new_design_name = "New design".to_string();
                self.new_design_desc.clear();
                self.new_design_form_open = false;
                self.open_design(d.id);
            }
            Err(e) => self.set_err(e),
        }
    }

    fn render_projects_sort_combo(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("projects_sort")
            .width(150.0)
            .selected_text(match self.projects_sort {
                ProjectsSort::UpdatedDesc => "Updated desc",
                ProjectsSort::UpdatedAsc => "Updated asc",
                ProjectsSort::NameAsc => "Name A-Z",
                ProjectsSort::NameDesc => "Name Z-A",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.projects_sort,
                    ProjectsSort::UpdatedDesc,
                    "Updated desc",
                );
                ui.selectable_value(
                    &mut self.projects_sort,
                    ProjectsSort::UpdatedAsc,
                    "Updated asc",
                );
                ui.selectable_value(&mut self.projects_sort, ProjectsSort::NameAsc, "Name A-Z");
                ui.selectable_value(&mut self.projects_sort, ProjectsSort::NameDesc, "Name Z-A");
            });
    }

    fn filtered_project_designs(&self) -> Vec<Design> {
        let q = self.projects_search.trim().to_lowercase();
        let mut rows: Vec<_> = self
            .designs
            .iter()
            .filter(|d| {
                q.is_empty()
                    || d.name.to_lowercase().contains(&q)
                    || d.description
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&q))
                        .unwrap_or(false)
            })
            .cloned()
            .collect();
        match self.projects_sort {
            ProjectsSort::NameAsc => rows.sort_by(|a, b| a.name.cmp(&b.name)),
            ProjectsSort::NameDesc => rows.sort_by(|a, b| b.name.cmp(&a.name)),
            ProjectsSort::UpdatedAsc => rows.sort_by_key(|a| a.updated_at),
            ProjectsSort::UpdatedDesc => rows.sort_by_key(|r| std::cmp::Reverse(r.updated_at)),
        }
        // Pinned designs float to the top, keeping the chosen sort within each
        // group (`partition` preserves order).
        let (mut pinned, mut rest): (Vec<_>, Vec<_>) = rows
            .into_iter()
            .partition(|d| self.projects_pinned.contains(&d.id));
        pinned.append(&mut rest);
        pinned
    }

    fn rename_active_project(&mut self, project_id: uuid::Uuid) {
        let name = self.project_rename_name.trim().to_string();
        if name.is_empty() {
            self.set_err("Project name is required");
            return;
        }
        let res = self.rt.block_on(async {
            tokito::store::projects::patch(
                &self.global_pool,
                project_id,
                PatchProject { name: Some(name) },
            )
            .await
        });
        match res {
            Ok(project) => {
                if let Some(existing) = self.projects.iter_mut().find(|p| p.id == project_id) {
                    *existing = project.clone();
                }
                if self.project_db_project_id == Some(project_id) {
                    let _ = self.rt.block_on(async {
                        tokito::store::projects::upsert_existing(&self.pool, &project).await
                    });
                }
                self.renaming_project_id = None;
                self.project_rename_name.clear();
                self.projects_list_dirty = true;
                self.refresh_projects();
                self.toast_ok("Project renamed");
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }

    fn rename_design_from_projects(&mut self, design_id: uuid::Uuid) {
        let name = self.design_rename_name.trim().to_string();
        if name.is_empty() {
            self.set_err("Design name is required");
            return;
        }
        let res = self.rt.block_on(async {
            tokito::store::designs::patch(
                &self.pool,
                design_id,
                PatchDesign {
                    name: Some(name),
                    description: None,
                    notes: None,
                },
            )
            .await
        });
        match res {
            Ok(row) => {
                if let Some(existing) = self.designs.iter_mut().find(|d| d.id == design_id) {
                    *existing = row.clone();
                }
                self.renaming_design_id = None;
                self.design_rename_name.clear();
                self.toast_ok("Design renamed");
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }

    fn export_active_project_zip(&mut self, pid: uuid::Uuid) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("project.zip")
            .save_file()
        {
            // Connect to the target project's database so the export reads the
            // right cluster (the card may not be the active project).
            self.connect_project_db_for_project(pid);
            let uid = self.user_id;
            let db_url = self.db_url.clone();
            let pg_ver = self.settings_file.database.pg_embed_version;
            let res = self
                .rt
                .block_on(tokito::services::project_archive::export_project_zip(
                    &self.pool,
                    pid,
                    uid,
                    &path,
                    Some(&db_url),
                    pg_ver,
                ));
            match res {
                Ok(()) => self.toast_ok("Project exported"),
                Err(e) => self.set_err(e.to_string()),
            }
        }
    }

    fn import_project_zip(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Zip", &["zip"])
            .pick_file()
        {
            let uid = self.user_id;
            let res = self
                .rt
                .block_on(tokito::services::project_archive::import_project_zip(
                    &self.global_pool,
                    &path,
                    uid,
                ));
            match res {
                Ok(id) => {
                    self.active_project_id = Some(id);
                    self.refresh_projects();
                    self.reload_projects();
                    self.toast_ok("Project imported");
                }
                Err(e) => self.set_err(e.to_string()),
            }
        }
    }
}

// ===========================================================================
// Free-function widgets (compose tokito_ui primitives into domain cards)
// ===========================================================================

fn render_brand(ui: &mut egui::Ui, t: &Tokens) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(7.0), t.accent);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icons::ph::CIRCUITRY,
            icons::font(14.0),
            t.accent_ink,
        );
        ui.add_space(9.0);
        ui.label(
            egui::RichText::new("Tokito")
                .size(14.5)
                .strong()
                .color(t.text),
        );
    });
}

/// A small uppercase field label inside a form.
fn field_label(ui: &mut egui::Ui, t: &Tokens, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .text_style(egui::TextStyle::Name("h3".into()))
            .color(t.text_2),
    );
    ui.add_space(5.0);
}

/// A small rounded icon chip — the folder / schematic glyph on a card.
fn icon_chip(ui: &mut egui::Ui, t: &Tokens, glyph: &str) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, t.rounding_sm(), t.bg);
    ui.painter().rect_stroke(
        rect.shrink(0.5),
        t.rounding_sm(),
        egui::Stroke::new(1.0, t.border),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        icons::font(16.0),
        t.text_2,
    );
}

/// Format a timestamp as a short `YYYY-MM-DD` date.
fn short_date(ts: &impl ToString) -> String {
    let s = ts.to_string();
    s.get(..10).map(String::from).unwrap_or(s)
}

/// Which entity a rename modal is editing.
#[derive(Clone, Copy)]
enum RenameTarget {
    Project(uuid::Uuid),
    Design(uuid::Uuid),
}

/// What a project card click resolved to.
enum ProjectAction {
    None,
    /// Body clicked — open this project's designs.
    Enter,
    /// Kebab → Rename.
    Rename,
    /// Kebab → Export .zip.
    Export,
}

/// What a design card click resolved to.
enum DesignAction {
    None,
    /// Body clicked — open the design in the studio.
    Open,
    /// Kebab → Rename.
    Rename,
    /// Kebab → Pin / Unpin.
    TogglePin,
}

/// A project card: folder chip + kebab (Rename / Export), name, updated date.
fn project_card(
    ui: &mut egui::Ui,
    t: &Tokens,
    name: &str,
    updated_at: &impl ToString,
    project_id: uuid::Uuid,
    width: f32,
) -> ProjectAction {
    let mut action = ProjectAction::None;
    let resp = c::card(ui, t, egui::vec2(width, CARD_H), |ui| {
        ui.horizontal(|ui| {
            icon_chip(ui, t, icons::ph::FOLDER);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                c::menu_button(
                    ui,
                    t,
                    ("project_kebab", project_id),
                    icons::ph::DOTS_THREE_VERTICAL,
                    26.0,
                    |ui| {
                        if c::menu_item(ui, t, icons::ph::PENCIL_SIMPLE, "Rename") {
                            action = ProjectAction::Rename;
                        }
                        if c::menu_item(ui, t, icons::ph::DOWNLOAD_SIMPLE, "Export .zip") {
                            action = ProjectAction::Export;
                        }
                    },
                );
            });
        });
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(crate::util::truncate_ui_chars(name, 26))
                .size(14.5)
                .strong()
                .color(t.text),
        );
        ui.add_space(8.0);
        ui.label(icons::icon_text(
            icons::ph::CLOCK,
            13.0,
            &short_date(updated_at),
            12.0,
            t.text_3,
        ));
    });
    if resp.clicked() && matches!(action, ProjectAction::None) {
        action = ProjectAction::Enter;
    }
    action
}

/// A design card: schematic chip + kebab (Rename / Pin), name, updated date.
fn design_card(
    ui: &mut egui::Ui,
    t: &Tokens,
    d: &Design,
    width: f32,
    is_pinned: bool,
) -> DesignAction {
    let mut action = DesignAction::None;
    let resp = c::card(ui, t, egui::vec2(width, CARD_H), |ui| {
        ui.horizontal(|ui| {
            icon_chip(ui, t, icons::ph::TREE_STRUCTURE);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                c::menu_button(
                    ui,
                    t,
                    ("design_kebab", d.id),
                    icons::ph::DOTS_THREE_VERTICAL,
                    26.0,
                    |ui| {
                        if c::menu_item(ui, t, icons::ph::PENCIL_SIMPLE, "Rename") {
                            action = DesignAction::Rename;
                        }
                        let pin_label = if is_pinned { "Unpin" } else { "Pin" };
                        if c::menu_item(ui, t, icons::ph::PUSH_PIN, pin_label) {
                            action = DesignAction::TogglePin;
                        }
                    },
                );
                if is_pinned {
                    ui.add_space(3.0);
                    ui.label(icons::icon(icons::ph::PUSH_PIN, 13.0, t.accent));
                }
            });
        });
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(crate::util::truncate_ui_chars(&d.name, 26))
                .size(14.5)
                .strong()
                .color(t.text),
        );
        ui.add_space(8.0);
        ui.label(icons::icon_text(
            icons::ph::CLOCK,
            13.0,
            &short_date(&d.updated_at.to_rfc3339()),
            12.0,
            t.text_3,
        ));
    });
    if resp.clicked() && matches!(action, DesignAction::None) {
        action = DesignAction::Open;
    }
    action
}
