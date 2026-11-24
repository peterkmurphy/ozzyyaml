# Grammar-Complete Parse (100% Events) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the ozzyyaml event parser to 100% YAML 1.2.2 production fidelity for structural parse and 100% of valid yaml-test-suite `test.event` cases (255/255 at design baseline).

**Architecture:** Grammar-shaped rewrite of structural scanning/parsing behind the existing `parse_str` / `Event` API. Keep Chapter 5 `chars.rs`, Reader, and escape tables. Replace indent-heuristic block logic, wrong anchor charset, incomplete flow-pair paths, and incorrect quoted folding with production-parameterized code (`s-indent(n)` as context). Dual harness in Phase 0; single full-suite gate in Phase 6.

**Tech Stack:** Rust 2021 (`ozzyyaml-parser` first; `ozzyyaml` / emit / Python as regression consumers), vendor `yaml-test-suite`, `cargo test`.

**Spec:** `docs/superpowers/specs/2026-11-25-ozzyyaml-grammar-complete-design.md`

## Global Constraints

- Never claim 100% event coverage in public docs until Phase 6 is green.
- Never allowlist a failing suite ID; Phase 6 deletes the subset allowlist.
- Public `parse_str` / `Parser` / `Event` remain the façade (SemVer-safe).
- Re-run **all** valid `test.event` cases after every phase (Phase 0 report + allowlist gate).
- `cargo test` workspace + Python `maturin develop && pytest` after each phase.
- `#![forbid(unsafe_code)]`; keep edited `.rs` files under ~300 lines (split modules).
- No PEG codegen, emit presentation, merge keys, or Serde in this plan.
- Commit after each task; human pushes.

## File structure (lock-in)

| Path | Role |
| --- | --- |
| `crates/ozzyyaml-parser/tests/yaml_test_suite.rs` | Allowlist gate + **new** full-suite report / final gate |
| `crates/ozzyyaml-parser/tests/suite_allowlist.txt` | Regression gate until Phase 6; then remove or stub |
| `crates/ozzyyaml-parser/src/chars.rs` | `[102] ns-anchor-char` replaces practical helper |
| `crates/ozzyyaml-parser/src/scanner/mod.rs` | Anchor fetch; dispatch |
| `crates/ozzyyaml-parser/src/scanner/scalar.rs` | Quoted / plain folding per `[74]` / `[112]`–`[116]` |
| `crates/ozzyyaml-parser/src/parser/flow.rs` | Flow pairs / empty keys |
| `crates/ozzyyaml-parser/src/scanner/flow.rs` | Flow tokens / `:` adjacent |
| `crates/ozzyyaml-parser/src/scanner/block.rs` | Block structure rewrite (Phase 5) |
| `crates/ozzyyaml-parser/src/scanner/block_scalar.rs` | Indent / tab (`R4YG`) |
| `crates/ozzyyaml-parser/tests/grammar_*.rs` | **Create** per-phase unit/suite wrappers |
| `docs/conformance.md` | Phase 6: 100% events claim |

---

### Task 0: Dual suite harness

**Files:**
- Modify: `crates/ozzyyaml-parser/tests/yaml_test_suite.rs`
- Test: same file

**Interfaces:**
- Produces: `fn valid_event_suite_ids(root: &Path) -> Vec<String>` — top-level dirs with `in.yaml` + `test.event`, no `error` file
- Produces: ignored (or `#[ignore]`) test `report_all_valid_suite_failures` that prints FAIL lines; keep `allowlisted_cases_match_expected_events` as hard gate

- [ ] **Step 1: Add helper + ignored full-suite report test**

```rust
fn valid_event_suite_ids(root: &std::path::Path) -> Vec<String> {
    let mut ids: Vec<_> = std::fs::read_dir(root)
        .expect("suite root")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|id| {
            let d = root.join(id);
            d.join("in.yaml").is_file()
                && d.join("test.event").is_file()
                && !d.join("error").exists()
        })
        .collect();
    ids.sort();
    ids
}

#[test]
#[ignore = "Phase 0 report: run with --ignored --nocapture"]
fn report_all_valid_suite_failures() {
    let root = suite_root();
    let allow: std::collections::HashSet<_> = allowlist().into_iter().collect();
    let mut fail = 0usize;
    let mut pass = 0usize;
    for id in valid_event_suite_ids(&root) {
        let dir = root.join(&id);
        let input = std::fs::read_to_string(dir.join("in.yaml")).unwrap();
        let expected: Vec<String> = std::fs::read_to_string(dir.join("test.event"))
            .unwrap()
            .lines()
            .map(String::from)
            .collect();
        match parse_str(&input) {
            Ok(ev) if render(&ev) == expected => {
                pass += 1;
            }
            Ok(ev) => {
                fail += 1;
                let on_allow = allow.contains(&id);
                println!("FAIL {id} mismatch allowlisted={on_allow}");
                let _ = ev;
            }
            Err(e) => {
                fail += 1;
                println!("FAIL {id} error allowlisted={} {e}", allow.contains(&id));
            }
        }
    }
    println!("SUMMARY pass={pass} fail={fail}");
}
```

