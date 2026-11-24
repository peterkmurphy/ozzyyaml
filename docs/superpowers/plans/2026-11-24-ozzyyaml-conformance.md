# YAML 1.2.2 Conformance Track Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring ozzyyaml from ~40% yaml-test-suite event coverage to ≈100% event cases, with strict Core compose, data round-trip emit, raised Limits, and UTF-16/32 byte APIs last — then honestly claim YAML 1.2.2.

**Architecture:** Extend the existing Reader → Scanner → Parser → Composer → Emitter pipeline. No greenfield scanner. Grow `suite_allowlist.txt` only with IDs that actually pass. Prefer TDD from suite-derived unit tests before allowlist updates.

**Tech Stack:** Rust 2021 workspace (`ozzyyaml-parser`, `ozzyyaml`, `ozzyyaml-emit`), yaml-test-suite submodule under `vendor/yaml-test-suite`, `cargo test`, existing event harness in `crates/ozzyyaml-parser/tests/yaml_test_suite.rs`.

**Spec:** `docs/superpowers/specs/2026-11-24-ozzyyaml-conformance-design.md`

## Global Constraints

- Never allowlist a failing yaml-test-suite ID.
- Do not claim full YAML 1.2.2 in public docs until Task 9 (track Done) completes; Task 0 removes overclaims immediately.
- No PyYAML 1.1 quirks, merge keys, Serde, or comment preservation in this track.
- Duplicate mapping keys remain retained (no uniqueness enforcement).
- Human pushes commits; agents commit locally when a task finishes.
- Test type suffix: `*.unit.test.rs` is not used in this repo — follow existing patterns (`tests/*.rs`, `#[cfg(test)]` modules). Prefer new focused test files under `crates/*/tests/` named for the feature (e.g. `folding.rs`).
- Max file sizes: keep `.rs` under ~300 lines when editing; split scanner helpers rather than bloating `scalar.rs` unboundedly.

## File structure (lock-in)

| Path | Role |
| --- | --- |
| `crates/ozzyyaml-parser/src/scanner/scalar.rs` | Plain / quoted / escapes (Phases 1, 3) |
| `crates/ozzyyaml-parser/src/scanner/block_scalar.rs` | **Create** — `\|` / `>` scanning (Phase 2) |
| `crates/ozzyyaml-parser/src/scanner/block.rs` | Block indicators, same-line mapping policy (Phases 2, 4, 6) |
| `crates/ozzyyaml-parser/src/scanner/document.rs` | Directive lines (Phase 5) |
| `crates/ozzyyaml-parser/src/reader.rs` | Line breaks / marks used by folding |
| `crates/ozzyyaml-parser/src/limits.rs` | Default Limits (Phase 7) |
| `crates/ozzyyaml-parser/tests/suite_allowlist.txt` | Event-suite allowlist |
| `crates/ozzyyaml-parser/tests/yaml_test_suite.rs` | Event harness (unchanged contract) |
| `crates/ozzyyaml-parser/tests/folding.rs` | **Create** — Phase 1 units |
| `crates/ozzyyaml-parser/tests/block_scalars.rs` | **Create** — Phase 2 units |
| `crates/ozzyyaml-parser/tests/escapes.rs` | **Create** — Phase 3 units |
| `crates/ozzyyaml-parser/tests/explicit_keys.rs` | **Create** — Phase 4 units |
| `crates/ozzyyaml-parser/tests/directives.rs` | **Create** — Phase 5 units |
| `crates/ozzyyaml/src/schema/core.rs` | Strict tags (Phase 6) |
| `crates/ozzyyaml/src/limits.rs` | Mirrors parser defaults (Phase 7) |
| `crates/ozzyyaml/tests/roundtrip.rs` | **Create** — Phase 3/8 property tests |
| `crates/ozzyyaml/tests/suite_allowlist.txt` | Compose allowlist (Phase 8) |
| `crates/ozzyyaml-emit/src/scalar.rs` | Escape emission aligned with parser (Phase 3) |
| `crates/ozzyyaml/src/encoding.rs` | **Create** — BOM / UTF-16/32 decode (Phase 9) |
| `README.md`, `docs/*.md`, crate READMEs | Honesty then final claim (Phases 0, 9) |

**Resolved open items (from design):**

