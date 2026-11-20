use std::collections::VecDeque;

use crate::chars::is_anchor_char;
use crate::error::{Error, Result};
use crate::event::ScalarStyle;
use crate::mark::Mark;
use crate::reader::Reader;
use crate::token::Token;

/// A token that may turn out to be a simple mapping key once `:` is seen.
#[derive(Debug, Clone, Copy)]
struct PossibleKey {
    token_number: usize,
    mark: Mark,
}

/// Converts characters from a [`Reader`] into [`Token`]s.
///
/// Recognises plain/quoted scalars, anchors, aliases, and block collections
/// (`- ` entries and single-line simple `key: value` pairs) and flow
/// collections (`[...]`, `{...}`). Indentation is tracked so
/// `BlockSequenceStart` / `BlockMappingStart` / `BlockEnd` tokens are emitted;
/// inside flow collections indentation is ignored. Tags, block scalars,
/// multi-line plain scalars and complex or flow-collection keys are not yet
/// supported.
#[derive(Debug)]
#[allow(dead_code)] // consumed by the parser in a later task
pub(crate) struct Scanner<'a> {
    reader: Reader<'a>,
    queue: VecDeque<Token>,
    tokens_taken: usize,
    /// Columns (1-based) of currently open block collections.
    indents: Vec<u32>,
    possible_key: Option<PossibleKey>,
    /// Number of currently open flow collections (`[` / `{`).
    flow_level: usize,
    /// Reader index just past the most recent quoted scalar; a `:` directly
    /// there is a value indicator inside flow collections (`{"a":1}`).
    quoted_end: Option<usize>,
    done: bool,
}

#[allow(dead_code)]
impl<'a> Scanner<'a> {
    pub fn new(reader: Reader<'a>) -> Self {
        Self {
            reader,
            queue: VecDeque::new(),
            tokens_taken: 0,
            indents: Vec::new(),
            possible_key: None,
            flow_level: 0,
            quoted_end: None,
            done: false,
        }
    }

    /// Returns the next token, or `None` once the input is exhausted.
    pub fn next_token(&mut self) -> Result<Option<Token>> {
        while self.need_more_tokens() {
            self.fetch_next_token()?;
        }
        let token = self.queue.pop_front();
        if token.is_some() {
            self.tokens_taken += 1;
        }
        Ok(token)
    }

    /// A queued token that may still become a key can't be released yet.
    fn need_more_tokens(&self) -> bool {
        !self.done
            && (self.queue.is_empty()
                || self
                    .possible_key
                    .is_some_and(|k| k.token_number == self.tokens_taken))
    }

    fn fetch_next_token(&mut self) -> Result<()> {
        self.skip_whitespace_and_comments();
        let mark = self.reader.mark();
        if self.possible_key.is_some_and(|k| k.mark.line != mark.line) {
            self.possible_key = None;
        }
        let Some(c) = self.reader.peek() else {
            self.unroll_indent(0, mark);
            self.possible_key = None;
            self.done = true;
            return Ok(());
        };
        if self.flow_level == 0 {
            self.unroll_indent(mark.column, mark);
        }
        match c {
            '[' | '{' => self.fetch_flow_collection_start(c, mark),
            ']' | '}' => self.fetch_flow_collection_end(c, mark)?,
            ',' if self.flow_level > 0 => self.fetch_flow_entry(mark),
            '-' if self.flow_level == 0 && self.indicator_followed_by_blank() => {
                self.fetch_block_entry(mark)
            }
            ':' if self.is_value_indicator(mark) => self.fetch_value(mark)?,
            '&' | '*' => self.fetch_anchor_or_alias(c, mark)?,
            '\'' => {
                self.save_possible_key(mark);
                let value = self.scan_single_quoted(mark)?;
                self.push_scalar(value, ScalarStyle::SingleQuoted, mark);
                self.quoted_end = Some(self.reader.mark().index);
            }
            '"' => {
                self.save_possible_key(mark);
                let value = self.scan_double_quoted(mark)?;
                self.push_scalar(value, ScalarStyle::DoubleQuoted, mark);
                self.quoted_end = Some(self.reader.mark().index);
            }
            _ => {
                self.save_possible_key(mark);
                let value = self.scan_plain();
                self.push_scalar(value, ScalarStyle::Plain, mark);
            }
        }
        Ok(())
    }

