//! Every valid yaml-test-suite case (in.yaml + test.event, no error marker)
//! must compose via `load_all`. Value trees are not compared.

use ozzyyaml::load_all;
use std::path::{Path, PathBuf};

fn suite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/yaml-test-suite")
}

fn valid_compose_suite_ids(root: &Path) -> Vec<String> {
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
fn all_valid_cases_compose_ok() {
    let root = suite_root();
    assert!(
        root.join("229Q").is_dir(),
        "yaml-test-suite submodule missing; run `git submodule update --init`"
    );

    let ids = valid_compose_suite_ids(&root);
    assert!(!ids.is_empty(), "no valid suite cases found");

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
        "compose failures ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
