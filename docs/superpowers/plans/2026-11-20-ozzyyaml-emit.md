# ozzyyaml Emit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `ozzyyaml-emit` — dump `Value` / `&[Value]` to YAML text with flow default, optional block style, and `Rc`-identity anchors/aliases (Unicode-capable).

**Architecture:** New workspace crate depends on `ozzyyaml`. Emitter walks `Value`, tracks seen `*const Value` for aliases, writes flow or block syntax per `EmitOptions`. Multi-doc via `dump_all` with per-document anchor maps.

**Tech Stack:** Rust 2021, `ozzyyaml` path dependency, existing workspace BSD-3-Clause.

## Global Constraints

- `#![forbid(unsafe_code)]`
- Bad graphs → `Err`, never panic (cycles allowed via aliases)
- Value-only input (no event emitter)
- Flow default; block configurable
- Not lossless
- No Serde / PyYAML-compat / Python / `io::Write` in this plan

---

### Task 1: Scaffold `ozzyyaml-emit`

**Files:**
- Create: `crates/ozzyyaml-emit/Cargo.toml`
- Create: `crates/ozzyyaml-emit/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1: Cargo.toml**

```toml
[package]
name = "ozzyyaml-emit"
description = "Emit ozzyyaml Value trees as YAML 1.2.2 text"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
rust-version.workspace = true

[dependencies]
ozzyyaml = { path = "../ozzyyaml" }
thiserror = "2"
```

Add `"crates/ozzyyaml-emit"` to workspace `members`.

- [ ] **Step 2: lib.rs stub**

```rust
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Emit [`ozzyyaml::Value`] trees as YAML 1.2.2 text.

mod emit;
mod error;
mod options;

pub use emit::{dump_all, dump_all_with_options, to_string, to_string_with_options};
pub use error::{Error, Result};
pub use options::{EmitOptions, EmitStyle};
```

Placeholder modules so the crate compiles: `error.rs` with empty/simple `Error`, `options.rs` with enums, `emit.rs` with `todo!()` or `unimplemented!` behind tests that do not yet call them — better: stubs returning `Ok(String::new())` temporarily only until Task 2; prefer failing tests first in Task 2.

Minimal compile stubs:

```rust
// error.rs
use thiserror::Error;
#[derive(Debug, Error, PartialEq)]
pub enum Error {
    #[error("emit error: {message}")]
    Emit { message: String },
}
pub type Result<T> = std::result::Result<T, Error>;

// options.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EmitStyle { #[default] Flow, Block }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOptions { pub style: EmitStyle }
impl Default for EmitOptions {
    fn default() -> Self { Self { style: EmitStyle::Flow } }
}

// emit.rs — temporary stubs returning Err until Task 2 implements
use crate::error::{Error, Result};
use crate::options::EmitOptions;
use ozzyyaml::Value;

pub fn to_string(value: &Value) -> Result<String> {
    to_string_with_options(value, &EmitOptions::default())
}
pub fn to_string_with_options(_value: &Value, _opts: &EmitOptions) -> Result<String> {
    Err(Error::Emit { message: "not implemented".into() })
}
pub fn dump_all(docs: &[Value]) -> Result<String> {
    dump_all_with_options(docs, &EmitOptions::default())
}
pub fn dump_all_with_options(_docs: &[Value], _opts: &EmitOptions) -> Result<String> {
    Err(Error::Emit { message: "not implemented".into() })
}
```

- [ ] **Step 3: Verify compile**

Run: `cargo test -p ozzyyaml-emit`

Expected: PASS (no tests yet) or lib builds

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml crates/ozzyyaml-emit
git commit -m "chore: scaffold ozzyyaml-emit crate"
```

---

### Task 2: Emit scalars + flow collections (no anchors yet)

**Files:**
- Modify: `crates/ozzyyaml-emit/src/emit.rs`
- Create: `crates/ozzyyaml-emit/tests/emit_basic.rs`

**Interfaces:**
- Produces working `to_string` / `to_string_with_options` for acyclic trees without shared `Rc` (each child uniquely owned or unshared)

- [ ] **Step 1: Failing tests**

```rust
use ozzyyaml::Value;
use ozzyyaml_emit::{to_string, to_string_with_options, EmitOptions, EmitStyle};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn emit_scalars_flow() {
    assert_eq!(to_string(&Value::Null).unwrap().trim(), "~");
    assert_eq!(to_string(&Value::Bool(true)).unwrap().trim(), "true");
    assert_eq!(to_string(&Value::Int(42)).unwrap().trim(), "42");
    assert_eq!(to_string(&Value::String("hi".into())).unwrap().trim(), "hi");
}

#[test]
fn emit_flow_sequence() {
    let v = Value::Sequence(RefCell::new(vec![
        Rc::new(Value::Int(1)),
        Rc::new(Value::Int(2)),
    ]));
    let s = to_string(&v).unwrap();
    assert!(s.contains('[') && s.contains(']'), "{s}");
    assert!(s.contains('1') && s.contains('2'), "{s}");
}

#[test]
fn emit_block_sequence_option() {
    let v = Value::Sequence(RefCell::new(vec![Rc::new(Value::String("a".into()))]));
    let opts = EmitOptions { style: EmitStyle::Block };
    let s = to_string_with_options(&v, &opts).unwrap();
    assert!(s.contains("- "), "{s}");
}
```

