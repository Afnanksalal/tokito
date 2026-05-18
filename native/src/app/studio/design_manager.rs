//! Design Manager: sheets, components, nets.

use crate::app::studio::chrome::TabChrome;
use crate::app::App;
use crate::editor::connectivity;

impl App {
    pub(crate) fn render_studio_design_manager_tab(
        &mut self,
        ui: &mut egui::Ui,
        design_id: uuid::Uuid,
    ) {
        let tokens = self.ui_tokens;
        let chrome = TabChrome::begin(ui, &tokens);
        chrome.header(
            ui,
            "Design manager",
            Some("Sheets, placed components, and nets. Click to select on canvas"),
        );

        if let Some(design) = self.design.clone() {
            chrome.subsection(ui, "Design");
            ui.label(egui::RichText::new("Name").small().weak());
            let name_resp = ui.text_edit_singleline(&mut self.design_edit_name);
            ui.label(egui::RichText::new("Description").small().weak());
            let desc_resp = ui.text_edit_singleline(&mut self.design_edit_desc);
            ui.label(egui::RichText::new("Design notes").small().weak());
            let notes_resp = ui.add(
                egui::TextEdit::multiline(&mut self.design_edit_notes)
                    .desired_rows(3)
                    .hint_text("Requirements, bring-up notes"),
            );
            if name_resp.changed() || desc_resp.changed() || notes_resp.changed() {
                self.design_save_debounce = 1.2;
            }
            ui.label(
                egui::RichText::new(format!("Last backup: {}", self.last_backup_label))
                    .small()
                    .weak(),
            );
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Save design info").clicked()
            {
                let id = design.id;
                let name = self.design_edit_name.trim().to_string();
                let desc = self.design_edit_desc.trim().to_string();
                let notes = self.design_edit_notes.trim().to_string();
                if name.is_empty() {
                    self.set_err("Design name required");
                } else {
                    let patch = tokito::models::PatchDesign {
                        name: Some(name),
                        description: if desc.is_empty() { None } else { Some(desc) },
                        notes: if notes.is_empty() { None } else { Some(notes) },
                    };
                    let res = self.rt.block_on(async {
                        tokito::store::designs::patch(&self.pool, id, patch).await
                    });
                    match res {
                        Ok(row) => {
                            self.design = Some(row);
                            self.log_console("Design info saved.");
                            self.toast_ok("Design info saved");
                        }
                        Err(e) => self.set_err(e.to_string()),
                    }
                }
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Backup design")
                    .clicked()
                {
                    self.backup_current_design(design_id);
                }
                if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Open exports folder")
                    .clicked()
                {
                    let dir = self.export_dir_for_design(design_id);
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = open::that(dir);
                }
            });
            ui.add_space(6.0);
            let ws = self.rt.block_on(async {
                tokito::store::projects::workspace_path_for_design(&self.pool, design_id).await
            });
            if let Ok(ws) = ws {
                let backups = tokito::services::backup::list_backups(&ws);
                if !backups.is_empty() {
                    ui.label(egui::RichText::new("Recent backups").small().weak());
                    for path in backups.into_iter().take(5) {
                        let label = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("backup");
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(label).small().monospace());
                            if ui.small_button("Open").clicked() {
                                let _ = open::that(&path);
                            }
                        });
                    }
                }
                if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Open backups folder")
                    .clicked()
                {
                    let dir = tokito::services::backup::open_backups_folder(&ws);
                    let _ = open::that(dir);
                }
                if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Restore from zip")
                    .clicked()
                {
                    if let Some(zip) = rfd::FileDialog::new()
                        .add_filter("Zip", &["zip"])
                        .pick_file()
                    {
                        let pid = design
                            .project_id
                            .unwrap_or_else(tokito::store::projects::default_project_id);
                        let res = self.rt.block_on(
                            tokito::services::project_archive::restore_design_archive(
                                &self.pool,
                                &zip,
                                pid,
                                self.user_id,
                            ),
                        );
                        match res {
                            Ok(new_id) => {
                                self.toast_ok("Design restored from archive");
                                self.open_design(new_id);
                            }
                            Err(e) => self.set_err(e.to_string()),
                        }
                    }
                }
            }
        }

        chrome.subsection(ui, "Sheets");
        crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
            for sheet in &self.editor.sheets.clone() {
                let active = sheet.id == self.editor.active_sheet_id;
                if ui
                    .selectable_label(
                        active,
                        egui::RichText::new(format!("{} | {}", sheet.id, sheet.name)).size(12.0),
                    )
                    .clicked()
                {
                    self.switch_active_sheet(sheet.id.clone());
                }
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "+ New sheet").clicked() {
                let n = self.editor.sheets.len() + 1;
                let id = format!("sheet{n}");
                self.editor.sheets.push(crate::editor::SheetInfo {
                    id: id.clone(),
                    name: format!("Sheet {n}"),
                });
                self.switch_active_sheet(id);
            }
        });

        chrome.subsection(ui, "Components");
        egui::ScrollArea::vertical()
            .id_salt("design_mgr_components")
            .max_height(200.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let rows = connectivity::components_summary(&self.editor.symbols, &self.part_cache);
                if rows.is_empty() {
                    chrome.empty(
                        ui,
                        "No symbols yet. Describe the board in Build, or use Place.",
                    );
                } else {
                    crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                        for (refdes, mpn) in &rows {
                            let selected = self.editor.selected_syms.contains(refdes);
                            if ui
                                .selectable_label(
                                    selected,
                                    egui::RichText::new(format!("{refdes}  |  {mpn}"))
                                        .monospace()
                                        .size(12.0),
                                )
                                .clicked()
                            {
                                self.editor.clear_selection();
                                self.editor.selected_syms.insert(refdes.clone());
                                self.editor.selected_sym = Some(refdes.clone());
                            }
                        }
                    });
                }
            });

        chrome.subsection(ui, "Nets");
        let nets = connectivity::all_net_names(&self.editor.wire_segments, &self.editor.net_labels);
        egui::ScrollArea::vertical()
            .id_salt("design_mgr_nets")
            .max_height(160.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if nets.is_empty() {
                    chrome.empty(ui, "No nets yet. Wire symbols or add net labels.");
                } else {
                    crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                            for net in &nets {
                                let seg_count = self
                                    .editor
                                    .wire_segments
                                    .iter()
                                    .filter(|s| s.net.trim() == net.as_str())
                                    .count();
                                if crate::ui::layout::filter_chip(
                                    ui,
                                    chrome.tokens,
                                    &format!("{net} ({seg_count})"),
                                    false,
                                ) {
                                    self.editor.clear_selection();
                                    if let Some(i) = self
                                        .editor
                                        .wire_segments
                                        .iter()
                                        .position(|s| s.net.trim() == net.as_str())
                                    {
                                        self.editor.selected_segments.insert(i);
                                        self.editor.selected_segment = Some(i);
                                    }
                                }
                            }
                        });
                    });
                }
            });
    }
}
