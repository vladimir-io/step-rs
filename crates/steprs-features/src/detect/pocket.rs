use crate::geom::{bbox_uv, loop_area_uv, rect_corners_uv, PlaneFrame};
use crate::model::{ManufacturingFeature, PocketProfile};
use smallvec::smallvec;
use steprs_core::ManufacturingFeatureKind;
use steprs_core::RecordStore;
use steprs_schema::SchemaCache;
use steprs_schema::Vec3;
use steprs_topology::{BRepModel, Face, SurfaceKind};

const MIN_POCKET_AREA_MM2: f64 = 20.0;
const MIN_LOOP_POINTS: usize = 3;

pub fn detect_planar_pockets(
    brep: &BRepModel,
    store: &RecordStore,
    cache: &SchemaCache,
) -> Vec<ManufacturingFeature> {
    let mut features = Vec::new();
    let mut seen_loops = std::collections::HashSet::new();

    for face in &brep.faces {
        if face.surface_kind != SurfaceKind::Plane {
            continue;
        }
        let (normal, origin) = match (face.plane_normal, face.plane_point) {
            (Some(n), Some(o)) => (n.normalize(), o),
            _ => continue,
        };

        let frame = plane_frame_for_face(face, cache, store, origin, normal);

        let inner_loops: Vec<u32> = face
            .bound_loops
            .iter()
            .filter(|l| !l.is_outer)
            .map(|l| l.loop_id)
            .collect();

        let candidate_loops: Vec<u32> = if inner_loops.len() >= 1 {
            inner_loops
        } else if is_recess_floor(face, brep) {
            face.bound_loop_ids.to_vec()
        } else if face.bound_loops.len() >= 2 {
            face.bound_loops
                .iter()
                .filter(|l| !l.is_outer)
                .map(|l| l.loop_id)
                .collect()
        } else {
            continue;
        };

        for loop_id in candidate_loops {
            if !seen_loops.insert(loop_id) {
                continue;
            }
            let Some(raw) = walk_loop(loop_id, store, cache) else {
                continue;
            };
            let area = loop_area_uv(&raw, &frame).abs();
            if area < MIN_POCKET_AREA_MM2 {
                continue;
            }
            let corners = orient_corners(&raw, &frame);
            let (min_u, max_u, min_v, max_v) = bbox_uv(&corners, &frame);
            let width = (max_u - min_u).abs();
            let length = (max_v - min_v).abs();
            let (w, l) = (width.min(length), width.max(length));

            let depth = depth_from_adjacency(brep, face, origin, normal)
                .or_else(|| estimate_depth(brep, face));

            let is_slot = l > w * 2.5 && w < 8.0;
            let (kind, label, confidence) = if is_slot {
                (
                    ManufacturingFeatureKind::Slot,
                    format!("Slot {:.1}×{:.1} mm", l, w),
                    0.88,
                )
            } else {
                (
                    ManufacturingFeatureKind::RectangularPocket,
                    format!(
                        "Pocket {:.1}×{:.1}×{:.1} mm",
                        l,
                        w,
                        depth.unwrap_or(5.0)
                    ),
                    0.9,
                )
            };

            let floor_z = origin.z;
            features.push(ManufacturingFeature {
                kind,
                label,
                face_ids: smallvec![face.id],
                axis_origin: Some(origin),
                axis_direction: Some(normal),
                radius: None,
                depth,
                width: Some(w),
                length: Some(l),
                pocket: Some(PocketProfile {
                    corners,
                    u_axis: [frame.u.x, frame.u.y, frame.u.z],
                    v_axis: [frame.v.x, frame.v.y, frame.v.z],
                    origin: [origin.x, origin.y, origin.z],
                    floor_z,
                    normal: [normal.x, normal.y, normal.z],
                }),
                confidence,
            });
        }
    }

    dedupe_pockets(features)
}

fn dedupe_pockets(features: Vec<ManufacturingFeature>) -> Vec<ManufacturingFeature> {
    let mut out: Vec<ManufacturingFeature> = Vec::new();
    for f in features {
        let Some(fp) = f.pocket.as_ref() else {
            out.push(f);
            continue;
        };
        if let Some(existing) = out.iter_mut().find(|e| pockets_overlap(e, &f)) {
            let keep_new = fp.floor_z > existing.pocket.as_ref().map(|p| p.floor_z).unwrap_or(0.0);
            if keep_new {
                *existing = f;
            }
            continue;
        }
        out.push(f);
    }
    out
}

fn pockets_overlap(a: &ManufacturingFeature, b: &ManufacturingFeature) -> bool {
    let (Some(wa), Some(la)) = (a.width, a.length) else {
        return false;
    };
    let (Some(wb), Some(lb)) = (b.width, b.length) else {
        return false;
    };
    (wa - wb).abs() < 1.5 && (la - lb).abs() < 1.5
}

fn plane_frame_for_face(
    face: &Face,
    cache: &SchemaCache,
    store: &RecordStore,
    origin: Vec3,
    normal: Vec3,
) -> PlaneFrame {
    let ref_dir = cache
        .planes
        .iter()
        .find(|p| p.id == face.surface_id)
        .and_then(|pl| cache.resolve_axis(store, pl.placement_id))
        .map(|a| a.x);
    PlaneFrame::from_face(origin, normal, ref_dir)
}

/// Pocket floor: face normal points into the void (inverted from underlying plane).
fn is_recess_floor(face: &Face, brep: &BRepModel) -> bool {
    if !face.same_sense {
        return true;
    }
    wall_neighbor_count(face, brep) >= 3
}

