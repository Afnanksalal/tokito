//! Messages / ERC panel — navigable violations.

use crate::app::studio::chrome::TabChrome;
use crate::app::App;
use tokito::models::ErcSeverity;

impl App {
    pub(crate) fn render_studio_messages_tab(&mut self, ui: &mut egui::Ui) {
        let tokens = self.ui_tokens;
        let chrome = TabChrome::begin(ui, &tokens);
        chrome.header(
            ui,
            "ERC / Messages",
            Some("Electrical rule check — click a row to jump on the schematic"),
        );

        ui.horizontal(|ui| {
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Run ERC").clicked() {
                self.run_erc_on_editor();
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Suggest fixes").clicked() {
                self.suggest_erc_fixes();
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Clear").clicked() {
                self.erc_violations.clear();
                self.editor.erc_markers.clear();
                self.editor.erc_marker_index = None;
            }
            let n = self.erc_violations.len();
            ui.label(
                egui::RichText::new(if n == 0 {
                    "No issues".into()
                } else {
                    format!("{n} issue(s)")
                })
                .small()
                .weak()
                .color(chrome.tokens.text_muted),
            );
        });
        ui.add_space(8.0);

        if self.erc_violations.is_empty() {
            chrome.empty(
                ui,
                "No ERC messages. Save the design or run ERC to analyze.",
            );
            return;
        }

        ui.horizontal(|ui| {
            crate::ui::table::sortable_header(ui, "Code", 0, &mut self.erc_sort);
            crate::ui::table::sortable_header(ui, "Message", 1, &mut self.erc_sort);
            crate::ui::table::sortable_header(ui, "Severity", 2, &mut self.erc_sort);
        });
        ui.add_space(4.0);

        let mut indexed: Vec<(usize, tokito::models::ErcViolation)> = self
            .erc_violations
            .clone()
            .into_iter()
            .enumerate()
            .map(|(i, v)| (i, v))
            .collect();
        indexed.sort_by(|a, b| {
            if self.erc_sort.dir == crate::ui::table::SortDir::None {
                return a.0.cmp(&b.0);
            }
            let ord = match self.erc_sort.column {
                0 => a.1.code.cmp(&b.1.code),
                1 => a.1.message.cmp(&b.1.message),
                2 => format!("{:?}", a.1.severity).cmp(&format!("{:?}", b.1.severity)),
                _ => std::cmp::Ordering::Equal,
            };
            let ord = match self.erc_sort.dir {
                crate::ui::table::SortDir::Asc => ord,
                crate::ui::table::SortDir::Desc => ord.reverse(),
                crate::ui::table::SortDir::None => std::cmp::Ordering::Equal,
            };
            ord.then(a.0.cmp(&b.0))
        });

        egui::ScrollArea::vertical()
            .id_salt("erc_messages")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, v) in indexed {
                    let color = match v.severity {
                        ErcSeverity::Error => chrome.tokens.danger,
                        ErcSeverity::Warning => chrome.tokens.warning,
                        ErcSeverity::Info => chrome.tokens.text_muted,
                    };
                    let selected = self.editor.erc_marker_index == Some(i);
                    crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                        let label = format!("[{}] {}", v.code, v.message);
                        if ui
                            .selectable_label(
                                selected,
                                egui::RichText::new(label).small().color(color),
                            )
                            .clicked()
                        {
                            self.navigate_erc_violation(i);
                        }
                        if let Some(d) = &v.detail {
                            ui.label(egui::RichText::new(d).small().weak());
                        }
                    });
                    ui.add_space(4.0);
                }
            });
    }

    pub(crate) fn run_erc_on_editor(&mut self) {
        self.editor.refresh_wire_connectivity();
        let doc = self.graph_to_document();
        let (body, _) = doc.to_replace_schematic();
        self.erc_violations = tokito::services::schematic_validate::erc_full_with_options(
            &body,
            &doc,
            self.erc_strict(),
        );
        self.set_erc_note_from_slice(&self.erc_violations.clone());
        self.editor.erc_markers = self
            .erc_violations
            .iter()
            .map(|v| crate::editor::live_erc::violation_to_canvas_marker(v, &self.editor))
            .collect();
        self.log_console(format!("ERC: {} issue(s).", self.erc_violations.len()));
    }

    pub(crate) fn navigate_erc_violation(&mut self, index: usize) {
        let Some(v) = self.erc_violations.get(index) else {
            return;
        };
        self.editor.erc_marker_index = Some(index);
        self.editor.clear_selection();
        if let Some(refdes) = &v.instance_ref {
            self.editor.selected_syms.insert(refdes.clone());
            self.editor.selected_sym = Some(refdes.clone());
            self.editor.request_zoom_fit();
        }
        if let Some(net) = &v.net_name {
            let indices = crate::editor::connectivity::segment_indices_for_net(
                net,
                &self.editor.wire_segments,
                &self.editor.net_labels,
            );
            for i in indices {
                self.editor.selected_segments.insert(i);
            }
            if let Some(&first) = self.editor.selected_segments.iter().next() {
                self.editor.selected_segment = Some(first);
            }
        }
        if self.editor.erc_markers.get(index).is_some() {
            self.editor.request_zoom_fit();
        }
    }
}
