use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    #[error("unexpected token at byte {offset}")]
    UnexpectedToken { offset: usize },
    #[error("unclosed delimiter")]
    UnclosedDelimiter,
    #[error("invalid entity id")]
    InvalidEntityId,
    #[error("missing DATA section")]
    MissingDataSection,
}

#[derive(Debug, Error, Clone)]
#[error("parse error: {kind}")]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub context: Option<String>,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind) -> Self {
        Self {
            kind,
            context: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

#[derive(Debug, Default, Clone)]
pub struct ParseReport {
    pub fatal: Option<ParseError>,
    pub record_errors: smallvec::SmallVec<[RecordError; 8]>,
}

#[derive(Debug, Clone)]
pub struct RecordError {
    pub entity_id: u32,
    pub message: String,
}

impl ParseReport {
    pub fn push_record_error(&mut self, entity_id: u32, message: impl Into<String>) {
        self.record_errors.push(RecordError {
            entity_id,
            message: message.into(),
        });
    }

    pub fn is_ok(&self) -> bool {
        self.fatal.is_none() && self.record_errors.is_empty()
    }
}
