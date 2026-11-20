# ozzyyaml — Design Spec

**Date:** 2026-11-20  
**Status:** Draft for review  
**License:** BSD-3-Clause  

## Intent

Build **ozzyyaml**: a from-scratch YAML **1.2.2** parsing/emitting stack that is correct, readable, test-heavy, and a showcase of library best practices. Motivated by PyYAML/libyaml gaps (especially Unicode in anchors/aliases, and 1.1-era defaults) where patches have long sat unmerged.

This is a personal project under `/home/peter/projects/ozzyyaml` (not a Greens project). Intended eventual consumer: **isityaml** (also BSD-3-Clause).

## Goals

- Spec: [YAML 1.2.2](https://yaml.org/spec/1.2.2/)
- Rust-first, Python bindings later
- Cleanest understandable code over cleverness
- Test-heavy; yaml-test-suite as a primary oracle
- `#![forbid(unsafe_code)]` on Rust crates
- Learning artifact: architecture docs + layer-by-layer milestones others can study with pride

## Non-goals (v1 era)

- Comment / whitespace lossless round-trip
- YAML 1.1 as the default schema
- Drop-in PyYAML API clone (inspired ergonomics OK; not quirk-identical)
- Fixing or forking libyaml/C

## Milestones

| Order | Deliverable | Crate |
| --- | --- | --- |
| A | Event stream (scan → parse → events) | `ozzyyaml-parser` |
| B | Compose + Core-schema construct → shared document graph | `ozzyyaml` |
| Emit | Value/events → YAML text | `ozzyyaml-emit` |
| Python | Thin PyO3 façade + pure-Python public API; **publish to PyPI** | `python/` (Maturin) |

**PyYAML-compat schema mode** is planned after Core schema works; it is not part of Milestone B’s first cut.

## Approach

**Hand-written scanner + event parser** (libyaml/saphyr-shaped), not parser-combinator-first and not grammar-codegen-first. Maximum clarity for indentation-sensitive YAML and per-production tests.

## Workspace layout

```
ozzyyaml/
  Cargo.toml                 # workspace
  crates/
    ozzyyaml-parser/         # Milestone A
    ozzyyaml/                # Milestone B
    ozzyyaml-emit/           # later
  python/                    # later
  docs/
    architecture.md          # learning: pipeline, anchors, limits
    superpowers/specs/       # this document
  README.md
  LICENSE                    # BSD-3-Clause
  CONTRIBUTING.md
```

Each crate keeps a **tiny public surface**; internals stay private. Crate roots re-export only curated APIs.

## Milestone A — `ozzyyaml-parser`

### Pipeline

```
&str / &[u8]
  → Reader   (encoding, BOM, position marks)
  → Scanner  (tokens)
  → Parser   (YAML 1.2.2 events)
  → pull API / iterator of Event
```

### Public API (sketch)

- `Event` and related enums (stream/doc/collection/scalar/alias, styles, optional tag/anchor)
- `parse_str` / `parse_bytes` and/or `Parser::next_event()`
- `Mark` (index, line, column) on errors
- Typed `Error` / `Result` — no panics on bad input

### Hard rules

- YAML 1.2.2 productions
- Unicode anchors and aliases supported (explicit acceptance tests; contrast libyaml)
- No document tree, no schema typing, no emit

## Milestone B — `ozzyyaml`

- Consume parser events
- **Composer:** shared node graph (references, not copies); aliases point at the same node
- **Constructor:** default **YAML 1.2.2 Core schema** (no 1.1 sexagesimal; `yes`/`no`/`on`/`off` as strings; octal via `0o` form, etc.)
- Later: optional **PyYAML-compat** schema mode
- Public API sketch: `from_str` / `from_slice` → `Document` / `Value`; traversal without silent alias expansion

## Emit — `ozzyyaml-emit`

- Value-level dump first (not lossless)
- Must emit Unicode anchors correctly when present

## Python (last)

- Domain logic stays in Rust; PyO3 is a thin adapter
- Maturin packaging; extension module as `_native` (or similar); idiomatic Python façade in `__init__.py`
- **Distributed as a normal Python library on [PyPI](https://pypi.org/)** (platform wheels + sdist), e.g. `pip install ozzyyaml` (name confirmed at publish time if taken)
- Default schema = Core; PyYAML-compat when available in Rust
- CI builds manylinux/macOS/Windows wheels; smoke-test install from the built artifact in a clean venv before release

## Security & resource limits

Billion-laughs style attacks are primarily **alias/anchor** related.

**Representation:** shared graph (e.g. `Rc` / arena + `NodeId`). Aliases are references, not deep copies. This removes the classic memory blow-up from materializing duplicates.

**Hard limits (configurable; safe defaults for untrusted input):**

- `max_anchors` — primary DoS guard
- `max_nesting_depth` — secondary
- `max_input_bytes` — secondary for untrusted streams

On breach: typed `LimitExceeded { kind, limit }`, not hang/OOM if avoidable.

Flattening APIs (JSON export, deep owned trees) must be explicit and budgeted; they must not silently expand a DAG into exponential work.

## Errors

- `thiserror`-style typed errors (`ScanError`, `ParseError`, `ComposeError`, `LimitExceeded`, …)
- Marks on errors where possible
- Recoverable failures return `Err`; input never panics the library

## Testing

- Unit tests per scanner/parser concern (including Unicode anchors/aliases)
- [yaml-test-suite](https://github.com/yaml/yaml-test-suite) for events (A), then compose (B)
- Adversarial tests: `max_anchors`, deep nesting, cyclic aliases
- CI: `cargo test`, `clippy -D warnings`, `rustfmt`, documented MSRV
- Later: fuzz smoke on parse; `cargo semver-checks` after 0.1 publish

## Engineering bar

- MSRV pinned and CI-checked (recent stable; exact version chosen at scaffold time)
- README: roadmap A→B→emit→Python; motivation vs libyaml Unicode anchors
- `docs/architecture.md` for learners
- CONTRIBUTING: prefer adding a failing test-suite case first

## License

**BSD-3-Clause**, consistent with isityaml / blueskyfeed / chordgenerator, so isityaml can depend on ozzyyaml without a mixed-license story.

## Decisions log

| Decision | Choice |
| --- | --- |
| Name | ozzyyaml |
| Spec | YAML 1.2.2 |
| Greenfield | From-scratch (not a thin wrap of saphyr) |
| Parser style | Hand-written scanner + event parser |
| Packaging | Cargo workspace, layered crates |
| Milestone order | Events → document/Core → emit → Python |
| Default schema | 1.2.2 Core |
| Compat mode | PyYAML-compat later |
| Safety lint | `forbid(unsafe_code)` |
| Anchors in memory | Shared references |
| DoS primary brake | `max_anchors` (+ nesting/input caps) |
| License | BSD-3-Clause |
| Location | `/home/peter/projects/ozzyyaml` |
| Python distribution | PyPI package (Maturin wheels) |
