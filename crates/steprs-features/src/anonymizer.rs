//! Strip spatial metadata from face-clustering output for context-free export.
//!
//! Removes axis origins, axis directions, face normals, face IDs, pocket corner
//! coordinates, and other holistic geometric relations. Retains only feature
//! classifications, quantities, diameters, segment depths, and volumetric limits.

use crate::model::ManufacturingFeature;
use serde::{Deserialize, Serialize};
use steprs_core::ManufacturingFeatureKind;

const SCHEMA_VERSION: &str = "1";
const PROFILE: &str = "anonymized_structural";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralSummary {
    pub schema_version: String,
    pub profile: String,
    pub unit: String,
    pub quantities: ClassificationQuantities,
    pub features: Vec<AnonymizedFeatureEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassificationQuantities {
    pub coaxial_step_bore: usize,
    pub through_coaxial_bore: usize,
    pub blind_hole: usize,
    pub through_hole: usize,
    pub cylindrical_face: usize,
    pub rectangular_pocket: usize,
    pub slot: usize,
    pub circular_pocket: usize,
    pub total_features: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizedFeatureEntry {
    pub classification: String,
    pub segment_quantity: usize,
    pub diameters_mm: Vec<f64>,
    pub segment_depths_mm: Vec<f64>,
    pub volume_limit_mm3: f64,
}

/// Anonymize coaxial hole clusters from `detect_coaxial_holes`.
pub fn anonymize_coaxial_holes(holes: &[ManufacturingFeature]) -> StructuralSummary {
    anonymize_manufacturing_features(holes)
}

/// Anonymize the combined output of coaxial detection and other face-clustering engines.
pub fn anonymize_manufacturing_features(features: &[ManufacturingFeature]) -> StructuralSummary {
    let mut quantities = ClassificationQuantities::default();
    let mut entries = Vec::with_capacity(features.len());

    for f in features {
        let entry = anonymize_one(f);
        bump_quantities(&mut quantities, &entry.classification);
        entries.push(entry);
    }

    quantities.total_features = entries.len();

    StructuralSummary {
        schema_version: SCHEMA_VERSION.to_string(),
        profile: PROFILE.to_string(),
        unit: "mm".to_string(),
        quantities,
        features: entries,
    }
}

pub fn anonymize_to_json(features: &[ManufacturingFeature]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&anonymize_manufacturing_features(features))
}

fn anonymize_one(f: &ManufacturingFeature) -> AnonymizedFeatureEntry {
    let classification = classify(f);
    let segment_quantity = segment_count(f);
    let diameters_mm = segment_diameters(f);
    let segment_depths_mm = segment_depths(f, segment_quantity);
    let volume_limit_mm3 = volume_limit(f, &diameters_mm, &segment_depths_mm);

    AnonymizedFeatureEntry {
        classification,
        segment_quantity,
        diameters_mm,
        segment_depths_mm,
        volume_limit_mm3,
    }
}

fn classify(f: &ManufacturingFeature) -> String {
    let seg = segment_count(f);
    match f.kind {
        ManufacturingFeatureKind::BlindHole if seg >= 2 => "coaxial_step_bore".into(),
        ManufacturingFeatureKind::ThroughHole if seg >= 2 => "through_coaxial_bore".into(),
        other => kind_label(other).into(),
    }
}

fn kind_label(kind: ManufacturingFeatureKind) -> &'static str {
    match kind {
        ManufacturingFeatureKind::ThroughHole => "through_hole",
        ManufacturingFeatureKind::BlindHole => "blind_hole",
        ManufacturingFeatureKind::Counterbore => "counterbore",
        ManufacturingFeatureKind::Countersink => "countersink",
        ManufacturingFeatureKind::RectangularPocket => "rectangular_pocket",
        ManufacturingFeatureKind::CircularPocket => "circular_pocket",
        ManufacturingFeatureKind::Boss => "boss",
        ManufacturingFeatureKind::PlanarFace => "planar_face",
        ManufacturingFeatureKind::CylindricalFace => "cylindrical_face",
        ManufacturingFeatureKind::Fillet => "fillet",
        ManufacturingFeatureKind::Chamfer => "chamfer",
        ManufacturingFeatureKind::Slot => "slot",
        ManufacturingFeatureKind::Unknown => "unknown",
    }
}

