# Conformance status

ozzyyaml targets YAML 1.2.2 but is **not yet complete**.

As of the 0.1.0 line, the event parser passes roughly 40% of
[yaml-test-suite](https://github.com/yaml/yaml-test-suite) cases that ship
`test.event` ground truth. Major gaps include multi-line scalar folding,
block scalars (`|` / `>`), explicit keys (`?`), the full double-quoted escape
set, and UTF-16/32 streams.

Track design:
[superpowers/specs/2026-11-24-ozzyyaml-conformance-design.md](superpowers/specs/2026-11-24-ozzyyaml-conformance-design.md).  
Implementation plan:
[superpowers/plans/2026-11-24-ozzyyaml-conformance.md](superpowers/plans/2026-11-24-ozzyyaml-conformance.md).

What *is* intentional and spec-aligned today: Unicode anchors, Core-schema
`yes` / `no` as strings (not booleans), and no YAML 1.1 merge keys.
