//! Research artifacts — view, add, and edit manual notes.

use crate::app::studio::chrome::TabChrome;
use crate::app::App;
use uuid::Uuid;

impl App {
    pub(crate) fn render_studio_research_tab(&mut self, ui: &mut egui::Ui, design_id: Uuid) {
        let tokens = self.ui_tokens;
        let chrome = TabChrome::begin(ui, &tokens);
        chrome.header(
            ui,
            "Research",
            Some("Firecrawl sources from Build and your manual notes"),
        );

        chrome.subsection(ui, "Add note");
        ui.label(egui::RichText::new("Title").small().weak());
        ui.text_edit_singleline(&mut self.research_draft_title);
        ui.label(egui::RichText::new("Note").small().weak());
        ui.add(
            egui::TextEdit::multiline(&mut self.research_draft_body)
                .desired_rows(4)
                .hint_text("Datasheet takeaways, requirements, links…"),
        );
        ui.horizontal(|ui| {
            let label = if self.research_editing_id.is_some() {
                "Update note"
            } else {
                "Add note"
            };
            if crate::ui::widgets::primary_button(ui, chrome.tokens, label).clicked() {
                self.save_research_note(design_id);
            }
            if self.research_editing_id.is_some()
                && crate::ui::widgets::secondary_button(ui, chrome.tokens, "Cancel edit")
                    .clicked()
            {
                self.research_editing_id = None;
                self.research_draft_title.clear();
                self.research_draft_body.clear();
            }
        });
        ui.add_space(10.0);

        let res = self.rt.block_on(async {
            tokito::store::research::list_for_design(&self.pool, design_id, 64).await
        });
        match res {
            Ok(rows) if rows.is_empty() => {
                chrome.empty(ui, "No research yet. Run Build or add a note above.");
            }
            Ok(mut rows) => {
                rows.sort_by(|a, b| {
                    let pa = a
                        .metadata_json
                        .get("pinned")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let pb = b
                        .metadata_json
                        .get("pinned")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    pb.cmp(&pa).then_with(|| b.created_at.cmp(&a.created_at))
                });
                ui.label(
                    egui::RichText::new(format!("{} artifact(s)", rows.len()))
                        .small()
                        .weak()
                        .color(chrome.tokens.text_muted),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    crate::ui::table::sortable_header(ui, "Kind", 0, &mut self.research_sort);
                    crate::ui::table::sortable_header(ui, "Title", 1, &mut self.research_sort);
                    crate::ui::table::sortable_header(ui, "Date", 2, &mut self.research_sort);
                });
                if self.research_sort.dir != crate::ui::table::SortDir::None {
                    rows.sort_by(|a, b| {
                        let ord = match self.research_sort.column {
                            0 => a.kind.cmp(&b.kind),
                            1 => a
                                .title
                                .as_deref()
                                .unwrap_or("")
                                .cmp(b.title.as_deref().unwrap_or("")),
                            2 => a.created_at.cmp(&b.created_at),
                            _ => std::cmp::Ordering::Equal,
                        };
                        match self.research_sort.dir {
                            crate::ui::table::SortDir::Asc => ord,
                            crate::ui::table::SortDir::Desc => ord.reverse(),
                            crate::ui::table::SortDir::None => std::cmp::Ordering::Equal,
                        }
                    });
                }
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .id_salt("research_list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for row in rows {
                            let id = row.id;
                            let kind = row.kind.clone();
                            let is_manual = kind == tokito::store::research::KIND_MANUAL_NOTE;
                            let pinned = row
                                .metadata_json
                                .get("pinned")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                                ui.horizontal(|ui| {
                                    let mut pin_flag = pinned;
                                    if ui
                                        .checkbox(&mut pin_flag, "Pin")
                                        .on_hover_text("Keep this artifact at the top of research")
                                        .changed()
                                    {
                                        let res = self.rt.block_on(async {
                                            tokito::store::research::set_pinned(
                                                &self.pool, id, pin_flag,
                                            )
                                            .await
                                        });
                                        if let Err(e) = res {
                                            self.set_err(e.to_string());
                                        }
                                    }
                                    ui.label(
                                        egui::RichText::new(&kind)
                                            .small()
                                            .strong()
                                            .color(chrome.tokens.accent),
                                    );
                                    if let Some(title) = &row.title {
                                        ui.label(egui::RichText::new(title).small());
                                    }
                                    if !is_manual && kind != tokito::store::research::KIND_ANNOTATION {
                                        if ui.small_button("Annotate").clicked() {
                                            self.research_annotate_parent = Some(id);
                                            self.research_draft_body.clear();
                                        }
                                    }
                                    if is_manual {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.small_button("Edit").clicked() {
                                                    self.research_editing_id = Some(id);
                                                    self.research_draft_title =
                                                        row.title.clone().unwrap_or_default();
                                                    self.research_draft_body =
                                                        row.content_text.clone();
                                                }
                                                if ui.small_button("Delete").clicked() {
                                                    let res = self.rt.block_on(async {
                                                        tokito::store::research::delete_artifact(
                                                            &self.pool, id,
                                                        )
                                                        .await
                                                    });
                                                    if let Err(e) = res {
                                                        self.set_err(e.to_string());
                                                    }
                                                }
                                            },
                                        );
                                    }
                                });
                                if let Some(url) = &row.source_url {
                                    ui.hyperlink_to(util_truncate(url, 72), url);
                                }
                                ui.label(
                                    egui::RichText::new(util_truncate(&row.content_text, 480))
                                        .small()
                                        .color(chrome.tokens.text_secondary),
                                );
                            });
                            ui.add_space(6.0);
                        }
                    });
            }
            Err(e) => {
                ui.label(egui::RichText::new(e.to_string()).color(chrome.tokens.danger));
            }
        }
    }

    fn save_research_note(&mut self, design_id: Uuid) {
        let body = self.research_draft_body.trim();
        if body.is_empty() {
            self.set_err("Note text is required");
            return;
        }
        let title = if self.research_draft_title.trim().is_empty() {
            None
        } else {
            Some(self.research_draft_title.trim().to_string())
        };
        if let Some(parent) = self.research_annotate_parent.take() {
            let res = self.rt.block_on(async {
                tokito::store::research::insert_annotation(&self.pool, design_id, parent, body)
                    .await
            });
            match res {
                Ok(_) => {
                    self.research_draft_body.clear();
                    self.log_console("Annotation saved.");
                }
                Err(e) => self.set_err(e.to_string()),
            }
            return;
        }
        if let Some(id) = self.research_editing_id {
            let res = self.rt.block_on(async {
                tokito::store::research::update_manual(
                    &self.pool,
                    id,
                    title.as_deref(),
                    body,
                )
                .await
            });
            match res {
                Ok(_) => {
                    self.research_editing_id = None;
                    self.research_draft_title.clear();
                    self.research_draft_body.clear();
                    self.log_console("Research note updated.");
                }
                Err(e) => self.set_err(e.to_string()),
            }
            return;
        }
        let res = self.rt.block_on(async {
            tokito::store::research::insert(
                &self.pool,
                design_id,
                tokito::store::research::KIND_MANUAL_NOTE,
                title.as_deref(),
                None,
                body,
                serde_json::json!({}),
            )
            .await
        });
        match res {
            Ok(_) => {
                self.research_draft_title.clear();
                self.research_draft_body.clear();
                self.log_console("Research note added.");
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }
}

fn util_truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}
