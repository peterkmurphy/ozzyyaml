//! Shape checks for a small yaml-test-suite allowlist.
//! Format per line: `ID kind [len]`. Never allowlist a mismatch.

use std::path::PathBuf;

use ozzyyaml::{load_all, Value};

fn suite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/yaml-test-suite")
}

struct Entry {
    id: String,
    kind: String,
    len: Option<usize>,
}

fn entries() -> Vec<Entry> {
    include_str!("suite_shape_allowlist.txt")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let mut parts = l.split_whitespace();
            let id = parts.next().expect("id").to_string();
            let kind = parts.next().expect("kind").to_string();
            let len = parts.next().map(|s| s.parse().expect("len"));
            Entry { id, kind, len }
        })
        .collect()
}

/// Returns `Err` with a description when `v` does not match the declared shape.
fn check_shape(v: &Value, kind: &str, len: Option<usize>) -> Result<(), String> {
    let seq_len = |what: &str, actual: Option<usize>| -> Result<(), String> {
        let actual = actual.ok_or_else(|| format!("not a {what}"))?;
        let want = len.ok_or_else(|| format!("{what} entry missing len"))?;
        if actual == want {
            Ok(())
        } else {
            Err(format!("{what} len {actual}, expected {want}"))
        }
    };
    let ok = |cond: bool, what: &str| {
        if cond {
            Ok(())
        } else {
            Err(format!("not a {what}"))
        }
    };
    match kind {
        "null" => ok(v.as_null().is_some(), "null"),
        "bool" => ok(v.as_bool().is_some(), "bool"),
        "int" => ok(v.as_i64().is_some(), "int"),
        "float" => ok(v.as_f64().is_some(), "float"),
        "str" => ok(v.as_str().is_some(), "str"),
        "seq" => seq_len("seq", v.as_sequence().map(|s| s.len())),
        "map" => seq_len("map", v.as_mapping().map(|m| m.len())),
        other => Err(format!("unknown kind {other}")),
    }
}

#[test]
fn allowlisted_cases_match_root_shape() {
    let root = suite_root();
    assert!(root.join("229Q").is_dir(), "submodule missing");
    let list = entries();
    assert!(list.len() >= 10, "need at least 10 shape cases");
    let mut failures = Vec::new();
    for e in &list {
        let input = std::fs::read_to_string(root.join(&e.id).join("in.yaml"))
            .unwrap_or_else(|err| panic!("{}: {err}", e.id));
        match load_all(&input) {
            Ok(docs) if docs.len() == 1 => {
                if let Err(msg) = check_shape(&docs[0], &e.kind, e.len) {
                    failures.push(format!("{}: shape mismatch: {msg}", e.id));
                }
            }
            Ok(docs) => failures.push(format!("{}: expected 1 doc, got {}", e.id, docs.len())),
            Err(err) => failures.push(format!("{}: compose {err}", e.id)),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
