//! YAML 1.2.2 Chapter 5 character classes (`[1]`–`[40]`).

/// `[1] c-printable`
pub(crate) fn is_printable(c: char) -> bool {
    matches!(
        c,
        '\t'
            | '\n'
            | '\r'
            | '\u{20}'..='\u{7E}'
            | '\u{85}'
            | '\u{A0}'..='\u{D7FF}'
            | '\u{E000}'..='\u{FFFD}'
            | '\u{10000}'..='\u{10FFFF}'
    )
}

/// `[2] nb-json`
pub(crate) fn is_nb_json(c: char) -> bool {
    c == '\t' || ('\u{20}'..=char::MAX).contains(&c)
}

/// `[23] c-flow-indicator`
pub(crate) fn is_flow_indicator(c: char) -> bool {
    matches!(c, ',' | '[' | ']' | '{' | '}')
}

/// `[21] c-reserved`
pub(crate) fn is_reserved(c: char) -> bool {
    matches!(c, '@' | '`')
}

/// `[22] c-indicator`
pub(crate) fn is_indicator(c: char) -> bool {
    matches!(
        c,
        '-' | '?' | ':' | ',' | '[' | ']' | '{' | '}' | '#' | '&' | '*' | '!' | '|' | '>' | '\''
            | '"' | '%' | '@' | '`'
    )
}

/// `[27] nb-char` = `c-printable - b-char - c-byte-order-mark`
pub(crate) fn is_nb_char(c: char) -> bool {
    is_printable(c) && !matches!(c, '\n' | '\r' | '\u{FEFF}')
}

/// `[31] s-space`
pub(crate) fn is_space(c: char) -> bool {
    c == ' '
}

/// `[32] s-tab`
pub(crate) fn is_tab(c: char) -> bool {
    c == '\t'
}

/// `[33] s-white`
pub(crate) fn is_white(c: char) -> bool {
    is_space(c) || is_tab(c)
}

/// `[34] ns-char` = `nb-char - s-white`
pub(crate) fn is_ns_char(c: char) -> bool {
    is_nb_char(c) && !is_white(c)
}

/// `[35] ns-dec-digit`
pub(crate) fn is_dec_digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// `[36] ns-hex-digit`
pub(crate) fn is_hex_digit(c: char) -> bool {
    c.is_ascii_hexdigit()
}

