//! Compose parser events into [`Value`] trees.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ozzyyaml_parser::{Event, LimitKind, Mark, Parser, ScalarStyle};

use crate::error::{Error, Result};
use crate::limits::Limits;
use crate::schema::core::resolve_scalar;
use crate::value::Value;

/// Parse a single YAML document into a [`Value`].
///
/// Returns an error if the stream contains zero documents or more than one.
pub fn from_str(input: &str) -> Result<Value> {
    from_str_with_limits(input, Limits::default())
}

/// [`from_str`] with custom [`Limits`].
pub fn from_str_with_limits(input: &str, limits: Limits) -> Result<Value> {
    let mut docs = load_all_with_limits(input, limits)?;
    match docs.len() {
        1 => Ok(docs.pop().expect("len checked")),
        0 => Err(Error::DocumentCount {
            message: "expected exactly one document, found none".into(),
        }),
        n => Err(Error::DocumentCount {
            message: format!("expected exactly one document, found {n}"),
        }),
    }
}

/// Parse every document in a YAML stream.
pub fn load_all(input: &str) -> Result<Vec<Value>> {
    load_all_with_limits(input, Limits::default())
}

/// [`load_all`] with custom [`Limits`].
pub fn load_all_with_limits(input: &str, limits: Limits) -> Result<Vec<Value>> {
    let mut parser = Parser::with_limits(input, limits.to_parser()).map_err(Error::Parse)?;
    let mut composer = Composer::new(limits);
    composer.compose_stream(&mut parser)
}

/// Parse a single YAML document from UTF-8 bytes into a [`Value`].
///
/// Returns [`Error::InvalidUtf8`] when `input` is not valid UTF-8, and the same
/// document-count errors as [`from_str`].
pub fn from_slice(input: &[u8]) -> Result<Value> {
    from_slice_with_limits(input, Limits::default())
}

