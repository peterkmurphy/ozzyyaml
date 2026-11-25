//! Flow-collection parsing states.

use super::node::{empty_scalar, unexpected_or_eof};
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

    /// Starts the single-pair mapping of an implicit flow-sequence entry
    /// (`[a: 1]`, `["a":b]`, `[: v]`). A leading `:` means an empty key;
    /// the key node itself is produced by `State::FlowPairKey`.
    fn parse_flow_pair(&mut self) -> Result<Event> {
        let mark = match self.peek()?.cloned() {
            Some(Token::Key { mark }) => {
                self.bump();
                mark
            }
            Some(Token::Value { mark }) => mark,
            other => return Err(unexpected_or_eof(other.as_ref())),
        };
        self.start_collection(State::FlowPairKey, mark)?;
        Ok(Event::MappingStart {
            anchor: None,
            tag: None,
            flow: true,
        })
    }

    pub(super) fn next_flow_sequence_entry(&mut self, first: bool) -> Result<Option<Event>> {
        if self.flow_separator(first, |t| matches!(t, Token::FlowSequenceEnd { .. }))? {
            self.bump();
            return self.end_collection(Event::SequenceEnd);
        }
        self.state = State::FlowSequenceEntry;
        if matches!(self.peek()?, Some(Token::Key { .. } | Token::Value { .. })) {
            return self.parse_flow_pair().map(Some);
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
        } else if matches!(self.peek()?, Some(Token::Value { .. })) {
            // Empty key: `: value` inside `{...}`.
            Ok(Some(empty_scalar(None, None)))
        } else {
            // `{a, b: 1}`: a key with no explicit `?` (may still have a value).
            self.parse_node(false).map(Some)
        }
    }
}
