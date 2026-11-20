# ozzyyaml-parser (Milestone A) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a pure-Rust, `forbid(unsafe_code)` YAML 1.2.2 **event** parser crate (`ozzyyaml-parser`) with Unicode anchors/aliases, typed errors with marks, input/nesting limits, and a yaml-test-suite harness — the foundation for later compose/emit/PyPI work.

**Architecture:** Hand-written pipeline `Reader → Scanner (tokens) → Parser (events)` behind a tiny public API (`parse_str` / `Parser`). Internals stay private. No document tree and no schema in this milestone.

**Tech Stack:** Rust (edition 2021), Cargo workspace, `thiserror`, `cargo test` / `clippy` / `rustfmt`, yaml-test-suite fixtures (git submodule), BSD-3-Clause.

## Global Constraints

- Spec: YAML 1.2.2 only (no 1.1 sexagesimal / PyYAML-compat defaults — those belong to Milestone B+)
- `#![forbid(unsafe_code)]` on every Rust crate
- Bad input → `Err`, never panic
- License: BSD-3-Clause
- Repo root: `/home/peter/projects/ozzyyaml`
- This plan is **Milestone A only**; do not implement compose, emit, or Python here
- Prefer readable code over micro-optimizations; split files before they exceed ~300 lines

---

## File map (Milestone A)

| Path | Responsibility |
| --- | --- |
| `Cargo.toml` | Workspace members |
| `LICENSE` | BSD-3-Clause text |
| `README.md` | Roadmap + motivation (Unicode anchors) |
| `crates/ozzyyaml-parser/Cargo.toml` | Crate manifest |
| `crates/ozzyyaml-parser/src/lib.rs` | Public re-exports + `forbid(unsafe_code)` |
| `crates/ozzyyaml-parser/src/mark.rs` | `Mark { index, line, column }` |
| `crates/ozzyyaml-parser/src/error.rs` | `Error`, `Result`, limit kinds |
| `crates/ozzyyaml-parser/src/limits.rs` | `Limits` defaults + builders |
| `crates/ozzyyaml-parser/src/reader.rs` | UTF-8 input, BOM, peek/advance, marks |
| `crates/ozzyyaml-parser/src/token.rs` | Private `Token` enum |
| `crates/ozzyyaml-parser/src/scanner.rs` | Tokenization |
| `crates/ozzyyaml-parser/src/event.rs` | Public `Event`, styles |
| `crates/ozzyyaml-parser/src/parser.rs` | Tokens → events |
| `crates/ozzyyaml-parser/tests/*.rs` | Integration / acceptance tests |
| `vendor/yaml-test-suite/` | Git submodule of official suite |
| `crates/ozzyyaml-parser/tests/yaml_test_suite.rs` | Suite driver |
| `.github/workflows/ci.yml` | fmt, clippy, test |
| `docs/architecture.md` | Learner-facing pipeline notes |

---

### Task 1: Workspace scaffold + license + crate stub

**Files:**
- Create: `Cargo.toml`
- Create: `LICENSE`
- Create: `README.md`
- Create: `crates/ozzyyaml-parser/Cargo.toml`
- Create: `crates/ozzyyaml-parser/src/lib.rs`
- Create: `.gitignore`

**Interfaces:**
- Consumes: nothing
- Produces: workspace that builds empty `ozzyyaml-parser` lib

- [ ] **Step 1: Create `.gitignore`**

```gitignore
/target
Cargo.lock
.idea/
.vscode/
*.swp
.DS_Store
```

- [ ] **Step 2: Create workspace + crate manifests**

Root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/ozzyyaml-parser"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "BSD-3-Clause"
repository = "https://github.com/peterkmurphy/ozzyyaml"
authors = ["Peter Murphy <peterkmurphy@gmail.com>"]
rust-version = "1.80"
```

`crates/ozzyyaml-parser/Cargo.toml`:

```toml
[package]
name = "ozzyyaml-parser"
description = "YAML 1.2.2 event parser (scan → parse → events)"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
rust-version.workspace = true