- [ ] **Step 2: Run allowlist gate (must PASS) and report (ignored)**

```bash
cargo test -p ozzyyaml-parser --test yaml_test_suite allowlisted_cases_match_expected_events
cargo test -p ozzyyaml-parser --test yaml_test_suite report_all_valid_suite_failures -- --ignored --nocapture
```

Expected: allowlist PASS; report shows ~22 FAIL IDs matching the design table.

- [ ] **Step 3: Commit**

```bash
git add crates/ozzyyaml-parser/tests/yaml_test_suite.rs
git commit -m "test(parser): dual harness for full event-suite reporting"
```

---

### Task 1: `[102]` ns-anchor-char (suite `W5VH`)

**Files:**
- Modify: `crates/ozzyyaml-parser/src/chars.rs` (`is_anchor_char` → spec)
- Modify: `crates/ozzyyaml-parser/src/scanner/mod.rs` (`fetch_anchor_or_alias` stop rules)
- Create: `crates/ozzyyaml-parser/tests/grammar_anchors.rs`
- Modify: `crates/ozzyyaml-parser/tests/suite_allowlist.txt` (add `W5VH` only when green)

**Spec:** `[102] ns-anchor-char ::= ns-char - c-flow-indicator`  
(`ns-char` = `nb-char - s-white`; includes `"`, `'`, `#`, `:`, etc.)

- [ ] **Step 1: Failing tests**

In `chars.rs` tests and `grammar_anchors.rs`:

```rust
#[test]
fn ns_anchor_char_matches_spec() {
    assert!(is_anchor_char('"'));
    assert!(is_anchor_char(':'));
    assert!(is_anchor_char('#'));
    assert!(!is_anchor_char(',')); // flow indicator
    assert!(!is_anchor_char(' '));
}

#[test]
fn suite_w5vh() {
    // Load vendor/yaml-test-suite/W5VH — events must match test.event
    assert_suite_events("W5VH");
}
```

Copy the local `assert_suite_events` helper pattern from `tests/explicit_keys.rs` (suite_root + render).

- [ ] **Step 2: Run — expect FAIL**

```bash
cargo test -p ozzyyaml-parser --lib chars::tests::ns_anchor_char_matches_spec
cargo test -p ozzyyaml-parser --test grammar_anchors
```

- [ ] **Step 3: Implement**

```rust
pub(crate) fn is_anchor_char(c: char) -> bool {
    is_ns_char(c) && !is_flow_indicator(c)
}
```

Adjust `fetch_anchor_or_alias` terminator: after the name, require whitespace / flow-indicator / end (existing), and ensure `:` inside the name is allowed when not a value indicator boundary per suite `W5VH` (`&:@*!$"<foo>:`). Prefer consuming with `is_anchor_char` until a stop that matches the suite events — add a focused unit if stop rules break Unicode anchors (existing `unicode_anchors` tests must stay green).

- [ ] **Step 4: PASS tests; add `W5VH` to allowlist; re-run allowlist + report**

```bash
cargo test -p ozzyyaml-parser
# add W5VH to suite_allowlist.txt
cargo test -p ozzyyaml-parser --test yaml_test_suite
```

- [ ] **Step 5: Commit**

```bash
git add crates/ozzyyaml-parser/src/chars.rs crates/ozzyyaml-parser/src/scanner/mod.rs \
  crates/ozzyyaml-parser/tests/grammar_anchors.rs crates/ozzyyaml-parser/tests/suite_allowlist.txt
git commit -m "fix(parser): ns-anchor-char per YAML 1.2.2 [102]"
```

---

### Task 2: Quoted multi-line folding (`7A4E` `9TFX` `PRH3` `T4YY` `NP9H` `Q8AD` `NAT4`)

**Files:**
- Modify: `crates/ozzyyaml-parser/src/scanner/scalar.rs` (`fold_quoted_breaks`, double/single scan)
- Create: `crates/ozzyyaml-parser/tests/grammar_quoted_fold.rs`
- Modify: allowlist when green

