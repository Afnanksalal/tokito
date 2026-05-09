fn world_bounds_symbols(symbols: &[crate::canvas::Sym]) -> Option<egui::Rect> {
    if symbols.is_empty() {
        return None;
    }
    let mut r = egui::Rect::NOTHING;
    for s in symbols {
        let half = egui::Vec2::new(70.0, 31.0);
        r = r.union(egui::Rect::from_center_size(s.pos, half * 2.0));
    }
    Some(r.expand(28.0))
}

impl App {
    pub(crate) fn render_studio_canvas_tab(&mut self, ui: &mut egui::Ui, _design_id: Uuid) {
        let tokens = crate::ui::tokens::UiTokens::default();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(
                egui::RichText::new("Canvas")
                    .strong()
                    .color(tokens.text_primary),
            );
            ui.separator();
            if ui
                .small_button("Zoom to fit")
                .on_hover_text("Fit all symbols in view (Home)")
                .clicked()
            {
                self.pending_zoom_fit = true;
            }
            ui.separator();
            ui.checkbox(&mut self.show_grid, "Grid");
            ui.checkbox(&mut self.snap_enabled, "Snap");
            ui.separator();
            ui.label(egui::RichText::new("Net:").small().weak());
            ui.add(
                egui::TextEdit::singleline(&mut self.new_wire_net)
                    .desired_width(100.0)
                    .hint_text("NET"),
            );
        });
        ui.add_space(4.0);

        let (rect, resp) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
        self.canvas_screen_rect = Some(rect);
        let origin = rect.min;

        if self.pending_zoom_fit {
            self.pending_zoom_fit = false;
            if let Some(bounds) = world_bounds_symbols(&self.symbols) {
                let w = bounds.width().max(160.0);
                let h = bounds.height().max(100.0);
                let zw = rect.width() / w;
                let zh = rect.height() / h;
                self.viewport.zoom = (zw.min(zh)).clamp(0.25, 2.8);
                let c = bounds.center();
                let target = rect.center();
                self.viewport.pan = target - origin - c.to_vec2() * self.viewport.zoom;
            }
        }

        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll.abs() > 0.0 {
            let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(rect.center());
            let before = self.viewport.screen_to_world(origin, mouse);
            let zoom = (self.viewport.zoom * (1.0 + scroll * 0.0015)).clamp(0.25, 2.8);
            self.viewport.zoom = zoom;
            let after = self.viewport.world_to_screen(origin, before);
            self.viewport.pan += mouse - after;
        }

        let pan_primary_canvas = matches!(self.canvas_tool, CanvasTool::Pan)
            || (matches!(self.canvas_tool, CanvasTool::Select) && self.dragging_sym.is_none());

        if resp.dragged_by(egui::PointerButton::Middle)
            || (resp.dragged_by(egui::PointerButton::Primary) && pan_primary_canvas)
        {
            self.viewport.pan += resp.drag_delta();
        }

        let painter = ui.painter_at(rect);
        let canvas_bg = tokens.bg_canvas;
        painter.rect_filled(rect, 10.0, canvas_bg);

        if self.show_grid {
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
        }

        if let Some(from) = &self.wire_drag_from {
            painter.text(
                rect.min + Vec2::new(14.0, 10.0),
                egui::Align2::LEFT_TOP,
                format!("Wiring from {from} — click target ({})", self.canvas_tool_label()),
                egui::FontId::proportional(13.0),
                tokens.text_secondary,
            );
        }

        for (i, w) in self.wires.iter().enumerate() {
            let a = self.symbols.iter().find(|s| s.ref_des == w.a);
            let b = self.symbols.iter().find(|s| s.ref_des == w.b);
            if let (Some(a), Some(b)) = (a, b) {
                let pa = self.viewport.world_to_screen(origin, a.pos);
                let pb = self.viewport.world_to_screen(origin, b.pos);
                let stroke = if self.selected_wire == Some(i) {
                    Stroke::new(
                        3.0,
                        egui::Color32::from_rgb(230, 235, 250),
                    )
                } else {
                    Stroke::new(
                        1.7,
                        egui::Color32::from_rgba_unmultiplied(130, 155, 210, 190),
                    )
                };
                painter.line_segment([pa, pb], stroke);
                let mid = Pos2::new((pa.x + pb.x) * 0.5, (pa.y + pb.y) * 0.5);
                painter.text(
                    mid,
                    egui::Align2::CENTER_CENTER,
                    &w.net,
                    egui::FontId::monospace(11.0),
                    egui::Color32::from_rgba_unmultiplied(180, 190, 215, 210),
                );
            }
        }

        let pointer = ui.input(|i| i.pointer.interact_pos());
        let mut clicked_any = false;
        let mut pending_wire: Option<(String, String)> = None;
        let mut hovered: Option<String> = None;

        if let Some(mp) = pointer {
            if rect.contains(mp) {
                for s in &self.symbols {
                    let p = self.viewport.world_to_screen(origin, s.pos);
                    let size = Vec2::new(140.0 * self.viewport.zoom, 62.0 * self.viewport.zoom);
                    let r = Rect::from_center_size(p, size);
                    if r.contains(mp) {
                        hovered = Some(s.ref_des.clone());
                        break;
                    }
                }
            }
        }
        self.canvas_hovered_sym = hovered.clone();

        for i in 0..self.symbols.len() {
            let ref_des = self.symbols[i].ref_des.clone();
            let p = self.viewport.world_to_screen(origin, self.symbols[i].pos);
            let size = Vec2::new(140.0 * self.viewport.zoom, 62.0 * self.viewport.zoom);
            let r = Rect::from_center_size(p, size);
            let id = ui.id().with(ref_des.clone());

            let sense = if matches!(self.canvas_tool, CanvasTool::Pan) {
                Sense::click()
            } else {
                Sense::click_and_drag()
            };

            let sym_resp = ui.interact(r, id, sense);

            let selected = self.selected_sym.as_deref() == Some(ref_des.as_str());
            let hovered_here = self.canvas_hovered_sym.as_deref() == Some(ref_des.as_str());
            let ink = if selected {
                egui::Color32::from_rgb(210, 225, 255)
            } else if hovered_here {
                egui::Color32::from_rgb(195, 215, 250)
            } else {
                egui::Color32::from_rgb(175, 195, 230)
            };
            let fill_bg = if selected {
                egui::Color32::from_rgba_unmultiplied(88, 149, 255, 48)
            } else if hovered_here {
                egui::Color32::from_rgba_unmultiplied(88, 149, 255, 22)
            } else {
                egui::Color32::from_rgba_unmultiplied(28, 32, 42, 200)
            };
            painter.rect_filled(r, 10.0, fill_bg);
            let stroke_box = if selected {
                Stroke::new(2.0, tokens.accent)
            } else if hovered_here {
                Stroke::new(1.5, egui::Color32::from_rgba_unmultiplied(140, 175, 240, 220))
            } else {
                Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(75, 85, 110, 160))
            };
            painter.rect(r, 10.0, egui::Color32::TRANSPARENT, stroke_box);

            let kind = crate::symbols_draw::kind_from_refdes(&ref_des);
            let lw = r.width() * 0.38;
            let lh = r.height() * 0.4;
            let stroke_px = 1.35 * self.viewport.zoom;
            if let Some(lib) = &self.kicad_symbols {
                lib.paint_kind_or_fallback(
                    &painter,
                    p,
                    lw,
                    lh,
                    self.symbols[i].rotation_deg,
                    kind,
                    ink,
                    stroke_px,
                );
            } else {
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
            }

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

            // Wire tool: complete connection with second click
            if matches!(self.canvas_tool, CanvasTool::Wire) && sym_resp.clicked() {
                if let Some(from) = &self.wire_drag_from {
                    if from != &ref_des {
                        pending_wire = Some((from.clone(), ref_des.clone()));
                        clicked_any = true;
                    }
                } else {
                    self.wire_drag_from = Some(ref_des.clone());
                    self.selected_wire = None;
                    clicked_any = true;
                }
            } else if matches!(self.canvas_tool, CanvasTool::Select) && sym_resp.clicked() {
                self.selected_sym = Some(ref_des.clone());
                self.selected_wire = None;
                clicked_any = true;
            }

            if matches!(self.canvas_tool, CanvasTool::Select)
                && sym_resp.drag_started()
            {
                self.before_canvas_edit();
                self.dragging_sym = Some(ref_des.clone());
            }
            if matches!(self.canvas_tool, CanvasTool::Select)
                && sym_resp.dragged()
                && self.dragging_sym.as_deref() == Some(ref_des.as_str())
            {
                let delta = sym_resp.drag_delta() / self.viewport.zoom;
                self.symbols[i].pos += delta;
            }
            if sym_resp.drag_stopped() {
                self.dragging_sym = None;
                if self.snap_enabled {
                    self.symbols[i].pos = snap_world_pos(self.symbols[i].pos);
                }
            }

            if matches!(self.canvas_tool, CanvasTool::Select)
                && self.wire_drag_from.is_some()
                && sym_resp.double_clicked()
                && self.wire_drag_from.as_deref() != Some(ref_des.as_str())
            {
                let from = self.wire_drag_from.take().unwrap();
                pending_wire = Some((from, ref_des.clone()));
            }
        }

        if let Some((a, b)) = pending_wire {
            self.before_canvas_edit();
            let net_name = {
                let n = self.new_wire_net.trim();
                if n.is_empty() {
                    "NET".to_string()
                } else {
                    n.to_string()
                }
            };
            self.wires.push(crate::canvas::Wire {
                a,
                b,
                net: net_name,
            });
            self.wire_drag_from = None;
            self.log_console("Added wire.".to_string());
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
    }

    fn canvas_tool_label(&self) -> &'static str {
        match self.canvas_tool {
            CanvasTool::Select => "Select",
            CanvasTool::Wire => "Wire",
            CanvasTool::Pan => "Pan",
        }
    }

    pub(crate) fn render_studio_copilot_tab(&mut self, ui: &mut egui::Ui, design_id: Uuid) {
        let tokens = crate::ui::tokens::UiTokens::default();
        let ty = crate::ui::TypeRamp::default();

        if let Some(d) = &self.design {
            ui.label(ty.title(d.name.clone()).color(tokens.text_primary));
            ui.add_space(4.0);
            ui.label(
                ty.small_weak(
                    "Describe the circuit. Intent is saved; generation runs research → BOM → schematic.",
                )
                .color(tokens.text_muted),
            );
        }
        ui.add_space(10.0);

        ui.label(ty.section("Goal"));
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::multiline(&mut self.prompt)
                .hint_text(
                    "Example: 12 V to 5 V buck converter, 2 A, external diode, enable pin…",
                )
                .desired_rows(12),
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
            .fill(tokens.accent)
            .min_size(egui::vec2(ui.available_width(), 44.0)),
        );
        if gen.clicked() {
            self.run_prompt_draft(design_id, ui.ctx());
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
            ty.small_weak("Shortcut: Ctrl/⌘+Enter generate · Esc cancels wiring · Del deletes")
                .color(tokens.text_muted),
        );
    }

    pub(crate) fn render_studio_inspector_tab(&mut self, ui: &mut egui::Ui) {
        let tokens = crate::ui::tokens::UiTokens::default();
        crate::ui::layout::section_header(ui, &tokens, "Selection", None);

        if ui
            .button(format!("{} Delete (Del)", crate::ui::icons::TRASH))
            .on_hover_text("Remove selected symbol or wire")
            .clicked()
        {
            self.delete_selected();
        }
        ui.add_space(8.0);

        if let Some(i) = self.selected_wire {
            if let Some(w) = self.wires.get(i) {
                crate::ui::layout::inspector_row(ui, &tokens, "Kind", "Wire");
                crate::ui::layout::inspector_row(ui, &tokens, "Net", w.net.clone());
                ui.add_space(6.0);
                if ui.button("Remove wire").clicked() {
                    self.before_canvas_edit();
                    if i < self.wires.len() {
                        self.wires.remove(i);
                    }
                    self.selected_wire = None;
                }
            }
        } else if let Some(r) = self.selected_sym.clone() {
            if let Some(idx) = self.symbols.iter().position(|s| s.ref_des == r) {
                let rd = self.symbols[idx].ref_des.clone();
                crate::ui::layout::inspector_row(ui, &tokens, "RefDes", rd.clone());
                let pid = self.symbols[idx]
                    .part_id
                    .and_then(|id| self.part_cache.get(&id).cloned())
                    .unwrap_or_else(|| "—".to_string());
                crate::ui::layout::inspector_row(ui, &tokens, "MPN", pid);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{} Rotation °", crate::ui::icons::ROTATE))
                            .small(),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.symbols[idx].rotation_deg)
                            .speed(1.0)
                            .range(-360.0..=360.0),
                    );
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Connect / wire").clicked() {
                        self.wire_drag_from = Some(rd.clone());
                        self.selected_wire = None;
                        self.canvas_tool = CanvasTool::Wire;
                        self.log_console("Wiring: pick second symbol (Wire tool).".to_string());
                    }
                    if ui.button("Remove symbol").clicked() {
                        self.selected_sym = None;
                        self.before_canvas_edit();
                        self.symbols.retain(|s| s.ref_des != rd);
                        self.wires.retain(|w| w.a != rd && w.b != rd);
                    }
                });
            }
        } else if self.wire_drag_from.is_some() {
            ui.label(
                egui::RichText::new("Finish wiring by clicking the target symbol (Wire tool or double-click in Select).")
                    .small(),
            );
            if ui.button("Cancel wiring").clicked() {
                self.wire_drag_from = None;
            }
        } else {
            ui.label(
                egui::RichText::new("Select a symbol or wire on the canvas.")
                    .weak()
                    .color(tokens.text_muted),
            );
        }
    }

    pub(crate) fn render_studio_bom_tab(&mut self, ui: &mut egui::Ui, design_id: Uuid) {
        let tokens = crate::ui::tokens::UiTokens::default();

        if self.bom_dirty || self.bom_loaded_for != Some(design_id) {
            self.refresh_bom(design_id);
        }

        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                self.bom_dirty = true;
            }
            ui.label(
                egui::RichText::new(format!("{} lines", self.bom_lines.len()))
                    .small()
                    .weak(),
            );
        });
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .id_salt("studio_bom_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("bom_grid")
                    .num_columns(5)
                    .spacing([10.0, 6.0])
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
                            let short = ts
                                .get(..10)
                                .map(String::from)
                                .unwrap_or_else(|| ts.clone());
                            ui.label(
                                egui::RichText::new(short)
                                    .small()
                                    .weak(),
                            );
                            ui.end_row();
                        }
                    });
                if self.bom_lines.is_empty() {
                    ui.label(
                        egui::RichText::new("No BOM lines yet — run Generate from Copilot.")
                            .weak()
                            .color(tokens.text_muted),
                    );
                }
            });
    }

    pub(crate) fn render_studio_parts_tab(&mut self, ui: &mut egui::Ui) {
        let tokens = crate::ui::tokens::UiTokens::default();
        crate::ui::layout::section_header(ui, &tokens, "Parts catalog", Some("Search org parts DB"));

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
            .id_salt("studio_parts_hits")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for p in self.parts_hits.clone() {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&p.mpn).monospace().strong());
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
                            self.log_console(format!("Placed {}", p.mpn));
                        }
                    });
                    ui.add_space(4.0);
                }
            });
    }

    pub(crate) fn render_studio_console_tab(&mut self, ui: &mut egui::Ui) {
        let tokens = crate::ui::tokens::UiTokens::default();
        ui.horizontal(|ui| {
            if ui.button("Clear").clicked() {
                self.console_lines.clear();
            }
        });
        ui.add_space(4.0);
        egui::ScrollArea::vertical()
            .id_salt("studio_console_scroll")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &self.console_lines {
                    ui.monospace(
                        egui::RichText::new(line)
                            .small()
                            .color(tokens.text_secondary),
                    );
                }
                if self.console_lines.is_empty() {
                    ui.label(
                        egui::RichText::new("Status and errors appear here.")
                            .weak()
                            .color(tokens.text_muted),
                    );
                }
            });
    }

    fn ui_projects(&mut self, ctx: &egui::Context) {
        let tokens = crate::ui::tokens::UiTokens::default();
        let ty = crate::ui::TypeRamp::default();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(ty.title("Tokito").color(tokens.text_primary));
                ui.label(
                    ty.small_weak("Hardware designs workspace")
                        .color(tokens.text_muted),
                );
            });
            ui.add_space(16.0);

            ui.horizontal(|ui| {
                // Left rail
                ui.vertical(|ui| {
                    ui.set_max_width(280.0);
                    egui::Frame::none()
                        .fill(tokens.bg_elevated)
                        .rounding(tokens.radius_md)
                        .inner_margin(egui::Margin::same(14.0))
                        .show(ui, |ui| {
                            ui.label(ty.section("New design"));
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("Name").small().weak());
                            ui.text_edit_singleline(&mut self.new_design_name);
                            ui.add_space(6.0);
                            ui.label(egui::RichText::new("Description").small().weak());
                            ui.text_edit_singleline(&mut self.new_design_desc);
                            ui.add_space(12.0);
                            if crate::ui::widgets::primary_button(ui, &tokens, "Create design").clicked() {
                                let name = self.new_design_name.trim().to_string();
                                if name.is_empty() {
                                    self.set_err("Name is required");
                                } else {
                                    let desc = self.new_design_desc.trim().to_string();
                                    let res = self.rt.block_on(async {
                                        tokito::store::designs::create(
                                            &self.pool,
                                            tokito::models::CreateDesign {
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
                    ui.add_space(14.0);
                    ui.label(
                        ty.small_weak("Tip: pin designs you use often; search filters name + description.")
                            .color(tokens.text_muted),
                    );
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);

                // Table
                ui.vertical(|ui| {
                    ui.set_min_width(520.0);
                    ui.horizontal(|ui| {
                        ui.label(ty.section("Library"));
                        ui.add_space(8.0);
                        ui.add(egui::TextEdit::singleline(&mut self.projects_search).hint_text(
                            "Search…",
                        ));
                        egui::ComboBox::from_id_salt("projects_sort")
                            .selected_text(match self.projects_sort {
                                ProjectsSort::UpdatedDesc => "Updated · newest",
                                ProjectsSort::UpdatedAsc => "Updated · oldest",
                                ProjectsSort::NameAsc => "Name A→Z",
                                ProjectsSort::NameDesc => "Name Z→A",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.projects_sort,
                                    ProjectsSort::UpdatedDesc,
                                    "Updated · newest",
                                );
                                ui.selectable_value(
                                    &mut self.projects_sort,
                                    ProjectsSort::UpdatedAsc,
                                    "Updated · oldest",
                                );
                                ui.selectable_value(
                                    &mut self.projects_sort,
                                    ProjectsSort::NameAsc,
                                    "Name A→Z",
                                );
                                ui.selectable_value(
                                    &mut self.projects_sort,
                                    ProjectsSort::NameDesc,
                                    "Name Z→A",
                                );
                            });
                    });
                    ui.add_space(10.0);

                    let mut rows: Vec<tokito::models::Design> = self
                        .designs
                        .iter()
                        .filter(|d| {
                            let q = self.projects_search.to_lowercase();
                            if q.is_empty() {
                                return true;
                            }
                            d.name.to_lowercase().contains(&q)
                                || d
                                    .description
                                    .as_ref()
                                    .map(|s| s.to_lowercase().contains(&q))
                                    .unwrap_or(false)
                        })
                        .cloned()
                        .collect();

                    match self.projects_sort {
                        ProjectsSort::NameAsc => rows.sort_by(|a, b| a.name.cmp(&b.name)),
                        ProjectsSort::NameDesc => rows.sort_by(|a, b| b.name.cmp(&a.name)),
                        ProjectsSort::UpdatedAsc => {
                            rows.sort_by(|a, b| a.updated_at.cmp(&b.updated_at))
                        }
                        ProjectsSort::UpdatedDesc => {
                            rows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at))
                        }
                    }

                    let pinned: Vec<_> = rows
                        .iter()
                        .filter(|d| self.projects_pinned.contains(&d.id))
                        .cloned()
                        .collect();
                    let recent_ids: HashSet<_> = self.recent_design_ids.iter().copied().collect();
                    let recent: Vec<_> = rows
                        .iter()
                        .filter(|d| {
                            recent_ids.contains(&d.id) && !self.projects_pinned.contains(&d.id)
                        })
                        .cloned()
                        .collect();
                    let mut seen = std::collections::HashSet::<uuid::Uuid>::new();
                    for d in &pinned {
                        seen.insert(d.id);
                    }
                    for d in &recent {
                        seen.insert(d.id);
                    }
                    let others: Vec<_> = rows
                        .into_iter()
                        .filter(|d| !seen.contains(&d.id))
                        .collect();

                    egui::ScrollArea::vertical()
                        .id_salt("projects_scroll")
                        .show(ui, |ui| {
                            if pinned.is_empty()
                                && recent.is_empty()
                                && others.is_empty()
                                && self.designs.is_empty()
                            {
                                ui.label(
                                    egui::RichText::new("No designs yet — create one.")
                                        .weak()
                                        .color(tokens.text_muted),
                                );
                                return;
                            }

                            let render_section = |ui: &mut egui::Ui,
                                                  this: &mut App,
                                                  title: &str,
                                                  designs: &[tokito::models::Design]| {
                                if designs.is_empty() {
                                    return;
                                }
                                ui.label(ty.section(title).color(tokens.text_secondary));
                                ui.add_space(6.0);
                                for d in designs {
                                    egui::Frame::none()
                                        .fill(tokens.bg_elevated)
                                        .rounding(tokens.radius_sm)
                                        .inner_margin(egui::Margin::same(10.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let mut pin = this.projects_pinned.contains(&d.id);
                                                if ui.checkbox(&mut pin, "").changed() {
                                                    if pin {
                                                        this.projects_pinned.insert(d.id);
                                                    } else {
                                                        this.projects_pinned.remove(&d.id);
                                                    }
                                                }
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(&d.name)
                                                            .strong()
                                                            .size(14.0),
                                                    );
                                                    if let Some(desc) = &d.description {
                                                        ui.label(
                                                            egui::RichText::new(
                                                                crate::util::truncate_ui_chars(
                                                                    desc, 140,
                                                                ),
                                                            )
                                                            .weak()
                                                            .small(),
                                                        );
                                                    }
                                                    let ts = d.updated_at.to_rfc3339();
                                                    let short = ts
                                                        .get(..10)
                                                        .map(String::from)
                                                        .unwrap_or_else(|| ts.clone());
                                                    ui.label(
                                                        egui::RichText::new(format!("Updated {short}"))
                                                            .small()
                                                            .weak()
                                                            .color(tokens.text_muted),
                                                    );
                                                });
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        if ui.button("Open").clicked() {
                                                            this.open_design(d.id);
                                                        }
                                                    },
                                                );
                                            });
                                        });
                                    ui.add_space(6.0);
                                }
                                ui.add_space(10.0);
                            };

                            render_section(ui, self, "Pinned", &pinned);
                            render_section(ui, self, "Recent", &recent);
                            render_section(ui, self, "All", &others);
                        });
                });

                ui.add_space(8.0);
            });
        });
    }

    fn ui_studio(&mut self, ctx: &egui::Context, design_id: Uuid) {
        let tokens = crate::ui::tokens::UiTokens::default();

        // CAD tool rail
        egui::SidePanel::left("cad_toolbar")
            .resizable(false)
            .exact_width(46.0)
            .frame(egui::Frame::none().fill(tokens.bg_panel))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    if ui
                        .selectable_label(
                            matches!(self.canvas_tool, CanvasTool::Select),
                            crate::ui::icons::SELECT,
                        )
                        .on_hover_text("Select (Q)")
                        .clicked()
                    {
                        self.canvas_tool = CanvasTool::Select;
                    }
                    ui.add_space(4.0);
                    if ui
                        .selectable_label(matches!(self.canvas_tool, CanvasTool::Wire), crate::ui::icons::WIRE)
                        .on_hover_text("Wire (W)")
                        .clicked()
                    {
                        self.canvas_tool = CanvasTool::Wire;
                    }
                    ui.add_space(4.0);
                    if ui
                        .selectable_label(matches!(self.canvas_tool, CanvasTool::Pan), crate::ui::icons::PAN)
                        .on_hover_text("Pan (H)")
                        .clicked()
                    {
                        self.canvas_tool = CanvasTool::Pan;
                    }
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    if crate::ui::widgets::toolbar_icon_btn(ui, &tokens, "Zoom to fit (Home)", crate::ui::icons::ZOOM_FIT)
                    {
                        self.pending_zoom_fit = true;
                    }
                    ui.add_space(6.0);
                    ui.checkbox(&mut self.show_grid, format!("{} Grid", crate::ui::icons::GRID))
                        .on_hover_text("Toggle grid (G)");
                    ui.add_space(4.0);
                    ui.checkbox(&mut self.snap_enabled, format!("{} Snap", crate::ui::icons::SNAP))
                        .on_hover_text("Toggle snap (S)");
                });
            });

        egui::TopBottomPanel::bottom("studio_status")
            .frame(egui::Frame::none().fill(tokens.bg_panel))
            .default_height(30.0)
            .min_height(28.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    ui.label(
                        egui::RichText::new(format!(
                            "Tool: {} · {}",
                            self.canvas_tool_label(),
                            if self.wire_drag_from.is_some() {
                                "wiring"
                            } else {
                                "ready"
                            }
                        ))
                        .small(),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(
                            "Middle-drag pan · scroll zoom · Ctrl/⌘+Z undo · Ctrl/⌘+Y redo · Home fit · Del delete",
                        )
                        .small()
                        .weak()
                        .color(tokens.text_muted),
                    );
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if crate::ui::widgets::secondary_button(
                    ui,
                    &tokens,
                    format!("{} Save", crate::ui::icons::SAVE),
                )
                .clicked()
                {
                    self.save_schematic(design_id);
                }
                if crate::ui::widgets::secondary_button(
                    ui,
                    &tokens,
                    format!("{} Reload", crate::ui::icons::REFRESH),
                )
                .clicked()
                {
                    self.open_design(design_id);
                }
                if ui.button("Undo").clicked() {
                    self.undo_canvas();
                }
                if ui.button("Redo").clicked() {
                    self.redo_canvas();
                }
            });
            ui.add_space(4.0);

            let mut style = DockStyle::from_egui(ui.style());
            style.separator.extra = 4.0;

            let mut viewer = crate::app::studio_dock::AppDockViewer {
                app: self as *mut App,
                design_id,
            };

            DockArea::new(&mut self.dock_state)
                .style(style)
                .show_inside(ui, &mut viewer);
        });
    }
}
