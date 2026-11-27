//! Iterative `Value` ↔ Python conversions (no depth-proportional Rust recursion).

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyList, PyString, PyTuple};
use std::collections::HashMap;
use std::rc::Rc;

use ozzyyaml::Value;

enum MapPhase {
    NeedKey,
    KeyReady,
    ValReady { key: Py<PyAny> },
}

enum ToPyFrame {
    Seq {
        list: Py<PyList>,
        items: Vec<Rc<Value>>,
        idx: usize,
    },
    Map {
        dict: Py<PyDict>,
        entries: Vec<(Rc<Value>, Rc<Value>)>,
        idx: usize,
        phase: MapPhase,
    },
}

/// Convert a [`Value`] graph to native Python objects.
///
/// Sequences/mappings are inserted into `cache` as empty containers before
/// children are filled so YAML aliases/cycles share identity.
pub(crate) fn value_to_py(
    py: Python<'_>,
    value: &Value,
    cache: &mut HashMap<*const Value, Py<PyAny>>,
) -> PyResult<Py<PyAny>> {
    let mut stack = Vec::new();
    let root = start_value(py, value, cache, &mut stack)?;
    while let Some(frame) = stack.pop() {
        match frame {
            ToPyFrame::Seq { list, items, idx } => {
                if idx >= items.len() {
                    continue;
                }
                let child = Rc::clone(&items[idx]);
                stack.push(ToPyFrame::Seq {
                    list: list.clone_ref(py),
                    items,
                    idx: idx + 1,
                });
                let obj = start_value(py, child.as_ref(), cache, &mut stack)?;
                list.bind(py).append(obj)?;
            }
            ToPyFrame::Map {
                dict,
                entries,
                idx,
                phase,
            } => match phase {
                MapPhase::NeedKey => {
                    if idx >= entries.len() {
                        continue;
                    }
                    let key_val = Rc::clone(&entries[idx].0);
                    stack.push(ToPyFrame::Map {
                        dict: dict.clone_ref(py),
                        entries,
                        idx,
                        phase: MapPhase::KeyReady,
                    });
                    start_value(py, key_val.as_ref(), cache, &mut stack)?;
                }
                MapPhase::KeyReady => {
                    let key_ptr = entries[idx].0.as_ref() as *const Value;
                    let raw = cache
                        .get(&key_ptr)
                        .expect("key must be cached after KeyReady")
                        .clone_ref(py);
                    let key = mapping_key_to_hashable(py, raw)?;
                    let val_node = Rc::clone(&entries[idx].1);
                    stack.push(ToPyFrame::Map {
                        dict: dict.clone_ref(py),
                        entries,
                        idx,
                        phase: MapPhase::ValReady { key },
                    });
                    start_value(py, val_node.as_ref(), cache, &mut stack)?;
                }
                MapPhase::ValReady { key } => {
                    let val_ptr = entries[idx].1.as_ref() as *const Value;
                    let val = cache
                        .get(&val_ptr)
                        .expect("value must be cached after ValReady")
                        .clone_ref(py);
                    dict.bind(py).set_item(key, val)?;
                    stack.push(ToPyFrame::Map {
                        dict,
                        entries,
                        idx: idx + 1,
                        phase: MapPhase::NeedKey,
                    });
                }
            },
        }
    }
    Ok(root)
}

fn start_value(
    py: Python<'_>,
    value: &Value,
    cache: &mut HashMap<*const Value, Py<PyAny>>,
    stack: &mut Vec<ToPyFrame>,
) -> PyResult<Py<PyAny>> {
    let ptr = value as *const Value;
    if let Some(existing) = cache.get(&ptr) {
        return Ok(existing.clone_ref(py));
    }

    match value {
        Value::Null => {
            let obj = py.None();
            cache.insert(ptr, obj.clone_ref(py));
            Ok(obj)
        }
        Value::Bool(b) => {
            let obj = PyBool::new(py, *b).as_any().clone().unbind();
            cache.insert(ptr, obj.clone_ref(py));
            Ok(obj)
        }
        Value::Int(i) => {
            let obj = (*i)
                .into_pyobject(py)
                .expect("i64 to Python is infallible")
                .into_any()
                .unbind();
            cache.insert(ptr, obj.clone_ref(py));
            Ok(obj)
        }
        Value::Float(f) => {
            let obj = PyFloat::new(py, *f).into_any().unbind();
            cache.insert(ptr, obj.clone_ref(py));
            Ok(obj)
        }
        Value::String(s) => {
            let obj = PyString::new(py, s).into_any().unbind();
            cache.insert(ptr, obj.clone_ref(py));
            Ok(obj)
        }
        Value::Sequence(items) => {
            let list = PyList::empty(py);
            let handle = list.clone().into_any().unbind();
            cache.insert(ptr, handle.clone_ref(py));
            let items = items.borrow().clone();
            stack.push(ToPyFrame::Seq {
                list: list.unbind(),
                items,
                idx: 0,
            });
            Ok(handle)
        }
        Value::Mapping(entries) => {
            let dict = PyDict::new(py);
            let handle = dict.clone().into_any().unbind();
            cache.insert(ptr, handle.clone_ref(py));
            let entries = entries.borrow().clone();
            stack.push(ToPyFrame::Map {
                dict: dict.unbind(),
                entries,
                idx: 0,
                phase: MapPhase::NeedKey,
            });
            Ok(handle)
        }
    }
}

fn mapping_key_to_hashable(py: Python<'_>, key: Py<PyAny>) -> PyResult<Py<PyAny>> {
    Ok(hashable_key_item(py, key.bind(py))?.unbind())
}

