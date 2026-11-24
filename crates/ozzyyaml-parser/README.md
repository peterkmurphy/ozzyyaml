# ozzyyaml-parser

Turn YAML **1.2.2–oriented** text into a stream of parse events — the first
stage of the [ozzyyaml](https://github.com/peterkmurphy/ozzyyaml) stack.
Coverage is incomplete; see
[conformance status](https://github.com/peterkmurphy/ozzyyaml/blob/main/docs/conformance.md).

This crate does not build a document tree. It scans and parses, then hands you
events you can feed into a composer (see the [`ozzyyaml`](https://crates.io/crates/ozzyyaml)
crate) or into your own tooling.

Licensed under the BSD-3-Clause.

## Why this exists

Common YAML stacks still stumble on parts of the 1.2.2 spec. In particular,
**libyaml** (and tools built on it) rejects Unicode characters in anchor and
alias names. ozzyyaml-parser accepts them on purpose: `&汉`, `*😀`, and similar
are valid here.

## Quick start

```rust
use ozzyyaml_parser::{parse_str, Event};

fn main() {
    let events = parse_str("[1, 2]\n").expect("valid YAML");
    for event in &events {
        println!("{event:?}");
    }
    assert!(matches!(events[0], Event::StreamStart));
}
```

For streaming control, construct a [`Parser`](https://docs.rs/ozzyyaml-parser/latest/ozzyyaml_parser/struct.Parser.html)
and call `next_event()` yourself. `Parser::last_mark` remembers the source
position of the most recent successful event — useful when a later stage
reports an error.

## Limits

Parsing is bounded by [`Limits`](https://docs.rs/ozzyyaml-parser/latest/ozzyyaml_parser/struct.Limits.html):

- `max_input_bytes` — raw input size
- `max_nesting_depth` — how deep collections may nest
- `max_anchors` — reserved for compose; the parser carries the same struct for
  a shared knobs story across the stack

Crossing a limit returns a typed error instead of hanging or allocating without
bound.

## What you get

| Piece | Role |
| --- | --- |
| `parse_str` | Drain a whole string into `Vec<Event>` |
| `Parser` | Pull events one at a time |
| `Event` / `ScalarStyle` | The event vocabulary |
| `Mark` | Byte index plus 1-based line and column |
| `Error` / `Limits` | Failures and resource caps |

Tag handles from `%TAG` directives are expanded onto the events. Schema typing
(null / bool / int / …) happens in the composer, not here.

## In the wider stack

```
YAML text → ozzyyaml-parser (events) → ozzyyaml (Value) → ozzyyaml-emit (text)
```

Python bindings live in the same repository and publish separately on PyPI as
[`ozzyyaml`](https://pypi.org/project/ozzyyaml/).

## Learn more

- [Architecture overview](https://github.com/peterkmurphy/ozzyyaml/blob/main/docs/architecture.md)
- [Repository](https://github.com/peterkmurphy/ozzyyaml)
- [API docs](https://docs.rs/ozzyyaml-parser)
