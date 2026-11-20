use crate::error::{Error, LimitKind, Result};
use crate::event::{Event, ScalarStyle};
use crate::limits::Limits;
use crate::mark::Mark;
use crate::reader::Reader;
use crate::scanner::Scanner;
use crate::token::Token;

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
    Done,
}

/// Pull parser that turns YAML text into a stream of [`Event`]s.
///
/// Supports a single implicit document whose root is a scalar, alias, block
/// sequence or block mapping (with anchors). Flow collections, tags, and
/// multiple documents are not supported yet.
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
            State::StreamEnd => {
                self.state = State::Done;
                Ok(Some(Event::StreamEnd))
            }
            State::Done => Ok(None),
        }
    }

    fn end_collection(&mut self, event: Event) -> Result<Option<Event>> {
        self.state = self.stack.pop().expect("collection state has a parent");
        Ok(Some(event))
    }

    /// Parses one node (optionally anchored). When `allow_empty` is set, a
    /// missing node yields an empty plain scalar.
    fn parse_node(&mut self, allow_empty: bool) -> Result<Event> {
        let anchor = match self.peek()? {
            Some(Token::Anchor { .. }) => match self.bump() {
                Some(Token::Anchor { name, .. }) => Some(name),
                _ => unreachable!("peeked an anchor"),
            },
            _ => None,
        };
        match self.peek()? {
            Some(Token::Alias { .. }) if anchor.is_some() => {
                Err(unexpected_or_eof(self.peeked.as_ref()))
            }
            Some(Token::Alias { .. }) => match self.bump() {
                Some(Token::Alias { name, .. }) => Ok(Event::Alias { anchor: name }),
                _ => unreachable!("peeked an alias"),
            },
            Some(Token::Scalar { .. }) => match self.bump() {
                Some(Token::Scalar { value, style, .. }) => Ok(Event::Scalar {
                    anchor,
                    tag: None,
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
                    tag: None,
                    flow: false,
                })
            }
            Some(Token::BlockMappingStart { .. }) => {
                let mark = self.bump_mark();
                self.start_collection(State::BlockMappingKey, mark)?;
                Ok(Event::MappingStart {
                    anchor,
                    tag: None,
                    flow: false,
                })
            }
            Some(Token::Key { .. } | Token::Value { .. } | Token::BlockEntry { .. })
            | Some(Token::BlockEnd { .. })
            | None
                if allow_empty || anchor.is_some() =>
            {
                Ok(empty_scalar(anchor))
            }
            other => Err(unexpected_or_eof(other)),
        }
    }

    fn bump_mark(&mut self) -> Mark {
        match self.bump() {
            Some(Token::BlockSequenceStart { mark } | Token::BlockMappingStart { mark }) => mark,
            _ => unreachable!("peeked a collection start"),
        }
    }

    fn start_collection(&mut self, inner: State, mark: Mark) -> Result<()> {
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

fn empty_scalar(anchor: Option<String>) -> Event {
    Event::Scalar {
        anchor,
        tag: None,
        value: String::new(),
        style: ScalarStyle::Plain,
    }
}

fn unexpected_or_eof(token: Option<&Token>) -> Error {
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

/// Parses `input` fully, collecting every event.
pub fn parse_str(input: &str) -> Result<Vec<Event>> {
    let mut parser = Parser::new(input)?;
    let mut events = Vec::new();
    while let Some(ev) = parser.next_event()? {
        events.push(ev);
    }
    Ok(events)
}

fn unexpected(token: &Token) -> Error {
    let mark = match token {
        Token::StreamStart { mark }
        | Token::StreamEnd { mark }
        | Token::DocumentStart { mark }
        | Token::DocumentEnd { mark }
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

#[cfg(test)]
mod tests {
    use crate::{parse_str, Event};

    #[test]
    fn parse_plain_scalar_document() {
        let evs = parse_str("hello").unwrap();
        assert_eq!(evs.first(), Some(&Event::StreamStart));
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::Scalar { value, .. } if value == "hello")));
        assert_eq!(evs.last(), Some(&Event::StreamEnd));
    }

    #[test]
    fn parse_quoted_scalar_keeps_style() {
        use crate::ScalarStyle;
        let evs = parse_str("\"hi\"").unwrap();
        assert!(evs.iter().any(|e| matches!(
            e,
            Event::Scalar { value, style: ScalarStyle::DoubleQuoted, .. } if value == "hi"
        )));
    }

    #[test]
    fn parse_empty_input_has_no_document() {
        assert_eq!(
            parse_str("").unwrap(),
            vec![Event::StreamStart, Event::StreamEnd]
        );
    }

    #[test]
    fn parse_exact_event_sequence() {
        let evs = parse_str("hello").unwrap();
        assert_eq!(evs.len(), 5);
        assert_eq!(evs[1], Event::DocumentStart { explicit: false });
        assert_eq!(evs[3], Event::DocumentEnd { explicit: false });
    }

    #[test]
    fn parse_second_scalar_is_error() {
        assert!(matches!(parse_str("a\nb"), Err(crate::Error::Parse { .. })));
    }

    #[test]
    fn next_event_returns_none_after_stream_end() {
        let mut p = crate::Parser::new("x").unwrap();
        while p.next_event().unwrap().is_some() {}
        assert_eq!(p.next_event().unwrap(), None);
    }
}