[dependencies]
thiserror = "2"
```

- [ ] **Step 3: Stub `lib.rs` and LICENSE/README**

`crates/ozzyyaml-parser/src/lib.rs`:

```rust
#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! YAML 1.2.2 event parser for ozzyyaml (Milestone A).

/// Placeholder so the crate compiles before real API lands.
pub fn crate_name() -> &'static str {
    "ozzyyaml-parser"
}
```

Copy BSD-3-Clause text into `LICENSE` (same style as isityaml: Copyright (c) 2026 Peter Murphy \<peterkmurphy@gmail.com\>).

`README.md` (minimal):

```markdown
# ozzyyaml

From-scratch YAML **1.2.2** stack (Rust-first, Python/PyPI later). BSD-3-Clause.

## Roadmap

1. **ozzyyaml-parser** — events (this milestone)
2. **ozzyyaml** — compose + Core schema
3. **ozzyyaml-emit** — dump
4. **PyPI** — Maturin wheels

Motivation: libyaml/PyYAML reject Unicode anchors/aliases; patches sat unmerged for years.

See `docs/superpowers/specs/2026-11-20-ozzyyaml-design.md`.
```

- [ ] **Step 4: Verify build**

Run: `cargo test -p ozzyyaml-parser`
Expected: PASS (0 or 1 trivial tests OK)

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml LICENSE README.md .gitignore crates/ozzyyaml-parser
git commit -m "chore: scaffold ozzyyaml workspace and parser crate"
```

---

### Task 2: `Mark`, `Limits`, and `Error`

**Files:**
- Create: `crates/ozzyyaml-parser/src/mark.rs`
- Create: `crates/ozzyyaml-parser/src/limits.rs`
- Create: `crates/ozzyyaml-parser/src/error.rs`
- Modify: `crates/ozzyyaml-parser/src/lib.rs`
- Create: `crates/ozzyyaml-parser/src/mark.unit.test` — use `#[cfg(test)]` modules inside each file (Rust convention)

**Interfaces:**
- Consumes: nothing
- Produces:
  - `Mark { index: usize, line: u32, column: u32 }` (1-based line/column)
  - `Limits { max_input_bytes: usize, max_nesting_depth: usize, max_anchors: usize }`
  - `Error` variants: `LimitExceeded { kind, limit, mark }`, `Scan { message, mark }`, `Parse { message, mark }`
  - `pub type Result<T> = std::result::Result<T, Error>`

- [ ] **Step 1: Write failing unit tests in `mark.rs` / `limits.rs` modules**

In `mark.rs` (create with tests first — temporarily stub empty types if needed, or write tests in `lib.rs` tests mod that won't compile):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_display_is_human() {
        let m = Mark { index: 10, line: 2, column: 5 };
        assert_eq!(m.to_string(), "line 2 column 5 (index 10)");
    }
}
```

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits_are_conservative() {
        let l = Limits::default();
        assert!(l.max_anchors <= 10_000);
        assert!(l.max_nesting_depth <= 1_000);
        assert!(l.max_input_bytes <= 64 * 1024 * 1024);
    }
}
```

- [ ] **Step 2: Run tests — expect compile/fail**

Run: `cargo test -p ozzyyaml-parser`
Expected: FAIL (modules/types missing)

- [ ] **Step 3: Implement types**

`mark.rs`:

```rust
use std::fmt;

/// Source position. `line` and `column` are 1-based; `index` is 0-based byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mark {
    /// Byte offset from start of input.
    pub index: usize,
    /// 1-based line number.
    pub line: u32,
    /// 1-based column number (Unicode scalar count on the line is fine for v1).
    pub column: u32,
}

impl fmt::Display for Mark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {} column {} (index {})",
            self.line, self.column, self.index
        )
    }
}
```

`limits.rs`:

