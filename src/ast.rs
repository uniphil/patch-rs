use std::borrow::Cow;
use std::fmt;

use chrono::{DateTime, FixedOffset};

use crate::parser::{parse_multiple_patches, parse_single_patch, ParseError};

pub trait FromS<T> {
    fn from_s(that: T) -> Self;
}

impl<Z, S: FromS<Z>> FromS<Option<Z>> for Option<S> {
    fn from_s(option: Option<Z>) -> Self {
        option.map(FromS::from_s)
    }
}

impl<Z, S: FromS<Z>> FromS<Vec<Z>> for Vec<S> {
    fn from_s(vec: Vec<Z>) -> Self {
        vec.into_iter().map(FromS::from_s).collect()
    }
}

impl<'a> FromS<&'a str> for Cow<'a, str> {
    fn from_s(s: &'a str) -> Self {
        s.into()
    }
}

impl<'a> FromS<String> for Cow<'a, str> {
    fn from_s(s: String) -> Self {
        s.into()
    }
}

impl<'a> FromS<Cow<'a, str>> for String {
    fn from_s(s: Cow<'a, str>) -> Self {
        s.into()
    }
}

impl<const N: usize, Z, S: FromS<Z>> FromS<[Z; N]> for [S; N] {
    fn from_s(array: [Z; N]) -> Self {
        array.map(FromS::from_s)
    }
}

impl<Z, S: FromS<Z>> FromS<(Z,)> for (S,) {
    fn from_s(tuple: (Z,)) -> Self {
        (tuple.0.into_s(),)
    }
}

impl<Z1, Z2, S1: FromS<Z1>, S2: FromS<Z2>> FromS<(Z1, Z2)> for (S1, S2) {
    fn from_s(tuple: (Z1, Z2)) -> Self {
        (tuple.0.into_s(), tuple.1.into_s())
    }
}

impl<Z1, Z2, Z3, S1: FromS<Z1>, S2: FromS<Z2>, S3: FromS<Z3>> FromS<(Z1, Z2, Z3)> for (S1, S2, S3) {
    fn from_s(tuple: (Z1, Z2, Z3)) -> Self {
        (tuple.0.into_s(), tuple.1.into_s(), tuple.2.into_s())
    }
}

impl<Z1, Z2, Z3, Z4, S1: FromS<Z1>, S2: FromS<Z2>, S3: FromS<Z3>, S4: FromS<Z4>>
    FromS<(Z1, Z2, Z3, Z4)> for (S1, S2, S3, S4)
{
    fn from_s(tuple: (Z1, Z2, Z3, Z4)) -> Self {
        (
            tuple.0.into_s(),
            tuple.1.into_s(),
            tuple.2.into_s(),
            tuple.3.into_s(),
        )
    }
}

pub trait IntoS<T> {
    fn into_s(self) -> T;
}

impl<Q, T: FromS<Q>> IntoS<T> for Q {
    fn into_s(self) -> T {
        FromS::from_s(self)
    }
}

/// A complete patch summarizing the differences between two files
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Patch<S> {
    /// The file information of the `-` side of the diff, line prefix: `---`
    pub old: File<S>,
    /// The file information of the `+` side of the diff, line prefix: `+++`
    pub new: File<S>,
    /// hunks of differences; each hunk shows one area where the files differ
    pub hunks: Vec<Hunk<S>>,
    /// true if the last line of the file ends in a newline character
    ///
    /// This will only be false if at the end of the patch we encounter the text:
    /// `\ No newline at end of file`
    pub end_newline: bool,
}

impl<Z, S: FromS<Z>> FromS<Patch<Z>> for Patch<S> {
    fn from_s(
        Patch {
            old,
            new,
            hunks,
            end_newline,
        }: Patch<Z>,
    ) -> Self {
        Patch {
            old: old.into_s(),
            new: new.into_s(),
            hunks: hunks.into_s(),
            end_newline,
        }
    }
}

impl<S: AsRef<str>> fmt::Display for Patch<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Display implementations typically hold up the invariant that there is no trailing
        // newline. This isn't enforced, but it allows them to work well with `println!`

        write!(f, "--- {}", self.old)?;
        write!(f, "\n+++ {}", self.new)?;
        for hunk in &self.hunks {
            write!(f, "\n{}", hunk)?;
        }
        if !self.end_newline {
            write!(f, "\n\\ No newline at end of file")?;
        }
        Ok(())
    }
}

