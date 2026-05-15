//! Pan/zoom viewport and fit-to-content.

use egui::{Pos2, Rect, Vec2};

use super::geometry::{content_bounds, empty_sheet_bounds, FIT_PADDING};
use crate::canvas::{CanvasSnapshot, Viewport};

impl Viewport {
    /// Fit all schematic content into `view_rect` (screen space).
    pub fn fit_content(&mut self, view_rect: Rect, origin: Pos2, snap: &CanvasSnapshot) {
        let bounds = content_bounds(snap).unwrap_or_else(empty_sheet_bounds);
        let w = bounds.width().max(80.0);
        let h = bounds.height().max(80.0);
        let margin = FIT_PADDING * 0.5;
        let zw = (view_rect.width() - margin * 2.0) / w;
        let zh = (view_rect.height() - margin * 2.0) / h;
        self.zoom = zw.min(zh).clamp(0.2, 4.0);

        let center_world = bounds.center();
        let target_screen = view_rect.center();
        self.pan = target_screen - origin - center_world.to_vec2() * self.zoom;
    }

    pub fn zoom_at_pointer(&mut self, origin: Pos2, pointer: Pos2, scroll_delta_y: f32) {
        if scroll_delta_y.abs() <= f32::EPSILON {
            return;
        }
        let before = self.screen_to_world(origin, pointer);
        self.zoom = (self.zoom * (1.0 + scroll_delta_y * 0.0015)).clamp(0.2, 4.0);
        let after = self.world_to_screen(origin, before);
        self.pan += pointer - after;
    }

    pub fn pan_by(&mut self, delta: Vec2) {
        self.pan += delta;
    }
}
