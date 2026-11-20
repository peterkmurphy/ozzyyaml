# ozzyyaml

**YAML 1.2.2** for Python, backed by a from-scratch Rust implementation.
Licensed under the BSD-3-Clause.

ozzyyaml exists because common stacks still struggle with parts of the 1.2.2
spec. In particular, **libyaml** (and therefore **PyYAML**) rejects Unicode in
anchor and alias names. ozzyyaml accepts them on purpose (`&汉`, `*😀`, and
similar).

## Install

```bash
pip install ozzyyaml
```

Wheels are published for Linux (x86_64 + aarch64), Windows (x64), and macOS
(Apple Silicon). Requires Python 3.10 or newer.

Contributors can still [build from a checkout](#develop-from-a-checkout).

## Quick start

Two APIs share the same parser:

- **Native** — ordinary `dict` / `list` / scalars (familiar, but cycles error)
- **Value** — structured nodes that keep alias identity, including cycles

```python
import ozzyyaml

doc = ozzyyaml.load("{name: Ada, ok: true}\n")
assert doc == {"name": "Ada", "ok": True}

print(ozzyyaml.dump({"items": [1, 2]}))  # flow style by default

# Shared aliases become the same Python object
v = ozzyyaml.load_value("[&a hello, *a]\n")
assert v[0] is v[1]
assert v[0].as_str() == "hello"

# Cycles need the Value API
cycle = ozzyyaml.load_value("&a [*a]\n")
assert cycle[0] is cycle[0][0]
```

Dump styles are `"flow"` (default) and `"block"`. Multi-document streams use
`load_all` / `dump_all` (native) or `load_all_value` (structured).

Failures from the Rust stack raise `ozzyyaml.OzzyYamlError`.

## What you get

| Need | Use |
| --- | --- |
| Everyday config / data | `load` / `dump` |
| Aliases that form a DAG | either API (`a is b` for shared nodes) |
| Cycles | `load_value` / `dump_value` |
| Resource caps | `max_anchors`, `max_nesting_depth`, `max_input_bytes` keyword args |

The loader follows the YAML **1.2.2 Core** schema, so plain `yes` / `no` stay
strings rather than becoming booleans.

## Develop from a checkout

```bash
git clone https://github.com/peterkmurphy/ozzyyaml.git
cd ozzyyaml
git submodule update --init --recursive
cd python
python3 -m venv .venv
source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install maturin pytest
maturin develop
pytest
```

## Learn more

- [Python guide](https://github.com/peterkmurphy/ozzyyaml/blob/main/docs/python.md) —
  types, limits, and the full Value API
- [Architecture](https://github.com/peterkmurphy/ozzyyaml/blob/main/docs/architecture.md) —
  how the Rust pipeline fits together
- [Repository](https://github.com/peterkmurphy/ozzyyaml) — Rust crates and CI
