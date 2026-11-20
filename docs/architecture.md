# Architecture

ozzyyaml is a YAML **1.2.2** stack. `ozzyyaml-parser` produces the event stream.
`ozzyyaml` composes those events into an `Rc`-shared Core-schema `Value`.

```
&str / &[u8]
  → Reader → Scanner → Parser (events)
  → Composer (ozzyyaml) → Rc-shared Value (Core schema)
```

Internals (`Reader`, `Scanner`, `Token`) stay crate-private. The parser's public
surface is `Parser` / `parse_str`, `Event`, `Limits`, `Error` / `Mark`. The
composer crate exposes `from_str` / `load_all` / `from_slice*` (and the
`*_with_limits` variants), plus `Value`, `Limits`, and `Error`.

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

`Parser::last_mark` is the source mark of the event most recently returned as
`Ok(Some(_))`. Compose errors use that mark (`UndefinedAlias`,
`DuplicateAnchor`, and anchor `LimitExceeded`). A parse error leaves
`last_mark` unchanged.

## Composer

`ozzyyaml` pulls events and builds Core-schema `Value`s. Plain scalars are
resolved by the Core schema; everything else stays a string.

- `from_str` / `from_str_with_limits` — exactly one document
- `load_all` / `load_all_with_limits` — every document in the stream
- `from_slice` / `from_slice_with_limits` and `load_all_from_slice` /
  `load_all_from_slice_with_limits` — the same loaders over `&[u8]`. Bytes are
  checked as UTF-8 first; invalid input is `Error::InvalidUtf8`. The parser
  itself still takes `&str`.

An alias is an `Rc` clone of the anchored node. `max_anchors` is enforced when
that anchor is registered, not while scanning events.

## Shared graph vs copy

Billion-laughs attacks are primarily **alias/anchor** related: a small document
can name one node many times.

The composer builds a **shared** node graph (`Rc<Value>`, not deep copies). An
alias points at the same node as its anchor. That removes the classic memory
blow-up from materializing duplicates.

Sequences and mappings store their children in `RefCell`. A collection is
registered as an anchor before its children are filled, so cyclic aliases
(`&a [*a]`) are representable.

Flattening APIs (JSON export, deep owned trees) must be explicit and budgeted.
They must not silently expand a DAG into exponential work.

## `max_anchors`

`Limits::max_anchors` (default 1_024) is the **primary DoS brake**. Nesting
depth and input size are secondary caps.

Enforcement is at **compose time**, when distinct anchors are registered on the
graph. The event parser does not count or reject excess anchors. Crossing the
limit returns `Error::LimitExceeded` (`LimitKind::Anchors`) positioned with
`Parser::last_mark`.

## Unicode anchors

YAML 1.2.2 allows non-ASCII characters in anchors and aliases. libyaml (and
therefore PyYAML) rejects them, treating anchor names as ASCII. A patch has
sat unmerged for years: [libyaml#132](https://github.com/yaml/libyaml/issues/132).

ozzyyaml accepts Unicode anchors on purpose (`&汉`, `*😀`, …). That is a
primary motivation for a from-scratch parser rather than wrapping or forking
libyaml. Acceptance tests live in `crates/ozzyyaml-parser/tests/unicode_anchors.rs`.
