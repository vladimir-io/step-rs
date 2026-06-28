//! Slim analysis payload for browser / WASM — omits heavy registry blobs.

use serde::Serialize;
use steprs_features::{FeatureModel, ScenePreview};
use steprs_path::{GCodeValidation, StockSimulation, ToolpathProgram};
use steprs_schema::{unit_label, EntityRegistry, TessellationMesh};

use crate::{AnalysisResult, BRepSummary};

#[derive(Debug, Clone, Serialize)]
pub struct WebAnalysisResult {
    pub stats: WebParseStats,
    pub brep: BRepSummary,
    pub features: FeatureModel,
    pub mesh: TessellationMesh,
    pub preview: ScenePreview,
    pub toolpath: Option<ToolpathProgram>,
    pub stock_simulation: Option<StockSimulation>,
    pub gcode: Option<String>,
    pub gcode_validation: Option<GCodeValidation>,
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
            features: r.features.clone(),
            mesh: r.mesh.clone(),
            preview: r.preview.clone(),
            toolpath: r.toolpath.clone(),
            stock_simulation: r.stock_simulation.as_ref().map(downsample_stock),
            gcode: r.gcode.clone(),
            gcode_validation: r.gcode_validation.clone(),
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

/// Cap stock heatmap resolution for WASM transfer (full grid used in sim math).
fn downsample_stock(sim: &StockSimulation) -> StockSimulation {
    const MAX: u32 = 256;
    let nx = sim.grid_nx;
    let ny = sim.grid_ny;
    if nx <= MAX && ny <= MAX {
        return sim.clone();
    }

    let step_x = (nx as f64 / MAX as f64).ceil() as u32;
    let step_y = (ny as f64 / MAX as f64).ceil() as u32;
    let out_nx = (nx + step_x - 1) / step_x;
    let out_ny = (ny + step_y - 1) / step_y;
    let mut out = Vec::with_capacity((out_nx * out_ny) as usize);

    for oy in 0..out_ny {
        for ox in 0..out_nx {
            let ix = (ox * step_x).min(nx - 1);
            let iy = (oy * step_y).min(ny - 1);
            out.push(sim.remaining_height[(iy * nx + ix) as usize]);
        }
    }

    StockSimulation {
        grid_nx: out_nx,
        grid_ny: out_ny,
        bounds_min: sim.bounds_min,
        bounds_max: sim.bounds_max,
        remaining_height: out,
        removed_volume_mm3: sim.removed_volume_mm3,
        collision_cells: sim.collision_cells,
        gouge_cells: sim.gouge_cells,
        overcut_cells: sim.overcut_cells,
    }
}
