#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! YAML 1.2.2 documents: compose parser events into `Rc`-shared [`Value`] trees
//! using the Core schema.

mod compose;
mod encoding;
mod error;
mod limits;
mod schema;
mod value;

pub use compose::{
    from_slice, from_slice_with_limits, from_str, from_str_with_limits, load_all,
    load_all_from_slice, load_all_from_slice_with_limits, load_all_with_limits,
};
pub use error::{Error, Result};
pub use limits::Limits;
pub use ozzyyaml_parser::LimitKind;
pub use value::Value;
