//! Recursive `Value` writer (flow and block styles) with `Rc`-identity anchors.

use crate::anchors::{Anchors, Visit};
use crate::error::Result;
use crate::options::{EmitOptions, EmitStyle};
use crate::scalar::{format_float, format_string};
use ozzyyaml::Value;
use std::fmt::Write;
use std::rc::Rc;

/// Emit one [`Value`] using [`EmitOptions::default`].
pub fn to_string(value: &Value) -> Result<String> {
    to_string_with_options(value, &EmitOptions::default())
}

/// Emit one [`Value`] with explicit options.
///
/// Nodes reachable more than once (same `Rc` allocation, including cycles)
/// are written once with an anchor (`&a`) and referenced by alias (`*a`)
/// elsewhere. Anchor names are generated (`a`, `b`, …); equal but distinct
/// nodes are never merged. `value` itself takes part in identity, so a cycle
/// back to the root is anchored at the root.
pub fn to_string_with_options(value: &Value, opts: &EmitOptions) -> Result<String> {
    let mut writer = Writer {
        anchors: Anchors::scan(value),
    };
    let text = match opts.style {
        EmitStyle::Flow => writer.flow(value),
        EmitStyle::Block => match writer.block(value) {
            Block::Inline(text) => text,
            Block::Lines { props, lines } => props
                .into_iter()
                .chain(lines)
                .collect::<Vec<_>>()
                .join("\n"),
        },
    };
    Ok(format!("{text}\n"))
}

/// Emit every document in `docs` using [`EmitOptions::default`].
///
/// See [`dump_all_with_options`] for the stream convention.
pub fn dump_all(docs: &[Value]) -> Result<String> {
    dump_all_with_options(docs, &EmitOptions::default())
}

/// Emit every document in `docs` with explicit options.
///
/// Each document is emitted on its own, so anchor names restart at `a` and an
/// `Rc` shared across documents is written in full in each one. Anchors do
/// not cross document boundaries.
///
/// - `docs.len() > 1`: every document, including the first, is prefixed with
///   `---\n`.
/// - `docs.len() == 1`: same text as [`to_string_with_options`] (no `---`).
/// - `docs.len() == 0`: an empty string (not a null document).
pub fn dump_all_with_options(docs: &[Value], opts: &EmitOptions) -> Result<String> {
    if docs.is_empty() {
        return Ok(String::new());
    }
    if docs.len() == 1 {
        return to_string_with_options(&docs[0], opts);
    }
    let mut out = String::new();
    for doc in docs {
        out.push_str("---\n");
        out.push_str(&to_string_with_options(doc, opts)?);
    }
    Ok(out)
}

/// Write an anchor declaration (`&name`); the name is written verbatim, so
/// any Unicode name that YAML accepts as an anchor works.
fn write_anchor_decl(out: &mut String, name: &str) -> std::fmt::Result {
    write!(out, "&{name}")
}

fn anchor_decl(name: &str) -> String {
    let mut out = String::new();
    write_anchor_decl(&mut out, name).expect("writing to a String cannot fail");
    out
}

/// Block rendering of a node.
enum Block {
    /// Single line: scalar, empty collection, or alias.
    Inline(String),
    /// Non-empty collection lines at relative indent 0; `props` is an anchor
    /// line that must precede them (it cannot share a line with a block
    /// mapping's first key, which it would otherwise anchor).
    Lines {
        props: Option<String>,
        lines: Vec<String>,
    },
}

struct Writer {
    anchors: Anchors,
}

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

fn is_empty_collection(value: &Value) -> bool {
    match value {
        Value::Sequence(items) => items.borrow().is_empty(),
        Value::Mapping(entries) => entries.borrow().is_empty(),
        _ => false,
    }
}

fn alias(name: &str) -> String {
    format!("*{name}")
}

impl Writer {
    fn flow(&mut self, value: &Value) -> String {
        let prefix = match self.anchors.visit(value) {
            Visit::Alias(name) => return alias(&name),
            Visit::Anchor(name) => format!("{} ", anchor_decl(&name)),
            Visit::Plain => String::new(),
        };
        let body = match value {
            Value::Sequence(items) => {
                let parts: Vec<_> = items.borrow().iter().map(|item| self.flow(item)).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Mapping(entries) => {
                let parts: Vec<_> = entries
                    .borrow()
                    .iter()
                    .map(|(k, v)| format!("{}: {}", self.key(k), self.flow(v)))
                    .collect();
                format!("{{{}}}", parts.join(", "))
            }
            other => scalar(other).expect("collections handled above"),
        };
        format!("{prefix}{body}")
    }

    /// Mapping key text (flow form). A trailing space after an alias keeps
    /// the `:` from being read as part of the alias name.
    fn key(&mut self, key: &Value) -> String {
        let text = self.flow(key);
        if text.starts_with('*') {
            format!("{text} ")
        } else {
            text
        }
    }

    fn block(&mut self, value: &Value) -> Block {
        if scalar(value).is_some() || is_empty_collection(value) {
            return Block::Inline(self.flow(value));
        }
        let props = match self.anchors.visit(value) {
            Visit::Alias(name) => return Block::Inline(alias(&name)),
            Visit::Anchor(name) => Some(anchor_decl(&name)),
            Visit::Plain => None,
        };
        let mut lines = Vec::new();
        match value {
            Value::Sequence(items) => {
                for item in items.borrow().iter() {
                    self.push_item(&mut lines, item);
                }
            }
            Value::Mapping(entries) => {
                for (k, v) in entries.borrow().iter() {
                    let key = self.key(k);
                    self.push_entry(&mut lines, &key, v);
                }
            }
            _ => unreachable!("scalars handled above"),
        }
        Block::Lines { props, lines }
    }

    /// Sequence item: compact nested block (`- - a`, `- k: v`) or inline.
    fn push_item(&mut self, lines: &mut Vec<String>, item: &Rc<Value>) {
        match self.block(item) {
            Block::Inline(text) => lines.push(format!("- {text}")),
            Block::Lines {
                props: None,
                lines: child,
            } => {
                let mut child = child.into_iter();
                if let Some(first) = child.next() {
                    lines.push(format!("- {first}"));
                }
                lines.extend(child.map(|line| format!("  {line}")));
            }
            Block::Lines {
                props: Some(p),
                lines: child,
            } => {
                lines.push(format!("- {p}"));
                lines.extend(child.into_iter().map(|line| format!("  {line}")));
            }
        }
    }

    /// Mapping entry: `key: inline` or `key:` followed by an indented block.
    fn push_entry(&mut self, lines: &mut Vec<String>, key: &str, v: &Rc<Value>) {
        match self.block(v) {
            Block::Inline(text) => lines.push(format!("{key}: {text}")),
            Block::Lines {
                props,
                lines: child,
            } => {
                match props {
                    Some(p) => lines.push(format!("{key}: {p}")),
                    None => lines.push(format!("{key}:")),
                }
                lines.extend(child.into_iter().map(|line| format!("  {line}")));
            }
        }
    }
}

#[cfg(test)]
mod anchor_name_tests {
    #[test]
    fn unicode_name_is_written_verbatim() {
        let mut s = String::new();
        super::write_anchor_decl(&mut s, "汉").unwrap();
        assert!(s.contains("&汉"));
    }
}
