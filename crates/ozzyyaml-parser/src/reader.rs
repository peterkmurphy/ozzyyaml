use crate::error::{Error, LimitKind, Result};
use crate::limits::Limits;
use crate::mark::Mark;

/// UTF-8 input cursor with byte offsets and one-based source positions.
#[derive(Debug)]
pub(crate) struct Reader<'a> {
    input: &'a str,
    index: usize,
    line: u32,
    column: u32,
}

/// YAML 1.2.2 §5.4 line breaks: `\r\n` and bare `\r` normalize to `\n`.
fn next_logical_char(input: &str) -> Option<(char, usize)> {
    if input.starts_with("\r\n") {
        Some(('\n', 2))
    } else {
        let current = input.chars().next()?;
        Some((
            if current == '\r' { '\n' } else { current },
            current.len_utf8(),
        ))
    }
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
        next_logical_char(self.rest()).map(|(current, _)| current)
    }

    /// Returns the next two characters without consuming them.
    pub fn peek2(&self) -> Option<(char, char)> {
        let rest = self.rest();
        let (first, first_len) = next_logical_char(rest)?;
        let (second, _) = next_logical_char(&rest[first_len..])?;
        Some((first, second))
    }

    /// Consumes and returns the next character.
    pub fn bump(&mut self) -> Option<char> {
        let (current, byte_len) = next_logical_char(self.rest())?;
        self.index += byte_len;
        if current == '\n' {
            self.line = self.line.saturating_add(1);
            self.column = 1;
        } else {
            self.column = self.column.saturating_add(1);
        }
        Some(current)
    }

    /// Returns the unconsumed input.
    pub fn rest(&self) -> &'a str {
        // `index` always sits on a UTF-8 boundary (BOM skip or scalar widths).
        &self.input[self.index..]
    }

    /// Returns whether all input has been consumed.
    pub fn is_eof(&self) -> bool {
        self.rest().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Limits;

    #[test]
    fn strips_utf8_bom() {
        let mut r = Reader::new("\u{FEFF}a", Limits::default()).unwrap();
        assert_eq!(
            r.mark(),
            Mark {
                index: 3,
                line: 1,
                column: 1
            }
        );
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
    fn normalizes_crlf() {
        let mut r = Reader::new("\r\nx", Limits::default()).unwrap();
        assert_eq!(r.peek(), Some('\n'));
        assert_eq!(r.bump(), Some('\n'));
        assert_eq!(
            r.mark(),
            Mark {
                index: 2,
                line: 2,
                column: 1
            }
        );
        assert_eq!(r.rest(), "x");
    }

    #[test]
    fn normalizes_bare_carriage_return() {
        let mut r = Reader::new("\rx", Limits::default()).unwrap();
        assert_eq!(r.peek(), Some('\n'));
        assert_eq!(r.bump(), Some('\n'));
        assert_eq!(
            r.mark(),
            Mark {
                index: 1,
                line: 2,
                column: 1
            }
        );
    }

    #[test]
    fn peek_and_peek2_match_bump() {
        let mut r = Reader::new("\r\n\r汉", Limits::default()).unwrap();
        assert_eq!(r.peek(), Some('\n'));
        assert_eq!(r.peek2(), Some(('\n', '\n')));
        assert_eq!(r.bump(), Some('\n'));
        assert_eq!(r.peek(), Some('\n'));
        assert_eq!(r.peek2(), Some(('\n', '汉')));
        assert_eq!(r.bump(), Some('\n'));
        assert_eq!(r.peek(), Some('汉'));
        assert_eq!(r.bump(), Some('汉'));
        assert_eq!(r.peek(), None);
        assert_eq!(r.peek2(), None);
        assert_eq!(r.bump(), None);
    }

    #[test]
    fn reports_eof() {
        let mut r = Reader::new("a", Limits::default()).unwrap();
        assert!(!r.is_eof());
        r.bump();
        assert!(r.is_eof());
    }

    #[test]
    fn returns_unconsumed_input() {
        let mut r = Reader::new("a汉", Limits::default()).unwrap();
        assert_eq!(r.rest(), "a汉");
        r.bump();
        assert_eq!(r.rest(), "汉");
    }

    #[test]
    fn advances_mark_by_utf8_bytes_and_one_column() {
        let mut r = Reader::new("汉", Limits::default()).unwrap();
        assert_eq!(r.bump(), Some('汉'));
        assert_eq!(
            r.mark(),
            Mark {
                index: 3,
                line: 1,
                column: 2
            }
        );
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
