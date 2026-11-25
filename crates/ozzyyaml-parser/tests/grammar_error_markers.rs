//! Error-suite representatives for document markers, glued comments, and
//! markers inside open quoted scalars / flow.

use ozzyyaml_parser::parse_str;
use std::path::PathBuf;

fn suite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/yaml-test-suite")
}

fn assert_suite_errs(id: &str) {
    let yaml = std::fs::read_to_string(suite_root().join(id).join("in.yaml"))
        .unwrap_or_else(|e| panic!("{id}: {e}"));
    assert!(
        parse_str(&yaml).is_err(),
        "{id}: expected parse error, got Ok"
    );
}

#[test]
fn suite_3hfz_junk_after_document_end() {
    assert_suite_errs("3HFZ");
}

#[test]
fn suite_5trb_doc_marker_in_double_quoted() {
    assert_suite_errs("5TRB");
}

#[test]
fn suite_rxy3_doc_marker_in_single_quoted() {
    assert_suite_errs("RXY3");
}

#[test]
fn suite_n782_doc_marker_inside_flow() {
    assert_suite_errs("N782");
}

#[test]
fn suite_su5z_comment_without_whitespace() {
    assert_suite_errs("SU5Z");
}

#[test]
fn document_end_same_line_junk_errs() {
    assert!(parse_str("---\nk: v\n... invalid\n").is_err());
}

#[test]
fn glued_hash_after_quoted_errs() {
    assert!(parse_str("key: \"value\"# bad\n").is_err());
}
