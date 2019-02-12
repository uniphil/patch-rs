use std::error::Error;

#[derive(Debug)]
pub enum PatchError<'a> {
    ParseError(nom::Err<&'a [u8]>),
}

impl<'a> From<nom::Err<&'a [u8]>> for PatchError<'a> {
    fn from(err: nom::Err<&'a [u8]>) -> Self {
        PatchError::ParseError(err)
    }
}

impl<'a> std::fmt::Display for PatchError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PatchError::ParseError(err) => write!(f, "Error while parsing: {}", err),
        }
    }
}

impl<'a> Error for PatchError<'a> {
    fn description(&self) -> &str {
        match self {
            PatchError::ParseError(err) => err.description(),
        }
    }
}
