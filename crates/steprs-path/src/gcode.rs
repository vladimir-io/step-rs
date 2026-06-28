use serde::{Deserialize, Serialize};
use steprs_features::FeatureModel;

use crate::post::{format_job, PostOptions};
use crate::toolpath::{plan_toolpath, ToolpathProgram};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachiningJob {
    pub program_name: String,
    pub units: Units,
    pub blocks: Vec<GCodeBlock>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    Mm,
    Inch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GCodeBlock {
    pub comment: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub diameter_mm: f64,
    pub feed_mm_min: f64,
    pub spindle_rpm: u32,
    pub safe_z_mm: f64,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            diameter_mm: 6.0,
            feed_mm_min: 800.0,
            spindle_rpm: 12_000,
            safe_z_mm: 5.0,
        }
    }
}

pub fn emit_gcode(model: &FeatureModel, tool: &ToolConfig) -> MachiningJob {
    emit_gcode_with_post(model, tool, &PostOptions::default())
}

pub fn emit_gcode_with_post(
    model: &FeatureModel,
    tool: &ToolConfig,
    post: &PostOptions,
) -> MachiningJob {
    let toolpath = plan_toolpath(model, tool);
    emit_gcode_from_toolpath(&toolpath, tool, post)
}

pub fn emit_gcode_from_toolpath(
    toolpath: &ToolpathProgram,
    tool: &ToolConfig,
    post: &PostOptions,
) -> MachiningJob {
    format_job(toolpath, tool, post)
}

pub fn job_to_string(job: &MachiningJob) -> String {
    let mut out = String::new();
    for block in &job.blocks {
        out.push_str(&format!("; {}\n", block.comment));
        for line in &block.lines {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}
