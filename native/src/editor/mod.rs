//! Schematic editor: viewport, rendering, hit-testing, and interaction.

mod annot_graphics;
mod commands;
pub mod connectivity;
pub mod document;
mod geometry;
mod hit_test;
mod interaction;
mod label_placement;
mod junctions;
pub mod live_erc;
mod net_sync;
#[cfg(test)]
mod netlist_golden;
mod render;
pub mod sheets;
mod state;
mod tools;
mod viewport;
mod wire_push;
mod wire_reroute;
mod wire_snap;

pub use connectivity::wire_indices_for_net;

pub use state::{ErcMarkerOnCanvas, PlaceSymbolRequest, SchematicEditor, SheetInfo};
pub use tools::CanvasTool;

use egui::{Stroke, Ui, Vec2};
use std::collections::HashMap;
use uuid::Uuid;

use crate::base_symbols::BaseSymbolLibrary;
use crate::ui::tokens::UiTokens;

use interaction::handle as handle_interaction;
use render::{
    paint_annotations, paint_box_select, paint_erc_markers, paint_grid, paint_symbols,
    paint_wire_rubber_band, RenderCtx,
};

/// Draw and handle input for the schematic canvas inside `ui`.
pub fn show(
    ui: &mut Ui,
    editor: &mut SchematicEditor,
    part_cache: &HashMap<Uuid, String>,
    symbol_lib: Option<&BaseSymbolLibrary>,
    tokens: &UiTokens,
) -> Option<String> {

    let (rect, resp) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
    if resp.clicked() || resp.drag_started() {
        resp.request_focus();
    }
    let focused = resp.has_focus();
    editor.canvas_has_focus = focused;

    editor.screen_rect = Some(rect);
    let origin = rect.min;

    editor.apply_zoom_fit_if_pending(rect, origin);

    // Zoom only while the pointer is over the schematic — not when scrolling side panels
    // (canvas may still have keyboard focus after a click).
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll.abs() > 0.0 && resp.hovered() {
        let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(rect.center());
        editor.viewport.zoom_at_pointer(origin, mouse, scroll);
    }

    let space_pan = ui.input(|i| i.key_down(egui::Key::Space));
    let pan_primary = matches!(editor.tool, CanvasTool::Pan)
        || (matches!(editor.tool, CanvasTool::Select) && space_pan);

    if resp.dragged_by(egui::PointerButton::Middle)
        || (resp.dragged_by(egui::PointerButton::Primary) && pan_primary)
    {
        editor.viewport.pan_by(resp.drag_delta());
    }

    let pointer = ui.input(|i| i.pointer.interact_pos());
    if let Some(mp) = pointer.filter(|p| rect.contains(*p)) {
        editor.cursor_world = Some(editor.snap_world(editor.viewport.screen_to_world(origin, mp)));
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, tokens.bg_canvas);
    let border_color = if focused {
        tokens.stroke_focus.color
    } else {
        tokens.stroke_subtle.color
    };
    painter.rect_stroke(
        rect.shrink(0.5),
        0.0,
        Stroke::new(if focused { 1.5 } else { 1.0 }, border_color),
    );

    paint_grid(&painter, rect, editor, &tokens);

    let rctx = RenderCtx {
        editor,
        origin,
        view_rect: rect,
        part_cache,
        symbol_lib,
        tokens: &tokens,
    };

    paint_annotations(&painter, &rctx);
    paint_erc_markers(&painter, &rctx);
    paint_box_select(&painter, &rctx);

    if let Some(from) = &editor.wire_drag_from.clone() {
        if let Some(mp) = pointer.filter(|p| rect.contains(*p)) {
            paint_wire_rubber_band(&painter, &rctx, from, mp);
        }
        painter.text(
            rect.min + Vec2::new(12.0, 8.0),
            egui::Align2::LEFT_TOP,
            format!(
                "Wiring {}.{} — click target pin (Esc cancel)",
                from.ref_des, from.pin_name
            ),
            egui::FontId::proportional(12.0),
            tokens.text_secondary,
        );
    }

    paint_symbols(ui, &painter, &rctx, pointer);

    let interaction = handle_interaction(ui, editor, &resp, rect, origin);
    interaction.log
}
