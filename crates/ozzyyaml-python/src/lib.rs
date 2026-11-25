//! PyO3 extension module for ozzyyaml (`ozzyyaml._native`).

mod graph;

use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyList, PyString, PyTuple};
use std::collections::HashMap;
use std::rc::Rc;

use ozzyyaml::Value;
use ozzyyaml_emit::{dump_all_with_options, to_string_with_options, EmitOptions, EmitStyle};

create_exception!(ozzyyaml, OzzyYamlError, PyException);

pub(crate) fn limits_from_kwargs(
    max_anchors: Option<usize>,
    max_nesting_depth: Option<usize>,
    max_input_bytes: Option<usize>,
) -> ozzyyaml::Limits {
    let mut limits = ozzyyaml::Limits::default();
    if let Some(n) = max_anchors {
        limits = limits.max_anchors(n);
    }
    if let Some(n) = max_nesting_depth {
        limits = limits.max_nesting_depth(n);
    }
    if let Some(n) = max_input_bytes {
        limits = limits.max_input_bytes(n);
    }
    limits
}

pub(crate) fn map_err(err: impl std::fmt::Display) -> PyErr {
    OzzyYamlError::new_err(err.to_string())
}

pub(crate) fn parse_style(style: &str) -> PyResult<EmitStyle> {
    match style {
        "flow" => Ok(EmitStyle::Flow),
        "block" => Ok(EmitStyle::Block),
        other => Err(PyValueError::new_err(format!(
            "style must be 'flow' or 'block', got {other:?}"
        ))),
    }
}

/// Python `str` or `bytes` input for loaders.
pub(crate) enum StreamInput {
    /// Unicode text (UTF-8 string APIs).
    Text(String),
    /// Encoded bytes (BOM UTF-8 / UTF-16 / UTF-32 via `from_slice*`).
    Bytes(Vec<u8>),
}

pub(crate) fn input_from_py(stream: Bound<'_, PyAny>) -> PyResult<StreamInput> {
    if let Ok(s) = stream.extract::<String>() {
        return Ok(StreamInput::Text(s));
    }
    if let Ok(bytes) = stream.extract::<Vec<u8>>() {
        return Ok(StreamInput::Bytes(bytes));
    }
    Err(PyTypeError::new_err("stream must be str or bytes"))
}

pub(crate) fn load_one(
    input: StreamInput,
    limits: ozzyyaml::Limits,
) -> Result<ozzyyaml::Value, PyErr> {
    match input {
        StreamInput::Text(s) => ozzyyaml::from_str_with_limits(&s, limits).map_err(map_err),
        StreamInput::Bytes(b) => ozzyyaml::from_slice_with_limits(&b, limits).map_err(map_err),
    }
}

pub(crate) fn load_docs(
    input: StreamInput,
    limits: ozzyyaml::Limits,
) -> Result<Vec<ozzyyaml::Value>, PyErr> {
    match input {
        StreamInput::Text(s) => ozzyyaml::load_all_with_limits(&s, limits).map_err(map_err),
        StreamInput::Bytes(b) => {
            ozzyyaml::load_all_from_slice_with_limits(&b, limits).map_err(map_err)
        }
    }
}

