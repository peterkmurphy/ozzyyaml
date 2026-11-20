use crate::event::ScalarStyle;
use crate::mark::Mark;

/// Lexical token produced by the scanner.
///
/// Each variant carries the source [`Mark`] where the token began. This shape
/// is the Milestone A convention (not a `Token { kind, mark }` pair).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum Token {
    StreamStart {
        mark: Mark,
    },
    StreamEnd {
        mark: Mark,
    },
    DocumentStart {
        mark: Mark,
    },
    DocumentEnd {
        mark: Mark,
    },
    /// A `%NAME arg…` directive line (e.g. `%YAML 1.2`, `%TAG ! prefix`).
    Directive {
        name: String,
        args: Vec<String>,
        mark: Mark,
    },
    /// Indent-aware start of a block sequence.
    BlockSequenceStart {
        mark: Mark,
    },
    /// Indent-aware start of a block mapping.
    BlockMappingStart {
        mark: Mark,
    },
    /// Indent-aware end of a block collection (`BLOCK-END`).
    BlockEnd {
        mark: Mark,
    },
    FlowSequenceStart {
        mark: Mark,
    },
    FlowSequenceEnd {
        mark: Mark,
    },
    FlowMappingStart {
        mark: Mark,
    },
    FlowMappingEnd {
        mark: Mark,
    },
    BlockEntry {
        mark: Mark,
    },
    FlowEntry {
        mark: Mark,
    },
    Key {
        mark: Mark,
    },
    Value {
        mark: Mark,
    },
    Anchor {
        name: String,
        mark: Mark,
    },
    Alias {
        name: String,
        mark: Mark,
    },
    Tag {
        handle: String,
        suffix: String,
        mark: Mark,
    },
    Scalar {
        value: String,
        style: ScalarStyle,
        mark: Mark,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::ScalarStyle;
    use crate::mark::Mark;

    #[test]
    fn scalar_token_holds_unicode_value_and_mark() {
        let t = Token::Scalar {
            value: "汉".into(),
            style: ScalarStyle::Plain,
            mark: Mark {
                index: 0,
                line: 1,
                column: 1,
            },
        };
        match t {
            Token::Scalar { value, mark, .. } => {
                assert_eq!(value, "汉");
                assert_eq!(mark.line, 1);
            }
            _ => panic!("wrong variant"),
        }
    }
}
