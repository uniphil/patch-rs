use patch::{File, Hunk, Line, ParseError, Patch, Range};

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
@@ -0,0 +0,0 @@
 x\r
";
    let patch = Patch::from_single(sample)?;
    assert_eq!(
        patch,
        Patch {
            old: File {
                path: "old.txt".into(),
                meta: None
            },
            new: File {
                path: "new.txt".into(),
                meta: None
            },
            hunks: vec![Hunk {
                old_range: Range { start: 0, count: 0 },
                new_range: Range { start: 0, count: 0 },
                range_hint: "",
                lines: vec![Line::Context("x")],
            }],
            end_newline: true,
        }
    );
    Ok(())
}
