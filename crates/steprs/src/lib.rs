//! End-to-end STEP analysis pipeline — B-rep topology and coaxial hole detection.

mod regression;
mod web;

pub use regression::{
    ap214_regression_specs, cylinder_block_spec, run_ap214_regression,
    run_cylinder_block_regression, run_system_tests, verify_sample, SampleExpect, SystemTestStatus,
    VerifyResult,
};
pub use web::{RegistrySummary, WebAnalysisResult, WebParseStats};

pub use steprs_core as core;
pub use steprs_features as features;
pub use steprs_schema as schema;
pub use steprs_topology as topology;

use serde::{Deserialize, Serialize};
use steprs_core::{
    parse_stats, parse_step_file, parse_step_streaming, ParseProgress, ParseStats, RecordStore,
};
use steprs_features::{
    anonymize_coaxial_holes, build_scene_preview, detect_coaxial_holes, recognize_features,
    FeatureModel, ManufacturingFeature, ScenePreview, StructuralSummary,
};
use steprs_schema::{build_registry, EntityRegistry, TessellationMesh};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub stats: ParseStats,
    /// Raw STEP length × this = millimetres.
    pub length_scale: f64,
    pub registry: EntityRegistry,
    pub brep: BRepSummary,
    pub features: FeatureModel,
    /// Output of `detect_coaxial_holes` only — face-clustered step bores and through bores.
    pub coaxial_holes: Vec<ManufacturingFeature>,
    /// Context-free structural summary (no spatial metadata).
    pub structural_summary: StructuralSummary,
    pub preview: ScenePreview,
    pub mesh: TessellationMesh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BRepSummary {
    pub solid_count: usize,
    pub face_count: usize,
    pub adjacency_count: usize,
}

pub fn analyze_step(content: &str) -> Result<AnalysisResult, steprs_core::ParseError> {
    let store = parse_step_file(content)?;
    analyze_from_store(&store, None)
}

pub fn analyze_step_with_progress(
    content: &str,
    progress: Option<&dyn ParseProgress>,
) -> Result<AnalysisResult, steprs_core::ParseError> {
    let store = parse_step_streaming(content, progress)?;
    analyze_from_store(&store, progress)
}

pub fn analyze_from_store(
    store: &RecordStore,
    progress: Option<&dyn ParseProgress>,
) -> Result<AnalysisResult, steprs_core::ParseError> {
    use steprs_core::{NoopProgress, ParsePhase};
    let p = progress.unwrap_or(&NoopProgress);

    let stats = parse_stats(store);
    let registry = build_registry(&store.by_type);

    p.on_phase(ParsePhase::Topology, 0, Some(1));
    let cache = steprs_schema::SchemaCache::build(store);
    let brep = topology::build_brep(store, &cache);
    p.on_phase(ParsePhase::Topology, 1, Some(1));

    p.on_phase(ParsePhase::Features, 0, Some(1));
    let features = recognize_features(&brep, store, &cache);
    let coaxial_holes = detect_coaxial_holes(&brep);
    let structural_summary = anonymize_coaxial_holes(&coaxial_holes);
    p.on_phase(ParsePhase::Features, 1, Some(1));

    let preview = build_scene_preview(&brep, &features, store, &cache);
    let mesh = preview.mesh.clone();

    Ok(AnalysisResult {
        stats,
        length_scale: cache.length_scale,
        registry,
        brep: BRepSummary {
            solid_count: brep.solids.len(),
            face_count: brep.faces.len(),
            adjacency_count: brep.adjacency.len(),
        },
        features,
        coaxial_holes,
        structural_summary,
        preview,
        mesh,
    })
}

pub fn analyze_json(content: &str) -> Result<String, steprs_core::ParseError> {
    let result = analyze_step(content)?;
    serde_json::to_string_pretty(&result).map_err(serde_err)
}

/// Browser-optimized JSON — no full entity registry.
pub fn analyze_web_json(content: &str) -> Result<String, steprs_core::ParseError> {
    analyze_web_json_with_progress(content, None)
}

/// Browser JSON with optional parse progress callbacks.
pub fn analyze_web_json_with_progress(
    content: &str,
    progress: Option<&dyn ParseProgress>,
) -> Result<String, steprs_core::ParseError> {
    let result = analyze_step_with_progress(content, progress)?;
    let web = WebAnalysisResult::from(&result);
    serde_json::to_string(&web).map_err(serde_err)
}

fn serde_err(e: serde_json::Error) -> steprs_core::ParseError {
    steprs_core::ParseError::new(steprs_core::ParseErrorKind::UnexpectedToken { offset: 0 })
        .with_context(e.to_string())
}

pub fn parse_only(content: &str) -> Result<ParseStats, steprs_core::ParseError> {
    let store = parse_step_file(content)?;
    Ok(parse_stats(&store))
}

/// JSON array of AP214 regression `SampleExpect` specs for browser verification.
pub fn regression_specs_json() -> Result<String, serde_json::Error> {
    regression::ap214_specs_json()
}
