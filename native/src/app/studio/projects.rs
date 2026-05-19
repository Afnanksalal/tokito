use std::collections::HashSet;

use crate::app::{App, ProjectsSort};
use crate::ui::{tokens::UiTokens, TypeRamp};
use tokito::models::{CreateProject, Design, PatchDesign, PatchProject};

impl App {
    pub(crate) fn ui_projects(&mut self, ctx: &egui::Context) {
        let tokens = self.ui_tokens;
        let ty = TypeRamp::default();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(tokens.bg_app))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
                egui::ScrollArea::vertical()
                    .id_salt("projects_page_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.add_space(12.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(12.0, 4.0);
                            ui.label(ty.title("Tokito").color(tokens.text_primary));
                            ui.label(
                                ty.small_weak(
                                    "Projects group designs. Pick a project, then open or create a schematic.",
                                )
                                .color(tokens.text_muted),
                            );
                        });
                        ui.add_space(10.0);

                        let width = ui.available_width().max(0.0);
                        ui.set_width(width);
                        ui.set_max_width(width);

                        if width >= 1120.0 {
                            let gap = 12.0;
                            let left_w = 280.0;
                            let right_w = 260.0;
                            let center_w = (width - left_w - right_w - gap * 2.0).max(0.0);
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                ui.vertical(|ui| {
                                    ui.set_width(left_w);
                                    ui.set_max_width(left_w);
                                    self.render_project_form_column(ui, &tokens, &ty);
                                });
                                ui.add_space(gap);
                                ui.vertical(|ui| {
                                    ui.set_width(center_w);
                                    ui.set_max_width(center_w);
                                    self.render_designs_panel(ui, &tokens, &ty);
                                });
                                ui.add_space(gap);
                                ui.vertical(|ui| {
                                    ui.set_width(right_w);
                                    ui.set_max_width(right_w);
                                    self.render_project_list_panel(ui, &tokens, &ty);
                                });
                            });
                        } else if width >= 760.0 {
                            let gap = 12.0;
                            let left_w = (width * 0.34).clamp(230.0, 280.0);
                            let center_w = (width - left_w - gap).max(0.0);
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                ui.vertical(|ui| {
                                    ui.set_width(left_w);
                                    ui.set_max_width(left_w);
                                    self.render_project_form_column(ui, &tokens, &ty);
                                });
                                ui.add_space(gap);
                                ui.vertical(|ui| {
                                    ui.set_width(center_w);
                                    ui.set_max_width(center_w);
                                    self.render_designs_panel(ui, &tokens, &ty);
                                });
                            });
                            ui.add_space(12.0);
                            self.render_project_list_panel(ui, &tokens, &ty);
                        } else {
                            self.render_project_form_column(ui, &tokens, &ty);
                            ui.add_space(12.0);
                            self.render_designs_panel(ui, &tokens, &ty);
                            ui.add_space(12.0);
                            self.render_project_list_panel(ui, &tokens, &ty);
                        }
                    });
            });
    }

    fn render_project_form_column(&mut self, ui: &mut egui::Ui, tokens: &UiTokens, ty: &TypeRamp) {
        self.render_new_project_card(ui, tokens, ty);
        ui.add_space(12.0);
        self.render_new_design_card(ui, tokens, ty);
    }

    fn render_new_project_card(&mut self, ui: &mut egui::Ui, tokens: &UiTokens, ty: &TypeRamp) {
        crate::ui::layout::content_card(ui, tokens, |ui| {
            ui.label(ty.section("Projects").color(tokens.text_primary));
            ui.add_space(6.0);
            ui.label(egui::RichText::new("New project").small().weak());
            ui.add(
                egui::TextEdit::singleline(&mut self.new_project_name)
                    .desired_width(ui.available_width().max(0.0)),
            );
            ui.checkbox(&mut self.new_project_embedded_db, "Isolated DB");
            ui.label(
                egui::RichText::new("Stores this project's data in its own local database.")
                    .small()
                    .weak()
                    .color(tokens.text_muted),
            );
            ui.add_space(8.0);
            if crate::ui::widgets::primary_button_full(ui, tokens, "Create project").clicked() {
                let name = self.new_project_name.trim().to_string();
                if name.is_empty() {
                    self.set_err("Project name is required");
                } else {
                    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        self.rt.block_on(async {
                            tokito::store::projects::create(
                                &self.global_pool,
                                CreateProject { name },
                            )
                            .await
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
                            self.refresh_projects();
                            self.reload_projects();
                            self.toast_ok("Project created");
                        }
                        Ok(Err(e)) => self.set_err(e.to_string()),
                        Err(_) => self.set_err("Project creation failed unexpectedly"),
                    }
                }
            }
        });
    }

    fn render_new_design_card(&mut self, ui: &mut egui::Ui, tokens: &UiTokens, ty: &TypeRamp) {
        crate::ui::layout::content_card(ui, tokens, |ui| {
            ui.label(ty.section("New design").color(tokens.text_primary));
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Name").small().weak());
            ui.add(
                egui::TextEdit::singleline(&mut self.new_design_name)
                    .desired_width(ui.available_width().max(0.0)),
            );
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Description").small().weak());
            ui.add(
                egui::TextEdit::singleline(&mut self.new_design_desc)
                    .desired_width(ui.available_width().max(0.0)),
            );
            ui.add_space(12.0);
            if crate::ui::widgets::primary_button_full(ui, tokens, "Create design").clicked() {
                let name = self.new_design_name.trim().to_string();
                if name.is_empty() {
                    self.set_err("Name is required");
                } else {
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
                            self.open_design(d.id);
                        }
                        Err(e) => self.set_err(e),
                    }
                }
            }
        });
    }

    fn render_designs_panel(&mut self, ui: &mut egui::Ui, tokens: &UiTokens, ty: &TypeRamp) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);
            ui.label(ty.section("Your designs").color(tokens.text_primary));
            if let Some(pid) = self.active_project_id {
                if let Some(p) = self.projects.iter().find(|p| p.id == pid) {
                    ui.label(
                        egui::RichText::new(format!("in {}", p.name))
                            .small()
                            .weak()
                            .color(tokens.text_muted),
                    );
                }
            }
        });
        ui.add_space(4.0);

        if ui.available_width() < 520.0 {
            let _ = crate::ui::layout::search_field(
                ui,
                &mut self.projects_search,
                "Search name or description",
            );
            self.render_projects_sort_combo(ui);
        } else {
            ui.horizontal(|ui| {
                let _ = crate::ui::layout::search_field(
                    ui,
                    &mut self.projects_search,
                    "Search name or description",
                );
                self.render_projects_sort_combo(ui);
            });
        }
        ui.add_space(8.0);

        let rows = self.filtered_project_designs();
        let pinned: Vec<_> = rows
            .iter()
            .filter(|d| self.projects_pinned.contains(&d.id))
            .cloned()
            .collect();
        let recent_ids: HashSet<_> = self.recent_design_ids.iter().copied().collect();
        let recent: Vec<_> = rows
            .iter()
            .filter(|d| recent_ids.contains(&d.id) && !self.projects_pinned.contains(&d.id))
            .cloned()
            .collect();
        let mut seen = HashSet::new();
        for d in &pinned {
            seen.insert(d.id);
        }
        for d in &recent {
            seen.insert(d.id);
        }
        let others: Vec<_> = rows.into_iter().filter(|d| !seen.contains(&d.id)).collect();

        if pinned.is_empty() && recent.is_empty() && others.is_empty() && self.designs.is_empty() {
            crate::ui::layout::empty_state(
                ui,
                tokens,
                "No designs in this project. Create one on the left.",
            );
            return;
        }

        self.render_design_section(ui, tokens, "Pinned", &pinned);
        self.render_design_section(ui, tokens, "Recent", &recent);
        self.render_design_section(ui, tokens, "All designs", &others);
    }

    fn render_projects_sort_combo(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("projects_sort")
            .width(ui.available_width().clamp(120.0, 160.0))
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
        rows
    }

    fn render_design_section(
        &mut self,
        ui: &mut egui::Ui,
        tokens: &UiTokens,
        title: &str,
        designs: &[Design],
    ) {
        if designs.is_empty() {
            return;
        }
        crate::ui::layout::subsection(ui, tokens, title);
        for d in designs {
            self.render_design_card(ui, tokens, d);
            ui.add_space(8.0);
        }
    }

    fn render_design_card(&mut self, ui: &mut egui::Ui, tokens: &UiTokens, d: &Design) {
        crate::ui::layout::content_card(ui, tokens, |ui| {
            if self.renaming_design_id == Some(d.id) {
                self.render_design_rename_row(ui, tokens, d);
                return;
            }
            let narrow = ui.available_width() < 540.0;
            if narrow {
                ui.horizontal_wrapped(|ui| {
                    let mut pin = self.projects_pinned.contains(&d.id);
                    if ui.checkbox(&mut pin, "Pinned").changed() {
                        if pin {
                            self.projects_pinned.insert(d.id);
                        } else {
                            self.projects_pinned.remove(&d.id);
                        }
                    }
                });
                ui.add_space(4.0);
                self.render_design_card_text(ui, tokens, d);
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                    if crate::ui::widgets::primary_button(ui, tokens, "Open").clicked() {
                        self.open_design(d.id);
                    }
                    if crate::ui::widgets::secondary_button(ui, tokens, "Rename").clicked() {
                        self.start_renaming_design(d);
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    let mut pin = self.projects_pinned.contains(&d.id);
                    if ui.checkbox(&mut pin, "").changed() {
                        if pin {
                            self.projects_pinned.insert(d.id);
                        } else {
                            self.projects_pinned.remove(&d.id);
                        }
                    }
                    ui.add_space(2.0);
                    let text_w = (ui.available_width() - 224.0).max(120.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(text_w, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| self.render_design_card_text(ui, tokens, d),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if crate::ui::widgets::primary_button(ui, tokens, "Open").clicked() {
                            self.open_design(d.id);
                        }
                        if crate::ui::widgets::secondary_button(ui, tokens, "Rename").clicked() {
                            self.start_renaming_design(d);
                        }
                    });
                });
            }
        });
    }

    fn start_renaming_design(&mut self, design: &Design) {
        self.renaming_design_id = Some(design.id);
        self.design_rename_name = design.name.clone();
    }

    fn render_design_rename_row(&mut self, ui: &mut egui::Ui, tokens: &UiTokens, d: &Design) {
        ui.label(
            egui::RichText::new("Rename design")
                .strong()
                .color(tokens.text_primary),
        );
        let resp = ui.add(
            egui::TextEdit::singleline(&mut self.design_rename_name)
                .desired_width(ui.available_width().max(0.0)),
        );
        let submit = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            let save = crate::ui::widgets::primary_button(ui, tokens, "Save").clicked();
            if save || submit {
                self.rename_design_from_projects(d.id);
            }
            if crate::ui::widgets::secondary_button(ui, tokens, "Cancel").clicked() {
                self.renaming_design_id = None;
                self.design_rename_name.clear();
            }
        });
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
                if self.design.as_ref().is_some_and(|d| d.id == design_id) {
                    self.design = Some(row.clone());
                    self.design_edit_name = row.name;
                }
                self.renaming_design_id = None;
                self.design_rename_name.clear();
                self.toast_ok("Design renamed");
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }

    fn render_design_card_text(&self, ui: &mut egui::Ui, tokens: &UiTokens, d: &Design) {
        ui.label(egui::RichText::new(&d.name).strong().size(14.0));
        if let Some(desc) = &d.description {
            ui.label(
                egui::RichText::new(crate::util::truncate_ui_chars(desc, 140))
                    .weak()
                    .small(),
            );
        }
        let ts = d.updated_at.to_rfc3339();
        let short = ts.get(..10).map(String::from).unwrap_or_else(|| ts.clone());
        ui.label(
            egui::RichText::new(format!("Updated {short}"))
                .small()
                .weak()
                .color(tokens.text_muted),
        );
    }

    fn render_project_list_panel(&mut self, ui: &mut egui::Ui, tokens: &UiTokens, ty: &TypeRamp) {
        crate::ui::layout::content_card(ui, tokens, |ui| {
            ui.label(ty.section("Project list").color(tokens.text_primary));
            ui.add_space(6.0);
            egui::ScrollArea::vertical()
                .id_salt("project_list")
                .max_height(360.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for p in self.projects.clone() {
                        let active = self.active_project_id == Some(p.id);
                        let max_chars = ((ui.available_width() / 7.5).floor() as usize)
                            .saturating_sub(2)
                            .clamp(8, 34);
                        let label = crate::util::truncate_ui_chars(&p.name, max_chars);
                        if ui
                            .selectable_label(active, egui::RichText::new(label).size(13.0))
                            .clicked()
                        {
                            self.active_project_id = Some(p.id);
                            self.reload_projects();
                        }
                    }
                });
            ui.add_space(8.0);
            if let Some(pid) = self.active_project_id {
                if let Some(project) = self.projects.iter().find(|p| p.id == pid).cloned() {
                    ui.separator();
                    if self.renaming_project_id == Some(pid) {
                        ui.label(
                            egui::RichText::new("Rename project")
                                .strong()
                                .color(tokens.text_primary),
                        );
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut self.project_rename_name)
                                .desired_width(ui.available_width().max(0.0)),
                        );
                        let submit =
                            resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                            let save =
                                crate::ui::widgets::primary_button(ui, tokens, "Save").clicked();
                            if save || submit {
                                self.rename_active_project(pid);
                            }
                            if crate::ui::widgets::secondary_button(ui, tokens, "Cancel").clicked()
                            {
                                self.renaming_project_id = None;
                                self.project_rename_name.clear();
                            }
                        });
                    } else {
                        ui.label(
                            egui::RichText::new(crate::util::truncate_ui_chars(&project.name, 36))
                                .strong()
                                .color(tokens.text_primary),
                        );
                        if crate::ui::widgets::secondary_button(ui, tokens, "Rename project")
                            .clicked()
                        {
                            self.renaming_project_id = Some(pid);
                            self.project_rename_name = project.name;
                        }
                    }
                    ui.add_space(8.0);
                }
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                    if crate::ui::widgets::secondary_button(ui, tokens, "Export project zip")
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("project.zip")
                            .save_file()
                        {
                            let uid = self.user_id;
                            let db_url = self.db_url.clone();
                            let pg_ver = self.settings_file.database.pg_embed_version;
                            let res = self.rt.block_on(
                                tokito::services::project_archive::export_project_zip(
                                    &self.pool,
                                    pid,
                                    uid,
                                    &path,
                                    Some(&db_url),
                                    pg_ver,
                                ),
                            );
                            match res {
                                Ok(()) => self.toast_ok("Project exported"),
                                Err(e) => self.set_err(e.to_string()),
                            }
                        }
                    }
                    if crate::ui::widgets::secondary_button(ui, tokens, "Import project zip")
                        .clicked()
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Zip", &["zip"])
                            .pick_file()
                        {
                            let uid = self.user_id;
                            let res = self.rt.block_on(
                                tokito::services::project_archive::import_project_zip(
                                    &self.global_pool,
                                    &path,
                                    uid,
                                ),
                            );
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
                });
            }
        });
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
}
