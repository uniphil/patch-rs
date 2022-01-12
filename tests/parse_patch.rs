use chrono::DateTime;
use patch::{File, FileMetadata, ParseError, Patch};

use pretty_assertions::assert_eq;

#[test]
fn test_parse() -> Result<(), ParseError<'static>> {
    let sample = "\
--- before.py
+++ after.py
@@ -1,4 +1,4 @@
-bacon
-eggs
-ham
+python
+eggy
+hamster
 guido\n";
    let patch = Patch::from_single(sample)?;
    assert_eq!(
        patch.old,
        File {
            path: "before.py".into(),
            meta: None
        }
    );
    assert_eq!(
        patch.new,
        File {
            path: "after.py".into(),
            meta: None
        }
    );
    assert_eq!(patch.end_newline, true);

    assert_eq!(format!("{}\n", patch), sample);

    Ok(())
}

#[test]
fn test_parse_no_newline_indicator() -> Result<(), ParseError<'static>> {
    let sample = "\
--- before.py
+++ after.py
@@ -1,4 +1,4 @@
-bacon
-eggs
-ham
+python
+eggy
+hamster
 guido
\\ No newline at end of file\n";
    let patch = Patch::from_single(sample)?;
    assert_eq!(
        patch.old,
        File {
            path: "before.py".into(),
            meta: None
        }
    );
    assert_eq!(
        patch.new,
        File {
            path: "after.py".into(),
            meta: None
        }
    );
    assert_eq!(patch.end_newline, false);

    assert_eq!(format!("{}\n", patch), sample);

    Ok(())
}

#[test]
fn test_parse_timestamps() -> Result<(), ParseError<'static>> {
    let sample = "\
--- before.py 2002-02-21 23:30:39.942229878 -0800
+++ after.py 2002-02-21 23:30:50 -0800
@@ -1,4 +1,4 @@
-bacon
-eggs
-ham
+python
+eggy
+hamster
 guido
\\ No newline at end of file";
    let patch = Patch::from_single(sample)?;
    assert_eq!(
        patch.old,
        File {
            path: "before.py".into(),
            meta: Some(FileMetadata::DateTime(
                DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00").unwrap()
            )),
        }
    );
    assert_eq!(
        patch.new,
        File {
            path: "after.py".into(),
            meta: Some(FileMetadata::DateTime(
                DateTime::parse_from_rfc3339("2002-02-21T23:30:50-08:00").unwrap()
            )),
        }
    );
    assert_eq!(patch.end_newline, false);

    // to_string() uses Display but adds no trailing newline
    assert_eq!(patch.to_string(), sample);

    Ok(())
}

#[test]
fn test_parse_other() -> Result<(), ParseError<'static>> {
    let sample = "\
--- before.py 08f78e0addd5bf7b7aa8887e406493e75e8d2b55
+++ after.py e044048282ce75186ecc7a214fd3d9ba478a2816
@@ -1,4 +1,4 @@
-bacon
-eggs
-ham
+python
+eggy
+hamster
 guido\n";
    let patch = Patch::from_single(sample)?;
    assert_eq!(
        patch.old,
        File {
            path: "before.py".into(),
            meta: Some(FileMetadata::Other(
                "08f78e0addd5bf7b7aa8887e406493e75e8d2b55".into()
            )),
        }
    );
    assert_eq!(
        patch.new,
        File {
            path: "after.py".into(),
            meta: Some(FileMetadata::Other(
                "e044048282ce75186ecc7a214fd3d9ba478a2816".into()
            )),
        }
    );
    assert_eq!(patch.end_newline, true);

    assert_eq!(format!("{}\n", patch), sample);

    Ok(())
}

#[test]
fn test_parse_escaped() -> Result<(), ParseError<'static>> {
    let sample = "\
--- before.py \"asdf \\\\ \\n \\t \\0 \\r \\\" \"
+++ \"My Work/after.py\" \"My project is cool! Wow!!; SELECT * FROM USERS;\"
@@ -1,4 +1,4 @@
-bacon
-eggs
-ham
+python
+eggy
+hamster
 guido\n";
    let patch = Patch::from_single(sample)?;
    assert_eq!(
        patch.old,
        File {
            path: "before.py".into(),
            meta: Some(FileMetadata::Other("asdf \\ \n \t \0 \r \" ".into())),
        }
    );
    assert_eq!(
        patch.new,
        File {
            path: "My Work/after.py".into(),
            meta: Some(FileMetadata::Other(
                "My project is cool! Wow!!; SELECT * FROM USERS;".into()
            )),
        }
    );
    assert_eq!(patch.end_newline, true);

    assert_eq!(format!("{}\n", patch), sample);

    Ok(())
}

#[test]
fn test_parse_triple_plus_minus() -> Result<(), ParseError<'static>> {
    // Our parser has some hacky rules to make sure that lines starting with +++ or --- aren't
    // interpreted as regular addition/removal lines that could be part of a hunk. This test checks
    // to make sure that code is working.
    let sample = r#"--- main.c
+++ main.c
@@ -1,4 +1,7 @@
+#include<stdio.h>
+
 int main() {
 double a;
---a;
+++a;
+printf("%d\n", a);
 }
"#;
    let patches = Patch::from_multiple(sample).unwrap();
    assert_eq!(patches.len(), 1);

    let patch = &patches[0];
    assert_eq!(
        patch.old,
        File {
            path: "main.c".into(),
            meta: None
        }
    );
    assert_eq!(
        patch.new,
        File {
            path: "main.c".into(),
            meta: None
        }
    );
    assert_eq!(patch.end_newline, true);

    assert_eq!(patch.hunks.len(), 1);
    assert_eq!(patch.hunks[0].lines.len(), 8);

    assert_eq!(format!("{}\n", patch), sample);

    Ok(())
}

//FIXME: This test should NOT panic. When we have more sophisticated chunk line parsing that
// actually takes the hunk ranges into account, the #[should_panic] annotation should be removed.
// See the FIXME comment on top of the chunk_line parser.
#[test]
#[should_panic]
fn test_parse_triple_plus_minus_hack() {
    // Our parser has some hacky rules to make sure that lines starting with +++ or --- aren't
    // interpreted as regular addition/removal lines that could be part of a hunk. This test
    // demonstrates that these rules are not foolproof. The only differences between this test and
    // test_parse_triple_plus_minus are `--- a` and `+++ a` vs `---a` and `+++a`. In either case,
    // we should be able to determine that those lines do not start a new patch based on the ranges
    // provided for the hunk.
    let sample = r#"--- main.c
+++ main.c
@@ -1,4 +1,7 @@
+#include<stdio.h>
+
 int main() {
 double a;
--- a;
+++ a;
+printf("%d\n", a);
 }
"#;
    let patches = Patch::from_multiple(sample).unwrap();
    assert_eq!(patches.len(), 1);

    let patch = &patches[0];
    assert_eq!(
        patch.old,
        File {
            path: "main.c".into(),
            meta: None
        }
    );
    assert_eq!(
        patch.new,
        File {
            path: "main.c".into(),
            meta: None
        }
    );
    assert_eq!(patch.end_newline, true);

    assert_eq!(patch.hunks.len(), 1);
    assert_eq!(patch.hunks[0].lines.len(), 8);

    assert_eq!(format!("{}\n", patch), sample);
}
