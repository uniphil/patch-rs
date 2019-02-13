use std::error::Error;
use std::borrow::Cow;

use chrono::DateTime;
use nom_locate::LocatedSpan;
use nom::types::CompleteStr;
use nom::simple_errors::Context;
use nom::*;

use crate::ast::*;

type Input<'a> = LocatedSpan<CompleteStr<'a>>;

/// Type returned when an error occurs while parsing a patch
#[derive(Debug, Clone)]
pub struct ParseError<'a> {
    /// The line where the parsing error occurred
    pub line: u32,
    /// The offset on the line where the parsing error occurred
    pub offset: usize,
    /// The actual parsing error
    pub err: nom::Err<&'a str>,
}

#[doc(hidden)]
impl<'a> From<nom::Err<Input<'a>>> for ParseError<'a> {
    fn from(err: nom::Err<Input<'a>>) -> Self {
        match err {
            nom::Err::Incomplete(_) => unreachable!("bug: parser should not return incomplete"),
            // Unify both error types because at this point the error is not recoverable
            nom::Err::Error(ctx) |
            nom::Err::Failure(ctx) => match ctx {
                Context::Code(input, kind) => {
                    let LocatedSpan {line, offset, fragment: CompleteStr(input)} = input;
                    let err = nom::Err::Failure(Context::Code(input, kind));
                    Self {line, offset, err}
                },
            },
        }
    }
}

impl<'a> std::fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Line {}:{}: Error while parsing: {}", self.line, self.offset, self.err)
    }
}

impl<'a> Error for ParseError<'a> {
    fn description(&self) -> &str {
        self.err.description()
    }
}

fn input_to_str(input: Input) -> &str {
    let CompleteStr(s) = input.fragment;
    s
}

fn str_to_input(s: &str) -> Input {
    LocatedSpan::new(CompleteStr(s))
}

pub(crate) fn parse_single_patch(s: &str) -> Result<Patch, ParseError> {
    let (remaining_input, patch) = single_patch(str_to_input(s))?;
    // Parser should return an error instead of producing remaining input
    assert!(remaining_input.fragment.is_empty(), "bug: failed to parse entire input. \
        Remaining: '{}'", input_to_str(remaining_input));
    Ok(patch)
}

pub(crate) fn parse_multiple_patches(s: &str) -> Result<Vec<Patch>, ParseError> {
    let (remaining_input, patches) = multiple_patches(str_to_input(s))?;
    // Parser should return an error instead of producing remaining input
    assert!(remaining_input.fragment.is_empty(), "bug: failed to parse entire input");
    Ok(patches)
}

named!(multiple_patches(Input) -> Vec<Patch>,
    many1!(patch)
);

named!(single_patch(Input) -> Patch,
    terminated!(patch, eof!())
);

named!(patch(Input) -> Patch,
    do_parse!(
        files: headers >>
        hunks: chunks >>
        no_newline_indicator: no_newline_indicator >>
        ({
            let (old, new) = files;
            Patch {old, new, hunks, end_newline: !no_newline_indicator}
        })
    )
);

// Header lines
named!(headers(Input) -> (File, File),
    do_parse!(
        // Ignore any preamble lines in produced diffs
        many0!(tuple!(not!(tag!("---")), take_until_and_consume!("\n"))) >>
        tag!("--- ") >>
        oldfile: header_line_content >>
        char!('\n') >>
        tag!("+++ ") >>
        newfile: header_line_content >>
        char!('\n') >>
        (oldfile, newfile)
    )
);

named!(header_line_content(Input) -> File,
    do_parse!(
        filename: filename >>
        after: opt!(preceded!(space, map!(take_until!("\n"), input_to_str))) >>
        (File {
            path: filename,
            meta: after.and_then(|after| match after {
                "" => None,
                _ => Some(
                    DateTime::parse_from_str(after, "%F %T%.f %z")
                        .or_else(|_| DateTime::parse_from_str(after, "%F %T %z"))
                        .ok()
                        .map_or_else(|| FileMetadata::Other(after), FileMetadata::DateTime)
                ),
            }),
        })
    )
);

