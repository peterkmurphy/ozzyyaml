/// Resource limits for untrusted input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Reject inputs longer than this many bytes.
    pub max_input_bytes: usize,
    /// Max collection / indent nesting depth.
    pub max_nesting_depth: usize,
    /// Max distinct anchors registered (primary DoS brake; enforced in Milestone B compose, tracked for API stability here).
    pub max_anchors: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: 16 * 1024 * 1024,
            max_nesting_depth: 1_000,
            max_anchors: 1_024,
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

    #[test]
    fn default_limits_are_conservative() {
        let l = Limits::default();
        assert!(l.max_anchors <= 10_000);
        assert!(l.max_nesting_depth <= 1_000);
        assert!(l.max_input_bytes <= 64 * 1024 * 1024);
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
}
