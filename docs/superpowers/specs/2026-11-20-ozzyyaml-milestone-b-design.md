# ozzyyaml Milestone B — Design Spec

**Date:** 2026-11-20  
**Status:** Approved  
**Parent:** [2026-11-20-ozzyyaml-design.md](./2026-11-20-ozzyyaml-design.md)

## Intent

Add the `ozzyyaml` crate: compose `ozzyyaml-parser` events into an `Rc`-shared document graph and construct scalars with **YAML 1.2.2 Core schema**.

## Decisions

| Topic | Choice |
| --- | --- |
| Node sharing | `Rc<Value>` (not arena/`NodeId`) |
| Public API | Dynamic `Value` / documents only — **no Serde in B** |
| Packaging | New workspace crate `ozzyyaml` depending on `ozzyyaml-parser` |
| Schema | Core 1.2.2 default; PyYAML-compat later |
| Process | Lean plan + **inline** implementation (lighter than Milestone A SDD) |

## Public API (crate `ozzyyaml`)

- `Value` enum:
  - `Null`
  - `Bool(bool)`
  - `Int(i64)`
  - `Float(f64)`
  - `String(String)`
  - `Sequence(Vec<Rc<Value>>)`
  - `Mapping(Vec<(Rc<Value>, Rc<Value>)>)` — ordered pairs; duplicate keys retained as separate entries for now
- Alias targets and collection children are `Rc<Value>` so aliases share identity (`Rc::ptr_eq`)
- `from_str(input) -> Result<Value>` — single document (error if zero or >1 docs unless documented otherwise; prefer: require exactly one non-empty document)
- `load_all(input) -> Result<Vec<Value>>` — all documents in the stream
- `from_str_with_limits` / `load_all_with_limits` taking `Limits`
- Small accessors later as needed (`as_str`, etc.) — YAGNI until tests want them
- Re-export or wrap `Limits` / errors; compose adds `Compose` variants (undefined alias, duplicate anchor, anchor limit)

## Composer

1. Drive `ozzyyaml_parser::Parser` (or `parse_str` events).
2. Build nodes on `SequenceStart`/`MappingStart`/`Scalar`/`Alias`.
3. On anchor: insert into `HashMap<String, Rc<Value>>`; reject duplicates; enforce `max_anchors`.
4. On alias: look up and `Rc::clone` — never deep-copy.
5. Cycles are representable; do not provide a silent deep-clone/flatten API in B.

## Core schema constructor

Apply to **plain** scalars (and tagged Core types when tag is `tag:yaml.org,2002:*` / `!!`):

| Resolved type | Patterns (non-exhaustive; follow 1.2.2 Core) |
| --- | --- |
| null | empty, `~`, `null`/`Null`/`NULL` |
| bool | `true`/`True`/`TRUE`, `false`/`False`/`FALSE` only — **not** yes/no/on/off |
| int | decimal, `0o` octal, `0x` hex; **no** sexagesimal |
| float | Core float forms including `.inf` / `.nan` variants per Core |
| string | everything else |

Quoted scalars stay strings (unless explicitly tagged). Tagged `!!str` / `!!int` / etc. honor the tag when supported; unknown tags may keep a string node or error — **B choice: unknown non-Core tags → keep as `String` with tag ignored for typing** (tag metadata not stored on `Value` in B to keep the type small; revisit if needed).

## Limits

- Parse-time: `max_input_bytes`, `max_nesting_depth` (parser)
- Compose-time: `max_anchors` (primary billion-laughs brake with sharing)

## Non-goals (B)

- Serde
- Emit
- PyYAML-compat schema
- Merge keys (`<<`)
- Comment/whitespace preservation
- Storing tags/anchors on the public `Value` for round-trip (emit can re-derive later)

## Testing

- Core scalar table unit tests
- Alias sharing via `Rc::ptr_eq`
- Cyclic structures construct without panic
- `max_anchors` → `LimitExceeded`
- Unicode anchors end-to-end through `from_str`
- Multi-document `load_all`

## File map

```
crates/ozzyyaml/
  Cargo.toml
  src/lib.rs
  src/value.rs
  src/error.rs
  src/limits.rs      # re-export / thin wrap of parser Limits
  src/compose.rs
  src/schema/core.rs
  tests/*.rs
```
