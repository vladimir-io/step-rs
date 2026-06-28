use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use steprs_schema::Vec3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BRepModel {
    pub solids: Vec<Solid>,
    pub faces: Vec<Face>,
    pub adjacency: Vec<FaceAdjacency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solid {
    pub id: u32,
    pub shell_id: u32,
    pub face_ids: SmallVec<[u32; 8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    Plane,
    Cylinder,
    Cone,
    Torus,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceLoop {
    pub loop_id: u32,
    pub is_outer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Face {
    pub id: u32,
    pub surface_id: u32,
    pub surface_kind: SurfaceKind,
    pub bound_loops: SmallVec<[FaceLoop; 4]>,
    pub bound_loop_ids: SmallVec<[u32; 4]>,
    pub same_sense: bool,
    pub axis_origin: Option<Vec3>,
    pub axis_direction: Option<Vec3>,
    pub radius: Option<f64>,
    pub semi_angle: Option<f64>,
    pub major_radius: Option<f64>,
    pub minor_radius: Option<f64>,
    pub plane_normal: Option<Vec3>,
    pub plane_point: Option<Vec3>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceAdjacency {
    pub face_a: u32,
    pub face_b: u32,
    pub shared_edge_loop: Option<u32>,
}
