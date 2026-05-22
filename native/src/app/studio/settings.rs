//! Application settings — a modal dialog over `settings.toml`, auto-saved.
//!
//! Settings used to be an `egui_dock` tab; the Tokito UI migration (board #26)
//! makes it a centred modal opened from the studio top bar. Every control
//! writes straight to `self.settings_file` and the change is committed to disk
//! (and the keychain) the moment a control loses focus or toggles — there is
//! no explicit Save button.

use crate::app::App;
use tokito::config::AiProvider;
use tokito::router::AppState;
use tokito_ui::components as c;
use tokito_ui::{icons, Tokens};

/// Which section the settings sidebar has selected.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    General,
    Ai,
    Catalog,
    Editor,
    Database,
}

impl SettingsSection {
    const ALL: [SettingsSection; 5] = [
        SettingsSection::General,
        SettingsSection::Ai,
        SettingsSection::Catalog,
        SettingsSection::Editor,
        SettingsSection::Database,
    ];

    fn label(self) -> &'static str {
        match self {
            SettingsSection::General => "General",
            SettingsSection::Ai => "AI & Build",
            SettingsSection::Catalog => "Catalog & Parts",
            SettingsSection::Editor => "Editor",
            SettingsSection::Database => "Database",
        }
    }
}

const MODAL_W_MAX: f32 = 780.0;
const SIDEBAR_W: f32 = 176.0;
const BODY_H_MAX: f32 = 430.0;
const FIELD_W: f32 = 380.0;

