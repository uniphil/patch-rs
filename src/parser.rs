use std::error::Error;
use std::borrow::Cow;

use chrono::DateTime;
use nom_locate::LocatedSpan;
use nom::*;

use nom::{
    branch::alt,
    multi::{many0, many1},
    combinator::{not, opt, map},
    sequence::{tuple, preceded, delimited, terminated},
    bytes::complete::{tag, take_till, take_until, is_not},
    character::complete::{char, space0, digit1, newline, none_of, one_of},
};


use crate::ast::*;

type Input<'a> = LocatedSpan<&'a str>;

/// Type returned when an error occurs while parsing a patch
#[derive(Debug, Clone)]
pub struct ParseError<'a> {
    /// The line where the parsing error occurred
    pub line: u32,
    /// The offset within the input where the parsing error occurred
    pub offset: usize,
    /// The failed input
    pub fragment: &'a str,
    /// The actual parsing error
    pub kind: nom::error::ErrorKind,
}

#[doc(hidden)]
impl<'a> From<nom::Err<(Input<'a>, nom::error::ErrorKind)>> for ParseError<'a> {
    fn from(err: nom::Err<(Input<'a>, nom::error::ErrorKind)>) -> Self {
        match err {
            nom::Err::Incomplete(_) => unreachable!("bug: parser should not return incomplete"),
            // Unify both error types because at this point the error is not recoverable
            nom::Err::Error((input, kind)) |
            nom::Err::Failure((input, kind)) => {
                let LocatedSpan {line, offset, fragment, extra: _extra } = input;
                Self {line, offset, fragment, kind}
            },
        }
    }
}

impl<'a> std::fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Line {}: Error while parsing: {}", self.line, self.fragment)
    }
}

impl<'a> Error for ParseError<'a> {
    fn description(&self) -> &str {
        self.kind.description()
    }
}

fn input_to_str(input: Input) -> &str {
    input.fragment
}

fn str_to_input(s: &str) -> Input {
    LocatedSpan::new(s)
}

fn consume_line(input: Input) -> IResult<Input, &str> {
    let (input, raw) = terminated(take_till(|c| c == '\n'), newline)(input)?;
    Ok((input, input_to_str(raw)))
}

pub(crate) fn parse_single_patch(s: &str) -> Result<Patch, ParseError> {
    let (remaining_input, patch) = patch(str_to_input(s))?;
    // Parser should return an error instead of producing remaining input
    assert!(remaining_input.fragment.is_empty(), "bug: failed to parse entire input. \
        Remaining: '{}'", input_to_str(remaining_input));
    Ok(patch)
}

pub(crate) fn parse_multiple_patches(s: &str) -> Result<Vec<Patch>, ParseError> {
    let (remaining_input, patches) = multiple_patches(str_to_input(s))?;
    // Parser should return an error instead of producing remaining input
    assert!(remaining_input.fragment.is_empty(), "bug: failed to parse entire input. \
        Remaining: '{}'", input_to_str(remaining_input));
    Ok(patches)
}

fn multiple_patches(input: Input) -> IResult<Input, Vec<Patch>> {
    many1(patch)(input)
}

fn patch(input: Input) -> IResult<Input, Patch> {
    let (input, files) = headers(input)?;
    let (input, hunks) = chunks(input)?;
    let (input, no_newline_indicator) = no_newline_indicator(input)?;
    // Ignore trailing empty lines produced by some diff programs
    let (input, _) = many0(newline)(input)?;

    let (old, new) = files;
    Ok((input, Patch {old, new, hunks, end_newline: !no_newline_indicator}))
}

// Header lines
fn headers(input: Input) -> IResult<Input, (File, File)> {
    // Ignore any preamble lines in produced diffs
    let (input, _) = take_until("---")(input)?;
    let (input, _) = tag("--- ")(input)?;
    let (input, oldfile) = header_line_content(input)?;
    let (input, _) = newline(input)?;
    let (input, _) = tag("+++ ")(input)?;
    let (input, newfile) = header_line_content(input)?;
    let (input, _) = newline(input)?;
    Ok((input, (oldfile, newfile)))
}

fn header_line_content(input: Input) -> IResult<Input, File> {
    let (input, filename) = filename(input)?;
    let (input, after) = opt(preceded(space0, file_metadata))(input)?;

    Ok((input, File {
        path: filename,
        meta: after.and_then(|after| match after {
            Cow::Borrowed("") => None,
            _ => Some(
                DateTime::parse_from_str(after.as_ref(), "%F %T%.f %z")
                .or_else(|_| DateTime::parse_from_str(after.as_ref(), "%F %T %z"))
                .ok()
                .map_or_else(|| FileMetadata::Other(after), FileMetadata::DateTime)
            ),
        }),
    }))
}

