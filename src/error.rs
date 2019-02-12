use std::error::Error;

use crate::parser::ParseError;

#[derive(Debug)]
pub enum PatchError<'a> {
    ParseError(ParseError<'a>),
}

impl<'a> std::fmt::Display for PatchError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PatchError::ParseError(ParseError {line, offset, err}) => {
                write!(f, "Line {}:{}: Error while parsing: {}", line, offset, err)
            },
        }
    }
}

impl<'a> Error for PatchError<'a> {
    fn description(&self) -> &str {
        match self {
            PatchError::ParseError(ParseError {err, ..}) => err.description(),
        }
    }
}
