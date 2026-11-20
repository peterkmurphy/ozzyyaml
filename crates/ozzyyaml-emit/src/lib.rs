#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Emit [`ozzyyaml::Value`] trees as YAML 1.2.2 text.

mod anchors;
mod emit;
mod error;
mod options;
mod scalar;

pub use emit::{dump_all, dump_all_with_options, to_string, to_string_with_options};
pub use error::{Error, Result};
pub use options::{EmitOptions, EmitStyle};
