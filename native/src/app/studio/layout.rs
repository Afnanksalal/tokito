use egui_dock::{DockArea, Style as DockStyle};
use uuid::Uuid;

use crate::app::{studio_dock, App};
use crate::editor::CanvasTool;
use crate::ui::layout::panel_frame;

impl App {
    pub(crate) fn ui_studio(&mut self, ctx: &egui::Context, design_id: Uuid) {
        let tokens = crate::ui::tokens::UiTokens::default();

        // CAD tool rail (leftmost)
        egui::SidePanel::left("cad_toolbar")
            .resizable(false)
            .exact_width(52.0)
            .frame(egui::Frame::none().fill(tokens.bg_panel))
            .show(ctx, |ui| {
                self.render_cad_toolbar(ui, &tokens);
            });

        if !self.canvas_focus_mode {
            egui::SidePanel::left("place_panel")
                .resizable(true)
                .default_width(300.0)
                .width_range(260.0..=420.0)
                .frame(panel_frame(&tokens))
                .show(ctx, |ui| {
                    self.render_studio_place_panel(ui);
                });

            egui::SidePanel::right("properties_panel")
                .resizable(true)
                .default_width(300.0)
                .width_range(260.0..=400.0)
                .frame(panel_frame(&tokens))
                .show(ctx, |ui| {
                    self.render_studio_inspector_tab(ui);
                });
        }

        egui::TopBottomPanel::bottom("studio_status")
            .frame(egui::Frame::none().fill(tokens.bg_panel))
            .exact_height(26.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    let cursor = self
                        .editor
                        .cursor_world
                        .map(|p| format!("{:.0}, {:.0}", p.x, p.y))
                        .unwrap_or_else(|| "—, —".into());
                    ui.label(
                        egui::RichText::new(format!(
                            "X/Y {}   Zoom {:.0}%   {}",
                            cursor,
                            self.editor.viewport.zoom * 100.0,
                            self.editor.tool.label(),
                        ))
                        .small()
                        .color(tokens.text_secondary),
                    );
                    ui.separator();
                    let sel = if self.editor.wire_drag_from.is_some() {
                        "wiring".to_string()
                    } else {
                        let n = self.editor.selection_count();
                        if n == 0 {
                            "none".into()
                        } else {
                            format!("{n} selected")
                        }
                    };
                    if ui
                        .small_button(format!("Filter: {}", self.editor.selection_filter.label()))
                        .on_hover_text("Click to cycle selection filter")
                        .clicked()
                    {
                        self.editor.selection_filter = self.editor.selection_filter.cycle();
                    }
                    ui.label(
                        egui::RichText::new(format!("Selection: {sel}"))
                            .small()
                            .color(tokens.text_muted),
                    );
                    ui.separator();
                    ui.label(egui::RichText::new("Net").small().color(tokens.text_muted));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.editor.new_wire_net)
                            .desired_width(88.0)
                            .hint_text("NET"),
                    );
                    if ui
                        .small_button("Fit")
                        .on_hover_text("Zoom to fit (Home)")
                        .clicked()
                    {
                        self.editor.request_zoom_fit();
                    }
                    if self.studio_dirty {
                        ui.label(
                            egui::RichText::new("● Modified")
                                .small()
                                .color(tokens.warning),
                        );
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(tokens.bg_app))
            .show(ctx, |ui| {
                if self.canvas_focus_mode {
                    self.render_studio_canvas_tab(ui, design_id);
                    return;
                }

                let mut style = DockStyle::from_egui(ui.style());
                style.separator.extra = 1.5;
                style.tab_bar.fill_tab_bar = true;
                style.tab_bar.bg_fill = tokens.bg_panel;
                style.tab_bar.hline_color = tokens.stroke_subtle.color;
                style.tab.active.bg_fill = tokens.bg_elevated;
                style.tab.active.outline_color = tokens.accent;
                style.tab.inactive.bg_fill = tokens.bg_panel;
                style.tab.inactive.outline_color = tokens.stroke_subtle.color;
                style.tab.hovered.bg_fill = tokens.bg_hover;
                style.tab.tab_body.stroke.width = 0.0;
                style.tab_bar.height = 26.0;
                style.tab.tab_body.inner_margin = egui::Margin::symmetric(10.0, 4.0);

                let mut viewer = studio_dock::AppDockViewer {
                    app: self as *mut App,
                    design_id,
                };

                DockArea::new(&mut self.dock_state)
                    .style(style)
                    .show_add_buttons(true)
                    .show_add_popup(true)
                    .show_inside(ui, &mut viewer);
            });
    }

    fn render_cad_toolbar(&mut self, ui: &mut egui::Ui, tokens: &crate::ui::tokens::UiTokens) {
        use crate::ui::widgets::ToolIcon;
        ui.vertical_centered(|ui| {
            ui.add_space(6.0);
            let tools = [
                (ToolIcon::Select, CanvasTool::Select, "Select (Q)"),
                (ToolIcon::Place, CanvasTool::PlaceSymbol, "Place (A)"),
                (ToolIcon::Wire, CanvasTool::Wire, "Wire (W)"),
                (ToolIcon::NetLabel, CanvasTool::NetLabel, "Net label (N)"),
                (ToolIcon::Power, CanvasTool::Power, "Power (P)"),
                (ToolIcon::Junction, CanvasTool::Junction, "Junction (J)"),
                (ToolIcon::NoConnect, CanvasTool::NoConnect, "No connect (X)"),
                (ToolIcon::Bus, CanvasTool::Bus, "Bus (B)"),
                (ToolIcon::Text, CanvasTool::Text, "Text (T)"),
                (ToolIcon::Pan, CanvasTool::Pan, "Pan (H)"),
            ];
            for (icon, tool, tip) in tools {
                let selected = self.editor.tool == tool
                    || (tool == CanvasTool::PlaceSymbol
                        && matches!(self.editor.tool, CanvasTool::PlaceSymbol));
                if crate::ui::widgets::cad_tool_button(ui, tokens, icon, selected, tip) {
                    if tool == CanvasTool::PlaceSymbol {
                        self.place_generic_symbol("U");
                    } else {
                        self.editor.tool = tool;
                    }
                }
                ui.add_space(3.0);
            }
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);
            if crate::ui::widgets::cad_tool_button(
                ui,
                tokens,
                ToolIcon::Focus,
                self.canvas_focus_mode,
                "Focus canvas",
            ) {
                self.canvas_focus_mode = !self.canvas_focus_mode;
            }
            ui.add_space(3.0);
            if crate::ui::widgets::toolbar_icon_btn(
                ui,
                tokens,
                "Zoom to fit (Home)",
                ToolIcon::ZoomFit,
            ) {
                self.editor.request_zoom_fit();
            }
            ui.add_space(3.0);
            if crate::ui::widgets::cad_tool_button(
                ui,
                tokens,
                ToolIcon::Grid,
                self.editor.show_grid,
                "Grid (G)",
            ) {
                self.editor.show_grid = !self.editor.show_grid;
            }
            ui.add_space(3.0);
            if crate::ui::widgets::cad_tool_button(
                ui,
                tokens,
                ToolIcon::Snap,
                self.editor.snap_enabled,
                "Snap (S)",
            ) {
                self.editor.snap_enabled = !self.editor.snap_enabled;
            }
        });
    }
}
