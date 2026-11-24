//! Block scalars `|` / `>` (YAML 1.2.2 §8.1.4–§8.1.6).

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
    let got = render(&events);
    assert_eq!(got, expected, "suite {id} event mismatch");
}

fn scalar_of(yaml: &str) -> (String, ScalarStyle) {
    parse_str(yaml)
        .unwrap()
        .into_iter()
        .find_map(|e| match e {
            Event::Scalar { value, style, .. } => Some((value, style)),
            _ => None,
        })
        .expect("expected a scalar event")
}

#[test]
fn literal_preserves_newlines() {
    let (value, style) = scalar_of("|\n  a\n  b\n");
    assert_eq!(style, ScalarStyle::Literal);
    assert_eq!(value, "a\nb\n");
}

#[test]
fn literal_strip_chomp() {
    let (value, style) = scalar_of("|-\n  a\n");
    assert_eq!(style, ScalarStyle::Literal);
    assert_eq!(value, "a");
}

#[test]
fn literal_keep_chomp() {
    let (value, _) = scalar_of("|+\n  a\n\n");
    assert_eq!(value, "a\n\n");
}

#[test]
fn folded_joins_lines_with_space() {
    let (value, style) = scalar_of(">\n  Mark McGwire's\n  year was crippled\n  by a knee injury.\n");
    assert_eq!(style, ScalarStyle::Folded);
    assert_eq!(value, "Mark McGwire's year was crippled by a knee injury.\n");
}

#[test]
fn explicit_indent_indicator() {
    let (value, _) = scalar_of("- |1\n  explicit\n");
    assert_eq!(value, " explicit\n");
}

#[test]
fn suite_a6f9_chomping() {
    assert_suite_events("A6F9");
}

#[test]
fn suite_96l6_folded() {
    assert_suite_events("96L6");
}

#[test]
fn suite_4qfq_indent_indicators() {
    assert_suite_events("4QFQ");
}
