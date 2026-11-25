//! Structured [`YamlValue`] wrappers that preserve `Rc` identity.

use pyo3::exceptions::{PyIndexError, PyKeyError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ozzyyaml::Value;
use ozzyyaml_emit::{to_string_with_options, EmitOptions};

use crate::{map_err, parse_style};

/// Shared identity cache for one loaded document graph.
pub(crate) struct Graph {
    cache: RefCell<HashMap<*const Value, Py<YamlValue>>>,
}

impl Graph {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            cache: RefCell::new(HashMap::new()),
        })
    }
}

/// A YAML node that preserves alias identity (including cycles).
#[pyclass(name = "Value", module = "ozzyyaml", unsendable)]
pub struct YamlValue {
    inner: Rc<Value>,
    graph: Rc<Graph>,
}

impl YamlValue {
    pub(crate) fn wrap(py: Python<'_>, inner: Rc<Value>, graph: Rc<Graph>) -> PyResult<Py<Self>> {
        let ptr = Rc::as_ptr(&inner);
        if let Some(existing) = graph.cache.borrow().get(&ptr) {
            return Ok(existing.clone_ref(py));
        }
        let obj = Py::new(
            py,
            Self {
                inner: Rc::clone(&inner),
                graph: Rc::clone(&graph),
            },
        )?;
        graph.cache.borrow_mut().insert(ptr, obj.clone_ref(py));
        Ok(obj)
    }

    pub(crate) fn wrap_owned(py: Python<'_>, value: Value) -> PyResult<Py<Self>> {
        let graph = Graph::new();
        Self::wrap(py, Rc::new(value), graph)
    }
}

#[pymethods]
impl YamlValue {
    /// Node kind: ``null``, ``bool``, ``int``, ``float``, ``str``, ``seq``, or ``map``.
    #[getter]
    fn kind(&self) -> &'static str {
        match &*self.inner {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "str",
            Value::Sequence(_) => "seq",
            Value::Mapping(_) => "map",
        }
    }

    fn is_null(&self) -> bool {
        matches!(&*self.inner, Value::Null)
    }

    fn as_bool(&self) -> Option<bool> {
        self.inner.as_bool()
    }

    fn as_int(&self) -> Option<i64> {
        self.inner.as_i64()
    }

    fn as_float(&self) -> Option<f64> {
        self.inner.as_f64()
    }

    fn as_str(&self) -> Option<String> {
        self.inner.as_str().map(str::to_owned)
    }

    fn __len__(&self) -> PyResult<usize> {
        match &*self.inner {
            Value::Sequence(items) => Ok(items.borrow().len()),
            Value::Mapping(entries) => Ok(entries.borrow().len()),
            _ => Err(PyTypeError::new_err("value is not a sequence or mapping")),
        }
    }

    fn __getitem__(&self, py: Python<'_>, key: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        match &*self.inner {
            Value::Sequence(items) => {
                let idx: isize = key.extract()?;
                let items = items.borrow();
                let len = items.len() as isize;
                let i = if idx < 0 { len + idx } else { idx };
                if i < 0 || i >= len {
                    return Err(PyIndexError::new_err("sequence index out of range"));
                }
                let child = Rc::clone(&items[i as usize]);
                Ok(Self::wrap(py, child, Rc::clone(&self.graph))?.into_any())
            }
            Value::Mapping(entries) => {
                if let Ok(idx) = key.extract::<isize>() {
                    let entries = entries.borrow();
                    let len = entries.len() as isize;
                    let i = if idx < 0 { len + idx } else { idx };
                    if i < 0 || i >= len {
                        return Err(PyIndexError::new_err("mapping index out of range"));
                    }
                    let (k, v) = &entries[i as usize];
                    let pair = PyList::empty(py);
                    pair.append(Self::wrap(py, Rc::clone(k), Rc::clone(&self.graph))?)?;
                    pair.append(Self::wrap(py, Rc::clone(v), Rc::clone(&self.graph))?)?;
                    return Ok(pair.into_any().unbind());
                }
                // Lookup by scalar key equality for convenience.
                let want = key_to_value(&key)?;
                for (k, v) in entries.borrow().iter() {
                    if values_equal(k.as_ref(), &want) {
                        return Ok(Self::wrap(py, Rc::clone(v), Rc::clone(&self.graph))?.into_any());
                    }
                }
                Err(PyKeyError::new_err("key not found"))
            }
            _ => Err(PyTypeError::new_err("value is not a sequence or mapping")),
        }
    }

    fn __repr__(&self) -> String {
        match &*self.inner {
            Value::Null => "Value(null)".into(),
            Value::Bool(b) => format!("Value({b})"),
            Value::Int(i) => format!("Value({i})"),
            Value::Float(f) => format!("Value({f})"),
            Value::String(s) => format!("Value({s:?})"),
            Value::Sequence(items) => format!("Value(seq, len={})", items.borrow().len()),
            Value::Mapping(entries) => format!("Value(map, len={})", entries.borrow().len()),
        }
    }

    /// True when both objects wrap the same underlying node (alias identity).
    fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

fn key_to_value(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Value::Bool(b));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::Int(i));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(Value::Float(f));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(Value::String(s));
    }
    Err(PyTypeError::new_err(
        "mapping key lookup supports null/bool/int/float/str only",
    ))
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y || (x.is_nan() && y.is_nan()),
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

#[pyfunction]
#[pyo3(signature = (stream, *, max_anchors=None, max_nesting_depth=None, max_input_bytes=None))]
pub(crate) fn load_value(
    py: Python<'_>,
    stream: Bound<'_, PyAny>,
    max_anchors: Option<usize>,
    max_nesting_depth: Option<usize>,
    max_input_bytes: Option<usize>,
) -> PyResult<Py<YamlValue>> {
    let input = crate::input_from_py(stream)?;
    let limits = crate::limits_from_kwargs(max_anchors, max_nesting_depth, max_input_bytes);
    let value = crate::load_one(input, limits)?;
    YamlValue::wrap_owned(py, value)
}

#[pyfunction]
#[pyo3(signature = (stream, *, max_anchors=None, max_nesting_depth=None, max_input_bytes=None))]
pub(crate) fn load_all_value(
    py: Python<'_>,
    stream: Bound<'_, PyAny>,
    max_anchors: Option<usize>,
    max_nesting_depth: Option<usize>,
    max_input_bytes: Option<usize>,
) -> PyResult<Py<PyAny>> {
    let input = crate::input_from_py(stream)?;
    let limits = crate::limits_from_kwargs(max_anchors, max_nesting_depth, max_input_bytes);
    let docs = crate::load_docs(input, limits)?;
    let list = PyList::empty(py);
    for doc in docs {
        // Each document gets its own identity graph.
        list.append(YamlValue::wrap_owned(py, doc)?)?;
    }
    Ok(list.into_any().unbind())
}

#[pyfunction]
#[pyo3(signature = (value, *, style="flow"))]
pub(crate) fn dump_value(value: PyRef<'_, YamlValue>, style: &str) -> PyResult<String> {
    let opts = EmitOptions {
        style: parse_style(style)?,
    };
    to_string_with_options(value.inner.as_ref(), &opts).map_err(map_err)
}
