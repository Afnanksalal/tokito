//! egui painting for the schematic canvas.

use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

use super::hit_test::PIN_HIT_RADIUS;
use super::state::{PartCache, SchematicEditor};
use super::tools::CanvasTool;
use crate::base_symbols::BaseSymbolLibrary;
use crate::canvas::{display_pins_for_symbol, symbol_hit_half_extents, symbol_pin_world};
use crate::symbols_draw;
use crate::ui::tokens::UiTokens;

pub struct RenderCtx<'a> {
    pub editor: &'a SchematicEditor,
    pub origin: Pos2,
    pub view_rect: Rect,
    pub part_cache: PartCache<'a>,
    pub symbol_lib: Option<&'a BaseSymbolLibrary>,
    pub tokens: &'a UiTokens,
}

pub fn paint_grid(painter: &Painter, rect: Rect, editor: &SchematicEditor, tokens: &UiTokens) {
    if !editor.show_grid {
        return;
    }
    let grid = crate::canvas::GRID_PX * editor.viewport.zoom;
    let grid_color = tokens.canvas_grid_minor;
    let major_grid_color = tokens.canvas_grid_major;
    if grid < 8.0 {
        return;
    }
    let start_x = (rect.min.x + (editor.viewport.pan.x % grid)).floor();
    let start_y = (rect.min.y + (editor.viewport.pan.y % grid)).floor();
    let mut x = start_x;
    let mut ix = 0usize;
    while x < rect.max.x {
        let color = if ix % 5 == 0 {
            major_grid_color
        } else {
            grid_color
        };
        painter.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            Stroke::new(1.0, color),
        );
        x += grid;
        ix += 1;
    }
    let mut y = start_y;
    let mut iy = 0usize;
    while y < rect.max.y {
        let color = if iy % 5 == 0 {
            major_grid_color
        } else {
            grid_color
        };
        painter.line_segment(
            [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
            Stroke::new(1.0, color),
        );
        y += grid;
        iy += 1;
    }
}

pub fn paint_sheet_frame(painter: &Painter, ctx: &RenderCtx<'_>) {
    let vp = &ctx.editor.viewport;
    let origin = ctx.origin;
    let frame = crate::canvas::sheet_bounds_world();
    let stroke = Stroke::new(1.0, ctx.tokens.canvas_frame);
    let tl = vp.world_to_screen(origin, frame.min);
    let br = vp.world_to_screen(origin, frame.max);
    let sr = egui::Rect::from_min_max(tl, br);
    painter.rect_stroke(sr, 0.0, stroke);
    painter.text(
        sr.left_top() + Vec2::new(8.0, 6.0),
        egui::Align2::LEFT_TOP,
        ctx.editor
            .sheets
            .iter()
            .find(|s| s.id == ctx.editor.active_sheet_id)
            .map(|s| s.name.as_str())
            .unwrap_or("Sheet"),
        egui::FontId::proportional(12.0),
        ctx.tokens.text_muted,
    );
}

pub fn paint_box_select(painter: &Painter, ctx: &RenderCtx<'_>) {
    let Some(a) = ctx.editor.box_select_start else {
        return;
    };
    let b = ctx.editor.box_select_current.unwrap_or(a);
    let vp = &ctx.editor.viewport;
    let origin = ctx.origin;
    let sa = vp.world_to_screen(origin, a);
    let sb = vp.world_to_screen(origin, b);
    let r = egui::Rect::from_two_pos(sa, sb);
    let fill = ctx.tokens.accent_dim;
    painter.rect_filled(r, 0.0, fill);
    painter.rect_stroke(r, 0.0, Stroke::new(1.0, ctx.tokens.accent));
}

