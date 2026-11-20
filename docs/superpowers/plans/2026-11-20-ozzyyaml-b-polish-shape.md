# ozzyyaml B Polish Shape Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close deferred B polish: byte-API tests, mark assertions, accessor negatives, and a small compose shape allowlist.

**Architecture:** Test-only changes in `ozzyyaml` (no public API change). Keep the Ok-only suite harness; add a second shape harness with hand-picked IDs.

**Tech Stack:** Rust 2021, existing `ozzyyaml` / vendor yaml-test-suite.

## Global Constraints

- `#![forbid(unsafe_code)]`
- Never allowlist a failing or shape-mismatched suite case
- No emit / Serde / PyYAML-compat / Python in this plan
- Bad input → `Err`, never panic

---

### Task 1: Byte API + mark + accessor tests

**Files:**
- Modify: `crates/ozzyyaml/tests/compose.rs`
- Modify: `crates/ozzyyaml/src/value.rs` (unit test only)

**Interfaces:**
- Consumes: existing `from_slice_with_limits`, `load_all_from_slice`, `load_all_from_slice_with_limits`, `Error::{InvalidUtf8,DuplicateAnchor,LimitExceeded}`, `Value` accessors

- [ ] **Step 1: Add failing/expanding tests in `compose.rs`**

```rust
use ozzyyaml::{
    from_slice, from_slice_with_limits, from_str, from_str_with_limits, load_all,
    load_all_from_slice, load_all_from_slice_with_limits, Error, Limits, Value,
};

#[test]
fn from_slice_with_limits_parses() {
    assert_eq!(
        from_slice_with_limits(b"7\n", Limits::default()).unwrap(),
        Value::Int(7)
    );
}

#[test]
fn load_all_from_slice_multi_doc() {
    let docs = load_all_from_slice(b"1\n---\n2\n").unwrap();
    assert_eq!(docs, vec![Value::Int(1), Value::Int(2)]);
}

#[test]
fn load_all_from_slice_rejects_invalid_utf8() {
    let err = load_all_from_slice(&[b'1', b'\n', 0xff]).unwrap_err();
    match err {
        Error::InvalidUtf8 { valid_up_to } => assert_eq!(valid_up_to, 2),
        other => panic!("expected InvalidUtf8, got {other:?}"),
    }
}

#[test]
fn load_all_from_slice_with_limits_rejects_invalid_utf8() {
    let err = load_all_from_slice_with_limits(&[0x80], Limits::default()).unwrap_err();
    match err {
        Error::InvalidUtf8 { valid_up_to: 0 } => {}
        other => panic!("expected InvalidUtf8, got {other:?}"),
    }
}

#[test]
fn duplicate_anchor_has_mark() {
    let err = from_str("[&a 1, &a 2]\n").unwrap_err();
    match err {
        Error::DuplicateAnchor { name, mark } => {
            assert_eq!(name, "a");
            assert!(mark.line >= 1);
            assert!(mark.column >= 1);
            // Second `&a` is not at stream start.
            assert!(mark.index > 0 || mark.column > 1);
        }
        other => panic!("expected DuplicateAnchor, got {other:?}"),
    }
}

#[test]
fn max_anchors_limit_has_mark() {
    let err = from_str_with_limits("&a 1\n", Limits::default().max_anchors(0)).unwrap_err();
    match err {
        Error::LimitExceeded { mark, limit: 0, .. } => {
            assert!(mark.line >= 1);
            assert!(mark.column >= 1);
        }
        other => panic!("expected LimitExceeded, got {other:?}"),
    }
}
```

In `value.rs` tests, replace/extend `scalar_accessors`:

```rust
#[test]
fn scalar_accessors() {
    assert_eq!(Value::Null.as_null(), Some(()));
    assert_eq!(Value::Bool(true).as_bool(), Some(true));
    assert_eq!(Value::Int(7).as_i64(), Some(7));
    assert_eq!(Value::Float(1.5).as_f64(), Some(1.5));
    assert_eq!(Value::String("a".into()).as_str(), Some("a"));

    assert_eq!(Value::Int(1).as_null(), None);
    assert_eq!(Value::Null.as_bool(), None);
    assert_eq!(Value::Bool(true).as_i64(), None);
    assert_eq!(Value::Int(1).as_f64(), None);
    assert_eq!(Value::Float(1.0).as_i64(), None);
    assert_eq!(Value::Int(1).as_str(), None);
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p ozzyyaml`

