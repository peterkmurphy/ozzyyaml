//! Plain, single-quoted and double-quoted scalar scanning.

use super::{scan_error, unterminated, Scanner};
use crate::chars::is_flow_indicator;
use crate::error::Result;
use crate::mark::Mark;

impl Scanner<'_> {
    /// Reads a single-line plain scalar, stopping at a newline, ` #` comment or
    /// `:` indicator (and, in flow context, `,[]{}`).
    pub(super) fn scan_plain(&mut self) -> String {
        let mut value = String::new();
        while let Some(c) = self.reader.peek() {
            let starts_comment = matches!(self.reader.peek2(), Some((' ' | '\t', '#')));
            let flow_stop = self.flow_level > 0 && is_flow_indicator(c);
            let is_value = c == ':' && self.is_plain_value_indicator();
            if c == '\n' || starts_comment || flow_stop || is_value {
                break;
            }
            value.push(c);
            self.reader.bump();
        }
        value.truncate(value.trim_end_matches([' ', '\t']).len());
        value
    }

    /// Like [`Self::is_value_indicator`] but without the quoted-scalar rule,
    /// which can't apply in the middle of a plain scalar.
    fn is_plain_value_indicator(&self) -> bool {
        self.indicator_followed_by_blank()
            || (self.flow_level > 0
                && self
                    .reader
                    .peek2()
                    .is_some_and(|(_, next)| is_flow_indicator(next)))
    }

    pub(super) fn scan_single_quoted(&mut self, start: Mark) -> Result<String> {
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

    pub(super) fn scan_double_quoted(&mut self, start: Mark) -> Result<String> {
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
