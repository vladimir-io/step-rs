use serde::{Deserialize, Serialize};

/// Phase 0 — manufacturing feature MVP catalog (product contract).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManufacturingFeatureKind {
    ThroughHole,
    BlindHole,
    Counterbore,
    Countersink,
    RectangularPocket,
    CircularPocket,
    Boss,
    PlanarFace,
    CylindricalFace,
    Fillet,
    Chamfer,
    Slot,
    Unknown,
}

impl ManufacturingFeatureKind {
    pub const MVP: &'static [Self] = &[
        Self::ThroughHole,
        Self::BlindHole,
        Self::RectangularPocket,
        Self::CircularPocket,
        Self::Boss,
        Self::CylindricalFace,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ThroughHole => "Through hole",
            Self::BlindHole => "Blind hole",
            Self::Counterbore => "Counterbore",
            Self::Countersink => "Countersink",
            Self::RectangularPocket => "Rectangular pocket",
            Self::CircularPocket => "Circular pocket",
            Self::Boss => "Boss",
            Self::PlanarFace => "Planar face",
            Self::CylindricalFace => "Cylindrical face",
            Self::Fillet => "Fillet",
            Self::Chamfer => "Chamfer",
            Self::Slot => "Slot",
            Self::Unknown => "Unknown",
        }
    }
}
