# ozzyyaml

Build YAML **1.2.2** documents into a shared [`Value`](https://docs.rs/ozzyyaml/latest/ozzyyaml/enum.Value.html)
graph using the Core schema.

This is the compose layer of the
[ozzyyaml](https://github.com/peterkmurphy/ozzyyaml) stack: it consumes events
from [`ozzyyaml-parser`](https://crates.io/crates/ozzyyaml-parser) and gives you
something you can inspect, share, and (with
[`ozzyyaml-emit`](https://crates.io/crates/ozzyyaml-emit)) write back out.

Licensed under the BSD-3-Clause.

## Why this exists

Everyday YAML tools still miss pieces of 1.2.2 that show up in real documents.
Unicode anchor names are the headline example — libyaml rejects them;
ozzyyaml accepts `&汉` and friends. The compose layer also keeps alias
**identity** instead of exploding shared structure into deep copies, so
cycles such as `&a [*a]` are representable.

## Quick start

```rust
use ozzyyaml::{from_str, Value};

fn main() {
    let value = from_str("{name: Ada, ok: true}\n").expect("valid YAML");
    match value {
        Value::Mapping(entries) => {
            println!("{} entries", entries.borrow().len());
        }
        other => println!("{other:?}"),
    }
}
```

Multi-document streams:

```rust
use ozzyyaml::load_all;

let docs = load_all("1\n---\n2\n").unwrap();
assert_eq!(docs.len(), 2);
```

Each loader has a `*_with_limits` twin when you need tighter caps than the
defaults.

## Values, aliases, and cycles

Sequences and mappings store children behind `Rc` and `RefCell`. An alias is
an `Rc` clone of the anchored node — the same pointer, not a duplicate tree.
That is how ozzyyaml avoids classic “billion laughs” blow-ups from aggressive
expansion, and how cyclic graphs stay honest.

Under the **Core** schema, plain scalars become null, bool, int, float, or
string by the usual Core rules. Quoted scalars stay strings unless an explicit
Core tag says otherwise. Values such as `yes` / `no` / `on` / `off` remain
strings (they are not booleans in Core).

## Limits

[`Limits`](https://docs.rs/ozzyyaml/latest/ozzyyaml/struct.Limits.html) carries:

| Limit | When it bites |
| --- | --- |
| `max_anchors` | Compose time — primary DoS brake |
| `max_nesting_depth` | Parse time |
| `max_input_bytes` | Read time |

## In the wider stack

```
YAML text → ozzyyaml-parser (events) → ozzyyaml (Value) → ozzyyaml-emit (text)
```

For a Python façade with the same Core behaviour, see the
[`ozzyyaml`](https://pypi.org/project/ozzyyaml/) package on PyPI.

## Learn more

- [Architecture overview](https://github.com/peterkmurphy/ozzyyaml/blob/main/docs/architecture.md)
- [Repository](https://github.com/peterkmurphy/ozzyyaml)
- [API docs](https://docs.rs/ozzyyaml)