```rust
/// Resource limits for untrusted input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Reject inputs longer than this many bytes.
    pub max_input_bytes: usize,
    /// Max collection / indent nesting depth.
    pub max_nesting_depth: usize,
    /// Max distinct anchors registered (primary DoS brake; enforced in Milestone B compose, tracked for API stability here).
    pub max_anchors: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: 16 * 1024 * 1024,
            max_nesting_depth: 1_000,
            max_anchors: 1_024,
        }
    }
}

impl Limits {
    /// Builder-style override.
    pub fn max_input_bytes(mut self, n: usize) -> Self {
        self.max_input_bytes = n;
        self
    }
    pub fn max_nesting_depth(mut self, n: usize) -> Self {
        self.max_nesting_depth = n;
        self
    }
    pub fn max_anchors(mut self, n: usize) -> Self {
        self.max_anchors = n;
        self
    }
}
```

`error.rs`:

```rust
use crate::mark::Mark;
use thiserror::Error;

/// Which limit was exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitKind {
    InputBytes,
    NestingDepth,
    Anchors,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("limit exceeded: {kind:?} (limit {limit}) at {mark}")]
    LimitExceeded {
        kind: LimitKind,
        limit: usize,
        mark: Mark,
    },
    #[error("scan error: {message} at {mark}")]
    Scan { message: String, mark: Mark },
    #[error("parse error: {message} at {mark}")]
    Parse { message: String, mark: Mark },
}

pub type Result<T> = std::result::Result<T, Error>;
```

Re-export from `lib.rs`: `Mark`, `Limits`, `LimitKind`, `Error`, `Result`.

- [ ] **Step 4: Run tests — expect PASS**

Run: `cargo test -p ozzyyaml-parser`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml-parser/src
git commit -m "feat(parser): add Mark, Limits, and Error types"
```

---

### Task 3: `Reader` (UTF-8, BOM, marks)

**Files:**
- Create: `crates/ozzyyaml-parser/src/reader.rs`
- Modify: `crates/ozzyyaml-parser/src/lib.rs`

**Interfaces:**
- Consumes: `Limits`, `Error`, `Mark`
- Produces: `struct Reader<'a>` with:
  - `fn new(input: &'a str, limits: Limits) -> Result<Self>`
  - `fn mark(&self) -> Mark`
  - `fn peek(&self) -> Option<char>`
  - `fn peek2(&self) -> Option<(char, char)>`
  - `fn bump(&mut self) -> Option<char>`
  - `fn is_eof(&self) -> bool`
  - `fn rest(&self) -> &'a str`

- [ ] **Step 1: Write failing reader tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::Limits;

    #[test]
    fn strips_utf8_bom() {
        let mut r = Reader::new("\u{FEFF}a", Limits::default()).unwrap();
        assert_eq!(r.bump(), Some('a'));
    }

    #[test]
    fn tracks_line_and_column() {
        let mut r = Reader::new("a\nb", Limits::default()).unwrap();
        assert_eq!(r.mark(), Mark { index: 0, line: 1, column: 1 });
        r.bump();
        r.bump(); // newline
        assert_eq!(r.mark().line, 2);
        assert_eq!(r.mark().column, 1);
    }

    #[test]
    fn rejects_oversize_input() {
        let limits = Limits::default().max_input_bytes(2);
        let err = Reader::new("abcd", limits).unwrap_err();
        assert!(matches!(
            err,
            crate::error::Error::LimitExceeded {
                kind: crate::error::LimitKind::InputBytes,
                ..
            }
        ));
    }
}
```

- [ ] **Step 2: Run tests — expect FAIL**

Run: `cargo test -p ozzyyaml-parser reader::`
Expected: FAIL

- [ ] **Step 3: Implement `Reader`**

Implement peek/bump over `input.char_indices()`, skip leading BOM, enforce `max_input_bytes` on `input.len()`, update `line`/`column` on `\n` (treat `\r\n` as one newline advancing past both). Keep the implementation small and heavily commented.

- [ ] **Step 4: Run tests — expect PASS**

Run: `cargo test -p ozzyyaml-parser`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml-parser/src/reader.rs crates/ozzyyaml-parser/src/lib.rs
git commit -m "feat(parser): add UTF-8 Reader with BOM and marks"
```

---

### Task 4: `Token` and `Event` public shapes

