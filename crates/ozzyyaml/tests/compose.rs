use std::rc::Rc;

use ozzyyaml::{
    from_slice, from_slice_with_limits, from_str, from_str_with_limits, load_all,
    load_all_from_slice, load_all_from_slice_with_limits, Error, Limits, Value,
};

#[test]
fn scalar_document() {
    assert_eq!(from_str("42\n").unwrap(), Value::Int(42));
    assert_eq!(from_str("true\n").unwrap(), Value::Bool(true));
    assert_eq!(from_str("hello\n").unwrap(), Value::String("hello".into()));
}

#[test]
fn sequence_and_mapping() {
    let v = from_str("- a\n- b\n").unwrap();
    let seq = v.as_sequence().unwrap();
    assert_eq!(seq.len(), 2);
    assert_eq!(*seq[0], Value::String("a".into()));
    assert_eq!(*seq[1], Value::String("b".into()));

    let v = from_str("k: v\n").unwrap();
    let map = v.as_mapping().unwrap();
    assert_eq!(map.len(), 1);
    assert_eq!(*map[0].0, Value::String("k".into()));
    assert_eq!(*map[0].1, Value::String("v".into()));
}

#[test]
fn alias_shares_identity() {
    let v = from_str("[&a hello, *a]\n").unwrap();
    let seq = v.as_sequence().unwrap();
    assert!(Rc::ptr_eq(&seq[0], &seq[1]));
}

#[test]
fn cyclic_sequence() {
    let v = from_str("&a [*a]\n").unwrap();
    let child = Rc::clone(&v.as_sequence().unwrap()[0]);
    let grandchild = Rc::clone(&child.as_sequence().unwrap()[0]);
    assert!(Rc::ptr_eq(&child, &grandchild));
}

#[test]
fn unicode_anchor_round_trip() {
    let v = from_str("[&汉 x, *汉]\n").unwrap();
    let seq = v.as_sequence().unwrap();
    assert!(Rc::ptr_eq(&seq[0], &seq[1]));
    assert_eq!(*seq[0], Value::String("x".into()));
}

#[test]
fn max_anchors_limit() {
    let err = from_str_with_limits("&a 1\n", Limits::default().max_anchors(0)).unwrap_err();
    match err {
        Error::LimitExceeded { limit: 0, .. } => {}
        other => panic!("expected LimitExceeded, got {other:?}"),
    }
}

#[test]
fn undefined_alias() {
    assert!(matches!(
        from_str("*missing\n").unwrap_err(),
        Error::UndefinedAlias { name, .. } if name == "missing"
    ));
}

#[test]
fn undefined_alias_has_nonzero_mark() {
    // A bare `*missing` after a scalar is a second document root (parse error).
    // A block sequence keeps the undefined alias on line 2.
    let err = from_str("- x\n- *missing\n").unwrap_err();
    match err {
        Error::UndefinedAlias { mark, .. } => {
            // `- *missing` places `*` at line 2, column 3 (byte index 6).
            assert_eq!(mark.line, 2);
            assert_eq!(mark.column, 3);
            assert_eq!(mark.index, 6);
        }
        other => panic!("expected UndefinedAlias, got {other:?}"),
    }
}

#[test]
fn from_slice_rejects_invalid_utf8() {
    let err = from_slice(&[0x80]).unwrap_err();
    match err {
        Error::InvalidUtf8 { valid_up_to } => assert_eq!(valid_up_to, 0),
        other => panic!("expected InvalidUtf8, got {other:?}"),
    }
}

#[test]
fn from_slice_parses_utf8() {
    assert_eq!(from_slice(b"42\n").unwrap(), Value::Int(42));
}

#[test]
fn duplicate_anchor() {
    let err = from_str("[&a 1, &a 2]\n").unwrap_err();
    assert!(matches!(err, Error::DuplicateAnchor { name, .. } if name == "a"));
}

#[test]
fn load_all_multi_document() {
    let docs = load_all("1\n---\n2\n").unwrap();
    assert_eq!(docs, vec![Value::Int(1), Value::Int(2)]);
}

#[test]
fn from_str_rejects_multi_doc() {
    assert!(matches!(
        from_str("1\n---\n2\n").unwrap_err(),
        Error::DocumentCount { .. }
    ));
}

#[test]
fn core_rejects_yes_as_bool() {
    assert_eq!(from_str("yes\n").unwrap(), Value::String("yes".into()));
}

#[test]
fn from_slice_with_limits_parses() {
    assert_eq!(
        from_slice_with_limits(b"7\n", Limits::default()).unwrap(),
        Value::Int(7)
    );
}

#[test]
fn load_all_from_slice_multi_doc() {
    let docs = load_all_from_slice(b"1\n---\n2\n").unwrap();
    assert_eq!(docs, vec![Value::Int(1), Value::Int(2)]);
}

#[test]
fn load_all_from_slice_rejects_invalid_utf8() {
    let err = load_all_from_slice(&[b'1', b'\n', 0xff]).unwrap_err();
    match err {
        Error::InvalidUtf8 { valid_up_to } => assert_eq!(valid_up_to, 2),
        other => panic!("expected InvalidUtf8, got {other:?}"),
    }
}

#[test]
fn load_all_from_slice_with_limits_rejects_invalid_utf8() {
    let err = load_all_from_slice_with_limits(&[0x80], Limits::default()).unwrap_err();
    match err {
        Error::InvalidUtf8 { valid_up_to: 0 } => {}
        other => panic!("expected InvalidUtf8, got {other:?}"),
    }
}

#[test]
fn duplicate_anchor_has_mark() {
    let err = from_str("[&a 1, &a 2]\n").unwrap_err();
    match err {
        Error::DuplicateAnchor { name, mark } => {
            assert_eq!(name, "a");
            assert!(mark.line >= 1);
            assert!(mark.column >= 1);
            // Second `&a` is not at stream start.
            assert!(mark.index > 0 || mark.column > 1);
        }
        other => panic!("expected DuplicateAnchor, got {other:?}"),
    }
}

#[test]
fn max_anchors_limit_has_mark() {
    let err = from_str_with_limits("&a 1\n", Limits::default().max_anchors(0)).unwrap_err();
    match err {
        Error::LimitExceeded { mark, limit: 0, .. } => {
            assert!(mark.line >= 1);
            assert!(mark.column >= 1);
        }
        other => panic!("expected LimitExceeded, got {other:?}"),
    }
}
