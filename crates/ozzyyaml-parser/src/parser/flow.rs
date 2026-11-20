//! Flow-collection parsing states.

use super::node::unexpected_or_eof;
use super::{Parser, State};
use crate::error::Result;
use crate::event::Event;
use crate::token::Token;

impl Parser<'_> {
    /// Consumes the `,` separating flow entries (not before the first one).
    /// Returns true if the collection's closing token is next instead.
    pub(super) fn flow_separator(
        &mut self,
        first: bool,
        is_end: fn(&Token) -> bool,
    ) -> Result<bool> {
        if !first && !self.peek()?.is_some_and(is_end) {
            match self.peek()? {
                Some(Token::FlowEntry { .. }) => {
                    self.bump();
                }
                other => return Err(unexpected_or_eof(other)),
            }
        }
        Ok(self.peek()?.is_some_and(is_end))
    }

    pub(super) fn next_flow_sequence_entry(&mut self, first: bool) -> Result<Option<Event>> {
        if self.flow_separator(first, |t| matches!(t, Token::FlowSequenceEnd { .. }))? {
            self.bump();
            return self.end_collection(Event::SequenceEnd);
        }
        self.state = State::FlowSequenceEntry;
        if let Some(Token::Key { mark }) = self.peek()?.cloned() {
            // `[a: 1]` is a sequence entry holding a single-pair mapping.
            self.bump();
            self.start_collection(State::FlowPairKey, mark)?;
            return Ok(Some(Event::MappingStart {
                anchor: None,
                tag: None,
                flow: true,
            }));
        }
        self.parse_node(false).map(Some)
    }

    pub(super) fn next_flow_mapping_key(&mut self, first: bool) -> Result<Option<Event>> {
        if self.flow_separator(first, |t| matches!(t, Token::FlowMappingEnd { .. }))? {
            self.bump();
            return self.end_collection(Event::MappingEnd);
        }
        self.state = State::FlowMappingValue;
        if matches!(self.peek()?, Some(Token::Key { .. })) {
            self.bump();
            self.parse_node(true).map(Some)
        } else {
            // `{a, b: 1}`: a key with no value.
            self.parse_node(false).map(Some)
        }
    }
}
