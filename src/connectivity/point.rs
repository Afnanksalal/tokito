//! Schematic point keys (shared by editor and document export).

use super::disjoint_set::PointKey;

/// Quantization step for electrical coincidence (matches document model).
pub const POINT_QUANTUM: f64 = 0.001;

#[inline]
pub fn point_key_xy(x: f64, y: f64) -> PointKey {
    (
        (x / POINT_QUANTUM).round() as i64,
        (y / POINT_QUANTUM).round() as i64,
    )
}

#[inline]
pub fn point_key_f32(x: f32, y: f32) -> PointKey {
    point_key_xy(x as f64, y as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearby_points_snap_together() {
        assert_eq!(point_key_f32(40.0004, 80.0002), point_key_f32(40.0, 80.0));
    }
}
