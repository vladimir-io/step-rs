use crate::model::{BRepModel, Face, FaceAdjacency, FaceLoop, Solid, SurfaceKind};
use smallvec::SmallVec;
use steprs_core::RecordStore;
use steprs_schema::{AdvancedFace, SchemaCache};

pub fn build_brep(store: &RecordStore, cache: &SchemaCache) -> BRepModel {
    let mut faces = Vec::new();

    for advanced in &cache.faces {
        faces.push(build_face(store, cache, advanced));
    }

    let mut solids = Vec::new();
    for solid in &cache.solids {
        if let Some(shell) = cache.shells.iter().find(|s| s.id == solid.shell_id) {
            solids.push(Solid {
                id: solid.id,
                shell_id: shell.id,
                face_ids: shell.face_ids.clone().into(),
            });
        }
    }

    let adjacency = compute_adjacency(&faces, store, cache);

    BRepModel {
        solids,
        faces,
        adjacency,
    }
}

fn build_face(store: &RecordStore, cache: &SchemaCache, advanced: &AdvancedFace) -> Face {
    let info = resolve_surface(store, cache, advanced.surface_id);

    let bound_loops: SmallVec<[FaceLoop; 4]> = advanced
        .bound_ids
        .iter()
        .filter_map(|bid| {
            cache.face_bounds.iter().find(|fb| fb.id == *bid).map(|fb| {
                FaceLoop {
                    loop_id: fb.loop_id,
                    is_outer: fb.is_outer,
                }
            })
        })
        .collect();
    let bound_loop_ids: SmallVec<[u32; 4]> =
        bound_loops.iter().map(|l| l.loop_id).collect();

    Face {
        id: advanced.id,
        surface_id: advanced.surface_id,
        surface_kind: info.kind,
        bound_loops,
        bound_loop_ids,
        same_sense: advanced.same_sense,
        axis_origin: info.axis_origin,
        axis_direction: info.axis_direction,
        radius: info.radius,
        semi_angle: info.semi_angle,
        major_radius: info.major_radius,
        minor_radius: info.minor_radius,
        plane_normal: info.plane_normal,
        plane_point: info.plane_point,
    }
}

struct SurfaceInfo {
    kind: SurfaceKind,
    axis_origin: Option<steprs_schema::Vec3>,
    axis_direction: Option<steprs_schema::Vec3>,
    radius: Option<f64>,
    semi_angle: Option<f64>,
    major_radius: Option<f64>,
    minor_radius: Option<f64>,
    plane_normal: Option<steprs_schema::Vec3>,
    plane_point: Option<steprs_schema::Vec3>,
}

