use serde::{Deserialize, Serialize};
use steprs_core::RecordStore;
use steprs_schema::{extract_mesh_scaled, SchemaCache, TessellationMesh, Vec3};
use steprs_topology::{BRepModel, SurfaceKind};

use crate::model::FeatureModel;
use crate::tessellate::tessellate_brep_faces;

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
    let mut mesh = extract_mesh_scaled(store, cache.length_scale);
    if mesh.vertices.is_empty() {
        mesh = tessellate_brep_faces(brep, features);
    }
    let mut scene = ScenePreview {
        mesh,
        ..ScenePreview::default()
    };
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];

    for face in &brep.faces {
        match face.surface_kind {
            SurfaceKind::Cylinder => {
                if let (Some(o), Some(a), Some(r)) =
                    (face.axis_origin, face.axis_direction, face.radius)
                {
                    let height = cylinder_height_for_face(face.id, features, r);
                    let cyl = PreviewCylinder {
                        id: face.id,
                        origin: v3(o),
                        axis: v3(a.normalize()),
                        radius: r,
                        height,
                    };
                    extend_bounds(&mut min, &mut max, &cyl);
                    scene.cylinders.push(cyl);
                }
            }
            SurfaceKind::Plane => {
                if let (Some(p), Some(n)) = (face.plane_point, face.plane_normal) {
                    let plane = PreviewPlane {
                        id: face.id,
                        point: v3(p),
                        normal: v3(n.normalize()),
                        half_extent: 15.0,
                    };
                    extend_bounds_point(&mut min, &mut max, plane.point);
                    scene.planes.push(plane);
                }
            }
            _ => {}
        }
    }

    for f in &features.features {
        if let (Some(o), Some(r)) = (f.axis_origin, f.radius) {
            let depth = f.depth.unwrap_or(r * 2.0);
            let cyl = PreviewCylinder {
                id: f.face_ids.first().copied().unwrap_or(0),
                origin: v3(o),
                axis: [0.0, 0.0, 1.0],
                radius: r,
                height: depth,
            };
            extend_bounds(&mut min, &mut max, &cyl);
            if !scene.cylinders.iter().any(|c| c.id == cyl.id) {
                scene.cylinders.push(cyl);
            }
        }
    }

    if min[0].is_finite() {
        scene.bounds.min = min;
        scene.bounds.max = max;
        let pad = 5.0;
        scene.stock = Some(StockBox {
            min: [min[0] - pad, min[1] - pad, min[2] - pad],
            max: [max[0] + pad, max[1] + pad, max[2] + pad],
        });
    }

    scene
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

fn extend_bounds(min: &mut [f64; 3], max: &mut [f64; 3], c: &PreviewCylinder) {
    let r = c.radius;
    for dx in [-r, r] {
        for dy in [-r, r] {
            extend_bounds_point(min, max, [c.origin[0] + dx, c.origin[1] + dy, c.origin[2]]);
            extend_bounds_point(
                min,
                max,
                [c.origin[0] + dx, c.origin[1] + dy, c.origin[2] - c.height],
            );
        }
    }
}

fn extend_bounds_point(min: &mut [f64; 3], max: &mut [f64; 3], p: [f64; 3]) {
    for i in 0..3 {
        min[i] = min[i].min(p[i]);
        max[i] = max[i].max(p[i]);
    }
}
