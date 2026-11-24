# Chapter 5 characters + §5.2 encoding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make YAML 1.2.2 productions `[1]`–`[62]` and §5.2 encoding detection match the spec: named character predicates enforced at use sites, plus BOM and null-pattern byte decoding.

**Architecture:** Expand `ozzyyaml-parser` `chars.rs` with 1:1 production helpers; wire them into Reader/Scanner scalar and tag paths; extend `ozzyyaml` `decode_yaml_bytes` to the full §5.2 detection table. TDD throughout. No scanner rewrite; `[102] ns-anchor-char` stays out of scope.

**Tech Stack:** Rust 2021 workspace (`ozzyyaml-parser`, `ozzyyaml`), existing `cargo test` / Python pytest after `maturin develop`.

**Spec:** `docs/superpowers/specs/2026-11-25-ozzyyaml-chapter5-chars-design.md`

## Global Constraints

- Match YAML 1.2.2 productions exactly — no “practical” character subsets.
- Do not change `[102] ns-anchor-char` behaviour as a goal of this plan (W5VH remains deferred).
- Do not allowlist new suite IDs in this plan unless they pass solely due to Chapter 5 fixes.
- `#![forbid(unsafe_code)]` on Rust crates.
- Test via existing patterns: `#[cfg(test)]` in modules and/or `crates/*/tests/*.rs`.
- Keep edited `.rs` files under ~300 lines; split helpers rather than bloating `scalar.rs`.
- Commit after each task; human pushes.

## File structure (lock-in)

| Path | Role |
| --- | --- |
| `crates/ozzyyaml-parser/src/chars.rs` | Named `[1]`–`[40]` predicates + unit tests |
| `crates/ozzyyaml-parser/src/scanner/mod.rs` | Plain start reserved check; BOM-inside-doc hook if needed |
| `crates/ozzyyaml-parser/src/scanner/scalar.rs` | Enforce `nb-char` / `nb-json`; hex via `is_hex_digit` |
| `crates/ozzyyaml-parser/src/scanner/block_scalar.rs` | Enforce `nb-char` on block content |
| `crates/ozzyyaml-parser/src/scanner/document.rs` | Replace loose `is_tag_char` with spec helpers |
| `crates/ozzyyaml-parser/src/reader.rs` | Optional leading/document-start BOM skip; reject mid-doc BOM outside quotes |
| `crates/ozzyyaml-parser/tests/chapter5.rs` | **Create** — enforcement integration tests |
| `crates/ozzyyaml/src/encoding.rs` | Full §5.2 BOM + null-pattern decode |
| `crates/ozzyyaml/tests/encoding.rs` | Encoding matrix expansion |
| `docs/conformance.md` | Note Chapter 5 + §5.2 enforcement |
| `docs/python.md` | Brief note if byte decode behaviour is mentioned |

---

### Task 1: Character class predicates (`chars.rs`)

**Files:**
- Modify: `crates/ozzyyaml-parser/src/chars.rs`
- Test: same file `#[cfg(test)]`

**Interfaces:**
- Produces: `is_printable`, `is_nb_json`, `is_nb_char`, `is_ns_char`, `is_space`, `is_tab`, `is_white`, `is_dec_digit`, `is_hex_digit`, `is_ascii_letter`, `is_word_char`, `is_uri_char`, `is_tag_char`, `is_flow_indicator`, `is_indicator`, `is_reserved` (all `pub(crate) fn(char) -> bool` except URI may need a small scanner helper — see steps)
- Keep existing `is_anchor_char` for now (out of scope to redefine); ensure it still compiles against updated helpers if it calls them

- [ ] **Step 1: Write failing unit tests for the missing predicates**

In `chars.rs` tests, assert at least:

