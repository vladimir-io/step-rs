use serde::{Deserialize, Serialize};
use steprs_features::{FeatureModel, ManufacturingFeature};
use steprs_schema::Vec3;
use steprs_topology::{BRepModel, SurfaceKind};

use crate::gcode::ToolConfig;
use crate::pocket_path::{is_pocket_like, plan_pocket_or_slot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolpathProgram {
    pub tool_diameter_mm: f64,
    pub safe_z_mm: f64,
    pub bounds: Bounds3,
    pub segments: Vec<ToolpathSegment>,
    pub stats: ToolpathStats,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Bounds3 {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolpathStats {
    pub segment_count: usize,
    pub rapid_count: usize,
    pub linear_count: usize,
    pub arc_count: usize,
    pub estimated_cut_length_mm: f64,
    pub estimated_rapid_length_mm: f64,
    pub estimated_time_min: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolpathSegment {
    Rapid {
        to: [f64; 3],
    },
    Linear {
        to: [f64; 3],
        feed_mm_min: f64,
    },
    Arc {
        to: [f64; 3],
        center_offset: [f64; 2],
        clockwise: bool,
        feed_mm_min: f64,
    },
    /// Peck/drill cycle (emitted as G81 on Fanuc/Haas).
    Drill {
        xy: [f64; 2],
        z_safe: f64,
        z_bottom: f64,
        feed_mm_min: f64,
    },
}

impl ToolpathProgram {
    pub fn push_rapid(&mut self, to: [f64; 3]) {
        self.segments.push(ToolpathSegment::Rapid { to });
        self.stats.rapid_count += 1;
        self.stats.segment_count += 1;
        self.extend_bounds(to);
    }

    pub fn push_linear(&mut self, to: [f64; 3], feed: f64, cut: bool) {
        if let Some(last) = self.last_position() {
            let len = distance(last, to);
            if cut {
                self.stats.estimated_cut_length_mm += len;
                self.stats.estimated_time_min += len / feed.max(1.0);
            } else {
                self.stats.estimated_rapid_length_mm += len;
            }
        }
        self.segments.push(ToolpathSegment::Linear {
            to,
            feed_mm_min: feed,
        });
        self.stats.linear_count += 1;
        self.stats.segment_count += 1;
        self.extend_bounds(to);
    }

    pub fn push_drill(&mut self, xy: [f64; 2], z_safe: f64, z_bottom: f64, feed: f64) {
        if let Some(last) = self.last_position() {
            let plunge = (last[2] - z_bottom).abs();
            let retract = (z_safe - z_bottom).abs();
            self.stats.estimated_cut_length_mm += plunge;
            self.stats.estimated_rapid_length_mm += retract;
            self.stats.estimated_time_min += plunge / feed.max(1.0);
        }
        self.segments.push(ToolpathSegment::Drill {
            xy,
            z_safe,
            z_bottom,
            feed_mm_min: feed,
        });
        self.stats.segment_count += 1;
        self.extend_bounds([xy[0], xy[1], z_bottom]);
        self.extend_bounds([xy[0], xy[1], z_safe]);
    }

    pub fn push_arc(&mut self, to: [f64; 3], center_offset: [f64; 2], clockwise: bool, feed: f64) {
        if let Some(last) = self.last_position() {
            let arc_len = arc_sweep_length(last, to, center_offset, clockwise);
            self.stats.estimated_cut_length_mm += arc_len;
            self.stats.estimated_time_min += arc_len / feed.max(1.0);
        }
        self.segments.push(ToolpathSegment::Arc {
            to,
            center_offset,
            clockwise,
            feed_mm_min: feed,
        });
        self.stats.arc_count += 1;
        self.stats.segment_count += 1;
        self.extend_bounds(to);
    }

    fn last_position(&self) -> Option<[f64; 3]> {
        for seg in self.segments.iter().rev() {
            return Some(seg.end_point());
        }
        None
    }

    fn extend_bounds(&mut self, _p: [f64; 3]) {}
}

impl ToolpathSegment {
    pub fn end_point(&self) -> [f64; 3] {
        match self {
            Self::Rapid { to } | Self::Linear { to, .. } | Self::Arc { to, .. } => *to,
            Self::Drill { xy, z_safe, .. } => [xy[0], xy[1], *z_safe],
        }
    }
}

fn arc_sweep_length(from: [f64; 3], to: [f64; 3], center_offset: [f64; 2], clockwise: bool) -> f64 {
    let cx = from[0] + center_offset[0];
    let cy = from[1] + center_offset[1];
    let r = (center_offset[0].powi(2) + center_offset[1].powi(2))
        .sqrt()
        .max(1e-6);
    let a0 = (from[1] - cy).atan2(from[0] - cx);
    let a1 = (to[1] - cy).atan2(to[0] - cx);
    let mut sweep = a1 - a0;
    if clockwise {
        if sweep >= 0.0 {
            sweep -= std::f64::consts::TAU;
        }
    } else if sweep <= 0.0 {
        sweep += std::f64::consts::TAU;
    }
    r * sweep.abs()
}

/// Plan toolpath from recognized features (source of truth before G-code).
pub fn plan_toolpath(model: &FeatureModel, tool: &ToolConfig) -> ToolpathProgram {
    plan_toolpath_brep(model, None, tool)
}

/// Plan with B-rep context for multi-segment coaxial bores.
pub fn plan_toolpath_brep(
    model: &FeatureModel,
    brep: Option<&BRepModel>,
    tool: &ToolConfig,
) -> ToolpathProgram {
    let mut program = ToolpathProgram {
        tool_diameter_mm: tool.diameter_mm,
        safe_z_mm: tool.safe_z_mm,
        bounds: Bounds3::default(),
        segments: Vec::new(),
        stats: ToolpathStats::default(),
    };

    for feature in &model.features {
        if let Some(b) = brep {
            if plan_coaxial_step(&mut program, feature, b, tool) {
                continue;
            }
        }
        plan_feature(&mut program, feature, tool);
    }

    if !program.segments.is_empty() {
        recompute_bounds(&mut program);
    }

    program
}

fn plan_coaxial_step(
    program: &mut ToolpathProgram,
    feature: &ManufacturingFeature,
    brep: &BRepModel,
    tool: &ToolConfig,
) -> bool {
    use steprs_core::ManufacturingFeatureKind;
    if feature.face_ids.len() < 2 {
        return false;
    }
    if !matches!(
        feature.kind,
        ManufacturingFeatureKind::BlindHole | ManufacturingFeatureKind::ThroughHole
    ) {
        return false;
    }

    let mut segments: Vec<(f64, f64, f64, f64)> = Vec::new();
    for &fid in &feature.face_ids {
        let face = match brep.faces.iter().find(|f| f.id == fid) {
            Some(f) => f,
            None => return false,
        };
        if face.surface_kind != SurfaceKind::Cylinder {
            return false;
        }
        let (Some(origin), Some(radius)) = (face.axis_origin, face.radius) else {
            return false;
        };
        segments.push((origin.x, origin.y, origin.z, radius));
    }

    segments.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let z_safe_top = segments
        .iter()
        .map(|(_, _, z, r)| z + r + tool.safe_z_mm)
        .fold(f64::NEG_INFINITY, f64::max);

    let (cx, cy) = (segments[0].0, segments[0].1);
    program.push_rapid([cx, cy, z_safe_top]);

    for (i, &(_, _, z_origin, radius)) in segments.iter().enumerate() {
        let plunge = radius * 2.0;
        let z_bottom = if i + 1 < segments.len() {
            segments[i + 1].2
        } else {
            z_origin - plunge
        };
        program.push_drill([cx, cy], z_safe_top, z_bottom, tool.feed_mm_min / 2.0);
    }

    program.push_rapid([cx, cy, z_safe_top]);
    true
}

fn recompute_bounds(program: &mut ToolpathProgram) {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for seg in &program.segments {
        let p = seg.end_point();
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    program.bounds.min = min;
    program.bounds.max = max;
}

fn plan_feature(program: &mut ToolpathProgram, feature: &ManufacturingFeature, tool: &ToolConfig) {
    if is_pocket_like(feature.kind) {
        if plan_pocket_or_slot(program, feature, tool) {
            return;
        }
    }

    let (origin, radius) = match (feature.axis_origin, feature.radius) {
        (Some(o), Some(r)) => (o, r),
        _ => return,
    };

    let depth = feature.depth.unwrap_or(radius * 2.0);
    let cx = origin.x;
    let cy = origin.y;
    let z_safe = origin.z + tool.safe_z_mm;
    let z_cut = origin.z - depth;

    program.push_rapid([cx, cy, z_safe]);
    program.push_drill([cx, cy], z_safe, z_cut, tool.feed_mm_min / 2.0);
    program.push_rapid([cx, cy, z_safe]);
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let dz = b[2] - a[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Parse G-code back into segments for simulation verification.
pub fn toolpath_from_gcode(gcode: &str, safe_z: f64) -> ToolpathProgram {
    let mut program = ToolpathProgram {
        tool_diameter_mm: 6.0,
        safe_z_mm: safe_z,
        bounds: Bounds3::default(),
        segments: Vec::new(),
        stats: ToolpathStats::default(),
    };

    let mut x = 0.0f64;
    let mut y = 0.0f64;
    let mut z = safe_z;
    let mut feed = 800.0;

    for line in gcode.lines() {
        let line = line.split(';').next().unwrap_or("").trim();
        if line.is_empty() || line == "%" {
            continue;
        }
        let upper = line.to_ascii_uppercase();

        if let Some(v) = parse_axis(&upper, 'X') {
            x = v;
        }
        if let Some(v) = parse_axis(&upper, 'Y') {
            y = v;
        }
        if let Some(v) = parse_axis(&upper, 'Z') {
            z = v;
        }
        if let Some(v) = parse_axis(&upper, 'F') {
            feed = v;
        }

        let to = [x, y, z];
        if upper.contains("G00") || upper.contains("G0 ") {
            program.push_rapid(to);
        } else if upper.contains("G01") || upper.contains("G1 ") {
            program.push_linear(to, feed, true);
        } else if upper.contains("G02") || upper.contains("G2 ") {
            let i = parse_axis(&upper, 'I').unwrap_or(0.0);
            let j = parse_axis(&upper, 'J').unwrap_or(0.0);
            program.push_arc(to, [i, j], true, feed);
        } else if upper.contains("G03") || upper.contains("G3 ") {
            let i = parse_axis(&upper, 'I').unwrap_or(0.0);
            let j = parse_axis(&upper, 'J').unwrap_or(0.0);
            program.push_arc(to, [i, j], false, feed);
        } else if upper.contains("G81") {
            let r = parse_axis(&upper, 'R').unwrap_or(safe_z);
            program.push_drill([x, y], r, z, feed);
        }
    }

    recompute_bounds(&mut program);
    program
}

fn parse_axis(line: &str, axis: char) -> Option<f64> {
    for token in line.split_whitespace() {
        if let Some(rest) = token.strip_prefix(axis) {
            return rest.parse().ok();
        }
    }
    None
}

pub fn vec3_to_array(v: Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}
