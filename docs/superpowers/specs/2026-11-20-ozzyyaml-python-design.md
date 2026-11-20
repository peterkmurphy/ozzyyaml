# ozzyyaml Python bindings and packaging — Design Spec

**Date:** 2026-11-20  
**Status:** Approved  
**Parent:** [2026-11-20-ozzyyaml-design.md](./2026-11-20-ozzyyaml-design.md)

## Intent

Ship a Python package for ozzyyaml from this monorepo, built with Maturin and
PyO3. Domain logic stays in Rust. The first public surface looks familiar to
PyYAML users (native Python types). A second step exposes a structured Value
API that preserves alias identity and cycles. Human-facing documentation is
rewritten in clear prose before the Python work lands.

PyPI **upload** is out of scope for this milestone: we build and CI-test wheels
only. Publishing `0.1.0` comes later (trusted publisher or API token).

## Work order

1. Rewrite human-facing docs (`README.md`, `docs/architecture.md`).
2. Python phase A — native `load` / `dump` API, Maturin layout, CI wheels →
   commit → push.
3. Python phase B — structured Value / graph API → commit → push.
4. Do **not** spend this milestone rewriting agent-oriented files under
   `docs/superpowers/` into long-form prose.

## Documentation rewrite (step 1)

### README

Readable overview for newcomers:

- What ozzyyaml is (YAML 1.2.2, Rust-first, Unicode anchors)
- Why it exists (libyaml / PyYAML Unicode-anchor gap)
- Crate map: parser, compose, emit
- Rust quick start (`cargo test`, short `from_str` / `to_string` examples)
- Roadmap with completed milestones marked
- Pointer to `docs/architecture.md`
- After phase A: `pip install` / wheel note and a short Python example (even if
  install is “from git / local maturin” until PyPI publish)

### Architecture

Rewrite `docs/architecture.md` in plain language:

- Pipeline: Reader → Scanner → Parser → Composer → Emitter
- Shared graphs, aliases, cycles, `max_anchors`
- What each crate owns
- Keep accurate; avoid checklist shorthand

### Python guide

Add `docs/python.md` when phase A lands: install (dev + eventual PyPI),
`load` / `dump` examples, limits, and (after phase B) the Value API.

## Packaging layout

```
python/
  pyproject.toml          # maturin backend
  README.md               # short package readme or link to root
  ozzyyaml/
    __init__.py           # public façade
    _native.pyi           # optional stubs
  tests/                  # pytest
crates/ozzyyaml-python/   # OR maturin points at a small cdylib crate
  Cargo.toml              # pyo3 cdylib wrapping ozzyyaml + ozzyyaml-emit
  src/lib.rs
```

Preferred structure (one clear choice for implementation):

- Maturin project under `python/` with `tool.maturin` module name `ozzyyaml._native`
- A workspace member crate `crates/ozzyyaml-python` (`cdylib`) that the Maturin
  manifest builds; depends on `ozzyyaml` and `ozzyyaml-emit`
- Pure-Python `ozzyyaml/__init__.py` imports `_native` and exposes the public API

Distribution name and import name: **`ozzyyaml`** (confirm still free on PyPI
at publish time).

License: BSD-3-Clause, matching the repo.

Requires-Python: **>=3.10** (adjust only if PyO3/Maturin force a higher floor).

## Phase A — native Python types

### Public API

```python
def load(stream: str | bytes, *, max_anchors: int | None = None,
         max_nesting_depth: int | None = None,
         max_input_bytes: int | None = None) -> object: ...

def load_all(stream: str | bytes, *, ...) -> list[object]: ...

def dump(data: object, *, style: str = "flow") -> str: ...

def dump_all(docs: list[object], *, style: str = "flow") -> str: ...
```

`style` is `"flow"` (default) or `"block"`, matching Rust `EmitStyle`.

Limit kwargs map onto Rust `Limits` when provided; omit → Rust defaults.

### Type mapping (compose → Python)

| Rust `Value` | Python |
| --- | --- |
| Null | `None` |
| Bool | `bool` |
| Int | `int` |
| Float | `float` |
| String | `str` |
| Sequence | `list` |
| Mapping | `dict` (ordered; duplicate keys: last wins, documented) |

### Type mapping (Python → emit)

Symmetric conversion into Rust `Value`, then `ozzyyaml-emit`. Unsupported
Python types (custom classes, sets, …) raise a clear `TypeError`.

### Aliases and cycles (natives)

- **DAG sharing:** when converting Rust → Python, shared `Rc` nodes become one
  Python object referenced from multiple places (`is` identity), where the
  conversion can do so safely.
- **Cycles:** natives path must not hang. Raise a typed error (e.g.
  `OzzyYamlError` / `CycleError`) when a cycle would be required to build
  ordinary `list`/`dict` trees, **or** document and implement a conservative
  policy agreed at implementation time that never loops. Prefer **error on
  cycle** for `load` natives in v1 so behaviour is obvious.
- Emit side: converting Python objects that are cyclic via mutual references
  should error similarly unless we detect and emit anchors (phase B Value API
  is the better place for full cycle round-trip).

### Errors

Map Rust errors to a small Python exception hierarchy rooted at
`OzzyYamlError`, with message text preserved. Marks may appear in the message
string in phase A (dedicated attributes optional).

## Phase B — structured Value API

Separate commit and push after phase A.

### Goals

- Python objects that mirror Rust `Value` and preserve identity for aliases
  and cycles (including cyclic graphs).
- Round-trip: YAML → Value → YAML with sharing intact.
- Coexist with natives: e.g. `load_value` / `dump_value` (exact names chosen
  at implementation; avoid breaking phase A names).

### Non-goals for B

- Full PyYAML `Loader` / `Dumper` class hierarchy
- Arbitrary Python object serialization (pickle tags, etc.)

## CI

Extend GitHub Actions:

- Keep existing Rust job (fmt, clippy, test, submodules).
- Add a job (or matrix) that:
  - installs Maturin
  - builds a wheel
  - creates a clean venv
  - installs the wheel
  - runs a smoke import + a few `load`/`dump` assertions (and pytest under
    `python/tests` when present)

Target at least **manylinux** on Ubuntu; macOS/Windows wheel jobs are desirable
but can start with Linux-only if time-boxed, then widen.

## Non-goals (this milestone)

- Uploading to PyPI
- PyYAML-compat schema mode
- Serde
- Rewriting all `docs/superpowers/**` into long prose

## Testing

- Rust: existing workspace tests stay green
- Python: unit tests for mapping, limits, dump styles, error cases
- Cycle behaviour tests for natives (expect error)
- Phase B: alias `is` identity and cycle round-trip tests

## Success criteria

- Docs readable to a newcomer without reading agent plans
- `maturin develop` / wheel install works locally
- CI builds a wheel and smoke-tests it
- Phase A pushed to `main` on GitHub
- Phase B follows as its own commit + push with Value identity preserved
