# ozzyyaml YAML 1.2.2 conformance track — design

**Date:** 2026-11-24  
**Status:** Approved — implementation plan at `docs/superpowers/plans/2026-11-24-ozzyyaml-conformance.md`  
**Baseline:** `main` / 0.1.0 line; audit in the known-deviations note (parser ~39.6% of suite cases with `test.event`)

## Problem

ozzyyaml was presented as a YAML 1.2.2 stack while only accepting a large subset of the language. That overclaim is unacceptable. The next product track closes the gap until the library honestly implements YAML 1.2.2 for parse, Core compose, and data round-trip emit, with UTF-16/32 streams last.

## Goals

1. **Parser:** ≈100% of [yaml-test-suite](https://github.com/yaml/yaml-test-suite) cases that provide `test.event` pass in the existing event harness; allowlist grows only with actually green IDs.
2. **Composer:** Core schema resolution with **strict** validation of known Core tags (mismatch → compose error).
3. **Emit:** Data round-trip (`parse → dump → parse` equal as `Value`) for the greened set; escape tables aligned so the emitter does not produce text the parser rejects. No claim of comment/tag/presentation fidelity.
4. **Limits:** Default `Limits` raised so ordinary APIs accept large legitimate documents; callers can still tighten via `*_with_limits`.
5. **Encodings (last):** UTF-16LE/BE and UTF-32LE/BE (BOM detection) on byte-oriented APIs per spec §5.2 / §5.5. Emit may remain UTF-8.
6. **Docs:** Until the bar is met, stop implying full 1.2.2; after it is met, claim it with the documented Limits and round-trip caveats.

## Non-goals

- PyYAML 1.1 quirk compatibility / merge keys (`<<`)
- Serde
- Comment preservation or exact source layout on emit
- Byte-identical dump of every input document
- Moving the repo under `github.com/yaml` (listing on yaml.org/libraries is separate)

## Policy decisions

| Topic | Decision |
| --- | --- |
| Success bar | Full stack (parser + Core compose + data round-trip), UTF-16/32 last |
| Same-line nested implicit mappings | Accept when the grammar allows (remove intentional rejection) |
| Known Core tags | Strict: invalid content for `!!null` / `!!bool` / `!!int` / `!!float` / `!!str` (as applicable) → error |
| Duplicate mapping keys | Keep current “retain duplicates” behaviour unless a later opt-in uniqueness mode is designed |
| Default Limits | Raise defaults; keep overridable |
| Approach | Phased language completion (not suite-greedy, not scanner rewrite) |

## Phases

Each phase ships with unit tests derived from named suite cases (and/or a focused escape/BOM matrix), then allowlist updates for IDs that actually pass. Spec citations follow YAML 1.2.2.

### Phase 0 — Honesty

- README / `docs/architecture.md` / `docs/python.md` / crate READMEs: do not claim full 1.2.2.
- Point at this design (or a short public gaps summary) until Done.

**Spec:** —  
**Tests:** docs review only  
**Emitter:** optional one-line note only  

### Phase 1 — Plain and quoted line folding

- Implement multi-line plain scalars (fold breaks to space; blank-line rules).
- Implement multi-line single- and double-quoted scalars; escaped line breaks in double-quoted (`\` + break).
- Revisit simple-key rules for multi-line plains (must not become keys illegally).

**Spec:** §8.1.3, §6.6, related §7.4 simple-key constraints  
**Tests:** unit cases from suite `9YRD`, `EX5H`, `36F6`, `HS5T`, `NB6Z`, `4CQQ`, `6WPF`; allowlist every ID that goes green  
**Emitter:** no  

### Phase 2 — Block scalars

- Scan/parse `|` and `>` with indentation and chomping indicators (`|-`, `|+`, `|2`, etc.).
- Emit `ScalarStyle::Literal` / `Folded` events from the parser (types already exist).

**Spec:** §8.1.4–§8.1.6  
**Tests:** ~11 direct suite block-scalar IDs + unit tests for indicators  
**Emitter:** parsing only required for suite green; optional block-scalar *emission* may follow in Phase 8 or a small add-on  

### Phase 3 — Double-quoted escape set + round-trip property

- Complete `c-ns-esc-char`: letter escapes (`\b` `\v` `\f` `\e` `\N` `\_` `\L` `\P`, …) and `\x` `\u` `\U`.
- Align emitter escapes with the parser (fix E1: dump must be loadable).
- Add property test: for allowlisted inputs, `compose(dump(compose(x)))` equals `compose(x)` as Values (identity of structure/scalars, not anchors/tags/comments).

**Spec:** §7.3.2 escape table  
**Tests:** escape matrix unit tests; property test; suite IDs unblocked by escapes  
**Emitter:** **yes** (required)  

### Phase 4 — Explicit keys (`?`)

- Scanner/parser support for `?` keys, including multi-line / complex keys as required by the suite.

**Spec:** §8.1.1 / §7.4.1 (explicit key productions as cited in the audit)  
**Tests:** ~22 suite `?` cases + multi-line explicit-key units  
**Emitter:** no (flow/block key emission already produces legal YAML)  

### Phase 5 — Directives

- Parse `%YAML`: reject unsupported versions (§9.1.3 behaviour).
- Validate `%TAG` directive lines; keep handle expansion correct.

**Spec:** §9.1–§9.2 (directives)  
**Tests:** `%YAML 9.9` rejected; suite directive/tag-related IDs  
**Emitter:** no  

### Phase 6 — Policy alignment

- Remove “nested implicit mapping on the same line is not allowed” where the spec permits the form.
- Strict Core tag constraint checks in the composer.

**Spec:** block/flow mapping productions; Core tag constraints §10.3 / §8.2.1.1 as applicable  
**Tests:** specimens that fail under current policy; `!!int twelve` → error; `!!int 42` → int  
**Emitter:** no  

### Phase 7 — Default Limits

- Raise `max_input_bytes`, `max_nesting_depth`, `max_anchors` defaults enough that legitimate large documents and the suite are not rejected by default.
- Keep `Limits` / `*_with_limits` for tightening (DoS posture remains available).

**Spec:** none (product policy)  
**Tests:** defaults accept large inputs used by suite/CI; custom low limits still error  
**Emitter:** no  

### Phase 8 — Compose allowlist + emit round-trip breadth

- Grow compose-level allowlists / shape checks for cases that need more than events.
- Expand round-trip coverage across the event-green set; fix emit bugs discovered.
- Optionally emit literal/folded styles when dumping multi-line strings (quality, not suite-gated).

**Spec:** Core §10  
**Tests:** compose harness; round-trip property widened  
**Emitter:** **yes** as needed  

### Phase 9 — UTF-16 / UTF-32 streams (last)

- Byte APIs detect BOM and decode UTF-16LE/BE, UTF-32LE/BE, UTF-8 before existing `&str` pipeline.
- Document that string APIs remain UTF-8; emit remains UTF-8.

**Spec:** §5.2, §5.5  
**Tests:** BOM/encoding matrix unit tests (suite is UTF-8-only)  
**Emitter:** UTF-8 only is fine  

## Definition of Done (track)

- Event harness: ≈100% of suite cases with `test.event` passing and allowlisted.
- Strict Core tag validation in place; same-line forms accepted per grammar.
- Round-trip property green on the allowlisted set (Value equality).
- Raised default Limits; overridable.
- UTF-16/32 supported on byte APIs.
- Public docs claim YAML 1.2.2 with honest caveats (Limits, no comment/tag presentation round-trip).

## Implementation notes

- Prefer extending the current Reader → Scanner → Parser → Composer → Emitter pipeline; no greenfield scanner.
- TDD against suite-derived units; never allowlist a failing ID.
- After Phase 0, each phase ends with `cargo test`, allowlist diff, and a commit (or small commit series) suitable for review.
- Filing order matches the audit recommendation; Phase 3 is urgent relative to emit correctness even if some folding IDs remain.

## Open items for the implementation plan (not blocked)

- Exact new default numeric Limits values (choose with suite + “large but finite” in mind).
- Whether block-scalar *emission* is bundled into Phase 8 or a tiny Phase 2b.
- Precise list of Core tags treated as “known” for strict checking (`!!str` content policy).
