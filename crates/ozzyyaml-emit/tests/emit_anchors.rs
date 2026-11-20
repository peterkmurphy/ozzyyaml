use ozzyyaml::{from_str, Value};
use ozzyyaml_emit::{to_string, to_string_with_options, EmitOptions, EmitStyle};
use std::cell::RefCell;
use std::rc::Rc;

fn block() -> EmitOptions {
    EmitOptions {
        style: EmitStyle::Block,
    }
}

fn seq_of(items: Vec<Rc<Value>>) -> Value {
    Value::Sequence(RefCell::new(items))
}

/// Break `Rc` cycles so tests do not leak.
fn clear(value: &Value) {
    if let Value::Sequence(items) = value {
        items.borrow_mut().clear();
    }
}

#[test]
fn emit_shared_alias_round_trip() {
    let v = from_str("[&a hello, *a]\n").unwrap();
    let out = to_string(&v).unwrap();
    assert!(out.contains('*') || out.contains('&'), "{out}");
    let v2 = from_str(&out).unwrap();
    let seq = v2.as_sequence().unwrap();
    assert!(Rc::ptr_eq(&seq[0], &seq[1]));
}

#[test]
fn emit_cycle_round_trip() {
    let v = from_str("&a [*a]\n").unwrap();
    let out = to_string(&v).unwrap();
    let v2 = from_str(&out).unwrap();
    let child = Rc::clone(&v2.as_sequence().unwrap()[0]);
    let g = Rc::clone(&child.as_sequence().unwrap()[0]);
    assert!(Rc::ptr_eq(&child, &g));
}

#[test]
fn emit_unicode_anchor_round_trip() {
    let v = from_str("[&汉 x, *汉]\n").unwrap();
    let out = to_string(&v).unwrap();
    let v2 = from_str(&out).unwrap();
    let seq = v2.as_sequence().unwrap();
    assert!(Rc::ptr_eq(&seq[0], &seq[1]));
    assert_eq!(seq[0].as_str(), Some("x"));
}

#[test]
fn shared_scalar_flow_text() {
    let shared = Rc::new(Value::Int(9));
    let v = seq_of(vec![shared.clone(), shared]);
    assert_eq!(to_string(&v).unwrap(), "[&a 9, *a]\n");
}

#[test]
fn shared_scalar_block_text() {
    let shared = Rc::new(Value::String("hi".into()));
    let v = seq_of(vec![shared.clone(), shared]);
    assert_eq!(
        to_string_with_options(&v, &block()).unwrap(),
        "- &a hi\n- *a\n"
    );
}

#[test]
fn equal_but_distinct_rcs_are_not_anchored() {
    let v = seq_of(vec![
        Rc::new(Value::String("x".into())),
        Rc::new(Value::String("x".into())),
    ]);
    assert_eq!(to_string(&v).unwrap(), "[x, x]\n");
}

#[test]
fn shared_collection_round_trips_in_both_styles() {
    let inner = Rc::new(seq_of(vec![Rc::new(Value::Int(1)), Rc::new(Value::Int(2))]));
    let v = seq_of(vec![inner.clone(), inner.clone(), inner]);
    assert_eq!(to_string(&v).unwrap(), "[&a [1, 2], *a, *a]\n");
    for opts in [EmitOptions::default(), block()] {
        let out = to_string_with_options(&v, &opts).unwrap();
        let back = from_str(&out).unwrap_or_else(|e| panic!("{e}\n{out}"));
        let items = back.as_sequence().unwrap();
        assert!(Rc::ptr_eq(&items[0], &items[1]), "{out}");
        assert!(Rc::ptr_eq(&items[1], &items[2]), "{out}");
        assert_eq!(back, v, "{out}");
    }
}

#[test]
fn shared_block_collection_puts_anchor_on_its_own_line() {
    let inner = Rc::new(seq_of(vec![Rc::new(Value::Int(1)), Rc::new(Value::Int(2))]));
    let v = seq_of(vec![inner.clone(), inner]);
    assert_eq!(
        to_string_with_options(&v, &block()).unwrap(),
        "- &a\n  - 1\n  - 2\n- *a\n"
    );
}