/// `[37] ns-ascii-letter`
pub(crate) fn is_ascii_letter(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// `[38] ns-word-char`
pub(crate) fn is_word_char(c: char) -> bool {
    is_dec_digit(c) || is_ascii_letter(c) || c == '-'
}

/// Single-character branch of `[39] ns-uri-char` (not the `%` + 2 hex form).
pub(crate) fn is_uri_char(c: char) -> bool {
    is_word_char(c)
        || matches!(
            c,
            '#' | ';'
                | '/'
                | '?'
                | ':'
                | '@'
                | '&'
                | '='
                | '+'
                | '$'
                | ','
                | '_'
                | '.'
                | '!'
                | '~'
                | '*'
                | '\''
                | '('
                | ')'
                | '['
                | ']'
        )
}

/// `[40] ns-tag-char` single-char branch = uri char − `!` − flow indicators.
pub(crate) fn is_tag_char(c: char) -> bool {
    is_uri_char(c) && c != '!' && !is_flow_indicator(c)
}

/// If `s` starts with `%` + 2 hex digits (`[39]` escape form), return length 3.
pub(crate) fn uri_percent_escape_len(s: &str) -> Option<usize> {
    let mut chars = s.chars();
    if chars.next() != Some('%') {
        return None;
    }
    let a = chars.next()?;
    let b = chars.next()?;
    if is_hex_digit(a) && is_hex_digit(b) {
        Some(3)
    } else {
        None
    }
}

/// Number of chars in one `ns-uri-char` at the start of `s` (1, or 3 for `%HH`).
pub(crate) fn uri_char_len_at(s: &str) -> Option<usize> {
    if let Some(len) = uri_percent_escape_len(s) {
        return Some(len);
    }
    let c = s.chars().next()?;
    if is_uri_char(c) {
        Some(1)
    } else {
        None
    }
}

/// Number of chars in one `ns-tag-char` at the start of `s` (`%HH` or one tag char).
pub(crate) fn tag_char_len_at(s: &str) -> Option<usize> {
    if let Some(len) = uri_percent_escape_len(s) {
        return Some(len);
    }
    let c = s.chars().next()?;
    if is_tag_char(c) {
        Some(1)
    } else {
        None
    }
}

/// Practical anchor name character (not `[102]`; deferred). Kept for current scanner.
pub(crate) fn is_anchor_char(c: char) -> bool {
    is_printable(c)
        && !matches!(
            c,
            ' ' | '\t' | '\n' | '\r' | '\u{85}' | '"' | '\'' | '#' | '\u{FEFF}'
        )
        && !is_flow_indicator(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printable_matches_c_printable() {
        assert!(is_printable('\t'));
        assert!(is_printable('a'));
        assert!(is_printable('\u{85}'));
        assert!(!is_printable('\u{7f}'));
        assert!(!is_printable('\u{fffe}'));
        assert!(!is_printable('\u{ffff}'));
        assert!(!is_printable('\0'));
        // BOM is inside [xE000-xFFFD] in [1]; [27] subtracts it from nb-char.
        assert!(is_printable('\u{feff}'));
        assert!(!is_nb_char('\u{feff}'));
    }

    #[test]
    fn nb_json_and_nb_char() {
        assert!(is_nb_json('\t'));
        assert!(is_nb_json('\u{7f}'));
        assert!(!is_nb_json('\u{1}'));
        assert!(is_nb_char('a'));
        assert!(!is_nb_char('\n'));
        assert!(!is_nb_char('\r'));
        assert!(!is_nb_char('\u{feff}'));
        assert!(!is_nb_char('\u{7}'));
    }

    #[test]
    fn reserved_and_indicators() {
        assert!(is_reserved('@'));
        assert!(is_reserved('`'));
        assert!(!is_reserved('a'));
        assert!(is_indicator('-'));
        assert!(is_indicator('@'));
        assert!(is_flow_indicator(','));
        assert!(!is_flow_indicator(':'));
    }

    #[test]
    fn uri_and_tag_chars() {
        assert!(is_uri_char('A'));
        assert!(is_uri_char('@'));
        assert!(!is_uri_char('汉'));
        assert!(!is_uri_char('%')); // bare % is not a complete uri-char
        assert_eq!(uri_percent_escape_len("%E6xx"), Some(3));
        assert_eq!(uri_percent_escape_len("%ZZ"), None);
        assert!(is_tag_char('a'));
        assert!(!is_tag_char('!'));
        assert!(!is_tag_char(','));
        assert_eq!(tag_char_len_at("%2Frest"), Some(3));
        assert_eq!(tag_char_len_at("!foo"), None);
    }

    #[test]
    fn digits_and_word() {
        assert!(is_dec_digit('0'));
        assert!(is_hex_digit('f'));
        assert!(is_hex_digit('F'));
        assert!(!is_hex_digit('g'));
        assert!(is_ascii_letter('Z'));
        assert!(is_word_char('-'));
        assert!(is_ns_char('a'));
        assert!(!is_ns_char(' '));
        assert!(is_white('\t'));
        assert!(is_space(' '));
    }

    #[test]
    fn anchor_chars_include_unicode_but_not_space_or_flow_indicators() {
        for c in ['a', '-', '_', '汉', '😀', 'é', ':'] {
            assert!(is_anchor_char(c), "{c:?}");
        }
        for c in [
            ' ', '\t', '\n', ',', '[', ']', '{', '}', '"', '\'', '#', '\u{FEFF}', '\u{0}',
        ] {
            assert!(!is_anchor_char(c), "{c:?}");
        }
    }
}
