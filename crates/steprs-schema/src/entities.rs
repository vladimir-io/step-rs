use crate::math::{Axis3, Vec3};
use crate::units::detect_length_scale_to_mm;
use steprs_core::{Parameter, Record, RecordStore};

#[derive(Debug, Clone, PartialEq)]
pub struct CartesianPoint {
    pub id: u32,
    pub coordinates: Vec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Direction {
    pub id: u32,
    pub direction: Vec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Axis2Placement3d {
    pub id: u32,
    pub location_id: u32,
    pub axis_id: Option<u32>,
    pub ref_direction_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CylindricalSurface {
    pub id: u32,
    pub placement_id: u32,
    pub radius: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plane {
    pub id: u32,
    pub placement_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConicalSurface {
    pub id: u32,
    pub placement_id: u32,
    pub radius: f64,
    pub semi_angle: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToroidalSurface {
    pub id: u32,
    pub placement_id: u32,
    pub major_radius: f64,
    pub minor_radius: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdvancedFace {
    pub id: u32,
    pub bound_ids: Vec<u32>,
    pub surface_id: u32,
    pub same_sense: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FaceBound {
    pub id: u32,
    pub loop_id: u32,
    pub orientation: bool,
    /// `true` for `FACE_OUTER_BOUND`, `false` for hole `FACE_BOUND`.
    pub is_outer: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EdgeLoop {
    pub id: u32,
    pub edge_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrientedEdge {
    pub id: u32,
    pub edge_id: u32,
    pub orientation: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifoldSolidBrep {
    pub id: u32,
    pub shell_id: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClosedShell {
    pub id: u32,
    pub face_ids: Vec<u32>,
}

pub struct SchemaCache {
    /// Multiply raw STEP lengths by this to get millimetres.
    pub length_scale: f64,
    pub points: Vec<CartesianPoint>,
    pub directions: Vec<Direction>,
    pub placements: Vec<Axis2Placement3d>,
    pub cylinders: Vec<CylindricalSurface>,
    pub cones: Vec<ConicalSurface>,
    pub tori: Vec<ToroidalSurface>,
    pub planes: Vec<Plane>,
    pub faces: Vec<AdvancedFace>,
    pub face_bounds: Vec<FaceBound>,
    pub solids: Vec<ManifoldSolidBrep>,
    pub shells: Vec<ClosedShell>,
}

impl SchemaCache {
    pub fn build(store: &RecordStore) -> Self {
        let length_scale = detect_length_scale_to_mm(store);
        let mut cache = Self {
            length_scale,
            points: Vec::new(),
            directions: Vec::new(),
            placements: Vec::new(),
            cylinders: Vec::new(),
            cones: Vec::new(),
            tori: Vec::new(),
            planes: Vec::new(),
            faces: Vec::new(),
            face_bounds: Vec::new(),
            solids: Vec::new(),
            shells: Vec::new(),
        };

        for (idx, slot) in store.records.iter().enumerate() {
            let Some(record) = slot else { continue };
            let id = idx as u32;
            match record.name.as_str() {
                "CARTESIAN_POINT" => {
                    if let Some(mut p) = parse_cartesian_point(id, record) {
                        p.coordinates = p.coordinates.scale(length_scale);
                        cache.points.push(p);
                    }
                }
                "DIRECTION" => {
                    if let Some(d) = parse_direction(id, record) {
                        cache.directions.push(d);
                    }
                }
                "AXIS2_PLACEMENT_3D" => {
                    if let Some(a) = parse_axis2_placement(id, record) {
                        cache.placements.push(a);
                    }
                }
                "CYLINDRICAL_SURFACE" => {
                    if let Some(mut c) = parse_cylindrical_surface(id, record) {
                        c.radius *= length_scale;
                        cache.cylinders.push(c);
                    }
                }
                "CONICAL_SURFACE" => {
                    if let Some(mut c) = parse_conical_surface(id, record) {
                        c.radius *= length_scale;
                        cache.cones.push(c);
                    }
                }
                "TOROIDAL_SURFACE" => {
                    if let Some(mut t) = parse_toroidal_surface(id, record) {
                        t.major_radius *= length_scale;
                        t.minor_radius *= length_scale;
                        cache.tori.push(t);
                    }
                }
                "PLANE" => {
                    if let Some(p) = parse_plane(id, record) {
                        cache.planes.push(p);
                    }
                }
                "ADVANCED_FACE" => {
                    if let Some(f) = parse_advanced_face(id, record) {
                        cache.faces.push(f);
                    }
                }
                "FACE_BOUND" | "FACE_OUTER_BOUND" => {
                    if let Some(fb) = parse_face_bound(id, record) {
                        cache.face_bounds.push(fb);
                    }
                }
                "MANIFOLD_SOLID_BREP" => {
                    if let Some(s) = parse_manifold_solid(id, record) {
                        cache.solids.push(s);
                    }
                }
                "CLOSED_SHELL" => {
                    if let Some(sh) = parse_closed_shell(id, record) {
                        cache.shells.push(sh);
                    }
                }
                _ => {}
            }
        }

        cache
    }

    pub fn resolve_axis(&self, store: &RecordStore, placement_id: u32) -> Option<Axis3> {
        let from_store = store
            .get(placement_id)
            .and_then(|r| parse_axis2_placement(placement_id, r));
        let placement = self
            .placements
            .iter()
            .find(|p| p.id == placement_id)
            .or(from_store.as_ref())?;
        let origin = self.resolve_point(store, placement.location_id)?;
        let z = placement
            .axis_id
            .and_then(|id| self.resolve_direction(store, id))
            .unwrap_or(Vec3::new(0.0, 0.0, 1.0));
        let x = placement
            .ref_direction_id
            .and_then(|id| self.resolve_direction(store, id))
            .unwrap_or(Vec3::new(1.0, 0.0, 0.0));
        Some(Axis3 { origin, z, x })
    }

    pub fn resolve_point(&self, store: &RecordStore, id: u32) -> Option<Vec3> {
        if let Some(p) = self.points.iter().find(|p| p.id == id) {
            return Some(p.coordinates);
        }
        let record = store.get(id)?;
        parse_cartesian_point(id, record).map(|p| p.coordinates.scale(self.length_scale))
    }

    pub fn resolve_direction(&self, store: &RecordStore, id: u32) -> Option<Vec3> {
        if let Some(d) = self.directions.iter().find(|d| d.id == id) {
            return Some(d.direction);
        }
        let record = store.get(id)?;
        parse_direction(id, record).map(|d| d.direction)
    }
}

fn parse_cartesian_point(id: u32, record: &Record) -> Option<CartesianPoint> {
    let coords = record
        .param(1)
        .or_else(|| record.param(0))
        .and_then(|p| p.as_vec3())
        .or_else(|| {
            record.param(0).and_then(|p| p.as_list()).and_then(|l| {
                if l.len() >= 3 {
                    Some([l[0].as_real()?, l[1].as_real()?, l[2].as_real()?])
                } else {
                    None
                }
            })
        })?;
    Some(CartesianPoint {
        id,
        coordinates: Vec3::from_array(coords),
    })
}

fn parse_direction(id: u32, record: &Record) -> Option<Direction> {
    let ratios = record.param(1).or_else(|| record.param(0))?;
    let coords = ratios.as_vec3().or_else(|| {
        ratios.as_list().and_then(|l| {
            if l.len() >= 3 {
                Some([l[0].as_real()?, l[1].as_real()?, l[2].as_real()?])
            } else {
                None
            }
        })
    })?;
    let direction = Vec3::from_array(coords);
    Some(Direction { id, direction })
}

fn parse_axis2_placement(id: u32, record: &Record) -> Option<Axis2Placement3d> {
    Some(Axis2Placement3d {
        id,
        location_id: record.ref_at(1)?,
        axis_id: record.ref_at(2),
        ref_direction_id: record.ref_at(3),
    })
}

fn parse_cylindrical_surface(id: u32, record: &Record) -> Option<CylindricalSurface> {
    Some(CylindricalSurface {
        id,
        placement_id: record.ref_at(1)?,
        radius: record.param(2)?.as_real()?,
    })
}

fn parse_plane(id: u32, record: &Record) -> Option<Plane> {
    Some(Plane {
        id,
        placement_id: record.ref_at(1)?,
    })
}

fn parse_conical_surface(id: u32, record: &Record) -> Option<ConicalSurface> {
    Some(ConicalSurface {
        id,
        placement_id: record.ref_at(1)?,
        radius: record.param(2)?.as_real()?,
        semi_angle: record.param(3)?.as_real()?,
    })
}

fn parse_toroidal_surface(id: u32, record: &Record) -> Option<ToroidalSurface> {
    Some(ToroidalSurface {
        id,
        placement_id: record.ref_at(1)?,
        major_radius: record.param(2)?.as_real()?,
        minor_radius: record.param(3)?.as_real()?,
    })
}

fn parse_advanced_face(id: u32, record: &Record) -> Option<AdvancedFace> {
    let bounds_param = record.param(1)?;
    let bound_ids = ref_list(bounds_param);
    Some(AdvancedFace {
        id,
        bound_ids,
        surface_id: record.ref_at(2)?,
        same_sense: record.param(3)?.as_enum_bool().unwrap_or(true),
    })
}

fn parse_face_bound(id: u32, record: &Record) -> Option<FaceBound> {
    Some(FaceBound {
        id,
        loop_id: record.ref_at(1)?,
        orientation: record.param(2)?.as_enum_bool().unwrap_or(true),
        is_outer: record.name == "FACE_OUTER_BOUND",
    })
}

fn parse_manifold_solid(id: u32, record: &Record) -> Option<ManifoldSolidBrep> {
    Some(ManifoldSolidBrep {
        id,
        shell_id: record.ref_at(1)?,
    })
}

fn parse_closed_shell(id: u32, record: &Record) -> Option<ClosedShell> {
    let faces = record.param(1)?;
    Some(ClosedShell {
        id,
        face_ids: ref_list(faces),
    })
}

fn ref_list(param: &Parameter) -> Vec<u32> {
    match param {
        Parameter::List(items) => items
            .iter()
            .filter_map(|p| p.as_ref().as_ref_id())
            .collect(),
        Parameter::Ref(id) => vec![*id],
        _ => Vec::new(),
    }
}