fn resolve_surface(store: &RecordStore, cache: &SchemaCache, surface_id: u32) -> SurfaceInfo {
    if let Some(cyl) = cache.cylinders.iter().find(|c| c.id == surface_id) {
        let axis = cache.resolve_axis(store, cyl.placement_id);
        return SurfaceInfo {
            kind: SurfaceKind::Cylinder,
            axis_origin: axis.map(|a| a.origin),
            axis_direction: axis.map(|a| a.z),
            radius: Some(cyl.radius),
            semi_angle: None,
            major_radius: None,
            minor_radius: None,
            plane_normal: None,
            plane_point: None,
        };
    }
    if let Some(cone) = cache.cones.iter().find(|c| c.id == surface_id) {
        let axis = cache.resolve_axis(store, cone.placement_id);
        return SurfaceInfo {
            kind: SurfaceKind::Cone,
            axis_origin: axis.map(|a| a.origin),
            axis_direction: axis.map(|a| a.z),
            radius: Some(cone.radius),
            semi_angle: Some(cone.semi_angle),
            major_radius: None,
            minor_radius: None,
            plane_normal: None,
            plane_point: None,
        };
    }
    if let Some(torus) = cache.tori.iter().find(|t| t.id == surface_id) {
        let axis = cache.resolve_axis(store, torus.placement_id);
        return SurfaceInfo {
            kind: SurfaceKind::Torus,
            axis_origin: axis.map(|a| a.origin),
            axis_direction: axis.map(|a| a.z),
            radius: None,
            semi_angle: None,
            major_radius: Some(torus.major_radius),
            minor_radius: Some(torus.minor_radius),
            plane_normal: None,
            plane_point: None,
        };
    }
    if let Some(plane) = cache.planes.iter().find(|p| p.id == surface_id) {
        let axis = cache.resolve_axis(store, plane.placement_id);
        return SurfaceInfo {
            kind: SurfaceKind::Plane,
            axis_origin: None,
            axis_direction: None,
            radius: None,
            semi_angle: None,
            major_radius: None,
            minor_radius: None,
            plane_normal: axis.map(|a| a.z),
            plane_point: axis.map(|a| a.origin),
        };
    }
    if let Some(record) = store.get(surface_id) {
        match record.name.as_str() {
            "CYLINDRICAL_SURFACE" => {
                if let (Some(pid), Some(r)) =
                    (record.ref_at(1), record.param(2).and_then(|p| p.as_real()))
                {
                    let axis = cache.resolve_axis(store, pid);
                    return SurfaceInfo {
                        kind: SurfaceKind::Cylinder,
                        axis_origin: axis.map(|a| a.origin),
                        axis_direction: axis.map(|a| a.z),
                        radius: Some(r * cache.length_scale),
                        semi_angle: None,
                        major_radius: None,
                        minor_radius: None,
                        plane_normal: None,
                        plane_point: None,
                    };
                }
            }
            "CONICAL_SURFACE" => {
                if let (Some(pid), Some(r), Some(a)) = (
                    record.ref_at(1),
                    record.param(2).and_then(|p| p.as_real()),
                    record.param(3).and_then(|p| p.as_real()),
                ) {
                    let axis = cache.resolve_axis(store, pid);
                    return SurfaceInfo {
                        kind: SurfaceKind::Cone,
                        axis_origin: axis.map(|a| a.origin),
                        axis_direction: axis.map(|a| a.z),
                        radius: Some(r * cache.length_scale),
                        semi_angle: Some(a),
                        major_radius: None,
                        minor_radius: None,
                        plane_normal: None,
                        plane_point: None,
                    };
                }
            }
            "TOROIDAL_SURFACE" => {
                if let (Some(pid), Some(major), Some(minor)) = (
                    record.ref_at(1),
                    record.param(2).and_then(|p| p.as_real()),
                    record.param(3).and_then(|p| p.as_real()),
                ) {
                    let axis = cache.resolve_axis(store, pid);
                    return SurfaceInfo {
                        kind: SurfaceKind::Torus,
                        axis_origin: axis.map(|a| a.origin),
                        axis_direction: axis.map(|a| a.z),
                        radius: None,
                        semi_angle: None,
                        major_radius: Some(major * cache.length_scale),
                        minor_radius: Some(minor * cache.length_scale),
                        plane_normal: None,
                        plane_point: None,
                    };
                }
            }
            "PLANE" => {
                if let Some(pid) = record.ref_at(1) {
                    let axis = cache.resolve_axis(store, pid);
                    return SurfaceInfo {
                        kind: SurfaceKind::Plane,
                        axis_origin: None,
                        axis_direction: None,
                        radius: None,
                        semi_angle: None,
                        major_radius: None,
                        minor_radius: None,
                        plane_normal: axis.map(|a| a.z),
                        plane_point: axis.map(|a| a.origin),
                    };
                }
            }
            _ => {}
        }
    }
    SurfaceInfo {
        kind: SurfaceKind::Unknown,
        axis_origin: None,
        axis_direction: None,
        radius: None,
        semi_angle: None,
        major_radius: None,
        minor_radius: None,
        plane_normal: None,
        plane_point: None,
    }
}

fn compute_adjacency(
    faces: &[Face],
    store: &RecordStore,
    cache: &SchemaCache,
) -> Vec<FaceAdjacency> {
    let mut edge_to_faces: std::collections::HashMap<u32, Vec<u32>> =
        std::collections::HashMap::new();

    for face in faces {
        for &loop_id in &face.bound_loop_ids {
            if let Some(edge_ids) = edge_ids_for_loop(loop_id, store, cache) {
                for eid in edge_ids {
                    edge_to_faces.entry(eid).or_default().push(face.id);
                }
            }
        }
    }

    let mut pairs = std::collections::HashSet::new();
    let mut adjacency = Vec::new();

    for (_edge, face_ids) in edge_to_faces {
        if face_ids.len() < 2 {
            continue;
        }
        for i in 0..face_ids.len() {
            for j in (i + 1)..face_ids.len() {
                let a = face_ids[i].min(face_ids[j]);
                let b = face_ids[i].max(face_ids[j]);
                if pairs.insert((a, b)) {
                    adjacency.push(FaceAdjacency {
                        face_a: a,
                        face_b: b,
                        shared_edge_loop: None,
                    });
                }
            }
        }
    }

    adjacency
}

fn edge_ids_for_loop(
    loop_id: u32,
    store: &RecordStore,
    _cache: &SchemaCache,
) -> Option<Vec<u32>> {
    let record = store.get(loop_id)?;
    match record.name.as_str() {
        "EDGE_LOOP" => {
            let mut edges = Vec::new();
            if let Some(list) = record.param(1).and_then(|p| p.as_list()) {
                for item in list {
                    if let Some(oe_id) = item.as_ref_id() {
                        if let Some(oe) = store.get(oe_id) {
                            if let Some(eid) = oe.ref_at(3).or_else(|| oe.ref_at(1)) {
                                edges.push(eid);
                            }
                        }
                    }
                }
            }
            Some(edges)
        }
        _ => None,
    }
}
