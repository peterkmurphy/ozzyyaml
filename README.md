# ozzyyaml

From-scratch YAML **1.2.2** stack (Rust-first, Python/PyPI later). BSD-3-Clause.

## Roadmap

1. **ozzyyaml-parser** — events (this milestone)
2. **ozzyyaml** — compose + Core schema
3. **ozzyyaml-emit** — dump
4. **PyPI** — Maturin wheels

Motivation: libyaml/PyYAML reject Unicode anchors/aliases; patches sat unmerged for years.

See `docs/superpowers/specs/2026-11-20-ozzyyaml-design.md`.