- [ ] **Step 2: Run — expect FAIL** (`not implemented` or wrong output)

Run: `cargo test -p ozzyyaml-emit emit_scalars_flow`

- [ ] **Step 3: Implement scalar + collection emit (flow/block)**

In `emit.rs`, recursive writer:

- Null → `~`
- Bool → `true`/`false`
- Int → decimal display
- Float → reasonable Debug/format that Core re-parses (`1.5`, `.nan`, `.inf` as needed)
- String → plain if safe (no leading indicators, no `: ` / `#`, etc.); else single-quoted with `''` escape
- Sequence/Mapping per `opts.style`

Do **not** emit anchors yet (unique visits only).

- [ ] **Step 4: Tests PASS**

Run: `cargo test -p ozzyyaml-emit`

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml-emit
git commit -m "feat(emit): dump scalars and collections (flow/block)"
```

---

### Task 3: Anchors / aliases by `Rc` identity

**Files:**
- Modify: `crates/ozzyyaml-emit/src/emit.rs`
- Create: `crates/ozzyyaml-emit/tests/emit_anchors.rs`

**Interfaces:**
- Produces: shared `Rc` → `&a` / `*a`; cycles emit without stack overflow; Unicode anchor names when the graph was composed with Unicode anchors **or** emitter can generate ASCII names for synthesized graphs — round-trip test must use compose→emit→compose with Unicode input:

```yaml
[&汉 x, *汉]
```

- [ ] **Step 1: Failing tests**

```rust
use ozzyyaml::{from_str, Value};
use ozzyyaml_emit::to_string;
use std::rc::Rc;

#[test]
fn emit_shared_alias_round_trip() {
    let v = from_str("[&a hello, *a]\n").unwrap();
    let out = to_string(&v).unwrap();
    assert!(out.contains('*') || out.contains('&'), "{out}");
    let v2 = from_str(&out).unwrap();
    let seq = v2.as_sequence().unwrap();
    assert!(Rc::ptr_eq(&seq[0], &seq[1]));
}

#[test]
fn emit_cycle_round_trip() {
    let v = from_str("&a [*a]\n").unwrap();
    let out = to_string(&v).unwrap();
    let v2 = from_str(&out).unwrap();
    let child = Rc::clone(&v2.as_sequence().unwrap()[0]);
    let g = Rc::clone(&child.as_sequence().unwrap()[0]);
    assert!(Rc::ptr_eq(&child, &g));
}

#[test]
fn emit_unicode_anchor_round_trip() {
    let v = from_str("[&汉 x, *汉]\n").unwrap();
    let out = to_string(&v).unwrap();
    assert!(out.contains('汉'), "{out}");
    let v2 = from_str(&out).unwrap();
    let seq = v2.as_sequence().unwrap();
    assert!(Rc::ptr_eq(&seq[0], &seq[1]));
}
```

Note: composed values do **not** retain original anchor **names** — only `Rc` identity. Emitter must **generate** names. For Unicode round-trip, either (a) generate Unicode names in tests via a test-only hook, or (b) generate ASCII names and change the Unicode test to assert sharing after round-trip **without** requiring `汉` in output. Spec prefers proving Unicode anchors work: generate names from a counter that can emit Unicode for the test, **or** keep original names if you add optional name hints later.

**Pragmatic v1 (spec-aligned):** Emitter generates ASCII `a`, `b`, `c`, … for assigned anchors. Separate unit test calls an internal/testable name encoder that escapes/accepts Unicode code points into a valid anchor spelling, and an integration test builds a `Value` graph and uses `to_string` after manually…  

Simplest approach matching the Unicode motivation: when emitting, allow `EmitOptions { style, anchor_prefix: Option<String> }` — **too much**.

**Chosen approach for this plan:** Generator yields `n1`, `n2`, … (ASCII). Unicode test becomes: parse Unicode-anchored YAML, emit (ASCII names OK), re-parse, assert `Rc::ptr_eq` sharing. Add a **unit** test that `format_anchor_name` / write path accepts a Unicode name string if we expose `emit_with_anchor_names` — YAGNI.

Add explicit test that **libyaml gap is covered at compose** already; emit Unicode by optionally using the character `汉` as a generated name for the first anchor in a `#[cfg(test)]` only path — cleaner:

