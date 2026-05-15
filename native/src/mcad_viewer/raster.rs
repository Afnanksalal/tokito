//! Fast CPU 3D board preview (perspective + z-buffer + Lambert shading).

use glam::{Mat4, Vec3};

use super::scene::Placement3d;

pub fn render_board(
    placements: &[Placement3d],
    width: u32,
    height: u32,
    yaw: f32,
    pitch: f32,
    distance: f32,
    pan: glam::Vec2,
) -> Vec<u8> {
    let w = width.max(1) as usize;
    let h = height.max(1) as usize;
    let mut color = vec![22u8; w * h * 4];
    let mut depth = vec![f32::INFINITY; w * h];

    let aspect = w as f32 / h as f32;
    let proj = Mat4::perspective_rh(45_f32.to_radians(), aspect, 1.0, 2000.0);
    let eye = Vec3::new(
        distance * pitch.cos() * yaw.sin(),
        distance * pitch.sin().max(0.05),
        distance * pitch.cos() * yaw.cos(),
    ) + Vec3::new(pan.x, 0.0, pan.y);
    let view = Mat4::look_at_rh(eye, Vec3::new(pan.x, 0.0, pan.y), Vec3::Y);
    let view_proj = proj * view;

    draw_grid(&mut color, &mut depth, w, h, view_proj);

    for (i, p) in placements.iter().enumerate() {
        let tint = 0.55 + (i % 5) as f32 * 0.06;
        draw_box(
            &mut color,
            &mut depth,
            w,
            h,
            view_proj,
            p.center_mm,
            p.size_mm,
            p.rotation_y_rad,
            Vec3::new(tint, tint + 0.05, tint + 0.12),
        );
    }

    color
}

fn draw_grid(color: &mut [u8], depth: &mut [f32], w: usize, h: usize, vp: Mat4) {
    let grid = 40.0;
    for i in -8..=8 {
        let t = i as f32 * grid;
        line3d(
            color,
            depth,
            w,
            h,
            vp,
            Vec3::new(t, 0.0, -grid * 8.0),
            Vec3::new(t, 0.0, grid * 8.0),
            [40, 44, 52, 255],
        );
        line3d(
            color,
            depth,
            w,
            h,
            vp,
            Vec3::new(-grid * 8.0, 0.0, t),
            Vec3::new(grid * 8.0, 0.0, t),
            [40, 44, 52, 255],
        );
    }
}

fn draw_box(
    color: &mut [u8],
    depth: &mut [f32],
    w: usize,
    h: usize,
    vp: Mat4,
    center: Vec3,
    size: Vec3,
    rot_y: f32,
    base: Vec3,
) {
    let half = size * 0.5;
    let model = Mat4::from_translation(center) * Mat4::from_rotation_y(rot_y);
    let corners: [Vec3; 8] = [
        Vec3::new(-half.x, -half.y, -half.z),
        Vec3::new(half.x, -half.y, -half.z),
        Vec3::new(half.x, half.y, -half.z),
        Vec3::new(-half.x, half.y, -half.z),
        Vec3::new(-half.x, -half.y, half.z),
        Vec3::new(half.x, -half.y, half.z),
        Vec3::new(half.x, half.y, half.z),
        Vec3::new(-half.x, half.y, half.z),
    ];
    let world: Vec<Vec3> = corners
        .iter()
        .map(|c| (model * c.extend(1.0)).truncate())
        .collect();
    let faces: [(usize, usize, usize, usize, Vec3); 6] = [
        (0, 1, 2, 3, Vec3::new(0.0, 0.0, -1.0)),
        (5, 4, 7, 6, Vec3::new(0.0, 0.0, 1.0)),
        (4, 0, 3, 7, Vec3::new(-1.0, 0.0, 0.0)),
        (1, 5, 6, 2, Vec3::new(1.0, 0.0, 0.0)),
        (3, 2, 6, 7, Vec3::new(0.0, 1.0, 0.0)),
        (4, 5, 1, 0, Vec3::new(0.0, -1.0, 0.0)),
    ];
    let light = Vec3::new(0.3, 0.9, 0.4).normalize();
    for (a, b, c, d, normal) in faces {
        let n = (model * normal.extend(0.0)).truncate().normalize();
        let shade = 0.35 + 0.65 * n.dot(light).max(0.0);
        let col = [
            (base.x * shade * 255.0) as u8,
            (base.y * shade * 255.0) as u8,
            (base.z * shade * 255.0) as u8,
            255,
        ];
        fill_quad(
            color, depth, w, h, vp, world[a], world[b], world[c], world[d], col,
        );
    }
}

