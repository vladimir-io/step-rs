use crate::gcode::MachiningJob;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GCodeValidation {
    pub valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub fn validate_job(job: &MachiningJob) -> GCodeValidation {
    let mut v = GCodeValidation {
        valid: true,
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    if job.blocks.is_empty() {
        v.errors.push("program has no blocks".into());
    }

    let mut motion_lines = 0usize;
    let mut has_spindle = false;
    let mut has_end = false;

    for block in &job.blocks {
        for line in &block.lines {
            let upper = line.to_ascii_uppercase();
            if upper.starts_with('G') && (upper.contains("G0") || upper.contains("G1") || upper.contains("G2") || upper.contains("G3")) {
                motion_lines += 1;
            }
            if upper.contains("M03") || upper.contains("M3") {
                has_spindle = true;
            }
            if upper.contains("M30") {
                has_end = true;
            }
            if let Some(z) = extract_z(&upper) {
                if z < -500.0 {
                    v.warnings.push(format!("deep Z move ({z:.3}) — verify units"));
                }
            }
            if let Some(f) = extract_f(&upper) {
                if f <= 0.0 {
                    v.errors.push(format!("non-positive feed F{f}"));
                }
            }
        }
    }

    if motion_lines == 0 {
        v.warnings.push("no motion blocks (G0/G1/G2/G3)".into());
    }
    if !has_spindle {
        v.warnings.push("no spindle start (M03)".into());
    }
    if !has_end {
        v.warnings.push("no program end (M30)".into());
    }

    if !v.errors.is_empty() {
        v.valid = false;
    }

    v
}

fn extract_z(line: &str) -> Option<f64> {
    for token in line.split_whitespace() {
        if let Some(rest) = token.strip_prefix('Z') {
            return rest.parse().ok();
        }
    }
    None
}

fn extract_f(line: &str) -> Option<f64> {
    for token in line.split_whitespace() {
        if let Some(rest) = token.strip_prefix('F') {
            return rest.parse().ok();
        }
    }
    None
}
