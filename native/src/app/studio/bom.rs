use uuid::Uuid;

use crate::app::studio::chrome::TabChrome;
use crate::app::App;

impl App {
    pub(crate) fn render_studio_bom_tab(&mut self, ui: &mut egui::Ui, design_id: Uuid) {
        let tokens = crate::ui::tokens::UiTokens::default();
        let chrome = TabChrome::begin(ui, &tokens);

        if self.bom_dirty || self.bom_loaded_for != Some(design_id) {
            self.refresh_bom(design_id);
        }

        chrome.header(
            ui,
            "Bill of materials",
            Some("Quantities from schematic instances and catalog parts"),
        );

        ui.horizontal(|ui| {
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Refresh").clicked() {
                self.bom_dirty = true;
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
                "No BOM lines yet. Use Build to draft a board, or place parts manually.",
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
                        ui.label(egui::RichText::new("MPN").small().strong());
                        ui.label(egui::RichText::new("Qty").small().strong());
                        ui.label(egui::RichText::new("Notes").small().strong());
                        ui.label(egui::RichText::new("Part ID").small().strong());
                        ui.label(egui::RichText::new("Updated").small().strong());
                        ui.end_row();

                        for line in &self.bom_lines {
                            let mpn = self
                                .part_cache
                                .get(&line.part_id)
                                .cloned()
                                .unwrap_or_else(|| "—".to_string());
                            ui.label(egui::RichText::new(mpn).small());
                            ui.label(egui::RichText::new(format!("{}", line.quantity)).small());
                            ui.label(
                                egui::RichText::new(line.notes.clone().unwrap_or_default())
                                    .small()
                                    .weak(),
                            );
                            ui.monospace(egui::RichText::new(line.part_id.to_string()).small());
                            let ts = line.updated_at.to_rfc3339();
                            let short =
                                ts.get(..10).map(String::from).unwrap_or_else(|| ts.clone());
                            ui.label(egui::RichText::new(short).small().weak());
                            ui.end_row();
                        }
                    });
            });
    }
}
