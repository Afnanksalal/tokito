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
                            if ui
                                .add(egui::Button::new("Refresh").fill(egui::Color32::from_rgb(
                                    38, 41, 50,
                                )))
                                .clicked()
                            {
                                self.reload_projects();
                            }
                        }
                        Route::Studio { .. } => {
                            if ui
                                .add(egui::Button::new("← Projects").fill(egui::Color32::from_rgb(
                                    38, 41, 50,
                                )))
                                .clicked()
                            {
                                self.canvas_undo.clear();
                                self.canvas_redo.clear();
                                self.erc_note = None;
                                self.canvas_screen_rect = None;
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
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.wire_drag_from = None;
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
                        self.canvas_tool = CanvasTool::Select;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::W)) {
                        self.canvas_tool = CanvasTool::Wire;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::H)) {
                        self.canvas_tool = CanvasTool::Pan;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::G)) {
                        self.show_grid = !self.show_grid;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::S))
                        && !ctx.input(|i| i.modifiers.ctrl || i.modifiers.command)
                    {
                        self.snap_enabled = !self.snap_enabled;
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
                        self.pending_zoom_fit = true;
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
