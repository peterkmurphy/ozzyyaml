//! Temporary emission stubs. Task 2 replaces these with a real emitter.

use crate::error::{Error, Result};
use crate::options::EmitOptions;
use ozzyyaml::Value;

/// Emit one [`Value`] using [`EmitOptions::default`].
pub fn to_string(value: &Value) -> Result<String> {
    to_string_with_options(value, &EmitOptions::default())
}

/// Emit one [`Value`] with explicit options.
///
/// Returns an error until Task 2 implements emission.
pub fn to_string_with_options(_value: &Value, _opts: &EmitOptions) -> Result<String> {
    Err(Error::Emit {
        message: "not implemented".into(),
    })
}

/// Emit every document in `docs` using [`EmitOptions::default`].
pub fn dump_all(docs: &[Value]) -> Result<String> {
    dump_all_with_options(docs, &EmitOptions::default())
}

/// Emit every document in `docs` with explicit options.
///
/// Returns an error until Task 2 implements emission.
pub fn dump_all_with_options(_docs: &[Value], _opts: &EmitOptions) -> Result<String> {
    Err(Error::Emit {
        message: "not implemented".into(),
    })
}
