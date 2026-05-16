//! Studio keyboard shortcuts for the schematic canvas.

use egui::{Context, Key};

use crate::app::App;
use crate::editor::CanvasTool;

impl App {
    pub(crate) fn handle_studio_shortcuts(&mut self, ctx: &Context) {
        let mods = ctx.input(|i| i.modifiers);
        let cmd = mods.ctrl || mods.command;

        let canvas_focus = self.editor.canvas_has_focus;
        let text_field = ctx.wants_keyboard_input();
        let allow_canvas = canvas_focus || !text_field;

        let prompt_focused = ctx.memory(|m| {
            m.focused()
                .map(|id| format!("{:?}", id).contains("prompt"))
                .unwrap_or(false)
        });

        if !prompt_focused {
            if cmd && ctx.input(|i| i.key_pressed(Key::Z)) && !mods.shift {
                self.undo_canvas();
                return;
            }
            if cmd
                && (ctx.input(|i| i.key_pressed(Key::Y))
                    || (mods.shift && ctx.input(|i| i.key_pressed(Key::Z))))
            {
                self.redo_canvas();
                return;
            }
        }

        if allow_canvas
            && (ctx.input(|i| i.key_pressed(Key::Delete))
                || ctx.input(|i| i.key_pressed(Key::Backspace)))
        {
            self.delete_selected();
            return;
        }

        if allow_canvas && cmd {
            if ctx.input(|i| i.key_pressed(Key::A)) {
                self.editor.select_all();
                return;
            }
            if ctx.input(|i| i.key_pressed(Key::C)) {
                self.editor.copy_selection();
                return;
            }
            if ctx.input(|i| i.key_pressed(Key::X)) {
                self.cut_selection();
                return;
            }
            if ctx.input(|i| i.key_pressed(Key::V)) {
                self.paste_selection();
                return;
            }
            if ctx.input(|i| i.key_pressed(Key::D)) {
                self.duplicate_selection();
                return;
            }
        }

        if prompt_focused {
            return;
        }

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.editor.cancel_wire_tool();
            self.editor.clear_selection();
            return;
        }

        if !allow_canvas {
            return;
        }

        if ctx.input(|i| i.key_pressed(Key::Q)) {
            self.editor.tool = CanvasTool::Select;
        }
        if ctx.input(|i| i.key_pressed(Key::W)) {
            self.editor.tool = CanvasTool::Wire;
        }
        if ctx.input(|i| i.key_pressed(Key::A)) && !cmd {
            self.place_generic_symbol("U");
        }
        if ctx.input(|i| i.key_pressed(Key::N)) {
            self.editor.tool = CanvasTool::NetLabel;
        }
        if ctx.input(|i| i.key_pressed(Key::K)) {
            self.editor.tool = CanvasTool::SheetPort;
        }
        if ctx.input(|i| i.key_pressed(Key::P)) && !cmd {
            self.editor.tool = CanvasTool::Power;
        }
        if ctx.input(|i| i.key_pressed(Key::J)) {
            self.editor.tool = CanvasTool::Junction;
        }
        if ctx.input(|i| i.key_pressed(Key::X)) && !cmd {
            self.editor.tool = CanvasTool::NoConnect;
        }
        if ctx.input(|i| i.key_pressed(Key::B)) {
            self.editor.tool = CanvasTool::Bus;
        }
        if ctx.input(|i| i.key_pressed(Key::T)) {
            self.editor.tool = CanvasTool::Text;
        }
        if ctx.input(|i| i.key_pressed(Key::H)) {
            self.editor.tool = CanvasTool::Pan;
        }
        if ctx.input(|i| i.key_pressed(Key::G)) {
            self.editor.show_grid = !self.editor.show_grid;
        }
        if ctx.input(|i| i.key_pressed(Key::S)) && !cmd {
            self.editor.snap_enabled = !self.editor.snap_enabled;
        }
        if ctx.input(|i| i.key_pressed(Key::Home)) {
            self.editor.request_zoom_fit();
        }
        if ctx.input(|i| i.key_pressed(Key::R)) && !cmd {
            self.editor.rotate_selected_symbols(90.0);
        }
    }
}