fn segment_count(f: &ManufacturingFeature) -> usize {
    f.segment_diameters_mm
        .as_ref()
        .map(|d| d.len().max(1))
        .unwrap_or_else(|| f.face_ids.len().max(1))
}

fn segment_diameters(f: &ManufacturingFeature) -> Vec<f64> {
    if let Some(d) = &f.segment_diameters_mm {
        return round_vec(d);
    }
    if let Some(r) = f.radius {
        return vec![round_mm(r * 2.0)];
    }
    Vec::new()
}

fn segment_depths(f: &ManufacturingFeature, segment_quantity: usize) -> Vec<f64> {
    if let Some(d) = &f.segment_depths_mm {
        return round_vec(d);
    }
    if let Some(depth) = f.depth {
        if segment_quantity <= 1 {
            return vec![round_mm(depth)];
        }
        let per = depth / segment_quantity as f64;
        return (0..segment_quantity).map(|_| round_mm(per)).collect();
    }
    if f.width.is_some() && f.length.is_some() {
        return vec![round_mm(f.depth.unwrap_or(0.0))];
    }
    Vec::new()
}

fn volume_limit(f: &ManufacturingFeature, diameters: &[f64], depths: &[f64]) -> f64 {
    if !diameters.is_empty() && !depths.is_empty() {
        let n = diameters.len().min(depths.len());
        let cyl: f64 = (0..n)
            .map(|i| {
                let r = diameters[i] / 2.0;
                std::f64::consts::PI * r * r * depths[i]
            })
            .sum();
        return round_mm(cyl);
    }

    if let (Some(w), Some(l), Some(d)) = (f.width, f.length, f.depth) {
        return round_mm(w * l * d);
    }

    if let (Some(r), Some(d)) = (f.radius, f.depth) {
        return round_mm(std::f64::consts::PI * r * r * d);
    }

    0.0
}

fn bump_quantities(q: &mut ClassificationQuantities, classification: &str) {
    match classification {
        "coaxial_step_bore" => q.coaxial_step_bore += 1,
        "through_coaxial_bore" => q.through_coaxial_bore += 1,
        "blind_hole" => q.blind_hole += 1,
        "through_hole" => q.through_hole += 1,
        "cylindrical_face" => q.cylindrical_face += 1,
        "rectangular_pocket" => q.rectangular_pocket += 1,
        "slot" => q.slot += 1,
        "circular_pocket" => q.circular_pocket += 1,
        _ => {}
    }
}

fn round_mm(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn round_vec(v: &[f64]) -> Vec<f64> {
    v.iter().map(|x| round_mm(*x)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallvec::smallvec;
    use steprs_core::ManufacturingFeatureKind;

    fn coaxial_step() -> ManufacturingFeature {
        ManufacturingFeature {
            kind: ManufacturingFeatureKind::BlindHole,
            label: "Coaxial step bore — 2 segments, Ø25.40 → Ø12.70".into(),
            face_ids: smallvec![100, 101],
            axis_origin: Some(steprs_schema::Vec3::new(0.0, 0.0, 0.0)),
            axis_direction: Some(steprs_schema::Vec3::new(0.0, 0.0, 1.0)),
            radius: Some(12.7),
            depth: Some(22.7),
            width: None,
            length: None,
            pocket: None,
            confidence: 0.92,
            segment_diameters_mm: Some(vec![25.4, 12.7]),
            segment_depths_mm: Some(vec![10.0, 12.7]),
        }
    }

    #[test]
    fn strips_spatial_fields_from_summary() {
        let json = anonymize_to_json(&[coaxial_step()]).unwrap();
        assert!(!json.contains("axis_origin"));
        assert!(!json.contains("axis_direction"));
        assert!(!json.contains("face_ids"));
        assert!(!json.contains("normal"));
        assert!(json.contains("coaxial_step_bore"));
        assert!(json.contains("25.4"));
        assert!(json.contains("volume_limit_mm3"));
    }

    #[test]
    fn counts_classifications() {
        let summary = anonymize_coaxial_holes(&[coaxial_step()]);
        assert_eq!(summary.quantities.coaxial_step_bore, 1);
        assert_eq!(summary.features[0].segment_quantity, 2);
        assert_eq!(summary.features[0].diameters_mm, vec![25.4, 12.7]);
    }
}
