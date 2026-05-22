//! Build panel: AI researches parts and drafts the schematic; you review and edit.

use uuid::Uuid;

use crate::app::studio::chrome::TabChrome;
use crate::app::App;

impl App {
    pub(crate) fn render_studio_build_tab(&mut self, ui: &mut egui::Ui, design_id: Uuid) {
        let tokens = self.ui_tokens;
        let chrome = TabChrome::begin(ui, &tokens);
        chrome.header(
            ui,
            "Build",
            Some(
                "Describe the board. Tokito researches parts, drafts the BOM, and prepares schematic edits.",
            ),
        );

        if !self.ai_build_ready {
            crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                ui.label(
                    egui::RichText::new("AI build is not configured")
                        .strong()
                        .color(chrome.tokens.text_primary),
                );
                ui.add_space(4.0);
                ui.label(
                    "Open the Settings panel and enter your AI provider and Firecrawl API keys.",
                );
                if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Open Settings")
                    .clicked()
                {
                    self.open_settings();
                }
            });
            ui.add_space(12.0);
        }

        if let Some(batch) = &self.pending_edit_batch {
            chrome.subsection(ui, "Ready to apply");
            if self.pending_edit_selected.len() != batch.ops.len() {
                self.pending_edit_selected = vec![true; batch.ops.len()];
            }
            if !batch.provenance.is_empty() {
                let tags: Vec<String> = batch
                    .provenance
                    .iter()
                    .map(|p| tokito::services::erc_fixes::provenance_label(p).to_string())
                    .collect();
                ui.label(
                    egui::RichText::new(format!("Source: {}", tags.join(" | ")))
                        .small()
                        .weak(),
                );
                ui.add_space(4.0);
            }
            crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                for (i, op) in batch.ops.iter().enumerate() {
                    let mut on = self.pending_edit_selected.get(i).copied().unwrap_or(true);
                    let prov =
                        tokito::services::erc_fixes::provenance_label(&batch.provenance_for_op(i));
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut on, "");
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(op.summary()).small());
                            ui.label(
                                egui::RichText::new(prov)
                                    .small()
                                    .weak()
                                    .color(chrome.tokens.text_muted),
                            );
                        });
                    });
                    if let Some(slot) = self.pending_edit_selected.get_mut(i) {
                        *slot = on;
                    }
                    ui.add_space(4.0);
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if crate::ui::widgets::primary_button(ui, chrome.tokens, "Apply selected").clicked()
                {
                    self.apply_pending_edit_batch();
                }
                if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Discard all").clicked()
                {
                    self.discard_pending_edit_batch();
                }
            });
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
        }

        chrome.subsection(ui, "What should this board do?");
        ui.add(
            egui::TextEdit::multiline(&mut self.prompt)
                .hint_text("Example: 12 V to 5 V buck, 2 A, synchronous, enable pin, LC filter")
                .desired_rows(8)
                .margin(egui::Margin::symmetric(10.0, 8.0)),
        );

        if let Some(diff) = &self.build_bom_diff {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(format!("BOM: {diff}"))
                    .small()
                    .color(chrome.tokens.text_muted),
            );
        }

        if self.prompt_busy {
            ui.add_space(8.0);
            let label = if self.build_stage.is_empty() {
                "Building...".to_string()
            } else {
                self.build_stage.clone()
            };
            ui.horizontal(|ui| {
                ui.add(egui::ProgressBar::new(0.0).animate(true).text(label));
                if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Cancel").clicked() {
                    self.cancel_build();
                }
            });
        }

        if !self.build_warnings.is_empty() {
            ui.add_space(8.0);
            chrome.subsection(ui, "Build notes");
            crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                for w in &self.build_warnings {
                    ui.label(egui::RichText::new(w).small().color(chrome.tokens.warning));
                }
            });
        }

        ui.add_space(10.0);
        let can_build = self.ai_build_ready && !self.prompt_busy && !self.prompt.trim().is_empty();
        let build_label = if self.prompt_busy {
            "Building: research, BOM, schematic..."
        } else {
            "Build schematic"
        };
        if ui
            .add_enabled(
                can_build,
                egui::Button::new(
                    egui::RichText::new(build_label)
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(chrome.tokens.accent)
                .min_size(egui::vec2(ui.available_width(), 40.0)),
            )
            .clicked()
        {
            self.run_prompt_draft(design_id, ui.ctx());
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Save board").clicked() {
                self.save_schematic(design_id);
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Reload board").clicked() {
                self.open_design(design_id);
            }
        });

        ui.add_space(10.0);
        ui.label(
            egui::RichText::new(
                "Ctrl/Cmd+Enter build | Ctrl+Shift+P commands | Ctrl+D duplicate | Del delete",
            )
            .small()
            .weak()
            .color(chrome.tokens.text_muted),
        );
    }
}
