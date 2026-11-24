/// Resource limits for untrusted input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Reject inputs longer than this many bytes.
    pub max_input_bytes: usize,
    /// Max collection / indent nesting depth.
    pub max_nesting_depth: usize,
    /// Max distinct anchors registered (primary DoS brake; enforced in compose,
    /// tracked here for a shared API).
    pub max_anchors: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: 256 * 1024 * 1024,
            max_nesting_depth: 10_000,
            max_anchors: 100_000,
        }
    }
}

impl Limits {
    /// Builder-style override.
    #[must_use]
    pub fn max_input_bytes(mut self, n: usize) -> Self {
        self.max_input_bytes = n;
        self
    }
    /// Builder-style override.
    #[must_use]
    pub fn max_nesting_depth(mut self, n: usize) -> Self {
        self.max_nesting_depth = n;
        self
    }
    /// Builder-style override.
    #[must_use]
    pub fn max_anchors(mut self, n: usize) -> Self {
        self.max_anchors = n;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::Reader;

    #[test]
    fn default_limits_are_raised() {
        let l = Limits::default();
        assert_eq!(l.max_input_bytes, 256 * 1024 * 1024);
        assert_eq!(l.max_nesting_depth, 10_000);
        assert_eq!(l.max_anchors, 100_000);
    }

    #[test]
    fn limits_builders_override_defaults() {
        let l = Limits::default()
            .max_input_bytes(2)
            .max_nesting_depth(3)
            .max_anchors(4);
        assert_eq!(l.max_input_bytes, 2);
        assert_eq!(l.max_nesting_depth, 3);
        assert_eq!(l.max_anchors, 4);
    }

    #[test]
    fn tiny_input_byte_limit_still_rejects() {
        let limits = Limits::default().max_input_bytes(8);
        let err = Reader::new("123456789", limits).unwrap_err();
        assert!(matches!(
            err,
            crate::error::Error::LimitExceeded {
                kind: crate::error::LimitKind::InputBytes,
                limit: 8,
                ..
            }
        ));
    }
}
