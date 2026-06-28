//! ISO 10303-21 (Part 21) schema-agnostic parser and record store.

pub mod catalog;
pub mod error;
pub mod header;
pub mod parameter;
pub mod parser;
pub mod progress;
pub mod record;
pub mod store;
pub mod streaming;

pub use catalog::ManufacturingFeatureKind;
pub use error::{ParseError, ParseErrorKind, ParseReport};
pub use header::{ApplicationProtocol, StepHeader};
pub use parameter::Parameter;
pub use progress::{NoopProgress, ParsePhase, ParseProgress};
pub use record::{EntityInstance, Record};
pub use store::{parse_stats, ParseStats, RecordStore};
pub use streaming::parse_step_streaming;

/// Parse a STEP file (streaming, fail-soft per entity).
pub fn parse_step_file(content: &str) -> Result<RecordStore, crate::error::ParseError> {
    streaming::parse_step_file(content)
}
