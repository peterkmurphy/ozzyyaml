//! Recursive `Value` writer (flow and block styles).

use crate::error::{Error, Result};
use crate::options::{EmitOptions, EmitStyle};
use crate::scalar::{format_float, format_string};
use ozzyyaml::Value;
use std::rc::Rc;

/// Emit one [`Value`] using [`EmitOptions::default`].
pub fn to_string(value: &Value) -> Result<String> {
    to_string_with_options(value, &EmitOptions::default())
}

/// Emit one [`Value`] with explicit options.
///
/// Cyclic structures return an error (anchors are not emitted yet).
pub fn to_string_with_options(value: &Value, opts: &EmitOptions) -> Result<String> {
    let mut path = Vec::new();
    let text = match opts.style {
        EmitStyle::Flow => flow(value, &mut path)?,
        EmitStyle::Block => block_lines(value, &mut path)?.join("\n"),
    };
    Ok(format!("{text}\n"))
}

/// Emit every document in `docs` using [`EmitOptions::default`].
pub fn dump_all(docs: &[Value]) -> Result<String> {
    dump_all_with_options(docs, &EmitOptions::default())
}

/// Emit every document in `docs` with explicit options.
///
/// Returns an error until multi-document emission is implemented.
pub fn dump_all_with_options(_docs: &[Value], _opts: &EmitOptions) -> Result<String> {
    Err(Error::Emit {
        message: "not implemented".into(),
    })
}

/// Pointers of the collections currently being visited (cycle detection).
type Path = Vec<*const Value>;

fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::Null => Some("~".into()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Int(i) => Some(i.to_string()),
        Value::Float(f) => Some(format_float(*f)),
        Value::String(s) => Some(format_string(s)),
        Value::Sequence(_) | Value::Mapping(_) => None,
    }
}

fn enter(value: &Value, path: &mut Path) -> Result<()> {
    let ptr: *const Value = value;
    if path.contains(&ptr) {
        return Err(Error::Emit {
            message: "cyclic structure cannot be emitted without anchors".into(),
        });
    }
    path.push(ptr);
    Ok(())
}

fn flow(value: &Value, path: &mut Path) -> Result<String> {
    if let Some(text) = scalar(value) {
        return Ok(text);
    }
    enter(value, path)?;
    let text = match value {
        Value::Sequence(items) => {
            let parts = items
                .borrow()
                .iter()
                .map(|item| flow(item, path))
                .collect::<Result<Vec<_>>>()?;
            format!("[{}]", parts.join(", "))
        }
        Value::Mapping(entries) => {
            let parts = entries
                .borrow()
                .iter()
                .map(|(k, v)| Ok(format!("{}: {}", flow(k, path)?, flow(v, path)?)))
                .collect::<Result<Vec<_>>>()?;
            format!("{{{}}}", parts.join(", "))
        }
        _ => unreachable!("scalars handled above"),
    };
    path.pop();
    Ok(text)
}

fn is_empty_collection(value: &Value) -> bool {
    match value {
        Value::Sequence(items) => items.borrow().is_empty(),
        Value::Mapping(entries) => entries.borrow().is_empty(),
        _ => false,
    }
}

/// Block-style text for `value` as lines at relative indent 0.
///
/// Scalars and empty collections yield a single inline line.
fn block_lines(value: &Value, path: &mut Path) -> Result<Vec<String>> {
    if scalar(value).is_some() || is_empty_collection(value) {
        return Ok(vec![flow(value, path)?]);
    }
    enter(value, path)?;
    let mut lines = Vec::new();
    match value {
        Value::Sequence(items) => {
            for item in items.borrow().iter() {
                push_item(&mut lines, "- ", item, path)?;
            }
        }
        Value::Mapping(entries) => {
            for (k, v) in entries.borrow().iter() {
                let key = flow(k, path)?;
                push_entry(&mut lines, &key, v, path)?;
            }
        }
        _ => unreachable!("scalars handled above"),
    }
    path.pop();
    Ok(lines)
}

/// Sequence item: compact nested block (`- - a`, `- k: v`) or inline.
fn push_item(
    lines: &mut Vec<String>,
    marker: &str,
    item: &Rc<Value>,
    path: &mut Path,
) -> Result<()> {
    let mut child = block_lines(item, path)?.into_iter();
    if let Some(first) = child.next() {
        lines.push(format!("{marker}{first}"));
    }
    lines.extend(child.map(|line| format!("  {line}")));
    Ok(())
}

/// Mapping entry: `key: inline` or `key:` followed by an indented block.
fn push_entry(lines: &mut Vec<String>, key: &str, v: &Rc<Value>, path: &mut Path) -> Result<()> {
    let child = block_lines(v, path)?;
    if scalar(v).is_some() || is_empty_collection(v) {
        lines.push(format!("{key}: {}", child[0]));
    } else {
        lines.push(format!("{key}:"));
        lines.extend(child.into_iter().map(|line| format!("  {line}")));
    }
    Ok(())
}