**Spec focus:** `[74] s-flow-folded`, `[113]` `s-double-break`, `[114]` `nb-ns-double-in-line`, `[115]` `s-double-next-line`, `[116]` `nb-double-multi-line`; single-quoted analogues. Leading whitespace on continuation lines must not become extra spaces the way the current folder does (`folded  to` vs `folded to`).

- [ ] **Step 1: Failing suite wrappers**

```rust
#[test] fn suite_7a4e() { assert_suite_events("7A4E"); }
#[test] fn suite_np9h() { assert_suite_events("NP9H"); }
#[test] fn suite_prh3() { assert_suite_events("PRH3"); }
#[test] fn suite_nat4() { assert_suite_events("NAT4"); }
// likewise 9TFX, Q8AD, T4YY
```

- [ ] **Step 2: Run — expect FAIL** (extra spaces / empty quoted mismatch)

- [ ] **Step 3: Implement folding per productions**

Rewrite `fold_quoted_breaks` so that:

1. A single line break with only `s-white` on the next line folds to **one space** (not keeping prior trailing spaces + extra).
2. More-indented / blank-line rules match `[74]` / suite examples 7.5–7.6 / 7.9 (use `in.yaml` + `test.event` as oracle while coding).
3. `NAT4`: empty / whitespace-only quoted strings produce the exact scalar values in `test.event` (often empty or spaces-only — do not trim illegally; do not invent newlines).

