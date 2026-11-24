# ozzyyaml grammar-complete parse track — Design Spec

**Date:** 2026-11-25  
**Status:** Approved for implementation planning  
**Parent:** [2026-11-24-ozzyyaml-conformance-design.md](./2026-11-24-ozzyyaml-conformance-design.md)  
**Related:** [2026-11-25-ozzyyaml-chapter5-chars-design.md](./2026-11-25-ozzyyaml-chapter5-chars-design.md)

## Intent

Close the remaining gap between ozzyyaml’s event parser and YAML 1.2.2 so that:

1. Structural parsing follows the **productions** (not indent-stack heuristics), and  
2. **100%** of valid [yaml-test-suite](https://github.com/yaml/yaml-test-suite) cases with `test.event` pass.

Chapter 5 character classes and §5.2 encoding are already enforced. What remains is Chapters 6–9 structure: anchors, quoted folding fidelity, flow pairs, block indent/tab, and compact/same-indent block collections — about 22 suite misses today (~91% → 100%).

Approach: **grammar-shaped rewrite** of the structural scanner/parser (option 1). Machine-readable grammar codegen stays deferred.

## Goals

1. Production-faithful parse for the presentation stream → events.
2. 100% valid `test.event` suite coverage (event-line equality in the existing harness format).
3. Stable public API: `parse_str` / `Parser` / `Event` (and thus compose / emit / Python) keep working.
4. Honest docs: `docs/conformance.md` claims full event-suite coverage when Done.

## Non-goals

- PEG/codegen from a machine-readable grammar (later)
- Emit presentation fidelity (comments, tags-as-written, layout)
- YAML 1.1 merge keys / PyYAML quirk mode
- Serde
- Changing Core strict-tag policy
- Re-litigating Chapter 5 / §5.2 (already done)

## Architecture

**Keep**

- `chars.rs` Chapter 5 predicates and enforcement sites  
- Reader (line breaks, marks, limits, leading BOM skip)  
- Escape decode table; scalar scanners as a base (folding corrected to productions)  
- Public event/token shapes consumers already use  
- Compose / emit / Python as event/`Value` consumers  

**Rewrite / replace**

- Indent-stack “practical” block collection logic (`roll_indent` as the sole structure authority)  
- Hand-rolled `is_anchor_char` (replace with `[102] ns-anchor-char`)  
- Flow-pair / empty-key paths that are incomplete  
- Quoted multi-line folding that disagrees with `[74]` / `[112]`–`[116]` / single-quoted multi-line  
- Ad-hoc simple-key rules that contradict the grammar  

**Shape:** Hand-written recursive descent / production-parameterized parsers (`s-indent(n)` as context), modules named after production families. Not a full PEG toolchain in this track.

## Phases

| Phase | Focus | Spec | Suite IDs (primary) |
| --- | --- | --- | --- |
| **0** | Harness: all valid `test.event` report + keep current allowlist as regression gate | — | infra |
| **1** | Anchor/alias charset | `[102]`–`[104]` §6.9.2 | `W5VH` |
| **2** | Quoted multi-line folding + empty quoted | `[74]`, `[112]`–`[116]`, single-quoted multi-line | `7A4E` `9TFX` `PRH3` `T4YY` `NP9H` `Q8AD` `NAT4` |
| **3** | Flow pairs / empty keys / nested empty flow | `[139]`, `[146]`, `[150]`–`[154]` | `CFD4` `9MMW` `5MUD` `K3WX` `7ZZ5` |
| **4** | Block indent + tab vs space | `[63]` `[68]` `[163]` §8.2 | `R4YG` |
| **5** | Compact / same-indent / zero-indent block collections | `[183]`–`[195]` | `AZ63` `RLU9` `S9E8` `AB8U` `57H4` `S3PD` `6PBE` `SKE5` |
| **6** | Full-suite gate; drop subset allowlist; docs | — | **all** valid `test.event` |
| **7** (optional) | Differential fuzz vs another 1.2.2 event oracle | — | reliability |

Phase 5 is the hard core. Phases 1–4 land as production-faithful modules and delete old special cases as they go. Re-run the full event set every phase so folding/indent fixes do not silently regress greens.

## Testing

- Phase 0: dual mode — (a) current allowlist must stay green; (b) “all valid IDs” failure report.  
- Each phase: TDD from the listed suite cases + unit tests that cite production numbers.  
- Phase 6: one harness = every valid `test.event` case; allowlist file removed or reduced to a note that all valid IDs are required.  
- `cargo test` workspace + Python `maturin develop` / pytest each phase; compose allowlist and emit round-trip must not regress.

## Risks

- Block rewrite may break currently green indent cases → Phase 0 regression gate; prefer direct cutover with strong tests over long-lived feature flags.  
- Quoted folding fixes may perturb nearby green IDs → full event re-run each phase.  
- Broader `[102]` charset may interact with plain-scalar stop rules → explicit boundary tests.

## Success criteria

- 100% of valid suite cases with `test.event` pass event equality.  
- Remaining clusters implemented as named production expansions, not indent heuristics.  
- `docs/conformance.md` states full event-suite coverage; emit remains data round-trip only; Core-only schema unchanged.

## Baseline

At design time: **233 / 255** allowlisted valid `test.event` cases; **22** misses listed in the phase table above.