impl<'a> Patch<Cow<'a, str>> {
    #[allow(clippy::tabs_in_doc_comments)]
    /// Attempt to parse a patch from the given string.
    ///
    /// # Example
    ///
    /// ```
    /// # fn main() -> Result<(), patch::ParseError<'static>> {
    /// # use patch::Patch;
    /// let sample = "\
    /// --- lao	2002-02-21 23:30:39.942229878 -0800
    /// +++ tzu	2002-02-21 23:30:50.442260588 -0800
    /// @@ -1,7 +1,6 @@
    /// -The Way that can be told of is not the eternal Way;
    /// -The name that can be named is not the eternal name.
    ///  The Nameless is the origin of Heaven and Earth;
    /// -The Named is the mother of all things.
    /// +The named is the mother of all things.
    /// +
    ///  Therefore let there always be non-being,
    ///  so we may see their subtlety,
    ///  And let there always be being,
    /// @@ -9,3 +8,6 @@
    ///  The two are the same,
    ///  But after they are produced,
    ///  they have different names.
    /// +They both may be called deep and profound.
    /// +Deeper and more profound,
    /// +The door of all subtleties!
    /// \\ No newline at end of file\n";
    ///
    /// let patch = Patch::from_single(sample)?;
    /// assert_eq!(&patch.old.path, "lao");
    /// assert_eq!(&patch.new.path, "tzu");
    /// assert_eq!(patch.end_newline, false);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_single(s: &'a str) -> Result<Self, ParseError<'a>> {
        parse_single_patch(s)
    }

    /// Attempt to parse as many patches as possible from the given string. This is useful for when
    /// you have a complete diff of many files. String must contain at least one patch.
    ///
    /// # Example
    ///
    /// ```
    /// # fn main() -> Result<(), patch::ParseError<'static>> {
    /// # use patch::Patch;
    /// let sample = "\
    /// diff --git a/src/generator/place_items.rs b/src/generator/place_items.rs
    /// index 508f4e9..31a167e 100644
    /// --- a/src/generator/place_items.rs
    /// +++ b/src/generator/place_items.rs
    /// @@ -233,7 +233,7 @@ impl<'a> GameGenerator<'a> {
    ///          //     oooooooo
    ///          //
    ///          // x would pass all of the previous checks but get caught by this one
    /// -        if grid.adjacent_positions(inner_room_tile).find(|&pt| grid.is_room_entrance(pt)).is_some() {
    /// +        if grid.adjacent_positions(inner_room_tile).any(|&pt| grid.is_room_entrance(pt)) {
    ///              return None;
    ///          }
    ///
    /// diff --git a/src/ui/level_screen.rs b/src/ui/level_screen.rs
    /// index 81fe540..166bb2b 100644
    /// --- a/src/ui/level_screen.rs
    /// +++ b/src/ui/level_screen.rs
    /// @@ -48,7 +48,7 @@ impl<'a, 'b> LevelScreen<'a, 'b> {
    ///          // Find the empty position adjacent to this staircase. There should only be one.
    ///          let map = self.world.read_resource::<FloorMap>();
    ///          let tile_pos = map.world_to_tile_pos(pos);
    /// -        let empty = map.grid().adjacent_positions(tile_pos).find(|&p| !map.grid().get(p).is_wall())
    /// +        let empty = map.grid().adjacents(tile_pos).find(|t| !t.is_wall())
    ///              .expect(\"bug: should be one empty position adjacent to a staircase\");
    ///          empty.center(map.tile_size() as i32)
    ///      }
    /// @@ -64,7 +64,7 @@ impl<'a, 'b> LevelScreen<'a, 'b> {
    ///          // Find the empty position adjacent to this staircase. There should only be one.
    ///          let map = self.world.read_resource::<FloorMap>();
    ///          let tile_pos = map.world_to_tile_pos(pos);
    /// -        let empty = map.grid().adjacent_positions(tile_pos).find(|&p| !map.grid().get(p).is_wall())
    /// +        let empty = map.grid().adjacents(tile_pos).find(|t| !t.is_wall())
    ///              .expect(\"bug: should be one empty position adjacent to a staircase\");
    ///          empty.center(map.tile_size() as i32)
    ///      }\n";
    ///
    /// let patches = Patch::from_multiple(sample)?;
    /// assert_eq!(patches.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_multiple(s: &'a str) -> Result<Vec<Self>, ParseError<'a>> {
        parse_multiple_patches(s)
    }
}

/// Check if a string needs to be quoted, and format it accordingly
fn maybe_escape_quote(f: &mut fmt::Formatter, s: &str) -> fmt::Result {
    let quote = s
        .chars()
        .any(|ch| matches!(ch, ' ' | '\t' | '\r' | '\n' | '\"' | '\0' | '\\'));

    if quote {
        write!(f, "\"")?;
        for ch in s.chars() {
            match ch {
                '\0' => write!(f, r"\0")?,
                '\n' => write!(f, r"\n")?,
                '\r' => write!(f, r"\r")?,
                '\t' => write!(f, r"\t")?,
                '"' => write!(f, r#"\""#)?,
                '\\' => write!(f, r"\\")?,
                _ => write!(f, "{}", ch)?,
            }
        }
        write!(f, "\"")
    } else {
        write!(f, "{}", s)
    }
}

/// The file path and any additional info of either the old file or the new file
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct File<S> {
    /// The parsed path or file name of the file
    ///
    /// Avoids allocation if at all possible. Only allocates if the file path is a quoted string
    /// literal. String literals are necessary in some cases, for example if the file path has
    /// spaces in it. These literals can contain escaped characters which are initially seen as
    /// groups of two characters by the parser (e.g. '\\' + 'n'). A newly allocated string is
    /// used to unescape those characters (e.g. "\\n" -> '\n').
    ///
    /// **Note:** While this string is typically a file path, this library makes no attempt to
    /// verify the format of that path. That means that **this field can potentially be any
    /// string**. You should verify it before doing anything that may be security-critical.
    pub path: S,
    /// Any additional information provided with the file path
    pub meta: Option<FileMetadata<S>>,
}

impl<Z, S: FromS<Z>> FromS<File<Z>> for File<S> {
    fn from_s(File { path, meta }: File<Z>) -> Self {
        File {
            path: path.into_s(),
            meta: meta.into_s(),
        }
    }
}

impl<S: AsRef<str>> fmt::Display for File<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        maybe_escape_quote(f, self.path.as_ref())?;
        if let Some(meta) = &self.meta {
            write!(f, "\t{}", meta)?;
        }
        Ok(())
    }
}

