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

fn is_special_char(c: char) -> bool {
    c.is_control() || matches!(c, '\u{feff}' | '\u{2028}' | '\u{2029}')
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

fn double_quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if is_special_char(c) => {
                let n = c as u32;
                if n <= 0xff {
                    out.push_str(&format!("\\x{n:02x}"));
                } else {
                    out.push_str(&format!("\\u{n:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
