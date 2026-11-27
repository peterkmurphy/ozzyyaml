//! Iterative `Value` writer (flow and block styles) with `Rc`-identity anchors.

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

/// A node to emit: either the borrowed root or an owned `Rc` child.
enum Node<'a> {
    Borrowed(&'a Value),
    Owned(Rc<Value>),
}

impl Node<'_> {
    fn value(&self) -> &Value {
        match self {
            Node::Borrowed(v) => v,
            Node::Owned(rc) => rc.as_ref(),
        }
    }
}

enum FlowFrame {
    Seq {
        prefix: String,
        rest: Vec<Rc<Value>>,
        parts: Vec<String>,
        as_key: bool,
    },
    /// Waiting for the mapping key string; `waiting_value` is emitted next.
    MapKey {
        prefix: String,
        rest: Vec<(Rc<Value>, Rc<Value>)>,
        parts: Vec<String>,
        waiting_value: Rc<Value>,
        as_key: bool,
    },
    /// Waiting for the mapping value string after `pending_key`.
    MapValue {
        prefix: String,
        rest: Vec<(Rc<Value>, Rc<Value>)>,
        parts: Vec<String>,
        pending_key: String,
        as_key: bool,
    },
}

enum BlockFrame {
    Seq {
        props: Option<String>,
        rest: Vec<Rc<Value>>,
        lines: Vec<String>,
    },
    MapKey {
        props: Option<String>,
        rest: Vec<(Rc<Value>, Rc<Value>)>,
        lines: Vec<String>,
        waiting_value: Rc<Value>,
    },
    MapValue {
        props: Option<String>,
        rest: Vec<(Rc<Value>, Rc<Value>)>,
        lines: Vec<String>,
        pending_key: String,
    },
}

enum FlowStep<'a> {
    Done(String),
    Continue(Node<'a>, bool),
}

enum BlockStep<'a> {
    Done(Block),
    Continue(Node<'a>),
}

impl Writer {
    fn flow(&mut self, value: &Value) -> String {
        self.flow_node(Node::Borrowed(value), false)
    }

    fn maybe_key_space(text: String, as_key: bool) -> String {
        if as_key && text.starts_with('*') {
            format!("{text} ")
        } else {
            text
        }
    }

    fn flow_node(&mut self, root: Node<'_>, root_as_key: bool) -> String {
        let mut stack: Vec<FlowFrame> = Vec::new();
        let mut current = Some((root, root_as_key));
        let mut finished: Option<String> = None;

        loop {
            if let Some((node, as_key)) = current.take() {
                match self.flow_enter(node, as_key, &mut stack) {
                    FlowStep::Done(s) => finished = Some(s),
                    FlowStep::Continue(next, next_as_key) => {
                        current = Some((next, next_as_key));
                    }
                }
                continue;
            }

            let part = finished.take().expect("flow machine invariant");
            let Some(frame) = stack.pop() else {
                return part;
            };
            match frame {
                FlowFrame::Seq {
                    prefix,
                    mut rest,
                    mut parts,
                    as_key,
                } => {
                    parts.push(part);
                    if let Some(next) = rest.pop() {
                        stack.push(FlowFrame::Seq {
                            prefix,
                            rest,
                            parts,
                            as_key,
                        });
                        current = Some((Node::Owned(next), false));
                    } else {
                        finished = Some(Self::maybe_key_space(
                            format!("{prefix}[{}]", parts.join(", ")),
                            as_key,
                        ));
                    }
                }
                FlowFrame::MapKey {
                    prefix,
                    rest,
                    parts,
                    waiting_value,
                    as_key,
                } => {
                    stack.push(FlowFrame::MapValue {
                        prefix,
                        rest,
                        parts,
                        pending_key: part,
                        as_key,
                    });
                    current = Some((Node::Owned(waiting_value), false));
                }
                FlowFrame::MapValue {
                    prefix,
                    mut rest,
                    mut parts,
                    pending_key,
                    as_key,
                } => {
                    parts.push(format!("{pending_key}: {part}"));
                    if let Some((k, v)) = rest.pop() {
                        stack.push(FlowFrame::MapKey {
                            prefix,
                            rest,
                            parts,
                            waiting_value: v,
                            as_key,
                        });
                        current = Some((Node::Owned(k), true));
                    } else {
                        finished = Some(Self::maybe_key_space(
                            format!("{prefix}{{{}}}", parts.join(", ")),
                            as_key,
                        ));
                    }
                }
            }
        }
    }

