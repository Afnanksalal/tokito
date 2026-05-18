//! Type ramp with consistent pixel sizes for the dense CAD UI.

use egui::{FontId, RichText};

#[derive(Clone, Copy)]
#[allow(dead_code)] // ramp scales exposed for future widgets / monospace tuning
pub struct TypeRamp {
    pub title: f32,
    pub section: f32,
    pub body: f32,
    pub small: f32,
    pub mono: f32,
}

impl Default for TypeRamp {
    fn default() -> Self {
        Self {
            title: 18.0,
            section: 13.0,
            body: 12.0,
            small: 11.0,
            mono: 12.0,
        }
    }
}

#[allow(dead_code)]
impl TypeRamp {
    pub fn title(&self, s: impl Into<String>) -> RichText {
        RichText::new(s.into()).size(self.title).strong()
    }

    pub fn section(&self, s: impl Into<String>) -> RichText {
        RichText::new(s.into()).size(self.section).strong()
    }

    pub fn body(&self, s: impl Into<String>) -> RichText {
        RichText::new(s.into()).size(self.body)
    }

    pub fn small_weak(&self, s: impl Into<String>) -> RichText {
        RichText::new(s.into()).size(self.small).weak()
    }

    pub fn mono(&self, _s: impl Into<String>) -> FontId {
        FontId::monospace(self.mono)
    }

    pub fn proportional(&self, px: f32) -> FontId {
        FontId::proportional(px)
    }
}
