extern crate patch;
extern crate chrono;

#[test]
fn test_parse() {
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
    match patch::parse(sample) {
        Ok(p) => {
            assert_eq!(&p.old, "before.py");
            assert_eq!(&p.new, "after.py");
            assert_eq!(p.no_newline, true);
        },
        Err(e) => {
            println!("{:?}", e);
            panic!("failed to parse sample patch");
        },
    }
}

#[test]
fn test_parse_no_newline() {
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
\\ No newline at end of file";
    match patch::parse(sample) {
        Ok(p) => {
            assert_eq!(&p.old, "before.py");
            assert_eq!(&p.new, "after.py");
            assert_eq!(p.no_newline, false);
        },
        Err(e) => {
            println!("{:?}", e);
            panic!("failed to parse sample patch");
        },
    }
}

#[test]
fn test_parse_timestamps() {
    let sample = "\
--- before.py 2002-02-21 23:30:39.942229878 -0800
+++ after.py 2002-02-21 23:30:50.442260588 -0800
@@ -1,4 +1,4 @@
-bacon
-eggs
-ham
+python
+eggy
+hamster
 guido\n";
    match patch::parse(sample) {
        Ok(p) => {
            assert_eq!(&p.old, "before.py");
            assert_eq!(&p.new, "after.py");
            assert_eq!(p.old_timestamp.unwrap(), chrono::DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00").unwrap());
            assert_eq!(p.new_timestamp.unwrap(), chrono::DateTime::parse_from_rfc3339("2002-02-21T23:30:50.442260588-08:00").unwrap());
            assert_eq!(p.no_newline, true);
        },
        Err(e) => {
            println!("{:?}", e);
            panic!("failed to parse sample patch");
        },
    }
}
