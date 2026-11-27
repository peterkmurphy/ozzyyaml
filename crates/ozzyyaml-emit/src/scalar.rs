//! Scalar formatting: plain / single-quoted / double-quoted selection.

use ozzyyaml::Value;

/// Format a float so Core schema re-parses it as a float.
pub(crate) fn format_float(f: f64) -> String {
    if f.is_nan() {
        return ".nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { ".inf" } else { "-.inf" }.into();
    }
    let text = f.to_string();
    // `1` would re-parse as an int; keep it a float.
    if text.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        format!("{text}.0")
    } else {
        text
    }
}

/// Format a string scalar, quoting only when needed.
pub(crate) fn format_string(s: &str) -> String {
    if needs_double_quotes(s) {
        double_quoted(s)
    } else if needs_single_quotes(s) {
        format!("'{}'", s.replace('\'', "''"))
    } else {
        s.to_owned()
    }
}

/// YAML 1.2 §5.1 printable characters — everything else needs double-quoted escapes.
fn is_yaml_non_printable(c: char) -> bool {
    match c as u32 {
        0x9 | 0xa | 0xd => false,
        0x20..=0x7e => false,
        0x85 => false,
        0xa0..=0xd7ff => false,
        0xe000..=0xfffd => false,
        0x10000..=0x10ffff => false,
        _ => true,
    }
}

fn is_special_char(c: char) -> bool {
    // Non-printables and NBSP/BOM/line separators that plain/single cannot carry.
    is_yaml_non_printable(c)
        || matches!(
            c,
            '\u{85}' | '\u{a0}' | '\u{feff}' | '\u{2028}' | '\u{2029}'
        )
}

fn needs_double_quotes(s: &str) -> bool {
    s.chars().any(is_special_char)
}

fn needs_single_quotes(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    if s.starts_with(char::is_whitespace) || s.ends_with(char::is_whitespace) {
        return true;
    }
    if let Some(first) = s.chars().next() {
        if "-?:,[]{}#&*!|>'\"%@`".contains(first) {
            return true;
        }
    }
    if s.starts_with("---") || s.starts_with("...") {
        return true;
    }
    if s.contains(": ") || s.contains(" #") || s.ends_with(':') {
        return true;
    }
    if s.contains([',', '[', ']', '{', '}']) {
        return true;
    }
    // Would a plain scalar resolve to something other than this string?
    !matches!(ozzyyaml::from_str(s), Ok(Value::String(ref back)) if back == s)
}

/// Double-quoted form using the same escape set the parser accepts (§7.3.2).
fn double_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\0' => out.push_str("\\0"),
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{b}' => out.push_str("\\v"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '\u{1b}' => out.push_str("\\e"),
            '\u{85}' => out.push_str("\\N"),
            '\u{a0}' => out.push_str("\\_"),
            '\u{2028}' => out.push_str("\\L"),
            '\u{2029}' => out.push_str("\\P"),
            c if is_special_char(c) => {
                let n = c as u32;
                if n <= 0xff {
                    out.push_str(&format!("\\x{n:02x}"));
                } else if n <= 0xffff {
                    out.push_str(&format!("\\u{n:04x}"));
                } else {
                    out.push_str(&format!("\\U{n:08x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_named_escape_for_backspace() {
        assert_eq!(double_quoted("\u{8}"), "\"\\b\"");
    }

    #[test]
    fn emits_hex_for_other_controls() {
        assert_eq!(double_quoted("\u{1}"), "\"\\x01\"");
    }

    #[test]
    fn noncharacter_ffff_uses_double_quotes() {
        // U+FFFF is YAML-non-printable (§5.1) but not Rust `char::is_control`.
        assert!(needs_double_quotes("\u{ffff}"));
        assert_eq!(format_string("\u{ffff}"), "\"\\uffff\"");
        let yaml = format_string("x\u{ffff}y");
        let value = ozzyyaml::from_str(&format!("{yaml}\n")).expect("reparse");
        assert_eq!(value, Value::String("x\u{ffff}y".into()));
    }
}
