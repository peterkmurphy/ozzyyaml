//! Document boundaries, `%` directives and tag resolution.

use super::node::{empty_scalar, unexpected_or_eof};
use super::{Parser, State};
use crate::error::{Error, Result};
use crate::event::Event;
use crate::mark::Mark;
use crate::token::Token;

const DEFAULT_SECONDARY_PREFIX: &str = "tag:yaml.org,2002:";

impl Parser<'_> {
    /// Handles `State::DocumentStart`: directives, then `---` or a bare document.
    pub(super) fn next_document_start(&mut self) -> Result<Option<Event>> {
        while matches!(self.peek()?, Some(Token::DocumentEnd { .. })) {
            self.bump();
        }
        self.tag_directives.clear();
        self.block_mapping_key_line = None;
        self.block_mapping_key_column = None;
        self.document_start_line = None;
        let mut seen_yaml = false;
        let mut seen_directive = false;
        while let Some(Token::Directive { .. }) = self.peek()? {
            if let Some(Token::Directive { name, args, mark }) = self.bump() {
                self.apply_directive(&name, &args, mark, &mut seen_yaml)?;
                seen_directive = true;
            }
        }
        match self.peek()? {
            Some(Token::DocumentStart { mark }) => {
                self.document_start_line = Some(mark.line);
                self.bump();
                self.state = State::DocumentContent;
                Ok(Some(Event::DocumentStart { explicit: true }))
            }
            None if !seen_directive => {
                self.state = State::StreamEnd;
                self.next_event()
            }
            other if seen_directive => Err(unexpected_or_eof(other)),
            _ => {
                self.state = State::DocumentContent;
                Ok(Some(Event::DocumentStart { explicit: false }))
            }
        }
    }

    /// Handles `State::DocumentContent`: a node, or an empty scalar when the
    /// document body is missing (`---` directly followed by a marker or EOF).
    pub(super) fn next_document_content(&mut self) -> Result<Option<Event>> {
        self.state = State::DocumentEnd;
        match self.peek()? {
            None | Some(Token::DocumentStart { .. } | Token::DocumentEnd { .. }) => {
                Ok(Some(empty_scalar(None, None)))
            }
            _ => self.parse_node(false).map(Some),
        }
    }

    /// Handles `State::DocumentEnd`: an optional `...`, else the next `---`/EOF.
    pub(super) fn next_document_end(&mut self) -> Result<Option<Event>> {
        let explicit = match self.peek()? {
            Some(Token::DocumentEnd { .. }) => {
                self.bump();
                true
            }
            Some(Token::DocumentStart { .. }) | None => false,
            Some(extra) => {
                let err = super::node::unexpected(extra);
                self.state = State::Done;
                return Err(err);
            }
        };
        self.state = State::DocumentStart;
        Ok(Some(Event::DocumentEnd { explicit }))
    }

    fn apply_directive(
        &mut self,
        name: &str,
        args: &[String],
        mark: Mark,
        seen_yaml: &mut bool,
    ) -> Result<()> {
        match (name, args) {
            ("YAML", [version]) => {
                if std::mem::replace(seen_yaml, true) {
                    return Err(parse_error("duplicate %YAML directive", mark));
                }
                match version.split_once('.') {
                    Some((major, minor))
                        if major.parse::<u32>().is_ok() && minor.parse::<u32>().is_ok() =>
                    {
                        if major != "1" {
                            return Err(parse_error("unsupported YAML version", mark));
                        }
                        Ok(())
                    }
                    _ => Err(parse_error("malformed %YAML directive", mark)),
                }
            }
            ("YAML", _) => Err(parse_error("malformed %YAML directive", mark)),
            ("TAG", [handle, prefix]) => {
                if !is_tag_handle(handle) {
                    return Err(parse_error("malformed %TAG handle", mark));
                }
                if self.tag_directives.iter().any(|(h, _)| h == handle) {
                    return Err(parse_error("duplicate %TAG directive", mark));
                }
                self.tag_directives.push((handle.clone(), prefix.clone()));
                Ok(())
            }
            ("TAG", _) => Err(parse_error("malformed %TAG directive", mark)),
            _ => Ok(()), // unknown directives are ignored per the spec
        }
    }

    /// Expands a scanned tag (`handle` + `suffix`) to its full tag string.
    pub(super) fn resolve_tag(&self, handle: &str, suffix: &str, mark: Mark) -> Result<String> {
        let suffix = percent_decode(suffix, mark)?;
        if handle.is_empty() {
            return Ok(suffix); // verbatim `!<…>`
        }
        let declared = self.tag_directives.iter().find(|(h, _)| h == handle);
        let prefix = match (declared, handle) {
            (Some((_, prefix)), _) => prefix.as_str(),
            (None, "!") => "!",
            (None, "!!") => DEFAULT_SECONDARY_PREFIX,
            _ => return Err(parse_error("undeclared tag handle", mark)),
        };
        Ok(format!("{prefix}{suffix}"))
    }
}

/// Decodes `%HH` octets in a tag URI/suffix (YAML 1.2 tag characters).
fn percent_decode(input: &str, mark: Mark) -> Result<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(parse_error("incomplete percent-escape in tag", mark));
            }
            let hi = hex_nibble(bytes[i + 1])
                .ok_or_else(|| parse_error("invalid percent-escape in tag", mark))?;
            let lo = hex_nibble(bytes[i + 2])
                .ok_or_else(|| parse_error("invalid percent-escape in tag", mark))?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out)
        .map_err(|_| parse_error("tag is not valid UTF-8 after percent-decode", mark))
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn is_tag_handle(handle: &str) -> bool {
    handle == "!"
        || (handle.len() >= 2
            && handle.starts_with('!')
            && handle.ends_with('!')
            && handle[1..handle.len() - 1]
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-'))
}

fn parse_error(message: &str, mark: Mark) -> Error {
    Error::Parse {
        message: message.into(),
        mark,
    }
}