- New default Limits: `max_input_bytes = 256 MiB`, `max_nesting_depth = 10_000`, `max_anchors = 100_000`.
- Block-scalar *emission*: Phase 8 (optional quality); Phase 2 is parse/events only.
- Strict Core tags: `!!null`, `!!bool`, `!!int`, `!!float` must match `try_*` or compose error; `!!str` always accepts content as string; unknown tags remain untyped (string / plain-resolve as today).

---

### Task 0: Honesty docs (Phase 0)

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture.md`
- Modify: `docs/python.md`
- Modify: `crates/ozzyyaml-parser/README.md`
- Modify: `crates/ozzyyaml/README.md`
- Modify: `crates/ozzyyaml-emit/README.md`
- Modify: `python/README.md`
- Create: `docs/conformance.md` (short public gaps pointer)

**Spec coverage:** Design Phase 0  
**Test coverage:** Manual read of public docs — no suite change  
**Emitter:** note only if emit README overclaims  

**Interfaces:**
- Consumes: design doc path
- Produces: `docs/conformance.md` linking design + stating ~40% suite / missing folding, block scalars, `?`, full escapes, UTF-16/32 last

- [ ] **Step 1: Add `docs/conformance.md`**

```markdown
# Conformance status

ozzyyaml targets YAML 1.2.2 but is **not yet complete**.

As of the 0.1.0 line, the event parser passes roughly 40% of
[yaml-test-suite](https://github.com/yaml/yaml-test-suite) cases that ship
`test.event` ground truth. Major gaps include multi-line scalar folding,
block scalars (`|` / `>`), explicit keys (`?`), the full double-quoted escape
set, and UTF-16/32 streams.

Track design: [superpowers/specs/2026-11-24-ozzyyaml-conformance-design.md](superpowers/specs/2026-11-24-ozzyyaml-conformance-design.md).

What *is* intentional and spec-aligned today: Unicode anchors, Core-schema
`yes`/`no` as strings, no 1.1 merge keys.
```

- [ ] **Step 2: Soften overclaims in README and crate READMEs**

Replace absolute “YAML 1.2.2 stack” wording with “YAML 1.2.2–oriented; see [docs/conformance.md](docs/conformance.md) for status.” Keep Unicode-anchor motivation.

- [ ] **Step 3: Commit**

```bash
git add docs/conformance.md README.md docs/architecture.md docs/python.md \
  crates/ozzyyaml-parser/README.md crates/ozzyyaml/README.md \
  crates/ozzyyaml-emit/README.md python/README.md
git commit -m "$(cat <<'EOF'
docs: stop claiming full YAML 1.2.2 until suite-complete

Point readers at conformance status while the language gap is closed.
EOF
)"
```

---

### Task 1: Plain and quoted line folding (Phase 1)

**Files:**
- Modify: `crates/ozzyyaml-parser/src/scanner/scalar.rs`
- Possibly Modify: `crates/ozzyyaml-parser/src/scanner/block.rs` (simple-key / indent interaction)
- Create: `crates/ozzyyaml-parser/tests/folding.rs`
- Modify: `crates/ozzyyaml-parser/tests/suite_allowlist.txt`

**Spec coverage:** YAML 1.2.2 §8.1.3 (line folding), §6.6 (escaped line break in double-quoted), §7.4.1 (simple keys must not be multi-line plains)  
**Test coverage:** Units below + allowlist every newly green ID (seed list: `9YRD`, `EX5H`, `36F6`, `HS5T`, `NB6Z`, `4CQQ`, `6WPF`, then discover more by running the harness)  
**Emitter:** no  

**Interfaces:**
- Consumes: existing `Scanner::scan_plain`, `scan_single_quoted`, `scan_double_quoted`, `Reader` newline normalization
- Produces: multi-line scalar strings with correct folded content; `parse_str` events matching suite `test.event` for greened IDs

- [ ] **Step 1: Write failing unit tests**

Create `crates/ozzyyaml-parser/tests/folding.rs`:

```rust
use ozzyyaml_parser::{parse_str, Event, ScalarStyle};

fn scalar_values(yaml: &str) -> Vec<(String, ScalarStyle)> {
    parse_str(yaml)
        .expect("parse")
        .into_iter()
        .filter_map(|e| match e {
            Event::Scalar { value, style, .. } => Some((value, style)),
            _ => None,
        })
        .collect()
}

