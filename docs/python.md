# Python bindings

ozzyyaml can be used from Python through a small Maturin / PyO3 package. The
heavy lifting stays in Rust; Python sees ordinary native values.

Wheels are built in CI. Publishing to PyPI is not set up yet, so for now you
install from a local checkout.

## Install from a checkout

```bash
git submodule update --init --recursive
cd python
python3 -m venv .venv
source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install maturin pytest
maturin develop
pytest
```

The import name is `ozzyyaml`.

## Load and dump

```python
import ozzyyaml

doc = ozzyyaml.load("{name: Ada, ok: true}\n")
assert doc == {"name": "Ada", "ok": True}

text = ozzyyaml.dump({"items": [1, 2]}, style="flow")
assert ozzyyaml.load(text) == {"items": [1, 2]}

docs = ozzyyaml.load_all("1\n---\n2\n")
assert docs == [1, 2]
```

Supported dump styles are `"flow"` (default) and `"block"`.

Optional limit keyword arguments on `load` / `load_all` map to the Rust
`Limits` fields: `max_anchors`, `max_nesting_depth`, and `max_input_bytes`.

## Types

| YAML / Rust | Python |
| --- | --- |
| null | `None` |
| bool | `bool` |
| int | `int` |
| float | `float` |
| string | `str` |
| sequence | `list` |
| mapping | `dict` (insertion order; duplicate keys keep the last value) |

The loader uses the YAML 1.2.2 Core schema, so values like `yes` stay strings
rather than becoming booleans.

Shared aliases that form a DAG become one Python object referenced from
several places (`a is b`). **Cycles cannot be represented as ordinary lists
and dicts**; `load` raises `OzzyYamlError` in that case. Full cycle support
belongs with the structured Value API (phase B).

## Errors

Failures from the Rust stack surface as `ozzyyaml.OzzyYamlError`. Passing an
unsupported Python type to `dump` raises `TypeError`.

## See also

- [Architecture](architecture.md) — how compose and emit work in Rust
- Repository [README](../README.md)
