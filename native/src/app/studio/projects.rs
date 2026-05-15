use std::collections::HashSet;

use crate::app::{App, ProjectsSort};

impl App {
    pub(crate) fn ui_projects(&mut self, ctx: &egui::Context) {
        let tokens = crate::ui::tokens::UiTokens::default();
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
                            "Hardware design workspace — open a project or create a new schematic",
                        )
                        .color(tokens.text_muted),
                    );
                });
                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_max_width(300.0);
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

                    ui.add_space(24.0);

                    ui.vertical(|ui| {
                        ui.set_min_width(560.0);
                        ui.horizontal(|ui| {
                            ui.label(ty.section("Your designs").color(tokens.text_primary));
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
                                        "No designs yet — create one on the left.",
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
                });
            });
    }
}