```rust
// In emit, AnchorGen::next() -> String using "a", "b", ...
// Extra test: manually construct isn't needed.
// unicode test: round-trip sharing without requiring 汉 in output.
```

Update unicode test to:

```rust
#[test]
fn emit_unicode_anchor_round_trip() {
    let v = from_str("[&汉 x, *汉]\n").unwrap();
    let out = to_string(&v).unwrap();
    let v2 = from_str(&out).unwrap();
    let seq = v2.as_sequence().unwrap();
    assert!(Rc::ptr_eq(&seq[0], &seq[1]));
    assert_eq!(seq[0].as_str(), Some("x"));
}
```

Add one emitter unit test writing a Unicode anchor name through a small `pub(crate) fn write_alias(name: &str)` sanity check that `汉` is emitted when passed as the assigned name — implement AnchorGen that for tests can be seeded; **or** document that generated names are ASCII and Unicode is proven on the parse side.

Spec: "Unicode-capable names when testing round-trip of composed Unicode-anchored graphs". Round-trip sharing satisfies the spirit; add:

```rust
#[test]
fn emitted_anchor_names_may_be_unicode() {
    // Force a name by testing the scalar write helper:
    assert!(ozzyyaml_emit::debug_format_anchor("汉").contains('汉'));
}
```

Avoid `debug_format_anchor` on public API — use `#[cfg(test)]` module in `emit.rs`:

```rust
#[cfg(test)]
mod anchor_name_tests {
    #[test]
    fn unicode_name_is_written_verbatim() {
        let mut s = String::new();
        super::write_anchor_decl(&mut s, "汉").unwrap();
        assert!(s.contains("&汉"));
    }
}
```

- [ ] **Step 2: Implement identity map + alias emission**

Algorithm sketch:

1. First walk (optional): count `Rc` pointer occurrences.
2. Second walk: if count>1 or already in `assigned` and revisiting → alias; else if count>1 on first visit → assign name + emit anchored node.

For cycles: on first visit insert placeholder into `visiting`/`assigned` before recursing into children; child alias resolves.

- [ ] **Step 3: PASS + commit**

```bash
git add crates/ozzyyaml-emit
git commit -m "feat(emit): Rc-identity anchors and aliases"
```

---

### Task 4: `dump_all` multi-document streams

**Files:**
- Modify: `crates/ozzyyaml-emit/src/emit.rs`
- Create: `crates/ozzyyaml-emit/tests/emit_stream.rs`

- [ ] **Step 1: Failing tests**

```rust
use ozzyyaml::{from_str, load_all, Value};
use ozzyyaml_emit::{dump_all, to_string};

#[test]
fn dump_all_two_docs() {
    let docs = vec![Value::Int(1), Value::Int(2)];
    let s = dump_all(&docs).unwrap();
    assert!(s.contains("---"), "{s}");
    let back = load_all(&s).unwrap();
    assert_eq!(back, docs);
}

#[test]
fn dump_all_resets_anchors_per_doc() {
    let a = from_str("[&x 1, *x]\n").unwrap();
    let b = from_str("[&x 2, *x]\n").unwrap();
    let s = dump_all(&[a, b]).unwrap();
    let docs = load_all(&s).unwrap();
    assert_eq!(docs.len(), 2);
}
```

Convention: for `docs.len() > 1`, prefix each document with `---\n` (including the first). For `len == 1`, behave like `to_string` (no required `---`). For `len == 0`, return empty string or a single empty doc — choose empty string and document it.

- [ ] **Step 2: Implement `dump_all_with_options`**

Reset anchor state each document; join with `---`.

- [ ] **Step 3: PASS + commit**

```bash
git add crates/ozzyyaml-emit
git commit -m "feat(emit): dump_all multi-document streams"
```

---

### Task 5: Docs + README + workspace verify

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture.md` (short Emit section)

- [ ] **Step 1: README roadmap** — mark emit in progress/done; show:

```rust
use ozzyyaml::from_str;
use ozzyyaml_emit::to_string;
let v = from_str("[1, 2]\n").unwrap();
let _yaml = to_string(&v).unwrap();
```

- [ ] **Step 2: architecture.md** — Compose → Emit arrow; note flow default / block option / Rc anchors / `dump_all`.

- [ ] **Step 3: Full check**

Run: `cargo fmt --all && cargo clippy --workspace -- -D warnings && cargo test --workspace`

Expected: clean

- [ ] **Step 4: Commit**

```bash
git add README.md docs/architecture.md
git commit -m "docs: document ozzyyaml-emit"
```

---

## Spec coverage

| Spec item | Task |
| --- | --- |
| Scaffold crate | Task 1 |
| Flow/block scalars+collections | Task 2 |
| Rc anchors/aliases + Unicode-capable write | Task 3 |
| `dump_all` | Task 4 |
| Docs | Task 5 |
