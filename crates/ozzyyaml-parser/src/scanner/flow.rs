//! Flow collection tokens (`[`, `]`, `{`, `}`, `,`) and the `:` value indicator.

use super::{scan_error, Scanner};
use crate::chars::is_flow_indicator;
use crate::error::Result;
use crate::mark::Mark;
use crate::token::Token;

impl Scanner<'_> {
    /// True if the `:` at the reader position is a value indicator.
    pub(super) fn is_value_indicator(&self, mark: Mark) -> bool {
        if self.indicator_followed_by_blank() {
            return true;
        }
        self.flow_level > 0
            && (self.quoted_end == Some(mark.index)
                || self
                    .reader
                    .peek2()
                    .is_some_and(|(_, next)| is_flow_indicator(next)))
    }

    pub(super) fn fetch_flow_collection_start(&mut self, indicator: char, mark: Mark) {
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
        self.queue.push_back(if indicator == ']' {
            Token::FlowSequenceEnd { mark }
        } else {
            Token::FlowMappingEnd { mark }
        });
        Ok(())
    }

    pub(super) fn fetch_flow_entry(&mut self, mark: Mark) {
        self.possible_key = None;
        self.reader.bump();
        self.queue.push_back(Token::FlowEntry { mark });
    }
}
