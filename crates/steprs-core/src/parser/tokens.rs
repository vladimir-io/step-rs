use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{char, multispace0, one_of};
use nom::combinator::{map, opt, recognize};
use nom::sequence::{delimited, pair};
use nom::Parser;

use super::ParseResult;

pub fn ws(input: &str) -> ParseResult<'_, ()> {
    map(multispace0, |_| ()).parse(input)
}

pub fn keyword(input: &str) -> ParseResult<'_, &str> {
    recognize(take_while1(|c: char| {
        c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'
    }))
    .parse(input)
}

pub fn entity_id(input: &str) -> ParseResult<'_, u32> {
    let (input, _) = char('#').parse(input)?;
    let (input, digits) = take_while1(|c: char| c.is_ascii_digit()).parse(input)?;
    let id: u32 = digits.parse().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
    })?;
    Ok((input, id))
}

pub fn string_literal(input: &str) -> ParseResult<'_, String> {
    let (input, inner) = delimited(char('\''), parse_quoted_content, char('\'')).parse(input)?;
    Ok((input, inner))
}

fn parse_quoted_content(input: &str) -> ParseResult<'_, String> {
    let mut out = String::new();
    let mut cursor = input;
    loop {
        if cursor.starts_with("''") {
            out.push('\'');
            cursor = &cursor[2..];
            continue;
        }
        if let Some(i) = cursor.find('\'') {
            out.push_str(&cursor[..i]);
            return Ok((&cursor[i..], out));
        }
        out.push_str(cursor);
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }
}

pub fn real_number(input: &str) -> ParseResult<'_, f64> {
    let (input, s) = recognize(pair(
        opt(one_of("+-")),
        alt((
            recognize(pair(
                take_while1(|c: char| c.is_ascii_digit()),
                pair(char('.'), take_while1(|c: char| c.is_ascii_digit())),
            )),
            recognize(pair(
                opt(take_while1(|c: char| c.is_ascii_digit())),
                char('.'),
            )),
            take_while1(|c: char| c.is_ascii_digit()),
        )),
    ))
    .parse(input)?;
    let value: f64 = s.parse().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Float))
    })?;
    Ok((input, value))
}

pub fn integer_number(input: &str) -> ParseResult<'_, i64> {
    let (input, s) = recognize(pair(
        opt(one_of("+-")),
        take_while1(|c: char| c.is_ascii_digit()),
    ))
    .parse(input)?;
    let value: i64 = s.parse().map_err(|_| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
    })?;
    Ok((input, value))
}

pub fn enumeration(input: &str) -> ParseResult<'_, String> {
    let (input, value) = recognize(pair(
        char('.'),
        take_while1(|c: char| c.is_ascii_uppercase() || c == '_'),
    ))
    .parse(input)?;
    let (input, _) = char('.').parse(input)?;
    Ok((input, value.to_string()))
}

pub fn entity_ref(input: &str) -> ParseResult<'_, u32> {
    entity_id(input)
}

pub fn asterisk(input: &str) -> ParseResult<'_, ()> {
    map(tag("*"), |_| ()).parse(input)
}
