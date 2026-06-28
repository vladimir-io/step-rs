use crate::error::ParseReport;
use crate::header::StepHeader;
use crate::record::Record;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Arena-backed entity store indexed by `#id` (1-based STEP ids).
#[derive(Debug, Clone, Default)]
pub struct RecordStore {
    pub header: StepHeader,
    pub records: Vec<Option<Record>>,
    pub by_type: HashMap<String, Vec<u32>>,
    pub report: ParseReport,
}

impl RecordStore {
    pub fn get(&self, id: u32) -> Option<&Record> {
        let idx = id as usize;
        if idx == 0 || idx >= self.records.len() {
            return None;
        }
        self.records[idx].as_ref()
    }

    pub fn ids_of_type(&self, type_name: &str) -> &[u32] {
        self.by_type
            .get(type_name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn record_count(&self) -> usize {
        self.records.iter().filter(|r| r.is_some()).count()
    }

    pub fn parse_error_count(&self) -> usize {
        self.report.record_errors.len()
    }

    pub fn type_histogram(&self) -> Vec<(String, usize)> {
        let mut entries: Vec<_> = self
            .by_type
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        entries
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseStats {
    pub schema: String,
    pub application_protocol: String,
    pub record_count: usize,
    pub type_count: usize,
    pub parse_errors: usize,
    pub top_types: Vec<TypeCount>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCount {
    pub name: String,
    pub count: usize,
}

pub fn parse_stats(store: &RecordStore) -> ParseStats {
    let ap = store.header.application_protocol();
    ParseStats {
        schema: store
            .header
            .schemas
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown".into()),
        application_protocol: ap.as_str().into(),
        record_count: store.record_count(),
        type_count: store.by_type.len(),
        parse_errors: store.parse_error_count(),
        top_types: store
            .type_histogram()
            .into_iter()
            .take(32)
            .map(|(name, count)| TypeCount { name, count })
            .collect(),
        errors: store
            .report
            .record_errors
            .iter()
            .take(64)
            .map(|e| format!("#{}: {}", e.entity_id, e.message))
            .collect(),
    }
}
