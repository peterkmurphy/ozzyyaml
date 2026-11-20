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

impl Value {
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
    }
}
