use ozzyyaml_parser::{parse_str, Event};

#[test]
fn unicode_anchor_and_alias_round_trip_events() {
    let yaml = "foo: &汉 bar\nbaz: *汉\n";
    let evs = parse_str(yaml).expect("parse");
    let has_anchor = evs
        .iter()
        .any(|e| matches!(e, Event::Scalar { anchor: Some(a), .. } if a == "汉"));
    let has_alias = evs
        .iter()
        .any(|e| matches!(e, Event::Alias { anchor } if anchor == "汉"));
    assert!(has_anchor, "expected unicode anchor, got {evs:?}");
    assert!(has_alias, "expected unicode alias, got {evs:?}");
}

#[test]
fn emoji_anchor_name() {
    let yaml = "- &😀 value\n- *😀\n";
    parse_str(yaml).expect("emoji anchors must parse under YAML 1.2.2");
}

#[test]
fn unicode_mapping_exact_events() {
    use ozzyyaml_parser::ScalarStyle::Plain;
    let evs = parse_str("foo: &汉 bar\nbaz: *汉\n").unwrap();
    let scalar = |anchor: Option<&str>, value: &str| Event::Scalar {
        anchor: anchor.map(Into::into),
        tag: None,
        value: value.into(),
        style: Plain,
    };
    assert_eq!(
        evs,
        vec![
            Event::StreamStart,
            Event::DocumentStart { explicit: false },
            Event::MappingStart {
                anchor: None,
                tag: None,
                flow: false
            },
            scalar(None, "foo"),
            scalar(Some("汉"), "bar"),
            scalar(None, "baz"),
            Event::Alias {
                anchor: "汉".into()
            },
            Event::MappingEnd,
            Event::DocumentEnd { explicit: false },
            Event::StreamEnd,
        ]
    );
}

#[test]
fn emoji_sequence_has_anchor_and_alias() {
    let evs = parse_str("- &😀 value\n- *😀\n").unwrap();
    assert!(evs
        .iter()
        .any(|e| matches!(e, Event::Scalar { anchor: Some(a), value, .. } if a == "😀" && value == "value")));
    assert!(evs.contains(&Event::Alias {
        anchor: "😀".into()
    }));
    assert!(evs.contains(&Event::SequenceEnd));
}

#[test]
fn unicode_anchor_on_mapping_key_and_ascii_anchor() {
    let evs = parse_str("&é key: &a v\nother: *é\n").unwrap();
    assert!(evs.iter().any(
        |e| matches!(e, Event::Scalar { anchor: Some(a), value, .. } if a == "é" && value == "key")
    ));
    assert!(evs.contains(&Event::Alias {
        anchor: "é".into()
    }));
}

#[test]
fn empty_anchor_name_is_error() {
    assert!(parse_str("a: & b\n").is_err());
    assert!(parse_str("a: *\n").is_err());
}
