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

- [ ] Depend on `ozzyyaml-parser` (path)
- [ ] `forbid(unsafe_code)`, re-export placeholder
- [ ] `cargo test -p ozzyyaml` passes
- [ ] Commit `chore: scaffold ozzyyaml crate for Milestone B`

---

### Task 2: `Value`, `Error`, `Limits`

**Files:** `value.rs`, `error.rs`, `limits.rs`, `lib.rs`

- [ ] Define `Value` as in the B design (`Rc` in seq/map)
- [ ] `Error` with `Parse(#[from] ozzyyaml_parser::Error)`, compose variants, `LimitExceeded`
- [ ] Wrap/re-export limits with same defaults as parser
- [ ] Unit test: `Rc::ptr_eq` on two clones
- [ ] Commit `feat(ozzyyaml): add Value, Error, and Limits`

---

### Task 3: Core schema scalar resolution

**Files:** `schema/core.rs` (+ `schema/mod.rs`)

- [ ] `resolve_plain(s: &str) -> Value` implementing Core 1.2.2 (no yes/no/on/off; `0o` octal)
- [ ] Table-driven unit tests
- [ ] Commit `feat(ozzyyaml): Core schema plain scalar resolution`

---

### Task 4: Composer + `from_str` / `load_all`

**Files:** `compose.rs`, `lib.rs`, `tests/compose.rs`

- [ ] Walk events → `Value` / `Vec<Value>`
- [ ] Anchors map; aliases `Rc::clone`; duplicate/undefined alias errors
- [ ] `max_anchors` enforcement
- [ ] `from_str` (exactly one document), `load_all`
- [ ] Tests: sharing, cycle, unicode anchor, limits, multi-doc
- [ ] Commit `feat(ozzyyaml): compose events into Rc-shared Values`

---

### Task 5: Wire CI + README + freeze

- [ ] CI already tests workspace — ensure `cargo test` covers new crate (workspace default)
- [ ] README Milestone B note
- [ ] `cargo fmt`, `clippy -D warnings`, full test
- [ ] Commit `docs: Milestone B readiness`
