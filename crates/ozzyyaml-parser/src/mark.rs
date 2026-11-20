use std::fmt;

/// Source position. `line` and `column` are 1-based; `index` is 0-based byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    /// Byte offset from start of input.
    pub index: usize,
    /// 1-based line number.
    pub line: u32,
    /// 1-based column number (Unicode scalar count on the line is fine for v1).
    pub column: u32,
}

impl Default for Mark {
    /// Start-of-stream position (`line`/`column` are 1-based).
    fn default() -> Self {
        Self {
            index: 0,
            line: 1,
            column: 1,
        }
    }
}

impl fmt::Display for Mark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {} column {} (index {})",
            self.line, self.column, self.index
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_display_is_human() {
        let m = Mark {
            index: 10,
            line: 2,
            column: 5,
        };
        assert_eq!(m.to_string(), "line 2 column 5 (index 10)");
    }

    #[test]
    fn mark_default_is_start_of_stream() {
        assert_eq!(
            Mark::default(),
            Mark {
                index: 0,
                line: 1,
                column: 1,
            }
        );
    }
}
