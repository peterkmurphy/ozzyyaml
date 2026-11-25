import ozzyyaml


def test_version():
    assert ozzyyaml.__version__


def test_load_scalar():
    assert ozzyyaml.load("42\n") == 42
    assert ozzyyaml.load("true\n") is True
    assert ozzyyaml.load("hello\n") == "hello"
    assert ozzyyaml.load("~\n") is None


def test_load_collection():
    assert ozzyyaml.load("[1, 2]\n") == [1, 2]
    assert ozzyyaml.load("{a: 1}\n") == {"a": 1}


def test_dump_roundtrip():
    data = {"k": [1, True, None]}
    text = ozzyyaml.dump(data)
    assert ozzyyaml.load(text) == data


def test_dump_block_style():
    text = ozzyyaml.dump([1, 2], style="block")
    assert "- " in text


def test_load_all():
    assert ozzyyaml.load_all("1\n---\n2\n") == [1, 2]


def test_dump_all():
    text = ozzyyaml.dump_all([1, 2])
    assert "---" in text
    assert ozzyyaml.load_all(text) == [1, 2]


def test_shared_alias_is_same_object():
    data = ozzyyaml.load("[&a hello, *a]\n")
    assert data[0] is data[1]


def test_cycle_loads_natively():
    # Compose keeps a cyclic root behind an owned shell when Rc::try_unwrap fails;
    # the inner sequence is the self-referential node (same as Rust cyclic_sequence).
    data = ozzyyaml.load("&a [*a]\n")
    assert isinstance(data, list)
    assert len(data) == 1
    inner = data[0]
    assert isinstance(inner, list)
    assert inner[0] is inner


def test_collection_mapping_key_becomes_tuple():
    data = ozzyyaml.load("? [a, b]\n: c\n")
    assert data == {("a", "b"): "c"}


def test_bytes_input():
    assert ozzyyaml.load(b"7\n") == 7


def test_core_yes_is_string():
    assert ozzyyaml.load("yes\n") == "yes"