#[test]
fn plain_folded_line_becomes_space() {
    // Suite-shaped: multi-line plain → single scalar with space fold (§8.1.3)
    let v = scalar_values("a\n  b\n");
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].0, "a b");
    assert_eq!(v[0].1, ScalarStyle::Plain);
}

#[test]
fn plain_blank_line_becomes_newline() {
    let v = scalar_values("a\n\nb\n");
    assert_eq!(v[0].0, "a\nb");
}

#[test]
fn double_quoted_escaped_line_break_suppresses_fold() {
    // §6.6 / suite 6WPF family
    let v = scalar_values("\"a\\\n b\"\n");
    assert_eq!(v[0].0, "ab"); // escaped break + leading ws trimmed per quoted folding rules
}

#[test]
fn single_quoted_folds_across_lines() {
    let v = scalar_values("'a\n  b'\n");
    assert_eq!(v[0].0, "a b");
}
```

Adjust the escaped-break expected string to match the suite `6WPF` `test.event` after reading that file — the assertion above is a starting point; **suite ground truth wins**.

- [ ] **Step 2: Run tests — expect FAIL**

```bash
cargo test -p ozzyyaml-parser --test folding
```

Expected: FAIL (plain stops at newline / quoted unterminated).

- [ ] **Step 3: Implement folding in `scan_plain` / quoted scanners**

Rewrite `scan_plain` to continue across newlines while indentation and context allow: fold single breaks to `' '`, preserve blank-line runs as `\n` per §8.1.3. Stop at comment, flow indicators (in flow), value indicators, and dedent.

Extend `scan_single_quoted` / `scan_double_quoted` to handle newlines inside quotes; in double-quoted, `\\` + break consumes the break without inserting space (escaped line break).

Keep functions focused; if `scalar.rs` grows past ~250 lines, extract `fold_break` helpers into `scanner/fold.rs`.

- [ ] **Step 4: Re-run unit tests — expect PASS**

```bash
cargo test -p ozzyyaml-parser --test folding
```

- [ ] **Step 5: Green suite IDs and allowlist**

```bash
# Compare candidate IDs (example):
cargo test -p ozzyyaml-parser --test yaml_test_suite -- --nocapture
```

For each of `9YRD EX5H 36F6 HS5T NB6Z 4CQQ 6WPF` (and any others that now match `test.event`), append the ID to `crates/ozzyyaml-parser/tests/suite_allowlist.txt` **only if** the harness would pass. Temporarily add one ID, run:

```bash
cargo test -p ozzyyaml-parser --test yaml_test_suite
```

- [ ] **Step 6: Commit**

```bash
git add crates/ozzyyaml-parser/src/scanner/scalar.rs \
  crates/ozzyyaml-parser/src/scanner/fold.rs \
  crates/ozzyyaml-parser/tests/folding.rs \
  crates/ozzyyaml-parser/tests/suite_allowlist.txt
git commit -m "$(cat <<'EOF'
feat(parser): multi-line plain and quoted scalar folding

Implement YAML 1.2.2 line folding so common multi-line scalars parse.
EOF
)"
```

---

### Task 2: Block scalars `|` / `>` (Phase 2)

**Files:**
- Create: `crates/ozzyyaml-parser/src/scanner/block_scalar.rs`
- Modify: `crates/ozzyyaml-parser/src/scanner/mod.rs` (wire `|` / `>` fetch)
- Modify: `crates/ozzyyaml-parser/src/token.rs` / parser node path as needed to emit `ScalarStyle::Literal` / `Folded`
- Create: `crates/ozzyyaml-parser/tests/block_scalars.rs`
- Modify: `crates/ozzyyaml-parser/tests/suite_allowlist.txt`

**Spec coverage:** §8.1.4–§8.1.6 (block scalar headers, chomping, indentation)  
**Test coverage:** Unit tests for `|-`, `|+`, `|2`, `>`; allowlist ~11 direct suite block-scalar IDs that go green  
**Emitter:** parse/events only (no dump style change yet)  

**Interfaces:**
- Consumes: `ScalarStyle::Literal` / `Folded` already in `event.rs`
- Produces: scalar events with those styles and chomped/indented values

- [ ] **Step 1: Failing tests in `tests/block_scalars.rs`**

```rust
use ozzyyaml_parser::{parse_str, Event, ScalarStyle};

