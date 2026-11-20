# ozzyyaml B Polish + Allowlist Growth Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Value accessors, `Parser::last_mark` for compose errors, `from_slice*` APIs, docs refresh, then grow parser + compose yaml-test-suite allowlists.

**Architecture:** Parser stashes the mark of the latest successful event; composer reads it on compose failures. Byte entry points UTF-8-check then reuse `&str` compose. Allowlists only gain IDs that already pass.

**Tech Stack:** Rust 2021, existing `ozzyyaml` / `ozzyyaml-parser` workspace, vendor yaml-test-suite submodule.

## Global Constraints

- `#![forbid(unsafe_code)]`
- Bad input → `Err`, never panic
- Never allowlist a failing yaml-test-suite case
- Do not add `Mark` to `Event` or `Value`
- No Serde / emit / PyYAML-compat / Python in this plan

---

### Task 1: `Parser::last_mark`

**Files:**
- Modify: `crates/ozzyyaml-parser/src/parser/mod.rs`
- Modify: `crates/ozzyyaml-parser/src/parser/tests.rs` (or add unit test there)
- Test: `cargo test -p ozzyyaml-parser last_mark`

**Interfaces:**
- Produces: `Parser::last_mark(&self) -> Mark` — mark of the event most recently returned by `next_event()` as `Ok(Some(_))`; `Mark::default()` before any event. On `Err`, leave the previous mark unchanged (parse `Error` already carries its own mark).

- [ ] **Step 1: Write the failing test**

In `crates/ozzyyaml-parser/src/parser/tests.rs` add:

```rust
#[test]
fn last_mark_tracks_alias_position() {
    let mut p = Parser::new("&a x\n*a\n").unwrap();
    assert_eq!(p.last_mark(), Mark::default());
    while let Some(ev) = p.next_event().unwrap() {
        if matches!(ev, Event::Alias { .. }) {
            let m = p.last_mark();
            assert!(m.line >= 2, "alias is on line 2, got {m}");
            return;
        }
    }
    panic!("expected Alias event");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ozzyyaml-parser last_mark_tracks_alias_position -- --nocapture`

Expected: FAIL (no `last_mark` method)

- [ ] **Step 3: Implement `last_mark`**

Add field on `Parser`:

```rust
last_mark: Mark,
```

Initialize to `Mark::default()` in `with_limits`.

Expose reader mark from scanner if needed. Minimal approach:

1. Add `pub(crate) fn mark(&self) -> Mark` on `Scanner` that returns `self.reader.mark()`.
2. In `next_event`:

```rust
pub fn next_event(&mut self) -> Result<Option<Event>> {
    let result = self.step();
    match &result {
        Ok(Some(_)) => {
            // Prefer mark of the token just consumed if still available via scanner;
            // otherwise reader mark after producing the event.
            self.last_mark = self.scanner.mark();
        }
        Ok(None) => {}
        Err(_) => {
            self.state = State::Done;
            self.stack.clear();
            self.peeked = None;
        }
    }
    result
}

pub fn last_mark(&self) -> Mark {
    self.last_mark
}
```

Refine if alias mark is wrong: when bumping a token that produces an event, assign `self.last_mark = token.mark()` at the production site (Alias / Scalar / collection starts). Acceptable implementation: set `last_mark` from `Token::mark()` inside `parse_node` and alias handling whenever those events are created; for structure events (`SequenceEnd`, etc.) use the end token’s mark. StreamStart/DocumentStart may remain `Mark::default()` or start-of-input.

Do whatever makes `last_mark_tracks_alias_position` pass with the alias on line 2.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p ozzyyaml-parser last_mark_tracks_alias_position`

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml-parser/src/parser/mod.rs crates/ozzyyaml-parser/src/scanner/mod.rs crates/ozzyyaml-parser/src/parser/tests.rs
git commit -m "feat(parser): expose last_mark for compose error positions"
```

---

### Task 2: Compose uses `last_mark` + `InvalidUtf8` + `from_slice*`

**Files:**
- Modify: `crates/ozzyyaml/src/error.rs`
- Modify: `crates/ozzyyaml/src/compose.rs`
- Modify: `crates/ozzyyaml/src/lib.rs`
- Modify: `crates/ozzyyaml/tests/compose.rs`

**Interfaces:**
- Consumes: `Parser::last_mark() -> Mark`
- Produces:
  - `Error::InvalidUtf8 { valid_up_to: usize }`
  - `from_slice(input: &[u8]) -> Result<Value>`
  - `from_slice_with_limits(input: &[u8], limits: Limits) -> Result<Value>`
  - `load_all_from_slice(input: &[u8]) -> Result<Vec<Value>>`
  - `load_all_from_slice_with_limits(input: &[u8], limits: Limits) -> Result<Vec<Value>>`

- [ ] **Step 1: Write failing tests**

In `crates/ozzyyaml/tests/compose.rs`:

