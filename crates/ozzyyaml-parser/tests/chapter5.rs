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

#[test]
fn tag_rejects_raw_non_ascii() {
    assert!(parse_str("!汉 value\n").is_err());
}

#[test]
fn tag_accepts_percent_encoded() {
    assert!(parse_str("!%E6%B1%89 value\n").is_ok());
}

#[test]
fn tag_rejects_non_tag_char_glued_to_suffix() {
    assert!(parse_str("!foo汉\n").is_err());
}

#[test]
fn verbatim_tag_rejects_non_uri() {
    assert!(parse_str("!<汉> value\n").is_err());
}

#[test]
fn bom_inside_document_errors() {
    let err = parse_str("- a\n\u{feff}- b\n").unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("byte order") || msg.contains("bom") || msg.contains("feff"),
        "{msg}"
    );
}

#[test]
fn bom_inside_double_quoted_ok() {
    assert!(parse_str("\"a\u{feff}b\"\n").is_ok());
}