**Files:**
- Create: `crates/ozzyyaml-parser/src/token.rs` (private to crate — `pub(crate)`)
- Create: `crates/ozzyyaml-parser/src/event.rs` (public)
- Modify: `crates/ozzyyaml-parser/src/lib.rs`

**Interfaces:**
- Consumes: nothing beyond `String` / `Mark` as needed
- Produces public:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarStyle {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    StreamStart,
    StreamEnd,
    DocumentStart { explicit: bool },
    DocumentEnd { explicit: bool },
    Alias { anchor: String },
    Scalar {
        anchor: Option<String>,
        tag: Option<String>,
        value: String,
        style: ScalarStyle,
    },
    SequenceStart {
        anchor: Option<String>,
        tag: Option<String>,
        flow: bool,
    },
    SequenceEnd,
    MappingStart {
        anchor: Option<String>,
        tag: Option<String>,
        flow: bool,
    },
    MappingEnd,
}
```

Private tokens should cover: stream/doc markers, flow/block entries, anchors, aliases, tags, scalars, indent/dedent or indent-aware tokens as the scanner design needs. Prefer an enum `Token` with `mark: Mark` on each variant or a `Token { kind, mark }` pair — pick one style and stick to it for the rest of Milestone A.

- [ ] **Step 1: Write a failing test that constructs events**

```rust
#[test]
fn scalar_event_holds_unicode_anchor_name() {
    let ev = Event::Scalar {
        anchor: Some("汉".into()),
        tag: None,
        value: "hi".into(),
        style: ScalarStyle::Plain,
    };
    match ev {
        Event::Scalar { anchor: Some(a), .. } => assert_eq!(a, "汉"),
        _ => panic!("wrong variant"),
    }
}
```

- [ ] **Step 2–4: Implement `event.rs` + minimal `token.rs`, export `Event`/`ScalarStyle`, test PASS, commit**

```bash
git commit -m "feat(parser): define Event and Token types"
```

---

### Task 5: Scanner — stream + plain scalar documents

**Files:**
- Create: `crates/ozzyyaml-parser/src/scanner.rs`
- Modify: `lib.rs`

**Interfaces:**
- Consumes: `Reader`, `Token`, `Limits`, `Error`
- Produces: `struct Scanner<'a>` with `fn next_token(&mut self) -> Result<Option<Token>>` (or iterator)

**Scope for this task only:** enough to tokenize:

```yaml
hello
```

and

```yaml
"hello"
```

and

```yaml
'hello'
```

into scalar tokens (plus stream start/end if you emit those at scanner level — otherwise leave stream markers to the parser). Do **not** implement block collections yet.

- [ ] **Step 1: Failing integration-style unit tests on scanner output**

```rust
#[test]
fn scans_plain_scalar() {
    let mut s = Scanner::new(Reader::new("hello", Limits::default()).unwrap());
    let t = s.next_token().unwrap().expect("token");
    assert!(matches!(t, Token::Scalar { ref value, .. } if value == "hello"));
}
```

(Adjust match to your `Token` shape.)

- [ ] **Step 2: Run — FAIL**
- [ ] **Step 3: Implement minimal scanner** (skip spaces/comments `#...`, recognize plain/single/double scalars)
- [ ] **Step 4: PASS**
- [ ] **Step 5: Commit** `feat(parser): scan plain and quoted scalars`

---

### Task 6: Parser — events for single-scalar documents

**Files:**
- Create: `crates/ozzyyaml-parser/src/parser.rs`
- Modify: `lib.rs`

**Interfaces:**
- Consumes: `Scanner` / tokens, `Event`, `Limits`
- Produces:

```rust
pub struct Parser<'a> { /* ... */ }

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Result<Self> { /* Limits::default() */ }
    pub fn with_limits(input: &'a str, limits: Limits) -> Result<Self> { /* ... */ }
    pub fn next_event(&mut self) -> Result<Option<Event>> { /* ... */ }
}

pub fn parse_str(input: &str) -> Result<Vec<Event>> { /* drain Parser */ }
```

Expected events for `"hello"` / `hello`:

