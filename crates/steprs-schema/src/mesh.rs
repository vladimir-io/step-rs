use serde::{Deserialize, Serialize};
use steprs_core::{Parameter, Record, RecordStore};

/// Triangle mesh extracted from STEP (faceted / tessellated entities).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TessellationMesh {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

pub fn extract_mesh(store: &RecordStore) -> TessellationMesh {
    extract_mesh_scaled(store, 1.0)
}

pub fn extract_mesh_scaled(store: &RecordStore, scale: f64) -> TessellationMesh {
    let mut mesh = TessellationMesh::default();
    let mut point_coords: std::collections::HashMap<u32, [f32; 3]> =
        std::collections::HashMap::new();

    for (idx, slot) in store.records.iter().enumerate() {
        let Some(record) = slot else { continue };
        if record.name == "CARTESIAN_POINT" {
            if let Some(coords) = parse_point(record) {
                point_coords.insert(
                    idx as u32,
                    [
                        (coords[0] as f64 * scale) as f32,
                        (coords[1] as f64 * scale) as f32,
                        (coords[2] as f64 * scale) as f32,
                    ],
                );
            }
        }
    }

    for (_idx, slot) in store.records.iter().enumerate() {
        let Some(record) = slot else { continue };
        match record.name.as_str() {
            "TRIANGULATED_FACE"
            | "TRIANGULATED_SURFACE"
            | "TESSELLATED_FACE"
            | "TESSELLATED_SHELL" => {
                ingest_triangulated(record, &mut mesh, store, &point_coords);
            }
            "TESSELLATED_SOLID" => {
                if let Some(outer_id) = record.ref_at(1) {
                    ingest_tessellated_solid(outer_id, &mut mesh, store, &point_coords);
                }
            }
            "FACETED_BREP" => {
                if let Some(shell_id) = record.ref_at(1) {
                    triangulate_shell(shell_id, store, &mut mesh, &point_coords);
                }
            }
            _ => {}
        }
    }

    if mesh.vertices.is_empty() {
        mesh_from_point_lists(store, &mut mesh, scale);
    }

    mesh
}

fn parse_point(record: &Record) -> Option<[f32; 3]> {
    let p = record.param(1).or_else(|| record.param(0))?;
    if let Some(coords) = p.as_vec3() {
        return Some([coords[0] as f32, coords[1] as f32, coords[2] as f32]);
    }
    let list = p.as_list()?;
    if list.len() >= 3 {
        Some([
            list[0].as_real()? as f32,
            list[1].as_real()? as f32,
            list[2].as_real()? as f32,
        ])
    } else {
        None
    }
}

