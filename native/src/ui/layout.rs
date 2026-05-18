//! Layout helpers: panels, search, filters, lists, inspector rows.

use crate::ui::tokens::UiTokens;
use egui::{Align, Frame, Layout, Margin, RichText, Ui};

pub fn panel_frame(tokens: &UiTokens) -> Frame {
    Frame::none()
        .fill(tokens.bg_panel)
        .inner_margin(tokens.margin_panel)
        .stroke(tokens.stroke_subtle)
}

/// Standard padding for dock tab interiors.
pub fn dock_tab_shell(ui: &mut Ui, tokens: &UiTokens) {
    ui.set_min_width(240.0);
    ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
    let _ = tokens;
}

pub fn section_header(ui: &mut Ui, tokens: &UiTokens, title: &str, subtitle: Option<&str>) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(title)
                .strong()
                .size(14.0)
                .color(tokens.text_primary),
        );
        if let Some(s) = subtitle {
            ui.add_space(2.0);
            ui.label(RichText::new(s).small().weak().color(tokens.text_muted));
        }
    });
    ui.add_space(tokens.spacing_sm);
}

pub fn subsection(ui: &mut Ui, tokens: &UiTokens, title: &str) {
    ui.add_space(4.0);
    ui.label(
        RichText::new(title)
            .small()
            .strong()
            .color(tokens.text_secondary),
    );
    ui.add_space(6.0);
}

pub fn empty_state(ui: &mut Ui, tokens: &UiTokens, message: &str) {
    egui::Frame::none()
        .fill(tokens.bg_elevated)
        .rounding(tokens.radius_sm)
        .inner_margin(Margin::same(14.0))
        .show(ui, |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(RichText::new(message).size(12.0).color(tokens.text_muted));
            });
        });
}

pub fn content_card(ui: &mut Ui, tokens: &UiTokens, add_contents: impl FnOnce(&mut Ui)) {
    let outer_width = ui.available_width();
    egui::Frame::none()
        .fill(tokens.bg_elevated)
        .rounding(tokens.radius_md)
        .inner_margin(Margin::same(14.0))
        .stroke(tokens.stroke_subtle)
        .show(ui, |ui| {
            if outer_width.is_finite() && outer_width > 28.0 {
                let inner_width = (outer_width - 28.0).max(0.0);
                ui.set_width(inner_width);
                ui.set_max_width(inner_width);
            }
            add_contents(ui);
        });
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
    ui.add_space(2.0);
}

pub fn toolbar_actions(ui: &mut Ui, tokens: &UiTokens, actions: &[(&str, bool)]) -> Option<usize> {
    let mut clicked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (i, (label, enabled)) in actions.iter().enumerate() {
            let r = crate::ui::widgets::secondary_button(ui, tokens, *label);
            if r.clicked() && *enabled {
                clicked = Some(i);
            }
        }
    });
    clicked
}

pub fn filter_chip(ui: &mut Ui, tokens: &UiTokens, label: &str, selected: bool) -> bool {
    let fill = if selected {
        tokens.bg_chip_selected
    } else {
        tokens.bg_elevated
    };
    let stroke = if selected {
        tokens.stroke_focus
    } else {
        tokens.stroke_subtle
    };
    ui.add(
        egui::Button::new(RichText::new(label).size(11.0).color(if selected {
            tokens.text_primary
        } else {
            tokens.text_secondary
        }))
        .fill(fill)
        .stroke(stroke)
        .rounding(tokens.radius_sm)
        .min_size(egui::vec2(0.0, 28.0)),
    )
    .clicked()
}

pub fn search_field(ui: &mut Ui, query: &mut String, hint: &str) -> bool {
    let mut submit = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Search")
                .size(11.0)
                .color(ui.visuals().weak_text_color()),
        );
        let r = ui.add(
            egui::TextEdit::singleline(query)
                .hint_text(hint)
                .desired_width(ui.available_width().max(0.0))
                .margin(egui::Margin::symmetric(8.0, 6.0)),
        );
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            submit = true;
        }
    });
    submit
}

pub fn list_section_label(ui: &mut Ui, tokens: &UiTokens, label: &str, count: usize) {
    ui.add_space(4.0);
    ui.label(
        RichText::new(format!("{label} ({count})"))
            .small()
            .strong()
            .color(tokens.text_secondary),
    );
    ui.add_space(4.0);
}