```rust
assert!(is_nb_json('\t'));
assert!(is_nb_json('\u{7f}')); // DEL — in nb-json, not c-printable
assert!(!is_nb_json('\u{1}'));
assert!(!is_printable('\u{7f}'));
assert!(!is_printable('\u{fffe}'));
assert!(is_nb_char('a'));
assert!(!is_nb_char('\n'));
assert!(is_reserved('@'));
assert!(is_reserved('`'));
assert!(is_uri_char('A'));
assert!(!is_uri_char('汉'));
assert!(is_tag_char('a'));
assert!(!is_tag_char('!'));
assert!(!is_tag_char(','));
```

Also keep/extend existing `is_printable` / `is_flow_indicator` tests so ranges match `[1]` / `[23]` exactly (including excluding `U+FFFE`/`U+FFFF` via range end `U+FFFD`).

- [ ] **Step 2: Run tests — expect FAIL (missing items)**

Run: `cargo test -p ozzyyaml-parser chars -- --nocapture`  
Expected: compile errors or assertion failures for new symbols.

- [ ] **Step 3: Implement predicates**

Implement each production literally from the spec. For `[39] ns-uri-char`, either:

- `is_uri_char(c)` for the single-char alternatives, plus `try_consume_uri_escape(rest) -> Option<(char_len)>` for `%`+2 hex at scan sites, **or**
- a `matches_uri_char_at(s: &str) -> Option<usize>` returning byte/char length.

Tag char `[40]` = uri char minus `!` minus flow indicators (for the non-`%` branch; `%HH` still allowed).

`is_indicator` is the union in `[22]`. `is_reserved` is `@` | `` ` ``.

- [ ] **Step 4: Run tests — expect PASS**

Run: `cargo test -p ozzyyaml-parser chars`

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml-parser/src/chars.rs
git commit -m "feat(parser): YAML 1.2.2 Chapter 5 character class predicates"
```

---

### Task 2: Enforce `nb-char` / `nb-json` / reserved plain start

**Files:**
- Modify: `crates/ozzyyaml-parser/src/scanner/mod.rs` (dispatch / plain start)
- Modify: `crates/ozzyyaml-parser/src/scanner/scalar.rs` (`scan_plain`, `scan_single_quoted`, `scan_double_quoted`, `scan_hex_escape`)
- Modify: `crates/ozzyyaml-parser/src/scanner/block_scalar.rs` (content chars)
- Create: `crates/ozzyyaml-parser/tests/chapter5.rs`

**Interfaces:**
- Consumes: Task 1 predicates
- Produces: scan errors for illegal characters; reserved `@`/`\`` cannot start a plain scalar

- [ ] **Step 1: Write failing integration tests**

In `tests/chapter5.rs` using `parse_str` / public parser API:

