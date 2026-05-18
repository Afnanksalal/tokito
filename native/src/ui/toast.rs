//! Transient toast notifications.

use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Toast {
    pub message: String,
    pub until: Instant,
    pub kind: ToastKind,
}

#[derive(Clone, Copy)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Default)]
pub struct ToastStack {
    items: Vec<Toast>,
}

impl ToastStack {
    pub fn push(&mut self, message: impl Into<String>, kind: ToastKind) {
        self.items.push(Toast {
            message: message.into(),
            until: Instant::now() + Duration::from_secs(4),
            kind,
        });
    }

    pub fn prune(&mut self) {
        let now = Instant::now();
        self.items.retain(|t| t.until > now);
    }

    pub fn show(&mut self, ctx: &egui::Context, tokens: &crate::ui::UiTokens) {
        self.prune();
        if self.items.is_empty() {
            return;
        }
        egui::Area::new(egui::Id::new("toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    for t in self.items.iter().rev() {
                        let color = match t.kind {
                            ToastKind::Success => tokens.accent,
                            ToastKind::Warning => tokens.warning,
                            ToastKind::Error => tokens.danger,
                            ToastKind::Info => tokens.text_primary,
                        };
                        egui::Frame::popup(ui.style())
                            .fill(tokens.bg_elevated)
                            .stroke(egui::Stroke::new(1.0, color))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(&t.message).small().color(color));
                            });
                    }
                });
            });
    }
}
