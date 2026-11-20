"""YAML 1.2.2 load/dump with Unicode anchor support."""

from ozzyyaml._native import (
    OzzyYamlError,
    __version__,
    dump,
    dump_all,
    load,
    load_all,
)

__all__ = [
    "OzzyYamlError",
    "__version__",
    "dump",
    "dump_all",
    "load",
    "load_all",
]