```rust
#[test]
fn reserved_at_rejects_plain() {
    assert!(parse_str("@text\n").is_err());
}
#[test]
fn reserved_grave_rejects_plain() {
    assert!(parse_str("`text\n").is_err());
}
#[test]
fn nonprintable_rejects_plain() {
    assert!(parse_str("\u{7}\n").is_err());
    assert!(parse_str("\u{fffe}\n").is_err());
}
#[test]
fn double_quoted_rejects_raw_c0() {
    assert!(parse_str("\"\u{1}\"\n").is_err());
}
#[test]
fn double_quoted_accepts_escaped_c0() {
    assert!(parse_str("\"\\a\"\n").is_ok());
}
#[test]
fn double_quoted_accepts_del_unescaped() {
    // DEL is nb-json but not c-printable
    assert!(parse_str("\"\u{7f}\"\n").is_ok());
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test -p ozzyyaml-parser --test chapter5`  
Expected: several failures (currently accept illegal input).

- [ ] **Step 3: Implement enforcement**

- Before `scan_plain` path when first char is reserved → scan error (example 5.10).
- While scanning plain / single / block content, reject chars failing `is_nb_char` (line breaks still via reader normalization, not as `nb-char` content on the same path — follow how breaks are already handled; do not accept `\x0C` / BEL / `U+FFFE` as content).
- In `scan_double_quoted`, for each unescaped character, require `is_nb_json`; escapes unchanged.
- `scan_hex_escape`: validate digits with `is_hex_digit` instead of only `to_digit(16)` (equivalent set, but call the named helper).

- [ ] **Step 4: Run chapter5 + scanner tests — expect PASS**

Run: `cargo test -p ozzyyaml-parser --test chapter5 && cargo test -p ozzyyaml-parser`

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml-parser/src/scanner crates/ozzyyaml-parser/tests/chapter5.rs
git commit -m "fix(parser): enforce nb-char, nb-json, and reserved plain starts"
```

---

### Task 3: Tag URI / tag-char charset

**Files:**
- Modify: `crates/ozzyyaml-parser/src/scanner/document.rs` (`fetch_tag`, replace local `is_tag_char`)
- Modify: `crates/ozzyyaml-parser/tests/chapter5.rs` (add cases)
- Possibly: `crates/ozzyyaml-parser/src/parser/document.rs` if percent-decode assumptions change

**Interfaces:**
- Consumes: `is_tag_char`, URI escape helper from Task 1
- Produces: reject `!汉`, accept `!%E6%B1%89` (or equivalent `%`-encoded form) and ASCII `ns-tag-char` tags

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn tag_rejects_raw_non_ascii() {
    assert!(parse_str("!汉 value\n").is_err());
}
#[test]
fn tag_accepts_percent_encoded() {
    assert!(parse_str("!%E6%B1%89 value\n").is_ok());
}
#[test]
fn tag_rejects_flow_indicator_in_shorthand() {
    assert!(parse_str("!foo,bar value\n").is_err()); // or parses as tag !foo then unexpected — assert no silent accept of comma inside tag name
}
```

Tune the comma case to the actual token boundary the scanner should use (tag ends when `is_tag_char` fails).

- [ ] **Step 2: Run — expect FAIL**

- [ ] **Step 3: Implement**

Replace `document.rs` `is_tag_char` with `chars::is_tag_char` and proper `%HH` consumption for both shorthand suffix and verbatim `!<…>` body (`is_uri_char` / URI escape). Do not treat Unicode letters as URI chars.

- [ ] **Step 4: Run tests — expect PASS**

Run: `cargo test -p ozzyyaml-parser --test chapter5 && cargo test -p ozzyyaml-parser`

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml-parser/src/scanner/document.rs crates/ozzyyaml-parser/tests/chapter5.rs
git commit -m "fix(parser): restrict tags to ns-uri-char / ns-tag-char"
```

---

### Task 4: BOM inside documents vs quoted

**Files:**
- Modify: `crates/ozzyyaml-parser/src/reader.rs` and/or scanner whitespace/scalar paths
- Modify: `crates/ozzyyaml-parser/tests/chapter5.rs`

**Interfaces:**
- Leading BOM at stream/document start: skip (existing reader behaviour + allow at document start if needed)
- Mid-document BOM outside double quotes: error
- Double-quoted BOM: allowed content (`nb-json` includes `U+FEFF`)

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn bom_inside_document_errors() {
    let err = parse_str("- a\n\u{feff}- b\n").unwrap_err();
    let msg = err.to_string();
    assert!(msg.to_lowercase().contains("byte order") || msg.contains("BOM") || msg.contains("feff"), "{msg}");
}
#[test]
fn bom_inside_double_quoted_ok() {
    assert!(parse_str("\"a\u{feff}b\"\n").is_ok());
}
```

- [ ] **Step 2: Run — expect FAIL** (wrong error or accept)

- [ ] **Step 3: Implement**

When a `U+FEFF` is encountered outside double-quoted scanning and not at a permitted document-start position, return a clear scan error. Inside `scan_double_quoted`, `U+FEFF` passes `is_nb_json` and is kept.

- [ ] **Step 4: Run — expect PASS**

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml-parser/src/reader.rs crates/ozzyyaml-parser/src/scanner crates/ozzyyaml-parser/tests/chapter5.rs
git commit -m "fix(parser): reject mid-document BOM; allow BOM in double quotes"
```

---

### Task 5: Full §5.2 encoding detection (BOM + null-pattern)

**Files:**
- Modify: `crates/ozzyyaml/src/encoding.rs`
- Modify: `crates/ozzyyaml/tests/encoding.rs`
- Possibly unit tests inside `encoding.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: unchanged public `from_slice*` 
- Produces: `decode_yaml_bytes` detecting per spec table order

- [ ] **Step 1: Write failing encoding tests**

Add to `crates/ozzyyaml/tests/encoding.rs`:

```rust
#[test]
fn utf16le_without_bom_via_null_pattern() {
    // "a\n" as UTF-16LE: 61 00 0A 00
    let bytes = vec![0x61, 0x00, 0x0a, 0x00];
    assert_eq!(from_slice(&bytes).unwrap(), Value::String("a".into()));
}
#[test]
fn utf16be_without_bom_via_null_pattern() {
    let bytes = vec![0x00, 0x61, 0x00, 0x0a];
    assert_eq!(from_slice(&bytes).unwrap(), Value::String("a".into()));
}
#[test]
fn utf32le_without_bom_via_null_pattern() {
    let mut bytes = vec![0x61, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00];
    assert_eq!(from_slice(&bytes).unwrap(), Value::String("a".into()));
}
#[test]
fn utf32be_without_bom_via_null_pattern() {
    let bytes = vec![0x00, 0x00, 0x00, 0x61, 0x00, 0x00, 0x00, 0x0a];
    assert_eq!(from_slice(&bytes).unwrap(), Value::String("a".into()));
}
```

Keep existing BOM tests green.

- [ ] **Step 2: Run — expect FAIL** on null-pattern cases (today misread as UTF-8)

Run: `cargo test -p ozzyyaml --test encoding`

- [ ] **Step 3: Implement detection table**

In `decode_yaml_bytes`, after BOM checks, before default UTF-8, apply null-pattern rows exactly as in the design/spec (careful with short inputs: only match when enough bytes exist; `xx 00` for UTF-16LE must not steal valid UTF-8 that happens to contain a NUL in a way the spec does not intend — follow the published table order and “first few bytes” rules).

- [ ] **Step 4: Run encoding + compose tests — expect PASS**

Run: `cargo test -p ozzyyaml --test encoding && cargo test -p ozzyyaml`

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml/src/encoding.rs crates/ozzyyaml/tests/encoding.rs
git commit -m "feat(encoding): §5.2 null-pattern UTF-16/32 detection"
```

---

### Task 6: Docs + Python regression

**Files:**
- Modify: `docs/conformance.md`
- Modify: `docs/python.md` (only if encoding/`bytes` wording needs null-pattern)
- Test: `python/tests/` after `maturin develop`

- [ ] **Step 1: Update conformance blurb**

State that Chapter 5 character classes and §5.2 (BOM **and** null-pattern) are enforced; remaining ~91% suite caveat unchanged.

- [ ] **Step 2: Rebuild Python + run tests**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cd python && . .venv/bin/activate && maturin develop -q && pytest -q
```

Expected: pass (add a small null-pattern bytes test only if you want parity with Rust matrix; optional).

- [ ] **Step 3: Full workspace test**

Run: `cargo test`  
Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add docs/conformance.md docs/python.md python/tests
git commit -m "docs: Chapter 5 and §5.2 encoding enforcement"
```

---

## Done when

- Audit holes for `[1]`, `[2]`, `[3]`/§5.2, `[21]`, `[27]`, `[34]`, `[39]`, `[40]` closed.
- Escape `[41]`–`[62]` still green; indicators `[4]`–`[20]` unchanged.
- `cargo test` and Python pytest green.
- Design success criteria in the spec file satisfied.
