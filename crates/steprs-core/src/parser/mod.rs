pub mod entity;
pub mod parameter;
pub mod tokens;

pub type ParseResult<'a, T> = nom::IResult<&'a str, T>;
