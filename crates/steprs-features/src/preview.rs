use serde::{Deserialize, Serialize};
use steprs_core::RecordStore;
use steprs_schema::{extract_mesh_scaled, SchemaCache, TessellationMesh, Vec3};
use steprs_topology::{BRepModel, SurfaceKind};

use crate::model::{FeatureModel, ManufacturingFeature};
use crate::tessellate::push_stock_box_mesh;

/// Reject malformed face geometry from dirty STEP exports (mm values > 100 m).
const MAX_COORD_MM: f64 = 100_000.0;

/// Lightweight analytic preview mesh for the browser (no tessellation kernel).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenePreview {
    pub stock: Option<StockBox>,
    pub cylinders: Vec<PreviewCylinder>,
    pub planes: Vec<PreviewPlane>,
    pub mesh: TessellationMesh,
    pub bounds: PreviewBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockBox {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewCylinder {
    pub id: u32,
    pub origin: [f64; 3],
    pub axis: [f64; 3],
    pub radius: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewPlane {
    pub id: u32,
    pub point: [f64; 3],
    pub normal: [f64; 3],
    pub half_extent: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PreviewBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

pub fn build_scene_preview(
    brep: &BRepModel,
    features: &FeatureModel,
    store: &RecordStore,
    cache: &SchemaCache,
) -> ScenePreview {
    let native = extract_mesh_scaled(store, cache.length_scale);
    let mut scene = ScenePreview {
        mesh: native,
        ..ScenePreview::default()
    };
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];

    for f in &features.features {
        extend_feature_bounds(&mut min, &mut max, f);
    }

    for face in &brep.faces {
        if let Some(p) = face.plane_point {
            extend_bounds_point_sane(&mut min, &mut max, v3(p));
        }
        if let (Some(o), Some(r)) = (face.axis_origin, face.radius) {
            extend_bounds_sphere_sane(&mut min, &mut max, v3(o), r);
        }
        match face.surface_kind {
            SurfaceKind::Cylinder => {
                if let (Some(o), Some(a), Some(r)) =
                    (face.axis_origin, face.axis_direction, face.radius)
                {
                    if !is_sane_point(v3(o)) {
                        continue;
                    }
                    let height = cylinder_height_for_face(face.id, features, r);
                    let cyl = PreviewCylinder {
                        id: face.id,
                        origin: v3(o),
                        axis: v3(a.normalize()),
                        radius: r,
                        height,
                    };
                    extend_bounds_cylinder(&mut min, &mut max, &cyl);
                    scene.cylinders.push(cyl);
                }
            }
            SurfaceKind::Plane => {
                if let (Some(p), Some(n)) = (face.plane_point, face.plane_normal) {
                    if !is_sane_point(v3(p)) {
                        continue;
                    }
                    let plane = PreviewPlane {
                        id: face.id,
                        point: v3(p),
                        normal: v3(n.normalize()),
                        half_extent: 15.0,
                    };
                    extend_bounds_point_sane(&mut min, &mut max, plane.point);
                    scene.planes.push(plane);
                }
            }
            _ => {}
        }
    }

    if min[0].is_finite() {
        scene.bounds.min = min;
        scene.bounds.max = max;
        let pad = (max_margin(&min, &max) * 0.08).clamp(2.0, 25.0);
        let stock = StockBox {
            min: [min[0] - pad, min[1] - pad, min[2] - pad],
            max: [max[0] + pad, max[1] + pad, max[2] + pad],
        };
        if scene.mesh.vertices.is_empty() {
            let mut mesh = TessellationMesh::default();
            push_stock_box_mesh(&mut mesh, stock.min, stock.max);
            scene.mesh = mesh;
        }
        scene.stock = Some(stock);
    }

    scene
}

fn is_sane_point(p: [f64; 3]) -> bool {
    p.iter()
        .all(|c| c.is_finite() && c.abs() <= MAX_COORD_MM)
}

fn extend_bounds_point_sane(min: &mut [f64; 3], max: &mut [f64; 3], p: [f64; 3]) {
    if is_sane_point(p) {
        extend_bounds_point(min, max, p);
    }
}

fn extend_bounds_sphere_sane(min: &mut [f64; 3], max: &mut [f64; 3], center: [f64; 3], radius: f64) {
    if !is_sane_point(center) || !radius.is_finite() || radius <= 0.0 || radius > MAX_COORD_MM {
        return;
    }
    extend_bounds_sphere(min, max, center, radius);
}

fn extend_feature_bounds(min: &mut [f64; 3], max: &mut [f64; 3], f: &ManufacturingFeature) {
    let Some(o) = f.axis_origin else { return };
    let r = f.radius.unwrap_or(1.0);
    if !is_sane_point(v3(o)) {
        return;
    }
    extend_bounds_sphere(min, max, v3(o), r);
    if let (Some(d), Some(dir)) = (f.depth, f.axis_direction) {
        let end = Vec3 {
            x: o.x + dir.x * d,
            y: o.y + dir.y * d,
            z: o.z + dir.z * d,
        };
        if is_sane_point(v3(end)) {
            extend_bounds_sphere(min, max, v3(end), r);
        }
    }
}

fn max_margin(min: &[f64; 3], max: &[f64; 3]) -> f64 {
    (0..3)
        .map(|i| (max[i] - min[i]).abs())
        .fold(0.0_f64, f64::max)
}

fn v3(v: Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}

fn cylinder_height_for_face(face_id: u32, features: &FeatureModel, radius: f64) -> f64 {
    for f in &features.features {
        if f.face_ids.contains(&face_id) && f.face_ids.len() >= 2 {
            return (radius * 2.0).max(1.0);
        }
        if f.face_ids.contains(&face_id) {
            if let Some(d) = f.depth {
                return d.min(radius * 4.0).max(radius);
            }
        }
    }
    (radius * 2.0).max(1.0)
}

fn extend_bounds_cylinder(min: &mut [f64; 3], max: &mut [f64; 3], c: &PreviewCylinder) {
    let r = c.radius;
    let ax = c.axis;
    let o = c.origin;
    let h = c.height;
    let end = [
        o[0] + ax[0] * h,
        o[1] + ax[1] * h,
        o[2] + ax[2] * h,
    ];
    for p in [o, end] {
        extend_bounds_sphere_sane(min, max, p, r);
    }
}

fn extend_bounds_point(min: &mut [f64; 3], max: &mut [f64; 3], p: [f64; 3]) {
    for i in 0..3 {
        min[i] = min[i].min(p[i]);
        max[i] = max[i].max(p[i]);
    }
}

fn extend_bounds_sphere(min: &mut [f64; 3], max: &mut [f64; 3], center: [f64; 3], radius: f64) {
    for i in 0..3 {
        min[i] = min[i].min(center[i] - radius);
        max[i] = max[i].max(center[i] + radius);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use steprs_core::ManufacturingFeatureKind;
    use smallvec::smallvec;

    #[test]
    fn rejects_garbage_plane_coordinates() {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        extend_bounds_point_sane(&mut min, &mut max, [-500_000.0, -12.0, -499_983.9]);
        assert!(!min[0].is_finite());
    }

    #[test]
    fn feature_bounds_anchor_stock() {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        let f = ManufacturingFeature {
            kind: ManufacturingFeatureKind::BlindHole,
            label: String::new(),
            face_ids: smallvec![1],
            axis_origin: Some(Vec3 {
                x: 11.6,
                y: -16.9,
                z: 28.0,
            }),
            axis_direction: Some(Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            }),
            radius: Some(3.5),
            depth: Some(28.5),
            width: None,
            length: None,
            pocket: None,
            confidence: 1.0,
            segment_diameters_mm: None,
            segment_depths_mm: None,
        };
        extend_feature_bounds(&mut min, &mut max, &f);
        assert!(min[0].is_finite());
        assert!(max[2] > 50.0);
        assert!(min[0] > -50.0);
    }
}
