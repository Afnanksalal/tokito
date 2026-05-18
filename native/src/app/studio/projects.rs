use std::collections::HashSet;

use crate::app::{App, ProjectsSort};
use tokito::models::CreateProject;

impl App {
    pub(crate) fn ui_projects(&mut self, ctx: &egui::Context) {
        let tokens = self.ui_tokens;
        let ty = crate::ui::TypeRamp::default();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(tokens.bg_app))
            .show(ctx, |ui| {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(ty.title("Tokito").color(tokens.text_primary));
                    ui.add_space(12.0);
                    ui.label(
                        ty.small_weak(
                            "Projects group designs — pick a project, then open or create a schematic",
                        )
                        .color(tokens.text_muted),
                    );
                });
                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_max_width(280.0);
                        crate::ui::layout::content_card(ui, &tokens, |ui| {
                            ui.label(ty.section("Projects").color(tokens.text_primary));
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("New project").small().weak());
                            ui.text_edit_singleline(&mut self.new_project_name);
                            ui.checkbox(
                                &mut self.new_project_embedded_db,
                                "Isolated database (per-project Postgres)",
                            );
                            ui.add_space(8.0);
                            if crate::ui::widgets::primary_button(ui, &tokens, "Create project")
                                .clicked()
                            {
                                let name = self.new_project_name.trim().to_string();
                                if name.is_empty() {
                                    self.set_err("Project name is required");
                                } else {
                                    let res = self.rt.block_on(async {
                                        tokito::store::projects::create(
                                            &self.pool,
                                            CreateProject { name },
                                        )
                                        .await
                                    });
                                    match res {
                                        Ok(p) => {
                                            self.new_project_name.clear();
                                            if self.new_project_embedded_db {
                                                let ws = std::path::PathBuf::from(&p.workspace_path);
                                                let mut meta =
                                                    tokito::project_toml::read(&ws).unwrap_or_default();
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
                                        Err(e) => self.set_err(e.to_string()),
                                    }
                                }
                            }
                        });
                        ui.add_space(12.0);
                        crate::ui::layout::content_card(ui, &tokens, |ui| {
                            ui.label(ty.section("New design").color(tokens.text_primary));
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Name").small().weak());
                            ui.text_edit_singleline(&mut self.new_design_name);
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("Description").small().weak());
                            ui.text_edit_singleline(&mut self.new_design_desc);
                            ui.add_space(12.0);
                            if crate::ui::widgets::primary_button(ui, &tokens, "Create design")
                                .clicked()
                            {
                                let name = self.new_design_name.trim().to_string();
                                if name.is_empty() {
                                    self.set_err("Name is required");
                                } else {
                                    let desc = self.new_design_desc.trim().to_string();
                                    let project_id = self.active_project_id;
                                    let res = self.rt.block_on(async {
                                        tokito::store::designs::create(
                                            &self.pool,
                                            tokito::models::CreateDesign {
                                                name,
                                                description: if desc.is_empty() {
                                                    None
                                                } else {
                                                    Some(desc)
                                                },
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
                    });

                    ui.add_space(16.0);

                    ui.vertical(|ui| {
                        ui.set_min_width(560.0);
                        ui.horizontal(|ui| {
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
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let _ = crate::ui::layout::search_field(
                                ui,
                                &mut self.projects_search,
                                "Search name or description…",
                            );
                            egui::ComboBox::from_id_salt("projects_sort")
                                .selected_text(match self.projects_sort {
                                    ProjectsSort::UpdatedDesc => "Updated ↓",
                                    ProjectsSort::UpdatedAsc => "Updated ↑",
                                    ProjectsSort::NameAsc => "Name A→Z",
                                    ProjectsSort::NameDesc => "Name Z→A",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.projects_sort,
                                        ProjectsSort::UpdatedDesc,
                                        "Updated ↓",
                                    );
                                    ui.selectable_value(
                                        &mut self.projects_sort,
                                        ProjectsSort::UpdatedAsc,
                                        "Updated ↑",
                                    );
                                    ui.selectable_value(
                                        &mut self.projects_sort,
                                        ProjectsSort::NameAsc,
                                        "Name A→Z",
                                    );
                                    ui.selectable_value(
                                        &mut self.projects_sort,
                                        ProjectsSort::NameDesc,
                                        "Name Z→A",
                                    );
                                });
                        });
                        ui.add_space(12.0);

                        let mut rows: Vec<tokito::models::Design> = self
                            .designs
                            .iter()
                            .filter(|d| {
                                let q = self.projects_search.to_lowercase();
                                if q.is_empty() {
                                    return true;
                                }
                                d.name.to_lowercase().contains(&q)
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
                            ProjectsSort::UpdatedAsc => {
                                rows.sort_by(|a, b| a.updated_at.cmp(&b.updated_at))
                            }
                            ProjectsSort::UpdatedDesc => {
                                rows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at))
                            }
                        }

                        let pinned: Vec<_> = rows
                            .iter()
                            .filter(|d| self.projects_pinned.contains(&d.id))
                            .cloned()
                            .collect();
                        let recent_ids: HashSet<_> =
                            self.recent_design_ids.iter().copied().collect();
                        let recent: Vec<_> = rows
                            .iter()
                            .filter(|d| {
                                recent_ids.contains(&d.id) && !self.projects_pinned.contains(&d.id)
                            })
                            .cloned()
                            .collect();
                        let mut seen = std::collections::HashSet::<uuid::Uuid>::new();
                        for d in &pinned {
                            seen.insert(d.id);
                        }
                        for d in &recent {
                            seen.insert(d.id);
                        }
                        let others: Vec<_> =
                            rows.into_iter().filter(|d| !seen.contains(&d.id)).collect();

                        egui::ScrollArea::vertical()
                            .id_salt("projects_scroll")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if pinned.is_empty()
                                    && recent.is_empty()
                                    && others.is_empty()
                                    && self.designs.is_empty()
                                {
                                    crate::ui::layout::empty_state(
                                        ui,
                                        &tokens,
                                        "No designs in this project — create one on the left.",
                                    );
                                    return;
                                }

                                let render_section =
                                    |ui: &mut egui::Ui,
                                     this: &mut App,
                                     title: &str,
                                     designs: &[tokito::models::Design]| {
                                        if designs.is_empty() {
                                            return;
                                        }
                                        crate::ui::layout::subsection(ui, &tokens, title);
                                        for d in designs {
                                            crate::ui::layout::content_card(ui, &tokens, |ui| {
                                                ui.horizontal(|ui| {
                                                    let mut pin =
                                                        this.projects_pinned.contains(&d.id);
                                                    if ui.checkbox(&mut pin, "").changed() {
                                                        if pin {
                                                            this.projects_pinned.insert(d.id);
                                                        } else {
                                                            this.projects_pinned.remove(&d.id);
                                                        }
                                                    }
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(&d.name)
                                                                .strong()
                                                                .size(14.0),
                                                        );
                                                        if let Some(desc) = &d.description {
                                                            ui.label(
                                                                egui::RichText::new(
                                                                    crate::util::truncate_ui_chars(
                                                                        desc, 140,
                                                                    ),
                                                                )
                                                                .weak()
                                                                .small(),
                                                            );
                                                        }
                                                        let ts = d.updated_at.to_rfc3339();
                                                        let short = ts
                                                            .get(..10)
                                                            .map(String::from)
                                                            .unwrap_or_else(|| ts.clone());
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "Updated {short}"
                                                            ))
                                                            .small()
                                                            .weak()
                                                            .color(tokens.text_muted),
                                                        );
                                                    });
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            if crate::ui::widgets::primary_button(
                                                                ui, &tokens, "Open",
                                                            )
                                                            .clicked()
                                                            {
                                                                this.open_design(d.id);
                                                            }
                                                        },
                                                    );
                                                });
                                            });
                                            ui.add_space(8.0);
                                        }
                                    };

                                render_section(ui, self, "Pinned", &pinned);
                                render_section(ui, self, "Recent", &recent);
                                render_section(ui, self, "All designs", &others);
                            });
                    });

                    ui.add_space(16.0);

                    ui.vertical(|ui| {
                        ui.set_max_width(220.0);
                        crate::ui::layout::content_card(ui, &tokens, |ui| {
                            ui.label(ty.section("Project list").color(tokens.text_primary));
                            ui.add_space(6.0);
                            egui::ScrollArea::vertical()
                                .id_salt("project_list")
                                .max_height(360.0)
                                .show(ui, |ui| {
                                    for p in self.projects.clone() {
                                        let active = self.active_project_id == Some(p.id);
                                        if ui
                                            .selectable_label(
                                                active,
                                                egui::RichText::new(&p.name).size(13.0),
                                            )
                                            .clicked()
                                        {
                                            self.active_project_id = Some(p.id);
                                            self.reload_projects();
                                        }
                                    }
                                });
                            ui.add_space(6.0);
                            if let Some(pid) = self.active_project_id {
                                ui.horizontal(|ui| {
                                    if crate::ui::widgets::secondary_button(
                                        ui,
                                        &tokens,
                                        "Export project zip",
                                    )
                                    .clicked()
                                    {
                                        if let Some(path) = rfd::FileDialog::new()
                                            .set_file_name("project.zip")
                                            .save_file()
                                        {
                                            let uid = self.user_id;
                                            let db_url = self.db_url.clone();
                                            let pg_ver =
                                                self.settings_file.database.pg_embed_version;
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
                                    if crate::ui::widgets::secondary_button(
                                        ui,
                                        &tokens,
                                        "Import project zip",
                                    )
                                    .clicked()
                                    {
                                        if let Some(path) = rfd::FileDialog::new()
                                            .add_filter("Zip", &["zip"])
                                            .pick_file()
                                        {
                                            let uid = self.user_id;
                                            let res = self.rt.block_on(
                                                tokito::services::project_archive::import_project_zip(
                                                    &self.pool,
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
                    });
                });
            });
    }
}
