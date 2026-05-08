//! Layout helpers — section chrome, inspector rows, dense stacks.

use crate::ui::tokens::UiTokens;
use egui::{Align, Layout, RichText, Ui};

pub fn section_header(ui: &mut Ui, tokens: &UiTokens, title: &str, subtitle: Option<&str>) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(title)
                .strong()
                .size(13.0)
                .color(tokens.text_primary),
        );
        if let Some(s) = subtitle {
            ui.label(RichText::new(s).small().weak().color(tokens.text_muted));
        }
    });
    ui.add_space(tokens.spacing_sm);
}

pub fn inspector_row(ui: &mut Ui, tokens: &UiTokens, label: &str, value: impl Into<String>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).small().color(tokens.text_muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(value.into())
                    .small()
                    .color(tokens.text_primary),
            );
        });
    });
}
