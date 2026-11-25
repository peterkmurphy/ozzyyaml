//! Block scalars `|` / `>` (YAML 1.2.2 §8.1.4–§8.1.6).

use super::{scan_error, Scanner};
use crate::error::Result;
use crate::event::ScalarStyle;
use crate::mark::Mark;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Chomping {
    /// `-` — strip final break and trailing empty lines.
    Strip,
    /// default — keep one final break; strip trailing empty lines.
    Clip,
    /// `+` — keep final break and trailing empty lines.
    Keep,
}

impl Scanner<'_> {
    /// Scans a block scalar starting at `|` or `>`.
    pub(super) fn scan_block_scalar(&mut self, start: Mark) -> Result<(String, ScalarStyle)> {
        let indicator = self.reader.bump().expect("block scalar indicator");
        let literal = indicator == '|';
        let style = if literal {
            ScalarStyle::Literal
        } else {
            ScalarStyle::Folded
        };

        let (chomping, increment) = self.scan_block_scalar_header(start)?;

        // Parent indent in libyaml's 0-based column space.
        let parent: i32 = self
            .indents
            .last()
            .map(|&c| c as i32 - 1)
            .unwrap_or(-1);

        let mut indent: u32 = if increment > 0 {
            if parent >= 0 {
                (parent as u32).saturating_add(increment)
            } else {
                increment
            }
        } else {
            0 // determined from content
        };

        let mut value = String::new();
        let mut leading_break = String::new();
        let mut trailing_breaks = String::new();

        self.scan_block_scalar_breaks(start, &mut indent, &mut trailing_breaks)?;

        let mut leading_blank = false;
        while self.reader.mark().column.saturating_sub(1) == indent && self.reader.peek().is_some()
        {
            // `---` / `...` at column 1 end the document (and this scalar).
            // Needed for zero-indent block scalars (suite W4TN / spec ex. 9.5).
            if self.at_document_marker_line() {
                break;
            }
            // More-indented / blank-leading lines affect folded joining.
            let trailing_blank = matches!(self.reader.peek(), Some(' ' | '\t'));

            if !literal
                && leading_break == "\n"
                && !leading_blank
                && !trailing_blank
            {
                // Fold: a single `\n` between non-blank lines becomes a space.
                if trailing_breaks.is_empty() {
                    value.push(' ');
                }
                leading_break.clear();
            } else {
                value.push_str(&leading_break);
                leading_break.clear();
            }
            value.push_str(&trailing_breaks);
            trailing_breaks.clear();

            leading_blank = trailing_blank;

            // Consume the rest of the line (content past the indent).
            while let Some(c) = self.reader.peek() {
                if c == '\n' {
                    break;
                }
                if !crate::chars::is_nb_char(c) {
                    return Err(scan_error(
                        format!("non-printable character U+{:04X}", c as u32),
                        self.reader.mark(),
                    ));
                }
                value.push(c);
                self.reader.bump();
            }

            // Consume the line break into leading_break.
            if self.reader.peek() == Some('\n') {
                self.reader.bump();
                leading_break.push('\n');
            }

            self.scan_block_scalar_breaks(start, &mut indent, &mut trailing_breaks)?;
        }

        // Chomp the tail.
        match chomping {
            Chomping::Strip => {}
            Chomping::Clip => {
                value.push_str(&leading_break);
            }
            Chomping::Keep => {
                value.push_str(&leading_break);
                value.push_str(&trailing_breaks);
            }
        }

        Ok((value, style))
    }

    /// Parses chomping / indentation indicators and eats the rest of the header line.
    fn scan_block_scalar_header(&mut self, start: Mark) -> Result<(Chomping, u32)> {
        let mut chomping = Chomping::Clip;
        let mut increment = 0u32;

        match self.reader.peek() {
            Some('+') => {
                self.reader.bump();
                chomping = Chomping::Keep;
                if let Some(d) = self.reader.peek().and_then(indent_digit) {
                    self.reader.bump();
                    increment = d;
                }
            }
            Some('-') => {
                self.reader.bump();
                chomping = Chomping::Strip;
                if let Some(d) = self.reader.peek().and_then(indent_digit) {
                    self.reader.bump();
                    increment = d;
                }
            }
            Some(c) if indent_digit(c).is_some() => {
                increment = indent_digit(c).unwrap();
                self.reader.bump();
                match self.reader.peek() {
                    Some('+') => {
                        self.reader.bump();
                        chomping = Chomping::Keep;
                    }
                    Some('-') => {
                        self.reader.bump();
                        chomping = Chomping::Strip;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // Spaces then optional comment to end of line. A `#` glued to the
        // indicator (no whitespace) is invalid (suite X4QW).
        let mut saw_ws = false;
        while matches!(self.reader.peek(), Some(' ' | '\t')) {
            saw_ws = true;
            self.reader.bump();
        }
        if self.reader.peek() == Some('#') {
            if !saw_ws {
                return Err(scan_error(
                    "comment after block scalar indicator requires whitespace".into(),
                    self.reader.mark(),
                ));
            }
            while self.reader.peek().is_some_and(|c| c != '\n') {
                self.reader.bump();
            }
        }

        match self.reader.peek() {
            None | Some('\n') => {
                if self.reader.peek() == Some('\n') {
                    self.reader.bump();
                }
                Ok((chomping, increment))
            }
            Some(_) => Err(scan_error(
                "expected comment or line break after block scalar header".into(),
                start,
            )),
        }
    }

    /// Eats indentation spaces and empty lines; may determine `indent` on first use.
    fn scan_block_scalar_breaks(
        &mut self,
        start: Mark,
        indent: &mut u32,
        breaks: &mut String,
    ) -> Result<()> {
        let parent: i32 = self
            .indents
            .last()
            .map(|&c| c as i32 - 1)
            .unwrap_or(-1);

        let mut max_indent = 0u32;

        loop {
            // Eat spaces up to the content indent (tabs in indent are an error).
            while ({
                let col0 = self.reader.mark().column.saturating_sub(1);
                *indent == 0 || col0 < *indent
            }) && self.reader.peek() == Some(' ')
            {
                self.reader.bump();
            }

            let col0 = self.reader.mark().column.saturating_sub(1);
            if col0 > max_indent {
                max_indent = col0;
            }

            // Spec §8.2 example 8.2 / suite R4YG: while the indent is still being
            // auto-detected, leading spaces set it and a following tab is
            // *content* of the first non-empty line (s-indent [63] counts spaces
            // only); it is not an indentation error. Once the indent is known, a
            // tab before reaching it is still rejected ([68] s-block-line-prefix).
            if *indent != 0 && col0 < *indent && self.reader.peek() == Some('\t') {
                return Err(scan_error(
                    "tab character where an indentation space is expected".into(),
                    start,
                ));
            }

            // Non-empty line (or EOF): stop collecting breaks.
            if self.reader.peek() != Some('\n') {
                break;
            }

            self.reader.bump();
            breaks.push('\n');
        }

        if *indent == 0 {
            let mut determined = max_indent;
            let min = if parent >= 0 {
                (parent as u32).saturating_add(1)
            } else {
                0
            };
            if determined < min {
                determined = min;
            }
            // Allow indent 0 at document root (suite DK3J); otherwise require ≥1
            // when there is a parent collection, matching libyaml's floor.
            if parent >= 0 && determined < 1 {
                determined = 1;
            }
            *indent = determined;
        }

        Ok(())
    }

    /// True if the reader is at column 1 on a `---` / `...` document marker line.
    fn at_document_marker_line(&self) -> bool {
        if self.reader.mark().column != 1 {
            return false;
        }
        let rest = self.reader.rest();
        let marker = if rest.starts_with("---") {
            "---"
        } else if rest.starts_with("...") {
            "..."
        } else {
            return false;
        };
        match rest[marker.len()..].chars().next() {
            None | Some(' ' | '\t' | '\n' | '\r') => true,
            Some(_) => false,
        }
    }
}

fn indent_digit(c: char) -> Option<u32> {
    match c {
        '1'..='9' => Some(c.to_digit(10).unwrap()),
        _ => None,
    }
}
