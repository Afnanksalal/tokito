use uuid::Uuid;

use crate::app::studio::chrome::TabChrome;
use crate::app::App;

impl App {
    pub(crate) fn render_studio_bom_tab(&mut self, ui: &mut egui::Ui, design_id: Uuid) {
        let tokens = self.ui_tokens;
        let chrome = TabChrome::begin(ui, &tokens);

        if self.bom_dirty || self.bom_loaded_for != Some(design_id) {
            self.refresh_bom(design_id);
        }

        let sync_badge = self.bom_sync_status(design_id);
        chrome.header(
            ui,
            "Bill of materials",
            Some("Edit lines here or sync quantities from placed parts"),
        );
        ui.label(
            egui::RichText::new(sync_badge)
                .small()
                .color(chrome.tokens.text_muted),
        );

        ui.horizontal(|ui| {
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Refresh").clicked() {
                self.bom_dirty = true;
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Sync from schematic")
                .clicked()
            {
                self.sync_bom_from_schematic(design_id);
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Export CSV").clicked() {
                self.export_schematic_file("bom_csv");
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "+ Add line").clicked() {
                self.add_empty_bom_line(design_id);
            }
            ui.label(
                egui::RichText::new(format!("{} line(s)", self.bom_lines.len()))
                    .small()
                    .weak()
                    .color(chrome.tokens.text_muted),
            );
        });
        ui.add_space(8.0);

        if self.bom_lines.is_empty() {
            chrome.empty(
                ui,
                "No BOM lines yet. Use Build, Sync from schematic, or place catalog parts.",
            );
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("studio_bom_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("bom_grid")
                    .num_columns(5)
                    .spacing([12.0, 8.0])
                    .striped(true)
                    .show(ui, |ui| {
                        crate::ui::table::sortable_header(ui, "MPN", 0, &mut self.bom_sort);
                        crate::ui::table::sortable_header(ui, "Qty", 1, &mut self.bom_sort);
                        crate::ui::table::sortable_header(ui, "Notes", 2, &mut self.bom_sort);
                        crate::ui::table::sortable_header(ui, "Part ID", 3, &mut self.bom_sort);
                        ui.label(egui::RichText::new("").small());
                        ui.end_row();

                        let mut rows: Vec<(tokito::models::BomLine, String)> = self
                            .bom_lines
                            .iter()
                            .map(|line| {
                                let mpn = self
                                    .part_cache
                                    .get(&line.part_id)
                                    .cloned()
                                    .unwrap_or_else(|| "—".to_string());
                                (line.clone(), mpn)
                            })
                            .collect();
                        rows.sort_by(|a, b| {
                            if self.bom_sort.dir == crate::ui::table::SortDir::None {
                                return std::cmp::Ordering::Equal;
                            }
                            let ord = match self.bom_sort.column {
                                0 => a.1.cmp(&b.1),
                                1 => {
                                    a.0.quantity
                                        .partial_cmp(&b.0.quantity)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                }
                                2 => {
                                    a.0.notes
                                        .as_deref()
                                        .unwrap_or("")
                                        .cmp(b.0.notes.as_deref().unwrap_or(""))
                                }
                                3 => a.0.part_id.cmp(&b.0.part_id),
                                _ => std::cmp::Ordering::Equal,
                            };
                            match self.bom_sort.dir {
                                crate::ui::table::SortDir::Asc => ord,
                                crate::ui::table::SortDir::Desc => ord.reverse(),
                                crate::ui::table::SortDir::None => std::cmp::Ordering::Equal,
                            }
                        });

                        for (line, mpn) in rows {
                            ui.label(egui::RichText::new(mpn).small());

                            let mut qty = line.quantity;
                            let qty_resp = ui.add(
                                egui::DragValue::new(&mut qty)
                                    .speed(0.1)
                                    .range(0.01..=1_000_000.0),
                            );

                            let mut notes = line.notes.clone().unwrap_or_default();
                            let notes_resp =
                                ui.add(egui::TextEdit::singleline(&mut notes).desired_width(120.0));

                            ui.monospace(
                                egui::RichText::new(line.part_id.to_string()).small().weak(),
                            );

                            let save_clicked =
                                crate::ui::widgets::secondary_button(ui, chrome.tokens, "Save")
                                    .clicked();
                            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Del")
                                .clicked()
                            {
                                let lid = line.id;
                                let res = self.rt.block_on(async {
                                    tokito::store::bom::delete_line(&self.pool, lid).await
                                });
                                match res {
                                    Ok(()) => self.bom_dirty = true,
                                    Err(e) => self.set_err(e.to_string()),
                                }
                            }

                            if save_clicked || qty_resp.lost_focus() || notes_resp.lost_focus() {
                                let qty_changed = (qty - line.quantity).abs() > f64::EPSILON;
                                let notes_changed = notes != line.notes.clone().unwrap_or_default();
                                if save_clicked || qty_changed || notes_changed {
                                    let line_id = line.id;
                                    let notes_ref = if notes.is_empty() {
                                        None
                                    } else {
                                        Some(notes.as_str())
                                    };
                                    let res = self.rt.block_on(async {
                                        tokito::store::bom::patch_line(
                                            &self.pool,
                                            line_id,
                                            Some(qty),
                                            notes_ref,
                                        )
                                        .await
                                    });
                                    match res {
                                        Ok(_) => self.bom_dirty = true,
                                        Err(e) => self.set_err(e.to_string()),
                                    }
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
    }
}
