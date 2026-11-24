# Conformance status

ozzyyaml targets YAML 1.2.2 but is **not yet complete**.

The event parser’s allowlist is growing as language gaps close (see
`crates/ozzyyaml-parser/tests/suite_allowlist.txt`). Multi-line plain and
quoted scalar folding (§8.1.3) is implemented. Remaining major gaps include
block scalars (`|` / `>`), explicit keys (`?`), the full double-quoted escape
set, and UTF-16/32 streams.

Track design:
[superpowers/specs/2026-11-24-ozzyyaml-conformance-design.md](superpowers/specs/2026-11-24-ozzyyaml-conformance-design.md).  
Implementation plan:
[superpowers/plans/2026-11-24-ozzyyaml-conformance.md](superpowers/plans/2026-11-24-ozzyyaml-conformance.md).

What *is* intentional and spec-aligned today: Unicode anchors, Core-schema
`yes` / `no` as strings (not booleans), and no YAML 1.1 merge keys.
