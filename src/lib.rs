//! patch-rs is a parser library for [Unified Format]
//! (https://www.gnu.org/software/diffutils/manual/html_node/Unified-Format.html#Unified-Format)
//! diffs.
//!
//! GVR also honed down the spec a bit more:
//! http://www.artima.com/weblogs/viewpost.jsp?thread=164293

extern crate chrono;
extern crate nom;

use std::error::Error;

use self::parser::patch;
pub use self::parser::{File, FileMetadata, Hunk, Line, Patch, Range};

mod parser;

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

pub fn parse(diff: &str) -> Result<Patch, PatchError> {
    Ok(patch(diff.as_bytes()).map(|(remaining_input, patch)| {
        // Parser should return an error instead of producing remaining input
        assert!(remaining_input.is_empty(), "bug: failed to parse entire input");
        patch
    })?)
}
