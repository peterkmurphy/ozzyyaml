# ozzyyaml Python + Docs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite human-facing docs, then ship Python phase A (native load/dump + Maturin + CI wheels). Stop for the human to push. Phase B (structured Value) is a later gate after that push.

**Architecture:** Maturin builds workspace crate `ozzyyaml-python` (`cdylib`) into `ozzyyaml._native`. Pure-Python façade exposes PyYAML-ish `load`/`dump`. Docs rewritten in clear prose first.

**Tech Stack:** Rust 2021, PyO3, Maturin, pytest, GitHub Actions.

## Global Constraints

- `#![forbid(unsafe_code)]` on Rust crates (PyO3 may require allowing only where the crate attribute forces — prefer `ozzyyaml-python` without forbidding if PyO3 needs it; keep parser/compose/emit forbid)
- Domain logic stays in Rust
- No PyPI upload in this plan
- Phase B must not start until the human has pushed phase A and says to continue
- Human-facing docs: clear full sentences, not agent shorthand
- BSD-3-Clause; package name `ozzyyaml`; Python >=3.10

## Gate

After Task 5 (phase A complete + local verify): **STOP**. Tell the human phase A is ready to push. Do not start Task 6+ until they confirm the push and ask for phase B.

---

### Task 1: Rewrite README and architecture.md

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture.md`

- [ ] **Step 1: Rewrite README** in clear prose covering: purpose, Unicode-anchor motivation with libyaml#132 link, crate list, Rust install/test commands, short compose+emit example, roadmap with checkmarks, link to architecture. Avoid dumping paths to every agent spec; one “design notes” link to `docs/superpowers/specs/` is enough.

- [ ] **Step 2: Rewrite architecture.md** as a readable guide (same technical content as today: pipeline stages, limits, sharing/cycles, Unicode anchors, emit behaviour) with full sentences and short paragraphs. Keep accuracy.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/architecture.md
git commit -m "docs: rewrite README and architecture in clear prose"
```

---

### Task 2: Scaffold Maturin + `ozzyyaml-python` crate

**Files:**
- Create: `crates/ozzyyaml-python/Cargo.toml`
- Create: `crates/ozzyyaml-python/src/lib.rs` (minimal `#[pymodule]`)
- Create: `python/pyproject.toml`
- Create: `python/ozzyyaml/__init__.py`
- Modify: root `Cargo.toml` workspace members
- Modify: `.gitignore` if needed (already has Python entries)

**Interfaces:**
- Produces: importable `ozzyyaml._native` after `maturin develop`

- [ ] **Step 1: Workspace crate**

```toml
# crates/ozzyyaml-python/Cargo.toml
[package]
name = "ozzyyaml-python"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false

[lib]
name = "ozzyyaml_native"
crate-type = ["cdylib"]

[dependencies]
ozzyyaml = { path = "../ozzyyaml" }
ozzyyaml-emit = { path = "../ozzyyaml-emit" }
pyo3 = { version = "0.23", features = ["extension-module"] }
```

Add `crates/ozzyyaml-python` to workspace members.

Minimal `lib.rs`:

```rust
use pyo3::prelude::*;

#[pymodule]
fn _native(m: &Bound<PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
```

Module name must match Maturin config (`ozzyyaml._native`).

- [ ] **Step 2: `python/pyproject.toml`**

```toml
[build-system]
requires = ["maturin>=1.7,<2"]
build-backend = "maturin"

[project]
name = "ozzyyaml"
version = "0.1.0"
description = "YAML 1.2.2 parser and emitter with Unicode anchor support"
readme = "README.md"
requires-python = ">=3.10"
license = { text = "BSD-3-Clause" }
authors = [{ name = "Peter Murphy", email = "peterkmurphy@gmail.com" }]

[tool.maturin]
manifest-path = "../crates/ozzyyaml-python/Cargo.toml"
module-name = "ozzyyaml._native"
python-source = "."
```

- [ ] **Step 3: Façade stub**

```python
# python/ozzyyaml/__init__.py
from ozzyyaml._native import __version__

__all__ = ["__version__"]
```

- [ ] **Step 4: Verify**

```bash
cd python && python3 -m venv .venv && . .venv/bin/activate
pip install maturin pytest
maturin develop
python -c "import ozzyyaml; print(ozzyyaml.__version__)"
```

Expected: prints `0.1.0`

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/ozzyyaml-python python
git commit -m "chore: scaffold Maturin Python package and ozzyyaml-python crate"
```

---

### Task 3: Native `load` / `load_all` / `dump` / `dump_all`

**Files:**
- Modify: `crates/ozzyyaml-python/src/lib.rs` (split into modules if >300 lines: `convert.rs`, `lib.rs`)
- Modify: `python/ozzyyaml/__init__.py`
- Create: `python/tests/test_load_dump.py`

**Interfaces:**
- Produces Python API per design spec

- [ ] **Step 1: Failing pytest**

```python
import ozzyyaml

