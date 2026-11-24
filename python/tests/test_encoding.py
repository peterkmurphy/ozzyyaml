"""Byte-stream encodings via load(bytes) — YAML 1.2.2 §5.2."""

import ozzyyaml


def _utf16le(text: str) -> bytes:
    return b"\xff\xfe" + text.encode("utf-16-le")


def _utf16be(text: str) -> bytes:
    return b"\xfe\xff" + text.encode("utf-16-be")


def _utf32le(text: str) -> bytes:
    return b"\xff\xfe\x00\x00" + text.encode("utf-32-le")


def _utf32be(text: str) -> bytes:
    return b"\x00\x00\xfe\xff" + text.encode("utf-32-be")


def test_utf8_bytes_still_work():
    assert ozzyyaml.load(b"7\n") == 7


def test_utf8_bom_bytes():
    assert ozzyyaml.load(b"\xef\xbb\xbf42\n") == 42


def test_utf16le_bom_bytes():
    assert ozzyyaml.load(_utf16le("a\n")) == "a"


def test_utf16be_bom_bytes():
    assert ozzyyaml.load(_utf16be("true\n")) is True


def test_utf32le_bom_bytes():
    assert ozzyyaml.load(_utf32le("[1, 2]\n")) == [1, 2]


def test_utf32be_bom_bytes():
    assert ozzyyaml.load(_utf32be("k: v\n")) == {"k": "v"}


def test_load_value_utf16le_bom_bytes():
    v = ozzyyaml.load_value(_utf16le("hello\n"))
    assert v.kind == "str"
    assert v.as_str() == "hello"


def test_load_all_utf16le_bom_bytes():
    assert ozzyyaml.load_all(_utf16le("1\n---\n2\n")) == [1, 2]


def test_utf16le_null_pattern_without_bom():
    assert ozzyyaml.load(bytes([0x61, 0x00, 0x0A, 0x00])) == "a"
