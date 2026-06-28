use crate::record::{EntityInstance, Record};
use nom::branch::alt;
use nom::character::complete::char;
use nom::combinator::{map, opt};
use nom::multi::many1;
use nom::sequence::{delimited, preceded, terminated, tuple};
use nom::Parser;
use smallvec::SmallVec;

use super::parameter::parse_parameter_list;
use super::tokens::*;
use super::ParseResult;

pub fn entity_instance(input: &str) -> ParseResult<'_, EntityInstance> {
    alt((complex_entity_instance, simple_entity_instance)).parse(input)
}

fn simple_entity_instance(input: &str) -> ParseResult<'_, EntityInstance> {
    let (input, record) = terminated(simple_record, char(';')).parse(input)?;
    Ok((input, EntityInstance::Simple(record)))
}

fn complex_entity_instance(input: &str) -> ParseResult<'_, EntityInstance> {
    let (input, (id, records)) = terminated(
        tuple((
            entity_id,
            preceded(preceded(ws, char('=')), preceded(ws, subsuper_record)),
        )),
        char(';'),
    )
    .parse(input)?;

    let mut records = records;
    if let Some(first) = records.first_mut() {
        first.id = id;
    }

    Ok((
        input,
        EntityInstance::Complex {
            id,
            records,
        },
    ))
}

fn simple_record(input: &str) -> ParseResult<'_, Record> {
    let (input, (id, name, parameters)) = tuple((
        entity_id,
        preceded(preceded(ws, char('=')), preceded(ws, keyword)),
        preceded(
            ws,
            delimited(
                char('('),
                map(opt(parse_parameter_list), |p| p.unwrap_or_default()),
                preceded(ws, char(')')),
            ),
        ),
    ))
    .parse(input)?;

    Ok((
        input,
        Record {
            id,
            name: name.to_string(),
            parameters,
        },
    ))
}

fn subsuper_record(input: &str) -> ParseResult<'_, SmallVec<[Record; 2]>> {
    let (input, records) = delimited(
        char('('),
        many1(preceded(
            ws,
            map(
                tuple((
                    keyword,
                    delimited(
                        char('('),
                        map(opt(parse_parameter_list), |p| p.unwrap_or_default()),
                        preceded(ws, char(')')),
                    ),
                )),
                |(name, parameters)| Record {
                    id: 0,
                    name: name.to_string(),
                    parameters,
                },
            ),
        )),
        preceded(ws, char(')')),
    )
    .parse(input)?;

    Ok((input, records.into_iter().collect()))
}
