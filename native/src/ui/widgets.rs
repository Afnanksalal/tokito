//! Reusable widgets — primary CTA, toolbar icon button, pill.

use crate::ui::tokens::UiTokens;
use egui::{RichText, Ui, Widget};

pub fn primary_button(ui: &mut Ui, tokens: &UiTokens, label: impl Into<String>) -> egui::Response {
    let label = label.into();
    ui.add_sized(
        [ui.available_width(), 36.0],
        egui::Button::new(RichText::new(label).strong().color(egui::Color32::WHITE))
            .fill(tokens.accent),
    )
}

pub fn secondary_button(
    ui: &mut Ui,
    tokens: &UiTokens,
    label: impl Into<String>,
) -> egui::Response {
    egui::Button::new(RichText::new(label.into()).color(tokens.text_primary))
        .fill(tokens.bg_elevated)
        .stroke(tokens.stroke_subtle)
        .ui(ui)
}

pub fn toolbar_icon_btn(ui: &mut Ui, tokens: &UiTokens, tooltip: &str, text_icon: &str) -> bool {
    let r = ui
        .add_sized(
            [30.0, 26.0],
            egui::Button::new(RichText::new(text_icon).size(14.0))
                .fill(tokens.bg_elevated)
                .stroke(tokens.stroke_subtle),
        )
        .on_hover_text(tooltip);
    r.clicked()
}