#[test]
fn literal_preserves_newlines() {
    let events = parse_str("|\n  a\n  b\n").unwrap();
    let s = events.into_iter().find_map(|e| match e {
        Event::Scalar { value, style, .. } => Some((value, style)),
        _ => None,
    });
    assert_eq!(s, Some(("a\nb\n".into(), ScalarStyle::Literal)));
}

#[test]
fn literal_strip_chomp() {
    let events = parse_str("|-\n  a\n").unwrap();
    let val = events.into_iter().find_map(|e| match e {
        Event::Scalar { value, .. } => Some(value),
        _ => None,
    });
    assert_eq!(val.as_deref(), Some("a"));
}
```

Tune expected strings against suite `test.event` for the first allowlisted block-scalar ID you target.

- [ ] **Step 2: Run — expect FAIL** (`cargo test -p ozzyyaml-parser --test block_scalars`)

- [ ] **Step 3: Implement header parse + block content scan; emit Literal/Folded events**

- [ ] **Step 4: Tests PASS; allowlist greened block-scalar IDs**

- [ ] **Step 5: Commit** `feat(parser): block scalars | and >`

---

### Task 3: Full escapes + emit alignment + round-trip property (Phase 3)

**Files:**
- Modify: `crates/ozzyyaml-parser/src/scanner/scalar.rs` (`scan_escape`)
- Modify: `crates/ozzyyaml-emit/src/scalar.rs` (`double_quoted`)
- Create: `crates/ozzyyaml-parser/tests/escapes.rs`
- Create: `crates/ozzyyaml/tests/roundtrip.rs`
- Modify: allowlists as IDs green

**Spec coverage:** §7.3.2 `c-ns-esc-char` (incl. `\x` `\u` `\U`, `\b` `\v` `\f` `\e` `\N` `\_` `\L` `\P`)  
**Test coverage:** Escape matrix; property test; suite IDs unblocked by escapes; **fixes E1** (emit `\x` must parse)  
**Emitter:** **required**  

**Interfaces:**
- Consumes: Task 1 folding (double-quoted may span lines)
- Produces: parser accepts full escape set; emitter only emits escapes the parser accepts (prefer full set both sides)

- [ ] **Step 1: Failing escape tests**

```rust
// crates/ozzyyaml-parser/tests/escapes.rs
use ozzyyaml_parser::{parse_str, Event};

fn one_string(yaml: &str) -> String {
    parse_str(yaml)
        .unwrap()
        .into_iter()
        .find_map(|e| match e {
            Event::Scalar { value, .. } => Some(value),
            _ => None,
        })
        .unwrap()
}

#[test]
fn hex_escape_x() {
    assert_eq!(one_string("\"\\x41\"\n"), "A");
}

#[test]
fn unicode_escape_u() {
    assert_eq!(one_string("\"\\u0041\"\n"), "A");
}

#[test]
fn bell_and_nbsp_letter_escapes() {
    assert_eq!(one_string("\"\\b\"\n"), "\u{8}");
    assert_eq!(one_string("\"\\_\"\n"), "\u{a0}");
}
```

- [ ] **Step 2: Failing round-trip that exhibits E1**

```rust
// crates/ozzyyaml/tests/roundtrip.rs
use ozzyyaml::{from_str, Value};
use ozzyyaml_emit::to_string;
use std::rc::Rc;

fn values_equal(a: &Value, b: &Value) -> bool {
    // structural equality using PartialEq on Value — OK for acyclic trees;
    // for cycles, compare via Rc::ptr_eq on roots after shared compose only.
    a == b
}