fn wall_neighbor_count(face: &Face, brep: &BRepModel) -> usize {
    let mut count = 0;
    for adj in &brep.adjacency {
        let other_id = if adj.face_a == face.id {
            adj.face_b
        } else if adj.face_b == face.id {
            adj.face_a
        } else {
            continue;
        };
        let Some(other) = brep.faces.iter().find(|f| f.id == other_id) else {
            continue;
        };
        if other.surface_kind == SurfaceKind::Plane {
            if let (Some(n1), Some(n2)) = (face.plane_normal, other.plane_normal) {
                if n1.dot(n2).abs() < 0.95 {
                    count += 1;
                }
            }
        } else {
            count += 1;
        }
    }
    count
}

fn orient_corners(points: &[[f64; 3]], frame: &PlaneFrame) -> Vec<[f64; 3]> {
    if points.len() <= 4 {
        return points.to_vec();
    }
    let (min_u, max_u, min_v, max_v) = bbox_uv(points, frame);
    rect_corners_uv(frame, min_u, max_u, min_v, max_v)
}

fn walk_loop(
    loop_id: u32,
    store: &RecordStore,
    cache: &SchemaCache,
) -> Option<Vec<[f64; 3]>> {
    let record = store.get(loop_id)?;
    let list = record.param(1)?.as_list()?;
    let mut out = Vec::new();
    for item in list {
        let oe_id = item.as_ref_id()?;
        let oe = store.get(oe_id)?;
        let edge_id = oe.ref_at(3).or_else(|| oe.ref_at(1))?;
        let forward = oe
            .param(4)
            .or_else(|| oe.param(3))
            .and_then(|p| p.as_enum_bool())
            .unwrap_or(true);
        if let Some((mut pts, _closed)) = edge_polyline(edge_id, store, cache) {
            if !forward {
                pts.reverse();
            }
            append_dedupe(&mut out, pts);
        }
    }
    if out.len() >= MIN_LOOP_POINTS {
        Some(out)
    } else {
        None
    }
}

fn append_dedupe(out: &mut Vec<[f64; 3]>, pts: Vec<[f64; 3]>) {
    for p in pts {
        if let Some(last) = out.last() {
            if (last[0] - p[0]).hypot(last[1] - p[1]) < 0.01
                && (last[2] - p[2]).abs() < 0.01
            {
                continue;
            }
        }
        out.push(p);
    }
}

fn edge_polyline(
    edge_id: u32,
    store: &RecordStore,
    cache: &SchemaCache,
) -> Option<(Vec<[f64; 3]>, bool)> {
    let edge = store.get(edge_id)?;
    let v1 = edge.ref_at(1)?;
    let v2 = edge.ref_at(2)?;
    let p1 = vertex_coords(v1, store, cache)?;
    let p2 = vertex_coords(v2, store, cache)?;
    let mut pts = vec![p1];
    if (p1[0] - p2[0]).hypot(p1[1] - p2[1]) + (p1[2] - p2[2]).abs() > 0.01 {
        pts.push(p2);
    } else if let Some(curve_id) = edge.ref_at(3) {
        if let Some(end) = line_curve_endpoint(curve_id, p1, store, cache) {
            pts.push(end);
        } else {
            pts.push(p2);
        }
    } else {
        pts.push(p2);
    }
    Some((pts, true))
}

fn vertex_coords(
    vertex_id: u32,
    store: &RecordStore,
    cache: &SchemaCache,
) -> Option<[f64; 3]> {
    let v = store.get(vertex_id)?;
    let pid = v.ref_at(1)?;
    let p = cache.resolve_point(store, pid)?;
    Some([p.x, p.y, p.z])
}

fn line_curve_endpoint(
    curve_id: u32,
    start: [f64; 3],
    store: &RecordStore,
    cache: &SchemaCache,
) -> Option<[f64; 3]> {
    let curve = store.get(curve_id)?;
    if curve.name != "LINE" {
        return None;
    }
    let vector_id = curve.ref_at(2)?;
    let vec_rec = store.get(vector_id)?;
    let dir_id = vec_rec.ref_at(1)?;
    let mag = vec_rec.param(2)?.as_real()?;
    let dir = cache.resolve_direction(store, dir_id)?;
    Some([
        start[0] + dir.x * mag,
        start[1] + dir.y * mag,
        start[2] + dir.z * mag,
    ])
}

fn depth_from_adjacency(
    brep: &BRepModel,
    face: &Face,
    origin: Vec3,
    normal: Vec3,
) -> Option<f64> {
    let mut best = 0.0f64;
    for adj in &brep.adjacency {
        let other_id = if adj.face_a == face.id {
            adj.face_b
        } else if adj.face_b == face.id {
            adj.face_a
        } else {
            continue;
        };
        let other = brep.faces.iter().find(|f| f.id == other_id)?;
        let p = other.plane_point?;
        let d = (p.x - origin.x) * normal.x
            + (p.y - origin.y) * normal.y
            + (p.z - origin.z) * normal.z;
        if d > 0.05 {
            best = best.max(d);
        }
    }
    if best > 0.05 {
        Some(best)
    } else {
        None
    }
}

fn estimate_depth(brep: &BRepModel, face: &Face) -> Option<f64> {
    let normal = face.plane_normal?;
    let origin = face.plane_point?;
    let mut min_dist = f64::MAX;
    for other in &brep.faces {
        if other.id == face.id || other.plane_point.is_none() {
            continue;
        }
        if other.surface_kind != SurfaceKind::Plane {
            continue;
        }
        let p = other.plane_point?;
        let d = (p.x - origin.x) * normal.x
            + (p.y - origin.y) * normal.y
            + (p.z - origin.z) * normal.z;
        if d.abs() > 1e-6 && d.abs() < min_dist {
            min_dist = d.abs();
        }
    }
    if min_dist < f64::MAX {
        Some(min_dist)
    } else {
        None
    }
}
