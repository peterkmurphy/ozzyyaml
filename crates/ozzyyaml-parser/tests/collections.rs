use ozzyyaml_parser::{Error, Event, LimitKind, Limits, Parser, ScalarStyle};

fn parse_all(input: &str, limits: Limits) -> Result<Vec<Event>, Error> {
    let mut parser = Parser::with_limits(input, limits)?;
    let mut events = Vec::new();
    while let Some(ev) = parser.next_event()? {
        events.push(ev);
    }
    Ok(events)
}

fn parse(input: &str) -> Vec<Event> {
    parse_all(input, Limits::default()).unwrap_or_else(|e| panic!("{input:?}: {e}"))
}

fn scalar(value: &str) -> Event {
    styled(value, ScalarStyle::Plain)
}

fn styled(value: &str, style: ScalarStyle) -> Event {
    Event::Scalar {
        anchor: None,
        tag: None,
        value: value.into(),
        style,
    }
}

fn seq(flow: bool) -> Event {
    Event::SequenceStart {
        anchor: None,
        tag: None,
        flow,
    }
}

fn map(flow: bool) -> Event {
    Event::MappingStart {
        anchor: None,
        tag: None,
        flow,
    }
}

/// Wraps content events in the implicit stream/document events.
fn doc(mut body: Vec<Event>) -> Vec<Event> {
    let mut out = vec![Event::StreamStart, Event::DocumentStart { explicit: false }];
    out.append(&mut body);
    out.push(Event::DocumentEnd { explicit: false });
    out.push(Event::StreamEnd);
    out
}

#[test]
fn block_sequence() {
    assert_eq!(
        parse("- a\n- b\n"),
        doc(vec![
            seq(false),
            scalar("a"),
            scalar("b"),
            Event::SequenceEnd
        ])
    );
}

#[test]
fn flow_mapping() {
    assert_eq!(
        parse("{a: 1, b: 2}"),
        doc(vec![
            map(true),
            scalar("a"),
            scalar("1"),
            scalar("b"),
            scalar("2"),
            Event::MappingEnd
        ])
    );
}

#[test]
fn nested_block_mapping() {
    assert_eq!(
        parse("a:\n  b: c\n"),
        doc(vec![
            map(false),
            scalar("a"),
            map(false),
            scalar("b"),
            scalar("c"),
            Event::MappingEnd,
            Event::MappingEnd
        ])
    );
}

#[test]
fn flow_sequence() {
    assert_eq!(
        parse("[a, 'b', \"c\"]"),
        doc(vec![
            seq(true),
            scalar("a"),
            styled("b", ScalarStyle::SingleQuoted),
            styled("c", ScalarStyle::DoubleQuoted),
            Event::SequenceEnd
        ])
    );
}

#[test]
fn empty_flow_collections() {
    assert_eq!(parse("[]"), doc(vec![seq(true), Event::SequenceEnd]));
    assert_eq!(parse("{}"), doc(vec![map(true), Event::MappingEnd]));
}

#[test]
fn flow_trailing_comma_is_allowed() {
    assert_eq!(
        parse("[a, b,]"),
        doc(vec![
            seq(true),
            scalar("a"),
            scalar("b"),
            Event::SequenceEnd
        ])
    );
    assert_eq!(
        parse("{a: 1,}"),
        doc(vec![map(true), scalar("a"), scalar("1"), Event::MappingEnd])
    );
}

#[test]
fn nested_flow_collections() {
    assert_eq!(
        parse("{a: [1, 2], b: {c: d}}"),
        doc(vec![
            map(true),
            scalar("a"),
            seq(true),
            scalar("1"),
            scalar("2"),
            Event::SequenceEnd,
            scalar("b"),
            map(true),
            scalar("c"),
            scalar("d"),
            Event::MappingEnd,
            Event::MappingEnd
        ])
    );
}

#[test]
fn flow_mapping_missing_values_are_empty() {
    assert_eq!(
        parse("{a, b: , c: 1}"),
        doc(vec![
            map(true),
            scalar("a"),
            scalar(""),
            scalar("b"),
            scalar(""),
            scalar("c"),
            scalar("1"),
            Event::MappingEnd
        ])
    );
}

#[test]
fn flow_mapping_json_style_adjacent_value() {
    assert_eq!(
        parse("{\"a\":1}"),
        doc(vec![
            map(true),
            styled("a", ScalarStyle::DoubleQuoted),
            scalar("1"),
            Event::MappingEnd
        ])
    );
}

#[test]
fn flow_single_pair_in_sequence_becomes_mapping() {
    assert_eq!(
        parse("[a: 1, b]"),
        doc(vec![
            seq(true),
            map(true),
            scalar("a"),
            scalar("1"),
            Event::MappingEnd,
            scalar("b"),
            Event::SequenceEnd
        ])
    );
}

