use ozzyyaml_parser::{parse_str, Error, Event, ScalarStyle};

const STR_TAG: &str = "tag:yaml.org,2002:str";

fn parse(input: &str) -> Vec<Event> {
    parse_str(input).unwrap_or_else(|e| panic!("{input:?}: {e}"))
}

fn scalar(value: &str) -> Event {
    tagged_scalar(None, value)
}

fn tagged_scalar(tag: Option<&str>, value: &str) -> Event {
    Event::Scalar {
        anchor: None,
        tag: tag.map(String::from),
        value: value.into(),
        style: ScalarStyle::Plain,
    }
}

fn doc_start(explicit: bool) -> Event {
    Event::DocumentStart { explicit }
}

fn doc_end(explicit: bool) -> Event {
    Event::DocumentEnd { explicit }
}

#[test]
fn two_explicit_documents_with_tag() {
    let evs = parse("---\n!!str foo\n...\n---\nbar\n");
    assert_eq!(
        evs,
        vec![
            Event::StreamStart,
            doc_start(true),
            tagged_scalar(Some(STR_TAG), "foo"),
            doc_end(true),
            doc_start(true),
            scalar("bar"),
            doc_end(false),
            Event::StreamEnd,
        ]
    );
}

#[test]
fn explicit_start_marker_only() {
    assert_eq!(
        parse("--- foo\n"),
        vec![
            Event::StreamStart,
            doc_start(true),
            scalar("foo"),
            doc_end(false),
            Event::StreamEnd,
        ]
    );
}

#[test]
fn explicit_end_marker_closes_implicit_document() {
    assert_eq!(
        parse("foo\n...\n"),
        vec![
            Event::StreamStart,
            doc_start(false),
            scalar("foo"),
            doc_end(true),
            Event::StreamEnd,
        ]
    );
}

#[test]
fn implicit_document_followed_by_explicit_document() {
    assert_eq!(
        parse("a\n---\nb\n"),
        vec![
            Event::StreamStart,
            doc_start(false),
            scalar("a"),
            doc_end(false),
            doc_start(true),
            scalar("b"),
            doc_end(false),
            Event::StreamEnd,
        ]
    );
}

#[test]
fn bare_document_after_document_end_marker() {
    assert_eq!(
        parse("a\n...\nb\n"),
        vec![
            Event::StreamStart,
            doc_start(false),
            scalar("a"),
            doc_end(true),
            doc_start(false),
            scalar("b"),
            doc_end(false),
            Event::StreamEnd,
        ]
    );
}

#[test]
fn empty_explicit_documents_yield_empty_scalars() {
    assert_eq!(
        parse("---\n---\n"),
        vec![
            Event::StreamStart,
            doc_start(true),
            scalar(""),
            doc_end(false),
            doc_start(true),
            scalar(""),
            doc_end(false),
            Event::StreamEnd,
        ]
    );
}

#[test]
fn block_collections_inside_multiple_documents() {
    let evs = parse("---\na: 1\n---\n- x\n- y\n");
    assert_eq!(
        evs,
        vec![
            Event::StreamStart,
            doc_start(true),
            Event::MappingStart {
                anchor: None,
                tag: None,
                flow: false
            },
            scalar("a"),
            scalar("1"),
            Event::MappingEnd,
            doc_end(false),
            doc_start(true),
            Event::SequenceStart {
                anchor: None,
                tag: None,
                flow: false
            },
            scalar("x"),
            scalar("y"),
            Event::SequenceEnd,
            doc_end(false),
            Event::StreamEnd,
        ]
    );
}

#[test]
fn trailing_document_end_marker_adds_no_document() {
    let evs = parse("a\n...\n...\n");
    assert_eq!(evs.len(), 5);
    assert_eq!(evs.last(), Some(&Event::StreamEnd));
}

#[test]
fn marker_text_that_is_not_a_marker_stays_scalar() {
    assert_eq!(
        parse("----"),
        vec![
            Event::StreamStart,
            doc_start(false),
            scalar("----"),
            doc_end(false),
            Event::StreamEnd,
        ]
    );
}

#[test]
fn local_and_verbatim_tags() {
    assert_eq!(parse("!foo bar")[2], tagged_scalar(Some("!foo"), "bar"));
    assert_eq!(
        parse("!<tag:x,2000:y> z")[2],
        tagged_scalar(Some("tag:x,2000:y"), "z")
    );
    assert_eq!(parse("! z")[2], tagged_scalar(Some("!"), "z"));
}

