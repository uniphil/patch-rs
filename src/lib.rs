//! Parse and produce patch files (diffs) in the [Unified Format].
//!
//! More info: Guido van Rossum also [honed down the spec][spec] a bit more.
//!
//! ## Example
//!
//! ```
//! # fn main() -> Result<(), patch::ParseError<'static>> {
//! // Make sure you add the `patch` crate to the `[dependencies]` key of your Cargo.toml file.
//! use patch::Patch;
//!
//! let sample = "\
//! --- before.py
//! +++ path/to/after.py
//! @@ -1,4 +1,4 @@
//! -bacon
//! -eggs
//! -ham
//! +python
//! +eggy
//! +hamster
//!  guido\n";
//!
//! let patch = Patch::from_str(sample)?;
//! assert_eq!(&patch.old.path, "before.py");
//! assert_eq!(&patch.new.path, "path/to/after.py");
//!
//! // Print out the parsed patch file
//! println!("{:#?}", patch);
//! # Ok(())
//! # }
//! ```
//!
//! [Unified Format]: https://www.gnu.org/software/diffutils/manual/html_node/Unified-Format.html
//! [spec]: http://www.artima.com/weblogs/viewpost.jsp?thread=164293

mod parser;
mod ast;

pub use ast::*;
pub use parser::ParseError;
