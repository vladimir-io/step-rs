use std::collections::HashSet;

use steprs_schema::TessellationMesh;
use steprs_topology::{BRepModel, SurfaceKind};

use crate::geom::PlaneFrame;
use crate::model::FeatureModel;

const PLANE_SUBDIV: u32 = 10;
const CYL_SEGMENTS: u32 = 36;
const CONE_SEGMENTS: u32 = 32;
const TORUS_U: u32 = 32;
const TORUS_V: u32 = 16;

/// Analytic B-rep tessellation when STEP carries no native triangles.
/// Planes are omitted — full-scene plane patches read as overlapping sheets in the viewer.
pub fn tessellate_brep_faces(brep: &BRepModel, features: &FeatureModel) -> TessellationMesh {
    let mut mesh = TessellationMesh::default();
    let mut seen = HashSet::new();

    for face in &brep.faces {
        if !seen.insert(face.id) {
            continue;
        }
        match face.surface_kind {
            SurfaceKind::Cylinder => {
                if let (Some(o), Some(a), Some(r)) =
                    (face.axis_origin, face.axis_direction, face.radius)
                {
                    let height = cylinder_height_for_face(face.id, features, r);
                    push_cylinder_mesh(
                        &mut mesh,
                        [o.x as f32, o.y as f32, o.z as f32],
                        [a.x as f32, a.y as f32, a.z as f32],
                        r as f32,
                        height as f32,
                        CYL_SEGMENTS,
                    );
                }
            }
            SurfaceKind::Plane => {}
            SurfaceKind::Cone => {
                if let (Some(o), Some(a), Some(r), Some(angle)) = (
                    face.axis_origin,
                    face.axis_direction,
                    face.radius,
                    face.semi_angle,
                ) {
                    let height = cylinder_height_for_face(face.id, features, r);
                    push_cone_mesh(
                        &mut mesh,
                        [o.x as f32, o.y as f32, o.z as f32],
                        [a.x as f32, a.y as f32, a.z as f32],
                        r as f32,
                        angle as f32,
                        height as f32,
                        CONE_SEGMENTS,
                    );
                }
            }
            SurfaceKind::Torus => {
                if let (Some(o), Some(a), Some(major), Some(minor)) = (
                    face.axis_origin,
                    face.axis_direction,
                    face.major_radius,
                    face.minor_radius,
                ) {
                    push_torus_mesh(
                        &mut mesh,
                        [o.x as f32, o.y as f32, o.z as f32],
                        [a.x as f32, a.y as f32, a.z as f32],
                        major as f32,
                        minor as f32,
                        TORUS_U,
                        TORUS_V,
                    );
                }
            }
            SurfaceKind::Unknown => {}
        }
    }
    mesh
}

/// Solid stock envelope for browser preview when STEP has no native tessellation.
pub fn push_stock_box_mesh(mesh: &mut TessellationMesh, min: [f64; 3], max: [f64; 3]) {
    let mn = [min[0] as f32, min[1] as f32, min[2] as f32];
    let mx = [max[0] as f32, max[1] as f32, max[2] as f32];
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(&[
        [mn[0], mn[1], mn[2]],
        [mx[0], mn[1], mn[2]],
        [mx[0], mx[1], mn[2]],
        [mn[0], mx[1], mn[2]],
        [mn[0], mn[1], mx[2]],
        [mx[0], mn[1], mx[2]],
        [mx[0], mx[1], mx[2]],
        [mn[0], mx[1], mx[2]],
    ]);
    for face in [
        [4, 5, 6, 4, 6, 7],
        [0, 2, 1, 0, 3, 2],
        [1, 5, 4, 1, 4, 0],
        [2, 6, 5, 2, 5, 1],
        [3, 7, 6, 3, 6, 2],
        [0, 4, 7, 0, 7, 3],
    ] {
        mesh.indices.extend(face.iter().map(|i| base + i));
    }
}

