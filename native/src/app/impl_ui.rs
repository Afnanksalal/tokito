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
        painter.rect_filled(rect, 0.0, canvas_bg);
        painter.rect_stroke(
            rect.shrink(0.5),
            0.0,
            Stroke::new(1.0, egui::Color32::from_rgb(36, 44, 52)),
        );

        if self.show_grid {
            let grid = crate::canvas::GRID_PX * self.viewport.zoom;
            let grid_color = egui::Color32::from_rgba_unmultiplied(160, 178, 190, 18);
            let major_grid_color = egui::Color32::from_rgba_unmultiplied(160, 178, 190, 38);
            if grid >= 8.0 {
                let start_x = (rect.min.x + (self.viewport.pan.x % grid)).floor();
                let start_y = (rect.min.y + (self.viewport.pan.y % grid)).floor();
                let mut x = start_x;
                let mut ix = 0usize;
                while x < rect.max.x {
                    let color = if ix % 5 == 0 { major_grid_color } else { grid_color };
                    painter.line_segment(
                        [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
                        Stroke::new(1.0, color),
                    );
                    x += grid;
                    ix += 1;
                }
                let mut y = start_y;
                let mut iy = 0usize;
                while y < rect.max.y {
                    let color = if iy % 5 == 0 { major_grid_color } else { grid_color };
                    painter.line_segment(
                        [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                        Stroke::new(1.0, color),
                    );
                    y += grid;
                    iy += 1;
                }
            }
        }

        if let Some(from) = &self.wire_drag_from {
            painter.text(
                rect.min + Vec2::new(14.0, 10.0),
                egui::Align2::LEFT_TOP,
                format!(
                    "Wiring from {}.{} - click target pin ({})",
                    from.ref_des,
                    from.pin_name,
                    self.canvas_tool_label()
                ),
                egui::FontId::proportional(13.0),
                tokens.text_secondary,
            );
        }

        if let Some(from) = &self.wire_drag_from {
            if let (Some(start), Some(mp)) = (self.endpoint_world(from), ui.input(|i| i.pointer.hover_pos())) {
                if rect.contains(mp) {
                    let world = self.viewport.screen_to_world(origin, mp);
                    let end = if self.snap_enabled {
                        snap_world_pos(world)
                    } else {
                        world
                    };
                    for (a, b) in crate::canvas::route_segments(start, &[], end) {
                        painter.line_segment(
                            [
                                self.viewport.world_to_screen(origin, a),
                                self.viewport.world_to_screen(origin, b),
                            ],
                            Stroke::new(1.4, egui::Color32::from_rgb(235, 165, 72)),
                        );
                    }
                    painter.circle_filled(
                        self.viewport.world_to_screen(origin, start),
                        4.0,
                        egui::Color32::from_rgb(235, 165, 72),
                    );
                }
            }
        }

        let highlighted_net = self
            .selected_wire
            .and_then(|i| self.wires.get(i).map(|w| w.net.clone()))
            .or_else(|| {
                self.selected_net_label
                    .and_then(|i| self.net_labels.get(i).map(|l| l.name.clone()))
            });

        for (i, w) in self.wires.iter().enumerate() {
            let a = self.symbols.iter().find(|s| s.ref_des == w.a);
            let b = self.symbols.iter().find(|s| s.ref_des == w.b);
            if let (Some(a), Some(b)) = (a, b) {
                let pa_world = crate::canvas::symbol_pin_world(a, &w.a_pin);
                let pb_world = crate::canvas::symbol_pin_world(b, &w.b_pin);
                let route = crate::canvas::route_segments(pa_world, &w.bends, pb_world);
                let net_highlighted = highlighted_net.as_deref() == Some(w.net.as_str());
                let stroke = if self.selected_wire == Some(i) {
                    Stroke::new(3.0, egui::Color32::from_rgb(235, 165, 72))
                } else if net_highlighted {
                    Stroke::new(2.4, egui::Color32::from_rgb(93, 190, 255))
                } else {
                    Stroke::new(1.7, egui::Color32::from_rgb(154, 178, 198))
                };
                for (sa, sb) in &route {
                    painter.line_segment(
                        [
                            self.viewport.world_to_screen(origin, *sa),
                            self.viewport.world_to_screen(origin, *sb),
                        ],
                        stroke,
                    );
                }
                if self.selected_wire == Some(i) {
                    for (bend_idx, bend) in w.bends.iter().enumerate() {
                        let p = self.viewport.world_to_screen(origin, *bend);
                        let selected_bend = self.selected_wire_bend == Some(bend_idx);
                        painter.circle_filled(
                            p,
                            if selected_bend { 5.0 } else { 4.0 },
                            if selected_bend {
                                egui::Color32::from_rgb(255, 210, 115)
                            } else {
                                egui::Color32::from_rgb(235, 165, 72)
                            },
                        );
                        painter.circle_stroke(
                            p,
                            if selected_bend { 6.5 } else { 5.4 },
                            Stroke::new(1.0, egui::Color32::from_rgb(40, 28, 12)),
                        );
                    }
                }
                let pa = self.viewport.world_to_screen(origin, pa_world);
                let pb = self.viewport.world_to_screen(origin, pb_world);
                painter.circle_filled(pa, 2.5, egui::Color32::from_rgb(154, 178, 198));
                painter.circle_filled(pb, 2.5, egui::Color32::from_rgb(154, 178, 198));
                let mid_world = route
                    .get(route.len() / 2)
                    .map(|(sa, sb)| Pos2::new((sa.x + sb.x) * 0.5, (sa.y + sb.y) * 0.5))
                    .unwrap_or(Pos2::new(
                        (pa_world.x + pb_world.x) * 0.5,
                        (pa_world.y + pb_world.y) * 0.5,
                    ));
                let mid = self.viewport.world_to_screen(origin, mid_world);
                painter.text(
                    mid,
                    egui::Align2::CENTER_CENTER,
                    &w.net,
                    egui::FontId::monospace(11.0),
                    egui::Color32::from_rgb(142, 155, 168),
                );
            }
        }

        for (i, label) in self.net_labels.iter().enumerate() {
            let p = self.viewport.world_to_screen(origin, label.pos);
            let color = if self.selected_net_label == Some(i) {
                egui::Color32::from_rgb(235, 165, 72)
            } else if highlighted_net.as_deref() == Some(label.name.as_str()) {
                egui::Color32::from_rgb(93, 190, 255)
            } else {
                egui::Color32::from_rgb(190, 208, 220)
            };
            painter.text(
                p + Vec2::new(8.0, -8.0),
                egui::Align2::LEFT_CENTER,
                &label.name,
                egui::FontId::monospace(12.0),
                color,
            );
            painter.line_segment([p, p + Vec2::new(22.0, 0.0)], Stroke::new(1.4, color));
        }

        for (i, junction) in self.junctions.iter().enumerate() {
            let p = self.viewport.world_to_screen(origin, junction.pos);
            let color = if self.selected_junction == Some(i) {
                egui::Color32::from_rgb(235, 165, 72)
            } else {
                egui::Color32::from_rgb(93, 190, 255)
            };
            painter.circle_filled(p, 4.0, color);
        }

        for (i, nc) in self.no_connects.iter().enumerate() {
            let p = self.viewport.world_to_screen(origin, nc.pos);
            let color = if self.selected_no_connect == Some(i) {
                egui::Color32::from_rgb(235, 165, 72)
            } else {
                egui::Color32::from_rgb(235, 105, 105)
            };
            painter.line_segment([p + Vec2::new(-5.0, -5.0), p + Vec2::new(5.0, 5.0)], Stroke::new(1.5, color));
            painter.line_segment([p + Vec2::new(-5.0, 5.0), p + Vec2::new(5.0, -5.0)], Stroke::new(1.5, color));
        }

        for (i, pwr) in self.power_symbols.iter().enumerate() {
            let p = self.viewport.world_to_screen(origin, pwr.pos);
            let color = if self.selected_power_symbol == Some(i) {
                egui::Color32::from_rgb(235, 165, 72)
            } else {
                egui::Color32::from_rgb(124, 215, 163)
            };
            painter.line_segment([p, p + Vec2::new(0.0, -18.0)], Stroke::new(1.5, color));
            painter.line_segment([p + Vec2::new(-10.0, -18.0), p + Vec2::new(10.0, -18.0)], Stroke::new(1.5, color));
            painter.line_segment([p + Vec2::new(-6.0, -23.0), p + Vec2::new(6.0, -23.0)], Stroke::new(1.5, color));
            painter.text(
                p + Vec2::new(0.0, -33.0),
                egui::Align2::CENTER_BOTTOM,
                &pwr.name,
                egui::FontId::monospace(11.5),
                color,
            );
        }

        for (i, bus) in self.buses.iter().enumerate() {
            let a = self.viewport.world_to_screen(origin, bus.start);
            let b = self.viewport.world_to_screen(origin, bus.end);
            let color = if self.selected_bus == Some(i) {
                egui::Color32::from_rgb(235, 165, 72)
            } else {
                egui::Color32::from_rgb(174, 128, 235)
            };
            painter.line_segment([a, b], Stroke::new(4.0, color));
            if let Some(name) = &bus.name {
                painter.text(
                    a.lerp(b, 0.5) + Vec2::new(0.0, -10.0),
                    egui::Align2::CENTER_BOTTOM,
                    name,
                    egui::FontId::monospace(11.0),
                    color,
                );
            }
        }

        for (i, text) in self.text_items.iter().enumerate() {
            let p = self.viewport.world_to_screen(origin, text.pos);
            let color = if self.selected_text_item == Some(i) {
                egui::Color32::from_rgb(235, 165, 72)
            } else {
                egui::Color32::from_rgb(206, 214, 222)
            };
            painter.text(
                p,
                egui::Align2::LEFT_TOP,
                &text.text,
                egui::FontId::proportional(13.0),
                color,
            );
        }

        let pointer = ui.input(|i| i.pointer.interact_pos());
        let mut clicked_any = false;
        let mut pending_wire: Option<(crate::canvas::PinEndpoint, crate::canvas::PinEndpoint)> = None;
        let mut hovered: Option<String> = None;
        let mut hovered_pin: Option<crate::canvas::PinEndpoint> = None;

        if let Some(mp) = pointer {
            if rect.contains(mp) {
                for s in &self.symbols {
                    let p = self.viewport.world_to_screen(origin, s.pos);
                    let size = Vec2::new(140.0 * self.viewport.zoom, 62.0 * self.viewport.zoom);
                    let r = Rect::from_center_size(p, size);
                    if r.contains(mp) {
                        hovered = Some(s.ref_des.clone());
                        if hovered_pin.is_none() {
                            let mut best: Option<(crate::canvas::PinEndpoint, f32)> = None;
                            for pin_name in crate::canvas::display_pins_for_symbol(s, &self.wires) {
                                let pin_world = crate::canvas::symbol_pin_world(s, &pin_name);
                                let pin_screen = self.viewport.world_to_screen(origin, pin_world);
                                let d = pin_screen.distance(mp);
                                if best.as_ref().map(|(_, bd)| d < *bd).unwrap_or(true) {
                                    best = Some((
                                        crate::canvas::PinEndpoint {
                                            ref_des: s.ref_des.clone(),
                                            pin_name,
                                        },
                                        d,
                                    ));
                                }
                            }
                            hovered_pin = best.map(|(pin, _)| pin);
                        }
                    }
                    for pin_name in crate::canvas::display_pins_for_symbol(s, &self.wires) {
                        let pin_world = crate::canvas::symbol_pin_world(s, &pin_name);
                        let pin_screen = self.viewport.world_to_screen(origin, pin_world);
                        if pin_screen.distance(mp) <= 9.0 {
                            hovered = Some(s.ref_des.clone());
                            hovered_pin = Some(crate::canvas::PinEndpoint {
                                ref_des: s.ref_des.clone(),
                                pin_name,
                            });
                            break;
                        }
                    }
                    if hovered_pin.is_some() {
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
                    egui::Color32::from_rgb(245, 250, 255)
                } else if hovered_here {
                    egui::Color32::from_rgb(218, 236, 250)
                } else {
                    egui::Color32::from_rgb(205, 216, 225)
                };
            if selected || hovered_here {
                let color = if selected {
                    egui::Color32::from_rgb(93, 190, 255)
                } else {
                    egui::Color32::from_rgba_unmultiplied(93, 190, 255, 150)
                };
                painter.rect_stroke(r.expand(6.0), 0.0, Stroke::new(1.4, color));
            }

            let kind = crate::symbols_draw::kind_from_refdes(&ref_des);
            let lw = r.width() * 0.38;
            let lh = r.height() * 0.4;
            let stroke_px = 1.55 * self.viewport.zoom;
            if let Some(lib) = &self.kicad_symbols {
                lib.paint_kind_or_fallback(
                    &painter,
                    crate::kicad_symbols::SymbolPaintSpec::new(
                        p,
                        lw,
                        lh,
                        self.symbols[i].rotation_deg,
                        kind,
                        ink,
                        stroke_px,
                    ),
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
                egui::Color32::from_rgb(232, 240, 246),
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
                egui::Color32::from_rgb(146, 158, 170),
            );

            for pin_name in crate::canvas::display_pins_for_symbol(&self.symbols[i], &self.wires) {
                let pin_world = crate::canvas::symbol_pin_world(&self.symbols[i], &pin_name);
                let pin_screen = self.viewport.world_to_screen(origin, pin_world);
                let is_hover = hovered_pin.as_ref().map(|p| {
                    p.ref_des == ref_des && p.pin_name == pin_name
                }).unwrap_or(false);
                let pin_color = if is_hover {
                    egui::Color32::from_rgb(235, 165, 72)
                } else {
                    egui::Color32::from_rgb(93, 190, 255)
                };
                painter.circle_filled(pin_screen, if is_hover { 4.5 } else { 3.2 }, pin_color);
                painter.text(
                    pin_screen + Vec2::new(0.0, -9.0),
                    egui::Align2::CENTER_BOTTOM,
                    &pin_name,
                    egui::FontId::monospace(9.5 * z.max(0.8)),
                    egui::Color32::from_rgb(135, 150, 164),
                );
            }

            // Wire tool: complete connection with second click
            if matches!(self.canvas_tool, CanvasTool::Wire) && sym_resp.clicked() {
                if let Some(pin) = hovered_pin.clone().filter(|p| p.ref_des == ref_des) {
                    if let Some(from) = &self.wire_drag_from {
                        if from != &pin {
                            pending_wire = Some((from.clone(), pin));
                            clicked_any = true;
                        }
                    } else {
                        self.wire_drag_from = Some(pin);
                        self.selected_wire = None;
                        clicked_any = true;
                    }
                }
            } else if matches!(self.canvas_tool, CanvasTool::Select) && sym_resp.clicked() {
                self.selected_sym = Some(ref_des.clone());
                self.selected_wire = None;
                self.selected_net_label = None;
                self.selected_junction = None;
                self.selected_no_connect = None;
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
            {
                if let Some(pin) = hovered_pin.clone().filter(|p| p.ref_des == ref_des) {
                    let from = self.wire_drag_from.take().unwrap();
                    if from != pin {
                        pending_wire = Some((from, pin));
                    }
                }
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
                a: a.ref_des.clone(),
                a_pin: a.pin_name.clone(),
                b: b.ref_des.clone(),
                b_pin: b.pin_name.clone(),
                net: net_name,
                bends: {
                    let a_sym = self.symbols.iter().find(|s| s.ref_des == a.ref_des);
                    let b_sym = self.symbols.iter().find(|s| s.ref_des == b.ref_des);
                    match (a_sym, b_sym) {
                        (Some(sa), Some(sb)) => crate::canvas::manhattan_bends(
                            crate::canvas::symbol_pin_world(sa, &a.pin_name),
                            crate::canvas::symbol_pin_world(sb, &b.pin_name),
                        ),
                        _ => vec![],
                    }
                },
            });
            self.wire_drag_from = None;
            self.log_console("Added wire.".to_string());
        }

        if matches!(self.canvas_tool, CanvasTool::Select) {
            if let (Some(wire_idx), Some(mp)) = (self.selected_wire, pointer) {
                let bends = self
                    .wires
                    .get(wire_idx)
                    .map(|wire| wire.bends.clone())
                    .unwrap_or_default();
                for (bend_idx, bend) in bends.iter().enumerate() {
                    let p = self.viewport.world_to_screen(origin, *bend);
                    let id = ui.id().with(("wire_bend", wire_idx, bend_idx));
                    let bend_rect = egui::Rect::from_center_size(p, egui::vec2(16.0, 16.0));
                    let bend_resp = ui.interact(bend_rect, id, Sense::click_and_drag());
                    if bend_resp.clicked() || p.distance(mp) <= 8.0 && resp.clicked() {
                        self.selected_wire_bend = Some(bend_idx);
                        clicked_any = true;
                    }
                    if bend_resp.drag_started() {
                        self.before_canvas_edit();
                        self.dragging_wire_bend = Some((wire_idx, bend_idx));
                        self.selected_wire_bend = Some(bend_idx);
                        clicked_any = true;
                    }
                }
            }
        }

        if let Some((wire_idx, bend_idx)) = self.dragging_wire_bend {
            if let Some(mp) = pointer {
                let mut world = self.viewport.screen_to_world(origin, mp);
                if self.snap_enabled {
                    world = snap_world_pos(world);
                }
                if let Some(wire) = self.wires.get_mut(wire_idx) {
                    if let Some(bend) = wire.bends.get_mut(bend_idx) {
                        *bend = world;
                    }
                }
            }
            if ui.input(|i| i.pointer.any_released()) {
                self.dragging_wire_bend = None;
            }
        }

        if resp.double_clicked() && !clicked_any && matches!(self.canvas_tool, CanvasTool::Select) {
            if let Some(mp) = pointer {
                let mut best: Option<(usize, usize, f32)> = None;
                for (i, w) in self.wires.iter().enumerate() {
                    let a = self.symbols.iter().find(|s| s.ref_des == w.a);
                    let b = self.symbols.iter().find(|s| s.ref_des == w.b);
                    if let (Some(a), Some(b)) = (a, b) {
                        let pa = crate::canvas::symbol_pin_world(a, &w.a_pin);
                        let pb = crate::canvas::symbol_pin_world(b, &w.b_pin);
                        let route = crate::canvas::route_segments(pa, &w.bends, pb);
                        for (seg_idx, (sa, sb)) in route.iter().enumerate() {
                            let d = crate::util::dist_point_to_segment_px(
                                mp,
                                self.viewport.world_to_screen(origin, *sa),
                                self.viewport.world_to_screen(origin, *sb),
                            );
                            if d < 12.0 && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                                best = Some((i, seg_idx, d));
                            }
                        }
                    }
                }
                if let Some((wire_idx, seg_idx, _)) = best {
                    let mut world = self.viewport.screen_to_world(origin, mp);
                    if self.snap_enabled {
                        world = snap_world_pos(world);
                    }
                    self.before_canvas_edit();
                    if let Some(wire) = self.wires.get_mut(wire_idx) {
                        let insert_at = seg_idx.min(wire.bends.len());
                        wire.bends.insert(insert_at, world);
                        self.clear_canvas_selection();
                        self.selected_wire = Some(wire_idx);
                        self.selected_wire_bend = Some(insert_at);
                        clicked_any = true;
                    }
                }
            }
        }

        if resp.clicked() && !clicked_any {
            if let Some(mp) = pointer {
                let world = self.viewport.screen_to_world(origin, mp);
                let world = if self.snap_enabled {
                    snap_world_pos(world)
                } else {
                    world
                };
                match self.canvas_tool {
                    CanvasTool::NetLabel => {
                        self.before_canvas_edit();
                        let name = self.new_wire_net.trim();
                        self.net_labels.push(crate::canvas::NetLabel {
                            name: if name.is_empty() {
                                "NET".to_string()
                            } else {
                                name.to_string()
                            },
                            pos: world,
                        });
                        self.clear_canvas_selection();
                        self.selected_net_label = Some(self.net_labels.len() - 1);
                        clicked_any = true;
                    }
                    CanvasTool::Junction => {
                        self.before_canvas_edit();
                        self.junctions.push(crate::canvas::Junction { pos: world });
                        self.clear_canvas_selection();
                        self.selected_junction = Some(self.junctions.len() - 1);
                        clicked_any = true;
                    }
                    CanvasTool::Power => {
                        self.before_canvas_edit();
                        let name = self.new_wire_net.trim();
                        self.power_symbols.push(crate::canvas::PowerSymbol {
                            name: if name.is_empty() {
                                "VCC".to_string()
                            } else {
                                name.to_string()
                            },
                            pos: world,
                        });
                        self.clear_canvas_selection();
                        self.selected_power_symbol = Some(self.power_symbols.len() - 1);
                        clicked_any = true;
                    }
                    CanvasTool::NoConnect => {
                        self.before_canvas_edit();
                        self.no_connects.push(crate::canvas::NoConnect { pos: world });
                        self.clear_canvas_selection();
                        self.selected_no_connect = Some(self.no_connects.len() - 1);
                        clicked_any = true;
                    }
                    CanvasTool::Bus => {
                        self.before_canvas_edit();
                        self.buses.push(crate::canvas::BusSegment {
                            name: Some(self.new_wire_net.trim().to_string()).filter(|s| !s.is_empty()),
                            start: world,
                            end: world + Vec2::new(120.0, 0.0),
                        });
                        self.clear_canvas_selection();
                        self.selected_bus = Some(self.buses.len() - 1);
                        clicked_any = true;
                    }
                    CanvasTool::Text => {
                        self.before_canvas_edit();
                        self.text_items.push(crate::canvas::TextItem {
                            text: "Note".to_string(),
                            pos: world,
                        });
                        self.clear_canvas_selection();
                        self.selected_text_item = Some(self.text_items.len() - 1);
                        clicked_any = true;
                    }
                    _ => {}
                }
            }
        }

        if resp.clicked() && !clicked_any {
            if let Some(mp) = pointer {
                let mut best: Option<(usize, f32)> = None;
                for (i, w) in self.wires.iter().enumerate() {
                    let a = self.symbols.iter().find(|s| s.ref_des == w.a);
                    let b = self.symbols.iter().find(|s| s.ref_des == w.b);
                    if let (Some(a), Some(b)) = (a, b) {
                        let pa = crate::canvas::symbol_pin_world(a, &w.a_pin);
                        let pb = crate::canvas::symbol_pin_world(b, &w.b_pin);
                        let route = crate::canvas::route_segments(pa, &w.bends, pb);
                        for (sa, sb) in route {
                            let d = crate::util::dist_point_to_segment_px(
                                mp,
                                self.viewport.world_to_screen(origin, sa),
                                self.viewport.world_to_screen(origin, sb),
                            );
                            if d < 12.0 && (best.map(|(_, bd)| d < bd).unwrap_or(true)) {
                                best = Some((i, d));
                            }
                        }
                    }
                }
                if let Some((wi, _)) = best {
                    self.clear_canvas_selection();
                    self.selected_wire = Some(wi);
                    self.selected_wire_bend = None;
                    clicked_any = true;
                }
            }
            if !clicked_any {
                if let Some(mp) = pointer {
                    for (i, label) in self.net_labels.iter().enumerate() {
                        let p = self.viewport.world_to_screen(origin, label.pos);
                        if egui::Rect::from_min_size(
                            p + Vec2::new(4.0, -16.0),
                            Vec2::new((label.name.len() as f32 * 8.0).max(24.0), 22.0),
                        )
                        .contains(mp)
                        {
                            self.clear_canvas_selection();
                            self.selected_net_label = Some(i);
                            clicked_any = true;
                            break;
                        }
                    }
                    if !clicked_any {
                        for (i, junction) in self.junctions.iter().enumerate() {
                            let p = self.viewport.world_to_screen(origin, junction.pos);
                            if p.distance(mp) <= 10.0 {
                                self.clear_canvas_selection();
                                self.selected_junction = Some(i);
                                clicked_any = true;
                                break;
                            }
                        }
                    }
                    if !clicked_any {
                        for (i, nc) in self.no_connects.iter().enumerate() {
                            let p = self.viewport.world_to_screen(origin, nc.pos);
                            if p.distance(mp) <= 10.0 {
                                self.clear_canvas_selection();
                                self.selected_no_connect = Some(i);
                                clicked_any = true;
                                break;
                            }
                        }
                    }
                    if !clicked_any {
                        for (i, pwr) in self.power_symbols.iter().enumerate() {
                            let p = self.viewport.world_to_screen(origin, pwr.pos);
                            if egui::Rect::from_center_size(
                                p + Vec2::new(0.0, -18.0),
                                Vec2::new(48.0, 48.0),
                            )
                            .contains(mp)
                            {
                                self.clear_canvas_selection();
                                self.selected_power_symbol = Some(i);
                                clicked_any = true;
                                break;
                            }
                        }
                    }
                    if !clicked_any {
                        for (i, bus) in self.buses.iter().enumerate() {
                            let a = self.viewport.world_to_screen(origin, bus.start);
                            let b = self.viewport.world_to_screen(origin, bus.end);
                            let d = crate::util::dist_point_to_segment_px(mp, a, b);
                            if d <= 12.0 {
                                self.clear_canvas_selection();
                                self.selected_bus = Some(i);
                                clicked_any = true;
                                break;
                            }
                        }
                    }
                    if !clicked_any {
                        for (i, text) in self.text_items.iter().enumerate() {
                            let p = self.viewport.world_to_screen(origin, text.pos);
                            let width = (text.text.len() as f32 * 8.0).max(36.0);
                            if egui::Rect::from_min_size(p, Vec2::new(width, 22.0)).contains(mp) {
                                self.clear_canvas_selection();
                                self.selected_text_item = Some(i);
                                clicked_any = true;
                                break;
                            }
                        }
                    }
                }
            }
            if !clicked_any {
                self.clear_canvas_selection();
                self.wire_drag_from = None;
            }
        }
    }

    fn canvas_tool_label(&self) -> &'static str {
        match self.canvas_tool {
            CanvasTool::Select => "Select",
            CanvasTool::Wire => "Wire",
            CanvasTool::NetLabel => "Net Label",
            CanvasTool::Power => "Power",
            CanvasTool::Junction => "Junction",
            CanvasTool::NoConnect => "No Connect",
            CanvasTool::Bus => "Bus",
            CanvasTool::Text => "Text",
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
                crate::ui::layout::inspector_row(ui, &tokens, "From", format!("{}.{}", w.a, w.a_pin));
                crate::ui::layout::inspector_row(ui, &tokens, "To", format!("{}.{}", w.b, w.b_pin));
                let same_net_wires = self.wires.iter().filter(|x| x.net == w.net).count();
                let same_net_labels = self.net_labels.iter().filter(|x| x.name == w.net).count();
                crate::ui::layout::inspector_row(
                    ui,
                    &tokens,
                    "Net objects",
                    format!("{same_net_wires} wires, {same_net_labels} labels"),
                );
                crate::ui::layout::inspector_row(
                    ui,
                    &tokens,
                    "Bends",
                    format!("{}", w.bends.len()),
                );
                ui.label(egui::RichText::new("Net").small().weak());
                let mut net = w.net.clone();
                if ui.text_edit_singleline(&mut net).changed() {
                    self.before_canvas_edit();
                    if let Some(w) = self.wires.get_mut(i) {
                        w.net = net;
                    }
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Add bend").clicked() {
                        self.before_canvas_edit();
                        if let Some(wire) = self.wires.get_mut(i) {
                            let bend = wire
                                .bends
                                .last()
                                .copied()
                                .or_else(|| {
                                    let a = self.symbols.iter().find(|s| s.ref_des == wire.a)?;
                                    let b = self.symbols.iter().find(|s| s.ref_des == wire.b)?;
                                    let pa = crate::canvas::symbol_pin_world(a, &wire.a_pin);
                                    let pb = crate::canvas::symbol_pin_world(b, &wire.b_pin);
                                    Some(Pos2::new((pa.x + pb.x) * 0.5, (pa.y + pb.y) * 0.5))
                                })
                                .unwrap_or(Pos2::new(0.0, 0.0));
                            wire.bends.push(if self.snap_enabled {
                                snap_world_pos(bend)
                            } else {
                                bend
                            });
                            self.selected_wire_bend = Some(wire.bends.len() - 1);
                        }
                    }
                    if ui.button("Remove bend").clicked() {
                        if let Some(bend_idx) = self.selected_wire_bend {
                            self.before_canvas_edit();
                            if let Some(wire) = self.wires.get_mut(i) {
                                if bend_idx < wire.bends.len() {
                                    wire.bends.remove(bend_idx);
                                }
                            }
                            self.selected_wire_bend = None;
                        }
                    }
                });
                ui.add_space(4.0);
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
                crate::ui::layout::inspector_row(
                    ui,
                    &tokens,
                    "Pins",
                    crate::canvas::display_pins_for_symbol(&self.symbols[idx], &self.wires)
                        .join(", "),
                );
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
                    if ui.button("Wire from pin 1").clicked() {
                        self.wire_drag_from = Some(crate::canvas::PinEndpoint {
                            ref_des: rd.clone(),
                            pin_name: "1".to_string(),
                        });
                        self.selected_wire = None;
                        self.canvas_tool = CanvasTool::Wire;
                        self.log_console("Wiring: pick target pin.".to_string());
                    }
                    if ui.button("Remove symbol").clicked() {
                        self.selected_sym = None;
                        self.before_canvas_edit();
                        self.symbols.retain(|s| s.ref_des != rd);
                        self.wires.retain(|w| w.a != rd && w.b != rd);
                    }
                });
            }
        } else if let Some(i) = self.selected_net_label {
            if i < self.net_labels.len() {
                crate::ui::layout::inspector_row(ui, &tokens, "Kind", "Net label");
                ui.label(egui::RichText::new("Name").small().weak());
                let mut name = self.net_labels[i].name.clone();
                if ui.text_edit_singleline(&mut name).changed() {
                    self.before_canvas_edit();
                    if let Some(label) = self.net_labels.get_mut(i) {
                        label.name = name;
                    }
                }
                crate::ui::layout::inspector_row(
                    ui,
                    &tokens,
                    "Position",
                    format!("{:.0}, {:.0}", self.net_labels[i].pos.x, self.net_labels[i].pos.y),
                );
            }
        } else if let Some(i) = self.selected_power_symbol {
            if i < self.power_symbols.len() {
                crate::ui::layout::inspector_row(ui, &tokens, "Kind", "Power symbol");
                ui.label(egui::RichText::new("Net").small().weak());
                let mut name = self.power_symbols[i].name.clone();
                if ui.text_edit_singleline(&mut name).changed() {
                    self.before_canvas_edit();
                    if let Some(power) = self.power_symbols.get_mut(i) {
                        power.name = name;
                    }
                }
                crate::ui::layout::inspector_row(
                    ui,
                    &tokens,
                    "Position",
                    format!("{:.0}, {:.0}", self.power_symbols[i].pos.x, self.power_symbols[i].pos.y),
                );
            }
        } else if let Some(i) = self.selected_bus {
            if i < self.buses.len() {
                crate::ui::layout::inspector_row(ui, &tokens, "Kind", "Bus");
                ui.label(egui::RichText::new("Name").small().weak());
                let mut name = self.buses[i].name.clone().unwrap_or_default();
                if ui.text_edit_singleline(&mut name).changed() {
                    self.before_canvas_edit();
                    if let Some(bus) = self.buses.get_mut(i) {
                        bus.name = Some(name).filter(|s| !s.trim().is_empty());
                    }
                }
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("End X").small().weak());
                    ui.add(egui::DragValue::new(&mut self.buses[i].end.x).speed(4.0));
                    ui.label(egui::RichText::new("End Y").small().weak());
                    ui.add(egui::DragValue::new(&mut self.buses[i].end.y).speed(4.0));
                });
            }
        } else if let Some(i) = self.selected_text_item {
            if i < self.text_items.len() {
                crate::ui::layout::inspector_row(ui, &tokens, "Kind", "Text");
                ui.label(egui::RichText::new("Text").small().weak());
                let mut text = self.text_items[i].text.clone();
                if ui.text_edit_singleline(&mut text).changed() {
                    self.before_canvas_edit();
                    if let Some(item) = self.text_items.get_mut(i) {
                        item.text = text;
                    }
                }
                crate::ui::layout::inspector_row(
                    ui,
                    &tokens,
                    "Position",
                    format!("{:.0}, {:.0}", self.text_items[i].pos.x, self.text_items[i].pos.y),
                );
            }
        } else if let Some(i) = self.selected_junction {
            if let Some(j) = self.junctions.get(i) {
                crate::ui::layout::inspector_row(ui, &tokens, "Kind", "Junction");
                crate::ui::layout::inspector_row(
                    ui,
                    &tokens,
                    "Position",
                    format!("{:.0}, {:.0}", j.pos.x, j.pos.y),
                );
            }
        } else if let Some(i) = self.selected_no_connect {
            if let Some(nc) = self.no_connects.get(i) {
                crate::ui::layout::inspector_row(ui, &tokens, "Kind", "No connect");
                crate::ui::layout::inspector_row(
                    ui,
                    &tokens,
                    "Position",
                    format!("{:.0}, {:.0}", nc.pos.x, nc.pos.y),
                );
            }
        } else if self.wire_drag_from.is_some() {
            ui.label(
                egui::RichText::new("Finish wiring by clicking the target pin.")
                    .small(),
            );
            if ui.button("Cancel wiring").clicked() {
                self.wire_drag_from = None;
            }
        } else {
            ui.label(
                egui::RichText::new("Select a symbol, pin wire, label, junction, or no-connect marker.")
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
                        let (preview_rect, _) =
                            ui.allocate_exact_size(egui::vec2(64.0, 44.0), egui::Sense::hover());
                        ui.painter().rect_filled(
                            preview_rect,
                            4.0,
                            egui::Color32::from_rgb(244, 246, 241),
                        );
                        ui.painter().rect_stroke(
                            preview_rect.shrink(0.5),
                            4.0,
                            egui::Stroke::new(
                                1.0,
                                egui::Color32::from_rgba_unmultiplied(20, 30, 40, 32),
                            ),
                        );
                        let prefix = crate::util::guess_prefix(&p.mpn);
                        let kind = crate::symbols_draw::kind_from_refdes(prefix);
                        crate::symbols_draw::paint_symbol_body(
                            ui.painter(),
                            preview_rect.center(),
                            preview_rect.width() * 0.28,
                            preview_rect.height() * 0.28,
                            0.0,
                            kind,
                            egui::Color32::from_rgb(25, 31, 38),
                            1.25,
                        );
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
            .exact_width(50.0)
            .frame(egui::Frame::none().fill(tokens.bg_panel))
            .show(ctx, |ui| {
                use crate::ui::widgets::ToolIcon;
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::Select,
                        matches!(self.canvas_tool, CanvasTool::Select),
                        "Select (Q)",
                    )
                    {
                        self.canvas_tool = CanvasTool::Select;
                    }
                    ui.add_space(4.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::Wire,
                        matches!(self.canvas_tool, CanvasTool::Wire),
                        "Wire (W)",
                    )
                    {
                        self.canvas_tool = CanvasTool::Wire;
                    }
                    ui.add_space(4.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::NetLabel,
                        matches!(self.canvas_tool, CanvasTool::NetLabel),
                        "Net label (N)",
                    )
                    {
                        self.canvas_tool = CanvasTool::NetLabel;
                    }
                    ui.add_space(4.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::Power,
                        matches!(self.canvas_tool, CanvasTool::Power),
                        "Power symbol (P)",
                    )
                    {
                        self.canvas_tool = CanvasTool::Power;
                    }
                    ui.add_space(4.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::Junction,
                        matches!(self.canvas_tool, CanvasTool::Junction),
                        "Junction (J)",
                    )
                    {
                        self.canvas_tool = CanvasTool::Junction;
                    }
                    ui.add_space(4.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::NoConnect,
                        matches!(self.canvas_tool, CanvasTool::NoConnect),
                        "No connect (X)",
                    )
                    {
                        self.canvas_tool = CanvasTool::NoConnect;
                    }
                    ui.add_space(4.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::Bus,
                        matches!(self.canvas_tool, CanvasTool::Bus),
                        "Bus (B)",
                    )
                    {
                        self.canvas_tool = CanvasTool::Bus;
                    }
                    ui.add_space(4.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::Text,
                        matches!(self.canvas_tool, CanvasTool::Text),
                        "Text (T)",
                    )
                    {
                        self.canvas_tool = CanvasTool::Text;
                    }
                    ui.add_space(4.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::Pan,
                        matches!(self.canvas_tool, CanvasTool::Pan),
                        "Pan (H)",
                    )
                    {
                        self.canvas_tool = CanvasTool::Pan;
                    }
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::Focus,
                        self.canvas_focus_mode,
                        "Focus canvas",
                    ) {
                        self.canvas_focus_mode = !self.canvas_focus_mode;
                    }
                    ui.add_space(4.0);
                    if crate::ui::widgets::toolbar_icon_btn(
                        ui,
                        &tokens,
                        "Zoom to fit (Home)",
                        ToolIcon::ZoomFit,
                    )
                    {
                        self.pending_zoom_fit = true;
                    }
                    ui.add_space(6.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::Grid,
                        self.show_grid,
                        "Toggle grid (G)",
                    ) {
                        self.show_grid = !self.show_grid;
                    }
                    ui.add_space(4.0);
                    if crate::ui::widgets::cad_tool_button(
                        ui,
                        &tokens,
                        ToolIcon::Snap,
                        self.snap_enabled,
                        "Toggle snap (S)",
                    ) {
                        self.snap_enabled = !self.snap_enabled;
                    }
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

            if self.canvas_focus_mode {
                self.render_studio_canvas_tab(ui, design_id);
                return;
            }

            let mut style = DockStyle::from_egui(ui.style());
            style.separator.extra = 2.0;
            style.tab_bar.bg_fill = tokens.bg_panel;
            style.tab.active.bg_fill = tokens.bg_elevated;
            style.tab.active.outline_color = tokens.accent;
            style.tab.inactive.bg_fill = egui::Color32::from_rgb(27, 30, 36);
            style.tab.inactive.outline_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 16);
            style.tab.hovered.bg_fill = tokens.bg_hover;

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
