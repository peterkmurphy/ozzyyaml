# Python bindings

ozzyyaml can be used from Python through a small Maturin / PyO3 package. The
heavy lifting stays in Rust; Python can either receive ordinary native values
or structured `Value` objects that preserve alias identity (including cycles).

Coverage matches the Rust crates: YAML **1.2.2** parse, Core compose, and data
round-trip emit, including block scalars, explicit keys, double-quoted
escapes, and UTF-16/32 on `bytes` input (BOM or §5.2 null-pattern). See
[conformance.md](conformance.md) for suite percentages and honest caveats
(100% of valid `test.event` cases; 100% compose success for the same set via
`load_all` / `load_all_value`; emit is data equality, not
presentation-preserving).

## Install

```bash
pip install ozzyyaml
```

Requires Python 3.10 or newer. Wheels are published per CPython interpreter
for **3.10–3.14** on Linux (x86_64 + aarch64), Windows (x64), and macOS
(Apple Silicon + Intel via cross-build).

**Free-threading:** CI smoke-tests import/`load` on CPython **3.14t**
(`PYTHON_GIL=0`). That is a smoke gate, not a full free-threading support
claim — do not treat ozzyyaml as certified for free-threaded production yet.

For cutting a release (Trusted Publishing, tags, version bumps), see
[releasing.md](releasing.md).

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

# bytes: UTF-8, or UTF-16/32 with a BOM or null-pattern (§5.2)
assert ozzyyaml.load(b"\xff\xfea\x00\n\x00") == "a"
assert ozzyyaml.load(bytes([0x61, 0x00, 0x0A, 0x00])) == "a"
```

`load` / `load_all` (and the Value variants) accept `str` or `bytes`. String
input is UTF-8; byte input uses the same §5.2 detection as Rust `from_slice`
(BOM and null-pattern for UTF-8 / UTF-16 / UTF-32). Dump always emits UTF-8
text.

Supported dump styles are `"flow"` (default) and `"block"`.

Optional limit keyword arguments on `load` / `load_all` (and the Value
variants below) map to the Rust `Limits` fields: `max_anchors`,
`max_nesting_depth`, and `max_input_bytes`. Defaults match Rust (256 MiB
input, 10 000 nesting, 100 000 anchors).

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
rather than becoming booleans. Known tags are strict: `!!int twelve` and
`!!bool yes` raise `OzzyYamlError`.

Shared aliases (including cycles) become one Python object referenced from
several places (`a is b`). Collection **mapping keys** are coerced to hashable
tuples so they can live in a `dict`: a sequence key becomes a `tuple` of
items; a mapping key becomes a `tuple` of `(key, value)` pairs (nested
recursively). Scalar keys are unchanged. Prefer the structured Value API below
when you need the YAML graph without that key coercion.

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

- [Conformance status](conformance.md) — suite coverage and gaps
- [Architecture](architecture.md) — how compose and emit work in Rust
- Repository [README](../README.md)