Keep escaped line breaks (`\` + break) discarding the break.

- [ ] **Step 4: PASS; allowlist the seven IDs; full parser tests**

- [ ] **Step 5: Commit**

```bash
git commit -m "fix(parser): quoted line folding per §7.3 / s-flow-folded"
```

---

### Task 3: Flow pairs / empty keys (`CFD4` `9MMW` `5MUD` `K3WX` `7ZZ5`)

**Files:**
- Modify: `crates/ozzyyaml-parser/src/parser/flow.rs`
- Modify: `crates/ozzyyaml-parser/src/scanner/flow.rs`
- Modify: `crates/ozzyyaml-parser/src/parser/mod.rs` (states if needed)
- Create: `crates/ozzyyaml-parser/tests/grammar_flow_pairs.rs`
- Allowlist when green

**Spec:** `[150]` `ns-flow-pair`, `[146]` empty key entries, `[139]` `ns-flow-seq-entry`, adjacent `:` on following line (`5MUD` / `K3WX`).

- [ ] **Step 1: Failing suite wrappers for the five IDs**

- [ ] **Step 2: Run — expect FAIL**

- [ ] **Step 3: Implement**

1. Single-pair implicit entries in flow sequences (`[ YAML : separate ]`, JSON-like keys) — ensure `FlowPair*` states accept empty keys (`[ : empty key ]`, `[: another]`).
2. Flow mapping value `:` may appear on the next line after a key (and after a comment line) — extend `is_value_indicator` / possible-key recovery so `{ "foo"\n  :bar }` works.
3. Nested empty `[]` / `{}` under block (`7ZZ5`) — accept empty flow collections as entries/values without “unexpected token”.

Prefer production-shaped helpers (e.g. `parse_flow_pair`) over new one-off flags.

- [ ] **Step 4: PASS; allowlist five IDs; `cargo test -p ozzyyaml-parser`**

- [ ] **Step 5: Commit**

```bash
git commit -m "fix(parser): flow pairs and empty keys per §7.4"
```

---

### Task 4: Block indentation indicator + tab (`R4YG`)

**Files:**
- Modify: `crates/ozzyyaml-parser/src/scanner/block_scalar.rs`
- Create: `crates/ozzyyaml-parser/tests/grammar_indent.rs`
- Allowlist `R4YG` when green

**Spec:** `[163] c-indentation-indicator`, `[63] s-indent`, `[68] s-block-line-prefix`. Suite `R4YG` uses a **tab** where indentation spaces are expected in one folded scalar — today’s scanner errors; ground truth accepts the document. Match `test.event` exactly (do not invent a private stricter-than-suite rule).

- [ ] **Step 1: Failing `assert_suite_events("R4YG")`**

- [ ] **Step 2: Run — expect FAIL** (tab indentation error)

- [ ] **Step 3: Implement**

Compare `in.yaml` / `test.event` for `R4YG`. Adjust block-scalar break/indent scanning so tabs in the positions this example uses produce the expected folded/literal values. Keep rejecting tabs where the grammar/`s-indent` still requires spaces **if** that does not fail `R4YG`. Add a unit comment citing §8.2 example 8.2.

- [ ] **Step 4: PASS; allowlist `R4YG`**

- [ ] **Step 5: Commit**

```bash
git commit -m "fix(parser): block scalar indentation for suite R4YG"
```

---

### Task 5: Compact / same-indent block collections (8 IDs)

**Files:**
- Major modify: `crates/ozzyyaml-parser/src/scanner/block.rs`
- Possibly: `crates/ozzyyaml-parser/src/parser/mod.rs` / `node.rs`
- Create: `crates/ozzyyaml-parser/tests/grammar_block_compact.rs`
- Allowlist: `AZ63` `RLU9` `S9E8` `AB8U` `57H4` `S3PD` `6PBE` `SKE5`

**Spec:** `[183]`–`[195]` compact sequences/mappings, same-indent seq under map, zero-indent seq under `?`, empty key + seq value, tags on nested collections (`57H4`), anchor before zero-indent seq (`SKE5`).

This is the grammar rewrite core: stop treating `roll_indent` alone as “more indented ⇒ new collection.” Structure must follow `ns-l-block-map-entry` / `c-l-block-seq-entry` / `ns-l-compact-*` with explicit indent context `n`.

- [ ] **Step 1: Failing suite wrappers for all eight IDs**

- [ ] **Step 2: Run — expect FAIL**

- [ ] **Step 3: Implement in slices (still one commit series OK)**

Recommended internal slices (each with its own failing→green subcycle before moving on):

1. **Same-indent sequence under mapping** (`AZ63`, `RLU9`, `S9E8`) — `-` at the indent of the parent key’s value column starts a block sequence without requiring deeper indent than the key.
2. **Explicit-key zero-indent seq** (`6PBE`) — `?` / `:` entries whose values are sequences at column of `?`.
3. **Empty key + sequence** (`S3PD`) — `: # Both empty` / `"quoted key":\n- entry`.
4. **Looks-like-nested but isn’t** (`AB8U`) — multiline plain vs nested seq; match suite events.
5. **Tagged nested collections** (`57H4`) — `!!seq` / `!!map` with compact children.
6. **Anchor before zero-indent seq** (`SKE5`) — properties apply to the following compact sequence.

Replace or narrow `roll_indent` / `unroll_indent` so they implement production indent checks. Add parser-level acceptance where tokens are correct but states reject.

After each slice: `cargo test -p ozzyyaml-parser --test grammar_block_compact` and allowlist gate.

- [ ] **Step 4: All eight green; allowlist updated; full parser + compose tests**

```bash
cargo test -p ozzyyaml-parser
cargo test -p ozzyyaml
```

- [ ] **Step 5: Commit**

```bash
git commit -m "fix(parser): compact and same-indent block collections per §8.2"
```

(Use multiple commits during slices if clearer; final message as above for the completing commit.)

---

### Task 6: Full-suite gate + docs

**Files:**
- Modify: `crates/ozzyyaml-parser/tests/yaml_test_suite.rs` — hard-fail on **all** `valid_event_suite_ids`
- Modify/delete: `crates/ozzyyaml-parser/tests/suite_allowlist.txt`
- Modify: `docs/conformance.md` (100% events; keep emit/Core caveats)
- Modify: `README.md` / crate READMEs if they still say ~91%
- Regression: `cargo test`, Python pytest

- [ ] **Step 1: Switch harness**

Replace allowlist-only assertion with:

```rust
#[test]
fn all_valid_event_suite_cases_match() {
    let root = suite_root();
    let mut failures = Vec::new();
    for id in valid_event_suite_ids(&root) {
        // same compare logic as today’s allowlisted test
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
```

Remove or empty `suite_allowlist.txt` (if other crates reference parser allowlist count, update comments only). Keep compose allowlist separate unless those IDs are already covered.

- [ ] **Step 2: Run full gate — expect PASS (255/255 or current valid count)**

```bash
cargo test -p ozzyyaml-parser --test yaml_test_suite all_valid_event_suite_cases_match
cargo test
cd python && . .venv/bin/activate && maturin develop -q && pytest -q
```

- [ ] **Step 3: Update docs**

`docs/conformance.md`: event suite **100%** of valid `test.event` cases; note remaining non-goals (emit presentation, Core-only). Link this design/plan.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(parser): 100% yaml-test-suite event coverage"
```

---

### Task 7 (optional): Differential fuzz stub

**Files:**
- Create: `crates/ozzyyaml-parser/tests/fuzz_oracle.md` or a ignored test documenting how to diff against another 1.2.2 event emitter  
- Only if requested after Task 6

Skip unless the human asks.

---

## Done when

- `all_valid_event_suite_cases_match` is green.
- Design success criteria met; conformance docs updated.
- Compose/emit/Python regressions green.
- Optional Task 7 not required for “Done.”
