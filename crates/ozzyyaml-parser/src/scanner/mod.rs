use std::collections::VecDeque;

use crate::chars::is_anchor_char;
use crate::error::{Error, Result};
use crate::event::ScalarStyle;
use crate::mark::Mark;
use crate::reader::Reader;
use crate::token::Token;

mod block;
mod block_scalar;
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
/// Recognises plain/quoted scalars, block scalars (`|` / `>`), anchors, aliases,
/// tags, `%` directives, `---` / `...` document markers, and block collections
/// (`- ` entries, explicit `?` keys, and simple `key: value` pairs) and flow
/// collections (`[...]`, `{...}`). Indentation is tracked so
/// `BlockSequenceStart` / `BlockMappingStart` / `BlockEnd` tokens are emitted;
/// inside flow collections indentation is ignored. Multi-line plain and quoted
/// scalars are folded per YAML 1.2.2 §8.1.3.
#[derive(Debug)]
pub(crate) struct Scanner<'a> {
    reader: Reader<'a>,
    queue: VecDeque<Token>,
    tokens_taken: usize,
    /// Columns (1-based) of currently open block collections.
    indents: Vec<u32>,
    /// Possible simple key at the current flow level.
    possible_key: Option<PossibleKey>,
    /// Possible simple keys of enclosing flow levels (outer → inner).
    possible_key_stack: Vec<Option<PossibleKey>>,
    /// Number of currently open flow collections (`[` / `{`).
    flow_level: usize,
    /// The previous token was a JSON-like node, so a `:` at the start of the
    /// next token (even after whitespace, a comment or a line break) is an
    /// adjacent value indicator inside flow collections (`{ "a"\n :b }`).
    after_json_node: bool,
    /// Whether a simple key (or block `?` / `-` / empty-key `:`) may start here.
    simple_key_allowed: bool,
    /// A JSON-like node was just scanned (consumed by the next token fetch).
    json_node_pending: bool,
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
            possible_key_stack: Vec::new(),
            flow_level: 0,
            after_json_node: false,
            simple_key_allowed: true,
            json_node_pending: false,
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
            && (self.queue.is_empty() || self.pending_simple_key_at_front())
    }

    /// True if the next token to yield is still a candidate simple key
    /// (current level or an enclosing flow level).
    fn pending_simple_key_at_front(&self) -> bool {
        let at_front = |k: &PossibleKey| k.token_number == self.tokens_taken;
        self.possible_key.as_ref().is_some_and(at_front)
            || self.possible_key_stack.iter().flatten().any(at_front)
    }

    fn fetch_next_token(&mut self) -> Result<()> {
        self.skip_whitespace_and_comments()?;
        let mark = self.reader.mark();
        self.after_json_node = std::mem::take(&mut self.json_node_pending);
        if self.possible_key.is_some_and(|k| k.mark.line != mark.line) {
            self.possible_key = None;
        }
        let Some(c) = self.reader.peek() else {
            debug_assert!(self.reader.is_eof());
            self.unroll_indent(0, mark);
            self.possible_key = None;
            self.possible_key_stack.clear();
            self.done = true;
            return Ok(());
        };
        // Document markers at column 1 are illegal inside flow collections.
        if mark.column == 1 && self.flow_level > 0 {
            let rest = self.reader.rest();
            if let Some(marker) = ["---", "..."].into_iter().find(|m| rest.starts_with(m)) {
                let after_ok = match rest[3..].chars().next() {
                    None | Some(' ' | '\t' | '\n' | '\r') => true,
                    Some(_) => false,
                };
                if after_ok {
                    return Err(scan_error(
                        format!("document marker `{marker}` inside flow collection"),
                        mark,
                    ));
                }
            }
        }
        if let Some(marker) = self.document_marker(mark) {
            self.fetch_document_marker(marker, mark)?;
            return Ok(());
        }
        if self.flow_level == 0 {
            self.unroll_indent(mark.column, mark);
        }
        match c {
            '\u{FEFF}' => {
                return Err(scan_error(
                    "byte order mark inside document".into(),
                    mark,
                ));
            }
            '%' if mark.column == 1 && self.flow_level == 0 => self.fetch_directive(mark),
            '!' => self.fetch_tag(mark)?,
            '[' | '{' => self.fetch_flow_collection_start(c, mark),
            ']' | '}' => self.fetch_flow_collection_end(c, mark)?,
            ',' if self.flow_level > 0 => self.fetch_flow_entry(mark),
            '-' if self.flow_level == 0 && self.indicator_followed_by_blank() => {
                self.fetch_block_entry(mark)
            }
            '?' if self.indicator_followed_by_blank() => self.fetch_key(mark)?,
            ':' if self.is_value_indicator(mark) => self.fetch_value(mark)?,
            '&' | '*' => self.fetch_anchor_or_alias(c, mark)?,
            '\'' => {
                self.save_possible_key(mark);
                let value = self.scan_single_quoted(mark)?;
                self.push_scalar(value, ScalarStyle::SingleQuoted, mark);
                self.mark_json_node_end();
                self.simple_key_allowed = false;
            }
            '"' => {
                self.save_possible_key(mark);
                let value = self.scan_double_quoted(mark)?;
                self.push_scalar(value, ScalarStyle::DoubleQuoted, mark);
                self.mark_json_node_end();
                self.simple_key_allowed = false;
            }
            '|' | '>' if self.flow_level == 0 => {
                let (value, style) = self.scan_block_scalar(mark)?;
                self.push_scalar(value, style, mark);
                self.simple_key_allowed = true;
            }
            '#' if self.after_json_node => {
                return Err(scan_error(
                    "comment must be separated from previous token by whitespace".into(),
                    mark,
                ));
            }
            _ => {
                if crate::chars::is_reserved(c) {
                    return Err(scan_error(
                        "reserved indicators can't start a plain scalar".into(),
                        mark,
                    ));
                }
                self.save_possible_key(mark);
                let start_line = mark.line;
                let value = self.scan_plain()?;
                self.push_scalar(value, ScalarStyle::Plain, mark);
                // Multiline plain scalars allow a following simple key (libyaml).
                self.simple_key_allowed =
                    self.flow_level == 0 && self.reader.mark().line != start_line;
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
        self.after_json_node = false;
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
        self.simple_key_allowed = false;
        self.queue.push_back(if indicator == '&' {
            Token::Anchor { name, mark }
        } else {
            Token::Alias { name, mark }
        });
        Ok(())
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<()> {
        loop {
            // Block indentation may use spaces only (§8.2.1); a tab in the
            // line prefix before block content is a scan error (suite 4EJS).
            // Leading tabs before a flow collection are allowed (6CA3, Q5MG).
            if self.flow_level == 0 && self.reader.mark().column == 1 {
                while self.reader.peek() == Some(' ') {
                    self.reader.bump();
                }
                if self.reader.peek() == Some('\t') {
                    let mut rest = self.reader.rest();
                    while rest.starts_with('\t') || rest.starts_with(' ') {
                        rest = &rest[1..];
                    }
                    let flow_ok = rest.starts_with('[') || rest.starts_with('{');
                    if !flow_ok {
                        return Err(scan_error(
                            "tabs cannot be used as indentation".into(),
                            self.reader.mark(),
                        ));
                    }
                }
            }
            let mut skipped_ws = false;
            while matches!(self.reader.peek(), Some(' ' | '\t')) {
                skipped_ws = true;
                self.reader.bump();
            }
            if self.reader.peek() == Some('#') {
                // After a quoted/flow ("JSON") node, `#` must be separated by
                // whitespace (suite SU5Z). Elsewhere a `#` still starts a
                // comment even if a prior scan already consumed indentation.
                let col = self.reader.mark().column;
                if skipped_ws || col == 1 || !self.json_node_pending {
                    self.skip_to_end_of_line();
                } else {
                    break;
                }
            }
            if self.reader.peek() == Some('\n') {
                self.reader.bump();
                if self.flow_level == 0 {
                    self.simple_key_allowed = true;
                }
                continue;
            }
            break;
        }
        Ok(())
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