// Hunks of the file differences
fn chunks(input: Input) -> IResult<Input, Vec<Hunk>> {
    many1(chunk)(input)
}

fn chunk(input: Input) -> IResult<Input, Hunk> {
    let (input, ranges) = chunk_header(input)?;
    let (input, lines) = many1(chunk_line)(input)?;

    let (old_range, new_range) = ranges;
    Ok((input,  Hunk{old_range, new_range, lines}))
}

fn chunk_header(input: Input) -> IResult<Input, (Range, Range)> {
    let (input, _) = tag("@@ -")(input)?;
    let (input, old_range) = range(input)?;
    let (input, _) = tag(" +")(input)?;
    let (input, new_range) = range(input)?;
    let (input, _) = tag(" @@")(input)?;
    // Ignore any additional context provied after @@ (git sometimes adds this)
    let (input, _) = many0(newline)(input)?;
    Ok((input, (old_range, new_range)))
}

fn range(input: Input) -> IResult<Input, Range> {
    let (input, start) = u64_digit(input)?;
    let (input, count) = opt(preceded(tag(","), u64_digit))(input)?;
    let count = count.unwrap_or(1);
    Ok((input, Range { start, count }))
}

fn u64_digit(input: Input) -> IResult<Input, u64> {
    let (input, digits) = digit1(input)?;
    let num = digits.fragment.parse::<u64>().unwrap();
    Ok((input, num))
}

// Looks for lines starting with + or - or space, but not +++ or ---. Not a foolproof check.
//
// For example, if someone deletes a line that was using the pre-decrement (--) operator or adds a
// line that was using the pre-increment (++) operator, this will fail.
//
// Example where this doesn't work:
//
// --- main.c
// +++ main.c
// @@ -1,4 +1,7 @@
// +#include<stdio.h>
// +
//  int main() {
//  double a;
// --- a;
// +++ a;
// +printf("%d\n", a);
//  }
//
// We will fail to parse this entire diff.
//
// By checking for `+++ ` instead of just `+++`, we add at least a little more robustness because
// we know that people typically write `++a`, not `++ a`. That being said, this is still not enough
// to guarantee correctness in all cases.
//
//FIXME: Use the ranges in the chunk header to figure out how many chunk lines to parse. Will need
// to figure out how to count in nom more robustly than many1!(). Maybe using switch!()?
//FIXME: The test_parse_triple_plus_minus_hack test will no longer panic when this is fixed.
fn chunk_line(input: Input) -> IResult<Input, Line> {
   alt((
    map(preceded(tuple((tag("+"), not(tag("++ ")))), consume_line), Line::Add),
    map(preceded(tuple((tag("-"), not(tag("-- ")))), consume_line), Line::Remove),
    map(preceded(tag(" "), consume_line), Line::Context),
   ))(input)
}

// Trailing newline indicator
fn no_newline_indicator(input: Input) -> IResult<Input, bool> {
    map(opt(terminated(tag("\\ No newline at end of file"), opt(newline))), |matched| matched.is_some())(input)
}

fn filename(input: Input) -> IResult<Input, Cow<str>> {
    alt((quoted, bare))(input)
}

fn file_metadata(input: Input) -> IResult<Input, Cow<str>> {
    alt((quoted, map(take_till(|c| c == '\n'), |data: Input| input_to_str(data).into())))(input)
}

fn quoted(input: Input) -> IResult<Input, Cow<str>> {
    delimited(tag("\""), unescaped_str, tag("\""))(input)
}

fn bare(input: Input) -> IResult<Input, Cow<str>> {
    map(is_not(" \t\r\n"), |data| input_to_str(data).into())(input)
}

fn unescaped_str(input: Input) -> IResult<Input, Cow<str>> {
    let (input, raw) = many1(alt((unescaped_char, escaped_char)))(input)?;
    Ok((input, raw.into_iter().collect::<Cow<str>>()))
}

// Parses an unescaped character
fn unescaped_char(input: Input) -> IResult<Input, char> {
    none_of("\0\n\r\t\\\"")(input)
}