```rust
#[test]
fn undefined_alias_has_nonzero_mark() {
    let err = from_str("x\n*missing\n").unwrap_err();
    match err {
        Error::UndefinedAlias { mark, .. } => {
            assert!(mark.line >= 2, "expected mark on alias line, got {mark}");
        }
        other => panic!("expected UndefinedAlias, got {other:?}"),
    }
}

#[test]
fn from_slice_rejects_invalid_utf8() {
    let err = from_slice(&[0x80]).unwrap_err();
    match err {
        Error::InvalidUtf8 { valid_up_to } => assert_eq!(valid_up_to, 0),
        other => panic!("expected InvalidUtf8, got {other:?}"),
    }
}

#[test]
fn from_slice_parses_utf8() {
    assert_eq!(from_slice(b"42\n").unwrap(), Value::Int(42));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p ozzyyaml undefined_alias_has_nonzero_mark from_slice -- --nocapture`

Expected: FAIL (missing `from_slice` / mark still default on line 1)

- [ ] **Step 3: Implement**

`error.rs` — add:

```rust
/// Input bytes were not valid UTF-8.
#[error("invalid UTF-8 at byte {valid_up_to}")]
InvalidUtf8 {
    /// Byte index of the first invalid sequence.
    valid_up_to: usize,
},
```

`compose.rs` — replace `Mark::default()` in compose error paths with `parser.last_mark()` (thread `&Parser` or pass `mark: Mark` into helpers). Update `register_anchor` / alias handling to accept a `Mark` argument from the caller who just saw the event.

Add:

```rust
pub fn from_slice(input: &[u8]) -> Result<Value> {
    from_slice_with_limits(input, Limits::default())
}

pub fn from_slice_with_limits(input: &[u8], limits: Limits) -> Result<Value> {
    let s = std::str::from_utf8(input).map_err(|e| Error::InvalidUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    from_str_with_limits(s, limits)
}

pub fn load_all_from_slice(input: &[u8]) -> Result<Vec<Value>> {
    load_all_from_slice_with_limits(input, Limits::default())
}

pub fn load_all_from_slice_with_limits(input: &[u8], limits: Limits) -> Result<Vec<Value>> {
    let s = std::str::from_utf8(input).map_err(|e| Error::InvalidUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    load_all_with_limits(s, limits)
}
```

Re-export new functions from `lib.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p ozzyyaml`

Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml/src/error.rs crates/ozzyyaml/src/compose.rs crates/ozzyyaml/src/lib.rs crates/ozzyyaml/tests/compose.rs
git commit -m "feat(ozzyyaml): from_slice APIs and mark-aware compose errors"
```

---

### Task 3: `Value` accessors

**Files:**
- Modify: `crates/ozzyyaml/src/value.rs`
- Test: unit tests in `value.rs`

**Interfaces:**
- Produces: `as_null() -> Option<()>`, `as_bool() -> Option<bool>`, `as_i64() -> Option<i64>`, `as_f64() -> Option<f64>`, `as_str() -> Option<&str>`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn scalar_accessors() {
    assert_eq!(Value::Null.as_null(), Some(()));
    assert_eq!(Value::Bool(true).as_bool(), Some(true));
    assert_eq!(Value::Int(7).as_i64(), Some(7));
    assert_eq!(Value::Float(1.5).as_f64(), Some(1.5));
    assert_eq!(Value::String("a".into()).as_str(), Some("a"));
    assert_eq!(Value::Int(1).as_str(), None);
}
```

- [ ] **Step 2: Run test — expect FAIL**

Run: `cargo test -p ozzyyaml scalar_accessors`

- [ ] **Step 3: Implement accessors**

```rust
pub fn as_null(&self) -> Option<()> {
    match self {
        Self::Null => Some(()),
        _ => None,
    }
}
pub fn as_bool(&self) -> Option<bool> { /* Bool */ }
pub fn as_i64(&self) -> Option<i64> { /* Int */ }
pub fn as_f64(&self) -> Option<f64> { /* Float */ }
pub fn as_str(&self) -> Option<&str> { /* String */ }
```

- [ ] **Step 4: Run test — expect PASS**

Run: `cargo test -p ozzyyaml scalar_accessors`

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml/src/value.rs
git commit -m "feat(ozzyyaml): add Value scalar accessors"
```

---

### Task 4: Docs refresh

**Files:**
- Modify: `docs/architecture.md`
- Modify: `README.md`

- [ ] **Step 1: Update architecture pipeline**

Replace the Milestone-A-only framing with:

```
&str / &[u8]
  → Reader → Scanner → Parser (events)
  → Composer (ozzyyaml) → Rc-shared Value (Core schema)
