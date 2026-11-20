# ozzyyaml B Polish + Allowlist Growth — Design Spec

**Date:** 2026-11-20  
**Status:** Approved  
**Parent:** [2026-11-20-ozzyyaml-milestone-b-design.md](./2026-11-20-ozzyyaml-milestone-b-design.md)

## Intent

Small Milestone B follow-ups, then grow yaml-test-suite coverage for the
parser and add a lean compose-level allowlist.

## Scope

### 1. `Value` accessors

Add Option-returning helpers on `Value`:

| Method | Returns |
| --- | --- |
| `as_null` | `Option<()>` |
| `as_bool` | `Option<bool>` |
| `as_i64` | `Option<i64>` |
| `as_f64` | `Option<f64>` |
| `as_str` | `Option<&str>` |

Existing `as_sequence` / `as_mapping` stay. No panicking getters in this pass.

### 2. Marks on compose errors

**Choice:** Do **not** add `Mark` fields to `Event`.

Instead, `ozzyyaml_parser::Parser` records the source `Mark` of the event
most recently produced by `next_event()` and exposes:

```rust
pub fn last_mark(&self) -> Mark
```

Before any event, this is `Mark::default()`. After an `Err` from `next_event`,
behavior may leave the previous mark (composer already maps parse errors
which carry their own marks).

`ozzyyaml` composer uses `parser.last_mark()` when building:

- `Error::UndefinedAlias`
- `Error::DuplicateAnchor`
- `Error::LimitExceeded` (compose-time `max_anchors`)
- other `Error::Compose` variants where a position is meaningful

### 3. Byte APIs

On `ozzyyaml`:

- `from_slice` / `from_slice_with_limits`
- `load_all_from_slice` / `load_all_from_slice_with_limits`

Flow: UTF-8 validate → existing `&str` compose path.

Invalid UTF-8 → new variant:

```rust
Error::InvalidUtf8 { valid_up_to: usize }
```

No UTF-16/32 detection in this pass. Parser still consumes `&str` only.

### 4. Docs refresh

Update `docs/architecture.md` (and README if needed) so Milestone B is
described as shipped: compose pipeline, `Rc` + `RefCell` collections, Core
schema, `max_anchors` at compose, `from_str` / `load_all` / `from_slice`.

### 5. Allowlist growth

**Parser:** Extend `crates/ozzyyaml-parser/tests/suite_allowlist.txt` only
with case IDs that already pass under the existing harness. Never allowlist
a failing case.

**Compose:** Add a small allowlist-driven test under `crates/ozzyyaml` that
parses suite `in.yaml` via `from_str` / `load_all` and checks a minimal
success/shape criterion (or compares against a documented expectation). Same
rule: only IDs that pass. Keep the first batch small and honest.

## Non-goals

- Marks on `Event` or `Value`
- Serde, emit, PyYAML-compat, Python
- Full YAML encoding sniffing for `from_slice`
- Expanding allowlists with known failures

## Testing

- Unit tests for accessors and `InvalidUtf8`
- Compose error tests assert non-default marks where input has content past
  the start of the stream
- Parser allowlist: `cargo test -p ozzyyaml-parser yaml_test_suite`
- Compose allowlist: `cargo test -p ozzyyaml` (new harness)

## File map (expected)

```
crates/ozzyyaml-parser/src/parser/mod.rs   # last_mark
crates/ozzyyaml/src/value.rs               # accessors
crates/ozzyyaml/src/error.rs               # InvalidUtf8
crates/ozzyyaml/src/compose.rs             # last_mark + from_slice*
crates/ozzyyaml/src/lib.rs                 # re-exports
crates/ozzyyaml/tests/...                  # compose suite allowlist (new)
crates/ozzyyaml-parser/tests/suite_allowlist.txt
docs/architecture.md
README.md                                  # if compose API needs a mention
```
