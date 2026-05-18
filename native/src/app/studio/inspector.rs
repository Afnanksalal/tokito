use crate::app::studio::chrome::TabChrome;
use crate::app::App;
use crate::editor::CanvasTool;
use tokito::models::NetLabelKind;

impl App {
    pub(crate) fn render_studio_inspector_tab(&mut self, ui: &mut egui::Ui) {
        let tokens = self.ui_tokens;
        let chrome = TabChrome::begin(ui, &tokens);
        chrome.header(ui, "Properties", None);

        ui.horizontal(|ui| {
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Delete").clicked() {
                self.delete_selected();
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Run ERC").clicked() {
                self.run_erc_on_editor();
            }
        });
        ui.add_space(8.0);

        let nothing_selected =
            self.editor.selection_count() == 0 && self.editor.wire_drag_from.is_none();

        if nothing_selected {
            chrome.subsection(ui, "Document");
            crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                crate::ui::layout::inspector_row(ui, chrome.tokens, "Type", "Schematic");
                let sheet = self
                    .editor
                    .sheets
                    .iter()
                    .find(|s| s.id == self.editor.active_sheet_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("root");
                crate::ui::layout::inspector_row(ui, chrome.tokens, "Active sheet", sheet);
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Default label scope").small().weak());
                egui::ComboBox::from_id_salt("label_kind")
                    .selected_text(format!("{:?}", self.editor.label_kind))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.editor.label_kind,
                            NetLabelKind::Local,
                            "Local",
                        );
                        ui.selectable_value(
                            &mut self.editor.label_kind,
                            NetLabelKind::Global,
                            "Global",
                        );
                        ui.selectable_value(
                            &mut self.editor.label_kind,
                            NetLabelKind::Hierarchical,
                            "Hierarchical",
                        );
                    });
            });
            return;
        }

        if let Some(i) = self.editor.selected_segment {
            let net_name = self
                .editor
                .wire_segments
                .get(i)
                .map(|s| s.net.clone())
                .unwrap_or_default();
            chrome.subsection(ui, "Wire");
            let mut net = net_name.clone();
            crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                crate::ui::layout::inspector_row(ui, chrome.tokens, "Net", net_name.clone());
                ui.label(egui::RichText::new("Rename net").small().weak());
                let _ = ui.text_edit_singleline(&mut net);
            });
            if net != net_name {
                self.before_canvas_edit();
                if let Some(s) = self.editor.wire_segments.get_mut(i) {
                    s.net = net;
                }
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Remove segment").clicked() {
                self.before_canvas_edit();
                if i < self.editor.wire_segments.len() {
                    self.editor.wire_segments.remove(i);
                }
                self.editor.selected_segment = None;
                self.editor.selected_segments.clear();
            }
        } else if let Some(r) = self.editor.selected_sym.clone() {
            if let Some(idx) = self.editor.symbols.iter().position(|s| s.ref_des == r) {
                let rd = self.editor.symbols[idx].ref_des.clone();
                let prefix: String = rd.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
                let value_hint = crate::component_value::value_placeholder_for_prefix(&prefix);
                let mut value = self.editor.symbols[idx].value.clone();
                chrome.subsection(ui, "Symbol");
                crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                    crate::ui::layout::inspector_row(ui, chrome.tokens, "RefDes", rd.clone());
                    ui.label(egui::RichText::new("Value").small().weak());
                    ui.add(
                        egui::TextEdit::singleline(&mut value)
                            .hint_text(value_hint)
                            .desired_width(ui.available_width()),
                    );
                    let pid = self.editor.symbols[idx]
                        .part_id
                        .and_then(|id| self.part_cache.get(&id).cloned())
                        .unwrap_or_else(|| "—".to_string());
                    crate::ui::layout::inspector_row(ui, chrome.tokens, "MPN", pid);
                    ui.label(egui::RichText::new("Footprint").small().weak());
                    if let Some(fp) = &mut self.editor.symbols[idx].footprint_ref {
                        ui.text_edit_singleline(fp);
                    } else {
                        let mut fp = String::new();
                        if ui.text_edit_singleline(&mut fp).changed() && !fp.is_empty() {
                            self.editor.symbols[idx].footprint_ref = Some(fp);
                        }
                    }
                    crate::ui::layout::inspector_row(
                        ui,
                        chrome.tokens,
                        "Pins",
                        crate::canvas::display_pins_for_symbol(
                            &self.editor.symbols[idx],
                            &self.editor.wire_segments,
                        )
                        .join(", "),
                    );
                });
                if value != self.editor.symbols[idx].value {
                    self.before_canvas_edit();
                    self.editor.symbols[idx].value = value;
                }
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    if crate::ui::layout::filter_chip(ui, chrome.tokens, "Rotate 90°", false) {
                        self.editor.rotate_selected_symbols(90.0);
                    }
                    if crate::ui::layout::filter_chip(ui, chrome.tokens, "Mirror X", false) {
                        self.editor.mirror_selected_symbols_x();
                    }
                    if crate::ui::layout::filter_chip(ui, chrome.tokens, "Duplicate", false) {
                        self.duplicate_selection();
                    }
                    if crate::ui::layout::filter_chip(ui, chrome.tokens, "Wire pin 1", false) {
                        self.editor.wire_drag_from = Some(crate::canvas::PinEndpoint {
                            ref_des: rd.clone(),
                            pin_name: "1".to_string(),
                        });
                        self.editor.tool = CanvasTool::Wire;
                    }
                    if crate::ui::layout::filter_chip(ui, chrome.tokens, "Remove", false) {
                        self.editor.selected_sym = None;
                        self.editor.selected_syms.remove(&rd);
                        self.before_canvas_edit();
                        self.editor.symbols.retain(|s| s.ref_des != rd);
                    }
                });
            }
        } else if let Some(i) = self.editor.selected_net_label {
            if i < self.editor.net_labels.len() {
                chrome.subsection(ui, "Net label");
                crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                    ui.label(egui::RichText::new("Name").small().weak());
                    let mut name = self.editor.net_labels[i].name.clone();
                    if ui.text_edit_singleline(&mut name).changed() {
                        self.before_canvas_edit();
                        if let Some(label) = self.editor.net_labels.get_mut(i) {
                            label.name = name;
                        }
                    }
                    ui.label(egui::RichText::new("Kind").small().weak());
                    let mut kind = self.editor.net_labels[i].kind;
                    egui::ComboBox::from_id_salt("inspector_label_kind")
                        .selected_text(format!("{kind:?}"))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut kind, NetLabelKind::Local, "Local");
                            ui.selectable_value(&mut kind, NetLabelKind::Global, "Global");
                            ui.selectable_value(
                                &mut kind,
                                NetLabelKind::Hierarchical,
                                "Hierarchical",
                            );
                        });
                    if kind != self.editor.net_labels[i].kind {
                        self.before_canvas_edit();
                        self.editor.net_labels[i].kind = kind;
                    }
                    ui.label(egui::RichText::new("Rotation °").small().weak());
                    let mut rot = self.editor.net_labels[i].rotation_deg;
                    if ui.add(egui::DragValue::new(&mut rot).range(0.0..=270.0).speed(1.0)).changed()
                    {
                        self.before_canvas_edit();
                        self.editor.net_labels[i].rotation_deg = rot;
                    }
                });
            }
        } else if self.editor.wire_drag_from.is_some() {
            chrome.empty(
                ui,
                "Finish wiring: click a pin or canvas point. Esc cancels.",
            );
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Cancel wire").clicked() {
                self.editor.wire_drag_from = None;
                self.editor.wire_chain_last = None;
            }
        } else {
            chrome.empty(
                ui,
                "Multiple objects selected. Use the canvas to refine selection.",
            );
        }
    }
}