    fn push_scalar(&mut self, value: String, style: ScalarStyle, mark: Mark) {
        self.queue.push_back(Token::Scalar { value, style, mark });
    }

    /// True if the `:` at the reader position is a value indicator.
    fn is_value_indicator(&self, mark: Mark) -> bool {
        if self.indicator_followed_by_blank() {
            return true;
        }
        self.flow_level > 0
            && (self.quoted_end == Some(mark.index)
                || matches!(self.reader.peek2(), Some((_, ',' | '[' | ']' | '{' | '}'))))
    }

    fn fetch_flow_collection_start(&mut self, indicator: char, mark: Mark) {
        // Flow collections can't be implicit keys (yet), and a pending key
        // belongs to the enclosing level, so drop it.
        self.possible_key = None;
        self.reader.bump();
        self.flow_level += 1;
        self.queue.push_back(if indicator == '[' {
            Token::FlowSequenceStart { mark }
        } else {
            Token::FlowMappingStart { mark }
        });
    }

    fn fetch_flow_collection_end(&mut self, indicator: char, mark: Mark) -> Result<()> {
        if self.flow_level == 0 {
            return Err(scan_error(
                format!("unexpected '{indicator}' outside a flow collection"),
                mark,
            ));
        }
        self.possible_key = None;
        self.reader.bump();
        self.flow_level -= 1;
        self.queue.push_back(if indicator == ']' {
            Token::FlowSequenceEnd { mark }
        } else {
            Token::FlowMappingEnd { mark }
        });
        Ok(())
    }

    fn fetch_flow_entry(&mut self, mark: Mark) {
        self.possible_key = None;
        self.reader.bump();
        self.queue.push_back(Token::FlowEntry { mark });
    }

    /// True if the next character is followed by whitespace or end of input.
    fn indicator_followed_by_blank(&self) -> bool {
        match self.reader.peek2() {
            None => true,
            Some((_, next)) => matches!(next, ' ' | '\t' | '\n'),
        }
    }

    fn save_possible_key(&mut self, mark: Mark) {
        if self.possible_key.is_none() {
            self.possible_key = Some(PossibleKey {
                token_number: self.tokens_taken + self.queue.len(),
                mark,
            });
        }
    }

    /// Opens a block collection at `column` if it is deeper than the current one.
    fn roll_indent(&mut self, column: u32) -> bool {
        if self.indents.last().map_or(true, |&top| top < column) {
            self.indents.push(column);
            true
        } else {
            false
        }
    }

    /// Closes block collections indented deeper than `column`.
    fn unroll_indent(&mut self, column: u32, mark: Mark) {
        while self.indents.last().is_some_and(|&top| top > column) {
            self.indents.pop();
            self.queue.push_back(Token::BlockEnd { mark });
        }
    }

    fn fetch_block_entry(&mut self, mark: Mark) {
        if self.roll_indent(mark.column) {
            self.queue.push_back(Token::BlockSequenceStart { mark });
        }
        self.possible_key = None;
        self.reader.bump();
        self.queue.push_back(Token::BlockEntry { mark });
    }

    fn fetch_value(&mut self, mark: Mark) -> Result<()> {
        let Some(key) = self.possible_key.take() else {
            return Err(scan_error(
                "mapping value without a simple key on the same line".into(),
                mark,
            ));
        };
        let index = key.token_number - self.tokens_taken;
        self.queue.insert(index, Token::Key { mark: key.mark });
        if self.flow_level == 0 && self.roll_indent(key.mark.column) {
            self.queue
                .insert(index, Token::BlockMappingStart { mark: key.mark });
        }
        self.reader.bump();
        self.queue.push_back(Token::Value { mark });
        Ok(())
    }

