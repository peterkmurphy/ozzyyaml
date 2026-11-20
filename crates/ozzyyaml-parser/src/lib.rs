#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! YAML 1.2.2 event parser for ozzyyaml (Milestone A).

mod error;
mod event;
mod limits;
mod mark;
mod reader;
mod token;

pub use error::{Error, LimitKind, Result};
pub use event::{Event, ScalarStyle};
pub use limits::Limits;
pub use mark::Mark;

/// Placeholder so the crate compiles before real API lands.
pub fn crate_name() -> &'static str {
    "ozzyyaml-parser"
}