// Parses an escaped character and returns its unescaped equivalent
fn escaped_char(input: Input) -> IResult<Input, char> {
    map(
        preceded(char('\\'), one_of(r#"0nrt"\"#)),
        |ch| match ch {
            '0' => '\0',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '"' => '"',
            '\\' => '\\',
            _ => unreachable!(),
        }
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    type ParseResult<'a, T> = Result<T, nom::Err<(Input<'a>, nom::error::ErrorKind)>>;

    // Using a macro instead of a function so that error messages cite the most helpful line number
    macro_rules! test_parser {
        ($parser:ident($input:expr) -> @($expected_remaining_input:expr, $expected:expr $(,)*)) => {
            let (remaining_input, result) = $parser(str_to_input($input))?;
            assert_eq!(input_to_str(remaining_input), $expected_remaining_input,
                "unexpected remaining input after parse");
            assert_eq!(result, $expected);
        };
        ($parser:ident($input:expr) -> $expected:expr) => {
            test_parser!($parser($input) -> @("", $expected));
        };
    }

    #[test]
    fn test_unescape() -> ParseResult<'static, ()> {
        test_parser!(unescaped_str("file \\\"name\\\"") -> "file \"name\"".to_string());
        Ok(())
    }

    #[test]
    fn test_quoted() -> ParseResult<'static, ()> {
        test_parser!(quoted("\"file name\"") -> "file name".to_string());
        Ok(())
    }

    #[test]
    fn test_bare() -> ParseResult<'static, ()> {
        test_parser!(bare("file-name ") -> @(" ", "file-name".to_string()));

        test_parser!(bare("file-name\n") -> @("\n", "file-name".to_string()));
        Ok(())
    }

    #[test]
    fn test_filename() -> ParseResult<'static, ()> {
        // bare
        test_parser!(filename("asdf ") -> @(" ", "asdf".to_string()));

        // quoted
        test_parser!(filename(r#""a/My Project/src/foo.rs" "#) -> @(" ", "a/My Project/src/foo.rs".to_string()));
        test_parser!(filename(r#""\"asdf\" fdsh \\\t\r" "#) -> @(" ", "\"asdf\" fdsh \\\t\r".to_string()));
        test_parser!(filename(r#""a s\"\nd\0f" "#) -> @(" ", "a s\"\nd\0f".to_string()));
        Ok(())
    }

    #[test]
    fn test_header_line_contents() -> ParseResult<'static, ()> {
        test_parser!(header_line_content("lao\n") -> @("\n", File {
            path: "lao".into(),
            meta: None,
        }));

        test_parser!(header_line_content("lao 2002-02-21 23:30:39.942229878 -0800\n") -> @(
            "\n",
            File {
                path: "lao".into(),
                meta: Some(FileMetadata::DateTime(
                    DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00").unwrap()
                )),
            },
        ));

        test_parser!(header_line_content("lao 2002-02-21 23:30:39 -0800\n") -> @(
            "\n",
            File {
                path: "lao".into(),
                meta: Some(FileMetadata::DateTime(
                    DateTime::parse_from_rfc3339("2002-02-21T23:30:39-08:00").unwrap()
                )),
            },
        ));

        test_parser!(header_line_content("lao 08f78e0addd5bf7b7aa8887e406493e75e8d2b55\n") -> @(
            "\n",
            File {
                path: "lao".into(),
                meta: Some(FileMetadata::Other("08f78e0addd5bf7b7aa8887e406493e75e8d2b55".into()))
            },
        ));
        Ok(())
    }

    #[test]
    fn test_headers() -> ParseResult<'static, ()> {
        let sample = "\
--- lao 2002-02-21 23:30:39.942229878 -0800
+++ tzu 2002-02-21 23:30:50.442260588 -0800\n";
        test_parser!(headers(sample) -> (
            File {
                path: "lao".into(),
                meta: Some(FileMetadata::DateTime(
                    DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00").unwrap()
                )),
            },
            File {
                path: "tzu".into(),
                meta: Some(FileMetadata::DateTime(
                    DateTime::parse_from_rfc3339("2002-02-21T23:30:50.442260588-08:00").unwrap()
                )),
            },
        ));

        let sample2 = "\
--- lao
+++ tzu\n";
        test_parser!(headers(sample2) -> (
            File {path: "lao".into(), meta: None},
            File {path: "tzu".into(), meta: None},
        ));

        let sample3 = "\
--- lao 08f78e0addd5bf7b7aa8887e406493e75e8d2b55
+++ tzu e044048282ce75186ecc7a214fd3d9ba478a2816\n";
        test_parser!(headers(sample3) -> (
            File {
                path: "lao".into(),
                meta: Some(FileMetadata::Other("08f78e0addd5bf7b7aa8887e406493e75e8d2b55".into())),
            },
            File {
                path: "tzu".into(),
                meta: Some(FileMetadata::Other("e044048282ce75186ecc7a214fd3d9ba478a2816".into())),
            },
        ));
        Ok(())
    }

    #[test]
    fn test_range() -> ParseResult<'static, ()> {
        test_parser!(range("1,7") -> Range { start: 1, count: 7 });

        test_parser!(range("2") -> Range { start: 2, count: 1 });
        Ok(())
    }

    #[test]
    fn test_chunk_header() -> ParseResult<'static, ()> {
        test_parser!(chunk_header("@@ -1,7 +1,6 @@\n") -> (
            Range { start: 1, count: 7 },
            Range { start: 1, count: 6 },
        ));
        Ok(())
    }

    #[test]
    fn test_chunk() -> ParseResult<'static, ()> {
        let sample = "\
@@ -1,7 +1,6 @@
-The Way that can be told of is not the eternal Way;
-The name that can be named is not the eternal name.
 The Nameless is the origin of Heaven and Earth;
-The Named is the mother of all things.
+The named is the mother of all things.
+
 Therefore let there always be non-being,
   so we may see their subtlety,
 And let there always be being,\n";
        let expected = Hunk {
            old_range: Range { start: 1, count: 7 },
            new_range: Range { start: 1, count: 6 },
            lines: vec![
                Line::Remove("The Way that can be told of is not the eternal Way;"),
                Line::Remove("The name that can be named is not the eternal name."),
                Line::Context("The Nameless is the origin of Heaven and Earth;"),
                Line::Remove("The Named is the mother of all things."),
                Line::Add("The named is the mother of all things."),
                Line::Add(""),
                Line::Context("Therefore let there always be non-being,"),
                Line::Context("  so we may see their subtlety,"),
                Line::Context("And let there always be being,"),
            ],
        };
        test_parser!(chunk(sample) -> expected);
        Ok(())
    }

    #[test]
    fn test_patch() -> ParseResult<'static, ()> {
        // https://www.gnu.org/software/diffutils/manual/html_node/Example-Unified.html
        let sample = "\
--- lao 2002-02-21 23:30:39.942229878 -0800
+++ tzu 2002-02-21 23:30:50.442260588 -0800
@@ -1,7 +1,6 @@
-The Way that can be told of is not the eternal Way;
-The name that can be named is not the eternal name.
 The Nameless is the origin of Heaven and Earth;
