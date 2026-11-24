use std::collections::VecDeque;

use crate::chars::is_anchor_char;
use crate::error::{Error, Result};
use crate::event::ScalarStyle;
use crate::mark::Mark;
use crate::reader::Reader;
use crate::token::Token;

mod block;
mod document;
mod flow;
mod scalar;
#[cfg(test)]
mod tests;

/// A token that may turn out to be a simple mapping key once `:` is seen.
#[derive(Debug, Clone, Copy)]
struct PossibleKey {
    token_number: usize,
    mark: Mark,
}

/// Converts characters from a [`Reader`] into [`Token`]s.
///
/// Recognises plain/quoted scalars, anchors, aliases, tags, `%` directives,
/// `---` / `...` document markers, and block collections
/// (`- ` entries and single-line simple `key: value` pairs) and flow
/// collections (`[...]`, `{...}`). Indentation is tracked so
/// `BlockSequenceStart` / `BlockMappingStart` / `BlockEnd` tokens are emitted;
/// inside flow collections indentation is ignored. Multi-line plain and quoted
/// scalars are folded per YAML 1.2.2 §8.1.3. Block scalars (`|` / `>`) and
/// explicit keys (`?`) are not yet supported.
#[derive(Debug)]
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
    /// Line of the most recent block-level `:` value indicator.
    value_line: Option<u32>,
    done: bool,
}

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
            value_line: None,
            done: false,
        }
    }

    /// Returns the reader's current source position.
    ///
    /// This can be past the last token yielded, because scanning looks ahead.
    pub(crate) fn mark(&self) -> Mark {
        self.reader.mark()
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
            debug_assert!(self.reader.is_eof());
            self.unroll_indent(0, mark);
            self.possible_key = None;
            self.done = true;
            return Ok(());
        };
        if let Some(marker) = self.document_marker(mark) {
            self.fetch_document_marker(marker, mark);
            return Ok(());
        }
        if self.flow_level == 0 {
            self.unroll_indent(mark.column, mark);
        }
        match c {
            '%' if mark.column == 1 && self.flow_level == 0 => self.fetch_directive(mark),
            '!' => self.fetch_tag(mark)?,
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

    /// True if the next character is followed by whitespace or end of input.
    fn indicator_followed_by_blank(&self) -> bool {
        match self.reader.peek2() {
            None => true,
            Some((_, next)) => matches!(next, ' ' | '\t' | '\n'),
        }
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
        // Properties must be separated from the following node content.
        if let Some(c) = self.reader.peek() {
            let ok = matches!(c, ' ' | '\t' | '\n')
                || crate::chars::is_flow_indicator(c)
                || (c == ':' && self.is_value_indicator(self.reader.mark()));
            if !ok {
                return Err(scan_error(
                    "expected whitespace after anchor/alias name".into(),
                    self.reader.mark(),
                ));
            }
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
}

fn unterminated(kind: &str, mark: Mark) -> Error {
    scan_error(format!("unterminated {kind} scalar"), mark)
}

fn scan_error(message: String, mark: Mark) -> Error {
    Error::Scan { message, mark }
}
