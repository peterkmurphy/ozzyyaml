/// Resource limits for untrusted input (compose + parse).
///
/// Field defaults match [`ozzyyaml_parser::Limits`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Reject inputs longer than this many bytes (parser).
    pub max_input_bytes: usize,
    /// Max collection nesting depth (parser).
    pub max_nesting_depth: usize,
    /// Max distinct anchors registered per document (composer).
    pub max_anchors: usize,
}

impl Default for Limits {
    fn default() -> Self {
        let p = ozzyyaml_parser::Limits::default();
        Self {
            max_input_bytes: p.max_input_bytes,
            max_nesting_depth: p.max_nesting_depth,
            max_anchors: p.max_anchors,
        }
    }
}

impl Limits {
    /// Convert to parser limits.
    pub(crate) fn to_parser(self) -> ozzyyaml_parser::Limits {
        ozzyyaml_parser::Limits {
            max_input_bytes: self.max_input_bytes,
            max_nesting_depth: self.max_nesting_depth,
            max_anchors: self.max_anchors,
        }
    }

    /// Override max input size.
    #[must_use]
    pub fn max_input_bytes(mut self, n: usize) -> Self {
        self.max_input_bytes = n;
        self
    }

    /// Override max nesting depth.
    #[must_use]
    pub fn max_nesting_depth(mut self, n: usize) -> Self {
        self.max_nesting_depth = n;
        self
    }

    /// Override max anchors.
    #[must_use]
    pub fn max_anchors(mut self, n: usize) -> Self {
        self.max_anchors = n;
        self
    }
}
