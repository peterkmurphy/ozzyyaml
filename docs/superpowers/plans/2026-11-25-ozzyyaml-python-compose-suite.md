# Python + compose suite 255/255 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hard-gate Rust compose and Python `load_all` / `load_all_value` at 100% of valid yaml-test-suite cases (255/255), by allowing anchor redefinition and fixing native Python cycles + collection keys.

**Architecture:** Fix `Composer::register_anchor` to overwrite on redefinition (YAML 1.2.2). Retire the compose allowlist in favour of the same valid-ID enumeration as the event gate. In PyO3 `value_to_py`, cache empty containers before filling (cycles) and coerce collection mapping keys to hashable tuples. Add a pytest suite gate mirroring Rust.

**Tech Stack:** Rust 2021 (`ozzyyaml`, `ozzyyaml-python` / PyO3), Maturin, pytest, vendor `yaml-test-suite`.

## Global Constraints

- Valid suite ID = top-level dir with `in.yaml` + `test.event` and no `error` file (same as event gate).
- Compose success only — no `test.event` equality from Python, no `in.json` tree compare.
- Leave `suite_shape_allowlist.txt` / shape harness unchanged.
- Do not expand emit presentation or round-trip data claims beyond relocating the round-trip ID list.
- Native collection-key coercion: seq → `tuple`, map → `tuple` of `(k,v)` pairs; values not coerced; `load_value` unchanged.
- Remove `Error::DuplicateAnchor` entirely once redefinition lands.

---

## File map

| File | Role |
| --- | --- |
| `crates/ozzyyaml/src/compose.rs` | Overwrite anchors in `register_anchor` |
| `crates/ozzyyaml/src/error.rs` | Remove `DuplicateAnchor` |
| `crates/ozzyyaml/tests/compose.rs` | Redefinition + suite probe unit tests |
| `crates/ozzyyaml/tests/yaml_test_suite_compose.rs` | Full valid-suite hard gate |
| `crates/ozzyyaml/tests/suite_allowlist.txt` | Retire; move contents for round-trip |
| `crates/ozzyyaml/tests/suite_roundtrip_allowlist.txt` | New home for round-trip ID list |
| `crates/ozzyyaml/tests/roundtrip.rs` | Point at round-trip allowlist file |
| `crates/ozzyyaml-python/src/lib.rs` | Cycle placeholders + key coercion + `py_to_value` tuples |
| `python/tests/test_load_dump.py` | Cycle + complex-key unit tests |
| `python/tests/test_yaml_test_suite_compose.py` | Full-suite Python gate |
| `docs/conformance.md`, `docs/python.md` | 255/255 compose + cycle/key docs |

---

### Task 1: Anchor redefinition + Rust compose 255/255 gate

**Files:**
- Modify: `crates/ozzyyaml/src/compose.rs` (`register_anchor`)
- Modify: `crates/ozzyyaml/src/error.rs` (remove `DuplicateAnchor`)
- Modify: `crates/ozzyyaml/tests/compose.rs`
- Modify: `crates/ozzyyaml/tests/yaml_test_suite_compose.rs`
- Create: `crates/ozzyyaml/tests/suite_roundtrip_allowlist.txt` (copy of current allowlist body)
- Modify: `crates/ozzyyaml/tests/roundtrip.rs` (read the new filename)
- Delete: `crates/ozzyyaml/tests/suite_allowlist.txt`

**Interfaces:**
- Consumes: `ozzyyaml::load_all`, `ozzyyaml::from_str`, `Value`
- Produces: compose hard gate `all_valid_cases_compose_ok`; redefinition semantics; no `DuplicateAnchor`

- [ ] **Step 1: Write failing redefinition tests (replace duplicate-anchor tests)**

In `crates/ozzyyaml/tests/compose.rs`, **delete** `duplicate_anchor` and `duplicate_anchor_has_mark`. Add:

