//! Default `max_nesting_depth` must be survivable for parse → emit → drop
//! on the platform default stack (debug and release). See fuzz-nightly
//! finding Nov 2026 (stack overflow at ~7k flow-seq with the old 10_000 default).

use ozzyyaml::{from_str, from_str_with_limits, Limits};
use ozzyyaml_emit::to_string;

fn default_depth() -> usize {
    Limits::default().max_nesting_depth
}

fn flow_seq(depth: usize) -> String {
    format!("{}{}\n", "[".repeat(depth), "]".repeat(depth))
}

fn flow_map(depth: usize) -> String {
    let mut s = String::new();
    for _ in 0..depth {
        s.push_str("{a: ");
    }
    s.push('1');
    for _ in 0..depth {
        s.push('}');
    }
    s.push('\n');
    s
}

fn block_seq(depth: usize) -> String {
    let mut s = String::new();
    for i in 0..depth {
        s.push_str(&"  ".repeat(i));
        if i + 1 == depth {
            s.push_str("- x\n");
        } else {
            s.push_str("-\n");
        }
    }
    s
}

fn roundtrip_ok(input: &str) {
    let value =
        from_str(input).unwrap_or_else(|e| panic!("parse failed: {e}; input len={}", input.len()));
    let yaml = to_string(&value).unwrap_or_else(|e| panic!("emit failed: {e}"));
    let _ = from_str(&yaml).unwrap_or_else(|e| panic!("reparse failed: {e}; yaml={yaml:?}"));
}

fn assert_rejected_as_nesting_limit(input: &str) {
    let err = from_str(input).expect_err("expected nesting-depth rejection");
    // Parser-enforced nesting surfaces as transparent `Error::Parse`.
    let msg = err.to_string();
    assert!(
        msg.contains("NestingDepth") || msg.to_ascii_lowercase().contains("nesting"),
        "expected NestingDepth limit, got: {err:?} / {msg}"
    );
}

#[test]
fn default_nesting_depth_is_stack_safe() {
    // Documented hotfix default; keep in sync with parser Limits::default.
    assert_eq!(default_depth(), 512);
}

#[test]
fn flow_seq_at_default_depth_roundtrips() {
    let d = default_depth();
    roundtrip_ok(&flow_seq(d));
    roundtrip_ok(&flow_seq(d.saturating_sub(1)));
}

#[test]
fn flow_map_at_default_depth_roundtrips() {
    let d = default_depth();
    roundtrip_ok(&flow_map(d));
    roundtrip_ok(&flow_map(d.saturating_sub(1)));
}

#[test]
fn block_seq_at_default_depth_roundtrips() {
    let d = default_depth();
    roundtrip_ok(&block_seq(d));
    roundtrip_ok(&block_seq(d.saturating_sub(1)));
}

#[test]
fn one_past_default_depth_is_clean_error() {
    let d = default_depth() + 1;
    assert_rejected_as_nesting_limit(&flow_seq(d));
    assert_rejected_as_nesting_limit(&flow_map(d));
}

#[test]
fn raised_limit_still_overrides() {
    let input = flow_seq(64);
    let v = from_str_with_limits(&input, Limits::default().max_nesting_depth(128)).unwrap();
    let _ = to_string(&v).unwrap();
}
