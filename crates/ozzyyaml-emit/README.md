# ozzyyaml-emit

Write [`ozzyyaml::Value`](https://docs.rs/ozzyyaml/latest/ozzyyaml/enum.Value.html)
graphs back out as YAML **1.2.2** text.

This is the emit side of the
[ozzyyaml](https://github.com/peterkmurphy/ozzyyaml) stack. You compose (or
build) a `Value`, then ask this crate for a string. Shared nodes become
anchors and aliases again — including cycles.

Licensed under the BSD-3-Clause.

## Quick start

```rust
use ozzyyaml::from_str;
use ozzyyaml_emit::to_string;

fn main() {
    let value = from_str("[1, 2]\n").unwrap();
    let yaml = to_string(&value).unwrap();
    println!("{yaml}"); // flow style by default: [1, 2]
}
```

Prefer block style?

```rust
use ozzyyaml_emit::{to_string_with_options, EmitOptions, EmitStyle};

let opts = EmitOptions {
    style: EmitStyle::Block,
};
let yaml = to_string_with_options(&value, &opts).unwrap();
```

Multi-document streams use `dump_all` / `dump_all_with_options`. Several
documents each get a `---` marker (including the first). A single document
matches `to_string` (no `---`). An empty slice yields an empty string.

## How sharing is preserved

Emit walks the graph and keys shared structure by **`Rc` identity**. The first
time a node is seen it may receive a generated ASCII anchor (`a`, `b`, …); later
visits become aliases. That means a cycle round-trips without trying to print
forever.

Dumping is value-level: comments, original tags, and exact source layout are
not preserved. One style (flow or block) applies to the whole dump.

## In the wider stack

```
YAML text → ozzyyaml-parser (events) → ozzyyaml (Value) → ozzyyaml-emit (text)
```

Unicode anchors are first-class on the parse side; the emitter can write
Unicode anchor spellings when you give it those names, and otherwise generates
simple ASCII anchors for shared nodes.

## Learn more

- [Architecture overview](https://github.com/peterkmurphy/ozzyyaml/blob/main/docs/architecture.md)
- [Repository](https://github.com/peterkmurphy/ozzyyaml)
- [API docs](https://docs.rs/ozzyyaml-emit)
