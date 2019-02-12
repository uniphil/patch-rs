use patch::Patch;

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
    match Patch::from_str(sample) {
        Ok(p) => {
            assert_eq!(
                p.old,
                patch::File {
                    name: "before.py".to_string(),
                    meta: None,
                }
            );
            assert_eq!(
                p.new,
                patch::File {
                    name: "after.py".to_string(),
                    meta: None,
                }
            );
            assert_eq!(p.no_newline, true);
        }
        Err(e) => {
            println!("{:?}", e);
            panic!("failed to parse sample patch");
        }
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
    match Patch::from_str(sample) {
        Ok(p) => {
            assert_eq!(
                p.old,
                patch::File {
                    name: "before.py".to_string(),
                    meta: None,
                }
            );
            assert_eq!(
                p.new,
                patch::File {
                    name: "after.py".to_string(),
                    meta: None,
                }
            );
            assert_eq!(p.no_newline, false);
        }
        Err(e) => {
            println!("{:?}", e);
            panic!("failed to parse sample patch");
        }
    }
}

#[test]
fn test_parse_timestamps() {
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
 guido\n";
    match Patch::from_str(sample) {
        Ok(p) => {
            assert_eq!(
                p.old,
                patch::File {
                    name: "before.py".to_string(),
                    meta: Some(patch::FileMetadata::DateTime(
                        chrono::DateTime::parse_from_rfc3339("2002-02-21T23:30:39.942229878-08:00")
                            .unwrap()
                    )),
                }
            );
            assert_eq!(
                p.new,
                patch::File {
                    name: "after.py".to_string(),
                    meta: Some(patch::FileMetadata::DateTime(
                        chrono::DateTime::parse_from_rfc3339("2002-02-21T23:30:50-08:00").unwrap()
                    )),
                }
            );
            assert_eq!(p.no_newline, true);
        }
        Err(e) => {
            println!("{:?}", e);
            panic!("failed to parse sample patch");
        }
    }
}

#[test]
fn test_parse_other() {
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
    match Patch::from_str(sample) {
        Ok(p) => {
            assert_eq!(
                p.old,
                patch::File {
                    name: "before.py".to_string(),
                    meta: Some(patch::FileMetadata::Other(
                        "08f78e0addd5bf7b7aa8887e406493e75e8d2b55"
                    )),
                }
            );
            assert_eq!(
                p.new,
                patch::File {
                    name: "after.py".to_string(),
                    meta: Some(patch::FileMetadata::Other(
                        "e044048282ce75186ecc7a214fd3d9ba478a2816"
                    )),
                }
            );
            assert_eq!(p.no_newline, true);
        }
        Err(e) => {
            println!("{:?}", e);
            panic!("failed to parse sample patch");
        }
    }
}
