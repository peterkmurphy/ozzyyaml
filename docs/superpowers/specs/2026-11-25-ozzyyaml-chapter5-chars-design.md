# ozzyyaml Chapter 5 character productions — Design Spec

**Date:** 2026-11-25  
**Status:** Approved for implementation planning  
**Parent:** [2026-11-24-ozzyyaml-conformance-design.md](./2026-11-24-ozzyyaml-conformance-design.md)  
**Scope:** YAML 1.2.2 productions `[1]`–`[62]` and §5.2 encoding detection

## Intent

Make Chapter 5 (character productions) and §5.2 encoding detection match the
spec exactly. An audit showed escapes and indicator glyphs are largely correct,
while character-class predicates are missing, unused, or looser than the
productions (reserved `@`/`\``, non-printables, tag URI charset, BOM-less
encoding).

Approach **A**: named predicates + enforcement at grammar use sites; full §5.2
BOM **and** null-pattern detection. Machine-readable grammar codegen is
explicitly deferred.

## Goals

1. Named, unit-tested helpers for character classes `[1]`–`[40]` that match the
   productions 1:1 (no “practical” subsets).
2. Enforce those classes at the correct sites (stream / plain / single / block
   vs double-quoted; tags; plain scalar start).
3. Keep escape table `[41]`–`[62]` (already correct); route hex digits through
   `ns-hex-digit`.
4. `decode_yaml_bytes` implements the full §5.2 detection table (BOM rows +
   null-pattern rows, in order).
5. Document the enforcement in `docs/conformance.md` (and Python notes if needed).

## Non-goals

- `[102] ns-anchor-char` / suite `W5VH` (later grammar track)
- Remaining compact block/flow suite gaps (~21 other `test.event` misses)
- Rewriting the scanner to a production tree
- Machine-readable grammar / codegen
- Emit presentation changes beyond what load rejection already implies

## Character predicates (`chars.rs`)

| Helper | Production(s) |
| --- | --- |
| `is_printable` | `[1] c-printable` |
| `is_nb_json` | `[2] nb-json` |
| `is_nb_char` / `is_ns_char` | `[27]` / `[34]` |
| `is_space` / `is_tab` / `is_white` | `[31]`–`[33]` |
| `is_dec_digit` / `is_hex_digit` / `is_ascii_letter` / `is_word_char` | `[35]`–`[38]` |
| `is_uri_char` / `is_tag_char` | `[39]` / `[40]` — ASCII + `%`+2 hex; tag = uri − `!` − flow indicators |
| `is_flow_indicator` / `is_indicator` / `is_reserved` | `[23]` / `[22]` / `[21]` |
| Escape decode | `[41]`–`[62]` — existing table; hex via `is_hex_digit` |

Indicators `[4]`–`[20]` remain character literals at token sites.

### Enforcement

- Plain / single-quoted / block scalar content: `nb-char` (reject non-printables,
  `U+FFFE` / `U+FFFF`, raw C0 other than line breaks handled by the reader).
- Double-quoted **unescaped** content: `nb-json` (raw C0 other than tab → error;
  escapes still produce any scalar).
- Plain scalar **start**: reject `c-reserved` (`@`, `` ` ``).
- Tag scan: `is_tag_char`; verbatim `!<…>` body via `is_uri_char` (reject raw
  non-ASCII; `%`-encoded octets OK and not decoded for comparison beyond
  existing percent-decode rules for resolution).

`is_anchor_char` is **out of scope** except where it must not contradict `[1]` /
`[34]` while this track lands. Correct `[102]` behaviour is a later track.

## Encoding & BOM (`[3]` / §5.2)

`decode_yaml_bytes` in `ozzyyaml` follows the spec table **in order**:

1. Explicit BOMs: UTF-32BE, UTF-32LE, UTF-16BE, UTF-16LE, UTF-8
2. Null-pattern (no BOM): `00 00 00 xx` → UTF-32BE; `xx 00 00 00` → UTF-32LE;
   `00 xx` → UTF-16BE; `xx 00` → UTF-16LE
3. Else UTF-8

Leading BOM is stripped from the returned character string. Empty input → empty
UTF-8 string.

In the character stream (`ozzyyaml-parser`):

- BOM at the **start of a document** may be skipped (spec allows BOM at the
  start of any document; stream encoding is fixed at byte decode).
- BOM **inside** a document (not at document start) → error (example 5.2).
- BOM inside **double-quoted** scalars → allowed as content `U+FEFF` (`nb-json`).

`&str` APIs remain UTF-8 character streams (reader still skips a leading BOM).

## Errors

Reuse existing scan / parse / `InvalidEncoding` / `InvalidUtf8` channels. New
messages name the rule in plain language (e.g. non-printable character, reserved
indicator, invalid tag character, byte order mark inside document). No new
public error variants unless mapping is impossible.

## Tests

1. Unit tests on each predicate in `chars.rs`.
2. Scanner/parser cases for reserved plain start, non-printable rejection,
   double-quoted `nb-json` vs escapes, tag charset, mid-document BOM, BOM in
   double quotes.
3. Encoding matrix: every BOM row, every null-pattern row, existing
   UTF-32LE-vs-UTF-16LE precedence, plus regressions.
4. Workspace + Python suites: remain green aside from inputs that become
   intentionally illegal.

## Docs

Update `docs/conformance.md` (and `docs/python.md` if needed) to state Chapter 5
+ §5.2 (BOM and null-pattern) are enforced; remaining event-suite gaps unchanged.

## Success criteria

- Audit holes for `[1]`, `[2]`, `[3]`/§5.2, `[21]`, `[27]`, `[34]`, `[39]`,
  `[40]` closed with failing-then-passing tests.
- `[41]`–`[62]` still pass; indicator literals `[4]`–`[20]` unchanged in
  behaviour.
- `cargo test` workspace green; Python tests green after rebuild.
