use crate::error::Result;
use crate::event::Event;
use crate::limits::Limits;
use crate::reader::Reader;
use crate::scanner::Scanner;
use crate::token::Token;

mod flow;
mod node;
#[cfg(test)]
mod tests;

use node::{empty_scalar, unexpected, unexpected_or_eof};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    StreamStart,
    DocumentStart,
    DocumentContent,
    DocumentEnd,
    StreamEnd,
    BlockSequenceEntry,
    BlockMappingKey,
    BlockMappingValue,
    FlowSequenceFirst,
    FlowSequenceEntry,
    /// Key of a single-pair mapping inside a flow sequence (`[a: 1]`).
    FlowPairKey,
    FlowPairValue,
    FlowPairEnd,
    FlowMappingFirst,
    FlowMappingKey,
    FlowMappingValue,
    Done,
}

/// Pull parser that turns YAML text into a stream of [`Event`]s.
///
/// Supports a single implicit document whose root is a scalar, alias, or a
/// block or flow sequence/mapping (with anchors). Tags and multiple documents
/// are not supported yet.
#[derive(Debug)]
pub struct Parser<'a> {
    scanner: Scanner<'a>,
    limits: Limits,
    state: State,
    /// Continuation states of enclosing collections.
    stack: Vec<State>,
    peeked: Option<Token>,
}

impl<'a> Parser<'a> {
    /// Creates a parser with [`Limits::default`].
    pub fn new(input: &'a str) -> Result<Self> {
        Self::with_limits(input, Limits::default())
    }

    /// Creates a parser enforcing the given [`Limits`].
    pub fn with_limits(input: &'a str, limits: Limits) -> Result<Self> {
        let reader = Reader::new(input, limits)?;
        Ok(Self {
            scanner: Scanner::new(reader),
            limits,
            state: State::StreamStart,
            stack: Vec::new(),
            peeked: None,
        })
    }

    fn peek(&mut self) -> Result<Option<&Token>> {
        if self.peeked.is_none() {
            self.peeked = self.scanner.next_token()?;
        }
        Ok(self.peeked.as_ref())
    }

    fn bump(&mut self) -> Option<Token> {
        self.peeked.take()
    }

    /// Returns the next event, or `None` after the final `StreamEnd`.
    pub fn next_event(&mut self) -> Result<Option<Event>> {
        match self.state {
            State::StreamStart => {
                self.state = State::DocumentStart;
                Ok(Some(Event::StreamStart))
            }
            State::DocumentStart => {
                if self.peek()?.is_none() {
                    self.state = State::StreamEnd;
                    return self.next_event();
                }
                self.state = State::DocumentContent;
                Ok(Some(Event::DocumentStart { explicit: false }))
            }
            State::DocumentContent => {
                self.state = State::DocumentEnd;
                self.parse_node(false).map(Some)
            }
            State::DocumentEnd => {
                if let Some(extra) = self.peek()? {
                    let err = unexpected(extra);
                    self.state = State::Done;
                    return Err(err);
                }
                self.state = State::StreamEnd;
                Ok(Some(Event::DocumentEnd { explicit: false }))
            }
            State::BlockSequenceEntry => match self.peek()? {
                Some(Token::BlockEntry { .. }) => {
                    self.bump();
                    self.parse_node(true).map(Some)
                }
                Some(Token::BlockEnd { .. }) => {
                    self.bump();
                    self.end_collection(Event::SequenceEnd)
                }
                other => Err(unexpected_or_eof(other)),
            },
            State::BlockMappingKey => match self.peek()? {
                Some(Token::Key { .. }) => {
                    self.bump();
                    self.state = State::BlockMappingValue;
                    self.parse_node(true).map(Some)
                }
                Some(Token::BlockEnd { .. }) => {
                    self.bump();
                    self.end_collection(Event::MappingEnd)
                }
                other => Err(unexpected_or_eof(other)),
            },
            State::BlockMappingValue => {
                self.state = State::BlockMappingKey;
                if matches!(self.peek()?, Some(Token::Value { .. })) {
                    self.bump();
                    self.parse_node(true).map(Some)
                } else {
                    Ok(Some(empty_scalar(None)))
                }
            }
            State::FlowSequenceFirst => self.next_flow_sequence_entry(true),
            State::FlowSequenceEntry => self.next_flow_sequence_entry(false),
            State::FlowPairKey => {
                self.state = State::FlowPairValue;
                self.parse_node(true).map(Some)
            }
            State::FlowPairValue => {
                self.state = State::FlowPairEnd;
                if matches!(self.peek()?, Some(Token::Value { .. })) {
                    self.bump();
                    self.parse_node(true).map(Some)
                } else {
                    Ok(Some(empty_scalar(None)))
                }
            }
            State::FlowPairEnd => self.end_collection(Event::MappingEnd),
            State::FlowMappingFirst => self.next_flow_mapping_key(true),
            State::FlowMappingKey => self.next_flow_mapping_key(false),
            State::FlowMappingValue => {
                self.state = State::FlowMappingKey;
                if matches!(self.peek()?, Some(Token::Value { .. })) {
                    self.bump();
                    self.parse_node(true).map(Some)
                } else {
                    Ok(Some(empty_scalar(None)))
                }
            }
            State::StreamEnd => {
                self.state = State::Done;
                Ok(Some(Event::StreamEnd))
            }
            State::Done => Ok(None),
        }
    }
}

/// Parses `input` fully, collecting every event.
pub fn parse_str(input: &str) -> Result<Vec<Event>> {
    let mut parser = Parser::new(input)?;
    let mut events = Vec::new();
    while let Some(ev) = parser.next_event()? {
        events.push(ev);
    }
    Ok(events)
}
