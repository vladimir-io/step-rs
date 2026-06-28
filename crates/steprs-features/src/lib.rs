mod anonymizer;
mod detect;
mod geom;
mod model;
pub mod preview;
mod tessellate;

pub use anonymizer::{
    anonymize_coaxial_holes, anonymize_manufacturing_features, anonymize_to_json,
    AnonymizedFeatureEntry, ClassificationQuantities, StructuralSummary,
};
pub use detect::{detect_coaxial_holes, detect_isolated_cylinders, detect_planar_pockets};
pub use model::{FeatureModel, FeatureSummary, ManufacturingFeature, PocketProfile};
pub use preview::{build_scene_preview, ScenePreview};

use steprs_core::RecordStore;
use steprs_schema::SchemaCache;
use steprs_topology::BRepModel;

pub fn recognize_features(
    brep: &BRepModel,
    store: &RecordStore,
    cache: &SchemaCache,
) -> FeatureModel {
    let mut features = Vec::new();
    features.extend(detect_coaxial_holes(brep));

    let coaxial_face_ids: std::collections::HashSet<u32> = features
        .iter()
        .flat_map(|f| f.face_ids.iter().copied())
        .collect();

    for f in detect_isolated_cylinders(brep) {
        if f.face_ids.iter().all(|id| !coaxial_face_ids.contains(id)) {
            features.push(f);
        }
    }

    features.extend(detect_planar_pockets(brep, store, cache));

    let summary = FeatureSummary {
        hole_count: features
            .iter()
            .filter(|f| {
                matches!(
                    f.kind,
                    steprs_core::ManufacturingFeatureKind::ThroughHole
                        | steprs_core::ManufacturingFeatureKind::BlindHole
                )
            })
            .count(),
        pocket_count: features
            .iter()
            .filter(|f| f.kind == steprs_core::ManufacturingFeatureKind::RectangularPocket)
            .count(),
        slot_count: features
            .iter()
            .filter(|f| f.kind == steprs_core::ManufacturingFeatureKind::Slot)
            .count(),
        boss_count: features
            .iter()
            .filter(|f| f.kind == steprs_core::ManufacturingFeatureKind::Boss)
            .count(),
        cylindrical_face_count: features
            .iter()
            .filter(|f| f.kind == steprs_core::ManufacturingFeatureKind::CylindricalFace)
            .count(),
    };

    FeatureModel { features, summary }
}
