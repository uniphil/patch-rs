#[macro_use]
extern crate nom;

use nom::*;

// unified diff format:
// https://www.gnu.org/software/diffutils/manual/html_node/Detailed-Unified.html
// http://www.artima.com/weblogs/viewpost.jsp?thread=164293

named!(nonEscape<char>,
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
        many1!( alt!( nonEscape | escape ) ),
        |chars: Vec<char>| chars.into_iter().collect::<String>()
    )
);

#[test]
fn test_unescape() {
    assert_eq!(unescape("file \\\"name\\\"".as_bytes()),
        IResult::Done("".as_bytes(), "file \"name\"".to_string()));
}

named!(quoted<String>,
    chain!(
        tag!("\"") ~
        unescaped: unescape ~
        tag!("\"") ,
        || unescaped
    )
);

#[test]
fn test_quoted() {
    assert_eq!(quoted("\"file name\"".as_bytes()),
        IResult::Done("".as_bytes(), "file name".to_string()));
}


named!(bare<String>,
    map_res!(
        map_res!(
            take_until!(" "),
            std::str::from_utf8
        ),
        std::str::FromStr::from_str
    )
);

#[test]
fn test_bare() {
    assert_eq!(bare("file-name ".as_bytes()),
        IResult::Done(" ".as_bytes(), "file-name".to_string()));
}

named!(fname<String>, alt!(quoted | bare));

#[test]
fn test_fname() {
    assert_eq!(fname("asdf ".as_bytes()),
        IResult::Done(" ".as_bytes(), "asdf".to_string()));

    assert_eq!(fname("\"asdf\" ".as_bytes()),
        IResult::Done(" ".as_bytes(), "asdf".to_string()));

    assert_eq!(fname("\"a s\\\"df\" ".as_bytes()),
        IResult::Done(" ".as_bytes(), "a s\"df".to_string()));
}


named!(header_line_content<String>,
    chain!(
        filename: fname ~
        opt!(
            chain!(
                multispace ~  // gnu says space, guido says tab
                take_until!(" ") ~ space ~  // date
                take_until!(" ") ~ space ~ // time
                is_a!("+-") ~ digit ,  // timezone
                || 0
            )
        ) ,

        || filename
    )
);

#[test]
fn test_header_line_contents() {
    assert_eq!(header_line_content("lao 2002-02-21 23:30:39.942229878 -0800".as_bytes()),
        IResult::Done("".as_bytes(), "lao".to_string()));
}

named!(headers<(String, String)>,
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

#[test]
fn test_headers() {
    let sample = "\
--- lao 2002-02-21 23:30:39.942229878 -0800
+++ tzu 2002-02-21 23:30:50.442260588 -0800\n".as_bytes();
    assert_eq!(headers(sample),
        IResult::Done("".as_bytes(), ("lao".to_string(), "tzu".to_string())));
}

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

#[test]
fn test_chunk_intro() {
    let sample = "@@ -1,7 +1,6 @@\n".as_bytes();
    assert_eq!(chunk_intro(sample),
        IResult::Done("".as_bytes(), 0))
}

named!(chunk_line<u8>,
    chain!(
        one_of!(" +-") ~
        take_until_and_consume!("\n") ,
        || 0
    )
);

named!(chunk<u8>,
    chain!(
        chunk_intro ~
        many1!(chunk_line) ,
        || 0
    )
);

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
    assert_eq!(chunk(sample),
        IResult::Done("".as_bytes(), 0))
}

named!(chunks<Vec<u8> >, many0!(chunk));

named!(patch<(String, String)>,
    chain!(
        filenames: headers ~
        chunks ,
        || filenames
    )
);

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
+The door of all subtleties!\n".as_bytes();

    assert_eq!(patch(sample),
        IResult::Done("".as_bytes(), ("lao".to_string(), "tzu".to_string())));
}