```

Document: `max_anchors` enforced at compose; collections use `RefCell` so cyclic aliases work; `Parser::last_mark` feeds compose errors; `from_str` / `load_all` / `from_slice*`.

- [ ] **Step 2: Touch README Develop section** if byte APIs or compose examples are missing — one short example:

```rust
use ozzyyaml::from_str;
let v = from_str("k: v\n").unwrap();
assert_eq!(v.as_mapping().unwrap()[0].1.as_str(), Some("v"));
```

- [ ] **Step 3: Commit**

```bash
git add docs/architecture.md README.md
git commit -m "docs: refresh architecture and README for compose"
```

---

### Task 5: Grow parser yaml-test-suite allowlist

**Files:**
- Modify: `crates/ozzyyaml-parser/tests/suite_allowlist.txt`
- Optionally add a small helper script under `crates/ozzyyaml-parser/tests/` only if useful; prefer one-shot shell during implementation

- [ ] **Step 1: Discover passing IDs not yet allowlisted**

From repo root (with submodule present):

```bash
cd /home/peter/projects/ozzyyaml
# Probe candidates: for each suite dir with in.yaml+test.event, try parse+render
# and collect IDs that match. Keep existing allowlist; append new passes only.
```

Concrete approach: write a temporary `cargo test` or use a one-off `rustc`/`cargo` binary — simplest is extend the existing harness temporarily with a `#[ignore]` discovery test, OR run:

```bash
python3 <<'PY'
# not a substitute for the Rust renderer — use Rust.
print("use cargo test discovery below")
PY
```

Preferred: add a **dev-only ignored test** `discover_passing_suite_cases` in `yaml_test_suite.rs` that prints IDs where render matches, then run it once, copy IDs into the allowlist, delete or keep `#[ignore]`.

```rust
#[test]
#[ignore = "manual discovery"]
fn discover_passing_suite_cases() {
    // walk suite_root children; skip if no test.event; try parse_str+render;
    // println IDs that match expected lines
}
```

Run: `cargo test -p ozzyyaml-parser discover_passing_suite_cases -- --ignored --nocapture`

- [ ] **Step 2: Append new IDs to `suite_allowlist.txt`** (sorted or grouped; keep comments). Only IDs printed as passing.

- [ ] **Step 3: Verify**

Run: `cargo test -p ozzyyaml-parser allowlisted_cases_match_expected_events`

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/ozzyyaml-parser/tests/suite_allowlist.txt crates/ozzyyaml-parser/tests/yaml_test_suite.rs
git commit -m "test(parser): grow yaml-test-suite allowlist"
```

---

### Task 6: Compose-level suite allowlist

**Files:**
- Create: `crates/ozzyyaml/tests/suite_allowlist.txt`
- Create: `crates/ozzyyaml/tests/yaml_test_suite_compose.rs`

**Interfaces:**
- Consumes: `load_all` / `from_str` from `ozzyyaml`
- Criterion for “pass”: `load_all(in.yaml)` returns `Ok(_)` (compose succeeds). Do **not** compare full Value trees to suite data in this pass unless a case has a trivial expected shape. Multi-doc → `load_all`; single-doc also fine via `load_all`.

- [ ] **Step 1: Scaffold harness**

```rust
//! Allowlisted compose success against yaml-test-suite `in.yaml`.
//! Only IDs listed in `suite_allowlist.txt`. Never allowlist a failing case.

use std::path::PathBuf;
use ozzyyaml::load_all;

fn suite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/yaml-test-suite")
}

fn allowlist() -> Vec<String> { /* same filter as parser */ }

#[test]
fn allowlisted_cases_compose_ok() {
    let root = suite_root();
    assert!(root.join("229Q").is_dir(), "submodule missing");
    let ids = allowlist();
    assert!(!ids.is_empty());
    let mut failures = Vec::new();
    for id in &ids {
        let input = std::fs::read_to_string(root.join(id).join("in.yaml")).unwrap();
        if let Err(e) = load_all(&input) {
            failures.push(format!("{id}: {e}"));
        }
    }
    assert!(failures.is_empty(), "compose failures:\n{}", failures.join("\n"));
}
```

Start `suite_allowlist.txt` with a few known-good IDs that parser already allowlists and that are single-ish documents (e.g. reuse IDs from parser list after verifying `load_all` succeeds).

- [ ] **Step 2: Discover more compose passes**

Add `#[ignore] fn discover_compose_passing_cases` that prints IDs where `load_all` is `Ok`. Run once; append a honest subset (aim for a meaningful bump, not the entire suite).

- [ ] **Step 3: Verify**

Run: `cargo test -p ozzyyaml allowlisted_cases_compose_ok`

Expected: PASS

- [ ] **Step 4: Full workspace check**

Run: `cargo fmt --all && cargo clippy --workspace -- -D warnings && cargo test --workspace`

Expected: clean

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml/tests/suite_allowlist.txt crates/ozzyyaml/tests/yaml_test_suite_compose.rs
git commit -m "test(ozzyyaml): add compose yaml-test-suite allowlist"
```

---

## Spec coverage check

| Spec item | Task |
| --- | --- |
| Accessors | Task 3 |
| `Parser::last_mark` | Task 1 |
| Compose uses marks | Task 2 |
| `InvalidUtf8` + `from_slice*` | Task 2 |
| Docs | Task 4 |
| Parser allowlist growth | Task 5 |
| Compose allowlist | Task 6 |
