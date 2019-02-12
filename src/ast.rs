use chrono::{DateTime, FixedOffset};

use crate::parser::{parse_patch, ParseError};

/// A complete patch summarizing the differences between two files
#[derive(Debug, Eq, PartialEq)]
pub struct Patch<'a> {
    /// The file information of the "-" side of the diff, line prefix: `---`
    pub old: File<'a>,
    /// The file information of the "+" side of the diff, line prefix: `+++`
    pub new: File<'a>,
    /// hunks of differences; each hunk shows one area where the files differ
    pub hunks: Vec<Hunk<'a>>,
    /// true if the last line of a file doesn't end in a newline character
    pub no_newline: bool,
}

impl<'a> Patch<'a> {
    /// Attempt to parse a patch from the given string
    pub fn from_str(s: &'a str) -> Result<Self, ParseError<'a>> {
        parse_patch(s)
    }
}

/// The file path and any additional info of either the old file or the new file
#[derive(Debug, Eq, PartialEq)]
pub struct File<'a> {
    /// The parsed path or file name of the file
    pub path: String,
    /// Any additional information provided with the file path
    pub meta: Option<FileMetadata<'a>>,
}

/// Additional metadata provided with the file path
#[derive(Debug, Eq, PartialEq)]
pub enum FileMetadata<'a> {
    /// A full datetime string, e.g. `2002-02-21 23:30:39.942229878 -0800`
    DateTime(DateTime<FixedOffset>),
    /// Any other string provided after the file path, e.g. git hash, unrecognized timestamp, etc.
    Other(&'a str),
}

/// One area where the files differ
#[derive(Debug, Eq, PartialEq)]
pub struct Hunk<'a> {
    /// The range of lines in the old file that this hunk represents
    pub old_range: Range,
    /// The range of lines in the new file that this hunk represents
    pub new_range: Range,
    pub lines: Vec<Line<'a>>,
}

/// A range of lines in a given file
#[derive(Debug, Eq, PartialEq)]
pub struct Range {
    /// The start line of the chunk in the old or new file
    pub start: u64,
    /// The chunk size in the old or new file
    pub count: u64,
}

/// A line of the old file, new file, or both
#[derive(Debug, Eq, PartialEq)]
pub enum Line<'a> {
    /// A line added to the old file in the new file
    Add(&'a str),
    /// A line removed from the old file in the new file
    Remove(&'a str),
    /// A line provided for context in the diff; from both the old and the new file
    Context(&'a str),
}
