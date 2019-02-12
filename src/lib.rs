//! patch-rs is a parser library for [Unified Format] diffs.
//!
//! More info: Guido van Rossum also [honed down the spec][spec] a bit more.
//!
//! [Unified Format]: https://www.gnu.org/software/diffutils/manual/html_node/Unified-Format.html#Unified-Format
//! [spec]: http://www.artima.com/weblogs/viewpost.jsp?thread=164293

extern crate chrono;
extern crate nom;

mod error;
mod parser;
mod ast;

pub use ast::*;
pub use parser::ParseError;
pub use error::*;
