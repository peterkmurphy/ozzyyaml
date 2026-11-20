use ozzyyaml::{from_str, load_all, Value};
use ozzyyaml_emit::{
    dump_all, dump_all_with_options, to_string, to_string_with_options, EmitOptions, EmitStyle,
};
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

#[test]
fn dump_all_two_docs() {
    let docs = vec![Value::Int(1), Value::Int(2)];
    let s = dump_all(&docs).unwrap();
    assert!(s.contains("---"), "{s}");
    let back = load_all(&s).unwrap();
    assert_eq!(back, docs);
}

#[test]
fn dump_all_resets_anchors_per_doc() {
    let a = from_str("[&x 1, *x]\n").unwrap();
    let b = from_str("[&x 2, *x]\n").unwrap();
    let s = dump_all(&[a, b]).unwrap();
    let docs = load_all(&s).unwrap();
    assert_eq!(docs.len(), 2);
}

#[test]
fn multi_doc_prefixes_every_document_including_the_first() {
    let docs = vec![Value::Int(1), Value::Int(2), Value::Int(3)];
    let s = dump_all(&docs).unwrap();
    assert_eq!(s, "---\n1\n---\n2\n---\n3\n");
    assert_eq!(load_all(&s).unwrap(), docs);
}

#[test]
fn single_doc_matches_to_string() {
    let doc = Value::String("only".into());
    let dumped = dump_all(std::slice::from_ref(&doc)).unwrap();
    assert_eq!(dumped, to_string(&doc).unwrap());
    assert!(!dumped.contains("---"));
}

#[test]
fn empty_stream_is_empty_string() {
    assert_eq!(dump_all(&[]).unwrap(), "");
}

#[test]
fn shared_rc_across_documents_is_reanchored() {
    let shared = Rc::new(Value::Int(1));
    let docs = vec![
        seq_of(vec![Rc::clone(&shared), Rc::clone(&shared)]),
        seq_of(vec![Rc::clone(&shared), Rc::clone(&shared)]),
    ];
    let s = dump_all(&docs).unwrap();
    assert_eq!(s, "---\n[&a 1, *a]\n---\n[&a 1, *a]\n");
    let back = load_all(&s).unwrap();
    assert_eq!(back.len(), 2);
    for doc in &back {
        let seq = doc.as_sequence().unwrap();
        assert!(Rc::ptr_eq(&seq[0], &seq[1]));
        assert_eq!(seq[0].as_i64(), Some(1));
    }
}

#[test]
fn dump_all_with_options_uses_block_style() {
    let docs = vec![
        seq_of(vec![Rc::new(Value::String("a".into()))]),
        seq_of(vec![Rc::new(Value::String("b".into()))]),
    ];
    let s = dump_all_with_options(&docs, &block()).unwrap();
    assert_eq!(s, "---\n- a\n---\n- b\n");
    assert_eq!(load_all(&s).unwrap(), docs);
}

#[test]
fn single_doc_with_options_matches_to_string_with_options() {
    let doc = seq_of(vec![Rc::new(Value::Int(7))]);
    let opts = block();
    assert_eq!(
        dump_all_with_options(std::slice::from_ref(&doc), &opts).unwrap(),
        to_string_with_options(&doc, &opts).unwrap()
    );
}
