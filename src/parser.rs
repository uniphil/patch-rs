use std;
use nom::*;
use chrono::{FixedOffset, DateTime};


#[derive(Debug, Eq, PartialEq)]
pub struct Patch<'a> {
    pub old: String,
    pub new: String,
    pub old_timestamp: Option<DateTime<FixedOffset>>,
    pub new_timestamp: Option<DateTime<FixedOffset>>,
    pub hunks: Vec<Hunk<'a>>,
    pub no_newline: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Hunk<'a> {
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

named!(non_escape<char>,
    none_of!("\\\"\0\n\r\t")
);

named!(escape<char>,
    chain!(
        tag!("\\") ~
        c: one_of!("\\\"0nrtb"),
        || c
    )
);

named!(unescape<String>,
    map!(
        many1!( alt!( non_escape | escape ) ),
        |chars: Vec<char>| chars.into_iter().collect::<String>()
    )
);

named!(quoted<String>,
    chain!(
        tag!("\"") ~
        unescaped: unescape ~
        tag!("\"") ,
        || unescaped
    )
);

named!(bare<String>,
    map_res!(
        map_res!(
            is_not!(" \n"),
            std::str::from_utf8
        ),
        std::str::FromStr::from_str
    )
);

named!(fname<String>, alt!(quoted | bare));


/*
 * Header lines
 */

named!(header_line_content<(String, Option<DateTime<FixedOffset>>)>,
    chain!(
        filename: fname ~
        opt!(space) ~
        after: map_res!(
            take_until!("\n"),
            std::str::from_utf8
        ) ,

        || (filename, DateTime::parse_from_str(after, "%F %T%.f %z").or_else(|_| DateTime::parse_from_str(after, "%F %T %z")).ok())
    )
);

named!(headers<((String, Option<DateTime<FixedOffset>>), (String, Option<DateTime<FixedOffset>>))>,
    chain!(
            tag!("---") ~
            space ~
        oldfile: header_line_content ~
            newline ~
            tag!("+++") ~
            space ~
        newfile: header_line_content ~
            newline ,

        || (oldfile, newfile)
    )
);

/*
 * Chunk intro
 */

named!(range<u8>,
    chain!(
        digit ~
        opt!(chain!(
            tag!(",") ~
            digit ,
            || 0
        )) ,
        || 0
    )
);

named!(chunk_intro<u8>,
    chain!(
        tag!("@@ -") ~
        range ~
        tag!(" +") ~
        range ~
        tag!(" @@") ~
        newline ,
        || 0
    )
);

/*
 * Chunk lines
 */

named!(chunk_line<Line>,
    alt!(
        map!(
            map_res!(
                chain!(
                    tag!("+") ~
                    line: take_until_and_consume!("\n") ,
                    || line
                ),
                std::str::from_utf8
            ),
            Line::Add
        ) |
        map!(
            map_res!(
                chain!(
                    tag!("-") ~
                    line: take_until_and_consume!("\n") ,
                    || line
                ),
                std::str::from_utf8
            ),
            Line::Remove
        ) |
        map!(
            map_res!(
                chain!(
                    tag!(" ") ~
                    line: take_until_and_consume!("\n") ,
                    || line
                ),
                std::str::from_utf8
            ),
            Line::Context
        )
    )
);

named!(chunk<Hunk>,
    chain!(
        chunk_intro ~
        lines: many1!(chunk_line) ,
        || Hunk { lines: lines }
    )
);

named!(chunks<Vec<Hunk> >, many0!(chunk));


/*
 * Trailing newline indicator
 */

named!(no_newline<bool>,
    map!(
        // complete!(take!(1)),
        // |e: Result<_, _>| e.is_ok()
        opt!(complete!(tag!("\\ No newline at end of file"))),
        |matched: Option<&[u8]>| matched.is_none()
    )
);


/*
 * The real deal
 */

named!(pub patch<(((String, Option<DateTime<FixedOffset>>), (String, Option<DateTime<FixedOffset>>)), Vec<Hunk>, bool)>,
    chain!(
        files: headers ~
        hunks: chunks ~
        no_newline: no_newline ,
        || (files, hunks, no_newline)
    )
);


#[test]
fn test_unescape() {
    assert_eq!(unescape("file \\\"name\\\"".as_bytes()),
        IResult::Done("".as_bytes(), "file \"name\"".to_string()));
}