/// Additional metadata provided with the file path
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FileMetadata<S> {
    /// A complete datetime, e.g. `2002-02-21 23:30:39.942229878 -0800`
    DateTime(DateTime<FixedOffset>),
    /// Any other string provided after the file path, e.g. git hash, unrecognized timestamp, etc.
    Other(S),
}

impl<Z, S: FromS<Z>> FromS<FileMetadata<Z>> for FileMetadata<S> {
    fn from_s(metadata: FileMetadata<Z>) -> Self {
        match metadata {
            FileMetadata::DateTime(datetime) => FileMetadata::DateTime(datetime),
            FileMetadata::Other(other) => FileMetadata::Other(other.into_s()),
        }
    }
}

impl<S: AsRef<str>> fmt::Display for FileMetadata<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileMetadata::DateTime(datetime) => {
                write!(f, "{}", datetime.format("%F %T%.f %z"))
            }
            FileMetadata::Other(data) => maybe_escape_quote(f, data.as_ref()),
        }
    }
}

/// One area where the files differ
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Hunk<S> {
    /// The range of lines in the old file that this hunk represents
    pub old_range: Range,
    /// The range of lines in the new file that this hunk represents
    pub new_range: Range,
    /// Any trailing text after the hunk's range information
    pub range_hint: S,
    /// Each line of text in the hunk, prefixed with the type of change it represents
    pub lines: Vec<Line<S>>,
}

impl<Z, S: FromS<Z>> FromS<Hunk<Z>> for Hunk<S> {
    fn from_s(
        Hunk {
            old_range,
            new_range,
            range_hint,
            lines,
        }: Hunk<Z>,
    ) -> Self {
        Hunk {
            old_range,
            new_range,
            range_hint: range_hint.into_s(),
            lines: lines.into_s(),
        }
    }
}

impl<S: AsRef<str>> Hunk<S> {
    /// A nicer way to access the optional hint
    pub fn hint(&self) -> Option<&str> {
        let h = self.range_hint.as_ref().trim_start();
        if h.is_empty() {
            None
        } else {
            Some(h)
        }
    }
}

impl<S: AsRef<str>> fmt::Display for Hunk<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "@@ -{} +{} @@{}",
            self.old_range,
            self.new_range,
            self.range_hint.as_ref()
        )?;

        for line in &self.lines {
            write!(f, "\n{}", line)?;
        }

        Ok(())
    }
}

/// A range of lines in a given file
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Range {
    /// The start line of the chunk in the old or new file
    pub start: u64,
    /// The chunk size (number of lines) in the old or new file
    pub count: u64,
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},{}", self.start, self.count)
    }
}

/// A line of the old file, new file, or both
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Line<S> {
    /// A line added to the old file in the new file
    Add(S),
    /// A line removed from the old file in the new file
    Remove(S),
    /// A line provided for context in the diff (unchanged); from both the old and the new file
    Context(S),
}

impl<Z, S: FromS<Z>> FromS<Line<Z>> for Line<S> {
    fn from_s(line: Line<Z>) -> Self {
        match line {
            Line::Add(line) => Line::Add(line.into_s()),
            Line::Remove(line) => Line::Remove(line.into_s()),
            Line::Context(line) => Line::Context(line.into_s()),
        }
    }
}

impl<S: AsRef<str>> fmt::Display for Line<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Line::Add(line) => write!(f, "+{}", line.as_ref()),
            Line::Remove(line) => write!(f, "-{}", line.as_ref()),
            Line::Context(line) => write!(f, " {}", line.as_ref()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_hint_helper() {
        let mut h = Hunk {
            old_range: Range { start: 0, count: 0 },
            new_range: Range { start: 0, count: 0 },
            range_hint: "",
            lines: vec![],
        };
        for (input, expected) in vec![
            ("", None),
            (" ", None),
            ("  ", None),
            ("x", Some("x")),
            (" x", Some("x")),
            ("x ", Some("x ")),
            (" x ", Some("x ")),
            ("  abc def ", Some("abc def ")),
        ] {
            h.range_hint = input;
            assert_eq!(h.hint(), expected);
        }
    }
}
