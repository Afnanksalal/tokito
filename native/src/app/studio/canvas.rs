use crate::app::App;
use uuid::Uuid;
impl App {
    pub(crate) fn render_studio_canvas_tab(&mut self, ui: &mut egui::Ui, _design_id: Uuid) {
        if let Some(msg) = crate::editor::show(
            ui,
            &mut self.editor,
            &self.part_cache,
            self.base_symbols.as_ref(),
            &self.ui_tokens,
        ) {
            self.log_console(msg);
        }
    }
}
