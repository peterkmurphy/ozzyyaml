use crate::error::{Error, Result};
use crate::event::ScalarStyle;
use crate::mark::Mark;
use crate::reader::Reader;
use crate::token::Token;

/// Converts characters from a [`Reader`] into [`Token`]s.
///
/// Currently recognises only plain, single-quoted and double-quoted scalars;
/// whitespace and `#` comments between tokens are skipped.
#[derive(Debug)]
#[allow(dead_code)] // consumed by the parser in a later task
pub(crate) struct Scanner<'a> {
    reader: Reader<'a>,
}

#[allow(dead_code)]
impl<'a> Scanner<'a> {
    pub fn new(reader: Reader<'a>) -> Self {
        Self { reader }
    }

    /// Returns the next token, or `None` once the input is exhausted.
    pub fn next_token(&mut self) -> Result<Option<Token>> {
        self.skip_whitespace_and_comments();
        let mark = self.reader.mark();
        let (value, style) = match self.reader.peek() {
            None => return Ok(None),
            Some('\'') => (self.scan_single_quoted(mark)?, ScalarStyle::SingleQuoted),
            Some('"') => (self.scan_double_quoted(mark)?, ScalarStyle::DoubleQuoted),
            Some(_) => (self.scan_plain(), ScalarStyle::Plain),
        };
        Ok(Some(Token::Scalar { value, style, mark }))
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(c) = self.reader.peek() {
            match c {
                ' ' | '\t' | '\n' => {
                    self.reader.bump();
                }
                '#' => self.skip_to_end_of_line(),
                _ => break,
            }
        }
    }

    fn skip_to_end_of_line(&mut self) {
        while self.reader.peek().is_some_and(|c| c != '\n') {
            self.reader.bump();
        }
    }

    /// Reads a single-line plain scalar, stopping at a newline or ` #` comment.
    fn scan_plain(&mut self) -> String {
        let mut value = String::new();
        while let Some(c) = self.reader.peek() {
            let starts_comment = matches!(self.reader.peek2(), Some((' ' | '\t', '#')));
            if c == '\n' || starts_comment {
                break;
            }
            value.push(c);
            self.reader.bump();
        }
        value.truncate(value.trim_end_matches([' ', '\t']).len());
        value
    }

    fn scan_single_quoted(&mut self, start: Mark) -> Result<String> {
        self.reader.bump(); // opening quote
        let mut value = String::new();
        loop {
            match self.reader.bump() {
                None => return Err(unterminated("single-quoted", start)),
                Some('\'') if self.reader.peek() == Some('\'') => {
                    self.reader.bump();
                    value.push('\'');
                }
                Some('\'') => return Ok(value),
                Some(c) => value.push(c),
            }
        }
    }

    fn scan_double_quoted(&mut self, start: Mark) -> Result<String> {
        self.reader.bump(); // opening quote
        let mut value = String::new();
        loop {
            match self.reader.bump() {
                None => return Err(unterminated("double-quoted", start)),
                Some('"') => return Ok(value),
                Some('\\') => value.push(self.scan_escape()?),
                Some(c) => value.push(c),
            }
        }
    }

    /// Decodes the character after a backslash (simple escapes only).
    fn scan_escape(&mut self) -> Result<char> {
        let mark = self.reader.mark();
        let decoded = match self.reader.bump() {
            Some('0') => '\0',
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            Some(c @ ('"' | '\\' | '/' | ' ')) => c,
            Some(c) => return Err(scan_error(format!("invalid escape '\\{c}'"), mark)),
            None => return Err(scan_error("unterminated escape".into(), mark)),
        };
        Ok(decoded)
    }
}

#[allow(dead_code)]
fn unterminated(kind: &str, mark: Mark) -> Error {
    scan_error(format!("unterminated {kind} scalar"), mark)
}

#[allow(dead_code)]
fn scan_error(message: String, mark: Mark) -> Error {
    Error::Scan { message, mark }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::event::ScalarStyle;
    use crate::limits::Limits;

    fn scanner(input: &str) -> Scanner<'_> {
        Scanner::new(Reader::new(input, Limits::default()).unwrap())
    }

    fn scalar(s: &mut Scanner<'_>) -> (String, ScalarStyle) {
        match s.next_token().unwrap().expect("token") {
            Token::Scalar { value, style, .. } => (value, style),
            other => panic!("expected scalar, got {other:?}"),
        }
    }

    #[test]
    fn scans_plain_scalar() {
        let mut s = scanner("hello");
        assert_eq!(scalar(&mut s), ("hello".into(), ScalarStyle::Plain));
        assert_eq!(s.next_token().unwrap(), None);
    }

    #[test]
    fn scans_double_quoted_scalar() {
        let mut s = scanner("\"hello\"");
        assert_eq!(scalar(&mut s), ("hello".into(), ScalarStyle::DoubleQuoted));
        assert_eq!(s.next_token().unwrap(), None);
    }

    #[test]
    fn scans_single_quoted_scalar() {
        let mut s = scanner("'hello'");
        assert_eq!(scalar(&mut s), ("hello".into(), ScalarStyle::SingleQuoted));
        assert_eq!(s.next_token().unwrap(), None);
    }

    #[test]
    fn skips_leading_spaces_and_records_mark() {
        let mut s = scanner("  hi");
        match s.next_token().unwrap().unwrap() {
            Token::Scalar { mark, .. } => {
                assert_eq!((mark.index, mark.line, mark.column), (2, 1, 3));
            }
            other => panic!("expected scalar, got {other:?}"),
        }
    }

    #[test]
    fn skips_comments_and_blank_lines() {
        let mut s = scanner("# note\n\n  hi # trailing\n");
        assert_eq!(scalar(&mut s), ("hi".into(), ScalarStyle::Plain));
        assert_eq!(s.next_token().unwrap(), None);
    }

    #[test]
    fn plain_scalar_keeps_inner_spaces_and_hash_without_space() {
        let mut s = scanner("a b#c");
        assert_eq!(scalar(&mut s), ("a b#c".into(), ScalarStyle::Plain));
    }

    #[test]
    fn empty_and_blank_input_yield_no_tokens() {
        assert_eq!(scanner("").next_token().unwrap(), None);
        assert_eq!(scanner("  # only a comment").next_token().unwrap(), None);
    }

    #[test]
    fn single_quote_doubled_is_literal_quote() {
        let mut s = scanner("'it''s'");
        assert_eq!(scalar(&mut s), ("it's".into(), ScalarStyle::SingleQuoted));
    }

    #[test]
    fn double_quoted_handles_simple_escapes() {
        let mut s = scanner(r#""a\tb\n\"c\\""#);
        assert_eq!(
            scalar(&mut s),
            ("a\tb\n\"c\\".into(), ScalarStyle::DoubleQuoted)
        );
    }

    #[test]
    fn unterminated_quoted_scalars_are_scan_errors() {
        for input in ["'abc", "\"abc"] {
            let err = scanner(input).next_token().unwrap_err();
            assert!(matches!(err, Error::Scan { .. }), "{input}: {err:?}");
        }
    }

    #[test]
    fn invalid_double_quote_escape_is_scan_error() {
        let err = scanner(r#""\q""#).next_token().unwrap_err();
        assert!(matches!(err, Error::Scan { .. }));
    }
}
