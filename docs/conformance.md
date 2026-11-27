# Conformance status

ozzyyaml implements **YAML 1.2.2** for parse, Core-schema compose, and data
round-trip emit, with the caveats below.

## What is covered

- Event parser language: multi-line plain/quoted folding (§8.1.3), block
  scalars `|` / `>` (§8.1.4–§8.1.6), full double-quoted escapes (§7.3.2),
  explicit keys (`?`), `%YAML` / `%TAG` directives, Unicode anchors
- Core compose with **strict** known tags (`!!int twelve` is an error),
  including YAML 1.2.2 **anchor redefinition**
- Emit → load data round-trip for acyclic documents in the round-trip
  allowlist
- Chapter 5 character classes: `c-printable` / `nb-char` / `nb-json`, reserved
  `@`/`\`` plain starts, and `ns-uri-char` / `ns-tag-char` for tags
- Byte APIs (`from_slice` / `load_all_from_slice`): UTF-8, UTF-16LE/BE, and
  UTF-32LE/BE via BOM **and** §5.2 null-pattern detection. String APIs and
  emit remain UTF-8. Mid-document BOM is an error; BOM inside double-quoted
  scalars is allowed
- Default `Limits`: 256 MiB input, **10 000** nesting, 100 000 anchors (overridable).
  Nesting is capped so recursive compose/emit/`Drop` (and the Python
  converters) cannot stack-overflow on an 8 MiB thread; raise via `Limits`
  only for trusted deep trees until walks are iterative (planned 0.3.x).

## Honest caveats

- **yaml-test-suite events:** **100%** of valid cases with `test.event`
  (255 / 255) match exactly, enforced by the hard gate
  `all_valid_event_suite_cases_match` in
  `crates/ozzyyaml-parser/tests/yaml_test_suite.rs`.
- **yaml-test-suite invalid:** **100%** of cases with an `error` marker
  (78 / 78) are rejected by `parse_str`, enforced by
  `all_error_suite_cases_reject` in
  `crates/ozzyyaml-parser/tests/yaml_test_suite_errors.rs`.
- **yaml-test-suite compose:** **100%** of the same valid cases (255 / 255)
  compose successfully via Rust `load_all` and Python `load_all` /
  `load_all_value`, enforced by `all_valid_cases_compose_ok` in
  `crates/ozzyyaml/tests/yaml_test_suite_compose.rs` and
  `python/tests/test_yaml_test_suite_compose.py`. This is compose-success
  only (not `in.json` tree equality, not Python `test.event` rendering).
- **Shape / round-trip allowlists** remain separate
  (`suite_shape_allowlist.txt`, `suite_roundtrip_allowlist.txt`).
- **Emit** does not preserve comments, tags as written, or source layout —
  only `Value` data equality after dump→load.
- **Core only:** `yes` / `no` stay strings; no YAML 1.1 merge keys.
- **Fuzzing:** `cargo-fuzz` targets for `parse_str` and compose/emit
  round-trip run bounded in CI and longer on a nightly schedule. See
  [fuzz-findings.md](superpowers/fuzz-findings.md).

Design tracks:
[0.2.0 pre-release](superpowers/specs/2026-11-26-ozzyyaml-0.2.0-prerelease-design.md),
[grammar complete](superpowers/specs/2026-11-25-ozzyyaml-grammar-complete-design.md),
[Python + compose suite](superpowers/specs/2026-11-25-ozzyyaml-python-compose-suite-design.md),
[conformance](superpowers/specs/2026-11-24-ozzyyaml-conformance-design.md),
[Chapter 5 chars](superpowers/specs/2026-11-25-ozzyyaml-chapter5-chars-design.md).  
Plans:
[0.2.0 pre-release](superpowers/plans/2026-11-26-ozzyyaml-0.2.0-prerelease.md),
[grammar complete](superpowers/plans/2026-11-25-ozzyyaml-grammar-complete.md),
[Python + compose suite](superpowers/plans/2026-11-25-ozzyyaml-python-compose-suite.md),
[conformance](superpowers/plans/2026-11-24-ozzyyaml-conformance.md),
[Chapter 5](superpowers/plans/2026-11-25-ozzyyaml-chapter5-chars.md).
