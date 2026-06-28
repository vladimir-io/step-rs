use crate::model::ManufacturingFeature;
use smallvec::smallvec;
use steprs_core::ManufacturingFeatureKind;
use steprs_schema::{lines_coaxial, Line};
use steprs_topology::{BRepModel, SurfaceKind};

const ANGLE_TOL: f64 = 2e-3;
const OFFSET_TOL: f64 = 0.1;

#[derive(Clone, Copy)]
struct CylFace {
    id: u32,
    line: Line,
    radius: f64,
    z: f64,
}

pub fn detect_coaxial_holes(brep: &BRepModel) -> Vec<ManufacturingFeature> {
    let cylindrical: Vec<CylFace> = brep
        .faces
        .iter()
        .filter(|f| f.surface_kind == SurfaceKind::Cylinder)
        .filter_map(|f| {
            Some(CylFace {
                id: f.id,
                line: Line {
                    point: f.axis_origin?,
                    direction: f.axis_direction?,
                },
                radius: f.radius?,
                z: f.axis_origin?.z,
            })
        })
        .collect();

    let mut used = vec![false; cylindrical.len()];
    let mut features = Vec::new();

    for i in 0..cylindrical.len() {
        if used[i] {
            continue;
        }
        let mut cluster = vec![i];
        used[i] = true;

        for j in (i + 1)..cylindrical.len() {
            if used[j] {
                continue;
            }
            if are_coaxial_cylinders(cylindrical[i], cylindrical[j]) {
                cluster.push(j);
                used[j] = true;
            }
        }

        if cluster.len() >= 2 {
            features.push(coaxial_cluster_feature(&cylindrical, &cluster));
        }
    }

    features
}

fn are_coaxial_cylinders(a: CylFace, b: CylFace) -> bool {
    if !lines_coaxial(a.line, b.line, ANGLE_TOL, OFFSET_TOL) {
        return false;
    }
    true
}

fn coaxial_cluster_feature(cylindrical: &[CylFace], cluster: &[usize]) -> ManufacturingFeature {
    let mut radii: Vec<f64> = cluster.iter().map(|&i| cylindrical[i].radius).collect();
    radii.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let max_r = radii[0];
    let min_r = radii[radii.len() - 1];

    let z_vals: Vec<f64> = cluster.iter().map(|&i| cylindrical[i].z).collect();
    let z_min = z_vals.iter().copied().fold(f64::INFINITY, f64::min);
    let z_max = z_vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let z_span = (z_max - z_min).abs();

    let (kind, label, confidence) = if (max_r - min_r).abs() > 0.05 {
        (
            ManufacturingFeatureKind::BlindHole,
            format!(
                "Coaxial step bore — {} segments, Ø{:.2} → Ø{:.2}",
                cluster.len(),
                max_r * 2.0,
                min_r * 2.0
            ),
            0.92,
        )
    } else {
        (
            ManufacturingFeatureKind::ThroughHole,
            format!(
                "Through coaxial bore — {} faces, Ø{:.2}",
                cluster.len(),
                max_r * 2.0
            ),
            0.88,
        )
    };

    let face_ids: smallvec::SmallVec<[u32; 8]> =
        cluster.iter().map(|&i| cylindrical[i].id).collect();
    let ref_face = cylindrical[cluster[0]];

    ManufacturingFeature {
        kind,
        label,
        face_ids,
        axis_origin: Some(ref_face.line.point),
        axis_direction: Some(ref_face.line.direction),
        radius: Some(max_r),
        depth: Some(if z_span > 0.05 {
            z_span + max_r
        } else {
            max_r * 2.0
        }),
        width: None,
        length: None,
        pocket: None,
        confidence,
    }
}

pub fn detect_isolated_cylinders(brep: &BRepModel) -> Vec<ManufacturingFeature> {
    brep.faces
        .iter()
        .filter(|f| f.surface_kind == SurfaceKind::Cylinder)
        .filter(|f| f.axis_origin.is_some() && f.radius.is_some())
        .map(|face| ManufacturingFeature {
            kind: ManufacturingFeatureKind::CylindricalFace,
            label: format!("Cylinder Ø{:.2}", face.radius.unwrap() * 2.0),
            face_ids: smallvec![face.id],
            axis_origin: face.axis_origin,
            axis_direction: face.axis_direction,
            radius: face.radius,
            depth: None,
            width: None,
            length: None,
            pocket: None,
            confidence: 0.72,
        })
        .collect()
}
