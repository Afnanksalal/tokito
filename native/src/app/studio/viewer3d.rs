//! Board preview — heuristic 3D blocks from schematic symbols with footprints (MCAD handoff aid).

use crate::app::studio::chrome::TabChrome;
use crate::app::App;
use crate::mcad_viewer::scene::placements_from_symbols;

impl App {
    pub(crate) fn render_studio_viewer3d_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let tokens = self.ui_tokens;
        let chrome = TabChrome::begin(ui, &tokens);
        chrome.header(
            ui,
            "Board preview",
            Some("Orbit: drag · pan: right-drag · zoom: scroll — not a PCB editor"),
        );

        ui.label(
            egui::RichText::new(
                "Schematic-only today: this panel draws simple 3D blocks from symbol positions and \
                 footprint names so you can orient the pile of parts and export MCAD JSON. \
                 There is no PCB canvas, routing, or true board 3D yet—that is normal EDA after layout.",
            )
            .small()
            .weak(),
        );
        ui.add_space(8.0);
        let placements = placements_from_symbols(&self.editor.symbols);
        if placements.len() != self.editor.symbols.len() {
            ui.label(
                egui::RichText::new(format!(
                    "{} of {} symbols have footprints assigned",
                    placements.len(),
                    self.editor.symbols.len()
                ))
                .small()
                .weak(),
            );
            ui.add_space(6.0);
        }

        self.mcad_viewer.ui(ui, ctx, &placements);

        ui.add_space(10.0);
        chrome.subsection(ui, "Export");
        ui.horizontal(|ui| {
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "MCAD JSON").clicked() {
                self.export_schematic_file("mcad");
            }
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Netlist").clicked() {
                self.export_schematic_file("sexp_netlist");
            }
        });

        if !placements.is_empty() {
            chrome.subsection(ui, "Footprints");
            egui::ScrollArea::vertical()
                .id_salt("viewer3d_fp_list")
                .max_height(120.0)
                .show(ui, |ui| {
                    for p in &placements {
                        crate::ui::layout::inspector_row(
                            ui,
                            chrome.tokens,
                            &p.ref_des,
                            &p.footprint,
                        );
                    }
                });
        }
    }
}