```rust
#[test]
fn redefined_anchor_later_alias_sees_new_value() {
    // 3GZX-shaped: first alias sees Foo; after redefine, alias sees Bar.
    let v = from_str(
        "First occurrence: &anchor Foo\n\
         Second occurrence: *anchor\n\
         Override anchor: &anchor Bar\n\
         Reuse anchor: *anchor\n",
    )
    .unwrap();
    let Value::Mapping(entries) = v else {
        panic!("expected mapping");
    };
    let entries = entries.borrow();
    let get = |key: &str| -> Value {
        entries
            .iter()
            .find(|(k, _)| matches!(k.as_ref(), Value::String(s) if s == key))
            .map(|(_, val)| val.as_ref().clone())
            .unwrap_or_else(|| panic!("missing {key}"))
    };
    assert_eq!(get("First occurrence"), Value::String("Foo".into()));
    assert_eq!(get("Second occurrence"), Value::String("Foo".into()));
    assert_eq!(get("Override anchor"), Value::String("Bar".into()));
    assert_eq!(get("Reuse anchor"), Value::String("Bar".into()));
}

#[test]
fn suite_3gzx_and_pw8x_compose_ok() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/yaml-test-suite");
    for id in ["3GZX", "PW8X"] {
        let yaml = std::fs::read_to_string(root.join(id).join("in.yaml")).unwrap();
        load_all(&yaml).unwrap_or_else(|e| panic!("{id}: {e}"));
    }
}
```

Add `use std::path::PathBuf;` and `use ozzyyaml::load_all` if not already imported in that file.

- [ ] **Step 2: Run tests — expect fail on DuplicateAnchor**

Run: `cargo test -p ozzyyaml --test compose redefined_anchor_later_alias_sees_new_value suite_3gzx_and_pw8x_compose_ok`

Expected: FAIL (`duplicate anchor` / `DuplicateAnchor`).

- [ ] **Step 3: Implement overwrite in `register_anchor`**

Replace the body in `crates/ozzyyaml/src/compose.rs`:

```rust
fn register_anchor(&mut self, name: String, node: Rc<Value>, mark: Mark) -> Result<()> {
    if !self.anchors.contains_key(&name) && self.anchors.len() >= self.limits.max_anchors {
        return Err(Error::LimitExceeded {
            kind: LimitKind::Anchors,
            limit: self.limits.max_anchors,
            mark,
        });
    }
    self.anchors.insert(name, node);
    Ok(())
}
```

Remove the `DuplicateAnchor` variant from `crates/ozzyyaml/src/error.rs` (and any doc comments that only describe it). Fix compile errors in tests/docs that match on it (compose tests already replaced).

- [ ] **Step 4: Re-run unit tests — expect pass**

Run: `cargo test -p ozzyyaml --test compose`

Expected: PASS (including redefinition + 3GZX/PW8X).

- [ ] **Step 5: Relocate round-trip allowlist**

```bash
cp crates/ozzyyaml/tests/suite_allowlist.txt \
   crates/ozzyyaml/tests/suite_roundtrip_allowlist.txt
```

Edit the header comment of `suite_roundtrip_allowlist.txt` to say it is the **round-trip / acyclic emit** allowlist, not the compose gate.

In `crates/ozzyyaml/tests/roundtrip.rs`, change the path string from `tests/suite_allowlist.txt` to `tests/suite_roundtrip_allowlist.txt`.

- [ ] **Step 6: Rewrite compose suite harness as full hard gate**

Replace `crates/ozzyyaml/tests/yaml_test_suite_compose.rs` with:

