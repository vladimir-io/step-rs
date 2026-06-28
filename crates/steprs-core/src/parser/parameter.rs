use crate::parameter::Parameter;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::char;
use nom::combinator::{map, value};
use nom::multi::separated_list0;
use nom::sequence::{delimited, preceded};
use nom::Parser;
use smallvec::SmallVec;

use super::tokens::*;
use super::ParseResult;

pub fn parse_parameter_list(input: &str) -> ParseResult<'_, SmallVec<[Parameter; 8]>> {
    let (input, _) = ws(input)?;
    let (input, params) = separated_list0(preceded(ws, char(',')), parameter).parse(input)?;
    Ok((input, params.into_iter().collect()))
}

pub fn parameter(input: &str) -> ParseResult<'_, Parameter> {
    let (input, _) = ws(input)?;
    alt((
        map(asterisk, |_| Parameter::Omitted),
        typed_parameter,
        map(entity_ref, Parameter::Ref),
        map(enumeration, Parameter::Enum),
        map(string_literal, Parameter::String),
        map(real_number, Parameter::Real),
        map(integer_number, Parameter::Integer),
        list_parameter,
        value(Parameter::Null, tag("$")),
    ))
    .parse(input)
}

fn list_parameter(input: &str) -> ParseResult<'_, Parameter> {
    let (input, items) = delimited(
        char('('),
        separated_list0(preceded(ws, char(',')), parameter),
        preceded(ws, char(')')),
    )
    .parse(input)?;
    Ok((
        input,
        Parameter::List(items.into_iter().map(Box::new).collect()),
    ))
}

fn typed_parameter(input: &str) -> ParseResult<'_, Parameter> {
    let (input, type_name) = keyword(input)?;
    let (input, _) = ws(input)?;
    let (input, value) = delimited(char('('), parameter, preceded(ws, char(')'))).parse(input)?;
    Ok((
        input,
        Parameter::Typed {
            type_name: type_name.to_string(),
            value: Box::new(value),
        },
    ))
}
