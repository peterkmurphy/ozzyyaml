use ozzyyaml_parser::{LimitKind, Mark};
use thiserror::Error;

/// Errors from parsing or composing YAML into [`crate::Value`].
#[derive(Debug, Error, PartialEq)]
pub enum Error {
    /// Underlying event-parser failure.
    #[error(transparent)]
    Parse(#[from] ozzyyaml_parser::Error),
    /// Alias name was not defined by an earlier anchor.
    #[error("undefined alias `{name}` at {mark}")]
    UndefinedAlias {
        /// Alias name that was not found.
        name: String,
        /// Source position of the alias event.
        mark: Mark,
    },
    /// A configured resource limit was exceeded.
    #[error("limit exceeded: {kind:?} (limit {limit}) at {mark}")]
    LimitExceeded {
        /// Which limit fired.
        kind: LimitKind,
        /// Configured limit value.
        limit: usize,
        /// Source position of the event that registered the anchor.
        mark: Mark,
    },
    /// Input bytes were not valid UTF-8 (no other encoding BOM matched).
    #[error("invalid UTF-8 at byte {valid_up_to}")]
    InvalidUtf8 {
        /// Byte index of the first invalid sequence.
        valid_up_to: usize,
    },
    /// Input bytes used a detected encoding but could not be decoded.
    #[error("invalid encoding: {message}")]
    InvalidEncoding {
        /// Human-readable explanation.
        message: String,
    },
    /// `from_str` requires exactly one document.
    #[error("{message}")]
    DocumentCount {
        /// Human-readable explanation.
        message: String,
    },
    /// Composer hit an unexpected event sequence.
    #[error("compose error: {message}")]
    Compose {
        /// Human-readable explanation.
        message: String,
    },
    /// A Core-schema tag was applied to a value that does not match it.
    #[error("invalid value for tag `{tag}` at {mark}")]
    InvalidTaggedValue {
        /// Resolved tag URI (e.g. `tag:yaml.org,2002:int`).
        tag: String,
        /// Source position of the scalar.
        mark: Mark,
    },
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;
