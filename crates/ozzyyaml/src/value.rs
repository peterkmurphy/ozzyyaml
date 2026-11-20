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
    fn sequence_can_form_cycle() {
        let seq = Rc::new(Value::Sequence(RefCell::new(vec![])));
        if let Value::Sequence(cell) = &*seq {
            cell.borrow_mut().push(Rc::clone(&seq));
        }
        let child = Rc::clone(&seq.as_sequence().unwrap()[0]);
        assert!(Rc::ptr_eq(&child, &seq));
    }
}
