use serde::{Deserialize, Serialize};

use crate::toolpath::{ToolpathProgram, ToolpathSegment};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockSimulation {
    pub grid_nx: u32,
    pub grid_ny: u32,
    pub bounds_min: [f64; 2],
    pub bounds_max: [f64; 2],
    /// Remaining material height per cell (mm), row-major.
    pub remaining_height: Vec<f32>,
    pub removed_volume_mm3: f64,
    /// Attempted cut above existing material (air / fixture).
    pub collision_cells: u32,
    /// Cut below stock zero plane.
    pub gouge_cells: u32,
    /// Cut deeper than planned floor (part gouge).
    pub overcut_cells: u32,
}

pub fn simulate_stock(
    stock_min: [f64; 3],
    stock_max: [f64; 3],
    toolpath: &ToolpathProgram,
    tool_diameter_mm: f64,
    grid: u32,
    floor_z: Option<f64>,
) -> StockSimulation {
    let grid = grid.clamp(48, 256);
    let nx = grid;
    let ny = grid;
    let cell_count = (nx * ny) as usize;
    let mut heights = vec![0f32; cell_count];
    let dx = (stock_max[0] - stock_min[0]) / nx as f64;
    let dy = (stock_max[1] - stock_min[1]) / ny as f64;
    let initial_h = (stock_max[2] - stock_min[2]).max(0.1) as f32;
    let floor = floor_z.unwrap_or(stock_min[2]) as f32;

    for h in heights.iter_mut() {
        *h = initial_h;
    }

    let r = (tool_diameter_mm / 2.0) as f32;
    let r2 = r * r;
    let mut removed = 0.0f64;
    let mut collisions = 0u32;
    let mut gouges = 0u32;
    let mut overcuts = 0u32;

    let mut pos = [stock_min[0], stock_min[1], stock_max[2] + 5.0];

    for seg in &toolpath.segments {
        let samples = sample_segment(pos, seg);
        let is_cut = matches!(
            seg,
            ToolpathSegment::Linear { .. }
                | ToolpathSegment::Arc { .. }
                | ToolpathSegment::Drill { .. }
        );

        for p in samples {
            if is_cut {
                let (rv, col, gouge, over) = carve_local(
                    &mut heights,
                    nx,
                    ny,
                    stock_min,
                    dx,
                    dy,
                    p[0],
                    p[1],
                    p[2],
                    r,
                    r2,
                    floor,
                );
                removed += rv;
                collisions += col;
                gouges += gouge;
                overcuts += over;
            }
            pos = p;
        }
    }

    StockSimulation {
        grid_nx: nx,
        grid_ny: ny,
        bounds_min: [stock_min[0], stock_min[1]],
        bounds_max: [stock_max[0], stock_max[1]],
        remaining_height: heights,
        removed_volume_mm3: removed,
        collision_cells: collisions,
        gouge_cells: gouges,
        overcut_cells: overcuts,
    }
}

fn sample_segment(from: [f64; 3], seg: &ToolpathSegment) -> Vec<[f64; 3]> {
    match seg {
        ToolpathSegment::Rapid { to } | ToolpathSegment::Linear { to, .. } => {
            let len = dist(from, *to);
            let steps = ((len / 0.5).ceil() as usize).clamp(2, 64);
            linear_samples(from, *to, steps)
        }
        ToolpathSegment::Arc {
            to,
            center_offset,
            clockwise,
            ..
        } => {
            arc_samples(from, *to, *center_offset, *clockwise, 32)
        }
        ToolpathSegment::Drill {
            xy,
            z_safe,
            z_bottom,
            ..
        } => {
            let mut pts = linear_samples(from, [xy[0], xy[1], *z_safe], 4);
            pts.extend(linear_samples(
                [xy[0], xy[1], *z_safe],
                [xy[0], xy[1], *z_bottom],
                16,
            ));
            pts.extend(linear_samples(
                [xy[0], xy[1], *z_bottom],
                [xy[0], xy[1], *z_safe],
                4,
            ));
            pts
        }
    }
}

fn linear_samples(from: [f64; 3], to: [f64; 3], steps: usize) -> Vec<[f64; 3]> {
    let mut out = Vec::with_capacity(steps);
    for s in 1..=steps {
        let t = s as f64 / steps as f64;
        out.push([
            from[0] + (to[0] - from[0]) * t,
            from[1] + (to[1] - from[1]) * t,
            from[2] + (to[2] - from[2]) * t,
        ]);
    }
    out
}

fn arc_samples(
    from: [f64; 3],
    to: [f64; 3],
    center_offset: [f64; 2],
    clockwise: bool,
    steps: usize,
) -> Vec<[f64; 3]> {
    let cx = from[0] + center_offset[0];
    let cy = from[1] + center_offset[1];
    let r = (center_offset[0].powi(2) + center_offset[1].powi(2)).sqrt().max(1e-6);
    let a0 = (from[1] - cy).atan2(from[0] - cx);
    let a1 = (to[1] - cy).atan2(to[0] - cx);
    let mut delta = a1 - a0;
    if clockwise {
        if delta >= 0.0 {
            delta -= std::f64::consts::TAU;
        }
    } else if delta <= 0.0 {
        delta += std::f64::consts::TAU;
    }
    let mut out = Vec::with_capacity(steps);
    for s in 1..=steps {
        let t = s as f64 / steps as f64;
        let a = a0 + delta * t;
        out.push([
            cx + r * a.cos(),
            cy + r * a.sin(),
            from[2] + (to[2] - from[2]) * t,
        ]);
    }
    out
}

fn carve_local(
    heights: &mut [f32],
    nx: u32,
    ny: u32,
    origin: [f64; 3],
    dx: f64,
    dy: f64,
    x: f64,
    y: f64,
    z: f64,
    r: f32,
    r2: f32,
    floor: f32,
) -> (f64, u32, u32, u32) {
    let mut removed = 0.0;
    let mut col = 0u32;
    let mut gouge = 0u32;
    let mut over = 0u32;

    let ix0 = ((x - origin[0] - r as f64) / dx).floor().max(0.0) as u32;
    let ix1 = ((x - origin[0] + r as f64) / dx).ceil().min(nx as f64 - 1.0) as u32;
    let iy0 = ((y - origin[1] - r as f64) / dy).floor().max(0.0) as u32;
    let iy1 = ((y - origin[1] + r as f64) / dy).ceil().min(ny as f64 - 1.0) as u32;

    let target = (z - origin[2]) as f32;

    for iy in iy0..=iy1 {
        for ix in ix0..=ix1 {
            let cx = origin[0] + (ix as f64 + 0.5) * dx;
            let cy = origin[1] + (iy as f64 + 0.5) * dy;
            let ddx = (cx - x) as f32;
            let ddy = (cy - y) as f32;
            if ddx * ddx + ddy * ddy > r2 {
                continue;
            }
            let idx = (iy * nx + ix) as usize;
            let before = heights[idx];
            if target < 0.0 {
                gouge += 1;
            }
            if target < floor - 0.02 {
                over += 1;
            }
            if target < before {
                let cell_vol = dx * dy * (before - target.max(0.0)) as f64;
                removed += cell_vol;
                heights[idx] = target.max(0.0);
            } else if target > before + 0.01 {
                col += 1;
            }
        }
    }

    (removed, col, gouge, over)
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let dz = b[2] - a[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}