Expected: PASS (these exercise existing APIs — if any fail, fix assertions to match observed marks, do not weaken to `Mark::default()`).

- [ ] **Step 3: Commit**

```bash
git add crates/ozzyyaml/tests/compose.rs crates/ozzyyaml/src/value.rs
git commit -m "test(ozzyyaml): cover slice APIs, marks, and accessor negatives"
```

---

### Task 2: Compose shape allowlist harness

**Files:**
- Create: `crates/ozzyyaml/tests/suite_shape_allowlist.txt`
- Create: `crates/ozzyyaml/tests/yaml_test_suite_compose_shape.rs`

**Interfaces:**
- Consumes: `ozzyyaml::load_all`, `Value` accessors
- Shape rule: after `Ok(docs)`, for single-doc cases assert docs.len()==1 and root kind/length as recorded in the allowlist file format below

- [ ] **Step 1: Define allowlist format**

`suite_shape_allowlist.txt` lines (skip `#` and blank):

```
# id  kind  [len]
229Q  str
27NA  map  1
```

Kinds: `null` | `bool` | `int` | `float` | `str` | `seq` | `map`.  
`len` required for `seq`/`map`, omitted for scalars.

Start with **at least 10** IDs that already appear in `suite_allowlist.txt` and whose `in.yaml` matches the declared shape. Verify each by hand or a one-off probe before adding.

Suggested starters (verify before committing — drop any that fail):

```
229Q  str
26DV  str
27NA  map  1
2AUY  seq  2
4GC6  map  1
4UYU  str
8MK2  str
HWV9  str
K54U  map  1
98YD  str
```

(Adjust kinds/lens to match actual composed values; 8MK2/HWV9/K54U/98YD are on the **parser** allowlist — only use IDs that are also on the **compose Ok** allowlist or verified `load_all` Ok.)

- [ ] **Step 2: Write harness**

```rust
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
            let id = parts.next().unwrap().to_string();
            let kind = parts.next().expect("kind").to_string();
            let len = parts.next().map(|s| s.parse().expect("len"));
            Entry { id, kind, len }
        })
        .collect()
}

fn assert_shape(v: &Value, kind: &str, len: Option<usize>, id: &str) {
    match kind {
        "null" => assert!(v.as_null().is_some(), "{id}"),
        "bool" => assert!(v.as_bool().is_some(), "{id}"),
        "int" => assert!(v.as_i64().is_some(), "{id}"),
        "float" => assert!(v.as_f64().is_some(), "{id}"),
        "str" => assert!(v.as_str().is_some(), "{id}"),
        "seq" => {
            let s = v.as_sequence().unwrap_or_else(|| panic!("{id}: not seq"));
            assert_eq!(s.len(), len.expect("len"), "{id}");
        }
        "map" => {
            let m = v.as_mapping().unwrap_or_else(|| panic!("{id}: not map"));
            assert_eq!(m.len(), len.expect("len"), "{id}");
        }
        other => panic!("{id}: unknown kind {other}"),
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
                if let Err(msg) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    assert_shape(&docs[0], &e.kind, e.len, &e.id);
                })) {
                    failures.push(format!("{}: shape {:?}", e.id, msg));
                }
            }
            Ok(docs) => failures.push(format!("{}: expected 1 doc, got {}", e.id, docs.len())),
            Err(err) => failures.push(format!("{}: compose {err}", e.id)),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
```

Prefer asserting without `catch_unwind` if you collect errors another way; the important part is clear failure messages and no false allowlist entries.

- [ ] **Step 3: Verify**

Run: `cargo test -p ozzyyaml allowlisted_cases_match_root_shape`

Expected: PASS

Run: `cargo fmt --all && cargo clippy --workspace -- -D warnings && cargo test -p ozzyyaml`

Expected: clean

- [ ] **Step 4: Commit**

```bash
git add crates/ozzyyaml/tests/suite_shape_allowlist.txt crates/ozzyyaml/tests/yaml_test_suite_compose_shape.rs
git commit -m "test(ozzyyaml): add compose suite shape allowlist"
```

---

## Spec coverage

| Spec item | Task |
| --- | --- |
| Byte API tests | Task 1 |
| Mark assertions | Task 1 |
| Accessor negatives | Task 1 |
| Shape allowlist | Task 2 |
