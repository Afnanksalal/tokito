impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_async_jobs(ctx);

        let tokens = crate::ui::tokens::UiTokens::default();

        egui::TopBottomPanel::top("topbar")
            .frame(egui::Frame::none().fill(tokens.bg_panel))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
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
                                    egui::RichText::new(&d.name)
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
                                    egui::RichText::new("● Unsaved")
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
                                if ui.button("MCAD handoff (.json)").clicked() {
                                    self.export_schematic_file("mcad");
                                    ui.close_menu();
                                }
                            });
                            ui.menu_button("Panels", |ui| {
                                use crate::app::studio_dock::{ensure_tab_visible, StudioTab};
                                for tab in StudioTab::DOCK_TABS {
                                    if ui.button(tab.panel_label()).clicked() {
                                        ensure_tab_visible(&mut self.dock_state, tab);
                                        ui.close_menu();
                                    }
                                }
                            });
                            if crate::ui::widgets::secondary_button(ui, &tokens, "Undo").clicked() {
                                self.undo_canvas();
                            }
                            if crate::ui::widgets::secondary_button(ui, &tokens, "Redo").clicked() {
                                self.redo_canvas();
                            }
                            if ui
                                .add(egui::Button::new("← Projects").fill(tokens.bg_elevated))
                                .clicked()
                            {
                                self.editor.clear_history();
                                self.erc_note = None;
                                self.editor.screen_rect = None;
                                self.generation_rx = None;
                                self.prompt_busy = false;
                                self.route = Route::Projects;
                                self.design = None;
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

        match self.route {
            Route::Projects => {
                if self.projects_need_refresh {
                    self.projects_need_refresh = false;
                    self.reload_projects();
                }
                self.ui_projects(ctx);
            }
            Route::Studio { design_id } => {
                self.show_command_palette(ctx);

                if ctx.input(|i| {
                    i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::P)
                }) {
                    self.command_palette_open = true;
                }

                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.editor.wire_drag_from = None;
                }

                if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
                    self.delete_selected();
                }

                let submit = ctx.input(|i| {
                    i.key_pressed(egui::Key::Enter)
                        && (i.modifiers.ctrl || i.modifiers.command)
                });
                if submit && !self.prompt.trim().is_empty() && !self.prompt_busy {
                    self.run_prompt_draft(design_id, ctx);
                }

                if !ctx.wants_keyboard_input() {
                    if ctx.input(|i| i.key_pressed(egui::Key::Q)) {
                        self.editor.tool = CanvasTool::Select;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::A)) {
                        self.place_generic_symbol("U");
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::W)) {
                        self.editor.tool = CanvasTool::Wire;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::N)) {
                        self.editor.tool = CanvasTool::NetLabel;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::P)) {
                        self.editor.tool = CanvasTool::Power;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::J)) {
                        self.editor.tool = CanvasTool::Junction;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::X)) {
                        self.editor.tool = CanvasTool::NoConnect;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::B)) {
                        self.editor.tool = CanvasTool::Bus;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::T)) {
                        self.editor.tool = CanvasTool::Text;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::H)) {
                        self.editor.tool = CanvasTool::Pan;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::G)) {
                        self.editor.show_grid = !self.editor.show_grid;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::S))
                        && !ctx.input(|i| i.modifiers.ctrl || i.modifiers.command)
                    {
                        self.editor.snap_enabled = !self.editor.snap_enabled;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
                        self.editor.request_zoom_fit();
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::R)) {
                        self.editor.rotate_selected_symbols(90.0);
                    }
                    if ctx.input(|i| {
                        i.key_pressed(egui::Key::D)
                            && (i.modifiers.ctrl || i.modifiers.command)
                    }) {
                        self.duplicate_selection();
                    }

                    let undo = ctx.input(|i| {
                        (i.modifiers.ctrl || i.modifiers.command)
                            && !i.modifiers.shift
                            && i.key_pressed(egui::Key::Z)
                    });
                    let redo = ctx.input(|i| {
                        (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::Y)
                    }) || ctx.input(|i| {
                        (i.modifiers.ctrl || i.modifiers.command)
                            && i.modifiers.shift
                            && i.key_pressed(egui::Key::Z)
                    });
                    if undo {
                        self.undo_canvas();
                    }
                    if redo {
                        self.redo_canvas();
                    }
                }

                self.ui_studio(ctx, design_id);
            }
        }
    }
}
