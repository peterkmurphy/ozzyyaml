#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! YAML 1.2.2 event parser for ozzyyaml (Milestone A).

mod error;
mod limits;
mod mark;
mod reader;

pub use error::{Error, LimitKind, Result};
pub use limits::Limits;
pub use mark::Mark;
pub use reader::Reader;

/// Placeholder so the crate compiles before real API lands.
pub fn crate_name() -> &'static str {
    "ozzyyaml-parser"
}