#[test]
fn shared_mapping_value_round_trips() {
    let shared = Rc::new(Value::Mapping(RefCell::new(vec![(
        Rc::new(Value::String("k".into())),
        Rc::new(Value::Int(1)),
    )])));
    let v = Value::Mapping(RefCell::new(vec![
        (Rc::new(Value::String("x".into())), shared.clone()),
        (Rc::new(Value::String("y".into())), shared),
    ]));
    assert_eq!(to_string(&v).unwrap(), "{x: &a {k: 1}, y: *a}\n");
    for opts in [EmitOptions::default(), block()] {
        let out = to_string_with_options(&v, &opts).unwrap();
        let back = from_str(&out).unwrap_or_else(|e| panic!("{e}\n{out}"));
        let Value::Mapping(entries) = &back else {
            panic!("not a mapping: {out}")
        };
        let entries = entries.borrow();
        assert!(Rc::ptr_eq(&entries[0].1, &entries[1].1), "{out}");
    }
}

#[test]
fn shared_key_alias_is_followed_by_space() {
    let key = Rc::new(Value::String("k".into()));
    let v = seq_of(vec![
        Rc::new(Value::Mapping(RefCell::new(vec![(
            key.clone(),
            Rc::new(Value::Int(1)),
        )]))),
        key,
    ]);
    assert_eq!(to_string(&v).unwrap(), "[{&a k: 1}, *a]\n");

    let s = Rc::new(Value::String("s".into()));
    let v2 = Value::Mapping(RefCell::new(vec![
        (s.clone(), s),
        (Rc::new(Value::String("z".into())), Rc::new(Value::Null)),
    ]));
    let out = to_string(&v2).unwrap();
    assert_eq!(out, "{&a s: *a, z: ~}\n");
    from_str(&out).unwrap();
}

#[test]
fn alias_as_mapping_key_round_trips() {
    let k = Rc::new(Value::String("k".into()));
    let holder = Rc::new(Value::Null);
    let v = seq_of(vec![
        k.clone(),
        Rc::new(Value::Mapping(RefCell::new(vec![(k, holder)]))),
    ]);
    for opts in [EmitOptions::default(), block()] {
        let out = to_string_with_options(&v, &opts).unwrap();
        assert!(out.contains("*a : ~"), "{out}");
        let back = from_str(&out).unwrap_or_else(|e| panic!("{e}\n{out}"));
        assert_eq!(back, v, "{out}");
    }
}

#[test]
fn self_cycle_emits_anchor_and_alias() {
    let outer = Rc::new(seq_of(vec![]));
    if let Value::Sequence(items) = &*outer {
        items.borrow_mut().push(outer.clone());
    }
    assert_eq!(to_string(&outer).unwrap(), "&a [*a]\n");
    assert_eq!(
        to_string_with_options(&outer, &block()).unwrap(),
        "&a\n- *a\n"
    );
    clear(&outer);
}

#[test]
fn nested_cycle_block_round_trip() {
    let inner = Rc::new(seq_of(vec![Rc::new(Value::Int(1))]));
    if let Value::Sequence(items) = &*inner {
        items.borrow_mut().push(inner.clone());
    }
    let v = seq_of(vec![inner.clone()]);
    for opts in [EmitOptions::default(), block()] {
        let out = to_string_with_options(&v, &opts).unwrap();
        let back = from_str(&out).unwrap_or_else(|e| panic!("{e}\n{out}"));
        let child = Rc::clone(&back.as_sequence().unwrap()[0]);
        let items = child.as_sequence().unwrap();
        assert_eq!(*items[0], Value::Int(1), "{out}");
        assert!(Rc::ptr_eq(&child, &items[1]), "{out}");
        drop(items);
        clear(&child);
    }
    clear(&inner);
}

#[test]
fn mutual_cycle_terminates() {
    let a = Rc::new(seq_of(vec![]));
    let b = Rc::new(seq_of(vec![a.clone()]));
    if let Value::Sequence(items) = &*a {
        items.borrow_mut().push(b.clone());
    }
    assert_eq!(to_string(&a).unwrap(), "&a [[*a]]\n");
    clear(&a);
    clear(&b);
}

#[test]
fn many_anchors_get_distinct_names() {
    let shared: Vec<Rc<Value>> = (0..30).map(|i| Rc::new(Value::Int(i))).collect();
    let items: Vec<Rc<Value>> = shared.iter().chain(shared.iter()).cloned().collect();
    let v = seq_of(items);
    let out = to_string(&v).unwrap();
    let back = from_str(&out).unwrap_or_else(|e| panic!("{e}\n{out}"));
    let items = back.as_sequence().unwrap();
    for i in 0..30 {
        assert!(Rc::ptr_eq(&items[i], &items[i + 30]), "{out}");
    }
}