`[StreamStart, DocumentStart { explicit: false }, Scalar { ... }, DocumentEnd { explicit: false }, StreamEnd]`

- [ ] **Step 1: Failing test**

```rust
#[test]
fn parse_plain_scalar_document() {
    let evs = parse_str("hello").unwrap();
    assert_eq!(evs.first(), Some(&Event::StreamStart));
    assert!(evs.iter().any(|e| matches!(e, Event::Scalar { value, .. } if value == "hello")));
    assert_eq!(evs.last(), Some(&Event::StreamEnd));
}
```

- [ ] **Step 2: FAIL → Step 3: implement parser glue → Step 4: PASS → Step 5: Commit**

```bash
git commit -m "feat(parser): emit events for single-scalar documents"
```

---

### Task 7: Unicode anchors and aliases (acceptance)

**Files:**
- Modify: `scanner.rs`, `parser.rs` as needed
- Create: `crates/ozzyyaml-parser/tests/unicode_anchors.rs`

**Interfaces:**
- Anchor/alias names: YAML 1.2.2 `ns-anchor-name` (non-space printable), **including non-ASCII**

- [ ] **Step 1: Failing acceptance tests**

```rust
use ozzyyaml_parser::{parse_str, Event};

#[test]
fn unicode_anchor_and_alias_round_trip_events() {
    let yaml = "foo: &汉 bar\nbaz: *汉\n";
    let evs = parse_str(yaml).expect("parse");
    let has_anchor = evs.iter().any(|e| matches!(e, Event::Scalar { anchor: Some(a), .. } if a == "汉"));
    let has_alias = evs.iter().any(|e| matches!(e, Event::Alias { anchor } if anchor == "汉"));
    assert!(has_anchor, "expected unicode anchor, got {evs:?}");
    assert!(has_alias, "expected unicode alias, got {evs:?}");
}

#[test]
fn emoji_anchor_name() {
    let yaml = "- &😀 value\n- *😀\n";
    parse_str(yaml).expect("emoji anchors must parse under YAML 1.2.2");
}
```

- [ ] **Step 2: FAIL (likely “expected alphabetic” style bug if copying libyaml habits)**
- [ ] **Step 3: Implement anchor/alias scanning using 1.2.2 character classes (not `[A-Za-z0-9-] only`)**
- [ ] **Step 4: Also support flow/block mapping/sequence **enough** for the tests above** (minimal collections)
- [ ] **Step 5: PASS + commit** `feat(parser): support Unicode anchors and aliases`

---

### Task 8: Block and flow collections (core shapes)

**Files:**
- Modify: `scanner.rs`, `parser.rs`
- Create: `crates/ozzyyaml-parser/tests/collections.rs`

**Interfaces:**
- Nesting depth checked against `Limits.max_nesting_depth` when entering collections; error `LimitExceeded { kind: NestingDepth, .. }`

- [ ] **Step 1: Failing tests** for:

```yaml
- a
- b
```

```yaml
{a: 1, b: 2}
```

```yaml
a:
  b: c
```

Assert correct `SequenceStart`/`MappingStart`/`End` nesting and scalar values.

- [ ] **Step 2: FAIL → implement indent stack + flow indicators → PASS**
- [ ] **Step 3: Failing test for nesting limit**

```rust
#[test]
fn nesting_limit_errors() {
    let limits = ozzyyaml_parser::Limits::default().max_nesting_depth(2);
    let yaml = "[[[\n]]]";
    let err = ozzyyaml_parser::Parser::with_limits(yaml, limits)
        .unwrap()
        .next_event();
    // drain until error
    // ...
}
```

Implement a small helper `fn parse_all(input, limits) -> Result<Vec<Event>>` in tests.

- [ ] **Step 4: PASS + commit** `feat(parser): parse flow and block collections with nesting limits`

---

### Task 9: Tags, explicit documents, multi-doc streams

**Files:**
- Modify: scanner/parser
- Create: `crates/ozzyyaml-parser/tests/documents.rs`

- [ ] **Step 1: Tests for**

```yaml
---
!!str foo
...
---
bar
```

