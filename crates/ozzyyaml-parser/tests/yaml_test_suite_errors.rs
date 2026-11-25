//! Every yaml-test-suite case with an `error` marker must fail to parse.

use ozzyyaml_parser::parse_str;
use std::path::{Path, PathBuf};

fn suite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/yaml-test-suite")
}

fn error_suite_ids(root: &Path) -> Vec<String> {
    let mut ids: Vec<_> = std::fs::read_dir(root)
        .expect("suite root")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|id| {
            let d = root.join(id);
            d.join("in.yaml").is_file() && d.join("error").exists()
        })
        .collect();
    ids.sort();
    ids
}

#[test]
fn all_error_suite_cases_reject() {
    let root = suite_root();
    assert!(
        root.join("229Q").is_dir(),
        "yaml-test-suite submodule missing; run `git submodule update --init`"
    );

    let ids = error_suite_ids(&root);
    assert!(!ids.is_empty(), "no error suite cases found");

    let mut failures = Vec::new();
    for id in &ids {
        let input = std::fs::read_to_string(root.join(id).join("in.yaml"))
            .unwrap_or_else(|e| panic!("{id}: cannot read in.yaml: {e}"));
        match parse_str(&input) {
            Ok(_) => failures.push(format!("{id}: falsely accepted")),
            Err(_) => {}
        }
    }

    assert!(
        failures.is_empty(),
        "error-suite false accepts ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
