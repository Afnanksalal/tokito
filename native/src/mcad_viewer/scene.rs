//! 3D scene description from schematic symbol placements.

use glam::Vec3;

#[derive(Clone, Debug)]
pub struct Placement3d {
    pub ref_des: String,
    pub footprint: String,
    pub center_mm: Vec3,
    pub size_mm: Vec3,
    pub rotation_y_rad: f32,
}

/// Heuristic board-space layout from schematic XY (mm) and footprint name.
pub fn placements_from_symbols(symbols: &[crate::canvas::Sym]) -> Vec<Placement3d> {
    symbols
        .iter()
        .filter_map(|s| {
            let fp = s.footprint_ref.as_deref().filter(|f| !f.is_empty())?;
            let size = footprint_dimensions(fp);
            Some(Placement3d {
                ref_des: s.ref_des.clone(),
                footprint: fp.to_string(),
                center_mm: Vec3::new(s.pos.x * 0.0254, 0.0, -s.pos.y * 0.0254),
                size_mm: size,
                rotation_y_rad: s.rotation_deg.to_radians(),
            })
        })
        .collect()
}

/// Parse common footprint name patterns → (width, height, thickness) mm.
pub fn footprint_dimensions(footprint: &str) -> Vec3 {
    let f = footprint.to_ascii_uppercase();
    if f.contains("0805") || f.contains("2012") {
        return Vec3::new(2.0, 0.6, 1.25);
    }
    if f.contains("1206") || f.contains("3216") {
        return Vec3::new(3.2, 0.6, 1.6);
    }
    if f.contains("SOT-23") || f.contains("SOT23") {
        return Vec3::new(3.0, 1.2, 2.9);
    }
    if f.contains("SOIC-8") || f.contains("SOIC8") {
        return Vec3::new(5.0, 1.75, 4.0);
    }
    if f.contains("QFP") || f.contains("LQFP") {
        return Vec3::new(10.0, 1.6, 10.0);
    }
    if f.contains("1X02") || f.contains("1x02") {
        return Vec3::new(5.08, 2.54, 8.0);
    }
    if f.contains("1X04") || f.contains("1x04") {
        return Vec3::new(10.16, 2.54, 8.0);
    }
    Vec3::new(5.0, 1.0, 5.0)
}
