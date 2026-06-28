use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use steprs_core::ManufacturingFeatureKind;
use steprs_schema::Vec3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureModel {
    pub features: Vec<ManufacturingFeature>,
    pub summary: FeatureSummary,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureSummary {
    pub hole_count: usize,
    pub pocket_count: usize,
    pub slot_count: usize,
    pub boss_count: usize,
    pub cylindrical_face_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManufacturingFeature {
    pub kind: ManufacturingFeatureKind,
    pub label: String,
    pub face_ids: SmallVec<[u32; 8]>,
    pub axis_origin: Option<Vec3>,
    pub axis_direction: Option<Vec3>,
    pub radius: Option<f64>,
    pub depth: Option<f64>,
    pub width: Option<f64>,
    pub length: Option<f64>,
    pub pocket: Option<PocketProfile>,
    pub confidence: f32,
}

/// Planar pocket boundary in world coordinates with face-local UV axes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PocketProfile {
    pub corners: Vec<[f64; 3]>,
    /// Face origin (floor point).
    #[serde(default = "default_origin")]
    pub origin: [f64; 3],
    #[serde(default = "default_u_axis")]
    pub u_axis: [f64; 3],
    #[serde(default = "default_v_axis")]
    pub v_axis: [f64; 3],
    pub floor_z: f64,
    pub normal: [f64; 3],
}

fn default_origin() -> [f64; 3] {
    [0.0, 0.0, 0.0]
}

fn default_u_axis() -> [f64; 3] {
    [1.0, 0.0, 0.0]
}

fn default_v_axis() -> [f64; 3] {
    [0.0, 1.0, 0.0]
}

impl PocketProfile {
    pub fn u_len(&self) -> f64 {
        (self.u_axis[0].powi(2) + self.u_axis[1].powi(2) + self.u_axis[2].powi(2)).sqrt()
    }

    pub fn uv_bbox(&self) -> (f64, f64, f64, f64) {
        let mut min_u = f64::INFINITY;
        let mut max_u = f64::NEG_INFINITY;
        let mut min_v = f64::INFINITY;
        let mut max_v = f64::NEG_INFINITY;
        for p in &self.corners {
            let du = p[0] - self.origin[0];
            let dv = p[1] - self.origin[1];
            let dw = p[2] - self.origin[2];
            let u = du * self.u_axis[0] + dv * self.u_axis[1] + dw * self.u_axis[2];
            let v = du * self.v_axis[0] + dv * self.v_axis[1] + dw * self.v_axis[2];
            min_u = min_u.min(u);
            max_u = max_u.max(u);
            min_v = min_v.min(v);
            max_v = max_v.max(v);
        }
        (min_u, max_u, min_v, max_v)
    }

    pub fn to_world(&self, u: f64, v: f64, z: f64) -> [f64; 3] {
        let base = [
            self.origin[0] + self.u_axis[0] * u + self.v_axis[0] * v,
            self.origin[1] + self.u_axis[1] * u + self.v_axis[1] * v,
            self.origin[2] + self.u_axis[2] * u + self.v_axis[2] * v,
        ];
        if self.normal[2].abs() > 0.95 {
            return [base[0], base[1], z];
        }
        let dz = z - self.floor_z;
        [
            base[0] + self.normal[0] * dz,
            base[1] + self.normal[1] * dz,
            base[2] + self.normal[2] * dz,
        ]
    }
}
