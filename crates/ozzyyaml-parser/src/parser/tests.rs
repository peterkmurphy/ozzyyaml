use crate::{parse_str, Event, Mark, Parser};

#[test]
fn parse_plain_scalar_document() {
    let evs = parse_str("hello").unwrap();
    assert_eq!(evs.first(), Some(&Event::StreamStart));
    assert!(evs
        .iter()
        .any(|e| matches!(e, Event::Scalar { value, .. } if value == "hello")));
    assert_eq!(evs.last(), Some(&Event::StreamEnd));
}

#[test]
fn parse_quoted_scalar_keeps_style() {
    use crate::ScalarStyle;
    let evs = parse_str("\"hi\"").unwrap();
    assert!(evs.iter().any(|e| matches!(
        e,
        Event::Scalar { value, style: ScalarStyle::DoubleQuoted, .. } if value == "hi"
    )));
}

#[test]
fn parse_empty_input_has_no_document() {
    assert_eq!(
        parse_str("").unwrap(),
        vec![Event::StreamStart, Event::StreamEnd]
    );
}

#[test]
fn parse_exact_event_sequence() {
    let evs = parse_str("hello").unwrap();
    assert_eq!(evs.len(), 5);
    assert_eq!(evs[1], Event::DocumentStart { explicit: false });
    assert_eq!(evs[3], Event::DocumentEnd { explicit: false });
}

#[test]
fn parse_second_scalar_is_error() {
    assert!(matches!(parse_str("a\nb"), Err(crate::Error::Parse { .. })));
}

#[test]
fn next_event_returns_none_after_stream_end() {
    let mut p = crate::Parser::new("x").unwrap();
    while p.next_event().unwrap().is_some() {}
    assert_eq!(p.next_event().unwrap(), None);
}

#[test]
fn last_mark_tracks_alias_position() {
    // `&a x\n*a\n` is a second root node, so this parser returns a parse error
    // and never emits Alias. A block sequence keeps that alias on line 2.
    let mut p = Parser::new("- &a x\n- *a\n").unwrap();
    assert_eq!(p.last_mark(), Mark::default());
    while let Some(ev) = p.next_event().unwrap() {
        if matches!(ev, Event::Alias { .. }) {
            let m = p.last_mark();
            assert!(m.line >= 2, "alias is on line 2, got {m}");
            return;
        }
    }
    panic!("expected Alias event");
}
