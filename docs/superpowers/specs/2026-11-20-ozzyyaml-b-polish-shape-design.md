# ozzyyaml B Polish (tests + shape allowlist) — Design Spec

**Date:** 2026-11-20  
**Status:** Approved  
**Parent:** [2026-11-20-ozzyyaml-b-polish-allowlist-design.md](./2026-11-20-ozzyyaml-b-polish-allowlist-design.md)

## Intent

Close deferred polish from the B polish review: stronger tests for byte APIs,
compose error marks, accessors, and a small compose **shape** allowlist beside
the existing `Ok`-only suite harness.

## Scope

### 1. Byte API tests

In `crates/ozzyyaml/tests/compose.rs` (or adjacent):

- `from_slice_with_limits` happy path
- `load_all_from_slice` / `load_all_from_slice_with_limits` happy path (multi-doc)
- Invalid UTF-8 on `load_all_from_slice*` → `Error::InvalidUtf8 { valid_up_to }`

### 2. Mark assertions

- `DuplicateAnchor` — assert mark line/column (or index) for a known fixture
- Compose `LimitExceeded` (`max_anchors`) — assert mark is not `Mark::default()`
  for a fixture that registers an anchor past the limit

Use single-document fixtures that the parser accepts (block sequences where
needed — avoid second-root shapes).

### 3. Accessor negatives

Extend `scalar_accessors` (or sibling test) so each of `as_null` / `as_bool` /
`as_i64` / `as_f64` / `as_str` has at least one mismatch → `None`. No
Int↔Float coercion.

### 4. Compose shape allowlist

Keep `crates/ozzyyaml/tests/suite_allowlist.txt` + `allowlisted_cases_compose_ok`
unchanged in purpose (`load_all` → `Ok` only).

Add:

- `crates/ozzyyaml/tests/suite_shape_allowlist.txt` (~10–20 IDs)
- `crates/ozzyyaml/tests/yaml_test_suite_compose_shape.rs`

For each ID: `load_all(in.yaml)` succeeds, then assert **root kind** (null /
bool / int / float / string / sequence / mapping) and **length** for
seq/map where applicable. A few nested checks allowed when cheap. Never
allowlist a failing or shape-mismatched case.

Hand-pick IDs from cases that already pass the Ok allowlist and have an
obvious root shape.

## Non-goals

- Emit, Serde, PyYAML-compat, Python
- Value-tree golden files for the full 143-ID list
- Changing public APIs except incidental test helpers if any

## Testing

- `cargo test -p ozzyyaml`
- `cargo clippy --workspace -- -D warnings`