pub fn paint_erc_markers(painter: &Painter, ctx: &RenderCtx<'_>) {
    let vp = &ctx.editor.viewport;
    let origin = ctx.origin;
    for (i, m) in ctx.editor.erc_markers.iter().enumerate() {
        let p = vp.world_to_screen(origin, m.position);
        let selected = ctx.editor.selected_erc_marker == Some(i);
        let color = match m.severity.as_str() {
            "error" => egui::Color32::from_rgb(235, 90, 90),
            "warning" => egui::Color32::from_rgb(235, 165, 72),
            _ => egui::Color32::from_rgb(140, 180, 220),
        };
        let r = if selected { 10.0 } else { 8.0 };
        painter.circle_filled(p, r, color);
        painter.circle_stroke(p, r + 1.5, Stroke::new(1.0, egui::Color32::BLACK));
        if selected {
            painter.text(
                p + Vec2::new(12.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("{}: {}", m.code, util_truncate(&m.message, 48)),
                egui::FontId::proportional(11.0),
                color,
            );
        }
    }
}

fn util_truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

pub fn paint_annotations(painter: &Painter, ctx: &RenderCtx<'_>) {
    paint_sheet_frame(painter, ctx);

    let origin = ctx.origin;
    let vp = &ctx.editor.viewport;
    let highlighted_net = ctx.editor.highlighted_net();
    let highlighted_indices = highlighted_net.as_ref().map(|net| {
        crate::editor::wire_indices_for_net(net, &ctx.editor.wire_segments, &ctx.editor.net_labels)
    });

    for (i, seg) in ctx.editor.wire_segments.iter().enumerate() {
        let net_highlighted = highlighted_indices
            .as_ref()
            .map(|ix| ix.contains(&i))
            .unwrap_or(false)
            || highlighted_net.as_deref() == Some(seg.net.as_str());
        let t = ctx.tokens;
        let stroke = if ctx.editor.selected_segments.contains(&i) {
            Stroke::new(2.8, t.wire_selected)
        } else if net_highlighted {
            Stroke::new(2.2, t.wire_highlight)
        } else {
            Stroke::new(1.6, t.wire)
        };
        painter.line_segment(
            [
                vp.world_to_screen(origin, seg.start),
                vp.world_to_screen(origin, seg.end),
            ],
            stroke,
        );
        if ctx.editor.selected_segment == Some(i) {
            for pt in [seg.start, seg.end] {
                let p = vp.world_to_screen(origin, pt);
                painter.circle_filled(p, 4.5, ctx.tokens.selection);
            }
        }
    }

    for (i, label) in ctx.editor.net_labels.iter().enumerate() {
        let p = vp.world_to_screen(origin, label.pos);
        let t = ctx.tokens;
        let color = if ctx.editor.selected_net_label == Some(i) {
            t.selection
        } else if highlighted_net.as_deref() == Some(label.name.as_str()) {
            t.wire_highlight
        } else {
            t.label_ink
        };
        if label.kind == tokito::models::NetLabelKind::Hierarchical {
            let sz = Vec2::new(28.0, 18.0);
            let r = egui::Rect::from_min_size(p, sz);
            painter.rect_stroke(r, 2.0, Stroke::new(1.5, color));
            painter.line_segment(
                [r.center() - Vec2::new(6.0, 0.0), r.right_center()],
                Stroke::new(1.5, color),
            );
            painter.text(
                r.min + Vec2::new(4.0, 2.0),
                egui::Align2::LEFT_TOP,
                &label.name,
                egui::FontId::monospace(10.0),
                color,
            );
        } else {
            painter.text(
                p + Vec2::new(8.0, -8.0),
                egui::Align2::LEFT_CENTER,
                &label.name,
                egui::FontId::monospace(12.0),
                color,
            );
            painter.line_segment([p, p + Vec2::new(22.0, 0.0)], Stroke::new(1.4, color));
        }
    }

    for (i, junction) in ctx.editor.junctions.iter().enumerate() {
        let p = vp.world_to_screen(origin, junction.pos);
        let t = ctx.tokens;
        let color = if ctx.editor.selected_junction == Some(i) {
            t.selection
        } else {
            t.wire_highlight
        };
        painter.circle_filled(p, 4.0, color);
    }

    for (i, nc) in ctx.editor.no_connects.iter().enumerate() {
        let p = vp.world_to_screen(origin, nc.pos);
        let t = ctx.tokens;
        let color = if ctx.editor.selected_no_connect == Some(i) {
            t.selection
        } else {
            t.danger
        };
        painter.line_segment(
            [p + Vec2::new(-5.0, -5.0), p + Vec2::new(5.0, 5.0)],
            Stroke::new(1.5, color),
        );
        painter.line_segment(
            [p + Vec2::new(-5.0, 5.0), p + Vec2::new(5.0, -5.0)],
            Stroke::new(1.5, color),
        );
    }

    for (i, pwr) in ctx.editor.power_symbols.iter().enumerate() {
        let p = vp.world_to_screen(origin, pwr.pos);
        let t = ctx.tokens;
        let color = if ctx.editor.selected_power_symbol == Some(i) {
            t.selection
        } else {
            Color32::from_rgb(32, 140, 88)
        };
        painter.line_segment([p, p + Vec2::new(0.0, -18.0)], Stroke::new(1.5, color));
        painter.line_segment(
            [p + Vec2::new(-10.0, -18.0), p + Vec2::new(10.0, -18.0)],
            Stroke::new(1.5, color),
        );
        painter.line_segment(
            [p + Vec2::new(-6.0, -23.0), p + Vec2::new(6.0, -23.0)],
            Stroke::new(1.5, color),
        );
        painter.text(
            p + Vec2::new(0.0, -33.0),
            egui::Align2::CENTER_BOTTOM,
            &pwr.name,
            egui::FontId::monospace(11.5),
            color,
        );
    }

    for (i, bus) in ctx.editor.buses.iter().enumerate() {
        let a = vp.world_to_screen(origin, bus.start);
        let b = vp.world_to_screen(origin, bus.end);
        let t = ctx.tokens;
        let color = if ctx.editor.selected_bus == Some(i) {
            t.selection
        } else {
            Color32::from_rgb(88, 72, 148)
        };
        painter.line_segment([a, b], Stroke::new(4.0, color));
        if let Some(name) = &bus.name {
            painter.text(
                a.lerp(b, 0.5) + Vec2::new(0.0, -10.0),
                egui::Align2::CENTER_BOTTOM,
                name,
                egui::FontId::monospace(11.0),
                color,
            );
        }
    }

    for (i, text) in ctx.editor.text_items.iter().enumerate() {
        let p = vp.world_to_screen(origin, text.pos);
        let t = ctx.tokens;
        let color = if ctx.editor.selected_text_item == Some(i) {
            t.selection
        } else {
            t.text_primary
        };
        painter.text(
            p,
            egui::Align2::LEFT_TOP,
            &text.text,
            egui::FontId::proportional(13.0),
            color,
        );
    }
}

pub fn paint_symbols(
    ui: &mut egui::Ui,
    painter: &Painter,
    ctx: &RenderCtx<'_>,
    pointer: Option<Pos2>,
) {
    let origin = ctx.origin;
    let vp = &ctx.editor.viewport;
    let z = vp.zoom;

    for i in 0..ctx.editor.symbols.len() {
        let ref_des = ctx.editor.symbols[i].ref_des.clone();
        let sym = &ctx.editor.symbols[i];
        let p = vp.world_to_screen(origin, sym.pos);
        let (hx, hy) = symbol_hit_half_extents(sym);
        let size = Vec2::new(hx * 2.0 * z, hy * 2.0 * z);
        let r = Rect::from_center_size(p, size);

        let selected = ctx.editor.selected_syms.contains(&ref_des)
            || ctx.editor.selected_sym.as_deref() == Some(ref_des.as_str());
        let hovered = ctx.editor.hovered_sym.as_deref() == Some(ref_des.as_str());
        let t = ctx.tokens;
        let ink = if selected {
            t.sym_ink_selected
        } else if hovered {
            t.sym_ink_hover
        } else {
            t.sym_ink
        };

        if selected || hovered {
            let ring = if selected {
                t.sym_sel_ring
            } else {
                t.accent.gamma_multiply(0.55)
            };
            painter.rect_stroke(r.expand(5.0), 0.0, Stroke::new(1.4, ring));
        }

        let kind = symbols_draw::kind_from_refdes(&ref_des);
        let stroke_px = (1.65 * z).clamp(1.25, 3.0);
        let canvas_paint = crate::base_symbols::CanvasSymbolPaint {
            painter,
            viewport: vp,
            origin,
            sym_pos: sym.pos,
            rot_deg: sym.rotation_deg,
            ink,
            outline: t.sym_outline,
            stroke_px,
        };
        if let Some(lib) = ctx.symbol_lib {
            if let Some(id) = sym.symbol_id.as_deref().filter(|s| !s.is_empty()) {
                lib.paint_named_on_canvas(canvas_paint, id, kind, &sym.pin_layout);
            } else {
                crate::base_symbols::paint_fallback_on_canvas(&canvas_paint, kind);
                crate::base_symbols::paint_pin_stubs_from_layout(&canvas_paint, &sym.pin_layout);
            }
        } else {
            crate::base_symbols::paint_fallback_on_canvas(&canvas_paint, kind);
            crate::base_symbols::paint_pin_stubs_from_layout(&canvas_paint, &sym.pin_layout);
        }

        let lib_sym = sym
            .symbol_id
            .as_deref()
            .and_then(|id| ctx.symbol_lib.and_then(|lib| lib.symbol(id)));
        let field_paint = crate::base_symbols::CanvasSymbolPaint {
            painter,
            viewport: vp,
            origin,
            sym_pos: sym.pos,
            rot_deg: sym.rotation_deg,
            ink,
            outline: t.sym_outline,
            stroke_px,
        };
        let bounds_half = symbol_hit_half_extents(sym);
        crate::base_symbols::paint_symbol_fields(
            &field_paint,
            lib_sym,
            &ref_des,
            &sym.value,
            t.refdes_ink,
            bounds_half,
        );
        if let Some(mpn) = sym.part_id.and_then(|pid| ctx.part_cache.get(&pid)) {
            if !mpn.is_empty() && mpn != &sym.value {
                painter.text(
                    r.center_bottom() + Vec2::new(0.0, 6.0 * z),
                    egui::Align2::CENTER_TOP,
                    mpn,
                    egui::FontId::proportional((9.0 * z).clamp(8.0, 14.0)),
                    t.text_muted,
                );
            }
        }

        let pins = display_pins_for_symbol(sym, &ctx.editor.wire_segments);
        for pin_name in pins {
            let pin_world = symbol_pin_world(sym, &pin_name);
            let pin_screen = vp.world_to_screen(origin, pin_world);
            let pin_hover = pointer
                .map(|mp| pin_screen.distance(mp) <= PIN_HIT_RADIUS)
                .unwrap_or(false);
            let wiring_active = ctx.editor.wire_drag_from.is_some();
            let pin_color =
                if pin_hover && (wiring_active || matches!(ctx.editor.tool, CanvasTool::Wire)) {
                    t.pin_hot
                } else if pin_hover {
                    t.wire_highlight
                } else {
                    t.pin_ink
                };
            let pr = if pin_hover {
                crate::canvas::PIN_VIS_RADIUS_HOVER
            } else {
                crate::canvas::PIN_VIS_RADIUS
            };
            painter.circle_filled(pin_screen, pr + 0.5, t.sym_outline);
            painter.circle_filled(pin_screen, pr, pin_color);
            painter.circle_stroke(pin_screen, pr, Stroke::new(0.85, t.sym_ink));
            if z >= 0.85 {
                painter.text(
                    pin_screen + Vec2::new(0.0, -7.0),
                    egui::Align2::CENTER_BOTTOM,
                    &pin_name,
                    egui::FontId::monospace((8.5 * z).clamp(8.0, 12.0)),
                    t.text_muted,
                );
            }
        }

        let _ = ui;
    }
}

pub fn paint_wire_rubber_band(
    painter: &Painter,
    ctx: &RenderCtx<'_>,
    from: &crate::canvas::PinEndpoint,
    pointer: Pos2,
) {
    let Some(start) = ctx.editor.endpoint_world(from) else {
        return;
    };
    if !ctx.view_rect.contains(pointer) {
        return;
    }
    let end = ctx
        .editor
        .snap_world(ctx.editor.viewport.screen_to_world(ctx.origin, pointer));
    for seg in crate::canvas::manhattan_segments(start, end, "") {
        painter.line_segment(
            [
                ctx.editor.viewport.world_to_screen(ctx.origin, seg.start),
                ctx.editor.viewport.world_to_screen(ctx.origin, seg.end),
            ],
            Stroke::new(1.8, ctx.tokens.pin_hot),
        );
    }
    let p0 = ctx.editor.viewport.world_to_screen(ctx.origin, start);
    painter.circle_filled(p0, 5.0, ctx.tokens.sym_outline);
    painter.circle_filled(p0, 4.0, ctx.tokens.pin_hot);
}
