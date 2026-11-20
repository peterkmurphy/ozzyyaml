use crate::error::{Error, Result};
use crate::event::Event;
use crate::limits::Limits;
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
    Done,
}

/// Pull parser that turns YAML text into a stream of [`Event`]s.
///
/// Currently supports documents consisting of a single scalar (or no content).
#[derive(Debug)]
pub struct Parser<'a> {
    scanner: Scanner<'a>,
    state: State,
    pending: Option<Token>,
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
            state: State::StreamStart,
            pending: None,
        })
    }

    /// Returns the next event, or `None` after the final `StreamEnd`.
    pub fn next_event(&mut self) -> Result<Option<Event>> {
        match self.state {
            State::StreamStart => {
                self.state = State::DocumentStart;
                Ok(Some(Event::StreamStart))
            }
            State::DocumentStart => {
                self.pending = self.scanner.next_token()?;
                if self.pending.is_none() {
                    self.state = State::StreamEnd;
                    return self.next_event();
                }
                self.state = State::DocumentContent;
                Ok(Some(Event::DocumentStart { explicit: false }))
            }
            State::DocumentContent => {
                let token = self
                    .pending
                    .take()
                    .expect("pending token set by DocumentStart");
                self.state = State::DocumentEnd;
                match token {
                    Token::Scalar { value, style, .. } => Ok(Some(Event::Scalar {
                        anchor: None,
                        tag: None,
                        value,
                        style,
                    })),
                    other => Err(unexpected(&other)),
                }
            }
            State::DocumentEnd => {
                if let Some(extra) = self.scanner.next_token()? {
                    self.state = State::Done;
                    return Err(unexpected(&extra));
                }
                self.state = State::StreamEnd;
                Ok(Some(Event::DocumentEnd { explicit: false }))
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
        message: "unexpected token; only a single scalar document is supported".into(),
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