impl App {
    /// Render the settings modal when `show_settings` is set. Composes
    /// `tokito_ui` primitives; commits to disk on any change or on close.
    pub(crate) fn show_settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }
        let theme = crate::theme::effective_theme(&self.settings_file.general.theme);
        let t = Tokens::from_name(&theme);
        let mut open = true;
        self.settings_dirty = false;

        // Size the modal to the window so it always keeps a margin around it.
        let screen = ctx.screen_rect();
        let modal_w = (screen.width() - 200.0).clamp(440.0, MODAL_W_MAX);
        let body_h = (screen.height() - 260.0).clamp(260.0, BODY_H_MAX);

        c::modal(ctx, &t, &mut open, "Settings", modal_w, |ui| {
            self.render_settings_body(ui, t, body_h);
        });

        // The modal clears `open` on Esc / backdrop / X; the Done button
        // clears `show_settings` directly. Either way, commit on the way out.
        let closing = !open || !self.show_settings;
        if !open {
            self.show_settings = false;
        }
        if closing || self.settings_dirty {
            self.commit_settings();
        }
    }

    fn render_settings_body(&mut self, ui: &mut egui::Ui, t: Tokens, body_h: f32) {
        // Body: a fixed-height row of [sidebar | divider | scrolling content].
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), body_h),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(SIDEBAR_W, body_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.add_space(2.0);
                        ui.spacing_mut().item_spacing.y = 4.0;
                        for section in SettingsSection::ALL {
                            let selected = self.settings_section == section;
                            if c::nav_item(ui, &t, section.label(), selected).clicked() {
                                self.settings_section = section;
                            }
                        }
                    },
                );
                ui.separator();
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), body_h),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                egui::Frame::none()
                                    .inner_margin(egui::Margin::symmetric(20.0, 6.0))
                                    .show(ui, |ui| {
                                        ui.spacing_mut().item_spacing.y = 9.0;
                                        match self.settings_section {
                                            SettingsSection::General => {
                                                self.settings_general(ui, &t)
                                            }
                                            SettingsSection::Ai => self.settings_ai(ui, &t),
                                            SettingsSection::Catalog => {
                                                self.settings_catalog(ui, &t)
                                            }
                                            SettingsSection::Editor => self.settings_editor(ui, &t),
                                            SettingsSection::Database => {
                                                self.settings_database(ui, &t)
                                            }
                                        }
                                    });
                            });
                    },
                );
            },
        );

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Settings auto-save as you change them")
                    .size(12.0)
                    .color(t.text_3),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if c::text_button(ui, &t, c::ButtonKind::Primary, "Done", 34.0).clicked() {
                    self.show_settings = false;
                }
            });
        });
    }

    // ----- sections ---------------------------------------------------------

    fn settings_general(&mut self, ui: &mut egui::Ui, t: &Tokens) {
        group_label(ui, t, "Appearance");
        field_label(ui, t, "Theme");
        let themes = ["system", "light", "dark"];
        let mut idx = themes
            .iter()
            .position(|&x| x == self.settings_file.general.theme)
            .unwrap_or(0);
        if c::segmented(ui, t, &["System", "Light", "Dark"], &mut idx, FIELD_W).changed() {
            self.settings_file.general.theme = themes[idx].to_string();
            let eff = crate::theme::effective_theme(&self.settings_file.general.theme);
            crate::theme::apply_with_theme(ui.ctx(), &eff);
            self.ui_tokens = crate::theme::tokens_for(&self.settings_file.general.theme);
            self.settings_dirty = true;
        }

        ui.add_space(8.0);
        group_label(ui, t, "Exports");
        field_label(ui, t, "Default export format");
        let fmts = [("pdf", "PDF"), ("svg", "SVG"), ("bundle", "Bundle")];
        let current = fmts
            .iter()
            .find(|(v, _)| *v == self.settings_file.general.default_export_format)
            .map_or("PDF", |(_, l)| *l);
        c::select(ui, t, "set_export_fmt", current, FIELD_W, |ui| {
            for (value, label) in fmts {
                let on = self.settings_file.general.default_export_format == value;
                if c::select_option(ui, t, label, on) {
                    self.settings_file.general.default_export_format = value.to_string();
                    self.settings_dirty = true;
                }
            }
        });
        ui.add_space(4.0);
        if c::checkbox(
            ui,
            t,
            &mut self.settings_file.general.export_open_after_save,
            "Open export after save",
            None,
        )
        .changed()
        {
            self.settings_dirty = true;
        }
        if c::checkbox(
            ui,
            t,
            &mut self.settings_file.general.export_reveal_in_folder,
            "Reveal export in folder",
            None,
        )
        .changed()
        {
            self.settings_dirty = true;
        }
    }

    fn settings_ai(&mut self, ui: &mut egui::Ui, t: &Tokens) {
        if self.settings_file.ai.llm_api_key.trim().is_empty() {
            c::banner(
                ui,
                t,
                c::BannerKind::Danger,
                icons::ph::WARNING_CIRCLE,
                "Missing API key",
                "Add an API key to enable AI features",
            );
        } else {
            c::banner(
                ui,
                t,
                c::BannerKind::Success,
                icons::ph::CHECK_CIRCLE,
                "API key set",
                "Build and Agent are configured",
            );
        }

        field_label(ui, t, "AI Provider");
        let providers = [
            ("xai", "xAI"),
            ("openai", "OpenAI"),
            ("anthropic", "Claude"),
            ("gemini", "Gemini"),
            ("kimi", "Kimi"),
        ];
        let current = provider_label(&self.settings_file.ai.provider);
        c::select(ui, t, "set_ai_provider", current, FIELD_W, |ui| {
            for (id, label) in providers {
                let on = self.settings_file.ai.provider == id;
                if c::select_option(ui, t, label, on) {
                    self.settings_file.ai.provider = id.to_string();
                    let p = AiProvider::parse(id);
                    self.settings_file.ai.llm_base_url = p.default_base_url().to_string();
                    self.settings_file.ai.agent_model = p.default_model().to_string();
                    self.settings_dirty = true;
                }
            }
        });

        field_label(ui, t, "API Key");
        if c::secret_input(
            ui,
            t,
            "ai_api_key",
            &mut self.settings_file.ai.llm_api_key,
            "Required for Build and Agent",
            FIELD_W,
        )
        .lost_focus()
        {
            self.settings_dirty = true;
        }

        field_label(ui, t, "Model");
        if c::text_input(
            ui,
            t,
            "ai_model",
            &mut self.settings_file.ai.agent_model,
            "Provider default",
            FIELD_W,
        )
        .lost_focus()
        {
            self.settings_dirty = true;
        }

        field_label(ui, t, "Firecrawl API key");
        if c::secret_input(
            ui,
            t,
            "ai_firecrawl_key",
            &mut self.settings_file.ai.firecrawl_api_key,
            "Required for research",
            FIELD_W,
        )
        .lost_focus()
        {
            self.settings_dirty = true;
        }

        ui.add_space(4.0);
        c::collapsing(ui, t, "ai_advanced", "Advanced options", |ui| {
            field_label(ui, t, "Provider base URL");
            if c::text_input(
                ui,
                t,
                "ai_base_url",
                &mut self.settings_file.ai.llm_base_url,
                "Provider default",
                FIELD_W,
            )
            .lost_focus()
            {
                self.settings_dirty = true;
            }
            field_label(ui, t, "Firecrawl base URL");
            if c::text_input(
                ui,
                t,
                "ai_firecrawl_url",
                &mut self.settings_file.ai.firecrawl_base_url,
                "",
                FIELD_W,
            )
            .lost_focus()
            {
                self.settings_dirty = true;
            }
        });
    }

    fn settings_catalog(&mut self, ui: &mut egui::Ui, t: &Tokens) {
        ui.add_space(2.0);
        if c::checkbox(
            ui,
            t,
            &mut self.settings_file.catalog.lcsc_anonymous_search,
            "Enable LCSC anonymous search",
            Some("Search parts without an account"),
        )
        .changed()
        {
            self.settings_dirty = true;
        }

        ui.add_space(6.0);
        group_label(ui, t, "Nexar (optional)");
        field_label(ui, t, "Client ID");
        if c::text_input(
            ui,
            t,
            "nexar_id",
            &mut self.settings_file.catalog.nexar_client_id,
            "Optional — for enhanced part data",
            FIELD_W,
        )
        .lost_focus()
        {
            self.settings_dirty = true;
        }
        field_label(ui, t, "Client Secret");
        if c::secret_input(
            ui,
            t,
            "nexar_secret",
            &mut self.settings_file.catalog.nexar_client_secret,
            "Optional",
            FIELD_W,
        )
        .lost_focus()
        {
            self.settings_dirty = true;
        }
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new("Nexar provides additional part data and availability info.")
                .size(11.0)
                .color(t.text_3),
        );
    }

    fn settings_editor(&mut self, ui: &mut egui::Ui, t: &Tokens) {
        group_label(ui, t, "Validation");
        if c::checkbox(
            ui,
            t,
            &mut self.settings_file.general.erc_strict_mode,
            "Strict ERC mode",
            Some("Treat warnings as errors"),
        )
        .changed()
        {
            self.settings_dirty = true;
        }

        ui.add_space(8.0);
        group_label(ui, t, "Automation");
        if c::checkbox(
            ui,
            t,
            &mut self.settings_file.general.auto_add_placed_parts_to_bom,
            "Auto-add placed parts to BOM",
            Some("Update the BOM when adding components"),
        )
        .changed()
        {
            self.settings_dirty = true;
        }
        if c::checkbox(
            ui,
            t,
            &mut self.settings_file.general.enable_bus_tool,
            "Enable bus tool",
            Some("Show the bus drawing tool in the schematic toolbar"),
        )
        .changed()
        {
            self.settings_dirty = true;
        }
    }

    fn settings_database(&mut self, ui: &mut egui::Ui, t: &Tokens) {
        let (kind, title, body) = match self.db_status {
            tokito::db::DatabaseStatus::Ready => (
                c::BannerKind::Success,
                "Database connected",
                "Embedded PostgreSQL running",
            ),
            tokito::db::DatabaseStatus::Starting => (
                c::BannerKind::Warning,
                "Database starting",
                "Embedded PostgreSQL is starting up",
            ),
            tokito::db::DatabaseStatus::Degraded => (
                c::BannerKind::Warning,
                "Database degraded",
                "Running with reduced reliability",
            ),
            tokito::db::DatabaseStatus::Error => (
                c::BannerKind::Danger,
                "Database error",
                "Embedded PostgreSQL is not available",
            ),
        };
        c::banner(ui, t, kind, icons::ph::DATABASE, title, body);
        c::banner(
            ui,
            t,
            c::BannerKind::Info,
            icons::ph::DATABASE,
            "Local-first storage",
            "All your projects and data stay on this machine unless you explicitly export or sync.",
        );

        field_label(ui, t, "Data directory");
        ui.horizontal(|ui| {
            if c::text_input(
                ui,
                t,
                "db_data_dir",
                &mut self.settings_file.database.data_dir,
                "Default location",
                FIELD_W - 70.0,
            )
            .lost_focus()
            {
                self.settings_dirty = true;
            }
            if c::text_button(ui, t, c::ButtonKind::Secondary, "Open", 34.0).clicked() {
                let dir = tokito::settings::postgres_data_dir(&self.settings_file);
                let _ = open::that(dir);
            }
        });

        ui.add_space(4.0);
        c::collapsing(ui, t, "db_advanced", "Advanced database options", |ui| {
            field_label(ui, t, "Embedded port");
            let r = c::text_input(ui, t, "db_port", &mut self.db_port_buf, "15432", 140.0);
            if r.changed() {
                if let Ok(p) = self.db_port_buf.trim().parse::<u16>() {
                    if p >= 1024 {
                        self.settings_file.database.embedded_port = p;
                    }
                }
            }
            if r.lost_focus() {
                self.settings_dirty = true;
            }

            field_label(ui, t, "PostgreSQL version");
            let r = c::text_input(ui, t, "db_pgver", &mut self.db_pgver_buf, "16", 140.0);
            if r.changed() {
                if let Ok(v) = self.db_pgver_buf.trim().parse::<u16>() {
                    if (16..=18).contains(&v) {
                        self.settings_file.database.pg_embed_version = v;
                    }
                }
            }
            if r.lost_focus() {
                self.settings_dirty = true;
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if c::text_button(ui, t, c::ButtonKind::Secondary, "Test connection", 30.0)
                    .clicked()
                {
                    let st = self.rt.block_on(tokito::db::test_connection(&self.pool));
                    self.db_status = st;
                    let label = match st {
                        tokito::db::DatabaseStatus::Ready => "connected",
                        _ => "failed",
                    };
                    self.toast_ok(format!("Database {label}"));
                }
                if c::text_button(ui, t, c::ButtonKind::Secondary, "Repair database", 30.0)
                    .clicked()
                {
                    let dir = tokito::settings::postgres_data_dir(&self.settings_file);
                    match tokito::db::repair_cluster_dir(&dir) {
                        Ok(()) => self.toast_ok("Database cluster reset; restart Tokito"),
                        Err(e) => self.set_err(e.to_string()),
                    }
                }
            });
        });
    }

    /// Persist `settings_file` to disk + the OS keychain, and reload anything
    /// derived from it. Quiet — no toast; auto-save runs this on every change.
    pub(crate) fn commit_settings(&mut self) {
        tokito::settings::apply_product_defaults(&mut self.settings_file);
        tokito::secrets::persist_keychain_from_settings(&self.settings_file);
        match tokito::settings::save_file(&self.settings_file) {
            Ok(()) => {
                if let Ok(cfg) = self.settings_file.to_config() {
                    self.ai_build_ready = cfg.llm.is_some() && cfg.firecrawl.is_some();
                    if let Ok(st) = AppState::try_new(self.pool.clone(), &cfg) {
                        self.state = st;
                    }
                }
                self.err = None;
            }
            Err(e) => self.set_err(e.to_string()),
        }
    }
}

/// A small bold heading above a group of fields ("Appearance", "Exports", …).
fn group_label(ui: &mut egui::Ui, t: &Tokens, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).strong().size(13.5).color(t.text));
    ui.add_space(2.0);
}

/// A muted caption directly above an input.
fn field_label(ui: &mut egui::Ui, t: &Tokens, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text).size(12.0).color(t.text_2));
}

fn provider_label(value: &str) -> &'static str {
    match AiProvider::parse(value) {
        AiProvider::OpenAi => "OpenAI",
        AiProvider::Anthropic => "Claude",
        AiProvider::Gemini => "Gemini",
        AiProvider::Xai => "xAI",
        AiProvider::Kimi => "Kimi",
    }
}