    fn fetch_anchor_or_alias(&mut self, indicator: char, mark: Mark) -> Result<()> {
        self.save_possible_key(mark);
        self.reader.bump();
        let mut name = String::new();
        while let Some(c) = self.reader.peek().filter(|&c| is_anchor_char(c)) {
            name.push(c);
            self.reader.bump();
        }
        if name.is_empty() {
            return Err(scan_error(
                "anchor/alias name must not be empty".into(),
                mark,
            ));
        }
        self.queue.push_back(if indicator == '&' {
            Token::Anchor { name, mark }
        } else {
            Token::Alias { name, mark }
        });
        Ok(())
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

    /// Reads a single-line plain scalar, stopping at a newline, ` #` comment or
    /// `:` indicator (and, in flow context, `,[]{}`).
    fn scan_plain(&mut self) -> String {
        let mut value = String::new();
        while let Some(c) = self.reader.peek() {
            let starts_comment = matches!(self.reader.peek2(), Some((' ' | '\t', '#')));
            let flow_stop = self.flow_level > 0 && matches!(c, ',' | '[' | ']' | '{' | '}');
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
                && matches!(self.reader.peek2(), Some((_, ',' | '[' | ']' | '{' | '}'))))
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

    fn tokens(input: &str) -> Vec<Token> {
        let mut s = scanner(input);
        let mut out = Vec::new();
        while let Some(t) = s.next_token().unwrap() {
            out.push(t);
        }
        out
    }

    fn kinds(input: &str) -> Vec<String> {
        tokens(input)
            .into_iter()
            .map(|t| match t {
                Token::Scalar { value, .. } => format!("S:{value}"),
                Token::Anchor { name, .. } => format!("&{name}"),
                Token::Alias { name, .. } => format!("*{name}"),
                other => format!("{other:?}").split(' ').next().unwrap().to_string(),
            })
            .collect()
    }

    #[test]
    fn scans_unicode_and_emoji_anchor_names() {
        assert_eq!(kinds("&汉 x"), ["&汉", "S:x"]);
        assert_eq!(kinds("*😀"), ["*😀"]);
        assert_eq!(kinds("*a,b"), ["*a", "S:,b"]);
    }

    #[test]
    fn empty_anchor_name_is_scan_error() {
        assert!(matches!(
            scanner("& x").next_token(),
            Err(Error::Scan { .. })
        ));
    }

    #[test]
    fn scans_block_mapping_tokens() {
        assert_eq!(
            kinds("a: 1\nb: &x 2\n"),
            [
                "BlockMappingStart",
                "Key",
                "S:a",
                "Value",
                "S:1",
                "Key",
                "S:b",
                "Value",
                "&x",
                "S:2",
                "BlockEnd"
            ]
        );
    }

    #[test]
    fn scans_block_sequence_tokens() {
        assert_eq!(
            kinds("- a\n- *x\n"),
            [
                "BlockSequenceStart",
                "BlockEntry",
                "S:a",
                "BlockEntry",
                "*x",
                "BlockEnd"
            ]
        );
    }

    #[test]
    fn nested_collections_close_in_order() {
        assert_eq!(
            kinds("a:\n  b: c\nd: e"),
            [
                "BlockMappingStart",
                "Key",
                "S:a",
                "Value",
                "BlockMappingStart",
                "Key",
                "S:b",
                "Value",
                "S:c",
                "BlockEnd",
                "Key",
                "S:d",
                "Value",
                "S:e",
                "BlockEnd"
            ]
        );
    }

    #[test]
    fn colon_without_following_blank_stays_in_plain_scalar() {
        assert_eq!(kinds("a:b"), ["S:a:b"]);
    }

    #[test]
    fn invalid_double_quote_escape_is_scan_error() {
        let err = scanner(r#""\q""#).next_token().unwrap_err();
        assert!(matches!(err, Error::Scan { .. }));
    }
}
