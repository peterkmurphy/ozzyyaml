//! Block structure: indentation tracking, `- ` entries, `?` keys and `:`.

use super::{scan_error, PossibleKey, Scanner};
use crate::error::Result;
use crate::mark::Mark;
use crate::token::Token;

impl Scanner<'_> {
    pub(super) fn save_possible_key(&mut self, mark: Mark) {
        if self.simple_key_allowed && self.possible_key.is_none() {
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
    pub(super) fn unroll_indent(&mut self, column: u32, mark: Mark) {
        while self.indents.last().is_some_and(|&top| top > column) {
            self.indents.pop();
            self.queue.push_back(Token::BlockEnd { mark });
        }
    }

    pub(super) fn fetch_block_entry(&mut self, mark: Mark) {
        if self.roll_indent(mark.column) {
            self.queue.push_back(Token::BlockSequenceStart { mark });
        }
        self.possible_key = None;
        self.simple_key_allowed = true;
        self.reader.bump();
        self.queue.push_back(Token::BlockEntry { mark });
    }

    /// Explicit key indicator `?`.
    pub(super) fn fetch_key(&mut self, mark: Mark) -> Result<()> {
        if self.flow_level == 0 {
            if !self.simple_key_allowed {
                return Err(scan_error(
                    "mapping keys are not allowed in this context".into(),
                    mark,
                ));
            }
            if self.roll_indent(mark.column) {
                self.queue.push_back(Token::BlockMappingStart { mark });
            }
        }
        self.possible_key = None;
        self.simple_key_allowed = self.flow_level == 0;
        self.reader.bump();
        self.queue.push_back(Token::Key { mark });
        Ok(())
    }

    pub(super) fn fetch_value(&mut self, mark: Mark) -> Result<()> {
        if let Some(key) = self.possible_key.take() {
            if self.flow_level == 0 {
                if self.value_line == Some(key.mark.line) {
                    return Err(scan_error(
                        "nested implicit mapping on the same line is not allowed".into(),
                        key.mark,
                    ));
                }
                self.value_line = Some(mark.line);
            }
            if key.token_number < self.tokens_taken {
                return Err(scan_error(
                    "simple key cannot be applied; token already consumed".into(),
                    key.mark,
                ));
            }
            let index = key.token_number - self.tokens_taken;
            self.queue.insert(index, Token::Key { mark: key.mark });
            if self.flow_level == 0 && self.roll_indent(key.mark.column) {
                self.queue
                    .insert(index, Token::BlockMappingStart { mark: key.mark });
            }
            self.simple_key_allowed = false;
        } else {
            // Complex / explicit key's value, or an empty key (`: value`).
            if self.flow_level == 0 {
                if !self.simple_key_allowed {
                    return Err(scan_error(
                        "mapping values are not allowed in this context".into(),
                        mark,
                    ));
                }
                if self.roll_indent(mark.column) {
                    self.queue.push_back(Token::BlockMappingStart { mark });
                }
            }
            self.simple_key_allowed = self.flow_level == 0;
        }
        self.reader.bump();
        self.queue.push_back(Token::Value { mark });
        Ok(())
    }
}
