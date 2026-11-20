//! Allowlisted compose success against yaml-test-suite `in.yaml`.
//! Only IDs listed in `suite_allowlist.txt`. Never allowlist a failing case.
//!
//! Criterion: `load_all(in.yaml)` returns `Ok(_)`. Value trees are not compared.

use ozzyyaml::load_all;
use std::path::PathBuf;

fn suite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/yaml-test-suite")
}

fn allowlist() -> Vec<String> {
    include_str!("suite_allowlist.txt")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

#[test]
fn allowlisted_cases_compose_ok() {
    let root = suite_root();
    assert!(
        root.join("229Q").is_dir(),
        "yaml-test-suite submodule missing; run `git submodule update --init`"
    );

    let ids = allowlist();
    assert!(!ids.is_empty(), "allowlist must not be empty");

    let mut failures = Vec::new();
    for id in &ids {
        let input = std::fs::read_to_string(root.join(id).join("in.yaml"))
            .unwrap_or_else(|e| panic!("{id}: cannot read in.yaml: {e}"));
        if let Err(e) = load_all(&input) {
            failures.push(format!("{id}: {e}"));
        }
    }

    assert!(
        failures.is_empty(),
        "compose failures:\n{}",
        failures.join("\n")
    );
}

/// Prints IDs of top-level suite cases (valid cases only: no `error` marker) whose
/// `in.yaml` composes successfully. Use to find candidates for `suite_allowlist.txt`.
#[test]
#[ignore = "manual discovery"]
fn discover_compose_passing_cases() {
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
        let Ok(input) = std::fs::read_to_string(dir.join("in.yaml")) else {
            continue;
        };
        let result = std::panic::catch_unwind(|| load_all(&input).is_ok());
        if matches!(result, Ok(true)) {
            passing.push(id);
        }
    }
    println!("PASSING_COUNT={}", passing.len());
    for id in &passing {
        println!("PASS {id}");
    }
}