#[test]
fn roundtrip_string_with_backspace() {
    let v = Value::String("\u{8}".into());
    let yaml = to_string(&v).unwrap();
    let back = from_str(&yaml).expect("load own dump");
    assert!(values_equal(&v, &back), "yaml was {yaml:?}");
}
```

- [ ] **Step 3: Implement parser escapes; implement emitter using the same table**

- [ ] **Step 4: `cargo test -p ozzyyaml-parser --test escapes` and `cargo test -p ozzyyaml --test roundtrip` PASS**

- [ ] **Step 5: Allowlist newly green suite IDs; commit**  
  `feat(parser,emit): complete double-quoted escapes and round-trip`

---

### Task 4: Explicit keys `?` (Phase 4)

**Files:**
- Modify: `crates/ozzyyaml-parser/src/scanner/block.rs` (and/or `flow.rs`)
- Modify: parser mapping states in `crates/ozzyyaml-parser/src/parser/`
- Create: `crates/ozzyyaml-parser/tests/explicit_keys.rs`
- Modify: `suite_allowlist.txt`

**Spec coverage:** Explicit key productions (§8.1.1 / §7.4.1 as used by suite)  
**Test coverage:** ~22 suite `?` cases + multi-line explicit key unit  
**Emitter:** no  

- [ ] **Step 1: Failing test** — `? a\n: b\n` produces a mapping with key `a` value `b`

- [ ] **Step 2: Implement `?` token + parser path**

- [ ] **Step 3: Allowlist greened `?` IDs; commit** `feat(parser): explicit keys`

---

### Task 5: Directives (Phase 5)

**Files:**
- Modify: `crates/ozzyyaml-parser/src/scanner/document.rs`
- Modify: `crates/ozzyyaml-parser/src/parser/document.rs` (version / TAG handling)
- Create: `crates/ozzyyaml-parser/tests/directives.rs`
- Modify: allowlist

**Spec coverage:** §9.1–§9.2 (`%YAML` version, `%TAG`)  
**Test coverage:** reject `%YAML 9.9`; accept `%YAML 1.2`; suite directive/tag IDs  
**Emitter:** no  

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn unsupported_yaml_version_is_error() {
    assert!(ozzyyaml_parser::parse_str("%YAML 9.9\n---\n0\n").is_err());
}

#[test]
fn yaml_1_2_directive_accepted() {
    assert!(ozzyyaml_parser::parse_str("%YAML 1.2\n---\n0\n").is_ok());
}
```

- [ ] **Step 2: Parse directive tokens instead of skip-to-EOL; validate version**

- [ ] **Step 3: Allowlist; commit** `feat(parser): %YAML and %TAG directive validation`

---

### Task 6: Policy alignment (Phase 6)

**Files:**
- Modify: `crates/ozzyyaml-parser/src/scanner/block.rs` (remove same-line nested mapping ban where grammar allows)
- Modify: `crates/ozzyyaml/src/schema/core.rs` (`resolve_scalar` → `Result` or throw compose error)
- Modify: `crates/ozzyyaml/src/compose.rs` / `error.rs` as needed
- Create/Modify: composer tests for strict tags
- Update any tests that expected the old rejection

**Spec coverage:** mapping productions; Core tag constraints  
**Test coverage:** same-line form that suite requires; `!!int twelve` errors; `!!int 42` ok; `!!str twelve` ok  
**Emitter:** no  

**Interfaces:**
- Produces: `resolve_scalar(...) -> Result<Value, Error>` (or compose-site checks) with strict Core tags

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn strict_int_tag_rejects_non_int() {
    let err = ozzyyaml::from_str("!!int twelve\n").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("int") || msg.contains("tag"), "{msg}");
}

