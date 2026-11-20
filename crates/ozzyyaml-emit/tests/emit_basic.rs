use ozzyyaml::{from_str, Value};
use ozzyyaml_emit::{to_string, to_string_with_options, EmitOptions, EmitStyle};
use std::cell::RefCell;
use std::rc::Rc;

fn seq(items: Vec<Value>) -> Value {
    Value::Sequence(RefCell::new(items.into_iter().map(Rc::new).collect()))
}

fn map(entries: Vec<(Value, Value)>) -> Value {
    Value::Mapping(RefCell::new(
        entries
            .into_iter()
            .map(|(k, v)| (Rc::new(k), Rc::new(v)))
            .collect(),
    ))
}

fn s(text: &str) -> Value {
    Value::String(text.into())
}

fn block() -> EmitOptions {
    EmitOptions {
        style: EmitStyle::Block,
    }
}

#[test]
fn emit_scalars_flow() {
    assert_eq!(to_string(&Value::Null).unwrap().trim(), "~");
    assert_eq!(to_string(&Value::Bool(true)).unwrap().trim(), "true");
    assert_eq!(to_string(&Value::Int(42)).unwrap().trim(), "42");
    assert_eq!(to_string(&Value::String("hi".into())).unwrap().trim(), "hi");
}

#[test]
fn emit_flow_sequence() {
    let v = Value::Sequence(RefCell::new(vec![
        Rc::new(Value::Int(1)),
        Rc::new(Value::Int(2)),
    ]));
    let s = to_string(&v).unwrap();
    assert!(s.contains('[') && s.contains(']'), "{s}");
    assert!(s.contains('1') && s.contains('2'), "{s}");
}

#[test]
fn emit_block_sequence_option() {
    let v = Value::Sequence(RefCell::new(vec![Rc::new(Value::String("a".into()))]));
    let opts = EmitOptions {
        style: EmitStyle::Block,
    };
    let s = to_string_with_options(&v, &opts).unwrap();
    assert!(s.contains("- "), "{s}");
}

#[test]
fn flow_exact_output() {
    let v = map(vec![
        (s("a"), seq(vec![Value::Int(1), Value::Int(2)])),
        (s("b"), Value::Null),
    ]);
    assert_eq!(to_string(&v).unwrap(), "{a: [1, 2], b: ~}\n");
}

#[test]
fn empty_collections() {
    assert_eq!(to_string(&seq(vec![])).unwrap(), "[]\n");
    assert_eq!(to_string(&map(vec![])).unwrap(), "{}\n");
    assert_eq!(
        to_string_with_options(&seq(vec![]), &block()).unwrap(),
        "[]\n"
    );
    assert_eq!(
        to_string_with_options(&map(vec![]), &block()).unwrap(),
        "{}\n"
    );
}

#[test]
fn block_exact_output() {
    let v = map(vec![
        (s("name"), s("x")),
        (
            s("list"),
            seq(vec![Value::Int(1), seq(vec![s("a"), s("b")])]),
        ),
        (
            s("nested"),
            map(vec![(s("k"), Value::Bool(false)), (s("e"), seq(vec![]))]),
        ),
    ]);
    let expected = "\
name: x
list:
  - 1
  - - a
    - b
nested:
  k: false
  e: []
";
    assert_eq!(to_string_with_options(&v, &block()).unwrap(), expected);
}

#[test]
fn block_sequence_of_mappings() {
    let v = seq(vec![
        map(vec![(s("a"), Value::Int(1)), (s("b"), Value::Int(2))]),
        map(vec![(s("c"), Value::Int(3))]),
    ]);
    let expected = "- a: 1\n  b: 2\n- c: 3\n";
    assert_eq!(to_string_with_options(&v, &block()).unwrap(), expected);
}

#[test]
fn floats() {
    assert_eq!(to_string(&Value::Float(1.5)).unwrap(), "1.5\n");
    assert_eq!(to_string(&Value::Float(1.0)).unwrap(), "1.0\n");
    assert_eq!(to_string(&Value::Float(-0.0)).unwrap(), "-0.0\n");
    assert_eq!(to_string(&Value::Float(f64::NAN)).unwrap(), ".nan\n");
    assert_eq!(to_string(&Value::Float(f64::INFINITY)).unwrap(), ".inf\n");
    assert_eq!(
        to_string(&Value::Float(f64::NEG_INFINITY)).unwrap(),
        "-.inf\n"
    );
}

#[test]
fn strings_that_need_quoting() {
    let cases = [
        ("", "''"),
        ("true", "'true'"),
        ("null", "'null'"),
        ("~", "'~'"),
        ("42", "'42'"),
        ("1.5", "'1.5'"),
        (".inf", "'.inf'"),
        ("a: b", "'a: b'"),
        ("a #b", "'a #b'"),
        ("- x", "'- x'"),
        ("[x]", "'[x]'"),
        ("a, b", "'a, b'"),
        (" lead", "' lead'"),
        ("trail ", "'trail '"),
        ("it's", "it's"),
        ("'quoted'", "'''quoted'''"),
        ("a\nb", "\"a\\nb\""),
    ];
    for (input, expected) in cases {
        let out = to_string(&s(input)).unwrap();
        assert_eq!(out, format!("{expected}\n"), "input {input:?}");
    }
}

#[test]
fn roundtrip_flow_and_block() {
    let v = map(vec![
        (s("str"), s("hello world")),
        (s("quoted"), s("a: b")),
        (s("numstr"), s("123")),
        (s("empty"), s("")),
        (s("multi"), s("line1\nline2\ttab")),
        (s("int"), Value::Int(-7)),
        (s("float"), Value::Float(2.5)),
        (
            s("seq"),
            seq(vec![Value::Null, Value::Bool(true), s("x, y")]),
        ),
        (
            s("deep"),
            map(vec![(
                s("a"),
                seq(vec![map(vec![(s("b"), Value::Int(1))])]),
            )]),
        ),
        (Value::Int(5), s("int key")),
    ]);
    for opts in [EmitOptions::default(), block()] {
        let text = to_string_with_options(&v, &opts).unwrap();
        let back = from_str(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        assert_eq!(back, v, "style {:?}\n{text}", opts.style);
    }
}

#[test]
fn collection_keys_use_flow_form() {
    // The in-tree parser cannot yet read collection keys back, so only the
    // emitted text is asserted here.
    let v = map(vec![(seq(vec![Value::Int(1)]), s("v"))]);
    assert_eq!(to_string(&v).unwrap(), "{[1]: v}\n");
    assert_eq!(to_string_with_options(&v, &block()).unwrap(), "[1]: v\n");
}
