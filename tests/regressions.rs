use patch::{Line, ParseError, Patch};

use pretty_assertions::assert_eq;

#[test]
fn hunk_header_context_is_not_a_line_15() -> Result<(), ParseError<'static>> {
    let sample = "\
--- old.txt
+++ new.txt
@@ -0,0 +0,0 @@ spoopadoop
 x
";
    let patch = Patch::from_single(sample)?;
    assert_eq!(patch.hunks[0].lines, [Line::Context("x")]);
    Ok(())
}

#[test]
fn crlf_breaks_stuff_17() -> Result<(), ParseError<'static>> {
    let sample = "\
--- old.txt\r
+++ new.txt\r
@@ -0,0 +0,0 @@ spoopadoop\r
 x\r
";
    let patch = Patch::from_single(sample)?;
    assert_eq!(patch.hunks[0].lines, [Line::Context("x")]);
    Ok(())
}
