use std::str;

use chrono::DateTime;
use nom::types::CompleteStr;
use nom::*;

use crate::ast::*;

#[derive(Debug)]
pub struct ParseError {}

type Input<'a> = CompleteStr<'a>;

fn input_to_str(input: Input) -> &str {
    let CompleteStr(s) = input;
    s
}

pub(crate) fn parse_patch(s: &str) -> Result<Patch, ParseError> {
    let input = CompleteStr(s);
    match patch(input) {
        Ok((remaining_input, patch)) => {
            // Parser should return an error instead of producing remaining input
            assert!(remaining_input.is_empty(), "bug: failed to parse entire input");
            Ok(patch)
        },
        Err(err) => unimplemented!(),
    }
}

/*
 * Filename parsing
 */

named!(non_escape(Input) -> char, none_of!("\\\"\0\n\r\t"));

named!(escape(Input) -> char,
    do_parse!(
        tag!("\\") >>
        c: one_of!("\\\"0nrtb") >>
        (c)
    )
);

named!(unescape(Input) -> String,
    map!(many1!(alt!(non_escape | escape)), |chars: Vec<char>| chars
        .into_iter()
        .collect::<String>())
);

named!(quoted(Input) -> String, delimited!(tag!("\""), unescape, tag!("\"")));

named!(bare(Input) -> String,
    map_res!(
        map!(is_not!(" \n"), input_to_str),
        str::FromStr::from_str
    )
);

named!(fname(Input) -> String, alt!(quoted | bare));

/*
 * Header lines
 */

named!(header_line_content(Input) -> File,
    do_parse!(
        filename: fname >>
        opt!(space) >>
        after: map!(take_until!("\n"), input_to_str) >>
        (File {
            name: filename,
            meta: {
                if after.is_empty() {
                    None
                } else if let Ok(dt) = DateTime::parse_from_str(after, "%F %T%.f %z")
                    .or_else(|_| DateTime::parse_from_str(after, "%F %T %z"))
                {
                    Some(FileMetadata::DateTime(dt))
                } else {
                    Some(FileMetadata::Other(after))
                }
            },
        })
    )
);

named!(headers(Input) -> (File, File),
    do_parse!(
        tag!("---") >>
        space >>
        oldfile: header_line_content >>
        char!('\n') >>
        tag!("+++") >>
        space >>
        newfile: header_line_content >>
        char!('\n') >>
        (oldfile, newfile)
    )
);

/*
 * Chunk intro
 */

named!(u64_digit(Input) -> u64,
    map_res!(
        map!(digit, input_to_str),
        str::FromStr::from_str
    )
);

named!(range(Input) -> Range,
    do_parse!(
        start: u64_digit
            >> count: opt!(complete!(preceded!(tag!(","), u64_digit)))
            >> (Range {
                start: start,
                count: count.unwrap_or(1)
            })
    )
);

named!(chunk_intro(Input) -> (Range, Range),
    do_parse!(
        tag!("@@ -") >>
        old_range: range >>
        tag!(" +") >>
        new_range: range >>
        tag!(" @@") >>
        char!('\n') >>
        (old_range, new_range)
    )
);

/*
 * Chunk lines
 */

named!(chunk_line(Input) -> Line,
    alt!(
        map!(
            map!(
                preceded!(tag!("+"), take_until_and_consume!("\n")),
                input_to_str
            ),
            Line::Add
        ) | map!(
            map!(
                preceded!(tag!("-"), take_until_and_consume!("\n")),
                input_to_str
            ),
            Line::Remove
        ) | map!(
            map!(
                preceded!(tag!(" "), take_until_and_consume!("\n")),
                input_to_str
            ),
            Line::Context
        )
    )
);

