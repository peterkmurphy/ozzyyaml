# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] — 2026-11-27

### Fixed

- Stack-overflow paths in emit (`Writer` / `Anchors::scan`), `Value::Drop`, and
  the Python converters (`value_to_py` / `py_to_value` / hashable keys) by
  rewriting those walks as iterative. Deep trees no longer die with SIGSEGV
  under default stack size once `Limits` accepts them.

### Changed

- **Breaking (behavioral):** default `max_nesting_depth` raised from **512**
  back to **10 000**. Nesting is only a DoS/memory guard again. Callers who
  relied on the 0.2.1 cap rejecting depth ≥ 513 will see those documents
  accepted under default `Limits`.

## [0.2.1] — 2026-11-27

### Fixed

- Stack-overflow crash (SIGSEGV) when handling deeply nested documents. The
  first `fuzz-nightly` run found inputs (~7 000-deep flow sequences) that
  `Limits` still accepted, then died in recursive compose / emit / `Drop`
  (and the matching Python converters) with no `Err` and no unwind.

### Changed

- **Breaking (behavioral):** default `max_nesting_depth` lowered from
  **10 000** to **512**, so the default is survivable on a typical 8 MiB
  thread stack for all recursive tree walks.
- **Migration:** trusted callers who need deeper trees must opt in:
  `Limits::default().max_nesting_depth(n)` (Rust) or the
  `max_nesting_depth=` keyword on Python `load` / `load_value` (and related
  APIs). The default will be raised again after iterative tree walks land in
  **0.3.x**; `Limits` overrides are the intended path until then.
- Fuzz roundtrip oracle now panics if emit output fails to reparse; nightly
  fuzz failures open a triage issue automatically.

## [0.2.0] — 2026-11-26

### Added

- Full yaml-test-suite gate: **255/255** valid `test.event` cases match
  exactly and **78/78** invalid cases are rejected, both enforced in CI.
- Grammar coverage for multi-line folding, block scalars, explicit keys,
  double-quoted escapes, `%YAML` / `%TAG` directives, and UTF-16/32
  detection on byte input (BOM and §5.2 null-pattern).
- Strict Core-schema compose (known tags reject non-canonical forms;
  `yes` / `no` stay strings).
- PyO3 **0.30** bindings; per-interpreter wheels for CPython **3.10–3.14**
  on Linux (x86_64 + aarch64), macOS (arm64 + x86_64 via cross-build), and
  Windows x64; free-threaded **3.14t** import smoke in CI.
- `cargo-fuzz` targets for `parse_str` and compose/emit roundtrip, with
  bounded CI and scheduled nightly runs.

## [0.1.0] — 2026-11-20

- Initial release: event parser, Core compose, emit, and Python bindings.

[0.3.0]: https://github.com/peterkmurphy/ozzyyaml/releases/tag/v0.3.0
[0.2.1]: https://github.com/peterkmurphy/ozzyyaml/releases/tag/v0.2.1
[0.2.0]: https://github.com/peterkmurphy/ozzyyaml/releases/tag/v0.2.0
[0.1.0]: https://github.com/peterkmurphy/ozzyyaml/releases/tag/v0.1.0
