//! End-to-end STEP analysis pipeline.

mod web;

pub use web::{RegistrySummary, WebAnalysisResult, WebParseStats};

pub use steprs_core as core;
pub use steprs_features as features;
pub use steprs_path as path;
pub use steprs_schema as schema;
pub use steprs_topology as topology;

use serde::{Deserialize, Serialize};
use steprs_core::{
    parse_stats, parse_step_file, parse_step_streaming, ParseProgress, ParseStats, RecordStore,
};
use steprs_features::{build_scene_preview, recognize_features, FeatureModel, ScenePreview};
use steprs_path::{
    emit_gcode_from_toolpath, job_to_string, plan_toolpath_brep, simulate_stock, validate_job,
    GCodeValidation, PostOptions, StockSimulation, ToolConfig, ToolpathProgram,
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
    pub preview: ScenePreview,
    pub mesh: TessellationMesh,
    pub toolpath: Option<ToolpathProgram>,
    pub stock_simulation: Option<StockSimulation>,
    pub gcode: Option<String>,
    pub gcode_validation: Option<GCodeValidation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BRepSummary {
    pub solid_count: usize,
    pub face_count: usize,
    pub adjacency_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PipelineOptions {
    pub emit_toolpath: bool,
    pub emit_gcode: bool,
    pub validate_gcode: bool,
    pub simulate_stock: bool,
    pub stock_grid: u32,
    pub tool: ToolConfig,
    pub post: PostOptions,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            emit_toolpath: true,
            emit_gcode: true,
            validate_gcode: true,
            simulate_stock: true,
            stock_grid: 96,
            tool: ToolConfig::default(),
            post: PostOptions::default(),
        }
    }
}

pub fn analyze_step(
    content: &str,
    options: &PipelineOptions,
) -> Result<AnalysisResult, steprs_core::ParseError> {
    let store = parse_step_file(content)?;
    analyze_from_store(&store, options, None)
}

pub fn analyze_step_with_progress(
    content: &str,
    options: &PipelineOptions,
    progress: Option<&dyn ParseProgress>,
) -> Result<AnalysisResult, steprs_core::ParseError> {
    let store = parse_step_streaming(content, progress)?;
    analyze_from_store(&store, options, progress)
}

pub fn analyze_from_store(
    store: &RecordStore,
    options: &PipelineOptions,
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
    p.on_phase(ParsePhase::Features, 1, Some(1));

    let preview = build_scene_preview(&brep, &features, store, &cache);
    let mesh = preview.mesh.clone();

    p.on_phase(ParsePhase::Toolpath, 0, Some(1));
    let toolpath = if options.emit_toolpath || options.emit_gcode {
        Some(plan_toolpath_brep(&features, Some(&brep), &options.tool))
    } else {
        None
    };
    p.on_phase(ParsePhase::Toolpath, 1, Some(1));

    p.on_phase(ParsePhase::StockSim, 0, Some(1));
    let stock_simulation = if options.simulate_stock {
        toolpath.as_ref().and_then(|tp| {
            stock_bounds(&preview, &mesh, &features).map(|(min, max, floor_z)| {
                simulate_stock(
                    min,
                    max,
                    tp,
                    options.tool.diameter_mm,
                    options.stock_grid,
                    Some(floor_z),
                )
            })
        })
    } else {
        None
    };
    p.on_phase(ParsePhase::StockSim, 1, Some(1));

    p.on_phase(ParsePhase::PostProcess, 0, Some(1));
    let (gcode, gcode_validation) = if options.emit_gcode {
        let tp = toolpath
            .as_ref()
            .expect("toolpath required when emit_gcode");
        let job = emit_gcode_from_toolpath(tp, &options.tool, &options.post);
        let validation = if options.validate_gcode {
            Some(validate_job(&job))
        } else {
            None
        };
        (Some(job_to_string(&job)), validation)
    } else {
        (None, None)
    };
    p.on_phase(ParsePhase::PostProcess, 1, Some(1));

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
        preview,
        mesh,
        toolpath,
        stock_simulation,
        gcode,
        gcode_validation,
    })
}

pub fn analyze_json(
    content: &str,
    options: &PipelineOptions,
) -> Result<String, steprs_core::ParseError> {
    let result = analyze_step(content, options)?;
    serde_json::to_string_pretty(&result).map_err(serde_err)
}

/// Browser-optimized JSON — no full entity registry, downsampled stock grid.
pub fn analyze_web_json(
    content: &str,
    options: &PipelineOptions,
) -> Result<String, steprs_core::ParseError> {
    analyze_web_json_with_progress(content, options, None)
}

/// Browser JSON with optional parse/post progress callbacks.
pub fn analyze_web_json_with_progress(
    content: &str,
    options: &PipelineOptions,
    progress: Option<&dyn ParseProgress>,
) -> Result<String, steprs_core::ParseError> {
    let result = analyze_step_with_progress(content, options, progress)?;
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

/// Stock envelope from mesh AABB, scene bounds, and pocket floors.
fn stock_bounds(
    preview: &ScenePreview,
    mesh: &TessellationMesh,
    features: &FeatureModel,
) -> Option<([f64; 3], [f64; 3], f64)> {
    let mut min = preview.bounds.min;
    let mut max = preview.bounds.max;
    if !min[0].is_finite() {
        if mesh.vertices.is_empty() {
            return preview.stock.as_ref().map(|s| (s.min, s.max, s.min[2]));
        }
        min = [f64::INFINITY; 3];
        max = [f64::NEG_INFINITY; 3];
    }
    for v in &mesh.vertices {
        for i in 0..3 {
            min[i] = min[i].min(v[i] as f64);
            max[i] = max[i].max(v[i] as f64);
        }
    }
    let mut floor_z = min[2];
    for f in &features.features {
        if let Some(p) = &f.pocket {
            floor_z = floor_z.min(p.floor_z);
            for c in &p.corners {
                for i in 0..3 {
                    min[i] = min[i].min(c[i]);
                    max[i] = max[i].max(c[i]);
                }
            }
        }
    }
    if !min[0].is_finite() {
        return None;
    }
    let pad = 2.0;
    Some((
        [min[0] - pad, min[1] - pad, min[2] - pad],
        [max[0] + pad, max[1] + pad, max[2] + pad],
        floor_z,
    ))
}
