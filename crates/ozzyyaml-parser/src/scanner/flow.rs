//! Flow collection tokens (`[`, `]`, `{`, `}`, `,`) and the `:` value indicator.

use super::{scan_error, Scanner};
use crate::chars::is_flow_indicator;
use crate::error::Result;
use crate::mark::Mark;
use crate::token::Token;

impl Scanner<'_> {
    /// True if the `:` at the reader position is a value indicator.
    pub(super) fn is_value_indicator(&self, _mark: Mark) -> bool {
        if self.indicator_followed_by_blank() {
            return true;
        }
        self.flow_level > 0
            && (self.after_json_node
                || self
                    .reader
                    .peek2()
                    .is_some_and(|(_, next)| is_flow_indicator(next)))
    }

    /// Records that a quoted scalar or flow collection just ended, so a
    /// following `:` is a value indicator without needing a blank after it.
    pub(super) fn mark_json_node_end(&mut self) {
        self.json_node_pending = true;
    }

    pub(super) fn fetch_flow_collection_start(&mut self, indicator: char, mark: Mark) {
        // `[` / `{` may themselves be a simple key at the *current* level
        // (`{a:1}: x`). Then push that slot and open an empty inner slot.
        self.save_possible_key(mark);
        self.possible_key_stack.push(self.possible_key.take());
        self.reader.bump();
        self.flow_level += 1;
        self.simple_key_allowed = true;
        self.queue.push_back(if indicator == '[' {
            Token::FlowSequenceStart { mark }
        } else {
            Token::FlowMappingStart { mark }
        });
    }

    pub(super) fn fetch_flow_collection_end(&mut self, indicator: char, mark: Mark) -> Result<()> {
        if self.flow_level == 0 {
            return Err(scan_error(
                format!("unexpected '{indicator}' outside a flow collection"),
                mark,
            ));
        }
        self.possible_key = None;
        self.reader.bump();
        self.flow_level -= 1;
        self.possible_key = self.possible_key_stack.pop().unwrap_or(None);
        self.simple_key_allowed = false;
        self.mark_json_node_end();
        self.queue.push_back(if indicator == ']' {
            Token::FlowSequenceEnd { mark }
        } else {
            Token::FlowMappingEnd { mark }
        });
        Ok(())
    }

    pub(super) fn fetch_flow_entry(&mut self, mark: Mark) {
        self.possible_key = None;
        self.simple_key_allowed = true;
        self.reader.bump();
        self.queue.push_back(Token::FlowEntry { mark });
    }
}