#[test]
fn strict_int_tag_accepts_int() {
    assert!(matches!(
        ozzyyaml::from_str("!!int 42\n").unwrap(),
        ozzyyaml::Value::Int(42)
    ));
}
```

- [ ] **Step 2: Change `resolve_scalar` strictness; fix same-line mapping scan**

For known tags:

```rust
Some(INT_TAG) => try_int(value).ok_or_else(|| /* Compose error */)?,
```

`!!str` → always `Value::String`.

- [ ] **Step 3: Full `cargo test -p ozzyyaml-parser -p ozzyyaml`; commit**  
  `feat: accept legal same-line maps; strict Core tags`

---

### Task 7: Raise default Limits (Phase 7)

**Files:**
- Modify: `crates/ozzyyaml-parser/src/limits.rs`
- Modify: `crates/ozzyyaml/src/limits.rs` (inherits via `Default`)
- Modify: unit test `default_limits_are_conservative` — update assertions to match new defaults

**Spec coverage:** none (product)  
**Test coverage:** defaults are the new numbers; tiny custom limits still fail oversized input  
**Emitter:** no  

**New defaults:**

```rust
max_input_bytes: 256 * 1024 * 1024,
max_nesting_depth: 10_000,
max_anchors: 100_000,
```

- [ ] **Step 1: Update defaults + tests**

```rust
#[test]
fn default_limits_are_raised() {
    let l = Limits::default();
    assert_eq!(l.max_input_bytes, 256 * 1024 * 1024);
    assert_eq!(l.max_nesting_depth, 10_000);
    assert_eq!(l.max_anchors, 100_000);
}
```

- [ ] **Step 2: Keep a test that `Limits::default().max_input_bytes(8)` rejects a 9-byte input**

- [ ] **Step 3: Commit** `fix(limits): raise defaults; keep overrides`

---

### Task 8: Compose allowlist + emit round-trip breadth (Phase 8)

**Files:**
- Modify: `crates/ozzyyaml/tests/suite_allowlist.txt` (and compose harness)
- Modify: `crates/ozzyyaml/tests/roundtrip.rs` — widen to many allowlisted `in.yaml` files
- Modify: `crates/ozzyyaml-emit/` as bugs appear
- Optional: emit `|` / `>` for multi-line strings

**Spec coverage:** Core §10 (compose); emit correctness not presentation  
**Test coverage:** compose allowlist growth; round-trip over parser-event-green ∩ compose-able set  
**Emitter:** yes as needed  

- [ ] **Step 1: Extend roundtrip test to load allowlisted suite `in.yaml` paths, skip cases with cycles if `PartialEq` is unsafe — use a visited-set equality or only acyclic filter documented in the test**

- [ ] **Step 2: Fix emit/compose failures until round-trip is green for the chosen set**

- [ ] **Step 3: Grow compose allowlist with passing IDs only**

- [ ] **Step 4: Commit** `test(compose,emit): widen allowlist and round-trip coverage`

---

### Task 9: UTF-16 / UTF-32 streams (Phase 9, last) + final docs claim

**Files:**
- Create: `crates/ozzyyaml/src/encoding.rs` (BOM detect + decode to `String`)
- Modify: `from_slice` / `load_all_from_slice` paths to decode before parse
- Mirror in parser byte entrypoints if any
- Create: `crates/ozzyyaml/tests/encoding.rs`
- Modify: `docs/conformance.md`, `README.md`, crate READMEs — claim 1.2.2 with Limits + round-trip caveats
- Update design status to Done

**Spec coverage:** §5.2, §5.5  
**Test coverage:** BOM matrix (UTF-8 BOM, UTF-16LE/BE, UTF-32LE/BE) round-trip to same `Value` as UTF-8 equivalent; suite unchanged (UTF-8)  
**Emitter:** UTF-8 only  

- [ ] **Step 1: Failing encoding tests**

```rust
#[test]
fn utf16le_bom_loads() {
    // BOM FF FE + UTF-16LE for "a\n"
    let mut bytes = vec![0xFF, 0xFE];
    for u in "a\n".encode_utf16() {
        bytes.extend(u.to_le_bytes());
    }
    let v = ozzyyaml::from_slice(&bytes).unwrap();
    assert!(matches!(v, ozzyyaml::Value::String(s) if s == "a"));
}
```

(Adjust expected root type if plain `a` resolves differently — use a quoted/form that matches Core.)

- [ ] **Step 2: Implement `decode_yaml_bytes(&[u8]) -> Result<String>` with BOM detection**

- [ ] **Step 3: Wire `from_slice*`; tests PASS**

- [ ] **Step 4: Verify event suite still ≈100% on `test.event` cases; document any remaining known skips with justification**

- [ ] **Step 5: Docs claim YAML 1.2.2; `docs/conformance.md` marks track complete; commit**  
  `feat: UTF-16/32 BOM loaders; claim YAML 1.2.2 conformance`

---

## Track completion checklist

- [ ] Parser allowlist ≈ all suite IDs with `test.event`
- [ ] Strict Core tags; same-line maps accepted
- [ ] Round-trip property green on agreed set
- [ ] Defaults = 256 MiB / 10k depth / 100k anchors; overrides work
- [ ] UTF-16/32 on byte APIs
- [ ] Public docs match reality

## Spec coverage matrix (self-check)

| Design phase | Plan task |
| --- | --- |
| 0 Honesty | Task 0 |
| 1 Folding | Task 1 |
| 2 Block scalars | Task 2 |
| 3 Escapes + round-trip | Task 3 |
| 4 Explicit keys | Task 4 |
| 5 Directives | Task 5 |
| 6 Policy | Task 6 |
| 7 Limits | Task 7 |
| 8 Compose/emit breadth | Task 8 |
| 9 UTF-16/32 + claim | Task 9 |
