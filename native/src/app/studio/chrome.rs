//! Shared chrome for dock tabs (title, spacing, actions).

use egui::Ui;

use crate::ui::tokens::UiTokens;

pub struct TabChrome<'a> {
    pub tokens: &'a UiTokens,
}

impl<'a> TabChrome<'a> {
    pub fn begin(ui: &mut Ui, tokens: &'a UiTokens) -> Self {
        crate::ui::layout::dock_tab_shell(ui, tokens);
        Self { tokens }
    }

    pub fn header(&self, ui: &mut Ui, title: &str, subtitle: Option<&str>) {
        crate::ui::layout::section_header(ui, self.tokens, title, subtitle);
    }

    pub fn subsection(&self, ui: &mut Ui, title: &str) {
        crate::ui::layout::subsection(ui, self.tokens, title);
    }

    pub fn empty(&self, ui: &mut Ui, message: &str) {
        crate::ui::layout::empty_state(ui, self.tokens, message);
    }

    pub fn actions(&self, ui: &mut Ui, labels: &[(&str, bool)]) -> Option<usize> {
        crate::ui::layout::toolbar_actions(ui, self.tokens, labels)
    }
}
