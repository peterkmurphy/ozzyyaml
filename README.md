# ozzyyaml

[![ci](https://github.com/peterkmurphy/ozzyyaml/actions/workflows/ci.yml/badge.svg)](https://github.com/peterkmurphy/ozzyyaml/actions/workflows/ci.yml)

ozzyyaml is a from-scratch **YAML 1.2.2** stack, written in Rust first, with
Python bindings on the way. It is licensed under the BSD-3-Clause.

The project exists because common YAML stacks still struggle with parts of the
1.2.2 spec that matter in real documents. In particular, **libyaml** (and
therefore **PyYAML**) rejects Unicode characters in anchor and alias names,
treating them as ASCII-only. A fix has sat unmerged for years
([libyaml#132](https://github.com/yaml/libyaml/issues/132)). ozzyyaml accepts
Unicode anchors on purpose (`&汉`, `*😀`, and similar).

## What you get

The repository is a Cargo workspace with three main crates:

| Crate | Role |
| --- | --- |
| `ozzyyaml-parser` | Turns YAML text into a stream of parse events |
| `ozzyyaml` | Builds those events into a shared document graph (`Value`) using the YAML 1.2.2 Core schema |
| `ozzyyaml-emit` | Writes a `Value` graph back out as YAML text |

Compose supports shared aliases (including cycles) via `Rc` identity.
Emit can dump a single document or a multi-document stream, with flow style by
default and optional block style.

## Quick start (Rust)

```bash
git submodule update --init --recursive
cargo test
```

```rust
use ozzyyaml::from_str;
use ozzyyaml_emit::to_string;

fn main() {
    let value = from_str("[1, 2]\n").unwrap();
    let yaml = to_string(&value).unwrap();
    println!("{yaml}");
}
```

CI runs `rustfmt`, Clippy with warnings denied, and the full workspace test
suite. Checkouts must fetch Git submodules so the yaml-test-suite harness can
run.

## Python

Bindings live under `python/` and are built with Maturin. Native helpers return
ordinary Python types; the structured `Value` API preserves alias identity
(including cycles):

```python
import ozzyyaml

assert ozzyyaml.load("[1, 2]\n") == [1, 2]
print(ozzyyaml.dump({"a": True}))

v = ozzyyaml.load_value("[&a x, *a]\n")
assert v[0] is v[1]
```

See [docs/python.md](docs/python.md) for install-from-checkout steps, limits,
and behaviour around aliases and cycles. PyPI upload is not configured yet;
CI builds and smoke-tests wheels.

## Roadmap

1. Event parser (`ozzyyaml-parser`) — done
2. Compose + Core schema (`ozzyyaml`) — done
3. Emit (`ozzyyaml-emit`) — done
4. Python package (Maturin wheels; PyPI publish later) — done (wheels in CI; no PyPI upload yet)
5. Structured Python `Value` API — done
6. Optional later work: Serde, PyYAML-compat quirks

## Learn more

- [Architecture overview](docs/architecture.md) — how the pipeline fits together
- [Python guide](docs/python.md) — install and API notes
- Design notes live under `docs/superpowers/specs/` for contributors who want
  the detailed decision history
