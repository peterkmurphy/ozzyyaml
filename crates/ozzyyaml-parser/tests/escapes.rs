//! Double-quoted escapes (YAML 1.2.2 §7.3.2 `c-ns-esc-char`).

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use ozzyyaml_parser::{parse_str, Event, ScalarStyle};

fn one_string(yaml: &str) -> String {
    parse_str(yaml)
        .unwrap()
        .into_iter()
        .find_map(|e| match e {
            Event::Scalar { value, .. } => Some(value),
            _ => None,
        })
        .unwrap()
}

#[test]
fn hex_escape_x() {
    assert_eq!(one_string("\"\\x41\"\n"), "A");
}

#[test]
fn unicode_escape_u() {
    assert_eq!(one_string("\"\\u0041\"\n"), "A");
}

#[test]
fn unicode_escape_u_smile() {
    assert_eq!(one_string("\"\\u263A\"\n"), "☺");
}

#[test]
fn unicode_escape_big_u() {
    assert_eq!(one_string("\"\\U0001F600\"\n"), "😀");
}

#[test]
fn bell_and_nbsp_letter_escapes() {
    assert_eq!(one_string("\"\\b\"\n"), "\u{8}");
    assert_eq!(one_string("\"\\_\"\n"), "\u{a0}");
}

#[test]
fn remaining_letter_escapes() {
    assert_eq!(
        one_string("\"\\a\\v\\f\\e\\N\\L\\P\"\n"),
        "\u{7}\u{b}\u{c}\u{1b}\u{85}\u{2028}\u{2029}"
    );
}

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

#[test]
fn suite_g4rs_escapes() {
    let id = "G4RS";
    let dir = suite_root().join(id);
    let yaml = fs::read_to_string(dir.join("in.yaml")).unwrap();
    let expected: Vec<String> = fs::read_to_string(dir.join("test.event"))
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    let events = parse_str(&yaml).unwrap_or_else(|e| panic!("{id}: {e}"));
    assert_eq!(render(&events), expected);
}
