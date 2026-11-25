# ozzyyaml Python + compose suite 255/255 — Design Spec

**Date:** 2026-11-25  
**Status:** Approved for implementation planning  
**Parent:** [2026-11-25-ozzyyaml-grammar-complete-design.md](./2026-11-25-ozzyyaml-grammar-complete-design.md)  
**Related:** [2026-11-20-ozzyyaml-python-design.md](./2026-11-20-ozzyyaml-python-design.md),
[2026-11-24-ozzyyaml-conformance-design.md](./2026-11-24-ozzyyaml-conformance-design.md)

## Intent

Bring **compose** to the same valid-suite bar as the event parser, and prove it
through both Rust and the Python package.

Success metric: **100%** of valid yaml-test-suite cases that have `test.event`
(255 / 255 at design baseline) compose successfully via:

1. Rust `ozzyyaml::load_all` / `load_all_with_limits`
2. Python `ozzyyaml.load_all_value` **and** `ozzyyaml.load_all`

This is a **compose-success** claim, not event-line equality from Python and
not tree equality against `in.json`.

## Goals

1. Allow YAML 1.2.2 **anchor redefinition** in compose (the only known blockers
   today are `3GZX` and `PW8X`, both `DuplicateAnchor`).
2. Hard-gate Rust compose on every valid suite ID (retire `suite_allowlist.txt`).
3. Hard-gate Python on the same ID set for both Value and native loaders.
4. Teach native Python `load` / `load_all` to represent **cycles** via placeholder
   insertion (lists/dicts can be cyclic; the binding currently refuses them).
5. Make native `load` / `load_all` accept collection mapping keys by converting
   them to hashable tuples (see below) so the suite gate can cover all valid IDs.
6. Update `docs/conformance.md` and `docs/python.md` with honest 255/255 compose
   claims and corrected cycle / key wording.

## Non-goals

- Exposing a Python event / `test.event` API
- Comparing composed trees to `in.json`
- Expanding the compose **shape** allowlist (`suite_shape_allowlist.txt`)
- Emit presentation fidelity or round-trip allowlist expansion
- Changing Core schema / strict known-tag policy
- Re-litigating event-parser grammar (already 255/255 `test.event`)

## Current gaps

| Area | State at design time |
| --- | --- |
| Event suite | 255 / 255 hard gate |
| Rust compose allowlist | 238 IDs; discovery reports **253** compose OK |
| Compose failures among valid | **`3GZX`**, **`PW8X`** — both `Error::DuplicateAnchor` |
| Python suite gate | None (unit tests only) |
| Native Python cycles | Explicit error when converting cyclic `Value` graphs |

## Compose semantics — anchor redefinition

YAML 1.2.2 allows an anchor name to be **redefined** later in the same
document. Earlier aliases that already resolved keep their prior node; later
aliases bind to the new node (`3GZX` / `in.json`).

**Change:** `Composer::register_anchor` overwrites an existing binding instead
of returning `DuplicateAnchor`.

**Limits:** Overwrite does not increase `HashMap` length, so `max_anchors` still
counts distinct names currently registered. First registration of a new name
still enforces the limit. Anchors continue to clear at document boundaries.

**`Error::DuplicateAnchor`:** Remove the variant and update tests that asserted
it. Undefined aliases remain errors.

**Unit coverage:** Redefinition then alias sees the new value; suite-facing
`3GZX` and `PW8X` compose `Ok`.

## Native Python cycles

Today `value_to_py` caches a node only **after** it is fully built, and errors
if a node is re-entered while still on the construction stack. That rejects
cycles even though Python containers can be cyclic.

**Change:** For `Sequence` and `Mapping`, allocate an empty `list` / `dict`,
**insert into the identity cache immediately**, then fill children. Cycles and
acyclic shared aliases both yield the same Python object (`a is b`). Scalars
unchanged.

Add a focused Python test that a cyclic document loads natively with
self-reference identity. After this fix, native `load_all` can participate in
the full-suite compose gate alongside `load_all_value`.

## Native Python mapping keys

YAML allows collection nodes as mapping keys. Python `dict` requires hashable
keys, so a bare `list` / `dict` key would raise `TypeError` and break a native
`load_all` suite gate.

**Change:** When inserting a mapping entry into a native `dict`, convert keys
that are not already hashable:

- sequence key → `tuple` of converted items (recursive)
- mapping key → `tuple` of `(converted_key, converted_value)` pairs in order
- scalars unchanged (`None` / `bool` / `int` / `float` / `str`)

Values are never tuple-coerced by this rule (only keys). Document the
conversion in `docs/python.md`. `load_value` keeps the full graph without
tuple coercion.

This is a native-API convenience mapping, not a claim that tuples are YAML
key nodes.

## Harnesses

### Rust (`crates/ozzyyaml/tests/yaml_test_suite_compose.rs`)

- Enumerate valid IDs the same way as the event gate: top-level dirs with
  `in.yaml` + `test.event`, no `error` marker.
- Assert `load_all(in.yaml)` is `Ok` for every ID; collect failures.
- Delete `suite_allowlist.txt` (retire allowlist).
- Leave `yaml_test_suite_compose_shape.rs` / shape allowlist unchanged.

### Python (`python/tests/test_yaml_test_suite_compose.py`)

- Resolve suite root at `../../vendor/yaml-test-suite` relative to the test file
  (repo layout: `python/tests/` → vendor).
- Same valid-ID filter as Rust.
- For each ID: `load_all_value(text)` and `load_all(text)` must not raise.
- Collect-all failure reporting (or equivalent clear multi-ID output).
- If the submodule is missing, fail with a clear init message (match Rust tone).

No new public Python APIs. No event rendering in Python.

## Documentation

- `docs/conformance.md`: compose **100%** of valid cases (255/255) via Rust
  `load_all` and Python `load` / `load_value` family; keep emit presentation and
  Core-only caveats; note shape/round-trip allowlists remain separate.
- `docs/python.md`: state that native load preserves alias identity including
  cycles; document tuple coercion for collection mapping keys; point at the
  compose suite claim.
- Do not claim Python `test.event` equality.

## Approach chosen

**Redefinition policy + dual gates + native cycle placeholders** (brainstorm
option 2, with cycle support added after clarifying that native refusal was an
implementation choice).

Rejected: compose-success only without removing `DuplicateAnchor` cleanly;
`in.json` tree oracles; Python-only gate while Rust keep an allowlist.

## Success criteria

1. `cargo test -p ozzyyaml --test yaml_test_suite_compose` hard-gates all valid IDs green.
2. `pytest` Python compose suite green for `load_all` and `load_all_value` on all valid IDs.
3. `3GZX` / `PW8X` covered by focused tests; no `DuplicateAnchor` in the public error enum.
4. Native cyclic load unit test green; collection-key → tuple unit test green.
5. Conformance and Python docs updated; no false “cannot represent cycles” wording.

## Implementation order (planning hint)

1. Anchor redefinition in Rust compose + unit/suite probes + retire allowlist gate.
2. Native Python cycle placeholders + collection-key tuple coercion + unit tests.
3. Python full-suite compose pytest (`load_all` + `load_all_value`).
4. Docs.
