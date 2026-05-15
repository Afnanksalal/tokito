//! Design Manager — sheets, components, nets (project tree).

use crate::app::studio::chrome::TabChrome;
use crate::app::App;
use crate::editor::connectivity;

impl App {
    pub(crate) fn render_studio_design_manager_tab(&mut self, ui: &mut egui::Ui) {
        let tokens = crate::ui::tokens::UiTokens::default();
        let chrome = TabChrome::begin(ui, &tokens);
        chrome.header(
            ui,
            "Design manager",
            Some("Sheets, placed components, and nets — click to select on canvas"),
        );

        chrome.subsection(ui, "Sheets");
        crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
            for sheet in &self.editor.sheets.clone() {
                let active = sheet.id == self.editor.active_sheet_id;
                if ui
                    .selectable_label(
                        active,
                        egui::RichText::new(format!("{} — {}", sheet.id, sheet.name)).size(12.0),
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
                                    egui::RichText::new(format!("{refdes}  ·  {mpn}"))
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
