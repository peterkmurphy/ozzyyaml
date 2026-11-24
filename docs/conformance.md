# Conformance status

ozzyyaml implements **YAML 1.2.2** for parse, Core-schema compose, and data
round-trip emit, with the caveats below.

## What is covered

- Event parser language: multi-line plain/quoted folding (§8.1.3), block
  scalars `|` / `>` (§8.1.4–§8.1.6), full double-quoted escapes (§7.3.2),
  explicit keys (`?`), `%YAML` / `%TAG` directives, Unicode anchors
- Core compose with **strict** known tags (`!!int twelve` is an error)
- Emit → load data round-trip for acyclic documents in the
  parser∩compose allowlist
- Chapter 5 character classes: `c-printable` / `nb-char` / `nb-json`, reserved
  `@`/`\`` plain starts, and `ns-uri-char` / `ns-tag-char` for tags
- Byte APIs (`from_slice` / `load_all_from_slice`): UTF-8, UTF-16LE/BE, and
  UTF-32LE/BE via BOM **and** §5.2 null-pattern detection. String APIs and
  emit remain UTF-8. Mid-document BOM is an error; BOM inside double-quoted
  scalars is allowed
- Default `Limits`: 256 MiB input, 10 000 nesting, 100 000 anchors (overridable)

## Honest caveats

- **yaml-test-suite events:** about **91%** of valid cases with `test.event`
  (233 / 255) are allowlisted today. Remaining misses are mostly compact
  empty-key / nested-flow edge cases, a few event-shape mismatches, and
  tabs used as block-scalar indentation. See
  `crates/ozzyyaml-parser/tests/suite_allowlist.txt`.
- **Emit** does not preserve comments, tags as written, or source layout —
  only `Value` data equality after dump→load.
- **Core only:** `yes` / `no` stay strings; no YAML 1.1 merge keys.

Design tracks:
[conformance](superpowers/specs/2026-11-24-ozzyyaml-conformance-design.md),
[Chapter 5 chars](superpowers/specs/2026-11-25-ozzyyaml-chapter5-chars-design.md).  
Plans:
[conformance](superpowers/plans/2026-11-24-ozzyyaml-conformance.md),
[Chapter 5](superpowers/plans/2026-11-25-ozzyyaml-chapter5-chars.md).