fn value_to_py(
    py: Python<'_>,
    value: &Value,
    cache: &mut HashMap<*const Value, Py<PyAny>>,
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

fn mapping_key_to_hashable(py: Python<'_>, key: Py<PyAny>) -> PyResult<Py<PyAny>> {
    Ok(hashable_key_item(py, key.bind(py))?.unbind())
}

fn hashable_key_item<'py>(py: Python<'py>, obj: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if let Ok(list) = obj.cast::<PyList>() {
        let mut items = Vec::with_capacity(list.len());
        for item in list.iter() {
            items.push(hashable_key_item(py, &item)?);
        }
        return Ok(PyTuple::new(py, items)?.into_any());
    }
    if let Ok(dict) = obj.cast::<PyDict>() {
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

fn py_to_value(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
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
    if let Ok(list) = obj.cast::<PyList>() {
        let mut items = Vec::with_capacity(list.len());
        for item in list.iter() {
            items.push(Rc::new(py_to_value(&item)?));
        }
        return Ok(Value::Sequence(std::cell::RefCell::new(items)));
    }
    if let Ok(tup) = obj.cast::<PyTuple>() {
        let mut items = Vec::with_capacity(tup.len());
        for item in tup.iter() {
            items.push(Rc::new(py_to_value(&item)?));
        }
        return Ok(Value::Sequence(std::cell::RefCell::new(items)));
    }
    if let Ok(dict) = obj.cast::<PyDict>() {
        let mut entries = Vec::with_capacity(dict.len());
        for (k, v) in dict.iter() {
            entries.push((Rc::new(py_to_value(&k)?), Rc::new(py_to_value(&v)?)));
        }
        return Ok(Value::Mapping(std::cell::RefCell::new(entries)));
    }
    Err(PyTypeError::new_err(format!(
        "cannot dump Python type {}",
        obj.get_type().name()?
    )))
}

#[pyfunction]
#[pyo3(signature = (stream, *, max_anchors=None, max_nesting_depth=None, max_input_bytes=None))]
fn load(
    py: Python<'_>,
    stream: Bound<'_, PyAny>,
    max_anchors: Option<usize>,
    max_nesting_depth: Option<usize>,
    max_input_bytes: Option<usize>,
) -> PyResult<Py<PyAny>> {
    let input = input_from_py(stream)?;
    let limits = limits_from_kwargs(max_anchors, max_nesting_depth, max_input_bytes);
    let value = load_one(input, limits)?;
    let mut cache = HashMap::new();
    value_to_py(py, &value, &mut cache)
}

#[pyfunction]
#[pyo3(signature = (stream, *, max_anchors=None, max_nesting_depth=None, max_input_bytes=None))]
fn load_all(
    py: Python<'_>,
    stream: Bound<'_, PyAny>,
    max_anchors: Option<usize>,
    max_nesting_depth: Option<usize>,
    max_input_bytes: Option<usize>,
) -> PyResult<Py<PyAny>> {
    let input = input_from_py(stream)?;
    let limits = limits_from_kwargs(max_anchors, max_nesting_depth, max_input_bytes);
    let docs = load_docs(input, limits)?;
    let list = PyList::empty(py);
    for doc in &docs {
        let mut cache = HashMap::new();
        list.append(value_to_py(py, doc, &mut cache)?)?;
    }
    Ok(list.into_any().unbind())
}

#[pyfunction]
#[pyo3(signature = (data, *, style="flow"))]
fn dump(data: Bound<'_, PyAny>, style: &str) -> PyResult<String> {
    let value = py_to_value(&data)?;
    let opts = EmitOptions {
        style: parse_style(style)?,
    };
    to_string_with_options(&value, &opts).map_err(map_err)
}

#[pyfunction]
#[pyo3(signature = (docs, *, style="flow"))]
fn dump_all(docs: Bound<'_, PyAny>, style: &str) -> PyResult<String> {
    let list = docs.cast::<PyList>()?;
    let mut values = Vec::with_capacity(list.len());
    for item in list.iter() {
        values.push(py_to_value(&item)?);
    }
    let opts = EmitOptions {
        style: parse_style(style)?,
    };
    dump_all_with_options(&values, &opts).map_err(map_err)
}

// Rc-backed Value graphs are intentionally unsendable; keep the GIL while
// free-threading support is only a build/import smoke, not a concurrency claim.
#[pymodule(gil_used = true)]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("OzzyYamlError", m.py().get_type::<OzzyYamlError>())?;
    m.add_class::<graph::YamlValue>()?;
    m.add_function(wrap_pyfunction!(load, m)?)?;
    m.add_function(wrap_pyfunction!(load_all, m)?)?;
    m.add_function(wrap_pyfunction!(dump, m)?)?;
    m.add_function(wrap_pyfunction!(dump_all, m)?)?;
    m.add_function(wrap_pyfunction!(graph::load_value, m)?)?;
    m.add_function(wrap_pyfunction!(graph::load_all_value, m)?)?;
    m.add_function(wrap_pyfunction!(graph::dump_value, m)?)?;
    Ok(())
}
