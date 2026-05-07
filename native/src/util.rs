//! Small helpers shared by UI and placement logic.

use crate::canvas::Sym;
use egui::Pos2;

pub fn dist_point_to_segment_px(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let vx = b.x - a.x;
    let vy = b.y - a.y;
    let wx = p.x - a.x;
    let wy = p.y - a.y;
    let len2 = vx * vx + vy * vy;
    let t = if len2 > 1e-6 {
        (wx * vx + wy * vy) / len2
    } else {
        0.0
    };
    let t = t.clamp(0.0, 1.0);
    let px = a.x + vx * t;
    let py = a.y + vy * t;
    let dx = p.x - px;
    let dy = p.y - py;
    (dx * dx + dy * dy).sqrt()
}

pub fn truncate_ui_chars(s: &str, max_chars: usize) -> String {
    let n = s.chars().count();
    let mut t: String = s.chars().take(max_chars).collect();
    if n > max_chars {
        t.push('…');
    }
    t
}

pub fn guess_prefix(mpn: &str) -> &'static str {
    let c = mpn
        .trim()
        .chars()
        .next()
        .unwrap_or('U')
        .to_ascii_uppercase();
    match c {
        'R' => "R",
        'C' => "C",
        'L' => "L",
        'D' => "D",
        'Q' => "Q",
        _ => "U",
    }
}

pub fn next_refdes(symbols: &[Sym], prefix: &str) -> String {
    let mut max = 0u32;
    for s in symbols {
        if let Some(rest) = s.ref_des.strip_prefix(prefix) {
            if let Ok(n) = rest.parse::<u32>() {
                max = max.max(n);
            }
        }
    }
    format!("{prefix}{}", max + 1)
}
