use smallvec::SmallVec;

/// ISO 10303-21 parameter value (schema-agnostic).
#[derive(Debug, Clone, PartialEq)]
pub enum Parameter {
    Integer(i64),
    Real(f64),
    String(String),
    Enum(String),
    Ref(u32),
    List(SmallVec<[Box<Parameter>; 4]>),
    Typed {
        type_name: String,
        value: Box<Parameter>,
    },
    Null,
    Omitted,
}

impl Parameter {
    pub fn as_ref_id(&self) -> Option<u32> {
        match self {
            Self::Ref(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_real(&self) -> Option<f64> {
        match self {
            Self::Real(v) => Some(*v),
            Self::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<Vec<&Parameter>> {
        match self {
            Self::List(items) => Some(items.iter().map(|p| p.as_ref()).collect()),
            _ => None,
        }
    }

    pub fn as_enum_bool(&self) -> Option<bool> {
        match self {
            Self::Enum(s) => match s.as_str() {
                ".T." | ".TRUE." => Some(true),
                ".F." | ".FALSE." => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// Extract (x, y, z) from a 3-element real list.
    pub fn as_vec3(&self) -> Option<[f64; 3]> {
        let list = self.as_list()?;
        if list.len() != 3 {
            return None;
        }
        Some([list[0].as_real()?, list[1].as_real()?, list[2].as_real()?])
    }
}