-The Named is the mother of all things.
+The named is the mother of all things.
+
 Therefore let there always be non-being,
   so we may see their subtlety,
 And let there always be being,
@@ -9,3 +8,6 @@
 The two are the same,
 But after they are produced,
   they have different names.
+They both may be called deep and profound.
+Deeper and more profound,
+The door of all subtleties!\n";

        let expected = Patch {
            old: File {
                path: "lao".into(),
                meta: Some(FileMetadata::DateTime(
                    DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00").unwrap(),
                )),
            },
            new: File {
                path: "tzu".into(),
                meta: Some(FileMetadata::DateTime(
                    DateTime::parse_from_rfc3339("2002-02-21T23:30:50.442260588-08:00").unwrap(),
                )),
            },
            hunks: vec![
                Hunk {
                    old_range: Range { start: 1, count: 7 },
                    new_range: Range { start: 1, count: 6 },
                    lines: vec![
                        Line::Remove("The Way that can be told of is not the eternal Way;"),
                        Line::Remove("The name that can be named is not the eternal name."),
                        Line::Context("The Nameless is the origin of Heaven and Earth;"),
                        Line::Remove("The Named is the mother of all things."),
                        Line::Add("The named is the mother of all things."),
                        Line::Add(""),
                        Line::Context("Therefore let there always be non-being,"),
                        Line::Context("  so we may see their subtlety,"),
                        Line::Context("And let there always be being,"),
                    ],
                },
                Hunk {
                    old_range: Range { start: 9, count: 3 },
                    new_range: Range { start: 8, count: 6 },
                    lines: vec![
                        Line::Context("The two are the same,"),
                        Line::Context("But after they are produced,"),
                        Line::Context("  they have different names."),
                        Line::Add("They both may be called deep and profound."),
                        Line::Add("Deeper and more profound,"),
                        Line::Add("The door of all subtleties!"),
                    ],
                },
            ],
            end_newline: true,
        };

        test_parser!(patch(sample) -> expected);

        assert_eq!(format!("{}\n", expected), sample);

        Ok(())
    }
}