#[test]
fn flow_collection_spans_lines_and_ignores_indentation() {
    assert_eq!(
        parse("a: [1,\n2]\nb: 3\n"),
        doc(vec![
            map(false),
            scalar("a"),
            seq(true),
            scalar("1"),
            scalar("2"),
            Event::SequenceEnd,
            scalar("b"),
            scalar("3"),
            Event::MappingEnd
        ])
    );
}

#[test]
fn flow_collections_inside_block_collections() {
    assert_eq!(
        parse("- [a, b]\n- {c: d}\n"),
        doc(vec![
            seq(false),
            seq(true),
            scalar("a"),
            scalar("b"),
            Event::SequenceEnd,
            map(true),
            scalar("c"),
            scalar("d"),
            Event::MappingEnd,
            Event::SequenceEnd
        ])
    );
}

#[test]
fn block_collections_inside_block_collections() {
    assert_eq!(
        parse("- a: 1\n  b: 2\n- - x\n  - y\n"),
        doc(vec![
            seq(false),
            map(false),
            scalar("a"),
            scalar("1"),
            scalar("b"),
            scalar("2"),
            Event::MappingEnd,
            seq(false),
            scalar("x"),
            scalar("y"),
            Event::SequenceEnd,
            Event::SequenceEnd
        ])
    );
}

#[test]
fn anchors_on_collections() {
    let evs = parse("a: &x [1]\nb: &y {c: d}\n");
    assert!(evs.contains(&Event::SequenceStart {
        anchor: Some("x".into()),
        tag: None,
        flow: true
    }));
    assert!(evs.contains(&Event::MappingStart {
        anchor: Some("y".into()),
        tag: None,
        flow: true
    }));
}

#[test]
fn plain_scalars_in_flow_stop_at_indicators() {
    assert_eq!(
        parse("[a b, c]"),
        doc(vec![
            seq(true),
            scalar("a b"),
            scalar("c"),
            Event::SequenceEnd
        ])
    );
}

#[test]
fn plain_scalars_in_block_keep_flow_indicators() {
    assert_eq!(
        parse("a: x,y]\n"),
        doc(vec![
            map(false),
            scalar("a"),
            scalar("x,y]"),
            Event::MappingEnd
        ])
    );
}

#[test]
fn malformed_flow_collections_are_errors() {
    for input in [
        "[a", "{a: 1", "[a}", "{a: 1]", "[a b", "[a,, b]", "[,]", "]", "}", "[a] b", "[a\n",
    ] {
        assert!(
            parse_all(input, Limits::default()).is_err(),
            "expected error for {input:?}"
        );
    }
}

#[test]
fn nesting_limit_errors() {
    let limits = Limits::default().max_nesting_depth(2);
    let err = parse_all("[[[\n]]]", limits).unwrap_err();
    assert!(
        matches!(
            err,
            Error::LimitExceeded {
                kind: LimitKind::NestingDepth,
                limit: 2,
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn nesting_limit_boundary_allows_exact_depth() {
    let limits = Limits::default().max_nesting_depth(2);
    assert!(parse_all("[[]]", limits).is_ok());
    assert!(parse_all("{a: {b: 1}}", limits).is_ok());
    assert!(parse_all("a:\n  b: 1\n", limits).is_ok());
    assert!(parse_all("- - x\n", limits).is_ok());
}

#[test]
fn nesting_limit_applies_to_block_and_mixed_collections() {
    let limits = Limits::default().max_nesting_depth(2);
    for input in [
        "a:\n  b:\n    c: 1\n",
        "- - - x\n",
        "a: {b: [c]}",
        "- [[x]]",
    ] {
        let err = parse_all(input, limits).unwrap_err();
        assert!(
            matches!(
                err,
                Error::LimitExceeded {
                    kind: LimitKind::NestingDepth,
                    ..
                }
            ),
            "{input:?}: {err:?}"
        );
    }
}

#[test]
fn nesting_limit_error_is_reported_when_event_is_pulled() {
    let limits = Limits::default().max_nesting_depth(1);
    let mut p = Parser::with_limits("[[x]]", limits).unwrap();
    assert_eq!(p.next_event().unwrap(), Some(Event::StreamStart));
    assert_eq!(
        p.next_event().unwrap(),
        Some(Event::DocumentStart { explicit: false })
    );
    assert_eq!(p.next_event().unwrap(), Some(seq(true)));
    assert!(matches!(
        p.next_event(),
        Err(Error::LimitExceeded {
            kind: LimitKind::NestingDepth,
            ..
        })
    ));
}