def test_load_scalar():
    assert ozzyyaml.load("42\n") == 42
    assert ozzyyaml.load("true\n") is True
    assert ozzyyaml.load("hello\n") == "hello"

def test_load_collection():
    assert ozzyyaml.load("[1, 2]\n") == [1, 2]
    assert ozzyyaml.load("{a: 1}\n") == {"a": 1}

def test_dump_roundtrip():
    data = {"k": [1, True, None]}
    text = ozzyyaml.dump(data)
    assert ozzyyaml.load(text) == data

def test_load_all():
    assert ozzyyaml.load_all("1\n---\n2\n") == [1, 2]

def test_cycle_errors():
    import pytest
    with pytest.raises(ozzyyaml.OzzyYamlError):
        ozzyyaml.load("&a [*a]\n")
```

- [ ] **Step 2: Implement conversion + bindings**

Rust:
- `value_to_py(py, &Value, visiting: &mut HashSet<*const Value>) -> PyResult<PyObject>` — on re-entering a pointer still in `visiting`, raise cycle error; for DAG sharing use a `HashMap<*const Value, PyObject>` cache so shared nodes become one Python object.
- Actually for **cycle → error**: if node is in `visiting` stack, error. If node was fully converted (cache hit), reuse object (DAG share).
- `py_to_value(obj) -> PyResult<Value>` for dump
- Wrap `from_str_with_limits` / `load_all_with_limits` / `to_string_with_options` / `dump_all_with_options`
- Exception: `OzzyYamlError` registered with PyO3

Python façade re-exports `load`, `load_all`, `dump`, `dump_all`, `OzzyYamlError`.

- [ ] **Step 3: pytest passes**

```bash
cd python && . .venv/bin/activate && maturin develop && pytest -q
```

- [ ] **Step 4: Commit**

```bash
git add crates/ozzyyaml-python python
git commit -m "feat(python): native load/dump API via PyO3"
```

---

### Task 4: `docs/python.md` + README Python section

**Files:**
- Create: `docs/python.md`
- Modify: `README.md`

- [ ] **Step 1: Write docs/python.md** — install via maturin/dev, API examples, limits kwargs, cycle behaviour (errors on `load`), note that PyPI upload is not yet done, link to architecture.

- [ ] **Step 2: README** — short “Python” section pointing at `docs/python.md` and showing `load`/`dump` snippet.

- [ ] **Step 3: Commit**

```bash
git add docs/python.md README.md
git commit -m "docs: add Python usage guide"
```

---

### Task 5: CI wheels + smoke test

**Files:**
- Modify: `.github/workflows/ci.yml` (or add `wheels.yml`)

- [ ] **Step 1: Add job** on `ubuntu-latest`:
  - checkout with submodules
  - setup Python 3.12 + Rust stable
  - `pip install maturin pytest`
  - `maturin build --release -m crates/ozzyyaml-python/Cargo.toml --out dist` (or from `python/`)
  - `pip install dist/*.whl`
  - `pytest python/tests` or inline smoke

Keep existing Rust job unchanged.

- [ ] **Step 2: Local verify**

```bash
cargo fmt --all && cargo clippy --workspace -- -D warnings
cargo test --workspace
# python tests as above
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: build and smoke-test Python wheels"
```

- [ ] **Step 4: STOP FOR HUMAN PUSH**

Announce: Phase A complete. Please push `main` (or the branch). Do not start phase B until confirmed.

---

### Task 6: Phase B — structured Value (AFTER PUSH GATE)

**Do not implement until the human confirms phase A is pushed and asks to continue.**

**Files:**
- Extend `ozzyyaml-python` + `python/ozzyyaml/`
- Create: `python/tests/test_value.py`

- [ ] Expose a `Value` Python type (or hierarchy) wrapping/converting while preserving identity for aliases and cycles
- [ ] `load_value` / `dump_value` (names may be refined)
- [ ] Tests: shared alias `is` identity; cycle round-trip
- [ ] Update `docs/python.md`
- [ ] Commit `feat(python): structured Value API with alias identity`
- [ ] STOP for human push again

---

## Spec coverage

| Spec item | Task |
| --- | --- |
| Docs rewrite | Task 1 |
| Maturin scaffold | Task 2 |
| Native API | Task 3 |
| python.md | Task 4 |
| CI wheels | Task 5 |
| Structured Value | Task 6 (gated) |
| No PyPI upload | respected |
