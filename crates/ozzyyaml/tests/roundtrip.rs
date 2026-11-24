//! Emit → parse round-trip (guards E1: emitter escapes must be parseable).

use ozzyyaml::{from_str, Value};
use ozzyyaml_emit::to_string;

#[test]
fn roundtrip_string_with_backspace() {
    let v = Value::String("\u{8}".into());
    let yaml = to_string(&v).unwrap();
    let back = from_str(&yaml).unwrap_or_else(|e| panic!("load own dump failed: {e}; yaml={yaml:?}"));
    assert_eq!(v, back, "yaml was {yaml:?}");
}

#[test]
fn roundtrip_string_with_nel_and_nbsp() {
    let v = Value::String("\u{85}\u{a0}".into());
    let yaml = to_string(&v).unwrap();
    let back = from_str(&yaml).unwrap_or_else(|e| panic!("load own dump failed: {e}; yaml={yaml:?}"));
    assert_eq!(v, back, "yaml was {yaml:?}");
}

#[test]
fn roundtrip_ascii_controls_and_emoji() {
    let v = Value::String("\u{7}\t\n😀".into());
    let yaml = to_string(&v).unwrap();
    let back = from_str(&yaml).unwrap_or_else(|e| panic!("load own dump failed: {e}; yaml={yaml:?}"));
    assert_eq!(v, back, "yaml was {yaml:?}");
}