#[test]
fn tag_suffix_percent_escapes_are_decoded() {
    assert_eq!(parse("!<tag:%41> z")[2], tagged_scalar(Some("tag:A"), "z"));
    assert_eq!(
        parse("!!%73%74%72 v")[2],
        tagged_scalar(Some("tag:yaml.org,2002:str"), "v")
    );
}

#[test]
fn tag_on_collections() {
    let evs = parse("!!map {a: b}");
    assert_eq!(
        evs[2],
        Event::MappingStart {
            anchor: None,
            tag: Some("tag:yaml.org,2002:map".into()),
            flow: true
        }
    );
    let evs = parse("!!seq\n- a\n");
    assert_eq!(
        evs[2],
        Event::SequenceStart {
            anchor: None,
            tag: Some("tag:yaml.org,2002:seq".into()),
            flow: false
        }
    );
}

#[test]
fn anchor_and_tag_in_either_order() {
    let expected = Event::Scalar {
        anchor: Some("a".into()),
        tag: Some(STR_TAG.into()),
        value: "v".into(),
        style: ScalarStyle::Plain,
    };
    assert_eq!(parse("&a !!str v")[2], expected);
    assert_eq!(parse("!!str &a v")[2], expected);
}

#[test]
fn tagged_mapping_key_and_value() {
    let evs = parse("!!str k: !!str v");
    assert_eq!(evs[3], tagged_scalar(Some(STR_TAG), "k"));
    assert_eq!(evs[4], tagged_scalar(Some(STR_TAG), "v"));
}

#[test]
fn tagged_empty_node() {
    let evs = parse("---\n!!str\n...\n");
    assert_eq!(evs[2], tagged_scalar(Some(STR_TAG), ""));
}

#[test]
fn tag_directive_defines_handle() {
    let evs = parse("%TAG !e! tag:example.com,2000:\n---\n!e!foo bar\n");
    assert_eq!(evs[1], doc_start(true));
    assert_eq!(
        evs[2],
        tagged_scalar(Some("tag:example.com,2000:foo"), "bar")
    );
}

#[test]
fn tag_directive_can_redefine_primary_handles() {
    let evs = parse("%TAG ! tag:example.com,2000:\n---\n!foo bar\n");
    assert_eq!(
        evs[2],
        tagged_scalar(Some("tag:example.com,2000:foo"), "bar")
    );
    let evs = parse("%TAG !! tag:example.com,2000:\n---\n!!int 1\n");
    assert_eq!(evs[2], tagged_scalar(Some("tag:example.com,2000:int"), "1"));
}

#[test]
fn tag_directives_do_not_leak_into_next_document() {
    let input = "%TAG !e! tag:example.com,2000:\n---\n!e!a x\n...\n---\n!e!a y\n";
    assert!(matches!(parse_str(input), Err(Error::Parse { .. })));
}

#[test]
fn yaml_directive_is_accepted() {
    let evs = parse("%YAML 1.2\n---\nfoo\n");
    assert_eq!(evs[1], doc_start(true));
    assert_eq!(evs[2], scalar("foo"));
}

#[test]
fn unknown_directive_is_ignored() {
    let evs = parse("%FOO bar baz\n---\nfoo\n");
    assert_eq!(evs[2], scalar("foo"));
}

#[test]
fn directives_per_document_after_end_marker() {
    let evs = parse("a\n...\n%YAML 1.2\n---\nb\n");
    assert_eq!(
        evs.iter()
            .filter(|e| matches!(e, Event::DocumentStart { .. }))
            .count(),
        2
    );
}

#[test]
fn duplicate_yaml_directive_is_error() {
    let input = "%YAML 1.2\n%YAML 1.2\n---\nfoo\n";
    assert!(matches!(parse_str(input), Err(Error::Parse { .. })));
}

#[test]
fn unsupported_yaml_major_version_is_error() {
    assert!(matches!(
        parse_str("%YAML 2.0\n---\nfoo\n"),
        Err(Error::Parse { .. })
    ));
}

#[test]
fn directive_without_document_start_is_error() {
    assert!(matches!(
        parse_str("%YAML 1.2\nfoo\n"),
        Err(Error::Parse { .. })
    ));
}

#[test]
fn undeclared_tag_handle_is_error() {
    assert!(matches!(parse_str("!e!foo bar"), Err(Error::Parse { .. })));
}

#[test]
fn percent_in_column_after_start_is_plain_text() {
    assert_eq!(parse("a %b")[2], scalar("a %b"));
}

#[test]
fn unterminated_verbatim_tag_is_scan_error() {
    assert!(matches!(parse_str("!<abc d"), Err(Error::Scan { .. })));
}
