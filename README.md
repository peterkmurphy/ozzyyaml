# ozzyyaml

[![ci](https://github.com/peterkmurphy/ozzyyaml/actions/workflows/ci.yml/badge.svg)](https://github.com/peterkmurphy/ozzyyaml/actions/workflows/ci.yml)

From-scratch YAML **1.2.2** stack (Rust-first, Python/PyPI later). BSD-3-Clause.

Milestone A (`ozzyyaml-parser`) is the event layer. Milestone B (`ozzyyaml`)
composes those events into `Rc`-shared Core-schema [`Value`] trees
(`from_str` / `load_all`), including Unicode anchors and cyclic aliases.
`ozzyyaml-emit` dumps those trees back to YAML text (`to_string` / `dump_all`).

## Roadmap

1. **ozzyyaml-parser** — events ✓
2. **ozzyyaml** — compose + Core schema ✓
3. **ozzyyaml-emit** — dump ✓
4. **PyPI** — Maturin wheels

Motivation: libyaml/PyYAML reject Unicode anchors/aliases; patches sat unmerged
for years ([libyaml#132](https://github.com/yaml/libyaml/issues/132)).

See [docs/architecture.md](docs/architecture.md) for the Reader → Scanner →
Parser → Compose → Emit pipeline and `max_anchors`. Design notes:
`docs/superpowers/specs/2026-11-20-ozzyyaml-design.md`,
`docs/superpowers/specs/2026-11-20-ozzyyaml-milestone-b-design.md`, and
`docs/superpowers/specs/2026-11-20-ozzyyaml-emit-design.md`.

## Develop

```bash
git submodule update --init --recursive
cargo test
# or: cargo test -p ozzyyaml-parser / cargo test -p ozzyyaml / cargo test -p ozzyyaml-emit
```

```rust
use ozzyyaml::from_str;
use ozzyyaml_emit::to_string;
let v = from_str("[1, 2]\n").unwrap();
let _yaml = to_string(&v).unwrap();
```

CI checks format, clippy (`-D warnings`), and tests. Checkout must fetch
submodules so the yaml-test-suite harness can run.
