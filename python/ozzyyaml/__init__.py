"""YAML 1.2.2 load/dump with Unicode anchor support."""

from ozzyyaml._native import (
    OzzyYamlError,
    Value,
    __version__,
    dump,
    dump_all,
    dump_value,
    load,
    load_all,
    load_all_value,
    load_value,
)

__all__ = [
    "OzzyYamlError",
    "Value",
    "__version__",
    "dump",
    "dump_all",
    "dump_value",
    "load",
    "load_all",
    "load_all_value",
    "load_value",
]
