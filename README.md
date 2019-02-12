# Patch

Parse unified diffs with rust

[![Build Status](https://travis-ci.org/uniphil/patch-rs.svg?branch=master)](https://travis-ci.org/uniphil/patch-rs)
[![Crates.io Badge](https://img.shields.io/crates/v/patch.svg)](https://crates.io/crates/patch)

```rust
extern crate patch;
use patch::{parse};

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

if let Ok(patch) = parse(sample) {
    assert_eq!(&patch.old.name, "before.py");
    assert_eq!(&patch.new.name, "after.py");
} else {
    panic!("failed to parse sample patch");
}
```
