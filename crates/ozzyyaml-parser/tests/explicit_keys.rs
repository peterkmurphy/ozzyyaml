//! Explicit mapping keys (`?`) — YAML 1.2.2 §8.2.1 / §7.4.2.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use ozzyyaml_parser::{parse_str, Event, ScalarStyle};

fn suite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/yaml-test-suite")
}

fn escape_value(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '\u{8}' => out.push_str("\\b"),
            c => out.push(c),
        }
    }
    out
}

fn props(anchor: &Option<String>, tag: &Option<String>) -> String {
    let mut s = String::new();
    if let Some(a) = anchor {
        let _ = write!(s, " &{a}");
    }
    if let Some(t) = tag {
        let _ = write!(s, " <{t}>");
    }
    s
}

fn render(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .map(|ev| match ev {
            Event::StreamStart => "+STR".to_string(),
            Event::StreamEnd => "-STR".to_string(),
            Event::DocumentStart { explicit } => {
                if *explicit {
                    "+DOC ---".to_string()
                } else {
                    "+DOC".to_string()
                }
            }
            Event::DocumentEnd { explicit } => {
                if *explicit {
                    "-DOC ...".to_string()
                } else {
                    "-DOC".to_string()
                }
            }
            Event::Alias { anchor } => format!("=ALI *{anchor}"),
            Event::Scalar {
                anchor,
                tag,
                value,
                style,
            } => {
                let ind = match style {
                    ScalarStyle::Plain => ':',
                    ScalarStyle::SingleQuoted => '\'',
                    ScalarStyle::DoubleQuoted => '"',
                    ScalarStyle::Literal => '|',
                    ScalarStyle::Folded => '>',
                };
                format!("=VAL{} {}{}", props(anchor, tag), ind, escape_value(value))
            }
            Event::SequenceStart { anchor, tag, flow } => {
                let f = if *flow { " []" } else { "" };
                format!("+SEQ{}{}", f, props(anchor, tag))
            }
            Event::SequenceEnd => "-SEQ".to_string(),
            Event::MappingStart { anchor, tag, flow } => {
                let f = if *flow { " {}" } else { "" };
                format!("+MAP{}{}", f, props(anchor, tag))
            }
            Event::MappingEnd => "-MAP".to_string(),
        })
        .collect()
}

fn assert_suite_events(id: &str) {
    let dir = suite_root().join(id);
    let yaml = fs::read_to_string(dir.join("in.yaml")).unwrap_or_else(|e| panic!("{id}: {e}"));
    let expected: Vec<String> = fs::read_to_string(dir.join("test.event"))
        .unwrap_or_else(|e| panic!("{id}: {e}"))
        .lines()
        .map(String::from)
        .collect();
    let events = parse_str(&yaml).unwrap_or_else(|e| panic!("{id}: parse failed: {e}"));
    assert_eq!(render(&events), expected, "suite {id} event mismatch");
}

#[test]
fn explicit_key_with_value() {
    let events = parse_str("? a\n: b\n").unwrap();
    let scalars: Vec<_> = events
        .into_iter()
        .filter_map(|e| match e {
            Event::Scalar { value, .. } => Some(value),
            _ => None,
        })
        .collect();
    assert_eq!(scalars, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn explicit_key_empty_value() {
    let events = parse_str("? a\n").unwrap();
    let scalars: Vec<_> = events
        .into_iter()
        .filter_map(|e| match e {
            Event::Scalar { value, .. } => Some(value),
            _ => None,
        })
        .collect();
    assert_eq!(scalars, vec!["a".to_string(), String::new()]);
}

#[test]
fn complex_sequence_key() {
    let events = parse_str("? - a\n: b\n").unwrap();
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::SequenceStart { .. })));
    let scalars: Vec<_> = events
        .into_iter()
        .filter_map(|e| match e {
            Event::Scalar { value, .. } => Some(value),
            _ => None,
        })
        .collect();
    assert_eq!(scalars, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn suite_35kp() {
    assert_suite_events("35KP");
}

#[test]
fn suite_dff7_flow() {
    assert_suite_events("DFF7");
}

#[test]
fn suite_kk5p_complex() {
    assert_suite_events("KK5P");
}
