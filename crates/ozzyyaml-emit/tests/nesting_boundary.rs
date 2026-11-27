//! Default `max_nesting_depth` (10 000) must be survivable for parse → emit →
//! drop on a small stack now that tree walks are iterative.

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
fn default_nesting_depth_is_ten_thousand() {
    assert_eq!(default_depth(), 10_000);
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

#[test]
fn emit_deep_flow_seq_on_small_stack() {
    // Writer + Anchors + Drop must not grow stack with depth.
    const DEPTH: usize = 10_000;
    const STACK: usize = 1024 * 1024;
    let handle = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(|| {
            let input = flow_seq(DEPTH);
            let value = from_str(&input).expect("compose at default depth");
            let yaml = to_string(&value).expect("emit deep flow-seq");
            assert!(
                yaml.starts_with('['),
                "unexpected emit: {}",
                &yaml[..yaml.len().min(32)]
            );
            drop(value);
        })
        .expect("spawn");
    handle.join().expect("emit thread panicked or overflowed");
}

#[test]
fn hundred_thousand_deep_on_small_stack_never_crashes() {
    // Plan Task 6: far past the default — clean LimitExceeded, or success with
    // an override; never SIGSEGV.
    const DEPTH: usize = 100_000;
    const STACK: usize = 1024 * 1024;
    let handle = std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(|| {
            assert_rejected_as_nesting_limit(&flow_seq(DEPTH));
            let value = from_str_with_limits(
                &flow_seq(DEPTH),
                Limits::default().max_nesting_depth(DEPTH + 1),
            )
            .expect("compose with raised limit");
            let yaml = to_string(&value).expect("emit 100k-deep flow-seq");
            assert!(yaml.starts_with('['));
            drop(value);
        })
        .expect("spawn");
    handle
        .join()
        .expect("100k-deep thread panicked or overflowed");
}
