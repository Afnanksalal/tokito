//! Heuristic “pile of footprints” preview for schematic-driven MCAD export (not PCB layout).

mod raster;
pub mod scene;

use egui::{ColorImage, TextureHandle, TextureOptions};

use scene::Placement3d;

pub struct McadViewer {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub pan: glam::Vec2,
    texture: Option<TextureHandle>,
    last_size: (u32, u32),
    last_scene_hash: u64,
    dirty: bool,
}

impl Default for McadViewer {
    fn default() -> Self {
        Self {
            yaw: 0.7,
            pitch: 0.45,
            distance: 120.0,
            pan: glam::Vec2::ZERO,
            texture: None,
            last_size: (0, 0),
            last_scene_hash: 0,
            dirty: true,
        }
    }
}

impl McadViewer {
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, placements: &[Placement3d]) {
        let available = ui.available_size();
        let size = egui::vec2(available.x.max(200.0), available.y.max(160.0));
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

        if response.dragged_by(egui::PointerButton::Primary) {
            let d = response.drag_delta();
            self.yaw += d.x * 0.01;
            self.pitch = (self.pitch + d.y * 0.01).clamp(0.05, 1.5);
            self.dirty = true;
        }
        if response.dragged_by(egui::PointerButton::Secondary) {
            let d = response.drag_delta();
            self.pan.x += d.x * 0.05;
            self.pan.y -= d.y * 0.05;
            self.dirty = true;
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > 0.0 {
                self.distance = (self.distance - scroll * 0.3).clamp(20.0, 800.0);
                self.dirty = true;
            }
        }

        let w = rect.width().max(1.0) as u32;
        let h = rect.height().max(1.0) as u32;
        if (w, h) != self.last_size {
            self.last_size = (w, h);
            self.dirty = true;
        }

        if placements.is_empty() {
            ui.painter()
                .rect_filled(rect, 4.0, crate::ui::tokens::UiTokens::default().bg_canvas);
            ui.put(
                rect,
                egui::Label::new(
                    egui::RichText::new(
                        "Assign footprints on symbols to see a rough preview.\n\
                     (Not PCB layout — schematic footprint hints only.)",
                    )
                    .small()
                    .weak(),
                ),
            );
            return;
        }

        let hash = scene_hash(placements);
        if self.dirty || hash != self.last_scene_hash {
            let rgba = raster::render_board(
                placements,
                w,
                h,
                self.yaw,
                self.pitch,
                self.distance,
                self.pan,
            );
            let image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            self.texture = Some(ctx.load_texture("mcad_viewport", image, TextureOptions::LINEAR));
            self.last_scene_hash = hash;
            self.dirty = false;
        }

        if let Some(tex) = &self.texture {
            ui.put(
                rect,
                egui::Image::new(tex)
                    .fit_to_exact_size(rect.size())
                    .sense(egui::Sense::hover()),
            );
        }
    }
}

fn scene_hash(placements: &[Placement3d]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    placements.len().hash(&mut h);
    for p in placements {
        p.ref_des.hash(&mut h);
        p.footprint.hash(&mut h);
        p.center_mm.x.to_bits().hash(&mut h);
    }
    h.finish()
}