    fn flow_enter<'a>(
        &mut self,
        node: Node<'a>,
        as_key: bool,
        stack: &mut Vec<FlowFrame>,
    ) -> FlowStep<'a> {
        let value = node.value();
        let prefix = match self.anchors.visit(value) {
            Visit::Alias(name) => {
                return FlowStep::Done(Self::maybe_key_space(alias(&name), as_key));
            }
            Visit::Anchor(name) => format!("{} ", anchor_decl(&name)),
            Visit::Plain => String::new(),
        };
        match value {
            Value::Sequence(items) => {
                let mut rest: Vec<Rc<Value>> = items.borrow().iter().cloned().collect();
                if rest.is_empty() {
                    return FlowStep::Done(Self::maybe_key_space(format!("{prefix}[]"), as_key));
                }
                rest.reverse();
                let first = rest.pop().expect("non-empty");
                stack.push(FlowFrame::Seq {
                    prefix,
                    rest,
                    parts: Vec::new(),
                    as_key,
                });
                FlowStep::Continue(Node::Owned(first), false)
            }
            Value::Mapping(entries) => {
                let mut rest: Vec<(Rc<Value>, Rc<Value>)> =
                    entries.borrow().iter().cloned().collect();
                if rest.is_empty() {
                    return FlowStep::Done(Self::maybe_key_space(format!("{prefix}{{}}"), as_key));
                }
                rest.reverse();
                let (k, v) = rest.pop().expect("non-empty");
                stack.push(FlowFrame::MapKey {
                    prefix,
                    rest,
                    parts: Vec::new(),
                    waiting_value: v,
                    as_key,
                });
                FlowStep::Continue(Node::Owned(k), true)
            }
            other => {
                let body = scalar(other).expect("collections handled above");
                FlowStep::Done(Self::maybe_key_space(format!("{prefix}{body}"), as_key))
            }
        }
    }

    fn block(&mut self, value: &Value) -> Block {
        self.block_node(Node::Borrowed(value))
    }

    fn block_node(&mut self, root: Node<'_>) -> Block {
        let mut stack: Vec<BlockFrame> = Vec::new();
        let mut current = Some(root);
        let mut finished: Option<Block> = None;

        loop {
            if let Some(node) = current.take() {
                match self.block_enter(node, &mut stack) {
                    BlockStep::Done(b) => finished = Some(b),
                    BlockStep::Continue(next) => current = Some(next),
                }
                continue;
            }

            let part = finished.take().expect("block machine invariant");
            let Some(frame) = stack.pop() else {
                return part;
            };
            match frame {
                BlockFrame::Seq {
                    props,
                    mut rest,
                    mut lines,
                } => {
                    Self::push_item_lines(&mut lines, part);
                    if let Some(next) = rest.pop() {
                        stack.push(BlockFrame::Seq { props, rest, lines });
                        current = Some(Node::Owned(next));
                    } else {
                        finished = Some(Block::Lines { props, lines });
                    }
                }
                BlockFrame::MapKey {
                    props,
                    rest,
                    lines,
                    waiting_value,
                } => {
                    // Keys are always flow-rendered; `part` must be Inline.
                    let key = match part {
                        Block::Inline(text) => text,
                        Block::Lines { .. } => {
                            unreachable!("mapping keys are scalars/aliases/empty")
                        }
                    };
                    stack.push(BlockFrame::MapValue {
                        props,
                        rest,
                        lines,
                        pending_key: key,
                    });
                    current = Some(Node::Owned(waiting_value));
                }
                BlockFrame::MapValue {
                    props,
                    mut rest,
                    mut lines,
                    pending_key,
                } => {
                    Self::push_entry_lines(&mut lines, &pending_key, part);
                    if let Some((k, v)) = rest.pop() {
                        stack.push(BlockFrame::MapKey {
                            props,
                            rest,
                            lines,
                            waiting_value: v,
                        });
                        // Key path uses flow via block_enter on scalars; for
                        // complex keys, block_enter still flow-wraps scalars /
                        // empty, and visits collections as block — keys that
                        // are collections go through block. Original used
                        // `self.key(k)` (flow). Preserve that: emit key via flow.
                        finished = Some(Block::Inline(self.flow_node(Node::Owned(k), true)));
                    } else {
                        finished = Some(Block::Lines { props, lines });
                    }
                }
            }
        }
    }

    fn block_enter<'a>(&mut self, node: Node<'a>, stack: &mut Vec<BlockFrame>) -> BlockStep<'a> {
        let value = node.value();
        if scalar(value).is_some() || is_empty_collection(value) {
            return BlockStep::Done(Block::Inline(self.flow_node(Node::Borrowed(value), false)));
        }
        let props = match self.anchors.visit(value) {
            Visit::Alias(name) => return BlockStep::Done(Block::Inline(alias(&name))),
            Visit::Anchor(name) => Some(anchor_decl(&name)),
            Visit::Plain => None,
        };
        match value {
            Value::Sequence(items) => {
                let mut rest: Vec<Rc<Value>> = items.borrow().iter().cloned().collect();
                if rest.is_empty() {
                    return BlockStep::Done(Block::Lines {
                        props,
                        lines: Vec::new(),
                    });
                }
                rest.reverse();
                let first = rest.pop().expect("non-empty");
                stack.push(BlockFrame::Seq {
                    props,
                    rest,
                    lines: Vec::new(),
                });
                BlockStep::Continue(Node::Owned(first))
            }
            Value::Mapping(entries) => {
                let mut rest: Vec<(Rc<Value>, Rc<Value>)> =
                    entries.borrow().iter().cloned().collect();
                if rest.is_empty() {
                    return BlockStep::Done(Block::Lines {
                        props,
                        lines: Vec::new(),
                    });
                }
                rest.reverse();
                let (k, v) = rest.pop().expect("non-empty");
                stack.push(BlockFrame::MapKey {
                    props,
                    rest,
                    lines: Vec::new(),
                    waiting_value: v,
                });
                // Keys use flow form (including alias trailing space).
                BlockStep::Done(Block::Inline(self.flow_node(Node::Owned(k), true)))
            }
            _ => unreachable!("scalars handled above"),
        }
    }

    fn push_item_lines(lines: &mut Vec<String>, item: Block) {
        match item {
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

    fn push_entry_lines(lines: &mut Vec<String>, key: &str, v: Block) {
        match v {
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
