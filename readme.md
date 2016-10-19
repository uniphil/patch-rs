# Patch

Parse unified diffs with rust

```rust
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

if let Ok(patch) = patch::parse(sample) {
    assert_eq!(&patch.old, "before.py");
    assert_eq!(&patch.new, "after.py");
} else {
    panic!("failed to parse sample patch");
}
```
