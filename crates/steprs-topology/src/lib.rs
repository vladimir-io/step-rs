mod builder;
mod model;

pub use builder::build_brep;
pub use model::{BRepModel, Face, FaceAdjacency, Solid, SurfaceKind};