fn fill_quad(
    color: &mut [u8],
    depth: &mut [f32],
    w: usize,
    h: usize,
    vp: Mat4,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    d: Vec3,
    col: [u8; 4],
) {
    fill_tri(color, depth, w, h, vp, a, b, c, col);
    fill_tri(color, depth, w, h, vp, a, c, d, col);
}

fn fill_tri(
    color: &mut [u8],
    depth: &mut [f32],
    w: usize,
    h: usize,
    vp: Mat4,
    a: Vec3,
    b: Vec3,
    c: Vec3,
    col: [u8; 4],
) {
    let pa = project(vp, a, w, h);
    let pb = project(vp, b, w, h);
    let pc = project(vp, c, w, h);
    if pa.2 < 0.0 || pb.2 < 0.0 || pc.2 < 0.0 {
        return;
    }
    let sa = (pa.0, pa.1);
    let sb = (pb.0, pb.1);
    let sc = (pc.0, pc.1);
    let min_x = pa.0.min(pb.0).min(pc.0).max(0.0) as i32;
    let max_x = pa.0.max(pb.0).max(pc.0).min((w - 1) as f32) as i32;
    let min_y = pa.1.min(pb.1).min(pc.1).max(0.0) as i32;
    let max_y = pa.1.max(pb.1).max(pc.1).min((h - 1) as f32) as i32;
    let area = edge2(sa, sb, sc);
    if area.abs() < 1e-4 {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let w0 = edge2(sb, sc, p) / area;
            let w1 = edge2(sc, sa, p) / area;
            let w2 = edge2(sa, sb, p) / area;
            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let z = w0 * pa.2 + w1 * pb.2 + w2 * pc.2;
                let idx = y as usize * w + x as usize;
                if z < depth[idx] {
                    depth[idx] = z;
                    let o = idx * 4;
                    color[o..o + 4].copy_from_slice(&col);
                }
            }
        }
    }
}

fn line3d(
    color: &mut [u8],
    depth: &mut [f32],
    w: usize,
    h: usize,
    vp: Mat4,
    a: Vec3,
    b: Vec3,
    col: [u8; 4],
) {
    let steps = 32;
    for i in 0..steps {
        let t0 = i as f32 / steps as f32;
        let t1 = (i + 1) as f32 / steps as f32;
        fill_tri(
            color,
            depth,
            w,
            h,
            vp,
            a.lerp(b, t0),
            a.lerp(b, t1),
            a.lerp(b, t0) + Vec3::Y * 0.01,
            col,
        );
    }
}

fn project(vp: Mat4, p: Vec3, w: usize, h: usize) -> (f32, f32, f32) {
    let clip = vp * p.extend(1.0);
    if clip.w <= 0.001 {
        return (0.0, 0.0, -1.0);
    }
    let ndc = clip.truncate() / clip.w;
    let x = (ndc.x * 0.5 + 0.5) * w as f32;
    let y = (1.0 - (ndc.y * 0.5 + 0.5)) * h as f32;
    (x, y, clip.w)
}

fn edge2(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (c.0 - a.0) * (b.1 - a.1) - (c.1 - a.1) * (b.0 - a.0)
}
