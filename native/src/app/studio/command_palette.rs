//! Command palette (Ctrl+Shift+P).

use crate::app::{App, Route};
use crate::editor::CanvasTool;

impl App {
    pub(crate) fn show_command_palette(&mut self, ctx: &egui::Context) {
        if !self.command_palette_open {
            return;
        }
        let mut chosen: Option<&'static str> = None;
        egui::Window::new("Command palette")
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 48.0])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Esc to close").small().weak());
                ui.add(
                    egui::TextEdit::singleline(&mut self.command_palette_query)
                        .hint_text("Command…")
                        .desired_width(f32::INFINITY),
                );
                ui.separator();
                let q = self.command_palette_query.to_lowercase();
                for (id, label) in COMMAND_LIST {
                    if !q.is_empty() && !label.to_lowercase().contains(&q) {
                        continue;
                    }
                    if ui.button(*label).clicked() {
                        chosen = Some(id);
                    }
                }
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.command_palette_open = false;
        }
        if let Some(id) = chosen {
            self.run_palette_command(id);
            self.command_palette_open = false;
            self.command_palette_query.clear();
        }
    }

    fn run_palette_command(&mut self, id: &str) {
        match id {
            "save" => {
                if let Route::Studio { design_id } = self.route {
                    self.save_schematic(design_id);
                }
            }
            "erc" => self.run_erc_on_editor(),
            "fit" => self.editor.request_zoom_fit(),
            "select" => self.editor.tool = CanvasTool::Select,
            "wire" => self.editor.tool = CanvasTool::Wire,
            "pan" => self.editor.tool = CanvasTool::Pan,
            "grid" => self.editor.show_grid = !self.editor.show_grid,
            "snap" => self.editor.snap_enabled = !self.editor.snap_enabled,
            "undo" => self.undo_canvas(),
            "redo" => self.redo_canvas(),
            "focus" => self.canvas_focus_mode = !self.canvas_focus_mode,
            "export_svg" => self.export_schematic_file("svg"),
            "export_netlist" => self.export_schematic_file("netlist"),
            "export_sexp" => self.export_schematic_file("sexp_netlist"),
            "export_pdf" => self.export_schematic_file("pdf"),
            "export_mcad" => self.export_schematic_file("mcad"),
            "duplicate" => self.duplicate_selection(),
            "delete" => self.delete_selected(),
            "label" => self.editor.tool = CanvasTool::NetLabel,
            "power" => self.editor.tool = CanvasTool::Power,
            "place" => self.place_generic_symbol("U"),
            _ => {}
        }
    }
}

const COMMAND_LIST: &[(&str, &str)] = &[
    ("save", "Save schematic"),
    ("erc", "Run ERC"),
    ("export_svg", "Export SVG"),
    ("export_netlist", "Export connectivity netlist"),
    ("export_sexp", "Export S-expression netlist"),
    ("export_pdf", "Export PDF plot"),
    ("export_mcad", "Export MCAD handoff JSON"),
    ("duplicate", "Duplicate selection"),
    ("delete", "Delete selection"),
    ("fit", "Zoom to fit"),
    ("select", "Tool: Select"),
    ("wire", "Tool: Wire"),
    ("label", "Tool: Net label"),
    ("power", "Tool: Power"),
    ("place", "Tool: Place symbol"),
    ("pan", "Tool: Pan"),
    ("grid", "Toggle grid"),
    ("snap", "Toggle snap"),
    ("undo", "Undo"),
    ("redo", "Redo"),
    ("focus", "Focus canvas"),
];
