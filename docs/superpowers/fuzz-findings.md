# Fuzz findings triage

Targets: `fuzz/fuzz_targets/fuzz_parse_str.rs`, `fuzz_roundtrip.rs`.
CI: bounded ~90s each on every push/PR (`.github/workflows/ci.yml`);
nightly ~15 min each (`.github/workflows/fuzz-nightly.yml`).

## Log

| Date | Target | Duration | Result |
| --- | --- | --- | --- |
| 2026-11-26 | fuzz_parse_str | 30s local (~294k runs) | No crashes |
| 2026-11-26 | fuzz_roundtrip | 30s local | No crashes |
| 2026-11-26 | fuzz_roundtrip | CI ~90s / local repro | **LSan:** Rc cycle retained after `from_str` of alias graphs (e.g. `&n?k?[ *n…`). Not UB — Value uses `Rc` for YAML anchors/aliases including cycles. CI runs roundtrip with `-detect_leaks=0`; ASan still active. |

Refresh corpus from the suite with `./fuzz/seed_corpus.sh` when adding
interesting seeds. Document every crash (or intentional non-fix after
Limits triage) in this table before cutting a release that claims fuzz CI.
