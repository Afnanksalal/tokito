use crate::app::studio::chrome::TabChrome;
use crate::app::App;

impl App {
    pub(crate) fn render_studio_console_tab(&mut self, ui: &mut egui::Ui) {
        let tokens = self.ui_tokens;
        let chrome = TabChrome::begin(ui, &tokens);
        chrome.header(
            ui,
            "Console",
            Some("Status, placement, and pipeline messages"),
        );

        ui.horizontal(|ui| {
            if crate::ui::widgets::secondary_button(ui, chrome.tokens, "Clear").clicked() {
                self.console_lines.clear();
            }
            ui.label(
                egui::RichText::new(format!("{} lines", self.console_lines.len()))
                    .small()
                    .weak()
                    .color(chrome.tokens.text_muted),
            );
        });
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .id_salt("studio_console_scroll")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if self.console_lines.is_empty() {
                    chrome.empty(ui, "Actions and errors are logged here.");
                    return;
                }
                for line in &self.console_lines {
                    crate::ui::layout::content_card(ui, chrome.tokens, |ui| {
                        ui.monospace(
                            egui::RichText::new(line)
                                .small()
                                .color(chrome.tokens.text_secondary),
                        );
                    });
                    ui.add_space(2.0);
                }
            });
    }
}
