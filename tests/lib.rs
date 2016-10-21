extern crate patch;

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
