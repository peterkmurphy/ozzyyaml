//! YAML 1.2.2 character classes.

/// `c-printable`: characters allowed to appear in a YAML stream.
pub(crate) fn is_printable(c: char) -> bool {
    matches!(c,
        '\t' | '\n' | '\r' | '\u{20}'..='\u{7E}' | '\u{85}'
        | '\u{A0}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')
        && c != '\u{FEFF}'
}

/// `c-flow-indicator`: `,` `[` `]` `{` `}`.
pub(crate) fn is_flow_indicator(c: char) -> bool {
    matches!(c, ',' | '[' | ']' | '{' | '}')
}

/// `ns-anchor-char`: a non-space printable character that is not a flow
/// indicator. Unlike libyaml, this deliberately includes non-ASCII characters.
pub(crate) fn is_anchor_char(c: char) -> bool {
    is_printable(c) && !matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{85}') && !is_flow_indicator(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_chars_include_unicode_but_not_space_or_flow_indicators() {
        for c in ['a', '-', '_', '汉', '😀', 'é', ':'] {
            assert!(is_anchor_char(c), "{c:?}");
        }
        for c in [
            ' ', '\t', '\n', ',', '[', ']', '{', '}', '\u{FEFF}', '\u{0}',
        ] {
            assert!(!is_anchor_char(c), "{c:?}");
        }
    }
}
