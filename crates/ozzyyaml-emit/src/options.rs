/// How collections are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmitStyle {
    /// Flow style (`[]` / `{}`).
    #[default]
    Flow,
    /// Block style.
    Block,
}

/// Options controlling YAML emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOptions {
    /// Collection style.
    pub style: EmitStyle,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            style: EmitStyle::Flow,
        }
    }
}
