//! Block structure: indentation tracking, `- ` entries and simple keys.

use super::{scan_error, PossibleKey, Scanner};
use crate::error::Result;
use crate::mark::Mark;
use crate::token::Token;

#[allow(dead_code)]
impl Scanner<'_> {
    pub(super) fn save_possible_key(&mut self, mark: Mark) {
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
        self.reader.bump();
        self.queue.push_back(Token::BlockEntry { mark });
    }

    pub(super) fn fetch_value(&mut self, mark: Mark) -> Result<()> {
        let Some(key) = self.possible_key.take() else {
            return Err(scan_error(
                "mapping value without a simple key on the same line".into(),
                mark,
            ));
        };
        if self.flow_level == 0 {
            if self.value_line == Some(key.mark.line) {
                return Err(scan_error(
                    "nested implicit mapping on the same line is not allowed".into(),
                    key.mark,
                ));
            }
            self.value_line = Some(mark.line);
        }
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
}
