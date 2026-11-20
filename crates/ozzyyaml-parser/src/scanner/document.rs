//! Document markers (`---`, `...`), `%` directives and `!` tags.

use super::{scan_error, Scanner};
use crate::chars::is_flow_indicator;
use crate::error::Result;
use crate::mark::Mark;
use crate::token::Token;

#[allow(dead_code)]
impl Scanner<'_> {
    /// Returns the marker (`---` or `...`) at the reader position, if the
    /// reader is at column 1 outside flow context and the marker is followed
    /// by whitespace or end of input.
    pub(super) fn document_marker(&self, mark: Mark) -> Option<&'static str> {
        if mark.column != 1 || self.flow_level > 0 {
            return None;
        }
        let rest = self.reader.rest();
        let marker = ["---", "..."].into_iter().find(|m| rest.starts_with(m))?;
        match rest[3..].chars().next() {
            None | Some(' ' | '\t' | '\n' | '\r') => Some(marker),
            Some(_) => None,
        }
    }

    pub(super) fn fetch_document_marker(&mut self, marker: &str, mark: Mark) {
        self.unroll_indent(0, mark);
        self.possible_key = None;
        for _ in 0..3 {
            self.reader.bump();
        }
        self.queue.push_back(if marker == "---" {
            Token::DocumentStart { mark }
        } else {
            Token::DocumentEnd { mark }
        });
    }

    /// Scans a `%NAME arg…` directive line.
    pub(super) fn fetch_directive(&mut self, mark: Mark) {
        self.unroll_indent(0, mark);
        self.possible_key = None;
        self.reader.bump(); // '%'
        let mut line = String::new();
        while let Some(c) = self.reader.peek().filter(|&c| c != '\n') {
            if matches!(c, ' ' | '\t') && matches!(self.reader.peek2(), Some((_, '#'))) {
                break;
            }
            line.push(c);
            self.reader.bump();
        }
        let mut words = line.split_whitespace().map(String::from);
        let name = words.next().unwrap_or_default();
        let args = words.collect();
        self.queue.push_back(Token::Directive { name, args, mark });
    }

    /// Scans a tag: `!`, `!suffix`, `!!suffix`, `!handle!suffix` or `!<uri>`.
    pub(super) fn fetch_tag(&mut self, mark: Mark) -> Result<()> {
        self.save_possible_key(mark);
        self.reader.bump(); // '!'
        let (handle, suffix) = if self.reader.peek() == Some('<') {
            self.reader.bump();
            let mut uri = String::new();
            loop {
                match self.reader.bump() {
                    Some('>') => break,
                    Some(c) if !c.is_whitespace() => uri.push(c),
                    _ => return Err(scan_error("unterminated verbatim tag".into(), mark)),
                }
            }
            if uri.is_empty() {
                return Err(scan_error("verbatim tag must not be empty".into(), mark));
            }
            (String::new(), uri)
        } else {
            let mut word = String::new();
            while let Some(c) = self.reader.peek().filter(|&c| is_tag_char(c)) {
                word.push(c);
                self.reader.bump();
            }
            match word.rfind('!') {
                Some(i) => (format!("!{}", &word[..=i]), word[i + 1..].to_string()),
                None => ("!".to_string(), word),
            }
        };
        self.queue.push_back(Token::Tag {
            handle,
            suffix,
            mark,
        });
        Ok(())
    }
}

fn is_tag_char(c: char) -> bool {
    !c.is_whitespace() && !is_flow_indicator(c)
}
