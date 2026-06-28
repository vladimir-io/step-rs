use serde::{Deserialize, Serialize};

use crate::gcode::{GCodeBlock, MachiningJob, ToolConfig};
use crate::toolpath::ToolpathProgram;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PostProcessor {
    #[default]
    Iso,
    Fanuc,
    Haas,
    Grbl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wcs {
    G54,
    G55,
    G56,
    G57,
    G58,
    G59,
}

impl Wcs {
    pub fn code(self) -> &'static str {
        match self {
            Self::G54 => "G54",
            Self::G55 => "G55",
            Self::G56 => "G56",
            Self::G57 => "G57",
            Self::G58 => "G58",
            Self::G59 => "G59",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostOptions {
    pub processor: PostProcessor,
    pub wcs: Wcs,
    pub program_number: u32,
    pub line_numbers: bool,
    pub tool_number: u32,
}

impl Default for PostOptions {
    fn default() -> Self {
        Self {
            processor: PostProcessor::Fanuc,
            wcs: Wcs::G54,
            program_number: 1000,
            line_numbers: true,
            tool_number: 1,
        }
    }
}

pub fn format_job(
    toolpath: &ToolpathProgram,
    tool: &ToolConfig,
    post: &PostOptions,
) -> MachiningJob {
    let mut lines = header_lines(tool, post);

    let mut n = 10u32;
    for block_line in toolpath_to_lines(toolpath) {
        let formatted = format_line(&block_line, post, n);
        lines.push(formatted);
        if post.line_numbers {
            n += 2;
        }
    }

    lines.extend(footer_lines(post));

    MachiningJob {
        program_name: format!("O{}", post.program_number),
        units: crate::gcode::Units::Mm,
        blocks: vec![GCodeBlock {
            comment: format!("{:?} post", post.processor),
            lines,
        }],
    }
}

fn header_lines(tool: &ToolConfig, post: &PostOptions) -> Vec<String> {
    let mut lines = vec!["%".into()];
    match post.processor {
        PostProcessor::Fanuc | PostProcessor::Haas => {
            lines.push(format!("O{:04} (STEPRS)", post.program_number));
            lines.push("(METRIC)".into());
            lines.push(format!("G90 G21 {}", post.wcs.code()));
            lines.push(format!("T{} M06", post.tool_number));
            lines.push(format!("S{} M03", tool.spindle_rpm));
            lines.push(format!("G43 H{} Z{}", post.tool_number, tool.safe_z_mm));
        }
        PostProcessor::Grbl => {
            lines.push("G21".into());
            lines.push("G90".into());
            lines.push(format!("S{} M03", tool.spindle_rpm));
        }
        PostProcessor::Iso => {
            lines.push("G21 (metric)".into());
            lines.push("G90 (absolute)".into());
            lines.push(format!("S{} M03", tool.spindle_rpm));
        }
    }
    lines
}

fn footer_lines(post: &PostOptions) -> Vec<String> {
    match post.processor {
        PostProcessor::Fanuc | PostProcessor::Haas => {
            vec![
                "G91 G28 Z0".into(),
                "G28 X0 Y0".into(),
                "M05".into(),
                "M30".into(),
                "%".into(),
            ]
        }
        _ => vec!["M05".into(), "M30".into(), "%".into()],
    }
}

fn toolpath_to_lines(toolpath: &ToolpathProgram) -> Vec<String> {
    use crate::toolpath::ToolpathSegment;
    let mut out = Vec::new();
    for seg in &toolpath.segments {
        match seg {
            ToolpathSegment::Rapid { to } => {
                out.push(format!("G00 X{:.3} Y{:.3} Z{:.3}", to[0], to[1], to[2]));
            }
            ToolpathSegment::Linear { to, feed_mm_min } => {
                out.push(format!(
                    "G01 X{:.3} Y{:.3} Z{:.3} F{:.0}",
                    to[0], to[1], to[2], feed_mm_min
                ));
            }
            ToolpathSegment::Arc {
                to,
                center_offset,
                clockwise,
                feed_mm_min,
            } => {
                let g = if *clockwise { "G02" } else { "G03" };
                out.push(format!(
                    "{g} X{:.3} Y{:.3} Z{:.3} I{:.3} J{:.3} F{:.0}",
                    to[0], to[1], to[2], center_offset[0], center_offset[1], feed_mm_min
                ));
            }
            ToolpathSegment::Drill {
                xy,
                z_safe,
                z_bottom,
                feed_mm_min,
            } => {
                out.push(format!(
                    "G81 X{:.3} Y{:.3} Z{:.3} R{:.3} F{:.0}",
                    xy[0], xy[1], z_bottom, z_safe, feed_mm_min
                ));
            }
        }
    }
    out
}

fn format_line(line: &str, post: &PostOptions, n: u32) -> String {
    if post.line_numbers {
        format!("N{n} {line}")
    } else {
        line.to_string()
    }
}
