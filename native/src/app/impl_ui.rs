impl App {
    fn ui_projects(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(32.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Tokito")
                        .size(26.0)
                        .strong()
                        .color(egui::Color32::from_rgb(235, 237, 240)),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Hardware designs")
                        .weak()
                        .color(egui::Color32::from_gray(150)),
                );
            });
            ui.add_space(28.0);

            ui.horizontal(|ui| {
                ui.add_space(56.0);
                ui.vertical(|ui| {
                    ui.set_max_width(520.0);
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(30, 33, 40))
                        .rounding(egui::Rounding::same(12.0))
                        .inner_margin(egui::Margin::same(22.0))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("New design").strong());
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("Name").small().weak());
                            ui.text_edit_singleline(&mut self.new_design_name);
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Description (optional)").small().weak());
                            ui.text_edit_singleline(&mut self.new_design_desc);
                            ui.add_space(14.0);
                            let create = ui.add_sized(
                                [ui.available_width(), 40.0],
                                egui::Button::new(egui::RichText::new("Create design").strong())
                                    .fill(egui::Color32::from_rgb(72, 118, 214)),
                            );
                            if create.clicked() {
                                let name = self.new_design_name.trim().to_string();
                                if name.is_empty() {
                                    self.set_err("Name is required");
                                } else {
                                    let desc = self.new_design_desc.trim().to_string();
                                    let res = self.rt.block_on(async {
                                        tokito::store::designs::create(
                                            &self.pool,
                                            CreateDesign {
                                                name,
                                                description: if desc.is_empty() {
                                                    None
                                                } else {
                                                    Some(desc)
                                                },
                                            },
                                            self.user_id,
                                        )
                                        .await
                                    });
                                    match res {
                                        Ok(d) => {
                                            self.new_design_name = "New design".to_string();
                                            self.new_design_desc.clear();
                                            self.open_design(d.id);
                                        }
                                        Err(e) => self.set_err(e),
                                    }
                                }
                            }
                        });
                });

                ui.add_space(28.0);

                ui.vertical(|ui| {
                    ui.set_min_width(340.0);
                    ui.set_max_width(520.0);
                    ui.label(
                        egui::RichText::new(format!("Your designs · {}", self.designs.len()))
                            .strong(),
                    );
                    ui.add_space(10.0);
                    egui::ScrollArea::vertical()
                        .id_source("projects_scroll")
                        .show(ui, |ui| {
                            if self.designs.is_empty() {
                                ui.label(
                                    egui::RichText::new("No designs yet — create one.")
                                        .weak()
                                        .color(egui::Color32::from_gray(140)),
                                );
                            } else {
                                for d in self.designs.clone() {
                                    let inner = egui::Frame::none()
                                        .fill(egui::Color32::from_rgb(30, 33, 40))
                                        .rounding(egui::Rounding::same(10.0))
                                        .inner_margin(egui::Margin::same(14.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(&d.name)
                                                            .strong()
                                                            .size(15.0),
                                                    );
                                                    if let Some(desc) = &d.description {
                                                        ui.label(
                                                            egui::RichText::new(desc.as_str())
                                                                .weak()
                                                                .small(),
                                                        );
                                                    }
                                                });
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        if ui.button("Open").clicked() {
                                                            self.open_design(d.id);
                                                        }
                                                    },
                                                );
                                            });
                                        });
                                    ui.add_space(8.0);
                                    inner.response.on_hover_text("Open this design");
                                }
                            }
                        });
                });

                ui.add_space(56.0);
            });
        });
    }

    fn ui_studio(&mut self, ctx: &egui::Context, design_id: Uuid) {
        egui::SidePanel::left("copilot")
            .resizable(true)
            .default_width(340.0)
            .min_width(280.0)
            .max_width(440.0)
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(26, 28, 34)))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                if let Some(d) = &self.design {
                    ui.label(
                        egui::RichText::new(&d.name)
                            .strong()
                            .size(18.0)
                            .color(egui::Color32::from_rgb(235, 237, 240)),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Describe the circuit. We save intent, optionally pull datasheets from the web, then generate a schematic you can edit.")
                            .small()
                            .weak()
                            .color(egui::Color32::from_gray(145)),
                    );
                }
                ui.add_space(14.0);

                ui.label(egui::RichText::new("What should this schematic do?").small());
                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::multiline(&mut self.prompt)
                        .hint_text(
                            "Example: 12 V to 5 V buck converter, 2 A, external diode, enable pin…",
                        )
                        .desired_rows(10),
                );

                ui.add_space(10.0);

                let can_gen = !self.prompt_busy && !self.prompt.trim().is_empty();
                let gen_label = if self.prompt_busy {
                    "Working — research → BOM → schematic…"
                } else {
                    "Generate (research → catalog → schematic)"
                };
                let gen = ui.add_enabled(
                    can_gen,
                    egui::Button::new(
                        egui::RichText::new(gen_label)
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(72, 118, 214))
                    .min_size(egui::vec2(ui.available_width(), 44.0)),
                );
                if gen.clicked() {
                    self.run_prompt_draft(design_id, ctx);
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let w = ui.available_width();
                    let save = ui.add_enabled(
                        !self.prompt_busy,
                        egui::Button::new("Save to board").min_size(egui::vec2(w * 0.49, 34.0)),
                    );
                    if save.clicked() {
                        self.save_schematic(design_id);
                    }
                    let reload = ui.add_enabled(
                        !self.prompt_busy,
                        egui::Button::new("Reload board").min_size(egui::vec2(w * 0.49, 34.0)),
                    );
                    if reload.clicked() {
                        self.open_design(design_id);
                    }
                });

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(
                        "Shortcut: Ctrl or ⌘ + Enter · Runs web search + BOM sync first, then drafts the schematic. Esc cancels wiring.",
                    )
                    .small()
                    .weak()
                    .color(egui::Color32::from_gray(120)),
                );

                ui.add_space(18.0);
                egui::CollapsingHeader::new("Parts catalog")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let r = ui.text_edit_singleline(&mut self.parts_query);
                            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                self.search_parts();
                            }
                            if ui.button("Search").clicked() {
                                self.search_parts();
                            }
                        });
                        ui.add_space(6.0);
                        egui::ScrollArea::vertical()
                            .id_source("studio_parts_hits")
                            .max_height(220.0)
                            .show(ui, |ui| {
                                for p in self.parts_hits.clone() {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(
                                                egui::RichText::new(&p.mpn).monospace().strong(),
                                            );
                                            if let Some(d) = &p.description {
                                                ui.label(
                                                    egui::RichText::new(crate::util::truncate_ui_chars(d, 120))
                                                        .small()
                                                        .weak(),
                                                );
                                            }
                                        });
                                        if ui.small_button("Place").clicked() {
                                            self.drop_part_as_symbol(&p);
                                        }
                                    });
                                    ui.add_space(6.0);
                                }
                            });
                    });
            });

        egui::TopBottomPanel::bottom("studio_strip")
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(22, 24, 28)))
            .default_height(56.0)
            .min_height(52.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    ui.label(
                        egui::RichText::new(
                            "Canvas · drag background to pan · scroll to zoom · drag symbols to move",
                        )
                        .small()
                        .weak(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut clear_link = false;
                        let mut start_link: Option<String> = None;
                        let mut remove_sym: Option<String> = None;
                        let mut remove_wire: Option<usize> = None;

                        if let Some(i) = self.selected_wire {
                            if let Some(w) = self.wires.get(i) {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Wire").small());
                                    ui.monospace(&w.net);
                                    if ui.button("Remove wire").clicked() {
                                        remove_wire = Some(i);
                                    }
                                });
                            }
                        } else if let Some(r) = self.selected_sym.clone() {
                            ui.horizontal(|ui| {
                                if let Some(sym) = self.symbols.iter_mut().find(|s| s.ref_des == r) {
                                    ui.label(egui::RichText::new(&sym.ref_des).monospace());
                                    ui.label(egui::RichText::new("Rotate °").small());
                                    ui.add(
                                        egui::DragValue::new(&mut sym.rotation_deg)
                                            .speed(1.0)
                                            .clamp_range(-360.0..=360.0),
                                    );
                                    if ui.button("Connect").clicked() {
                                        start_link = Some(sym.ref_des.clone());
                                    }
                                    if ui.button("Remove").clicked() {
                                        remove_sym = Some(sym.ref_des.clone());
                                    }
                                }
                            });
                        } else if self.wire_drag_from.is_some() {
                            ui.label(egui::RichText::new("Double-click another symbol to finish the connection").small());
                            if ui.button("Cancel").clicked() {
                                clear_link = true;
                            }
                        }

                        if clear_link {
                            self.wire_drag_from = None;
                        }
                        if let Some(s) = start_link {
                            self.wire_drag_from = Some(s);
                            self.selected_wire = None;
                        }
                        if let Some(r) = remove_sym {
                            self.selected_sym = None;
                            self.before_canvas_edit();
                            self.symbols.retain(|s| s.ref_des != r);
                            self.wires.retain(|w| w.a != r && w.b != r);
                        }
                        if let Some(i) = remove_wire {
                            self.before_canvas_edit();
                            if i < self.wires.len() {
                                self.wires.remove(i);
                            }
                            self.selected_wire = None;
                        }
                    });
                });
                ui.add_space(8.0);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Schematic")
                        .strong()
                        .size(16.0),
                );
            });
            ui.add_space(4.0);

            let (rect, resp) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
            self.canvas_screen_rect = Some(rect);
            let origin = rect.min;

            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > 0.0 {
                let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(rect.center());
                let before = self.viewport.screen_to_world(origin, mouse);
                let zoom = (self.viewport.zoom * (1.0 + scroll * 0.0015)).clamp(0.25, 2.8);
                self.viewport.zoom = zoom;
                let after = self.viewport.world_to_screen(origin, before);
                self.viewport.pan += mouse - after;
            }

            if resp.dragged_by(egui::PointerButton::Middle)
                || (resp.dragged_by(egui::PointerButton::Primary) && self.dragging_sym.is_none())
            {
                self.viewport.pan += resp.drag_delta();
            }

            let painter = ui.painter_at(rect);
            let canvas_bg = egui::Color32::from_rgb(18, 19, 23);
            painter.rect_filled(rect, 10.0, canvas_bg);

            let grid = crate::canvas::GRID_PX * self.viewport.zoom;
            let grid_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10);
            if grid >= 8.0 {
                let start_x = (rect.min.x + (self.viewport.pan.x % grid)).floor();
                let start_y = (rect.min.y + (self.viewport.pan.y % grid)).floor();
                let mut x = start_x;
                while x < rect.max.x {
                    painter.line_segment(
                        [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                        Stroke::new(1.0, grid_color),
                    );
                    x += grid;
                }
                let mut y = start_y;
                while y < rect.max.y {
                    painter.line_segment(
                        [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                        Stroke::new(1.0, grid_color),
                    );
                    y += grid;
                }
            }

            if let Some(from) = &self.wire_drag_from {
                painter.text(
                    rect.min + Vec2::new(14.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    format!("Wiring from {} — double-click target symbol", from),
                    egui::FontId::proportional(13.0),
                    egui::Color32::from_rgb(200, 205, 220),
                );
            }

            for (i, w) in self.wires.iter().enumerate() {
                let a = self.symbols.iter().find(|s| s.ref_des == w.a);
                let b = self.symbols.iter().find(|s| s.ref_des == w.b);
                if let (Some(a), Some(b)) = (a, b) {
                    let pa = self.viewport.world_to_screen(origin, a.pos);
                    let pb = self.viewport.world_to_screen(origin, b.pos);
                    let stroke = if self.selected_wire == Some(i) {
                        Stroke::new(2.5, egui::Color32::from_rgb(230, 235, 250))
                    } else {
                        Stroke::new(1.6, egui::Color32::from_rgba_unmultiplied(130, 155, 210, 180))
                    };
                    painter.line_segment([pa, pb], stroke);
                    let mid = Pos2::new((pa.x + pb.x) * 0.5, (pa.y + pb.y) * 0.5);
                    painter.text(
                        mid,
                        egui::Align2::CENTER_CENTER,
                        &w.net,
                        egui::FontId::monospace(11.0),
                        egui::Color32::from_rgba_unmultiplied(180, 190, 215, 200),
                    );
                }
            }

            let pointer = ui.input(|i| i.pointer.interact_pos());

            let mut clicked_any = false;
            let mut pending_wire: Option<(String, String)> = None;

            for i in 0..self.symbols.len() {
                let ref_des = self.symbols[i].ref_des.clone();
                let p = self.viewport.world_to_screen(origin, self.symbols[i].pos);
                let size = Vec2::new(140.0 * self.viewport.zoom, 62.0 * self.viewport.zoom);
                let r = Rect::from_center_size(p, size);
                let id = ui.id().with(ref_des.clone());
                let sym_resp = ui.interact(r, id, Sense::click_and_drag());

                let selected = self.selected_sym.as_deref() == Some(ref_des.as_str());
                let ink = if selected {
                    egui::Color32::from_rgb(200, 220, 255)
                } else {
                    egui::Color32::from_rgb(175, 195, 230)
                };
                let fill_bg = if selected {
                    egui::Color32::from_rgba_unmultiplied(88, 149, 255, 38)
                } else {
                    egui::Color32::from_rgba_unmultiplied(28, 32, 42, 200)
                };
                painter.rect_filled(r, 10.0, fill_bg);
                let stroke_box = if selected {
                    Stroke::new(1.5, egui::Color32::from_rgb(130, 170, 240))
                } else {
                    Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(75, 85, 110, 160))
                };
                painter.rect(r, 10.0, egui::Color32::TRANSPARENT, stroke_box);

                let kind = crate::symbols_draw::kind_from_refdes(&ref_des);
                let lw = r.width() * 0.38;
                let lh = r.height() * 0.4;
                let stroke_px = 1.35 * self.viewport.zoom;
                crate::symbols_draw::paint_symbol_body(
                    &painter,
                    p,
                    lw,
                    lh,
                    self.symbols[i].rotation_deg,
                    kind,
                    ink,
                    stroke_px,
                );

                let z = self.viewport.zoom;
                painter.text(
                    r.center_top() + Vec2::new(0.0, -8.0 * z),
                    egui::Align2::CENTER_BOTTOM,
                    &ref_des,
                    egui::FontId::monospace(12.5 * z),
                    egui::Color32::from_rgb(248, 250, 255),
                );
                let mpn = self.symbols[i]
                    .part_id
                    .and_then(|pid| self.part_cache.get(&pid).cloned())
                    .unwrap_or_else(|| "—".to_string());
                painter.text(
                    r.center_bottom() + Vec2::new(0.0, 10.0 * z),
                    egui::Align2::CENTER_TOP,
                    mpn,
                    egui::FontId::proportional(10.5 * z),
                    egui::Color32::from_gray(155),
                );

                if sym_resp.clicked() {
                    self.selected_sym = Some(ref_des.clone());
                    self.selected_wire = None;
                    clicked_any = true;
                }

                if sym_resp.drag_started() {
                    self.before_canvas_edit();
                    self.dragging_sym = Some(ref_des.clone());
                }
                if sym_resp.dragged() && self.dragging_sym.as_deref() == Some(ref_des.as_str()) {
                    let delta = sym_resp.drag_delta() / self.viewport.zoom;
                    self.symbols[i].pos += delta;
                }
                if sym_resp.drag_stopped() {
                    self.dragging_sym = None;
                    self.symbols[i].pos = snap_world_pos(self.symbols[i].pos);
                }

                if self.wire_drag_from.is_some()
                    && sym_resp.double_clicked()
                    && self.wire_drag_from.as_deref() != Some(ref_des.as_str())
                {
                    let from = self.wire_drag_from.take().unwrap();
                    pending_wire = Some((from, ref_des.clone()));
                }
            }

            if let Some((a, b)) = pending_wire {
                self.before_canvas_edit();
                self.wires.push(Wire {
                    a,
                    b,
                    net: "NET".to_string(),
                });
            }

            if resp.clicked() && !clicked_any {
                if let Some(mp) = pointer {
                    let mut best: Option<(usize, f32)> = None;
                    for (i, w) in self.wires.iter().enumerate() {
                        let a = self.symbols.iter().find(|s| s.ref_des == w.a);
                        let b = self.symbols.iter().find(|s| s.ref_des == w.b);
                        if let (Some(a), Some(b)) = (a, b) {
                            let pa = self.viewport.world_to_screen(origin, a.pos);
                            let pb = self.viewport.world_to_screen(origin, b.pos);
                            let d = crate::util::dist_point_to_segment_px(mp, pa, pb);
                            if d < 12.0 && (best.map(|(_, bd)| d < bd).unwrap_or(true)) {
                                best = Some((i, d));
                            }
                        }
                    }
                    if let Some((wi, _)) = best {
                        self.selected_wire = Some(wi);
                        self.selected_sym = None;
                        clicked_any = true;
                    }
                }
                if !clicked_any {
                    self.selected_sym = None;
                    self.selected_wire = None;
                    self.wire_drag_from = None;
                }
            }
        });
    }
}