enum HashJob {
    /// Convert this object; push result onto `results`.
    Convert(Py<PyAny>),
    /// Build a tuple from the last `n` results (list → tuple).
    FinishList { n: usize },
    /// Build a tuple of pairs from the last `n` pair-tuples (dict → tuple).
    FinishDict { n: usize },
    /// Wrap the last two results as a `(key, value)` pair tuple.
    FinishPair,
}

/// Coerce list/dict mapping keys to nested tuples (hashable).
pub(crate) fn hashable_key_item<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut jobs = vec![HashJob::Convert(obj.clone().unbind())];
    let mut results: Vec<Py<PyAny>> = Vec::new();

    while let Some(job) = jobs.pop() {
        match job {
            HashJob::Convert(obj) => {
                let bound = obj.bind(py);
                if let Ok(list) = bound.cast::<PyList>() {
                    let n = list.len();
                    jobs.push(HashJob::FinishList { n });
                    for item in list.iter().rev() {
                        jobs.push(HashJob::Convert(item.unbind()));
                    }
                } else if let Ok(dict) = bound.cast::<PyDict>() {
                    let n = dict.len();
                    jobs.push(HashJob::FinishDict { n });
                    let mut pairs: Vec<(Py<PyAny>, Py<PyAny>)> =
                        dict.iter().map(|(k, v)| (k.unbind(), v.unbind())).collect();
                    while let Some((k, v)) = pairs.pop() {
                        jobs.push(HashJob::FinishPair);
                        jobs.push(HashJob::Convert(v));
                        jobs.push(HashJob::Convert(k));
                    }
                } else {
                    results.push(obj);
                }
            }
            HashJob::FinishList { n } => {
                let start = results.len() - n;
                let items: Vec<Py<PyAny>> = results.drain(start..).collect();
                results.push(PyTuple::new(py, items)?.into_any().unbind());
            }
            HashJob::FinishDict { n } => {
                let start = results.len() - n;
                let pairs: Vec<Py<PyAny>> = results.drain(start..).collect();
                results.push(PyTuple::new(py, pairs)?.into_any().unbind());
            }
            HashJob::FinishPair => {
                let v = results.pop().expect("pair value");
                let k = results.pop().expect("pair key");
                results.push(PyTuple::new(py, [k, v])?.into_any().unbind());
            }
        }
    }

    Ok(results
        .pop()
        .expect("hashable produced a value")
        .into_bound(py))
}

enum FromPyJob {
    Convert(Py<PyAny>),
    FinishSeq {
        n: usize,
    },
    /// Assemble `n` key/value pairs from `2*n` trailing results (`k0,v0,k1,v1,…`).
    FinishMap {
        n: usize,
    },
}

/// Convert a native Python object to a [`Value`] graph.
pub(crate) fn py_to_value(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    let py = obj.py();
    let mut jobs = vec![FromPyJob::Convert(obj.clone().unbind())];
    let mut results: Vec<Value> = Vec::new();

    while let Some(job) = jobs.pop() {
        match job {
            FromPyJob::Convert(obj) => {
                let bound = obj.bind(py);
                if bound.is_none() {
                    results.push(Value::Null);
                } else if let Ok(b) = bound.extract::<bool>() {
                    results.push(Value::Bool(b));
                } else if let Ok(i) = bound.extract::<i64>() {
                    results.push(Value::Int(i));
                } else if let Ok(f) = bound.extract::<f64>() {
                    results.push(Value::Float(f));
                } else if let Ok(s) = bound.extract::<String>() {
                    results.push(Value::String(s));
                } else if let Ok(list) = bound.cast::<PyList>() {
                    let n = list.len();
                    jobs.push(FromPyJob::FinishSeq { n });
                    for item in list.iter().rev() {
                        jobs.push(FromPyJob::Convert(item.unbind()));
                    }
                } else if let Ok(tup) = bound.cast::<PyTuple>() {
                    let n = tup.len();
                    jobs.push(FromPyJob::FinishSeq { n });
                    for item in tup.iter().rev() {
                        jobs.push(FromPyJob::Convert(item.unbind()));
                    }
                } else if let Ok(dict) = bound.cast::<PyDict>() {
                    let pairs: Vec<(Py<PyAny>, Py<PyAny>)> =
                        dict.iter().map(|(k, v)| (k.unbind(), v.unbind())).collect();
                    let n = pairs.len();
                    jobs.push(FromPyJob::FinishMap { n });
                    for (k, v) in pairs.into_iter().rev() {
                        jobs.push(FromPyJob::Convert(v));
                        jobs.push(FromPyJob::Convert(k));
                    }
                } else {
                    return Err(PyTypeError::new_err(format!(
                        "cannot dump Python type {}",
                        bound.get_type().name()?
                    )));
                }
            }
            FromPyJob::FinishSeq { n } => {
                let start = results.len() - n;
                let items: Vec<Rc<Value>> = results.drain(start..).map(Rc::new).collect();
                results.push(Value::Sequence(std::cell::RefCell::new(items)));
            }
            FromPyJob::FinishMap { n } => {
                let start = results.len() - (2 * n);
                let flat: Vec<Value> = results.drain(start..).collect();
                let mut entries = Vec::with_capacity(n);
                let mut iter = flat.into_iter();
                while let Some(k) = iter.next() {
                    let v = iter.next().expect("map value paired with key");
                    entries.push((Rc::new(k), Rc::new(v)));
                }
                results.push(Value::Mapping(std::cell::RefCell::new(entries)));
            }
        }
    }

    Ok(results.pop().expect("py_to_value produced a value"))
}