```rust
//! Every valid yaml-test-suite case (in.yaml + test.event, no error marker)
//! must compose via `load_all`. Value trees are not compared.

use ozzyyaml::load_all;
use std::path::{Path, PathBuf};

fn suite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/yaml-test-suite")
}

fn valid_compose_suite_ids(root: &Path) -> Vec<String> {
    let mut ids: Vec<_> = std::fs::read_dir(root)
        .expect("suite root")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|id| {
            let d = root.join(id);
            d.join("in.yaml").is_file()
                && d.join("test.event").is_file()
                && !d.join("error").exists()
        })
        .collect();
    ids.sort();
    ids
}

#[test]
fn all_valid_cases_compose_ok() {
    let root = suite_root();
    assert!(
        root.join("229Q").is_dir(),
        "yaml-test-suite submodule missing; run `git submodule update --init`"
    );

    let ids = valid_compose_suite_ids(&root);
    assert!(!ids.is_empty(), "no valid suite cases found");

    let mut failures = Vec::new();
    for id in &ids {
        let input = std::fs::read_to_string(root.join(id).join("in.yaml"))
            .unwrap_or_else(|e| panic!("{id}: cannot read in.yaml: {e}"));
        if let Err(e) = load_all(&input) {
            failures.push(format!("{id}: {e}"));
        }
    }

    assert!(
        failures.is_empty(),
        "compose failures ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
```

Delete `crates/ozzyyaml/tests/suite_allowlist.txt`.

- [ ] **Step 7: Run compose + roundtrip gates**

Run:

```bash
cargo test -p ozzyyaml --test yaml_test_suite_compose
cargo test -p ozzyyaml --test roundtrip
cargo test -p ozzyyaml --test compose
```

Expected: all PASS; compose gate covers 255 IDs (failure message would list misses if any).

- [ ] **Step 8: Commit**

```bash
git add crates/ozzyyaml/src/compose.rs crates/ozzyyaml/src/error.rs \
  crates/ozzyyaml/tests/compose.rs \
  crates/ozzyyaml/tests/yaml_test_suite_compose.rs \
  crates/ozzyyaml/tests/suite_roundtrip_allowlist.txt \
  crates/ozzyyaml/tests/roundtrip.rs
git add -u crates/ozzyyaml/tests/suite_allowlist.txt
git commit -m "$(cat <<'EOF'
feat(compose): allow anchor redefinition; hard-gate valid suite

EOF
)"
```

---

### Task 2: Native Python cycles + collection-key coercion

**Files:**
- Modify: `crates/ozzyyaml-python/src/lib.rs` (`value_to_py`, `py_to_value`)
- Modify: `python/tests/test_load_dump.py`

**Interfaces:**
- Consumes: Rust `Value` graph from Task 1 compose
- Produces: native `load` / `load_all` that handle cycles and collection keys

- [ ] **Step 1: Rewrite failing/expected Python unit tests**

In `python/tests/test_load_dump.py`, **replace** `test_cycle_errors` with:

```python
def test_cycle_loads_natively():
    data = ozzyyaml.load("&a [*a]\n")
    assert isinstance(data, list)
    assert len(data) == 1
    assert data[0] is data


def test_collection_mapping_key_becomes_tuple():
    data = ozzyyaml.load("? [a, b]\n: c\n")
    assert data == {("a", "b"): "c"}
```

Keep `test_shared_alias_is_same_object` as-is.

- [ ] **Step 2: Run tests — expect fail**

From `python/` (venv with `maturin develop` already done, or refresh):

```bash
cd python && pytest tests/test_load_dump.py::test_cycle_loads_natively \
  tests/test_load_dump.py::test_collection_mapping_key_becomes_tuple -v
```

Expected: FAIL (`OzzyYamlError` cyclic / `TypeError` unhashable).

- [ ] **Step 3: Implement `value_to_py` placeholders + key coercion**

In `crates/ozzyyaml-python/src/lib.rs`, replace `value_to_py` (drop the `stack` parameter) with:

