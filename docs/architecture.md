# Architecture

ozzyyaml is a YAML **1.2.2** stack. Milestone A (`ozzyyaml-parser`) stops at the
event stream. Milestone B (`ozzyyaml`) will compose those events into a document
graph.

```
&str / &[u8]
  → Reader   (encoding, BOM, position marks)
  → Scanner  (tokens)
  → Parser   (YAML 1.2.2 events)
  → pull API / iterator of Event
```

Internals (`Reader`, `Scanner`, `Token`) stay crate-private. The public surface
is `Parser` / `parse_str`, `Event`, `Limits`, `Error` / `Mark`.

## Reader

`Reader` is a UTF-8 cursor over the input. It:

- Rejects input longer than `Limits::max_input_bytes`
- Skips a leading UTF-8 BOM (`U+FEFF`)
- Normalizes `\r\n` and lone `\r` to a logical `\n`
- Tracks a 1-based `Mark` (byte index, line, column) for errors

It does not interpret YAML syntax.

## Scanner

`Scanner` turns characters into `Token`s: scalars, anchors, aliases, tags,
directives, document markers (`---` / `...`), and collection structure
(`BlockEntry`, `Key` / `Value`, flow `[` `{` `]` `}`, indent-aware
`BlockSequenceStart` / `BlockMappingStart` / `BlockEnd`).

It keeps a token queue, an indent stack, and libyaml-style simple-key tracking
(single-line in Milestone A). Indentation is ignored inside flow collections.

Anchor and alias names use YAML 1.2.2 `ns-anchor-char` — printable, non-space,
not a flow indicator — including non-ASCII. See [Unicode anchors](#unicode-anchors).

## Parser

`Parser` is a pull state machine: `next_event()` consumes tokens and emits
`Event`s (`StreamStart`/`End`, `DocumentStart`/`End`, `Scalar`, `Alias`,
`SequenceStart`/`End`, `MappingStart`/`End`).

Collection nesting is bounded by `Limits::max_nesting_depth`. `%TAG` handles
are expanded to full tag strings on the emitted events. There is no document
tree and no schema typing at this layer.

`parse_str` is a convenience that drains `next_event()` into a `Vec<Event>`.

## Shared graph vs copy (Milestone B)

Billion-laughs attacks are primarily **alias/anchor** related: a small document
can name one node many times.

Milestone B's composer must build a **shared** node graph (references / arena
ids, not deep copies). An alias points at the same node as its anchor. That
removes the classic memory blow-up from materializing duplicates.

Flattening APIs (JSON export, deep owned trees) must be explicit and budgeted.
They must not silently expand a DAG into exponential work.

Milestone A only records anchor names on events. It does not resolve aliases or
build nodes.

## `max_anchors`

`Limits::max_anchors` (default 1_024) is the **primary DoS brake**. Nesting
depth and input size are secondary caps.

The field is on `Limits` in Milestone A for API stability (`LimitKind::Anchors`
already exists). Enforcement belongs at **compose time** in Milestone B, when
distinct anchors are actually registered on the graph. The event parser does
not count or reject excess anchors.

## Unicode anchors

YAML 1.2.2 allows non-ASCII characters in anchors and aliases. libyaml (and
therefore PyYAML) rejects them, treating anchor names as ASCII. A patch has
sat unmerged for years: [libyaml#132](https://github.com/yaml/libyaml/issues/132).

ozzyyaml accepts Unicode anchors on purpose (`&汉`, `*😀`, …). That is a
primary motivation for a from-scratch parser rather than wrapping or forking
libyaml. Acceptance tests live in `crates/ozzyyaml-parser/tests/unicode_anchors.rs`.