/// [`from_slice`] with custom [`Limits`].
pub fn from_slice_with_limits(input: &[u8], limits: Limits) -> Result<Value> {
    let s = std::str::from_utf8(input).map_err(|e| Error::InvalidUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    from_str_with_limits(s, limits)
}

/// Parse every document in a UTF-8 byte stream.
///
/// Returns [`Error::InvalidUtf8`] when `input` is not valid UTF-8.
pub fn load_all_from_slice(input: &[u8]) -> Result<Vec<Value>> {
    load_all_from_slice_with_limits(input, Limits::default())
}

/// [`load_all_from_slice`] with custom [`Limits`].
pub fn load_all_from_slice_with_limits(input: &[u8], limits: Limits) -> Result<Vec<Value>> {
    let s = std::str::from_utf8(input).map_err(|e| Error::InvalidUtf8 {
        valid_up_to: e.valid_up_to(),
    })?;
    load_all_with_limits(s, limits)
}

struct Composer {
    limits: Limits,
    /// Anchors of the current document (strong refs for completed / building nodes).
    anchors: HashMap<String, Rc<Value>>,
}

enum Frame {
    Sequence {
        node: Rc<Value>,
    },
    Mapping {
        node: Rc<Value>,
        pending_key: Option<Rc<Value>>,
    },
}

impl Composer {
    fn new(limits: Limits) -> Self {
        Self {
            limits,
            anchors: HashMap::new(),
        }
    }

    fn compose_stream(&mut self, parser: &mut Parser<'_>) -> Result<Vec<Value>> {
        let mut docs = Vec::new();
        let mut frames: Vec<Frame> = Vec::new();
        let mut doc_root: Option<Rc<Value>> = None;
        let mut in_document = false;

        while let Some(event) = parser.next_event().map_err(Error::Parse)? {
            match event {
                Event::StreamStart | Event::StreamEnd => {}
                Event::DocumentStart { .. } => {
                    self.anchors.clear();
                    frames.clear();
                    doc_root = None;
                    in_document = true;
                }
                Event::DocumentEnd { .. } => {
                    if !frames.is_empty() {
                        return Err(Error::Compose {
                            message: "document ended with open collection".into(),
                        });
                    }
                    if let Some(root) = doc_root.take() {
                        // Prefer returning owned Value when uniquely held.
                        let value = Rc::try_unwrap(root).unwrap_or_else(|rc| (*rc).clone());
                        docs.push(value);
                    } else {
                        // Empty document → null.
                        docs.push(Value::Null);
                    }
                    in_document = false;
                }
                Event::SequenceStart { anchor, .. } => {
                    let node = Rc::new(Value::Sequence(RefCell::new(Vec::new())));
                    if let Some(name) = anchor {
                        self.register_anchor(name, Rc::clone(&node), parser.last_mark())?;
                    }
                    frames.push(Frame::Sequence { node });
                }
                Event::MappingStart { anchor, .. } => {
                    let node = Rc::new(Value::Mapping(RefCell::new(Vec::new())));
                    if let Some(name) = anchor {
                        self.register_anchor(name, Rc::clone(&node), parser.last_mark())?;
                    }
                    frames.push(Frame::Mapping {
                        node,
                        pending_key: None,
                    });
                }
                Event::SequenceEnd => {
                    let frame = frames.pop().ok_or_else(|| Error::Compose {
                        message: "SequenceEnd without SequenceStart".into(),
                    })?;
                    let Frame::Sequence { node } = frame else {
                        return Err(Error::Compose {
                            message: "SequenceEnd while not in a sequence".into(),
                        });
                    };
                    self.push_node(&mut frames, &mut doc_root, node)?;
                }
                Event::MappingEnd => {
                    let frame = frames.pop().ok_or_else(|| Error::Compose {
                        message: "MappingEnd without MappingStart".into(),
                    })?;
                    let Frame::Mapping { node, pending_key } = frame else {
                        return Err(Error::Compose {
                            message: "MappingEnd while not in a mapping".into(),
                        });
                    };
                    if pending_key.is_some() {
                        return Err(Error::Compose {
                            message: "mapping ended with pending key".into(),
                        });
                    }
                    self.push_node(&mut frames, &mut doc_root, node)?;
                }
                Event::Scalar {
                    anchor,
                    tag,
                    value,
                    style,
                } => {
                    let plain = style == ScalarStyle::Plain;
                    let mark = parser.last_mark();
                    let resolved = resolve_scalar(&value, tag.as_deref(), plain, mark)?;
                    let node = Rc::new(resolved);
                    if let Some(name) = anchor {
                        self.register_anchor(name, Rc::clone(&node), parser.last_mark())?;
                    }
                    self.push_node(&mut frames, &mut doc_root, node)?;
                }
                Event::Alias { anchor } => {
                    let mark = parser.last_mark();
                    let node = self
                        .anchors
                        .get(&anchor)
                        .cloned()
                        .ok_or(Error::UndefinedAlias { name: anchor, mark })?;
                    self.push_node(&mut frames, &mut doc_root, node)?;
                }
            }
        }

        if in_document {
            return Err(Error::Compose {
                message: "stream ended inside a document".into(),
            });
        }
        Ok(docs)
    }

    fn register_anchor(&mut self, name: String, node: Rc<Value>, mark: Mark) -> Result<()> {
        if self.anchors.contains_key(&name) {
            return Err(Error::DuplicateAnchor { name, mark });
        }
        if self.anchors.len() >= self.limits.max_anchors {
            return Err(Error::LimitExceeded {
                kind: LimitKind::Anchors,
                limit: self.limits.max_anchors,
                mark,
            });
        }
        self.anchors.insert(name, node);
        Ok(())
    }

    fn push_node(
        &self,
        frames: &mut [Frame],
        doc_root: &mut Option<Rc<Value>>,
        node: Rc<Value>,
    ) -> Result<()> {
        match frames.last_mut() {
            None => {
                if doc_root.is_some() {
                    return Err(Error::Compose {
                        message: "multiple root nodes in document".into(),
                    });
                }
                *doc_root = Some(node);
                Ok(())
            }
            Some(Frame::Sequence { node: parent }) => {
                match &**parent {
                    Value::Sequence(cell) => cell.borrow_mut().push(node),
                    _ => {
                        return Err(Error::Compose {
                            message: "internal: sequence frame corrupt".into(),
                        })
                    }
                }
                Ok(())
            }
            Some(Frame::Mapping {
                node: parent,
                pending_key,
            }) => {
                if pending_key.is_none() {
                    *pending_key = Some(node);
                    Ok(())
                } else {
                    let key = pending_key.take().expect("checked");
                    match &**parent {
                        Value::Mapping(cell) => cell.borrow_mut().push((key, node)),
                        _ => {
                            return Err(Error::Compose {
                                message: "internal: mapping frame corrupt".into(),
                            })
                        }
                    }
                    Ok(())
                }
            }
        }
    }
}