named!(chunk(Input) -> Hunk,
    do_parse!(
        ranges: chunk_intro >>
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

// "Next come one or more hunks of differences"
named!(chunks(Input) -> Vec<Hunk>, many1!(chunk));

/*
 * Trailing newline indicator
 */

named!(no_newline(Input) -> bool,
    map!(
        opt!(complete!(tag!("\\ No newline at end of file"))),
        |matched: Option<_>| matched.is_none()
    )
);

/*
 * The real deal
 */

named!(patch(Input) -> Patch,
    do_parse!(
        files: headers >>
        hunks: chunks >>
        no_newline: no_newline >>
        ({
            let (old, new) = files;
            Patch {
                old: old,
                new: new,
                hunks: hunks,
                no_newline: no_newline,
            }
        })
    )
);

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn test_unescape() {
        assert_eq!(
            unescape("file \\\"name\\\"".into()).unwrap(),
            ("".into(), "file \"name\"".to_string())
        );
    }

    #[test]
    fn test_quoted() {
        assert_eq!(
            quoted("\"file name\"".into()).unwrap(),
            ("".into(), "file name".to_string())
        );
    }

    #[test]
    fn test_bare() {
        assert_eq!(
            bare("file-name ".into()).unwrap(),
            (" ".into(), "file-name".to_string())
        );

        assert_eq!(
            bare("file-name\n".into()).unwrap(),
            ("\n".into(), "file-name".to_string())
        );
    }

    #[test]
    fn test_fname() {
        assert_eq!(
            fname("asdf ".into()).unwrap(),
            (" ".into(), "asdf".to_string())
        );

        assert_eq!(
            fname("\"asdf\" ".into()).unwrap(),
            (" ".into(), "asdf".to_string())
        );

        assert_eq!(
            fname("\"a s\\\"df\" ".into()).unwrap(),
            (" ".into(), "a s\"df".to_string())
        );
    }

    #[test]
    fn test_header_line_contents() {
        assert_eq!(
            header_line_content("lao\n".into()).unwrap(),
            (
                "\n".into(),
                File {
                    name: "lao".to_string(),
                    meta: None
                }
            )
        );

        assert_eq!(
            header_line_content("lao 2002-02-21 23:30:39.942229878 -0800\n".into()).unwrap(),
            (
                "\n".into(),
                File {
                    name: "lao".to_string(),
                    meta: Some(FileMetadata::DateTime(
                        DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00").unwrap()
                    )),
                }
            )
        );

        assert_eq!(
            header_line_content("lao 2002-02-21 23:30:39 -0800\n".into()).unwrap(),
            (
                "\n".into(),
                File {
                    name: "lao".to_string(),
                    meta: Some(FileMetadata::DateTime(
                        DateTime::parse_from_rfc3339("2002-02-21T23:30:39-08:00").unwrap()
                    )),
                }
            )
        );

        assert_eq!(
            header_line_content("lao 08f78e0addd5bf7b7aa8887e406493e75e8d2b55\n".into()).unwrap(),
            (
                "\n".into(),
                File {
                    name: "lao".to_string(),
                    meta: Some(FileMetadata::Other(
                        "08f78e0addd5bf7b7aa8887e406493e75e8d2b55"
                    ))
                }
            )
        );
    }

    #[test]
    fn test_headers() {
        let sample = "\
--- lao 2002-02-21 23:30:39.942229878 -0800
+++ tzu 2002-02-21 23:30:50.442260588 -0800\n";
        assert_eq!(
            headers(sample.into()).unwrap(),
            (
                "".into(),
                (
                    File {
                        name: "lao".to_string(),
                        meta: Some(FileMetadata::DateTime(
                            DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00")
                            .unwrap()
                        )),
                    },
                    File {
                        name: "tzu".to_string(),
                        meta: Some(FileMetadata::DateTime(
                            DateTime::parse_from_rfc3339("2002-02-21T23:30:50.442260588-08:00")
                            .unwrap()
                        )),
                    }
                )
            )
        );

        let sample2 = "\
--- lao
+++ tzu\n";
        assert_eq!(
            headers(sample2.into()).unwrap(),
            (
                "".into(),
                (
                    File {
                        name: "lao".to_string(),
                        meta: None,
                    },
                    File {
                        name: "tzu".to_string(),
                        meta: None,
                    }
                )
            )
        );

        let sample3 = "\
--- lao 08f78e0addd5bf7b7aa8887e406493e75e8d2b55
+++ tzu e044048282ce75186ecc7a214fd3d9ba478a2816\n";
        assert_eq!(
            headers(sample3.into()).unwrap(),
            (
                "".into(),
                (
                    File {
                        name: "lao".to_string(),
                        meta: Some(FileMetadata::Other(
                            "08f78e0addd5bf7b7aa8887e406493e75e8d2b55"
                        )),
                    },
                    File {
                        name: "tzu".to_string(),
                        meta: Some(FileMetadata::Other(
                            "e044048282ce75186ecc7a214fd3d9ba478a2816"
                        )),
                    }
                )
            )
        );
    }

    #[test]
    fn test_range() {
        assert_eq!(
            range("1,7".into()).unwrap(),
            ("".into(), Range { start: 1, count: 7 })
        );

        assert_eq!(
            range("2".into()).unwrap(),
            ("".into(), Range { start: 2, count: 1 })
        );
    }

    #[test]
    fn test_chunk_intro() {
        let sample = "@@ -1,7 +1,6 @@\n".into();
        assert_eq!(
            chunk_intro(sample).unwrap(),
            (
                "".into(),
                (Range { start: 1, count: 7 }, Range { start: 1, count: 6 }),
            )
        );
    }

    #[test]
    fn test_chunk() {
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
        assert_eq!(chunk(sample.into()).unwrap(), ("".into(), expected));
    }

    #[test]
    fn test_patch() {
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
                name: "lao".to_string(),
                meta: Some(FileMetadata::DateTime(
                    DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00").unwrap(),
                )),
            },
            new: File {
                name: "tzu".to_string(),
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
            no_newline: true,
        };

        assert_eq!(patch(sample.into()).unwrap(), ("".into(), expected));
    }
}
