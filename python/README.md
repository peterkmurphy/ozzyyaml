# ozzyyaml (Python)

Python bindings for the ozzyyaml YAML 1.2.2 stack.

See the repository root [README](../README.md) and [Python guide](../docs/python.md)
for usage. Build locally with [Maturin](https://www.maturin.rs/):

```bash
cd python
python3 -m venv .venv
source .venv/bin/activate
pip install maturin pytest
maturin develop
pytest
```
