//! Known EXPRESS entities for typed schema extraction.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Known geometry/topology entities we extract into typed caches.
pub const TYPED_ENTITIES: &[&str] = &[
    "CARTESIAN_POINT",
    "DIRECTION",
    "VECTOR",
    "AXIS2_PLACEMENT_3D",
    "AXIS2_PLACEMENT_2D",
    "CYLINDRICAL_SURFACE",
    "CONICAL_SURFACE",
    "TOROIDAL_SURFACE",
    "PLANE",
    "LINE",
    "CIRCLE",
    "ELLIPSE",
    "ADVANCED_FACE",
    "FACE_SURFACE",
    "FACE_BOUND",
    "FACE_OUTER_BOUND",
    "EDGE_LOOP",
    "ORIENTED_EDGE",
    "EDGE_CURVE",
    "VERTEX_POINT",
    "MANIFOLD_SOLID_BREP",
    "BREP_WITH_VOIDS",
    "CLOSED_SHELL",
    "OPEN_SHELL",
    "ADVANCED_BREP_SHAPE_REPRESENTATION",
    "FACETED_BREP",
    "FACETED_BREP_SHAPE_REPRESENTATION",
    "TRIANGULATED_FACE",
    "TRIANGULATED_SURFACE",
    "TRIANGULATED_SURFACE_SET",
    "CARTESIAN_POINT_LIST",
    "POLY_LOOP",
    "POLYLINE",
];

/// AP242 / automotive extensions commonly seen in NX/CATIA exports.
pub const AP242_ENTITIES: &[&str] = &[
    "TESSELLATED_SOLID",
    "TESSELLATED_SHELL",
    "TESSELLATED_FACE",
    "GEOMETRICALLY_BOUNDED_SURFACE_SHAPE_REPRESENTATION",
    "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRegistry {
    pub typed_coverage: usize,
    pub ap242_entities_seen: Vec<String>,
    pub unknown_top: Vec<(String, usize)>,
}

pub fn build_registry(by_type: &std::collections::HashMap<String, Vec<u32>>) -> EntityRegistry {
    let typed: HashSet<&str> = TYPED_ENTITIES.iter().copied().collect();
    let ap242: HashSet<&str> = AP242_ENTITIES.iter().copied().collect();

    let mut ap242_seen = Vec::new();
    let mut unknown: Vec<(String, usize)> = Vec::new();
    let mut typed_hits = 0usize;

    for (name, ids) in by_type {
        let count = ids.len();
        if typed.contains(name.as_str()) {
            typed_hits += count;
        } else if ap242.contains(name.as_str()) {
            ap242_seen.push(name.clone());
        } else if !is_header_or_meta(name) {
            unknown.push((name.clone(), count));
        }
    }

    unknown.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    unknown.truncate(16);
    ap242_seen.sort();

    EntityRegistry {
        typed_coverage: typed_hits,
        ap242_entities_seen: ap242_seen,
        unknown_top: unknown,
    }
}

fn is_header_or_meta(name: &str) -> bool {
    matches!(
        name,
        "PRODUCT"
            | "PRODUCT_DEFINITION"
            | "PRODUCT_DEFINITION_FORMATION"
            | "PRODUCT_DEFINITION_SHAPE"
            | "PRODUCT_RELATED_PRODUCT_CATEGORY"
            | "APPLICATION_PROTOCOL_DEFINITION"
            | "SHAPE_DEFINITION_REPRESENTATION"
            | "SHAPE_REPRESENTATION"
            | "SHAPE_REPRESENTATION_RELATIONSHIP"
            | "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION"
            | "NEXT_ASSEMBLY_USAGE_OCCURRENCE"
            | "PROPERTY_DEFINITION"
            | "PROPERTY_DEFINITION_REPRESENTATION"
            | "MEASURE_REPRESENTATION_ITEM"
            | "LENGTH_MEASURE_WITH_UNIT"
            | "PLANE_ANGLE_MEASURE_WITH_UNIT"
            | "UNCERTAINTY_MEASURE_WITH_UNIT"
            | "SI_UNIT"
            | "NAMED_UNIT"
            | "DIMENSIONAL_EXPONENTS"
            | "MECHANICAL_DESIGN_GEOMETRIC_PRESENTATION_REPRESENTATION"
    )
}
