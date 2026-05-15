//! Research artifacts — provenance next to the editor.

use crate::app::studio::chrome::TabChrome;
use crate::app::App;
use uuid::Uuid;

impl App {
    pub(crate) fn render_studio_research_tab(&mut self, ui: &mut egui::Ui, design_id: Uuid) {
        let tokens = crate::ui::tokens::UiTokens::default();
        let chrome = TabChrome::begin(ui, &tokens);
        chrome.header(
            ui,
            "Research",
            Some("Sources gathered while AI builds the board (Firecrawl)"),
        );

        let res = self.rt.block_on(async {
            tokito::store::research::list_for_design(&self.pool, design_id, 64).await
        });
        match res {
            Ok(rows) if rows.is_empty() => {
                chrome.empty(
                    ui,
                    "No research artifacts yet. Run Build to collect sources.",
                );
            }
            Ok(rows) => {
                ui.label(
                    egui::RichText::new(format!("{} artifact(s)", rows.len()))
                        .small()
                        .weak()
                        .color(chrome.tokens.text_muted),
                );
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .id_salt("research_list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for row in rows {
                            crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                                ui.label(
                                    egui::RichText::new(&row.kind)
                                        .small()
                                        .strong()
                                        .color(chrome.tokens.accent),
                                );
                                if let Some(url) = &row.source_url {
                                    ui.hyperlink_to(util_truncate(url, 72), url);
                                }
                                ui.label(
                                    egui::RichText::new(util_truncate(&row.content_text, 320))
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
}

fn util_truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}
