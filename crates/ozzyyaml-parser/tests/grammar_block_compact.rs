//! Compact / same-indent block collections — YAML 1.2.2 §8.2.1–§8.2.3
//! [183]–[195]: `c-l-block-seq-entry`, `ns-l-block-map-entry`,
//! `ns-l-compact-sequence`, `ns-l-compact-mapping`, and zero-indent
//! sequences under explicit `?`/`:` keys.

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

/// Same-indent sequence under a mapping value (`-` at the key's column).
#[test]
fn suite_az63() {
    assert_suite_events("AZ63");
}

/// Same-indent sequence under one key, deeper sequence under the next.
#[test]
fn suite_rlu9() {
    assert_suite_events("RLU9");
}

/// Same-indent sequence plus compact mapping with explicit `?`/`:`.
#[test]
fn suite_s9e8() {
    assert_suite_events("S9E8");
}

/// Tagged nested collections (`!!seq` / `!!map`) with compact children.
#[test]
fn suite_57h4() {
    assert_suite_events("57H4");
}

/// Empty key + sequence value, and quoted-key compact sequence.
#[test]
fn suite_s3pd() {
    assert_suite_events("S3PD");
}

/// Anchor before a zero-indent sequence (properties apply to the sequence).
#[test]
fn suite_ske5() {
    assert_suite_events("SKE5");
}

/// Explicit `?`/`:` keys whose values are zero-indent sequences.
#[test]
fn suite_6pbe() {
    assert_suite_events("6PBE");
}

/// Sequence entry that looks like two with wrong indentation (multiline plain).
#[test]
fn suite_ab8u() {
    assert_suite_events("AB8U");
}

/// Sibling `-` at the collection indent after a plain scalar starts a new entry
/// (must not fold into the previous scalar — complement of AB8U).
#[test]
fn sibling_block_entry_after_plain_is_new_entry() {
    let events = parse_str("- hello\n- world\n").unwrap();
    let scalars: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::Scalar { value, .. } => Some(value.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(scalars, ["hello", "world"]);
}
