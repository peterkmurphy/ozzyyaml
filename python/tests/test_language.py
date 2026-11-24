"""Language features that landed in the Rust conformance track (§7–§8)."""

import pytest

import ozzyyaml


def test_literal_block_scalar():
    assert ozzyyaml.load("|\n  hello\n  world\n") == "hello\nworld\n"


def test_folded_block_scalar():
    assert ozzyyaml.load(">\n  hello\n  world\n") == "hello world\n"


def test_multiline_plain_folding():
    assert ozzyyaml.load("a\n b\n") == "a b"


def test_explicit_key():
    assert ozzyyaml.load("? a\n: b\n") == {"a": "b"}


def test_explicit_key_empty_value():
    assert ozzyyaml.load("? a\n") == {"a": None}


def test_double_quoted_escapes():
    assert ozzyyaml.load('"\\x41\\u0042\\U00000043"\n') == "ABC"
    assert ozzyyaml.load('"\\n\\t\\\\"\n') == "\n\t\\"


def test_strict_int_tag_rejects_non_int():
    with pytest.raises(ozzyyaml.OzzyYamlError, match="int|tag"):
        ozzyyaml.load("!!int twelve\n")


def test_strict_int_tag_accepts_int():
    assert ozzyyaml.load("!!int 42\n") == 42


def test_strict_bool_tag_rejects_yes():
    with pytest.raises(ozzyyaml.OzzyYamlError, match="bool|tag"):
        ozzyyaml.load("!!bool yes\n")


def test_strict_str_tag_keeps_text():
    assert ozzyyaml.load("!!str twelve\n") == "twelve"


def test_load_value_block_scalar():
    v = ozzyyaml.load_value("|\n  hi\n")
    assert v.kind == "str"
    assert v.as_str() == "hi\n"
