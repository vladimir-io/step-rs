pub mod entity;
pub mod parameter;
pub mod tokens;

use crate::error::{ParseError, ParseErrorKind};
use crate::record::EntityInstance;
pub type ParseResult<'a, T> = nom::IResult<&'a str, T>;

pub fn parse_data_entities(input: &str) -> Result<Vec<EntityInstance>, ParseError> {
    let data_start = input
        .find("DATA;")
        .ok_or_else(|| ParseError::new(ParseErrorKind::MissingDataSection))?;
    let mut cursor = &input[data_start + 5..];
    let end = cursor.find("ENDSEC;").unwrap_or(cursor.len());
    cursor = &cursor[..end];

    let mut entities = Vec::new();
    while let Some(next) = find_next_entity(cursor) {
        cursor = &cursor[next.offset..];
        match entity::entity_instance(cursor) {
            Ok((rest, instance)) => {
                entities.push(instance);
                cursor = rest;
            }
            Err(_) => {
                if let Some(skip) = skip_to_semicolon(cursor) {
                    cursor = &cursor[skip..];
                } else {
                    break;
                }
            }
        }
    }

    Ok(entities)
}

struct NextEntity {
    offset: usize,
}

fn find_next_entity(s: &str) -> Option<NextEntity> {
    let hash = s.find('#')?;
    Some(NextEntity { offset: hash })
}

fn skip_to_semicolon(s: &str) -> Option<usize> {
    let semi = s.find(';')?;
    Some(semi + 1)
}
