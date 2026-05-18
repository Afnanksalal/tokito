//! Agent tool-loop panel (AI provider + local DB tools).

use uuid::Uuid;

use crate::app::studio::chrome::TabChrome;
use crate::app::App;

impl App {
    pub(crate) fn render_studio_agent_tab(&mut self, ui: &mut egui::Ui, design_id: Uuid) {
        let tokens = self.ui_tokens;
        let chrome = TabChrome::begin(ui, &tokens);
        chrome.header(
            ui,
            "Agent",
            Some("Ask Tokito to search parts, scrape URLs, or update the BOM"),
        );

        if self.state.llm.is_none() {
            chrome.empty(
                ui,
                "Set an AI provider API key in Settings to use the agent.",
            );
            return;
        }

        if !self.agent_last_message.is_empty() {
            chrome.subsection(ui, "Last response");
            crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("agent_last_msg")
                    .max_height(160.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&self.agent_last_message)
                                .small()
                                .color(chrome.tokens.text_primary),
                        );
                    });
            });
            ui.add_space(8.0);
        }

        ui.label(egui::RichText::new("Message").small().weak());
        ui.add(
            egui::TextEdit::multiline(&mut self.agent_query)
                .desired_rows(4)
                .hint_text("Example: Find a 3.3 V LDO for 500 mA and add to BOM"),
        );
        ui.horizontal(|ui| {
            let can_run = !self.agent_busy && !self.agent_query.trim().is_empty();
            if crate::ui::widgets::primary_button(ui, chrome.tokens, "Run agent").clicked()
                && can_run
            {
                self.run_agent(design_id, ui.ctx());
            }
            if self.agent_busy {
                ui.spinner();
                ui.label(egui::RichText::new("Running...").small().weak());
            }
        });
        ui.add_space(8.0);
        chrome.subsection(ui, "Tips");
        ui.label(
            egui::RichText::new(
                "The agent can search_parts, scrape_url, sync_part_offers, get_design_bom, append_bom_lines. \
                 It does not edit the schematic; use Build for that.",
            )
            .small()
            .weak(),
        );
    }

    fn run_agent(&mut self, design_id: Uuid, ctx: &egui::Context) {
        if self.agent_rx.is_some() {
            return;
        }
        let query = self.agent_query.trim().to_string();
        self.agent_busy = true;
        let state = self.state.clone();
        let user_id = self.user_id;
        let (tx, rx) = std::sync::mpsc::channel();
        let repaint = ctx.clone();
        self.rt.spawn(async move {
            let messages = vec![serde_json::json!({"role": "user", "content": query})];
            let input = tokito::services::agent::AgentRunInput {
                messages,
                design_id: Some(design_id),
                model: None,
            };
            let auth = tokito::auth::AuthUser {
                user_id,
                email: "native@local".into(),
            };
            let result = tokito::services::agent::run(&state, auth, input)
                .await
                .map(|out| {
                    out.pointer("/final_message")
                        .and_then(|v| v.as_str())
                        .or_else(|| out.pointer("/summary").and_then(|v| v.as_str()))
                        .unwrap_or("Agent finished.")
                        .to_string()
                })
                .map_err(|e| e.to_string());
            let _ = tx.send(result);
            repaint.request_repaint();
        });
        self.agent_rx = Some(rx);
    }
}