Expect two documents, explicit start/end flags, tag on first scalar.

- [ ] **Step 2: Implement `%YAML` / `%TAG` directive handling enough to not choke; store resolved tag strings on events**
- [ ] **Step 3: PASS + commit** `feat(parser): multi-document streams and tags`

---

### Task 10: yaml-test-suite harness

**Files:**
- Create submodule: `vendor/yaml-test-suite`
- Create: `crates/ozzyyaml-parser/tests/yaml_test_suite.rs`
- Create: `crates/ozzyyaml-parser/tests/suite_allowlist.txt` (start small)

**Interfaces:**
- Driver reads test directories with `in.yaml` + `test.event` (or equivalent suite format for the pinned suite revision)
- Compare emitted events to expected event stream
- Allowlist file lists case IDs that must pass; others ignored until green

- [ ] **Step 1: Add submodule**

```bash
git submodule add https://github.com/yaml/yaml-test-suite.git vendor/yaml-test-suite
cd vendor/yaml-test-suite && git checkout main  # pin a commit SHA in the parent repo
```

- [ ] **Step 2: Write harness that loads allowlisted IDs and asserts event equality**
- [ ] **Step 3: Seed allowlist with 5–10 simplest cases already passing (scalar/doc)**
- [ ] **Step 4: Commit** `test(parser): add yaml-test-suite harness with allowlist`

**Follow-on (same milestone, repeated commits):** grow allowlist as features land; never expand allowlist for failing cases.

---

### Task 11: CI + architecture doc

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `docs/architecture.md`
- Modify: `README.md` (badges optional)

- [ ] **Step 1: CI workflow**

```yaml
name: ci
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          submodules: recursive
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - run: cargo fmt --all -- --check
      - run: cargo clippy -p ozzyyaml-parser -- -D warnings
      - run: cargo test -p ozzyyaml-parser
```

- [ ] **Step 2: `docs/architecture.md`** — describe Reader→Scanner→Parser, shared-vs-copy note for Milestone B, `max_anchors` rationale, Unicode anchor motivation with link to libyaml#132
- [ ] **Step 3: Commit** `chore: add CI and architecture notes`

---

### Task 12: Milestone A freeze checklist

- [ ] **Step 1: Run full local gate**

```bash
cargo fmt --all -- --check
cargo clippy -p ozzyyaml-parser -- -D warnings
cargo test -p ozzyyaml-parser
```

Expected: all PASS

- [ ] **Step 2: Confirm public API surface is only what `lib.rs` re-exports** (`Event`, `ScalarStyle`, `Mark`, `Limits`, `LimitKind`, `Error`, `Result`, `Parser`, `parse_str`) — no `Token`/`Reader` leakage
- [ ] **Step 3: Tag mindset** (optional): note in README “Milestone A usable for event consumers; compose not yet available”
- [ ] **Step 4: Commit any doc-only fixes** `docs: Milestone A readiness notes`

---

## Out of scope for this plan (separate later plans)

- `ozzyyaml` compose + Core schema + shared graph + `max_anchors` enforcement at compose time
- `ozzyyaml-emit`
- PyO3 / Maturin / PyPI
- PyYAML-compat schema mode

---

## Spec coverage self-check

| Spec item | Task(s) |
| --- | --- |
| YAML 1.2.2 events | 5–10 |
| Hand-written scanner/parser | 5–6 |
| Unicode anchors/aliases | 7 |
| `forbid(unsafe_code)` | 1+ |
| Typed errors + marks | 2–3 |
| `max_input_bytes` / nesting | 3, 8 |
| `max_anchors` field on Limits | 2 (enforced in B) |
| yaml-test-suite | 10 |
| Tiny public API | 4, 6, 12 |
| BSD-3-Clause | 1 |
| Architecture learning doc | 11 |
| No compose/emit/Python | explicit out of scope |

## Placeholder / consistency self-check

- Public method names locked: `parse_str`, `Parser::new`, `Parser::with_limits`, `Parser::next_event`
- `Limits::default` values fixed in Task 2
- Event variant names fixed in Task 4; later tasks must use those names
