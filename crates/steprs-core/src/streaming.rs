use crate::error::{ParseError, ParseErrorKind, ParseReport};
use crate::header::parse_header_section;
use crate::parser::entity;
use crate::progress::{NoopProgress, ParsePhase, ParseProgress};
use crate::record::{EntityInstance, Record};
use crate::store::RecordStore;
use std::collections::HashMap;

/// Incremental Part 21 DATA parser — no intermediate `Vec<EntityInstance>`.
pub fn parse_step_streaming(
    content: &str,
    progress: Option<&dyn ParseProgress>,
) -> Result<RecordStore, ParseError> {
    let progress = progress.unwrap_or(&NoopProgress);
    let header = parse_header_section(content).unwrap_or_default();

    let data_start = content
        .find("DATA;")
        .ok_or_else(|| ParseError::new(ParseErrorKind::MissingDataSection))?;

    progress.on_phase(ParsePhase::DataScan, 0, None);

    let mut cursor = &content[data_start + 5..];
    let end = cursor.find("ENDSEC;").unwrap_or(cursor.len());
    cursor = &cursor[..end];

    let total_est = count_entity_markers(cursor);
    progress.on_phase(ParsePhase::DataScan, 0, Some(total_est.max(1)));

    let mut store = RecordStore {
        header,
        records: Vec::with_capacity(total_est.max(4096)),
        by_type: HashMap::with_capacity(128),
        report: ParseReport::default(),
    };

    let mut parsed = 0usize;
    let mut failed = 0usize;
    let mut pos = 0usize;
    let data = cursor;
    let bytes = data.as_bytes();

    while pos < bytes.len() {
        if bytes[pos] == b'#' && pos + 1 < bytes.len() && bytes[pos + 1].is_ascii_digit() {
            let slice = &data[pos..];
            let entity_id = peek_entity_id(slice);

            match entity::entity_instance(slice) {
                Ok((rest, instance)) => {
                    ingest_instance(&mut store, instance);
                    pos += slice.len() - rest.len();
                    parsed += 1;
                    if parsed % 500 == 0 || parsed == total_est {
                        progress.on_phase(
                            ParsePhase::DataScan,
                            parsed,
                            Some(total_est.max(parsed + failed)),
                        );
                    }
                }
                Err(_) => {
                    failed += 1;
                    if let Some(id) = entity_id {
                        store
                            .report
                            .push_record_error(id, "failed to parse entity instance; skipped");
                    }
                    if let Some(semi) = slice.find(';') {
                        pos += semi + 1;
                    } else {
                        break;
                    }
                }
            }
        } else {
            pos += 1;
        }
    }

    progress.on_phase(ParsePhase::Indexing, parsed, Some(parsed + failed));

    if parsed == 0 && failed > 0 {
        return Err(
            ParseError::new(ParseErrorKind::UnexpectedToken { offset: data_start })
                .with_context(format!("no entities parsed ({failed} failures)")),
        );
    }

    Ok(store)
}

/// Count `#` markers followed by digits (cheap pre-scan for progress totals).
fn count_entity_markers(data: &str) -> usize {
    let bytes = data.as_bytes();
    let mut count = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                count += 1;
                i = j;
                continue;
            }
        }
        i += 1;
    }
    count
}

fn peek_entity_id(cursor: &str) -> Option<u32> {
    let rest = cursor.strip_prefix('#')?;
    let digits: usize = rest
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .map(|c| c.len_utf8())
        .sum();
    if digits == 0 {
        return None;
    }
    rest[..digits].parse().ok()
}

fn ingest_instance(store: &mut RecordStore, entity: EntityInstance) {
    match entity {
        EntityInstance::Simple(record) => insert_record(store, record),
        EntityInstance::Complex { id, mut records } => {
            if let Some(first) = records.first_mut() {
                first.id = id;
                insert_record(store, first.clone());
            }
        }
    }
}

fn insert_record(store: &mut RecordStore, record: Record) {
    let id = record.id as usize;
    if id >= store.records.len() {
        let new_len = (id + 1).max(store.records.len() * 2);
        store.records.resize(new_len, None);
    }
    store
        .by_type
        .entry(record.name.clone())
        .or_default()
        .push(record.id);
    store.records[id] = Some(record);
}

pub fn parse_step_file(content: &str) -> Result<RecordStore, ParseError> {
    parse_step_streaming(content, None)
}