#[allow(dead_code)]
fn scene_bounds(brep: &BRepModel) -> ([f64; 3], [f64; 3]) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for face in &brep.faces {
        if let Some(p) = face.plane_point {
            expand(&mut min, &mut max, [p.x, p.y, p.z]);
        }
        if let (Some(o), Some(r)) = (face.axis_origin, face.radius) {
            let r = r as f64;
            expand(&mut min, &mut max, [o.x - r, o.y - r, o.z - r]);
            expand(&mut min, &mut max, [o.x + r, o.y + r, o.z + r]);
        }
    }
    if !min[0].is_finite() {
        return ([-20.0, -20.0, -20.0], [20.0, 20.0, 20.0]);
    }
    (min, max)
}

fn expand(min: &mut [f64; 3], max: &mut [f64; 3], p: [f64; 3]) {
    for i in 0..3 {
        min[i] = min[i].min(p[i]);
        max[i] = max[i].max(p[i]);
    }
}

fn cylinder_height_for_face(face_id: u32, features: &FeatureModel, radius: f64) -> f64 {
    for f in &features.features {
        if f.face_ids.contains(&face_id) {
            if let Some(d) = f.depth {
                return d;
            }
        }
    }
    (radius * 3.0).max(8.0)
}

#[allow(dead_code)]
fn push_plane_patch(
    mesh: &mut TessellationMesh,
    origin: steprs_schema::Vec3,
    normal: steprs_schema::Vec3,
    bounds_min: [f64; 3],
    bounds_max: [f64; 3],
    subdiv: u32,
) {
    let frame = PlaneFrame::from_face(origin, normal, None);
    let corners = [
        [bounds_min[0], bounds_min[1], bounds_min[2]],
        [bounds_max[0], bounds_min[1], bounds_min[2]],
        [bounds_max[0], bounds_max[1], bounds_min[2]],
        [bounds_min[0], bounds_max[1], bounds_min[2]],
        [bounds_min[0], bounds_min[1], bounds_max[2]],
        [bounds_max[0], bounds_min[1], bounds_max[2]],
        [bounds_max[0], bounds_max[1], bounds_max[2]],
        [bounds_min[0], bounds_max[1], bounds_max[2]],
    ];
    let mut min_u = f64::INFINITY;
    let mut max_u = f64::NEG_INFINITY;
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for c in corners {
        let (u, v) = frame.project_array(c);
        min_u = min_u.min(u);
        max_u = max_u.max(u);
        min_v = min_v.min(v);
        max_v = max_v.max(v);
    }
    let pad = 2.0;
    min_u -= pad;
    max_u += pad;
    min_v -= pad;
    max_v += pad;

    let base = mesh.vertices.len() as u32;
    for iy in 0..=subdiv {
        let tv = iy as f32 / subdiv as f32;
        let v = min_v + (max_v - min_v) * tv as f64;
        for ix in 0..=subdiv {
            let tu = ix as f32 / subdiv as f32;
            let u = min_u + (max_u - min_u) * tu as f64;
            let w = frame.to_world(u, v);
            mesh.vertices.push([w[0] as f32, w[1] as f32, w[2] as f32]);
        }
    }
    let stride = subdiv + 1;
    for iy in 0..subdiv {
        for ix in 0..subdiv {
            let a = base + iy * stride + ix;
            let b = a + 1;
            let c = a + stride;
            let d = c + 1;
            mesh.indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
}

pub fn push_cylinder_mesh(
    mesh: &mut TessellationMesh,
    origin: [f32; 3],
    axis: [f32; 3],
    radius: f32,
    height: f32,
    segments: u32,
) {
    let (u, v, az) = orthonormal_basis(axis);
    let base = mesh.vertices.len() as u32;
    for ring in 0..2u32 {
        let h = -height * 0.5 + height * ring as f32;
        for s in 0..segments {
            let t = (s as f32 / segments as f32) * std::f32::consts::TAU;
            let (ct, st) = (t.cos(), t.sin());
            mesh.vertices.push([
                origin[0] + u[0] * radius * ct + v[0] * radius * st + az[0] * h,
                origin[1] + u[1] * radius * ct + v[1] * radius * st + az[1] * h,
                origin[2] + u[2] * radius * ct + v[2] * radius * st + az[2] * h,
            ]);
        }
    }
    for s in 0..segments {
        let s1 = (s + 1) % segments;
        let a0 = base + s;
        let a1 = base + s1;
        let b0 = base + segments + s;
        let b1 = base + segments + s1;
        mesh.indices.extend_from_slice(&[a0, b0, a1, a1, b0, b1]);
    }
}

fn push_cone_mesh(
    mesh: &mut TessellationMesh,
    origin: [f32; 3],
    axis: [f32; 3],
    radius: f32,
    semi_angle: f32,
    height: f32,
    segments: u32,
) {
    let (u, v, az) = orthonormal_basis(axis);
    let tan_a = semi_angle.tan();
    let apex = [
        origin[0] - az[0] * height * 0.5,
        origin[1] - az[1] * height * 0.5,
        origin[2] - az[2] * height * 0.5,
    ];
    let base_center = [
        origin[0] + az[0] * height * 0.5,
        origin[1] + az[1] * height * 0.5,
        origin[2] + az[2] * height * 0.5,
    ];
    let base_r = radius + height * tan_a;

    let apex_idx = mesh.vertices.len() as u32;
    mesh.vertices.push(apex);

    let base = mesh.vertices.len() as u32;
    for s in 0..segments {
        let t = (s as f32 / segments as f32) * std::f32::consts::TAU;
        let (ct, st) = (t.cos(), t.sin());
        mesh.vertices.push([
            base_center[0] + u[0] * base_r * ct + v[0] * base_r * st,
            base_center[1] + u[1] * base_r * ct + v[1] * base_r * st,
            base_center[2] + u[2] * base_r * ct + v[2] * base_r * st,
        ]);
    }
    for s in 0..segments {
        let s1 = (s + 1) % segments;
        mesh.indices
            .extend_from_slice(&[apex_idx, base + s, base + s1]);
    }
}

fn push_torus_mesh(
    mesh: &mut TessellationMesh,
    origin: [f32; 3],
    axis: [f32; 3],
    major_r: f32,
    minor_r: f32,
    segments_u: u32,
    segments_v: u32,
) {
    let (u, v, az) = orthonormal_basis(axis);
    let base = mesh.vertices.len() as u32;
    for iv in 0..=segments_v {
        let phi = (iv as f32 / segments_v as f32) * std::f32::consts::TAU;
        let (sp, cp) = (phi.sin(), phi.cos());
        let ring_r = major_r + minor_r * cp;
        let ring_offset = minor_r * sp;
        for iu in 0..=segments_u {
            let theta = (iu as f32 / segments_u as f32) * std::f32::consts::TAU;
            let (st, ct) = (theta.sin(), theta.cos());
            let px = u[0] * ring_r * ct + v[0] * ring_r * st + az[0] * ring_offset;
            let py = u[1] * ring_r * ct + v[1] * ring_r * st + az[1] * ring_offset;
            let pz = u[2] * ring_r * ct + v[2] * ring_r * st + az[2] * ring_offset;
            mesh.vertices
                .push([origin[0] + px, origin[1] + py, origin[2] + pz]);
        }
    }
    let stride = segments_u + 1;
    for iv in 0..segments_v {
        for iu in 0..segments_u {
            let a = base + iv * stride + iu;
            let b = a + 1;
            let c = a + stride;
            let d = c + 1;
            mesh.indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
}

fn orthonormal_basis(axis: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2])
        .sqrt()
        .max(1e-6);
    let az = [axis[0] / len, axis[1] / len, axis[2] / len];
    let up = if az[2].abs() < 0.9 {
        [0.0f32, 0.0, 1.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let ux = up[1] * az[2] - up[2] * az[1];
    let uy = up[2] * az[0] - up[0] * az[2];
    let uz = up[0] * az[1] - up[1] * az[0];
    let ul = (ux * ux + uy * uy + uz * uz).sqrt().max(1e-6);
    let u = [ux / ul, uy / ul, uz / ul];
    let vx = az[1] * u[2] - az[2] * u[1];
    let vy = az[2] * u[0] - az[0] * u[2];
    let vz = az[0] * u[1] - az[1] * u[0];
    let v = [vx, vy, vz];
    (u, v, az)
}