// Hunks of the file differences
named!(chunks(Input) -> Vec<Hunk>, many1!(chunk));

named!(chunk(Input) -> Hunk,
    do_parse!(
        ranges: chunk_header >>
        lines: many1!(chunk_line) >>
        ({
            let (old_range, new_range) = ranges;
            Hunk {
                old_range: old_range,
                new_range: new_range,
                lines: lines,
            }
        })
    )
);

named!(chunk_header(Input) -> (Range, Range),
    do_parse!(
        tag!("@@ -") >>
        old_range: range >>
        tag!(" +") >>
        new_range: range >>
        tag!(" @@") >>
        // Ignore any additional context provied after @@ (git sometimes adds this)
        take_until_and_consume!("\n") >>
        (old_range, new_range)
    )
);

named!(range(Input) -> Range,
    do_parse!(
        start: u64_digit >>
        count: opt!(preceded!(tag!(","), u64_digit)) >>
        (Range {start: start, count: count.unwrap_or(1)})
    )
);

named!(u64_digit(Input) -> u64,
    map_res!(digit, |input| input_to_str(input).parse())
);

named!(chunk_line(Input) -> Line,
    alt!(
        preceded!(tag!("+"), take_until_and_consume!("\n")) => {
            |line| Line::Add(input_to_str(line))
        } |
        preceded!(tag!("-"), take_until_and_consume!("\n")) => {
            |line| Line::Remove(input_to_str(line))
        } |
        preceded!(tag!(" "), take_until_and_consume!("\n")) => {
            |line| Line::Context(input_to_str(line))
        }
    )
);

// Trailing newline indicator
named!(no_newline_indicator(Input) -> bool,
    map!(
        opt!(terminated!(tag!("\\ No newline at end of file"), opt!(char!('\n')))),
        |matched| matched.is_some()
    )
);

// Filename parsing
named!(filename(Input) -> Cow<str>,
    alt!(quoted | bare)
);

named!(quoted(Input) -> Cow<str>,
    delimited!(tag!("\""), unescape, tag!("\""))
);

named!(bare(Input) -> Cow<str>,
    map!(is_not!(" \t\r\n"), |data| input_to_str(data).into())
);

named!(unescape(Input) -> Cow<str>,
    map!(
        many1!(alt!(non_escape | escape)),
        |chars: Vec<char>| chars.into_iter().collect::<Cow<str>>()
    )
);

named!(non_escape(Input) -> char,
    none_of!("\\\"\0\n\r\t")
);

named!(escape(Input) -> char,
    preceded!(tag!("\\"), one_of!("\\\"0nrtb"))
);

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    type ParseResult<'a, T> = Result<T, nom::Err<Input<'a>>>;

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
        test_parser!(unescape("file \\\"name\\\"") -> "file \"name\"".to_string());
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
        test_parser!(filename("asdf ") -> @(" ", "asdf".to_string()));

        test_parser!(filename("\"asdf\" ") -> @(" ", "asdf".to_string()));

        test_parser!(filename("\"a s\\\"df\" ") -> @(" ", "a s\"df".to_string()));
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
                meta: Some(FileMetadata::Other("08f78e0addd5bf7b7aa8887e406493e75e8d2b55"))
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
            File {
                path: "lao".into(),
                meta: None,
            },
            File {
                path: "tzu".into(),
                meta: None,
            },
        ));

        let sample3 = "\
--- lao 08f78e0addd5bf7b7aa8887e406493e75e8d2b55
+++ tzu e044048282ce75186ecc7a214fd3d9ba478a2816\n";
        test_parser!(headers(sample3) -> (
            File {
                path: "lao".into(),
                meta: Some(FileMetadata::Other(
                    "08f78e0addd5bf7b7aa8887e406493e75e8d2b55"
                )),
            },
            File {
                path: "tzu".into(),
                meta: Some(FileMetadata::Other(
                    "e044048282ce75186ecc7a214fd3d9ba478a2816"
                )),
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
