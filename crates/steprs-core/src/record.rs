use crate::parameter::Parameter;
use smallvec::SmallVec;

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub id: u32,
    pub name: String,
    pub parameters: SmallVec<[Parameter; 8]>,
}

impl Record {
    pub fn param(&self, index: usize) -> Option<&Parameter> {
        self.parameters.get(index)
    }

    pub fn ref_at(&self, index: usize) -> Option<u32> {
        self.param(index)?.as_ref_id()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityInstance {
    Simple(Record),
    Complex {
        id: u32,
        records: SmallVec<[Record; 2]>,
    },
}

impl EntityInstance {
    pub fn id(&self) -> u32 {
        match self {
            Self::Simple(r) => r.id,
            Self::Complex { id, .. } => *id,
        }
    }

    pub fn primary_record(&self) -> &Record {
        match self {
            Self::Simple(r) => r,
            Self::Complex { records, .. } => &records[0],
        }
    }
}
