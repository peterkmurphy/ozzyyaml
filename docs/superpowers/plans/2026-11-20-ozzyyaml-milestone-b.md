# ozzyyaml Milestone B Implementation Plan

> **For agentic workers:** Prefer **inline execution** (executing-plans style) over full SDD. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Ship `ozzyyaml` — compose parser events into `Rc`-shared `Value` trees with YAML 1.2.2 Core schema construction.

**Architecture:** New crate depends on `ozzyyaml-parser`. Composer walks events; Core schema resolves plain scalars; aliases are `Rc::clone`.

**Tech Stack:** Rust 2021, `ozzyyaml-parser`, existing workspace BSD-3-Clause.

## Global Constraints

- `#![forbid(unsafe_code)]`
- Bad input → `Err`, never panic
- No Serde / emit / PyYAML-compat in this milestone
- Keep files focused (~300 lines)
- Reuse parser `Limits` fields; enforce `max_anchors` at compose

---

### Task 1: Scaffold `ozzyyaml` crate

**Files:** Create `crates/ozzyyaml/Cargo.toml`, `src/lib.rs`; add to workspace `Cargo.toml`.

- [x] Depend on `ozzyyaml-parser` (path)
- [x] `forbid(unsafe_code)`, re-export placeholder
- [x] `cargo test -p ozzyyaml` passes
- [x] Commit `chore: scaffold ozzyyaml crate for Milestone B`

---

### Task 2: `Value`, `Error`, `Limits`

**Files:** `value.rs`, `error.rs`, `limits.rs`, `lib.rs`

- [x] Define `Value` as in the B design (`Rc` in seq/map)
- [x] `Error` with `Parse(#[from] ozzyyaml_parser::Error)`, compose variants, `LimitExceeded`
- [x] Wrap/re-export limits with same defaults as parser
- [x] Unit test: `Rc::ptr_eq` on two clones
- [x] Commit `feat(ozzyyaml): add Value, Error, and Limits`

---

### Task 3: Core schema scalar resolution

**Files:** `schema/core.rs` (+ `schema/mod.rs`)

- [x] `resolve_plain(s: &str) -> Value` implementing Core 1.2.2 (no yes/no/on/off; `0o` octal)
- [x] Table-driven unit tests
- [x] Commit `feat(ozzyyaml): Core schema plain scalar resolution`

---

### Task 4: Composer + `from_str` / `load_all`

**Files:** `compose.rs`, `lib.rs`, `tests/compose.rs`

- [x] Walk events → `Value` / `Vec<Value>`
- [x] Anchors map; aliases `Rc::clone`; duplicate/undefined alias errors
- [x] `max_anchors` enforcement
- [x] `from_str` (exactly one document), `load_all`
- [x] Tests: sharing, cycle, unicode anchor, limits, multi-doc
- [x] Commit `feat(ozzyyaml): compose events into Rc-shared Values`

---

### Task 5: Wire CI + README + freeze

- [x] CI already tests workspace — ensure `cargo test` covers new crate (workspace default)
- [x] README Milestone B note
- [x] `cargo fmt`, `clippy -D warnings`, full test
- [x] Commit `docs: Milestone B readiness`