#[test]
fn test_quoted() {
    assert_eq!(quoted("\"file name\"".as_bytes()),
        IResult::Done("".as_bytes(), "file name".to_string()));
}

#[test]
fn test_bare() {
    assert_eq!(bare("file-name ".as_bytes()),
        IResult::Done(" ".as_bytes(), "file-name".to_string()));

    assert_eq!(bare("file-name\n".as_bytes()),
        IResult::Done("\n".as_bytes(), "file-name".to_string()));
}

#[test]
fn test_fname() {
    assert_eq!(fname("asdf ".as_bytes()),
        IResult::Done(" ".as_bytes(), "asdf".to_string()));

    assert_eq!(fname("\"asdf\" ".as_bytes()),
        IResult::Done(" ".as_bytes(), "asdf".to_string()));

    assert_eq!(fname("\"a s\\\"df\" ".as_bytes()),
        IResult::Done(" ".as_bytes(), "a s\"df".to_string()));
}

#[test]
fn test_header_line_contents() {
    assert_eq!(header_line_content("lao 2002-02-21 23:30:39.942229878 -0800\n".as_bytes()),
        IResult::Done("\n".as_bytes(), ("lao".to_string(), DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00").ok())));

    assert_eq!(header_line_content("lao 2002-02-21 23:30:39 -0800\n".as_bytes()),
        IResult::Done("\n".as_bytes(), ("lao".to_string(), DateTime::parse_from_rfc3339("2002-02-21T23:30:39-08:00").ok())));

    assert_eq!(header_line_content("lao\n".as_bytes()),
        IResult::Done("\n".as_bytes(), ("lao".to_string(), None)));
}

#[test]
fn test_headers() {
    let sample = "\
--- lao 2002-02-21 23:30:39.942229878 -0800
+++ tzu 2002-02-21 23:30:50.442260588 -0800\n".as_bytes();
    assert_eq!(headers(sample),
        IResult::Done("".as_bytes(), (("lao".to_string(), DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00").ok()),
                                      ("tzu".to_string(), DateTime::parse_from_rfc3339("2002-02-21T23:30:50.442260588-08:00").ok()))));

    let sample2 = "\
--- lao
+++ tzu\n".as_bytes();
    assert_eq!(headers(sample2),
        IResult::Done("".as_bytes(), (("lao".to_string(), None),
                                      ("tzu".to_string(), None))));
}

#[test]
fn test_chunk_intro() {
    let sample = "@@ -1,7 +1,6 @@\n".as_bytes();
    assert_eq!(chunk_intro(sample),
        IResult::Done("".as_bytes(), 0))
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
 And let there always be being,\n".as_bytes();
    let expected = Hunk { lines: vec![
        Line::Remove("The Way that can be told of is not the eternal Way;"),
        Line::Remove("The name that can be named is not the eternal name."),
        Line::Context("The Nameless is the origin of Heaven and Earth;"),
        Line::Remove("The Named is the mother of all things."),
        Line::Add("The named is the mother of all things."),
        Line::Add(""),
        Line::Context("Therefore let there always be non-being,"),
        Line::Context("  so we may see their subtlety,"),
        Line::Context("And let there always be being,"),
    ] };
    assert_eq!(chunk(sample),
        IResult::Done("".as_bytes(), expected))
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

    let expected = (
        (("lao".to_string(), DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00").ok()),
         ("tzu".to_string(), DateTime::parse_from_rfc3339("2002-02-21T23:30:50.442260588-08:00").ok())),
        vec![
            Hunk { lines: vec! [
                Line::Remove("The Way that can be told of is not the eternal Way;"),
                Line::Remove("The name that can be named is not the eternal name."),
                Line::Context("The Nameless is the origin of Heaven and Earth;"),
                Line::Remove("The Named is the mother of all things."),
                Line::Add("The named is the mother of all things."),
                Line::Add(""),
                Line::Context("Therefore let there always be non-being,"),
                Line::Context("  so we may see their subtlety,"),
                Line::Context("And let there always be being,"),
            ] },
            Hunk { lines: vec! [
                Line::Context("The two are the same,"),
                Line::Context("But after they are produced,"),
                Line::Context("  they have different names."),
                Line::Add("They both may be called deep and profound."),
                Line::Add("Deeper and more profound,"),
                Line::Add("The door of all subtleties!"),
            ] },
        ],
        true,
    );

    assert_eq!(patch(sample.as_bytes()),
        IResult::Done("".as_bytes(), expected));
}
