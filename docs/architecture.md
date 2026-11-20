# Architecture

This document describes how ozzyyaml turns YAML text into structured data and
back again. It is meant to be readable on its own; you do not need the agent
planning docs to follow it.

## Big picture

ozzyyaml splits the work into clear stages:

```
YAML text (UTF-8)
  → Reader        — bytes, line/column marks, size limit
  → Scanner       — tokens (scalars, anchors, collection markers, …)
  → Parser        — YAML 1.2.2 events
  → Composer      — Core-schema Value graph (shared with Rc)
  → Emitter       — YAML text again
```

Each stage lives in a focused crate or module. Lower layers stay private where
possible so the public API stays small.

| Layer | Crate | Public highlights |
| --- | --- | --- |
| Events | `ozzyyaml-parser` | `Parser`, `parse_str`, `Event`, `Limits`, `Error`, `Mark` |
| Documents | `ozzyyaml` | `from_str` / `load_all` / `from_slice*`, `Value`, `Limits`, `Error` |
| Dump | `ozzyyaml-emit` | `to_string` / `dump_all`, `EmitOptions`, `EmitStyle`, `Error` |

## Reader

The reader is a UTF-8 cursor over the input string. It:

- Rejects input longer than `Limits::max_input_bytes`
- Skips a leading UTF-8 BOM if present
- Normalizes `\r\n` and lone `\r` to a logical newline
- Tracks a source position (`Mark`: byte index, 1-based line and column)

It does not understand YAML syntax. It only feeds characters to the scanner.

## Scanner

The scanner turns characters into tokens: plain and quoted scalars, anchors,
aliases, tags, directives, document markers (`---` / `...`), and collection
structure (block `-` entries, `key:` / value indicators, flow `[` `{` `]` `}`,
and indent-aware block start/end tokens).

Indentation matters in block style and is ignored inside flow collections.
Anchor and alias names follow YAML 1.2.2 rules and may include non-ASCII
characters. That is intentional; see [Unicode anchors](#unicode-anchors).

## Parser

The parser is a pull state machine. Call `next_event()` to receive events such
as stream/document boundaries, scalars, aliases, and sequence/mapping
start/end markers.

Nesting depth is capped by `Limits::max_nesting_depth`. Tag handles from
`%TAG` directives are expanded to full tag strings on the events. This layer
does not build a document tree and does not apply a schema.

`parse_str` drains the whole stream into a `Vec<Event>` for convenience.

`Parser::last_mark` records the source position of the most recent successful
event. The composer uses that mark when reporting problems such as an undefined
alias or a duplicate anchor.

## Composer

`ozzyyaml` consumes parser events and builds `Value` trees using the YAML
**1.2.2 Core schema**. Plain scalars become null, bool, int, float, or string
according to Core rules. Quoted scalars stay strings unless an explicit Core
tag says otherwise. Values such as `yes` / `no` / `on` / `off` remain strings
under Core (they are not booleans).

Useful entry points:

- `from_str` — exactly one document
- `load_all` — every document in the stream
- `from_slice` / `load_all_from_slice` — same loaders over bytes (UTF-8 checked
  first)

Each function has a `*_with_limits` variant.

An alias is an `Rc` clone of the node that carried the matching anchor. The
`max_anchors` limit is enforced when an anchor is registered during compose,
not while the scanner is tokenizing.

## Shared graphs, aliases, and cycles

YAML aliases are a common source of “billion laughs” style attacks: a small
document can name one node many times. ozzyyaml avoids the classic blow-up by
**sharing** nodes instead of deep-copying them. An alias points at the same
`Rc<Value>` as its anchor.

Sequences and mappings store children behind a `RefCell` so a collection can be
registered as an anchor before its children are filled. That makes cyclic
structures such as `&a [*a]` representable.

Anything that flattens a graph into a pure tree (for example some language
bindings) must handle cycles explicitly. Silent expansion of a DAG into an
exponential copy is not acceptable.

## Resource limits

`Limits` carries three caps, with conservative defaults:

| Limit | Default role |
| --- | --- |
| `max_anchors` | Primary DoS brake (compose time) |
| `max_nesting_depth` | Caps collection nesting (parse time) |
| `max_input_bytes` | Caps raw input size (read time) |

Crossing a limit returns a typed error rather than hanging or trying to
allocate without bound.

## Unicode anchors

YAML 1.2.2 allows non-ASCII characters in anchor and alias names. libyaml, and
tools built on it, often reject them. ozzyyaml accepts them. Tests live in
`crates/ozzyyaml-parser/tests/unicode_anchors.rs` and in the compose suite.

## Emitter

`ozzyyaml-emit` walks a `Value` and writes YAML 1.2.2 text. Dumping is
value-level: comments, original tags, and exact source layout are not
preserved.

**Flow style is the default** (`[1, 2]`, `{k: v}`). Pass block style through
`EmitOptions` when you want `-` lists and `key:` mappings. One style applies to
the whole dump.

Shared nodes are detected by **`Rc` identity**. A node reached more than once
(including through a cycle) is written once with a generated anchor and later
referenced with an alias. Generated names are ASCII (`a`, `b`, …); the writer
can also emit Unicode anchor spellings when given them.

`dump_all` writes a multi-document stream and resets the anchor map for each
document:

- Several documents: each one, including the first, is prefixed with `---`
- One document: same text as `to_string` (no `---`)
- No documents: an empty string
