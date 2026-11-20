//! Node, collection and error helpers shared by the parser states.

use super::{Parser, State};
use crate::error::{Error, LimitKind, Result};
use crate::event::{Event, ScalarStyle};
use crate::mark::Mark;
use crate::token::Token;

impl Parser<'_> {
    pub(super) fn end_collection(&mut self, event: Event) -> Result<Option<Event>> {
        self.state = self.stack.pop().expect("collection state has a parent");
        Ok(Some(event))
    }

    /// Parses an optional anchor and tag, in either order.
    fn parse_properties(&mut self) -> Result<(Option<String>, Option<String>)> {
        let (mut anchor, mut tag) = (None, None);
        loop {
            match self.peek()? {
                Some(Token::Anchor { .. }) if anchor.is_none() => {
                    if let Some(Token::Anchor { name, .. }) = self.bump() {
                        anchor = Some(name);
                    }
                }
                Some(Token::Tag { .. }) if tag.is_none() => {
                    if let Some(Token::Tag {
                        handle,
                        suffix,
                        mark,
                    }) = self.bump()
                    {
                        tag = Some(self.resolve_tag(&handle, &suffix, mark)?);
                    }
                }
                _ => return Ok((anchor, tag)),
            }
        }
    }

    /// Parses one node (optionally anchored/tagged). When `allow_empty` is
    /// set, a missing node yields an empty plain scalar.
    pub(super) fn parse_node(&mut self, allow_empty: bool) -> Result<Event> {
        let (anchor, tag) = self.parse_properties()?;
        let has_props = anchor.is_some() || tag.is_some();
        match self.peek()? {
            Some(Token::Alias { .. }) if has_props => Err(unexpected_or_eof(self.peeked.as_ref())),
            Some(Token::Alias { .. }) => match self.bump() {
                Some(Token::Alias { name, .. }) => Ok(Event::Alias { anchor: name }),
                _ => unreachable!("peeked an alias"),
            },
            Some(Token::Scalar { .. }) => match self.bump() {
                Some(Token::Scalar { value, style, .. }) => Ok(Event::Scalar {
                    anchor,
                    tag,
                    value,
                    style,
                }),
                _ => unreachable!("peeked a scalar"),
            },
            Some(Token::BlockSequenceStart { .. }) => {
                let mark = self.bump_mark();
                self.start_collection(State::BlockSequenceEntry, mark)?;
                Ok(Event::SequenceStart {
                    anchor,
                    tag,
                    flow: false,
                })
            }
            Some(Token::BlockMappingStart { .. }) => {
                let mark = self.bump_mark();
                self.start_collection(State::BlockMappingKey, mark)?;
                Ok(Event::MappingStart {
                    anchor,
                    tag,
                    flow: false,
                })
            }
            Some(Token::FlowSequenceStart { .. }) => {
                let mark = self.bump_mark();
                self.start_collection(State::FlowSequenceFirst, mark)?;
                Ok(Event::SequenceStart {
                    anchor,
                    tag,
                    flow: true,
                })
            }
            Some(Token::FlowMappingStart { .. }) => {
                let mark = self.bump_mark();
                self.start_collection(State::FlowMappingFirst, mark)?;
                Ok(Event::MappingStart {
                    anchor,
                    tag,
                    flow: true,
                })
            }
            Some(Token::Key { .. } | Token::Value { .. } | Token::BlockEntry { .. })
            | Some(Token::BlockEnd { .. })
            | Some(Token::FlowEntry { .. })
            | Some(Token::FlowSequenceEnd { .. })
            | Some(Token::FlowMappingEnd { .. })
            | Some(Token::DocumentStart { .. } | Token::DocumentEnd { .. })
            | None
                if allow_empty || has_props =>
            {
                Ok(empty_scalar(anchor, tag))
            }
            other => Err(unexpected_or_eof(other)),
        }
    }

    pub(super) fn bump_mark(&mut self) -> Mark {
        match self.bump() {
            Some(
                Token::BlockSequenceStart { mark }
                | Token::BlockMappingStart { mark }
                | Token::FlowSequenceStart { mark }
                | Token::FlowMappingStart { mark },
            ) => mark,
            _ => unreachable!("peeked a collection start"),
        }
    }

    pub(super) fn start_collection(&mut self, inner: State, mark: Mark) -> Result<()> {
        if self.stack.len() >= self.limits.max_nesting_depth {
            return Err(Error::LimitExceeded {
                kind: LimitKind::NestingDepth,
                limit: self.limits.max_nesting_depth,
                mark,
            });
        }
        self.stack.push(self.state);
        self.state = inner;
        Ok(())
    }
}

pub(super) fn empty_scalar(anchor: Option<String>, tag: Option<String>) -> Event {
    Event::Scalar {
        anchor,
        tag,
        value: String::new(),
        style: ScalarStyle::Plain,
    }
}

pub(super) fn unexpected_or_eof(token: Option<&Token>) -> Error {
    match token {
        Some(t) => unexpected(t),
        None => Error::Parse {
            message: "unexpected end of input".into(),
            mark: Mark {
                index: 0,
                line: 1,
                column: 1,
            },
        },
    }
}

pub(super) fn unexpected(token: &Token) -> Error {
    let mark = match token {
        Token::StreamStart { mark }
        | Token::StreamEnd { mark }
        | Token::DocumentStart { mark }
        | Token::DocumentEnd { mark }
        | Token::Directive { mark, .. }
        | Token::BlockSequenceStart { mark }
        | Token::BlockMappingStart { mark }
        | Token::BlockEnd { mark }
        | Token::FlowSequenceStart { mark }
        | Token::FlowSequenceEnd { mark }
        | Token::FlowMappingStart { mark }
        | Token::FlowMappingEnd { mark }
        | Token::BlockEntry { mark }
        | Token::FlowEntry { mark }
        | Token::Key { mark }
        | Token::Value { mark }
        | Token::Anchor { mark, .. }
        | Token::Alias { mark, .. }
        | Token::Tag { mark, .. }
        | Token::Scalar { mark, .. } => *mark,
    };
    Error::Parse {
        message: "unexpected token".into(),
        mark,
    }
}
