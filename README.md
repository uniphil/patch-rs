# Patch

[![Checks](https://github.com/uniphil/patch-rs/actions/workflows/checks.yml/badge.svg)](https://github.com/uniphil/patch-rs/actions/workflows/checks.yml)
[![Crates.io Badge](https://img.shields.io/crates/v/patch.svg)](https://crates.io/crates/patch)
[![docs.rs](https://docs.rs/patch/badge.svg)](https://docs.rs/patch)
[![Lines of Code](https://tokei.rs/b1/github/uniphil/patch-rs)](https://github.com/uniphil/patch-rs)

Rust crate for parsing and producing patch files in the [Unified Format].

The parser attempts to be forgiving enough to be compatible with diffs produced
by programs like git. It accomplishes this by ignoring the additional code
context and information provided in the diff by those programs.

See the **[Documentation]** for more information and for examples.

[Unified Format]: https://www.gnu.org/software/diffutils/manual/html_node/Unified-Format.html
[Documentation]: https://docs.rs/patch
