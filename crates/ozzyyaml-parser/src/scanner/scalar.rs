//! Plain, single-quoted and double-quoted scalar scanning (with line folding).

use super::{scan_error, unterminated, Scanner};
use crate::chars::{is_flow_indicator, is_hex_digit, is_nb_char, is_nb_json};
use crate::error::Result;
use crate::mark::Mark;

impl Scanner<'_> {
    /// Minimum indent a block-context continuation line must exceed.
    fn plain_min_indent(&self) -> u32 {
        if self.flow_level > 0 {
            0
        } else {
            self.indents.last().copied().unwrap_or(0)
        }
    }

    /// Reads a plain scalar, folding line breaks per YAML 1.2.2 §8.1.3.
    pub(super) fn scan_plain(&mut self) -> Result<String> {
        let min_indent = self.plain_min_indent();
        let mut value = String::new();
        loop {
            let chunk_start = value.len();
            while let Some(c) = self.reader.peek() {
                let starts_comment = matches!(self.reader.peek2(), Some((' ' | '\t', '#')));
                let flow_stop = self.flow_level > 0 && is_flow_indicator(c);
                let is_value = c == ':' && self.is_plain_value_indicator();
                if c == '\n' || starts_comment || flow_stop || is_value {
                    break;
                }
                if !is_nb_char(c) {
                    return Err(scan_error(
                        format!("non-printable character U+{:04X}", c as u32),
                        self.reader.mark(),
                    ));
                }
                value.push(c);
                self.reader.bump();
            }
            // Trailing spaces/tabs on a line are discarded when folding (§8.1.3).
            while value.len() > chunk_start && matches!(value.chars().last(), Some(' ' | '\t')) {
                value.pop();
            }

            if !self.plain_should_fold(min_indent) {
                break;
            }
            self.fold_block_breaks(&mut value, min_indent, /*trim_leading*/ true);
        }
        Ok(value)
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

    /// True if the next characters are a line break that continues this plain scalar.
    fn plain_should_fold(&self, min_indent: u32) -> bool {
        if self.reader.peek() != Some('\n') {
            return false;
        }
        match self.peek_after_folded_breaks(min_indent) {
            Some((col, first)) => {
                if self.flow_level == 0 && col <= min_indent {
                    return false;
                }
                // Flow collection terminators / entries end the scalar.
                if self.flow_level > 0 && is_flow_indicator(first) {
                    return false;
                }
                // A line whose first non-space is `#` is a comment, not content.
                if first == '#' {
                    return false;
                }
                // A more-indented `- ` (col > min_indent) is plain-scalar content,
                // not a new entry; a sibling `- ` at min_indent is already stopped
                // by the `col <= min_indent` check above (libyaml folds purely on
                // indentation — [126] ns-plain-first / §8.1.3).
                // Document markers at column 1 end the scalar.
                if col == 1 && self.peek_document_marker_ahead() {
                    return false;
                }
                true
            }
            None => false,
        }
    }

    /// After consuming the current `\n` and any empty lines, what is the next
    /// content column and first non-space character? Does not mutate the reader.
    fn peek_after_folded_breaks(&self, _min_indent: u32) -> Option<(u32, char)> {
        let mut rest = self.reader.rest();
        loop {
            // Require a break to enter / continue empty-line skipping.
            let br = line_break_len(rest)?;
            rest = &rest[br..];
            let mut col = 1u32;
            let mut i = 0usize;
            while i < rest.len() {
                let c = rest[i..].chars().next()?;
                match c {
                    ' ' => {
                        col += 1;
                        i += c.len_utf8();
                    }
                    '\t' => {
                        // Tabs in indentation: treat as a single column bump for
                        // continue checks; content-only tab lines count as empty.
                        col += 1;
                        i += c.len_utf8();
                    }
                    '\r' | '\n' => {
                        // Empty (or ws-only) line — keep scanning.
                        break;
                    }
                    other => return Some((col, other)),
                }
            }
            if i >= rest.len() {
                return None; // EOF after blanks
            }
            // Hit a break while scanning spaces — empty line; loop.
            rest = &rest[i..];
        }
    }

    fn peek_document_marker_ahead(&self) -> bool {
        let mut rest = self.reader.rest();
        // Skip one or more breaks and spaces to column 1 content.
        while let Some(br) = line_break_len(rest) {
            rest = &rest[br..];
            let mut i = 0;
            while i < rest.len() && matches!(rest[i..].chars().next(), Some(' ' | '\t')) {
                i += rest[i..].chars().next().unwrap().len_utf8();
            }
            rest = &rest[i..];
            if rest.starts_with("---") || rest.starts_with("...") {
                let after = rest.get(3..).unwrap_or("");
                return after.is_empty()
                    || after.starts_with(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r'));
            }
            if !rest.is_empty() && !rest.starts_with('\n') && !rest.starts_with('\r') {
                return false;
            }
        }
        false
    }

    /// Consume `\n` + empty lines + leading spaces on the next content line,
    /// appending a space or newlines to `value` (§8.1.3).
    fn fold_block_breaks(&mut self, value: &mut String, min_indent: u32, trim_leading: bool) {
        debug_assert_eq!(self.reader.peek(), Some('\n'));
        self.reader.bump(); // first break

        let mut empty_lines = 0u32;
        loop {
            // Count spaces/tabs on this line.
            let mut saw_tab = false;
            while matches!(self.reader.peek(), Some(' ' | '\t')) {
                if self.reader.peek() == Some('\t') {
                    saw_tab = true;
                }
                self.reader.bump();
            }
            match self.reader.peek() {
                Some('\n') => {
                    empty_lines += 1;
                    self.reader.bump();
                    let _ = saw_tab;
                    continue;
                }
                None => {
                    // Trailing empty lines at EOF: still contribute newlines.
                    for _ in 0..empty_lines {
                        value.push('\n');
                    }
                    return;
                }
                Some(_) => {
                    // Content. More-indented / empty-line rules:
                    // - If empty_lines > 0: append that many `\n` (no space).
                    // - Else: append a single space.
                    if empty_lines > 0 {
                        for _ in 0..empty_lines {
                            value.push('\n');
                        }
                    } else {
                        value.push(' ');
                    }
                    if trim_leading {
                        // Leading indentation already consumed above.
                        let _ = min_indent;
                    }
                    return;
                }
            }
        }
    }

    pub(super) fn scan_single_quoted(&mut self, start: Mark) -> Result<String> {
        self.reader.bump(); // opening quote
        let mut value = String::new();
        loop {
            match self.reader.peek() {
                None => return Err(unterminated("single-quoted", start)),
                Some('\'') => {
                    self.reader.bump();
                    if self.reader.peek() == Some('\'') {
                        self.reader.bump();
                        value.push('\'');
                    } else {
                        return Ok(value);
                    }
                }
                Some('\n') => {
                    self.fold_quoted_breaks(&mut value, 0)?;
                }
                Some(c) => {
                    if !is_nb_char(c) {
                        return Err(scan_error(
                            format!("non-printable character U+{:04X}", c as u32),
                            self.reader.mark(),
                        ));
                    }
                    self.reader.bump();
                    value.push(c);
                }
            }
        }
    }

    pub(super) fn scan_double_quoted(&mut self, start: Mark) -> Result<String> {
        self.reader.bump(); // opening quote
        let mut value = String::new();
        // Length of `value` up to which trailing white space is content
        // (produced by an escape or kept before an escaped break) and so
        // must not be trimmed by a following line fold (§7.3.1 [111]).
        let mut protected = 0usize;
        loop {
            match self.reader.peek() {
                None => return Err(unterminated("double-quoted", start)),
                Some('"') => {
                    self.reader.bump();
                    return Ok(value);
                }
                Some('\\') => {
                    self.reader.bump();
                    // Escaped line break: `\` + break discards the break (§6.6).
                    if self.reader.peek() == Some('\n') {
                        // [112] s-double-escaped: `\` b-non-content l-empty*
                        // s-flow-line-prefix; each empty line yields a `\n`.
                        self.reader.bump();
                        self.skip_inline_white();
                        while self.reader.peek() == Some('\n') {
                            self.reader.bump();
                            value.push('\n');
                            self.skip_inline_white();
                        }
                    } else {
                        value.push(self.scan_escape()?);
                    }
                    protected = value.len();
                }
                Some('\n') => {
                    self.fold_quoted_breaks(&mut value, protected)?;
                    protected = protected.min(value.len());
                }
                Some(c) => {
                    if !is_nb_json(c) {
                        return Err(scan_error(
                            format!(
                                "non-JSON character U+{:04X} in double-quoted scalar",
                                c as u32
                            ),
                            self.reader.mark(),
                        ));
                    }
                    self.reader.bump();
                    value.push(c);
                }
            }
        }
    }

    fn skip_inline_white(&mut self) {
        while matches!(self.reader.peek(), Some(' ' | '\t')) {
            self.reader.bump();
        }
    }

    /// Fold a line break inside a quoted scalar (§7.3.1 / [74] `s-flow-folded`).
    ///
    /// Trailing white space on the line being left is discarded (but never
    /// below `protected`, which holds escape-produced content), the first
    /// break is folded to a single space unless empty lines follow, in which
    /// case each empty line yields `\n`; leading white space on the
    /// continuation line is dropped.
    fn fold_quoted_breaks(&mut self, value: &mut String, protected: usize) -> Result<()> {
        debug_assert_eq!(self.reader.peek(), Some('\n'));
        self.reader.bump();

        while value.len() > protected && matches!(value.chars().last(), Some(' ' | '\t')) {
            value.pop();
        }

        let mut empty_lines = 0u32;
        let parent_indent = self.indents.last().copied().unwrap_or(0);
        loop {
            self.skip_inline_white();
            let col = self.reader.mark().column;
            // A document marker at column 1 cannot appear inside an open quoted scalar.
            if col == 1 {
                let rest = self.reader.rest();
                if rest.starts_with("---") || rest.starts_with("...") {
                    let after = rest.get(3..).unwrap_or("");
                    let marker_ok = after.is_empty()
                        || after.starts_with([' ', '\t', '\n', '\r']);
                    if marker_ok {
                        return Err(scan_error(
                            "document marker while scanning a quoted scalar".into(),
                            self.reader.mark(),
                        ));
                    }
                }
            }
            match self.reader.peek() {
                Some('\n') => {
                    empty_lines += 1;
                    self.reader.bump();
                }
                None => return Err(unterminated("quoted", self.reader.mark())),
                Some(_) => {
                    // In block context, continuation lines of a quoted scalar
                    // must be indented past the parent (suite QB6E).
                    if self.flow_level == 0 && col <= parent_indent {
                        return Err(scan_error(
                            "wrong indentation while scanning a quoted scalar".into(),
                            self.reader.mark(),
                        ));
                    }
                    if empty_lines > 0 {
                        for _ in 0..empty_lines {
                            value.push('\n');
                        }
                    } else {
                        value.push(' ');
                    }
                    return Ok(());
                }
            }
        }
    }

    /// Decodes the character after a backslash (YAML 1.2.2 §7.3.2 `c-ns-esc-char`).
    fn scan_escape(&mut self) -> Result<char> {
        let mark = self.reader.mark();
        let decoded = match self.reader.bump() {
            Some('0') => '\0',
            Some('a') => '\u{7}',
            Some('b') => '\u{8}',
            Some('t' | '\t') => '\t',
            Some('n') => '\n',
            Some('v') => '\u{b}',
            Some('f') => '\u{c}',
            Some('r') => '\r',
            Some('e') => '\u{1b}',
            Some(' ') => ' ',
            Some('"') => '"',
            Some('/') => '/',
            Some('\\') => '\\',
            Some('N') => '\u{85}',
            Some('_') => '\u{a0}',
            Some('L') => '\u{2028}',
            Some('P') => '\u{2029}',
            Some('x') => self.scan_hex_escape(mark, 2)?,
            Some('u') => self.scan_hex_escape(mark, 4)?,
            Some('U') => self.scan_hex_escape(mark, 8)?,
            Some(c) => return Err(scan_error(format!("invalid escape '\\{c}'"), mark)),
            None => return Err(scan_error("unterminated escape".into(), mark)),
        };
        Ok(decoded)
    }

    /// Reads `width` hex digits and returns the corresponding Unicode scalar.
    fn scan_hex_escape(&mut self, mark: Mark, width: usize) -> Result<char> {
        let mut code = 0u32;
        for _ in 0..width {
            let digit = match self.reader.bump() {
                Some(c) if is_hex_digit(c) => c.to_digit(16).expect("hex digit"),
                Some(c) => {
                    return Err(scan_error(
                        format!("invalid hex digit '{c}' in escape"),
                        mark,
                    ))
                }
                None => return Err(scan_error("unterminated hex escape".into(), mark)),
            };
            code = (code << 4) | digit;
        }
        char::from_u32(code).ok_or_else(|| {
            scan_error(
                format!("escape is not a valid Unicode scalar value U+{code:X}"),
                mark,
            )
        })
    }
}

fn line_break_len(s: &str) -> Option<usize> {
    if s.starts_with("\r\n") {
        Some(2)
    } else if s.starts_with('\n') || s.starts_with('\r') {
        Some(1)
    } else {
        None
    }
}
