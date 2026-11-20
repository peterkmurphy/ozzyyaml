import ozzyyaml


def test_load_value_scalar():
    v = ozzyyaml.load_value("42\n")
    assert v.kind == "int"
    assert v.as_int() == 42


def test_alias_identity():
    v = ozzyyaml.load_value("[&a hello, *a]\n")
    assert v.kind == "seq"
    assert len(v) == 2
    assert v[0].same(v[1])
    assert v[0] is v[1]


def test_cycle_identity():
    v = ozzyyaml.load_value("&a [*a]\n")
    child = v[0]
    assert child.same(child[0])
    assert child is child[0]


def test_dump_value_roundtrip_sharing():
    v = ozzyyaml.load_value("[&a x, *a]\n")
    text = ozzyyaml.dump_value(v)
    v2 = ozzyyaml.load_value(text)
    assert v2[0] is v2[1]
    assert v2[0].as_str() == "x"


def test_cycle_roundtrip():
    v = ozzyyaml.load_value("&a [*a]\n")
    text = ozzyyaml.dump_value(v)
    v2 = ozzyyaml.load_value(text)
    assert v2[0] is v2[0][0]


def test_unicode_anchor_value():
    v = ozzyyaml.load_value("[&汉 x, *汉]\n")
    assert v[0] is v[1]


def test_mapping_lookup():
    v = ozzyyaml.load_value("{a: 1, b: two}\n")
    assert v["a"].as_int() == 1
    assert v["b"].as_str() == "two"


def test_load_all_value():
    docs = ozzyyaml.load_all_value("1\n---\n2\n")
    assert len(docs) == 2
    assert docs[0].as_int() == 1
    assert docs[1].as_int() == 2
