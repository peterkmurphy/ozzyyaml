/// How a scalar was written in the YAML source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarStyle {
    /// Unquoted scalar.
    Plain,
    /// `'…'` scalar.
    SingleQuoted,
    /// `"…"` scalar.
    DoubleQuoted,
    /// `|` block scalar.
    Literal,
    /// `>` block scalar.
    Folded,
}

/// A YAML 1.2.2 event in the parse stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Start of a YAML stream.
    StreamStart,
    /// End of a YAML stream.
    StreamEnd,
    /// Start of a document. `explicit` is true when `---` was present.
    DocumentStart {
        /// Whether the document started with an explicit `---` marker.
        explicit: bool,
    },
    /// End of a document. `explicit` is true when `...` was present.
    DocumentEnd {
        /// Whether the document ended with an explicit `...` marker.
        explicit: bool,
    },
    /// An alias node (`*name`).
    Alias {
        /// The referenced anchor name (Unicode allowed).
        anchor: String,
    },
    /// A scalar node.
    Scalar {
        /// Optional anchor name attached to this node.
        anchor: Option<String>,
        /// Optional tag.
        tag: Option<String>,
        /// Scalar content.
        value: String,
        /// Presentation style in the source.
        style: ScalarStyle,
    },
    /// Start of a sequence.
    SequenceStart {
        /// Optional anchor name attached to this node.
        anchor: Option<String>,
        /// Optional tag.
        tag: Option<String>,
        /// True when written in flow style (`[…]`).
        flow: bool,
    },
    /// End of a sequence.
    SequenceEnd,
    /// Start of a mapping.
    MappingStart {
        /// Optional anchor name attached to this node.
        anchor: Option<String>,
        /// Optional tag.
        tag: Option<String>,
        /// True when written in flow style (`{…}`).
        flow: bool,
    },
    /// End of a mapping.
    MappingEnd,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_event_holds_unicode_anchor_name() {
        let ev = Event::Scalar {
            anchor: Some("汉".into()),
            tag: None,
            value: "hi".into(),
            style: ScalarStyle::Plain,
        };
        match ev {
            Event::Scalar {
                anchor: Some(a), ..
            } => assert_eq!(a, "汉"),
            _ => panic!("wrong variant"),
        }
    }
}