fn ingest_tessellated_solid(
    outer_id: u32,
    mesh: &mut TessellationMesh,
    store: &RecordStore,
    points: &std::collections::HashMap<u32, [f32; 3]>,
) {
    let Some(outer) = store.get(outer_id) else {
        return;
    };
    for i in 0..outer.parameters.len() {
        if let Some(shell_id) = outer.parameters[i].as_ref_id() {
            if let Some(shell) = store.get(shell_id) {
                if let Some(faces) = shell.param(1).and_then(|p| p.as_list()) {
                    for face_ref in faces {
                        if let Some(fid) = face_ref.as_ref_id() {
                            if let Some(face_rec) = store.get(fid) {
                                ingest_triangulated(face_rec, mesh, store, points);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn ingest_triangulated(
    record: &Record,
    mesh: &mut TessellationMesh,
    store: &RecordStore,
    points: &std::collections::HashMap<u32, [f32; 3]>,
) {
    for i in 0..record.parameters.len() {
        if let Some(pid) = record.parameters[i].as_ref_id() {
            if let Some(coords) = resolve_coordinates(pid, store, points) {
                push_triangle_fan(mesh, &coords);
            }
        }
    }
}

fn resolve_coordinates(
    id: u32,
    store: &RecordStore,
    points: &std::collections::HashMap<u32, [f32; 3]>,
) -> Option<Vec<[f32; 3]>> {
    if let Some(&p) = points.get(&id) {
        return Some(vec![p]);
    }
    let record = store.get(id)?;
    match record.name.as_str() {
        "CARTESIAN_POINT_LIST" | "COORDINATES_LIST" => coords_from_list(record),
        "CARTESIAN_POINT" => parse_point(record).map(|p| vec![p]),
        _ => None,
    }
}

fn coords_from_list(record: &Record) -> Option<Vec<[f32; 3]>> {
    let list = record.param(1).or_else(|| record.param(0))?;
    match list {
        Parameter::List(items) => {
            let mut out = Vec::new();
            for item in items {
                if let Some(triple) = item.as_list() {
                    if triple.len() >= 3 {
                        out.push([
                            triple[0].as_real()? as f32,
                            triple[1].as_real()? as f32,
                            triple[2].as_real()? as f32,
                        ]);
                    }
                } else if let Some(_r) = item.as_real() {
                    // flat list x,y,z,...
                    continue;
                }
            }
            if out.len() >= 3 {
                Some(out)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn push_triangle_fan(mesh: &mut TessellationMesh, coords: &[[f32; 3]]) {
    if coords.len() < 3 {
        return;
    }
    let base = mesh.vertices.len() as u32;
    for c in coords {
        mesh.vertices.push(*c);
    }
    for i in 1..(coords.len() as u32 - 1) {
        mesh.indices
            .extend_from_slice(&[base, base + i, base + i + 1]);
    }
}

fn triangulate_shell(
    shell_id: u32,
    store: &RecordStore,
    mesh: &mut TessellationMesh,
    points: &std::collections::HashMap<u32, [f32; 3]>,
) {
    let Some(shell) = store.get(shell_id) else {
        return;
    };
    if let Some(faces) = shell.param(1).and_then(|p| p.as_list()) {
        for face_ref in faces {
            if let Some(fid) = face_ref.as_ref_id() {
                if let Some(face) = store.get(fid) {
                    if face.name == "FACE_SURFACE" || face.name == "ADVANCED_FACE" {
                        if let Some(bounds) = face.param(1).and_then(|p| p.as_list()) {
                            for b in bounds {
                                if let Some(bid) = b.as_ref_id() {
                                    extract_poly_loop(bid, store, mesh, points);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn extract_poly_loop(
    bound_id: u32,
    store: &RecordStore,
    mesh: &mut TessellationMesh,
    points: &std::collections::HashMap<u32, [f32; 3]>,
) {
    let Some(bound) = store.get(bound_id) else {
        return;
    };
    let Some(loop_id) = bound.ref_at(1) else {
        return;
    };
    let Some(loop_rec) = store.get(loop_id) else {
        return;
    };
    if loop_rec.name != "EDGE_LOOP" && loop_rec.name != "POLY_LOOP" {
        return;
    }
    let mut coords = Vec::new();
    if let Some(list) = loop_rec.param(1).and_then(|p| p.as_list()) {
        for item in list {
            if let Some(pid) = item.as_ref_id() {
                if let Some(mut c) = resolve_coordinates(pid, store, points) {
                    coords.append(&mut c);
                }
            }
        }
    }
    if coords.len() >= 3 {
        push_triangle_fan(mesh, &coords);
    }
}

fn mesh_from_point_lists(store: &RecordStore, mesh: &mut TessellationMesh, scale: f64) {
    for slot in &store.records {
        let Some(record) = slot else { continue };
        if record.name == "CARTESIAN_POINT_LIST" {
            if let Some(coords) = coords_from_list(record) {
                let scaled: Vec<[f32; 3]> = coords
                    .iter()
                    .map(|c| {
                        [
                            (c[0] as f64 * scale) as f32,
                            (c[1] as f64 * scale) as f32,
                            (c[2] as f64 * scale) as f32,
                        ]
                    })
                    .collect();
                push_triangle_fan(mesh, &scaled);
            }
        }
    }
}
