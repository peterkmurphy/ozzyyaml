use crate::mark::Mark;
use thiserror::Error;

/// Which limit was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    /// Input longer than [`crate::Limits::max_input_bytes`].
    InputBytes,
    /// Nesting deeper than [`crate::Limits::max_nesting_depth`].
    NestingDepth,
    /// More distinct anchors than [`crate::Limits::max_anchors`].
    Anchors,
}

/// Parser failure with a source [`Mark`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// A resource limit was exceeded.
    #[error("limit exceeded: {kind:?} (limit {limit}) at {mark}")]
    LimitExceeded {
        /// Which limit was exceeded.
        kind: LimitKind,
        /// The configured limit value.
        limit: usize,
        /// Where the limit was hit.
        mark: Mark,
    },
    /// Scanner (lexical) error.
    #[error("scan error: {message} at {mark}")]
    Scan {
        /// Human-readable scan failure.
        message: String,
        /// Where the scan failed.
        mark: Mark,
    },
    /// Parser (syntactic) error.
    #[error("parse error: {message} at {mark}")]
    Parse {
        /// Human-readable parse failure.
        message: String,
        /// Where the parse failed.
        mark: Mark,
    },
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mark::Mark;

    #[test]
    fn limit_exceeded_display() {
        let err = Error::LimitExceeded {
            kind: LimitKind::InputBytes,
            limit: 2,
            mark: Mark {
                index: 10,
                line: 2,
                column: 5,
            },
        };
        assert_eq!(
            err.to_string(),
            "limit exceeded: InputBytes (limit 2) at line 2 column 5 (index 10)"
        );
    }

    #[test]
    fn scan_error_display() {
        let err = Error::Scan {
            message: "unexpected character".into(),
            mark: Mark {
                index: 10,
                line: 2,
                column: 5,
            },
        };
        assert_eq!(
            err.to_string(),
            "scan error: unexpected character at line 2 column 5 (index 10)"
        );
    }

    #[test]
    fn parse_error_display() {
        let err = Error::Parse {
            message: "expected mapping".into(),
            mark: Mark {
                index: 10,
                line: 2,
                column: 5,
            },
        };
        assert_eq!(
            err.to_string(),
            "parse error: expected mapping at line 2 column 5 (index 10)"
        );
    }
}
