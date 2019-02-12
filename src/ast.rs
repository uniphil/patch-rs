use std::str::FromStr;

use chrono::{DateTime, FixedOffset};

use crate::parser::ParseError;

#[derive(Debug, Eq, PartialEq)]
pub struct Patch<'a> {
    pub old: File<'a>,
    pub new: File<'a>,
    pub hunks: Vec<Hunk<'a>>,
    pub no_newline: bool,
}

impl<'a> FromStr for Patch<'a> {
    type Err = ParseError<'a>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        unimplemented!()
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
