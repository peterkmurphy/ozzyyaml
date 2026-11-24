//! Chapter 5 character production enforcement ([1]–[40], reserved, nb-json).

use ozzyyaml_parser::parse_str;

#[test]
fn reserved_at_rejects_plain() {
    assert!(parse_str("@text\n").is_err());
}

#[test]
fn reserved_grave_rejects_plain() {
    assert!(parse_str("`text\n").is_err());
}

#[test]
fn nonprintable_rejects_plain() {
    assert!(parse_str("\u{7}\n").is_err());
    assert!(parse_str("\u{fffe}\n").is_err());
}

#[test]
fn double_quoted_rejects_raw_c0() {
    assert!(parse_str("\"\u{1}\"\n").is_err());
}

#[test]
fn double_quoted_accepts_escaped_c0() {
    assert!(parse_str("\"\\a\"\n").is_ok());
}

#[test]
fn double_quoted_accepts_del_unescaped() {
    assert!(parse_str("\"\u{7f}\"\n").is_ok());
}

#[test]
fn quoted_at_is_ok() {
    assert!(parse_str("\"@text\"\n").is_ok());
}
