"""Default nesting depth must survive load_value → dump → load (no segfault)."""

import threading

import ozzyyaml

DEFAULT_DEPTH = 512
# Target depth after iterative Python converters (Task 5); default limit stays 512
# until Task 6 re-raises Limits::default().
RAISED_DEPTH = 10_000
SMALL_STACK = 1024 * 1024


def flow_seq(depth: int) -> str:
    return "[" * depth + "]" * depth + "\n"


def flow_map(depth: int) -> str:
    return "{a: " * depth + "1" + "}" * depth + "\n"


def block_seq(depth: int) -> str:
    parts = []
    for i in range(depth):
        indent = "  " * i
        if i + 1 == depth:
            parts.append(f"{indent}- x\n")
        else:
            parts.append(f"{indent}-\n")
    return "".join(parts)


def roundtrip_value(text: str) -> None:
    value = ozzyyaml.load_value(text)
    dumped = ozzyyaml.dump_value(value)
    ozzyyaml.load(dumped)


def nested_python_list(depth: int):
    node = None
    for _ in range(depth):
        node = [node]
    return node


def test_flow_seq_at_default_depth():
    roundtrip_value(flow_seq(DEFAULT_DEPTH))
    roundtrip_value(flow_seq(DEFAULT_DEPTH - 1))


def test_flow_map_at_default_depth():
    roundtrip_value(flow_map(DEFAULT_DEPTH))
    roundtrip_value(flow_map(DEFAULT_DEPTH - 1))


def test_block_seq_at_default_depth():
    roundtrip_value(block_seq(DEFAULT_DEPTH))
    roundtrip_value(block_seq(DEFAULT_DEPTH - 1))


def test_one_past_default_is_error():
    for maker in (flow_seq, flow_map):
        try:
            ozzyyaml.load(maker(DEFAULT_DEPTH + 1))
            raise AssertionError("expected OzzyYamlError for over-deep nesting")
        except ozzyyaml.OzzyYamlError:
            pass


def _on_small_stack(fn):
    """Run `fn` on a 1 MiB thread so Rust recursion cannot hide behind the default stack."""
    errors = []

    def target():
        try:
            fn()
        except Exception as exc:  # noqa: BLE001 — surface any failure to the joiner
            errors.append(exc)

    prev = threading.stack_size(SMALL_STACK)
    try:
        thread = threading.Thread(target=target)
        thread.start()
        thread.join()
    finally:
        threading.stack_size(prev)
    if thread.is_alive():
        raise AssertionError("small-stack worker did not finish")
    if errors:
        raise errors[0]


def test_load_at_raised_depth_on_small_stack():
    """value_to_py must not recurse with depth (compose is already iterative)."""

    def body():
        text = flow_seq(RAISED_DEPTH)
        data = ozzyyaml.load(text, max_nesting_depth=RAISED_DEPTH + 1)
        cur = data
        for _ in range(RAISED_DEPTH - 1):
            assert isinstance(cur, list) and len(cur) == 1
            cur = cur[0]
        assert cur == []

    _on_small_stack(body)


def test_dump_python_list_at_raised_depth_on_small_stack():
    """py_to_value must not recurse with depth."""

    def body():
        data = nested_python_list(RAISED_DEPTH)
        text = ozzyyaml.dump(data)
        assert text.startswith("[")
        loaded = ozzyyaml.load(text, max_nesting_depth=RAISED_DEPTH + 1)
        cur = loaded
        for _ in range(RAISED_DEPTH - 1):
            assert isinstance(cur, list) and len(cur) == 1
            cur = cur[0]
        assert cur == [None]

    _on_small_stack(body)


def test_load_flow_map_at_raised_depth_on_small_stack():
    def body():
        text = flow_map(RAISED_DEPTH)
        data = ozzyyaml.load(text, max_nesting_depth=RAISED_DEPTH + 1)
        cur = data
        for _ in range(RAISED_DEPTH):
            assert isinstance(cur, dict) and list(cur.keys()) == ["a"]
            cur = cur["a"]
        assert cur == 1

    _on_small_stack(body)
