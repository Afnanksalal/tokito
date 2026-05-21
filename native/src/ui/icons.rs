//! Phosphor icon helpers.
//!
//! Icons render via a dedicated `phosphor` font family (registered in
//! `theme.rs`) so their Private-Use-Area codepoints never collide with Inter
//! Var's glyph coverage. Rendering an icon through the normal proportional
//! family would let Inter intercept the codepoint and paint a stray glyph.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId, RichText};

/// Re-exported Phosphor Regular glyph constants (`icons::ph::FOLDER`, etc.).
pub use egui_phosphor::regular as ph;

fn phosphor_family() -> FontFamily {
    FontFamily::Name("phosphor".into())
}

/// A `FontId` in the phosphor family — for painting icons via `Painter::text`.
pub fn icon_font(size: f32) -> FontId {
    FontId::new(size, phosphor_family())
}

/// A standalone icon as `RichText` — for icon-only buttons and labels.
pub fn icon(glyph: &str, size: f32, color: Color32) -> RichText {
    RichText::new(glyph)
        .font(FontId::new(size, phosphor_family()))
        .color(color)
}

/// An icon followed by a text label, as a `LayoutJob`. The icon section uses
/// the phosphor family; the text section uses the default proportional family.
/// Pass the result straight to `Button::new(...)`, `ui.button(...)`,
/// `ui.label(...)`, `TextEdit::hint_text(...)`, etc.
pub fn icon_label(
    glyph: &str,
    icon_size: f32,
    text: &str,
    text_size: f32,
    color: Color32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        glyph,
        0.0,
        TextFormat {
            font_id: FontId::new(icon_size, phosphor_family()),
            color,
            ..Default::default()
        },
    );
    job.append(
        text,
        8.0,
        TextFormat {
            font_id: FontId::proportional(text_size),
            color,
            ..Default::default()
        },
    );
    job
}
