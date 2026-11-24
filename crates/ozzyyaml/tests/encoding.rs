//! Byte-stream encodings (YAML 1.2.2 §5.2 / §5.5).

use ozzyyaml::{from_slice, from_str, Value};

fn utf16le(s: &str) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFE];
    for u in s.encode_utf16() {
        out.extend(u.to_le_bytes());
    }
    out
}

fn utf16be(s: &str) -> Vec<u8> {
    let mut out = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        out.extend(u.to_be_bytes());
    }
    out
}

fn utf32le(s: &str) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFE, 0x00, 0x00];
    for c in s.chars() {
        out.extend((c as u32).to_le_bytes());
    }
    out
}

fn utf32be(s: &str) -> Vec<u8> {
    let mut out = vec![0x00, 0x00, 0xFE, 0xFF];
    for c in s.chars() {
        out.extend((c as u32).to_be_bytes());
    }
    out
}

fn assert_same_as_utf8(bytes: &[u8], utf8: &str) {
    let from_bytes = from_slice(bytes).unwrap_or_else(|e| panic!("bytes: {e}; utf8={utf8:?}"));
    let from_text = from_str(utf8).unwrap_or_else(|e| panic!("utf8: {e}"));
    assert_eq!(from_bytes, from_text);
}

#[test]
fn utf16le_bom_loads() {
    let bytes = utf16le("a\n");
    let v = from_slice(&bytes).unwrap();
    assert!(matches!(v, Value::String(s) if s == "a"));
}

#[test]
fn utf16be_bom_loads() {
    assert_same_as_utf8(&utf16be("true\n"), "true\n");
}

#[test]
fn utf32le_bom_loads() {
    assert_same_as_utf8(&utf32le("[1, 2]\n"), "[1, 2]\n");
}

#[test]
fn utf32be_bom_loads() {
    assert_same_as_utf8(&utf32be("k: v\n"), "k: v\n");
}

#[test]
fn utf8_bom_loads() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"42\n");
    assert_eq!(from_slice(&bytes).unwrap(), Value::Int(42));
}

#[test]
fn utf8_without_bom_still_works() {
    assert_eq!(from_slice(b"hello\n").unwrap(), Value::String("hello".into()));
}

#[test]
fn invalid_utf8_without_bom_is_error() {
    assert!(from_slice(&[0x80]).is_err());
}

#[test]
fn utf16le_without_bom_via_null_pattern() {
    let bytes = vec![0x61, 0x00, 0x0a, 0x00];
    assert_eq!(from_slice(&bytes).unwrap(), Value::String("a".into()));
}

#[test]
fn utf16be_without_bom_via_null_pattern() {
    let bytes = vec![0x00, 0x61, 0x00, 0x0a];
    assert_eq!(from_slice(&bytes).unwrap(), Value::String("a".into()));
}

#[test]
fn utf32le_without_bom_via_null_pattern() {
    let bytes = vec![0x61, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00];
    assert_eq!(from_slice(&bytes).unwrap(), Value::String("a".into()));
}

#[test]
fn utf32be_without_bom_via_null_pattern() {
    let bytes = vec![0x00, 0x00, 0x00, 0x61, 0x00, 0x00, 0x00, 0x0a];
    assert_eq!(from_slice(&bytes).unwrap(), Value::String("a".into()));
}
