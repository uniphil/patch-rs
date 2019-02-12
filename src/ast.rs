use chrono::{DateTime, FixedOffset};

use crate::parser::{parse_patch, ParseError};

#[derive(Debug, Eq, PartialEq)]
pub struct Patch<'a> {
    pub old: File<'a>,
    pub new: File<'a>,
    pub hunks: Vec<Hunk<'a>>,
    pub no_newline: bool,
}

impl<'a> Patch<'a> {
    /// Attempt to parse a patch from the given string
    pub fn from_str(s: &'a str) -> Result<Self, ParseError<'a>> {
        parse_patch(s)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct File<'a> {
    pub name: String,
    pub meta: Option<FileMetadata<'a>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum FileMetadata<'a> {
    DateTime(DateTime<FixedOffset>),
    Other(&'a str),
}

#[derive(Debug, Eq, PartialEq)]
pub struct Range {
    pub start: u64,
    pub count: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Hunk<'a> {
    pub old_range: Range,
    pub new_range: Range,
    pub lines: Vec<Line<'a>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Line<'a> {
    Add(&'a str),
    Remove(&'a str),
    Context(&'a str),
}
