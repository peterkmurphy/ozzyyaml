use crate::error::{Error, LimitKind, Result};
use crate::limits::Limits;
use crate::mark::Mark;

/// UTF-8 input cursor with byte offsets and one-based source positions.
#[derive(Debug)]
pub struct Reader<'a> {
    input: &'a str,
    index: usize,
    line: u32,
    column: u32,
}

impl<'a> Reader<'a> {
    /// Creates a reader, rejecting input that exceeds the configured byte limit.
    pub fn new(input: &'a str, limits: Limits) -> Result<Self> {
        if input.len() > limits.max_input_bytes {
            return Err(Error::LimitExceeded {
                kind: LimitKind::InputBytes,
                limit: limits.max_input_bytes,
                mark: Mark {
                    index: 0,
                    line: 1,
                    column: 1,
                },
            });
        }

        // A UTF-8 BOM is metadata rather than YAML content, so begin after it.
        let index = if input.starts_with('\u{FEFF}') {
            '\u{FEFF}'.len_utf8()
        } else {
            0
        };

        Ok(Self {
            input,
            index,
            line: 1,
            column: 1,
        })
    }

    /// Returns the current source position.
    pub fn mark(&self) -> Mark {
        Mark {
            index: self.index,
            line: self.line,
            column: self.column,
        }
    }

    /// Returns the next character without consuming it.
    pub fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    /// Returns the next two characters without consuming them.
    pub fn peek2(&self) -> Option<(char, char)> {
        let mut chars = self.rest().chars();
        Some((chars.next()?, chars.next()?))
    }

    /// Consumes and returns the next character.
    pub fn bump(&mut self) -> Option<char> {
        let current = self.peek()?;

        // Normalize CRLF to one logical newline and consume both bytes at once.
        if current == '\r' && self.rest().starts_with("\r\n") {
            self.index += 2;
            self.line += 1;
            self.column = 1;
            return Some('\n');
        }

        self.index += current.len_utf8();
        if current == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(current)
    }

    /// Returns whether all input has been consumed.
    pub fn is_eof(&self) -> bool {
        self.index == self.input.len()
    }

    /// Returns the unconsumed input.
    pub fn rest(&self) -> &'a str {
        &self.input[self.index..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Limits;

    #[test]
    fn strips_utf8_bom() {
        let mut r = Reader::new("\u{FEFF}a", Limits::default()).unwrap();
        assert_eq!(r.bump(), Some('a'));
    }

    #[test]
    fn tracks_line_and_column() {
        let mut r = Reader::new("a\nb", Limits::default()).unwrap();
        assert_eq!(
            r.mark(),
            Mark {
                index: 0,
                line: 1,
                column: 1
            }
        );
        r.bump();
        r.bump(); // newline
        assert_eq!(r.mark().line, 2);
        assert_eq!(r.mark().column, 1);
    }

    #[test]
    fn rejects_oversize_input() {
        let limits = Limits::default().max_input_bytes(2);
        let err = Reader::new("abcd", limits).unwrap_err();
        assert!(matches!(
            err,
            crate::error::Error::LimitExceeded {
                kind: crate::error::LimitKind::InputBytes,
                ..
            }
        ));
    }
}
