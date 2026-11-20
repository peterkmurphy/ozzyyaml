use thiserror::Error;

/// Errors from emitting a [`ozzyyaml::Value`] tree as YAML text.
#[derive(Debug, Error, PartialEq)]
pub enum Error {
    /// Emission failed.
    #[error("emit error: {message}")]
    Emit {
        /// Human-readable explanation.
        message: String,
    },
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;
