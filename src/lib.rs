//! patch-rs is a parser library for [Unified Format]
//! (https://www.gnu.org/software/diffutils/manual/html_node/Unified-Format.html#Unified-Format)
//! diffs.
//!
//! GVR also honed down the spec a bit more:
//! http://www.artima.com/weblogs/viewpost.jsp?thread=164293

extern crate chrono;
extern crate nom;

mod error;
mod parser;
mod ast;

pub use ast::*;
pub use parser::ParseError;
pub use error::*;
