//! Slim analysis payload for browser / WASM — coaxial holes and B-rep summary only.

use serde::Serialize;
use steprs_features::{ManufacturingFeature, ScenePreview, StructuralSummary};
use steprs_schema::{unit_label, EntityRegistry, TessellationMesh};

use crate::{AnalysisResult, BRepSummary};

#[derive(Debug, Clone, Serialize)]
pub struct WebAnalysisResult {
    pub stats: WebParseStats,
    pub brep: BRepSummary,
    pub coaxial_holes: Vec<ManufacturingFeature>,
    pub structural_summary: StructuralSummary,
    pub mesh: TessellationMesh,
    pub preview: ScenePreview,
    pub registry_summary: RegistrySummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebParseStats {
    pub record_count: usize,
    pub type_count: usize,
    pub application_protocol: String,
    pub schema: String,
    pub parse_errors: usize,
    pub error_samples: Vec<String>,
    pub top_types: Vec<TypeCount>,
    pub length_unit: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeCount {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegistrySummary {
    pub typed_entity_hits: usize,
    pub unknown_entity_types: usize,
    pub ap242_entity_kinds: usize,
}

impl From<&AnalysisResult> for WebAnalysisResult {
    fn from(r: &AnalysisResult) -> Self {
        Self {
            stats: WebParseStats::from_result(r),
            brep: r.brep.clone(),
            coaxial_holes: r.coaxial_holes.clone(),
            structural_summary: r.structural_summary.clone(),
            mesh: r.mesh.clone(),
            preview: r.preview.clone(),
            registry_summary: RegistrySummary::from(&r.registry),
        }
    }
}

impl WebParseStats {
    fn from_result(r: &AnalysisResult) -> Self {
        let s = &r.stats;
        Self {
            record_count: s.record_count,
            type_count: s.type_count,
            application_protocol: s.application_protocol.clone(),
            schema: s.schema.clone(),
            parse_errors: s.parse_errors,
            error_samples: s.errors.iter().take(6).cloned().collect(),
            top_types: s
                .top_types
                .iter()
                .take(8)
                .map(|t| TypeCount {
                    name: t.name.clone(),
                    count: t.count,
                })
                .collect(),
            length_unit: unit_label(r.length_scale).to_string(),
        }
    }
}

impl RegistrySummary {
    fn from(r: &EntityRegistry) -> Self {
        Self {
            typed_entity_hits: r.typed_coverage,
            unknown_entity_types: r.unknown_top.len(),
            ap242_entity_kinds: r.ap242_entities_seen.len(),
        }
    }
}
