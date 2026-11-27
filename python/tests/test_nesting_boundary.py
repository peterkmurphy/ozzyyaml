"""Default nesting depth must survive load_value → dump → load (no segfault)."""

import ozzyyaml

DEFAULT_DEPTH = 512


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
