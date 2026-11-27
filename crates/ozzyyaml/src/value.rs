use std::cell::RefCell;
use std::rc::Rc;

/// A YAML 1.2.2 Core-schema value.
///
/// Sequences and mappings hold [`Rc`] children so anchors/aliases share
/// identity without deep copying. Collection bodies use [`RefCell`] so a
/// node can be registered as an anchor before its children are filled,
/// which makes cyclic graphs representable.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// YAML null.
    Null,
    /// YAML boolean (`true` / `false` only under Core).
    Bool(bool),
    /// YAML integer (fits in `i64` for Milestone B).
    Int(i64),
    /// YAML float.
    Float(f64),
    /// YAML string (also the fallback for unresolved plain scalars).
    String(String),
    /// YAML sequence.
    Sequence(RefCell<Vec<Rc<Value>>>),
    /// YAML mapping as ordered key/value pairs (duplicate keys kept).
    Mapping(RefCell<Vec<(Rc<Value>, Rc<Value>)>>),
}

/// Borrowed mapping entries.
pub type MappingEntries<'a> = std::cell::Ref<'a, Vec<(Rc<Value>, Rc<Value>)>>;

impl Drop for Value {
    fn drop(&mut self) {
        let mut stack = Vec::new();
        Self::drain_children(self, &mut stack);
        while let Some(mut node) = stack.pop() {
            Self::drain_children(&mut node, &mut stack);
            // `node` is now an empty collection or scalar; its Drop is shallow.
        }
    }
}

impl Value {
    fn drain_children(value: &mut Value, stack: &mut Vec<Value>) {
        match value {
            Value::Sequence(cell) => {
                for child in cell.get_mut().drain(..) {
                    if let Ok(inner) = Rc::try_unwrap(child) {
                        stack.push(inner);
                    }
                }
            }
            Value::Mapping(cell) => {
                for (k, v) in cell.get_mut().drain(..) {
                    if let Ok(inner) = Rc::try_unwrap(k) {
                        stack.push(inner);
                    }
                    if let Ok(inner) = Rc::try_unwrap(v) {
                        stack.push(inner);
                    }
                }
            }
            _ => {}
        }
    }

    /// `Some(())` if this value is null.
    pub fn as_null(&self) -> Option<()> {
        match self {
            Self::Null => Some(()),
            _ => None,
        }
    }

    /// The boolean, or `None` if not a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// The integer, or `None` if not an integer.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// The float, or `None` if not a float.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(value) => Some(*value),
            _ => None,
        }
    }

    /// Borrow the string, or `None` if not a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    /// Borrow sequence items, or `None` if not a sequence.
    pub fn as_sequence(&self) -> Option<std::cell::Ref<'_, Vec<Rc<Value>>>> {
        match self {
            Self::Sequence(items) => Some(items.borrow()),
            _ => None,
        }
    }

    /// Borrow mapping entries, or `None` if not a mapping.
    pub fn as_mapping(&self) -> Option<MappingEntries<'_>> {
        match self {
            Self::Mapping(entries) => Some(entries.borrow()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc_clones_share_identity() {
        let v = Rc::new(Value::String("x".into()));
        let a = Rc::clone(&v);
        let b = Rc::clone(&v);
        assert!(Rc::ptr_eq(&a, &b));
    }

    #[test]
    fn scalar_accessors() {
        assert_eq!(Value::Null.as_null(), Some(()));
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Int(7).as_i64(), Some(7));
        assert_eq!(Value::Float(1.5).as_f64(), Some(1.5));
        assert_eq!(Value::String("a".into()).as_str(), Some("a"));

        assert_eq!(Value::Int(1).as_null(), None);
        assert_eq!(Value::Null.as_bool(), None);
        assert_eq!(Value::Bool(true).as_i64(), None);
        assert_eq!(Value::Int(1).as_f64(), None);
        assert_eq!(Value::Float(1.0).as_i64(), None);
        assert_eq!(Value::Int(1).as_str(), None);
    }

    #[test]
    fn sequence_can_form_cycle() {
        let seq = Rc::new(Value::Sequence(RefCell::new(vec![])));
        if let Value::Sequence(cell) = &*seq {
            cell.borrow_mut().push(Rc::clone(&seq));
        }
        let child = Rc::clone(&seq.as_sequence().unwrap()[0]);
        assert!(Rc::ptr_eq(&child, &seq));
        // Break the cycle so the test itself does not leak.
        if let Value::Sequence(cell) = &*seq {
            cell.borrow_mut().clear();
        }
    }

    #[test]
    fn deep_sequence_drops_on_small_stack() {
        const DEPTH: usize = 100_000;
        const STACK: usize = 1024 * 1024;
        let handle = std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(|| {
                let mut node = Value::String("x".into());
                for _ in 0..DEPTH {
                    node = Value::Sequence(RefCell::new(vec![Rc::new(node)]));
                }
                drop(node);
            })
            .expect("spawn");
        handle.join().expect("deep Drop overflowed or panicked");
    }
}
