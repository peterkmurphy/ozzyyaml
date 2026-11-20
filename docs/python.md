# Python bindings

ozzyyaml can be used from Python through a small Maturin / PyO3 package. The
heavy lifting stays in Rust; Python can either receive ordinary native values
or structured `Value` objects that preserve alias identity (including cycles).

## Install

```bash
pip install ozzyyaml
```

Requires Python 3.10 or newer. Wheels are built for Linux, macOS, and Windows.
Until the first PyPI release is up, install from a checkout instead.

## Publishing to PyPI

CI workflow: [`.github/workflows/publish-pypi.yml`](../.github/workflows/publish-pypi.yml).

1. On [pypi.org](https://pypi.org), create an API token scoped to the `ozzyyaml`
   project (or your account, for the first upload that creates the project).
2. In the GitHub repo, add a secret named **`PYPI_API_TOKEN`** with that token.
3. Ensure `python/pyproject.toml` version matches the tag you will push
   (for example `0.1.0` ↔ `v0.1.0`).
4. Tag and push:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The workflow builds manylinux (x86_64 + aarch64), Windows (x64), and macOS
(x86_64 + aarch64) wheels plus an sdist, smoke-tests on the native runners,
then uploads with Maturin. `workflow_dispatch` builds artifacts without
uploading; only `v*` tags publish.

### Install from a checkout

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

## Native load and dump

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

Optional limit keyword arguments on `load` / `load_all` (and the Value
variants below) map to the Rust `Limits` fields: `max_anchors`,
`max_nesting_depth`, and `max_input_bytes`.

### Native types

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
and dicts**; `load` raises `OzzyYamlError` in that case. Use the structured
Value API below when you need cycles.

## Structured Value API

`load_value` / `load_all_value` / `dump_value` return (or accept) `ozzyyaml.Value`
wrappers around the Rust graph. Alias identity is preserved: the same
underlying node is always the same Python object.

```python
import ozzyyaml

v = ozzyyaml.load_value("[&a hello, *a]\n")
assert v.kind == "seq"
assert v[0] is v[1]
assert v[0].same(v[1])
assert v[0].as_str() == "hello"

# Cycles are fine
cycle = ozzyyaml.load_value("&a [*a]\n")
assert cycle[0] is cycle[0][0]

text = ozzyyaml.dump_value(cycle)
assert ozzyyaml.load_value(text)[0] is ozzyyaml.load_value(text)[0][0]
```

Useful members:

| Member | Purpose |
| --- | --- |
| `kind` | `"null"`, `"bool"`, `"int"`, `"float"`, `"str"`, `"seq"`, or `"map"` |
| `as_bool` / `as_int` / `as_float` / `as_str` | Scalar accessors (or `None`) |
| `len(v)` | Length of a sequence or mapping |
| `v[i]` | Sequence index (supports negatives) |
| `v["key"]` | Mapping lookup by scalar key |
| `v[i]` on a map | `[key, value]` pair by entry index |
| `v.same(other)` | Same underlying node (alias identity) |
| `dump_value(v, style=...)` | Emit YAML, preserving anchors/aliases for shared nodes |

`Value` is **unsendable**: do not move instances across threads.

## Errors

Failures from the Rust stack surface as `ozzyyaml.OzzyYamlError`. Passing an
unsupported Python type to `dump` raises `TypeError`.

## See also

- [Architecture](architecture.md) — how compose and emit work in Rust
- Repository [README](../README.md)
