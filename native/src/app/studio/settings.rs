//! Application settings panel (`settings.toml`).

use crate::app::studio::chrome::TabChrome;
use crate::app::App;
use crate::theme;
use tokito::router::AppState;

impl App {
    pub(crate) fn render_studio_settings_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let tokens = self.ui_tokens;
        let chrome = TabChrome::begin(ui, &tokens);
        chrome.header(
            ui,
            "Settings",
            Some("Stored in your app data folder"),
        );

        crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
            ui.label(egui::RichText::new("General").strong());
            ui.horizontal(|ui| {
                ui.label("Theme:");
                egui::ComboBox::from_id_salt("settings_theme")
                    .selected_text(&self.settings_file.general.theme)
                    .show_ui(ui, |ui| {
                        for t in ["light", "dark", "system"] {
                            ui.selectable_value(
                                &mut self.settings_file.general.theme,
                                t.to_string(),
                                t,
                            );
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Default export:");
                egui::ComboBox::from_id_salt("settings_export_fmt")
                    .selected_text(&self.settings_file.general.default_export_format)
                    .show_ui(ui, |ui| {
                        for f in ["pdf", "svg", "bundle"] {
                            ui.selectable_value(
                                &mut self.settings_file.general.default_export_format,
                                f.to_string(),
                                f,
                            );
                        }
                    });
            });
            ui.label(
                egui::RichText::new(
                    "Built-in: OS keychain for API keys · Firecrawl incremental build · \
                     ERC strict · bus tool · LCSC catalog · auto-add parts to BOM · \
                     open/reveal after export",
                )
                .small()
                .weak(),
            );
        });

        ui.add_space(8.0);
        crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
            ui.label(egui::RichText::new("Database").strong());
            let status = match self.db_status {
                tokito::db::DatabaseStatus::Ready => "Ready",
                tokito::db::DatabaseStatus::Starting => "Starting",
                tokito::db::DatabaseStatus::Degraded => "Degraded",
                tokito::db::DatabaseStatus::Error => "Error",
            };
            ui.label(
                egui::RichText::new(format!(
                    "Status: {status} · Data: {}",
                    tokito::settings::postgres_data_dir(&self.settings_file).display()
                ))
                .small()
                .weak(),
            );
            ui.horizontal(|ui| {
                ui.label("Port:");
                ui.add(
                    egui::DragValue::new(&mut self.settings_file.database.embedded_port)
                        .range(1024..=65535),
                );
            });
            ui.horizontal(|ui| {
                ui.label("PG version:");
                ui.add(
                    egui::DragValue::new(&mut self.settings_file.database.pg_embed_version)
                        .range(16..=18),
                );
            });
            ui.label("Custom data directory (empty = default):");
            ui.horizontal(|ui| {
                ui.text_edit_singleline(&mut self.settings_file.database.data_dir);
                if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Browse…").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        self.settings_file.database.data_dir =
                            dir.to_string_lossy().into_owned();
                    }
                }
            });
            ui.horizontal(|ui| {
                if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Test connection")
                    .clicked()
                {
                    let st = self
                        .rt
                        .block_on(tokito::db::test_connection(&self.pool));
                    self.db_status = st;
                    let label = match st {
                        tokito::db::DatabaseStatus::Ready => "connected",
                        _ => "failed",
                    };
                    self.toast_ok(format!("Database {label}"));
                }
                if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Open data folder")
                    .clicked()
                {
                    let dir = tokito::settings::postgres_data_dir(&self.settings_file);
                    let _ = open::that(dir);
                }
                if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Repair database")
                    .clicked()
                {
                    let dir = tokito::settings::postgres_data_dir(&self.settings_file);
                    match tokito::db::repair_cluster_dir(&dir) {
                        Ok(()) => self.toast_ok("Database cluster reset — restart Tokito"),
                        Err(e) => self.set_err(e.to_string()),
                    }
                }
            });
            ui.label(
                egui::RichText::new(
                    "Per-project embedded DB: set database.mode = \"embedded\" in project.toml",
                )
                .small()
                .weak(),
            );
        });

        ui.add_space(8.0);
        crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
            ui.label(egui::RichText::new("AI (Build & Agent)").strong());
            ui.label("xAI API key:");
            ui.add(
                egui::TextEdit::singleline(&mut self.settings_file.ai.xai_api_key)
                    .password(true)
                    .hint_text("Required for Build"),
            );
            ui.label("Firecrawl API key:");
            ui.add(
                egui::TextEdit::singleline(&mut self.settings_file.ai.firecrawl_api_key)
                    .password(true)
                    .hint_text("Required for research"),
            );
            ui.label("Model:");
            ui.text_edit_singleline(&mut self.settings_file.ai.agent_model);
            ui.horizontal(|ui| {
                ui.label("Max iterations:");
                ui.add(egui::DragValue::new(&mut self.settings_file.ai.agent_max_iterations).range(1..=32));
                ui.label("Token budget:");
                ui.add(
                    egui::DragValue::new(&mut self.settings_file.ai.agent_max_llm_tokens)
                        .range(4096..=200_000),
                );
            });
        });

        ui.add_space(8.0);
        crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
            ui.label(egui::RichText::new("Catalog").strong());
            ui.label(
                egui::RichText::new("LCSC catalog search is always enabled.")
                    .small()
                    .weak(),
            );
            ui.label("Nexar client ID (richer catalog metadata):");
            ui.text_edit_singleline(&mut self.settings_file.catalog.nexar_client_id);
            ui.label("Nexar client secret:");
            ui.add(
                egui::TextEdit::singleline(&mut self.settings_file.catalog.nexar_client_secret)
                    .password(true),
            );
        });

        ui.collapsing("Advanced (HTTP server)", |ui| {
            ui.label("HTTP addr:");
            ui.text_edit_singleline(&mut self.settings_file.server.http_addr);
            ui.label("JWT secret:");
            ui.add(
                egui::TextEdit::singleline(&mut self.settings_file.server.jwt_secret)
                    .password(true),
            );
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if crate::ui::widgets::primary_button(ui, chrome.tokens, "Save settings").clicked() {
                self.save_settings(ctx);
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Reset defaults").clicked()
            {
                self.settings_file = tokito::settings::SettingsFile::default();
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Export redacted").clicked()
            {
                let text = tokito::settings::export_redacted(&self.settings_file);
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("tokito-settings-redacted.toml")
                    .save_file()
                {
                    let _ = std::fs::write(path, text);
                    self.toast_ok("Settings exported");
                }
            }
            if self.ai_build_ready {
                ui.label(
                    egui::RichText::new("AI build ready")
                        .small()
                        .color(chrome.tokens.accent),
                );
            }
        });
        ui.label(
            egui::RichText::new(format!(
                "File: {} · process env fills empty keys only (CI)",
                tokito::paths::settings_path().display()
            ))
            .small()
            .weak(),
        );
    }

    pub(crate) fn save_settings(&mut self, ctx: &egui::Context) {
        tokito::settings::apply_product_defaults(&mut self.settings_file);
        tokito::secrets::persist_keychain_from_settings(&self.settings_file);
        match tokito::settings::save_file(&self.settings_file) {
            Ok(()) => {
                if let Ok(cfg) = self.settings_file.to_config() {
                    self.ai_build_ready = cfg.xai.is_some() && cfg.firecrawl.is_some();
                    if let Ok(st) = AppState::try_new(self.pool.clone(), &cfg) {
                        self.state = st;
                    }
                }
                let eff = theme::effective_theme(&self.settings_file.general.theme);
                theme::apply_with_theme(ctx, &eff);
                self.ui_tokens = theme::tokens_for(&self.settings_file.general.theme);
                self.log_console(
                    "Settings saved — theme applies immediately; API keys reload for Build/Agent.",
                );
                self.toast_ok("Settings saved");
                self.err = None;
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }
}
