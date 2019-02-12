use chrono::{DateTime, FixedOffset};
use nom::*;
use std::str;

#[derive(Debug, Eq, PartialEq)]
pub struct Patch<'a> {
    pub old: File<'a>,
    pub new: File<'a>,
    pub hunks: Vec<Hunk<'a>>,
    pub no_newline: bool,
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

/*
 * Filename parsing
 */

named!(non_escape<char>, none_of!("\\\"\0\n\r\t"));

named!(
    escape<char>,
    do_parse!(
        tag!("\\") >>
        c: one_of!("\\\"0nrtb") >>
        (c)
    )
);

named!(
    unescape<String>,
    map!(many1!(alt!(non_escape | escape)), |chars: Vec<char>| chars
        .into_iter()
        .collect::<String>())
);

named!(quoted<String>, delimited!(tag!("\""), unescape, tag!("\"")));

named!(
    bare<String>,
    map_res!(
        map_res!(is_not!(" \n"), str::from_utf8),
        str::FromStr::from_str
    )
);

named!(fname<String>, alt!(quoted | bare));

/*
 * Header lines
 */

named!(
    header_line_content<File>,
    do_parse!(
        filename: fname
            >> opt!(space)
            >> after: map_res!(take_until!("\n"), str::from_utf8)
            >> (File {
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

named!(
    headers<(File, File)>,
    do_parse!(
        tag!("---")
            >> space
            >> oldfile: header_line_content
            >> newline
            >> tag!("+++")
            >> space
            >> newfile: header_line_content
            >> newline
            >> (oldfile, newfile)
    )
);

/*
 * Chunk intro
 */

named!(
    u64_digit<u64>,
    map_res!(
        map_res!(digit, str::from_utf8),
        str::FromStr::from_str
    )
);

named!(
    range<Range>,
    do_parse!(
        start: u64_digit
            >> count: opt!(complete!(preceded!(tag!(","), u64_digit)))
            >> (Range {
                start: start,
                count: count.unwrap_or(1)
            })
    )
);

named!(
    chunk_intro<(Range, Range)>,
    do_parse!(
        tag!("@@ -")
            >> old_range: range
            >> tag!(" +")
            >> new_range: range
            >> tag!(" @@")
            >> newline
            >> (old_range, new_range)
    )
);

/*
 * Chunk lines
 */

named!(
    chunk_line<Line>,
    alt!(
        map!(
            map_res!(
                preceded!(tag!("+"), take_until_and_consume!("\n")),
                str::from_utf8
            ),
            Line::Add
        ) | map!(
            map_res!(
                preceded!(tag!("-"), take_until_and_consume!("\n")),
                str::from_utf8
            ),
            Line::Remove
        ) | map!(
            map_res!(
                preceded!(tag!(" "), take_until_and_consume!("\n")),
                str::from_utf8
            ),
            Line::Context
        )
    )
);

named!(
    chunk<Hunk>,
    do_parse!(
        ranges: chunk_intro
            >> lines: many1!(chunk_line)
            >> ({
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
named!(chunks<Vec<Hunk>>, many1!(chunk));

/*
 * Trailing newline indicator
 */

named!(
    no_newline<bool>,
    map!(
        opt!(complete!(tag!("\\ No newline at end of file"))),
        |matched: Option<&[u8]>| matched.is_none()
    )
);

/*
 * The real deal
 */

named!(pub patch<Patch>,
    do_parse!(
           files: headers
        >> hunks: chunks
        >> no_newline: no_newline
        >> ( {
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

    #[test]
    fn test_unescape() {
        assert_eq!(
            unescape("file \\\"name\\\"".as_bytes()).unwrap(),
            ("".as_bytes(), "file \"name\"".to_string())
        );
    }

    #[test]
    fn test_quoted() {
        assert_eq!(
            quoted("\"file name\"".as_bytes()).unwrap(),
            ("".as_bytes(), "file name".to_string())
        );
    }

    #[test]
    fn test_bare() {
        assert_eq!(
            bare("file-name ".as_bytes()).unwrap(),
            (" ".as_bytes(), "file-name".to_string())
        );

        assert_eq!(
            bare("file-name\n".as_bytes()).unwrap(),
            ("\n".as_bytes(), "file-name".to_string())
        );
    }

    #[test]
    fn test_fname() {
        assert_eq!(
            fname("asdf ".as_bytes()).unwrap(),
            (" ".as_bytes(), "asdf".to_string())
        );

        assert_eq!(
            fname("\"asdf\" ".as_bytes()).unwrap(),
            (" ".as_bytes(), "asdf".to_string())
        );

        assert_eq!(
            fname("\"a s\\\"df\" ".as_bytes()).unwrap(),
            (" ".as_bytes(), "a s\"df".to_string())
        );
    }

    #[test]
    fn test_header_line_contents() {
        assert_eq!(
            header_line_content("lao\n".as_bytes()).unwrap(),
            (
                "\n".as_bytes(),
                File {
                    name: "lao".to_string(),
                    meta: None
                }
            )
        );

        assert_eq!(
            header_line_content("lao 2002-02-21 23:30:39.942229878 -0800\n".as_bytes()).unwrap(),
            (
                "\n".as_bytes(),
                File {
                    name: "lao".to_string(),
                    meta: Some(FileMetadata::DateTime(
                        DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00").unwrap()
                    )),
                }
            )
        );

        assert_eq!(
            header_line_content("lao 2002-02-21 23:30:39 -0800\n".as_bytes()).unwrap(),
            (
                "\n".as_bytes(),
                File {
                    name: "lao".to_string(),
                    meta: Some(FileMetadata::DateTime(
                        DateTime::parse_from_rfc3339("2002-02-21T23:30:39-08:00").unwrap()
                    )),
                }
            )
        );

        assert_eq!(
            header_line_content("lao 08f78e0addd5bf7b7aa8887e406493e75e8d2b55\n".as_bytes()).unwrap(),
            (
                "\n".as_bytes(),
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
        +++ tzu 2002-02-21 23:30:50.442260588 -0800\n"
        .as_bytes();
        assert_eq!(
            headers(sample).unwrap(),
            (
                "".as_bytes(),
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
        +++ tzu\n"
        .as_bytes();
        assert_eq!(
            headers(sample2).unwrap(),
            (
                "".as_bytes(),
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
        +++ tzu e044048282ce75186ecc7a214fd3d9ba478a2816\n"
        .as_bytes();
        assert_eq!(
            headers(sample3).unwrap(),
            (
                "".as_bytes(),
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
            range("1,7".as_bytes()).unwrap(),
            ("".as_bytes(), Range { start: 1, count: 7 })
        );

        assert_eq!(
            range("2".as_bytes()).unwrap(),
            ("".as_bytes(), Range { start: 2, count: 1 })
        );
    }

    #[test]
    fn test_chunk_intro() {
        let sample = "@@ -1,7 +1,6 @@\n".as_bytes();
        assert_eq!(
            chunk_intro(sample).unwrap(),
            (
                "".as_bytes(),
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
        And let there always be being,\n"
        .as_bytes();
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
        assert_eq!(chunk(sample).unwrap(), ("".as_bytes(), expected));
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

        assert_eq!(patch(sample.as_bytes()).unwrap(), ("".as_bytes(), expected));
    }
}
