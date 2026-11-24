//! Runs allowlisted cases from the official yaml-test-suite (`data` branch layout:
//! one directory per case containing `in.yaml` and `test.event`).
//!
//! Only IDs listed in `suite_allowlist.txt` are asserted. Never add a case that fails.

use std::fmt::Write as _;
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

/// Render events in the yaml-test-suite `test.event` line format.
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

fn allowlist() -> Vec<String> {
    include_str!("suite_allowlist.txt")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

fn valid_event_suite_ids(root: &std::path::Path) -> Vec<String> {
    let mut ids: Vec<_> = std::fs::read_dir(root)
        .expect("suite root")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|id| {
            let d = root.join(id);
            d.join("in.yaml").is_file()
                && d.join("test.event").is_file()
                && !d.join("error").exists()
        })
        .collect();
    ids.sort();
    ids
}

#[test]
fn allowlisted_cases_match_expected_events() {
    let root = suite_root();
    assert!(
        root.join("229Q").is_dir(),
        "yaml-test-suite submodule missing; run `git submodule update --init`"
    );

    let ids = allowlist();
    assert!(!ids.is_empty(), "allowlist must not be empty");

    let mut failures = Vec::new();
    for id in &ids {
        let dir = root.join(id);
        let input = std::fs::read_to_string(dir.join("in.yaml"))
            .unwrap_or_else(|e| panic!("{id}: cannot read in.yaml: {e}"));
        let expected: Vec<String> = std::fs::read_to_string(dir.join("test.event"))
            .unwrap_or_else(|e| panic!("{id}: cannot read test.event: {e}"))
            .lines()
            .map(String::from)
            .collect();

        match parse_str(&input) {
            Ok(events) => {
                let actual = render(&events);
                if actual != expected {
                    failures.push(format!(
                        "{id}: events differ\n  expected: {expected:?}\n  actual:   {actual:?}"
                    ));
                }
            }
            Err(e) => failures.push(format!("{id}: parse error: {e}")),
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "Phase 0 report: run with --ignored --nocapture"]
fn report_all_valid_suite_failures() {
    let root = suite_root();
    let allow: std::collections::HashSet<_> = allowlist().into_iter().collect();
    let mut fail = 0usize;
    let mut pass = 0usize;
    for id in valid_event_suite_ids(&root) {
        let dir = root.join(&id);
        let input = std::fs::read_to_string(dir.join("in.yaml")).unwrap();
        let expected: Vec<String> = std::fs::read_to_string(dir.join("test.event"))
            .unwrap()
            .lines()
            .map(String::from)
            .collect();
        match parse_str(&input) {
            Ok(ev) if render(&ev) == expected => {
                pass += 1;
            }
            Ok(ev) => {
                fail += 1;
                let on_allow = allow.contains(&id);
                println!("FAIL {id} mismatch allowlisted={on_allow}");
                let _ = ev;
            }
            Err(e) => {
                fail += 1;
                println!("FAIL {id} error allowlisted={} {e}", allow.contains(&id));
            }
        }
    }
    println!("SUMMARY pass={pass} fail={fail}");
}

/// Prints IDs of top-level suite cases (in.yaml + test.event, no `error` marker) whose
/// rendered events exactly match. Use to find candidates for `suite_allowlist.txt`.
#[test]
#[ignore = "manual discovery"]
fn discover_passing_suite_cases() {
    let root = suite_root();
    let mut ids: Vec<String> = std::fs::read_dir(&root)
        .expect("suite root")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort();

    let mut passing = Vec::new();
    for id in ids {
        let dir = root.join(&id);
        if dir.join("error").exists() {
            continue;
        }
        let (Ok(input), Ok(ev)) = (
            std::fs::read_to_string(dir.join("in.yaml")),
            std::fs::read_to_string(dir.join("test.event")),
        ) else {
            continue;
        };
        let expected: Vec<String> = ev.lines().map(String::from).collect();
        let result = std::panic::catch_unwind(|| parse_str(&input).map(|e| render(&e)));
        if let Ok(Ok(actual)) = result {
            if actual == expected {
                passing.push(id);
            }
        }
    }
    println!("PASSING_COUNT={}", passing.len());
    for id in &passing {
        println!("PASS {id}");
    }
}
