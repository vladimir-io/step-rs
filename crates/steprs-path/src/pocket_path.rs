use steprs_core::ManufacturingFeatureKind;
use steprs_features::{ManufacturingFeature, PocketProfile};

use crate::gcode::ToolConfig;
use crate::toolpath::ToolpathProgram;

pub fn plan_pocket_or_slot(
    program: &mut ToolpathProgram,
    feature: &ManufacturingFeature,
    tool: &ToolConfig,
) -> bool {
    let Some(profile) = feature.pocket.as_ref() else {
        return false;
    };
    if profile.corners.len() < 3 {
        return false;
    }

    let depth = feature.depth.unwrap_or(5.0);
    let stepover = tool.diameter_mm * 0.5;
    let stepdown = (tool.diameter_mm * 0.5).min(depth / 4.0).max(0.5);
    let margin = tool.diameter_mm / 2.0;

    let poly = uv_polygon(profile);
    let (min_u, max_u, min_v, max_v) = profile.uv_bbox();
    let z_floor = profile.floor_z;
    let z_safe = z_floor + tool.safe_z_mm;

    let u_start = min_u + margin;
    let u_end = max_u - margin;
    let v_start = min_v + margin;
    let v_end = max_v - margin;

    if u_end <= u_start || v_end <= v_start {
        return false;
    }

    let mut z = z_floor;
    let z_final = z_floor - depth;

    program.push_rapid(profile.to_world(u_start, v_start, z_safe));

    while z > z_final + 1e-6 {
        z = (z - stepdown).max(z_final);
        let feed_plunge = tool.feed_mm_min / 2.0;
        let feed_cut = tool.feed_mm_min;

        program.push_linear(profile.to_world(u_start, v_start, z), feed_plunge, true);

        let mut v = v_start;
        let mut forward = true;
        while v <= v_end + 1e-9 {
            let Some((u_lo, u_hi)) = horizontal_slice(&poly, v, u_start, u_end) else {
                v += stepover;
                continue;
            };
            let u_a = u_lo.max(u_start);
            let u_b = u_hi.min(u_end);
            if u_b <= u_a + 1e-6 {
                v += stepover;
                continue;
            }
            let (u_from, u_to) = if forward { (u_a, u_b) } else { (u_b, u_a) };
            program.push_linear(profile.to_world(u_from, v, z), feed_cut, true);
            program.push_linear(profile.to_world(u_to, v, z), feed_cut, true);
            forward = !forward;

            let next_v = v + stepover;
            if next_v > v_end {
                if (v_end - v).abs() > 1e-6 {
                    if let Some((ulo, uhi)) = horizontal_slice(&poly, v_end, u_start, u_end) {
                        let ua = ulo.max(u_start);
                        let ub = uhi.min(u_end);
                        if ub > ua + 1e-6 {
                            let (uf, ut) = if forward { (ua, ub) } else { (ub, ua) };
                            program.push_linear(profile.to_world(uf, v_end, z), feed_cut, true);
                            program.push_linear(profile.to_world(ut, v_end, z), feed_cut, true);
                        }
                    }
                }
                break;
            }
            if let Some((ulo, uhi)) = horizontal_slice(&poly, next_v, u_start, u_end) {
                let stay = if forward {
                    uhi.min(u_end)
                } else {
                    ulo.max(u_start)
                };
                program.push_linear(profile.to_world(stay, next_v, z), feed_cut, true);
            }
            v = next_v;
        }
    }

    program.push_rapid(profile.to_world(u_start, v_start, z_safe));
    true
}

fn uv_polygon(profile: &PocketProfile) -> Vec<(f64, f64)> {
    profile
        .corners
        .iter()
        .map(|p| corner_uv(profile, *p))
        .collect()
}

fn corner_uv(profile: &PocketProfile, p: [f64; 3]) -> (f64, f64) {
    let du = p[0] - profile.origin[0];
    let dv = p[1] - profile.origin[1];
    let dw = p[2] - profile.origin[2];
    let u = du * profile.u_axis[0] + dv * profile.u_axis[1] + dw * profile.u_axis[2];
    let v = du * profile.v_axis[0] + dv * profile.v_axis[1] + dw * profile.v_axis[2];
    (u, v)
}

/// Horizontal scan-line intersection → usable u interval inside polygon.
fn horizontal_slice(poly: &[(f64, f64)], v: f64, u_min: f64, u_max: f64) -> Option<(f64, f64)> {
    let mut xs = Vec::new();
    let n = poly.len();
    for i in 0..n {
        let (u0, v0) = poly[i];
        let (u1, v1) = poly[(i + 1) % n];
        if (v0 - v) * (v1 - v) < 0.0 {
            let t = (v - v0) / (v1 - v0);
            xs.push(u0 + t * (u1 - u0));
        } else if (v - v0).abs() < 1e-9 {
            xs.push(u0);
        }
    }
    if xs.is_empty() {
        return None;
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo = *xs.first()? + 1e-6;
    let hi = *xs.last()? - 1e-6;
    if hi <= lo {
        return None;
    }
    Some((lo.max(u_min), hi.min(u_max)))
}

pub fn is_pocket_like(kind: ManufacturingFeatureKind) -> bool {
    matches!(
        kind,
        ManufacturingFeatureKind::RectangularPocket
            | ManufacturingFeatureKind::CircularPocket
            | ManufacturingFeatureKind::Slot
    )
}
