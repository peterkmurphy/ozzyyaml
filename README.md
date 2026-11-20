# ozzyyaml

[![ci](https://github.com/peterkmurphy/ozzyyaml/actions/workflows/ci.yml/badge.svg)](https://github.com/peterkmurphy/ozzyyaml/actions/workflows/ci.yml)

From-scratch YAML **1.2.2** stack (Rust-first, Python/PyPI later). BSD-3-Clause.

Milestone A (`ozzyyaml-parser`) is usable for **event** consumers; compose is
not available yet.

## Roadmap

1. **ozzyyaml-parser** — events (this milestone)
2. **ozzyyaml** — compose + Core schema
3. **ozzyyaml-emit** — dump
4. **PyPI** — Maturin wheels

Motivation: libyaml/PyYAML reject Unicode anchors/aliases; patches sat unmerged
for years ([libyaml#132](https://github.com/yaml/libyaml/issues/132)).

See [docs/architecture.md](docs/architecture.md) for the Reader → Scanner →
Parser pipeline, `max_anchors`, and the shared-graph plan for Milestone B.
Design notes: `docs/superpowers/specs/2026-11-20-ozzyyaml-design.md`.

## Develop

```bash
git submodule update --init --recursive
cargo test -p ozzyyaml-parser
```

CI checks format, clippy (`-D warnings`), and tests. Checkout must fetch
submodules so the yaml-test-suite harness can run.
