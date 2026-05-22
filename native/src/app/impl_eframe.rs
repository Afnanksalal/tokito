impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.log_console("Shutting down database...".to_string());
        self.shutdown_database();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_async_jobs(ctx);
        if self.design_save_debounce > 0.0 {
            self.design_save_debounce -= ctx.input(|i| i.unstable_dt);
            if self.design_save_debounce <= 0.0 {
                if let Route::Studio { design_id } = self.route {
                    if let Some(design) = &self.design {
                        let dirty = self.design_edit_name.trim() != design.name
                            || self.design_edit_desc.trim()
                                != design.description.as_deref().unwrap_or("")
                            || self.design_edit_notes.trim()
                                != design.notes.as_deref().unwrap_or("");
                        if dirty && !self.design_edit_name.trim().is_empty() {
                            self.autosave_design_info(design_id);
                        }
                    }
                }
            }
        }
        self.toasts.show(ctx, &self.ui_tokens);

        let tokens = self.ui_tokens;

        // Studio route owns its own chrome (Save/ERC/Export/Settings/Panels).
        // Projects route has an in-page hero, so no global topbar is rendered.
        if matches!(self.route, Route::Studio { .. }) {
        egui::TopBottomPanel::top("topbar")
            .frame(egui::Frame::none().fill(tokens.bg_panel))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(10.0, 6.0);
                    ui.label(
                        egui::RichText::new("Tokito")
                            .strong()
                            .size(17.0)
                            .color(tokens.text_primary),
                    );

                    match self.route {
                        Route::Projects => {
                            if crate::ui::widgets::secondary_button(ui, &tokens, "Refresh")
                                .clicked()
                            {
                                self.reload_projects();
                            }
                        }
                        Route::Studio { design_id } => {
                            if let Some(d) = &self.design {
                                ui.label(
                                    egui::RichText::new(crate::util::truncate_ui_chars(
                                        &d.name, 36,
                                    ))
                                        .strong()
                                        .color(tokens.text_primary),
                                );
                                ui.separator();
                            }
                            if crate::ui::widgets::secondary_button(ui, &tokens, "Save").clicked() {
                                self.save_schematic(design_id);
                            }
                            if crate::ui::widgets::secondary_button(ui, &tokens, "ERC").clicked() {
                                self.run_erc_on_editor();
                            }
                            if self.studio_dirty {
                                ui.label(
                                    egui::RichText::new("Unsaved")
                                        .small()
                                        .color(tokens.warning),
                                );
                            }
                            ui.menu_button("Export", |ui| {
                                if ui.button("SVG schematic").clicked() {
                                    self.export_schematic_file("svg");
                                    ui.close_menu();
                                }
                                if ui.button("Connectivity netlist (.txt)").clicked() {
                                    self.export_schematic_file("netlist");
                                    ui.close_menu();
                                }
                                if ui.button("S-expression netlist (.net)").clicked() {
                                    self.export_schematic_file("sexp_netlist");
                                    ui.close_menu();
                                }
                                if ui.button("PDF plot (.pdf)").clicked() {
                                    self.export_schematic_file("pdf");
                                    ui.close_menu();
                                }
                                if ui.button("PDF pack (schematic + BOM + ERC)").clicked() {
                                    self.export_schematic_file("pdf_pack");
                                    ui.close_menu();
                                }
                                if ui.button("MCAD handoff (.json)").clicked() {
                                    self.export_schematic_file("mcad");
                                    ui.close_menu();
                                }
                                if ui.button("BOM CSV").clicked() {
                                    self.export_schematic_file("bom_csv");
                                    ui.close_menu();
                                }
                                if ui.button("Export bundle (all files)").clicked() {
                                    self.export_schematic_file("bundle");
                                    ui.close_menu();
                                }
                            });
                            if crate::ui::widgets::secondary_button(ui, &tokens, "Settings").clicked()
                            {
                                self.open_settings();
                            }
                            ui.menu_button("Panels", |ui| {
                                use crate::app::studio_dock::{ensure_tab_visible, StudioTab};
                                ui.set_min_width(220.0);
                                if ui
                                    .selectable_label(self.properties_panel_open, "Properties")
                                    .clicked()
                                {
                                    self.properties_panel_open = !self.properties_panel_open;
                                    ui.close_menu();
                                }
                                ui.separator();
                                egui::ScrollArea::vertical()
                                    .max_height(320.0)
                                    .show(ui, |ui| {
                                        for tab in StudioTab::ADDABLE_TABS {
                                            let open = self.dock_state.find_tab(&tab).is_some();
                                            if ui
                                                .selectable_label(open, tab.panel_label())
                                                .clicked()
                                            {
                                                ensure_tab_visible(&mut self.dock_state, tab);
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                ui.separator();
                                if ui.button("Reset workspace layout").clicked() {
                                    self.dock_state =
                                        crate::app::studio_dock::default_studio_dock();
                                    self.properties_panel_open = false;
                                    ui.close_menu();
                                }
                                if ui.button("Schematic").clicked() {
                                    ensure_tab_visible(&mut self.dock_state, StudioTab::Canvas);
                                    ui.close_menu();
                                }
                            });
                            if crate::ui::widgets::secondary_button(ui, &tokens, "Undo").clicked() {
                                self.undo_canvas();
                            }
                            if crate::ui::widgets::secondary_button(ui, &tokens, "Redo").clicked() {
                                self.redo_canvas();
                            }
                            if ui
                                .add(egui::Button::new("Projects").fill(tokens.bg_elevated))
                                .clicked()
                            {
                                self.editor.clear_history();
                                self.erc_note = None;
                                self.editor.screen_rect = None;
                                self.generation_rx = None;
                                self.prompt_busy = false;
                                self.route = Route::Projects;
                                self.design = None;
                                self.disconnect_project_db();
                                self.projects_need_refresh = true;
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            if let Some(err) = &self.err {
                                ui.label(
                                    egui::RichText::new(err)
                                        .color(tokens.danger)
                                        .small(),
                                );
                            }
                            if let Some(note) = &self.erc_note {
                                ui.label(
                                    egui::RichText::new(note)
                                        .small()
                                        .color(tokens.warning),
                                );
                            }
                        });
                    });
                });
                ui.add_space(8.0);
            });
        }

        match self.route {
            Route::Projects => {
                if self.projects_need_refresh {
                    self.projects_need_refresh = false;
                    self.reload_projects();
                }
                // ⌘K / Ctrl+K opens the project/design quick switcher.
                if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::K)) {
                    self.projects_palette_open = true;
                    self.projects_palette_query.clear();
                }
                self.ui_projects(ctx);
                self.show_projects_palette(ctx);
            }
            Route::Studio { design_id } => {
                self.show_command_palette(ctx);

                if ctx.input(|i| {
                    i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::P)
                }) {
                    self.command_palette_open = true;
                }

                let submit = ctx.input(|i| {
                    i.key_pressed(egui::Key::Enter)
                        && (i.modifiers.ctrl || i.modifiers.command)
                });
                if submit && !self.prompt.trim().is_empty() && !self.prompt_busy {
                    self.run_prompt_draft(design_id, ctx);
                }

                self.ui_studio(ctx, design_id);

                self.show_settings_modal(ctx);

                self.handle_studio_shortcuts(ctx);
            }
        }
    }
}
