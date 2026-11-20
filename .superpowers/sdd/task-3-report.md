# Task 3 Report: Reader

## Status

Implemented and exported `Reader<'a>` with UTF-8 character traversal, leading BOM
skipping, byte-limit enforcement, one-based marks, newline tracking, CRLF
normalization, lookahead, EOF detection, and access to unconsumed input.

## TDD evidence

- RED: `cargo test -p ozzyyaml-parser reader::` failed because `Reader` did not
  exist (E0433), as expected after adding the brief's tests.
- GREEN: `cargo test -p ozzyyaml-parser` passed all 9 unit tests and doc tests.

The environment did not expose Cargo on `PATH`; commands were run with the
installed stable toolchain directory prepended.

## Verification and self-review

- `cargo fmt --all -- --check`: passed before final line-ending preservation.
- `cargo clippy -p ozzyyaml-parser --all-targets -- -D warnings`: passed.
- Reviewed the final diff for scope: only `reader.rs` and the two required
  module/export lines in `lib.rs` are source changes.
- Confirmed limit errors use an explicit start mark of line 1, column 1 rather
  than `Mark::default()`.
- No unresolved functional concerns.

## Important review fixes

- Added a shared logical-character decoder used by `peek`, `peek2`, and `bump`.
  CRLF and bare CR now both normalize to `\n`, preserving lookahead/consume
  consistency while advancing by the correct byte count.
- Added Reader tests for CRLF, bare CR, lookahead/consume consistency, EOF,
  remaining input, multi-byte UTF-8 mark advancement, and BOM mark position.
- Restricted `Reader` to `pub(crate)` and removed its public crate re-export.
  Unused methods are temporarily allowed until the Scanner consumes Reader.

## Fix verification

- `cargo test -p ozzyyaml-parser reader::`: passed, 9 tests.
- `cargo test -p ozzyyaml-parser`: passed, 15 unit tests and 0 doc tests.
- `cargo clippy -p ozzyyaml-parser -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