```rust
fn value_to_py(
    py: Python<'_>,
    value: &Value,
    cache: &mut HashMap<*const Value, PyObject>,
) -> PyResult<PyObject> {
    let ptr = value as *const Value;
    if let Some(existing) = cache.get(&ptr) {
        return Ok(existing.clone_ref(py));
    }

    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(PyBool::new(py, *b).as_any().clone().unbind()),
        Value::Int(i) => Ok((*i)
            .into_pyobject(py)
            .expect("i64 to Python is infallible")
            .into_any()
            .unbind()),
        Value::Float(f) => Ok(PyFloat::new(py, *f).into_any().unbind()),
        Value::String(s) => Ok(PyString::new(py, s).into_any().unbind()),
        Value::Sequence(items) => {
            let list = PyList::empty(py);
            cache.insert(ptr, list.clone().into_any().unbind());
            for child in items.borrow().iter() {
                let obj = value_to_py(py, child.as_ref(), cache)?;
                list.append(obj)?;
            }
            Ok(list.into_any().unbind())
        }
        Value::Mapping(entries) => {
            let dict = PyDict::new(py);
            cache.insert(ptr, dict.clone().into_any().unbind());
            for (k, v) in entries.borrow().iter() {
                let key = value_to_py(py, k.as_ref(), cache)?;
                let key = mapping_key_to_hashable(py, key)?;
                let val = value_to_py(py, v.as_ref(), cache)?;
                dict.set_item(key, val)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}

fn mapping_key_to_hashable(py: Python<'_>, key: PyObject) -> PyResult<PyObject> {
    Ok(hashable_key_item(py, key.bind(py))?.unbind())
}

fn hashable_key_item<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    use pyo3::types::PyTuple;
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut items = Vec::with_capacity(list.len());
        for item in list.iter() {
            items.push(hashable_key_item(py, &item)?);
        }
        return Ok(PyTuple::new(py, items)?.into_any());
    }
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut pairs = Vec::with_capacity(dict.len());
        for (k, v) in dict.iter() {
            let tk = hashable_key_item(py, &k)?;
            let tv = hashable_key_item(py, &v)?;
            pairs.push(PyTuple::new(py, [tk, tv])?.into_any());
        }
        return Ok(PyTuple::new(py, pairs)?.into_any());
    }
    Ok(obj.clone())
}
```

Update `load` / `load_all` call sites to drop the `stack: HashSet` argument. Remove unused `HashSet` import if nothing else needs it.

In `py_to_value`, accept `tuple` as a sequence (so dump of tuple keys / values from loaded data works):

```rust
if let Ok(tup) = obj.downcast::<pyo3::types::PyTuple>() {
    let mut items = Vec::with_capacity(tup.len());
    for item in tup.iter() {
        items.push(Rc::new(py_to_value(&item)?));
    }
    return Ok(Value::Sequence(std::cell::RefCell::new(items)));
}
```

Place the tuple branch **before** the “unsupported type” error, after `PyList` (or instead of requiring list-only).

- [ ] **Step 4: Rebuild extension and re-run unit tests**

```bash
cd python && maturin develop && pytest tests/test_load_dump.py -v
```

Expected: PASS (including cycle + collection key).

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml-python/src/lib.rs python/tests/test_load_dump.py
git commit -m "$(cat <<'EOF'
feat(python): native load cycles and hashable collection keys

EOF
)"
```

---

### Task 3: Python full-suite compose pytest

**Files:**
- Create: `python/tests/test_yaml_test_suite_compose.py`

**Interfaces:**
- Consumes: `ozzyyaml.load_all`, `ozzyyaml.load_all_value` from Task 2
- Produces: hard gate over all valid suite IDs

- [ ] **Step 1: Add suite gate test file**

Create `python/tests/test_yaml_test_suite_compose.py`:

```python
"""Every valid yaml-test-suite case must load via load_all and load_all_value."""

from pathlib import Path

import ozzyyaml

SUITE_ROOT = Path(__file__).resolve().parents[2] / "vendor" / "yaml-test-suite"


