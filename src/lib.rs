//! Parse and produce patch files (diffs) in the [Unified Format].
//!
//! The format is not fully specified, but people like Guido van Rossum [have done the work][spec]
//! to figure out the details.
//!
//! The parser attempts to be forgiving enough to be compatible with diffs produced by programs
//! like git. It accomplishes this by ignoring the additional code context and information provided
//! in the diff by those programs.
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

pub use parser::ParseError;
pub use ast::*;
