//! Emit → parse round-trip.
//!
//! Covers hand-written escape cases (E1) and the compose allowlist (all of
//! which the parser handles). Cyclic documents are skipped
//! because structural `PartialEq` is not cycle-safe; equality uses float
//! bit-patterns so NaN round-trips.

use ozzyyaml::{from_str, load_all, Value};
use ozzyyaml_emit::{dump_all, to_string};
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

fn suite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/yaml-test-suite")
}

fn read_id_list(relative: &str) -> HashSet<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

/// True if `v` contains an `Rc` cycle (alias back to an ancestor).
fn is_cyclic(v: &Value) -> bool {
    fn walk(node: &Rc<Value>, stack: &mut HashSet<*const Value>) -> bool {
        let ptr = Rc::as_ptr(node);
        if !stack.insert(ptr) {
            return true;
        }
        let cyclic = match node.as_ref() {
            Value::Sequence(items) => items.borrow().iter().any(|c| walk(c, stack)),
            Value::Mapping(entries) => entries
                .borrow()
                .iter()
                .any(|(k, val)| walk(k, stack) || walk(val, stack)),
            _ => false,
        };
        stack.remove(&ptr);
        cyclic
    }
    let mut stack = HashSet::new();
    match v {
        Value::Sequence(items) => items.borrow().iter().any(|c| walk(c, &mut stack)),
        Value::Mapping(entries) => entries
            .borrow()
            .iter()
            .any(|(k, val)| walk(k, &mut stack) || walk(val, &mut stack)),
        _ => false,
    }
}

/// Structural equality that treats NaN as equal to NaN (via bits).
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x.to_bits() == y.to_bits(),
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Sequence(x), Value::Sequence(y)) => {
            let xb = x.borrow();
            let yb = y.borrow();
            xb.len() == yb.len() && xb.iter().zip(yb.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Mapping(x), Value::Mapping(y)) => {
            let xb = x.borrow();
            let yb = y.borrow();
            xb.len() == yb.len()
                && xb
                    .iter()
                    .zip(yb.iter())
                    .all(|((ak, av), (bk, bv))| values_equal(ak, bk) && values_equal(av, bv))
        }
        _ => false,
    }
}

#[test]
fn roundtrip_string_with_backspace() {
    let v = Value::String("\u{8}".into());
    let yaml = to_string(&v).unwrap();
    let back =
        from_str(&yaml).unwrap_or_else(|e| panic!("load own dump failed: {e}; yaml={yaml:?}"));
    assert_eq!(v, back, "yaml was {yaml:?}");
}

#[test]
fn roundtrip_string_with_nel_and_nbsp() {
    let v = Value::String("\u{85}\u{a0}".into());
    let yaml = to_string(&v).unwrap();
    let back =
        from_str(&yaml).unwrap_or_else(|e| panic!("load own dump failed: {e}; yaml={yaml:?}"));
    assert_eq!(v, back, "yaml was {yaml:?}");
}

#[test]
fn roundtrip_ascii_controls_and_emoji() {
    let v = Value::String("\u{7}\t\n😀".into());
    let yaml = to_string(&v).unwrap();
    let back =
        from_str(&yaml).unwrap_or_else(|e| panic!("load own dump failed: {e}; yaml={yaml:?}"));
    assert_eq!(v, back, "yaml was {yaml:?}");
}

#[test]
fn suite_allowlist_roundtrip_acyclic() {
    let root = suite_root();
    assert!(
        root.join("229Q").is_dir(),
        "yaml-test-suite submodule missing; run `git submodule update --init`"
    );

    // The parser now passes every valid `test.event` case, so the compose
    // allowlist alone selects the round-trip set.
    let mut ids: Vec<_> = read_id_list("tests/suite_roundtrip_allowlist.txt")
        .into_iter()
        .collect();
    ids.sort();
    assert!(
        ids.len() >= 100,
        "expected a large compose allowlist for round-trip, got {}",
        ids.len()
    );

    let mut tested = 0usize;
    let mut skipped_cycles = 0usize;
    let mut failures = Vec::new();

    for id in &ids {
        let path = root.join(id).join("in.yaml");
        let Ok(yaml) = std::fs::read_to_string(&path) else {
            continue;
        };
        let docs = match load_all(&yaml) {
            Ok(d) => d,
            Err(e) => {
                failures.push(format!("{id}: compose {e}"));
                continue;
            }
        };
        if docs.iter().any(is_cyclic) {
            skipped_cycles += 1;
            continue;
        }
        let emitted = match dump_all(&docs) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!("{id}: emit {e}"));
                continue;
            }
        };
        let back = match load_all(&emitted) {
            Ok(d) => d,
            Err(e) => {
                failures.push(format!("{id}: reload {e}; yaml={emitted:?}"));
                continue;
            }
        };
        if docs.len() != back.len()
            || docs
                .iter()
                .zip(back.iter())
                .any(|(a, b)| !values_equal(a, b))
        {
            failures.push(format!("{id}: value mismatch; emitted={emitted:?}"));
            continue;
        }
        tested += 1;
    }

    assert!(
        failures.is_empty(),
        "round-trip failures ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        tested >= 100,
        "too few acyclic round-trips tested ({tested}); skipped_cycles={skipped_cycles}"
    );
}