def valid_suite_ids(root: Path) -> list[str]:
    ids = []
    for d in root.iterdir():
        if not d.is_dir():
            continue
        if (d / "in.yaml").is_file() and (d / "test.event").is_file() and not (d / "error").exists():
            ids.append(d.name)
    ids.sort()
    return ids


def test_all_valid_cases_load_all_and_load_all_value():
    assert (SUITE_ROOT / "229Q").is_dir(), (
        "yaml-test-suite submodule missing; run `git submodule update --init`"
    )
    ids = valid_suite_ids(SUITE_ROOT)
    assert ids, "no valid suite cases found"

    failures: list[str] = []
    for id in ids:
        text = (SUITE_ROOT / id / "in.yaml").read_text(encoding="utf-8")
        try:
            ozzyyaml.load_all(text)
        except Exception as e:  # noqa: BLE001 — collect all suite failures
            failures.append(f"{id}: load_all: {e}")
            continue
        try:
            ozzyyaml.load_all_value(text)
        except Exception as e:  # noqa: BLE001
            failures.append(f"{id}: load_all_value: {e}")

    assert not failures, f"{len(failures)} failures:\n" + "\n".join(failures)
```

- [ ] **Step 2: Run the suite gate**

```bash
cd python && pytest tests/test_yaml_test_suite_compose.py -v
```

Expected: PASS (255 cases). If failures remain, fix in Task 1/2 code — do not skip IDs.

- [ ] **Step 3: Commit**

```bash
git add python/tests/test_yaml_test_suite_compose.py
git commit -m "$(cat <<'EOF'
test(python): hard-gate valid yaml-test-suite compose loads

EOF
)"
```

---

### Task 4: Docs

**Files:**
- Modify: `docs/conformance.md`
- Modify: `docs/python.md`

**Interfaces:**
- Consumes: green gates from Tasks 1–3
- Produces: accurate 255/255 compose claims

- [ ] **Step 1: Update `docs/conformance.md`**

In the honest caveats / coverage section, add or adjust so that:

- Event suite remains 255/255 `test.event`.
- **Compose:** 100% of the same valid cases succeed via Rust `load_all` and Python `load_all` / `load_all_value`, gated by `yaml_test_suite_compose` and `python/tests/test_yaml_test_suite_compose.py`.
- Shape and round-trip remain on separate allowlists (`suite_shape_allowlist.txt`, `suite_roundtrip_allowlist.txt`).
- Emit still presentation-non-preserving; Core-only unchanged.

Link the design/plan:

- `docs/superpowers/specs/2026-11-25-ozzyyaml-python-compose-suite-design.md`
- `docs/superpowers/plans/2026-11-25-ozzyyaml-python-compose-suite.md`

- [ ] **Step 2: Update `docs/python.md`**

- State native `load` preserves alias identity including **cycles**.
- Document collection mapping keys → nested tuples for `dict` hashability; prefer `load_value` when the YAML graph must be preserved without coercion.
- Point at compose 255/255 claim in `conformance.md`.
- Remove any wording that native load cannot represent cycles.

- [ ] **Step 3: Spot-check docs render mentally; commit**

```bash
git add docs/conformance.md docs/python.md
git commit -m "$(cat <<'EOF'
docs: claim compose 255/255 for Rust and Python loaders

EOF
)"
```

---

## Self-review (plan vs spec)

| Spec requirement | Task |
| --- | --- |
| Anchor redefinition / remove `DuplicateAnchor` | 1 |
| Rust compose hard gate; delete compose allowlist | 1 |
| Preserve round-trip allowlist elsewhere | 1 (`suite_roundtrip_allowlist.txt`) |
| Native cycles via placeholders | 2 |
| Collection keys → tuples | 2 |
| Python gate `load_all` + `load_all_value` | 3 |
| Docs | 4 |
| No event API / no `in.json` / no shape expansion | respected |

No TBD placeholders. Task 2 implements `hashable_key_item` exactly as shown.
