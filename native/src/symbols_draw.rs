//! ECAD-style schematic symbol strokes.

use egui::{Color32, Painter, Pos2, Rect, Stroke};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CompKind {
    Resistor,
    Capacitor,
    Inductor,
    Diode,
    Transistor,
    IC,
    Generic,
}

pub fn kind_from_refdes(ref_des: &str) -> CompKind {
    let c = ref_des
        .trim()
        .chars()
        .next()
        .unwrap_or('U')
        .to_ascii_uppercase();
    match c {
        'R' => CompKind::Resistor,
        'C' => CompKind::Capacitor,
        'L' => CompKind::Inductor,
        'D' | 'Z' => CompKind::Diode,
        'Q' => CompKind::Transistor,
        'U' | 'J' | 'M' | 'I' => CompKind::IC,
        _ => CompKind::Generic,
    }
}

fn xf(pt: Pos2, pivot: Pos2, rot_deg: f32) -> Pos2 {
    let rad = rot_deg.to_radians();
    let (s, c) = rad.sin_cos();
    let x = pt.x;
    let y = pt.y;
    Pos2::new(pivot.x + x * c - y * s, pivot.y + x * s + y * c)
}

/// Public transform helper (used by optional symbol providers).
#[inline]
pub fn xf_public(pt: Pos2, pivot: Pos2, rot_deg: f32) -> Pos2 {
    xf(pt, pivot, rot_deg)
}

#[allow(clippy::too_many_arguments)]
pub fn paint_symbol_body(
    painter: &Painter,
    pivot: Pos2,
    lw: f32,
    lh: f32,
    rot_deg: f32,
    kind: CompKind,
    ink: Color32,
    stroke_px: f32,
) {
    let stroke = Stroke::new(stroke_px.clamp(1.0, 3.5), ink);

    let seg = |a: Pos2, b: Pos2| {
        painter.line_segment([xf(a, pivot, rot_deg), xf(b, pivot, rot_deg)], stroke);
    };

    match kind {
        CompKind::Resistor => {
            let sx = lw * 0.92;
            let amp = lh * 0.38;
            let step = sx / 6.0;
            let mut prev = Pos2::new(-sx, 0.0);
            for i in 0..6usize {
                let x = -sx + step * i as f32 + step * 0.5;
                let y = if i % 2 == 0 { amp } else { -amp };
                let cur = Pos2::new(x, y);
                seg(prev, cur);
                prev = cur;
            }
            seg(prev, Pos2::new(sx, 0.0));
        }
        CompKind::Capacitor => {
            let gap = lw * 0.2;
            let plate_h = lh * 0.82;
            seg(Pos2::new(-lw, 0.0), Pos2::new(-gap, 0.0));
            seg(Pos2::new(gap, 0.0), Pos2::new(lw, 0.0));
            seg(
                Pos2::new(-gap, -plate_h * 0.5),
                Pos2::new(-gap, plate_h * 0.5),
            );
            seg(
                Pos2::new(gap, -plate_h * 0.5),
                Pos2::new(gap, plate_h * 0.5),
            );
        }
        CompKind::Inductor => {
            let coil_w = lw * 1.35;
            let amp = lh * 0.32;
            let steps = 36usize;
            seg(Pos2::new(-lw, 0.0), Pos2::new(-coil_w * 0.5, 0.0));
            let mut prev = Pos2::new(-coil_w * 0.5, 0.0);
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let x = -coil_w * 0.5 + t * coil_w;
                let y = (t * std::f32::consts::TAU * 2.75).sin() * amp;
                let cur = Pos2::new(x, y);
                seg(prev, cur);
                prev = cur;
            }
            seg(prev, Pos2::new(lw, 0.0));
        }
        CompKind::Diode => {
            let tip = lw * 0.52;
            let base = -lw * 0.32;
            let h = lh * 0.62;
            seg(Pos2::new(-lw, 0.0), Pos2::new(base, 0.0));
            seg(Pos2::new(tip, 0.0), Pos2::new(lw, 0.0));
            seg(Pos2::new(base, -h * 0.5), Pos2::new(base, h * 0.5));
            seg(Pos2::new(base, -h * 0.5), Pos2::new(tip, 0.0));
            seg(Pos2::new(base, h * 0.5), Pos2::new(tip, 0.0));
            let bar = tip + lh * 0.06;
            seg(Pos2::new(bar, -h * 0.52), Pos2::new(bar, h * 0.52));
        }
        CompKind::Transistor => {
            let bx = lw * 0.28;
            let tip = -lw * 0.52;
            let leg = lh * 0.58;
            seg(Pos2::new(-lw, leg * 0.42), Pos2::new(bx, leg * 0.42));
            seg(Pos2::new(-lw, -leg * 0.42), Pos2::new(bx, -leg * 0.42));
            seg(Pos2::new(bx, leg * 0.52), Pos2::new(lw, leg * 0.52));
            seg(Pos2::new(bx, -leg * 0.1), Pos2::new(bx, leg * 0.52));
            seg(Pos2::new(bx, -leg * 0.1), Pos2::new(tip, -leg * 0.38));
            seg(Pos2::new(bx, leg * 0.08), Pos2::new(tip, leg * 0.38));
            seg(Pos2::new(tip, -leg * 0.38), Pos2::new(tip, leg * 0.38));
            seg(
                Pos2::new(tip - lh * 0.05, -leg * 0.2),
                Pos2::new(tip - lh * 0.05, leg * 0.2),
            );
        }
        CompKind::IC => {
            let w = lw * 1.05;
            let h = lh * 1.22;
            let tl = xf(Pos2::new(-w, -h), pivot, rot_deg);
            let br = xf(Pos2::new(w, h), pivot, rot_deg);
            painter.rect_stroke(Rect::from_two_pos(tl, br), 0.0, stroke);
            let pins = 6usize;
            for i in 0..pins {
                let ty = -h + (2.0 * h) * (i as f32 + 0.5) / pins as f32;
                seg(Pos2::new(-w - lh * 0.12, ty), Pos2::new(-w, ty));
                seg(Pos2::new(w, ty), Pos2::new(w + lh * 0.12, ty));
            }
            let nr = lh * 0.18;
            let nc = xf(Pos2::new(-w + nr * 1.05, -h + nr * 1.05), pivot, rot_deg);
            painter.circle_stroke(nc, nr * 0.9, stroke);
        }
        CompKind::Generic => {
            let w = lw * 1.0;
            let h = lh * 1.05;
            let tl = xf(Pos2::new(-w, -h), pivot, rot_deg);
            let br = xf(Pos2::new(w, h), pivot, rot_deg);
            painter.rect_stroke(Rect::from_two_pos(tl, br), 0.0, stroke);
            seg(Pos2::new(-lw * 1.12, 0.0), Pos2::new(-w, 0.0));
            seg(Pos2::new(w, 0.0), Pos2::new(lw * 1.12, 0.0));
        }
    }
}
