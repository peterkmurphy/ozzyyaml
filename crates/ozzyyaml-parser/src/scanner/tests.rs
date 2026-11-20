use super::*;
use crate::error::Error;
use crate::event::ScalarStyle;
use crate::limits::Limits;

fn scanner(input: &str) -> Scanner<'_> {
    Scanner::new(Reader::new(input, Limits::default()).unwrap())
}

fn scalar(s: &mut Scanner<'_>) -> (String, ScalarStyle) {
    match s.next_token().unwrap().expect("token") {
        Token::Scalar { value, style, .. } => (value, style),
        other => panic!("expected scalar, got {other:?}"),
    }
}

#[test]
fn scans_plain_scalar() {
    let mut s = scanner("hello");
    assert_eq!(scalar(&mut s), ("hello".into(), ScalarStyle::Plain));
    assert_eq!(s.next_token().unwrap(), None);
}

#[test]
fn scans_double_quoted_scalar() {
    let mut s = scanner("\"hello\"");
    assert_eq!(scalar(&mut s), ("hello".into(), ScalarStyle::DoubleQuoted));
    assert_eq!(s.next_token().unwrap(), None);
}

#[test]
fn scans_single_quoted_scalar() {
    let mut s = scanner("'hello'");
    assert_eq!(scalar(&mut s), ("hello".into(), ScalarStyle::SingleQuoted));
    assert_eq!(s.next_token().unwrap(), None);
}

#[test]
fn skips_leading_spaces_and_records_mark() {
    let mut s = scanner("  hi");
    match s.next_token().unwrap().unwrap() {
        Token::Scalar { mark, .. } => {
            assert_eq!((mark.index, mark.line, mark.column), (2, 1, 3));
        }
        other => panic!("expected scalar, got {other:?}"),
    }
}

#[test]
fn skips_comments_and_blank_lines() {
    let mut s = scanner("# note\n\n  hi # trailing\n");
    assert_eq!(scalar(&mut s), ("hi".into(), ScalarStyle::Plain));
    assert_eq!(s.next_token().unwrap(), None);
}

#[test]
fn plain_scalar_keeps_inner_spaces_and_hash_without_space() {
    let mut s = scanner("a b#c");
    assert_eq!(scalar(&mut s), ("a b#c".into(), ScalarStyle::Plain));
}

#[test]
fn empty_and_blank_input_yield_no_tokens() {
    assert_eq!(scanner("").next_token().unwrap(), None);
    assert_eq!(scanner("  # only a comment").next_token().unwrap(), None);
}

#[test]
fn single_quote_doubled_is_literal_quote() {
    let mut s = scanner("'it''s'");
    assert_eq!(scalar(&mut s), ("it's".into(), ScalarStyle::SingleQuoted));
}

#[test]
fn double_quoted_handles_simple_escapes() {
    let mut s = scanner(r#""a\tb\n\"c\\""#);
    assert_eq!(
        scalar(&mut s),
        ("a\tb\n\"c\\".into(), ScalarStyle::DoubleQuoted)
    );
}

#[test]
fn unterminated_quoted_scalars_are_scan_errors() {
    for input in ["'abc", "\"abc"] {
        let err = scanner(input).next_token().unwrap_err();
        assert!(matches!(err, Error::Scan { .. }), "{input}: {err:?}");
    }
}

fn tokens(input: &str) -> Vec<Token> {
    let mut s = scanner(input);
    let mut out = Vec::new();
    while let Some(t) = s.next_token().unwrap() {
        out.push(t);
    }
    out
}

fn kinds(input: &str) -> Vec<String> {
    tokens(input)
        .into_iter()
        .map(|t| match t {
            Token::Scalar { value, .. } => format!("S:{value}"),
            Token::Anchor { name, .. } => format!("&{name}"),
            Token::Alias { name, .. } => format!("*{name}"),
            other => format!("{other:?}").split(' ').next().unwrap().to_string(),
        })
        .collect()
}

#[test]
fn scans_unicode_and_emoji_anchor_names() {
    assert_eq!(kinds("&汉 x"), ["&汉", "S:x"]);
    assert_eq!(kinds("*😀"), ["*😀"]);
    assert_eq!(kinds("*a,b"), ["*a", "S:,b"]);
}

#[test]
fn empty_anchor_name_is_scan_error() {
    assert!(matches!(
        scanner("& x").next_token(),
        Err(Error::Scan { .. })
    ));
}

#[test]
fn scans_block_mapping_tokens() {
    assert_eq!(
        kinds("a: 1\nb: &x 2\n"),
        [
            "BlockMappingStart",
            "Key",
            "S:a",
            "Value",
            "S:1",
            "Key",
            "S:b",
            "Value",
            "&x",
            "S:2",
            "BlockEnd"
        ]
    );
}

#[test]
fn scans_block_sequence_tokens() {
    assert_eq!(
        kinds("- a\n- *x\n"),
        [
            "BlockSequenceStart",
            "BlockEntry",
            "S:a",
            "BlockEntry",
            "*x",
            "BlockEnd"
        ]
    );
}

#[test]
fn nested_collections_close_in_order() {
    assert_eq!(
        kinds("a:\n  b: c\nd: e"),
        [
            "BlockMappingStart",
            "Key",
            "S:a",
            "Value",
            "BlockMappingStart",
            "Key",
            "S:b",
            "Value",
            "S:c",
            "BlockEnd",
            "Key",
            "S:d",
            "Value",
            "S:e",
            "BlockEnd"
        ]
    );
}

#[test]
fn colon_without_following_blank_stays_in_plain_scalar() {
    assert_eq!(kinds("a:b"), ["S:a:b"]);
}

#[test]
fn invalid_double_quote_escape_is_scan_error() {
    let err = scanner(r#""\q""#).next_token().unwrap_err();
    assert!(matches!(err, Error::Scan { .. }));
}

#[test]
fn scans_tag_forms() {
    let tags = |input: &str| match scanner(input).next_token().unwrap().unwrap() {
        Token::Tag { handle, suffix, .. } => (handle, suffix),
        other => panic!("expected tag, got {other:?}"),
    };
    assert_eq!(tags("!!str"), ("!!".into(), "str".into()));
    assert_eq!(tags("!e!foo"), ("!e!".into(), "foo".into()));
    assert_eq!(tags("!foo"), ("!".into(), "foo".into()));
    assert_eq!(tags("!"), ("!".into(), "".into()));
    assert_eq!(tags("!<tag:x,1:y>"), ("".into(), "tag:x,1:y".into()));
}

#[test]
fn scans_document_markers_and_directives() {
    let mut s = scanner("%YAML 1.2\n---\na\n...\n");
    assert!(matches!(
        s.next_token().unwrap(),
        Some(Token::Directive { name, args, .. }) if name == "YAML" && args == ["1.2"]
    ));
    assert!(matches!(
        s.next_token().unwrap(),
        Some(Token::DocumentStart { .. })
    ));
    assert_eq!(scalar(&mut s), ("a".into(), ScalarStyle::Plain));
    assert!(matches!(
        s.next_token().unwrap(),
        Some(Token::DocumentEnd { .. })
    ));
    assert_eq!(s.next_token().unwrap(), None);
}
