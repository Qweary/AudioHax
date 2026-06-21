# Quality Gate Review — Session 52 ("honesty cleanup", behavior-NEUTRAL)

**Verdict: PASS**

Reviewer: Quality Gate (independent). Repo HEAD `6fcb91c`. Review performed on the
uncommitted working tree (5 modified files + 1 new test). Findings below are from
independent inspection of the actual diff and a from-clean build/clippy/test run — not
a re-trust of prior agents' claims.

Files in scope (git status, exactly as expected — no out-of-scope modification):

```
 M assets/mappings.json
 M src/bin/modem_encode.rs
 M src/bin/unpack_tiled_payload.rs
 M src/mapping_loader.rs
 M src/modem.rs
?? tests/s52_probe_identity.rs
```

---

## STAGE 1 — MECHANICAL

| Gate | Result |
|------|--------|
| 1a. `cargo build --release` | **PASS** — `Finished release profile in 17.23s`, zero errors. |
| 1b. `cargo fmt -- --check` | **PASS** — clean, no output (exit 0). |
| 1c. `cargo clippy -- -W clippy::all` | **PASS** — exit 0; `seq`/`encoding` warnings CLEARED; ZERO net-new warnings. |
| 1d. `cargo test` (default features) | **PASS** — 546 passed / 0 failed across 45 result lines. |

### 1c detail — clippy delta (from-clean baseline vs working tree, like-for-like)

Captured by `git stash -u` → `cargo clean` → clippy at HEAD, then restore → `cargo clean`
→ clippy on working tree. (Separate-worktree baseline was discarded because the gitignored
`assets/soundfonts/default.sf2` is absent there and breaks compilation; stash keeps it.)

- HEAD baseline carried both target warnings:
  - `warning: value assigned to seq is never read` (`#[warn(unused_assignments)]`) in `modem_encode`
  - `warning: field encoding is never read` (`#[warn(dead_code)]`) in `unpack_tiled_payload`
- Working tree: BOTH gone (grep for unused/dead_code/seq/encoding returns nothing).
- Per-crate counts: `modem_encode` 6→4, `unpack_tiled_payload` 3→2; every other crate identical
  (lib 41=41, make_tiled_payload 4=4, channel_sim 5=5, modem_decode 6=6, audiohax 2=2).
- Total warning lines 74→71. Delta is strictly negative (3 removed, 0 added). **Zero net-new warnings.**

### 1d detail — explicitly required named suites

| Suite | Expected | Observed |
|-------|----------|----------|
| `s52_probe_identity` | 1 | **1 passed** |
| `engine_equivalence` | 9 | **9 passed** |
| `diversity_s13` | 10 | **10 passed** |
| `modem_realair` | 10 | **10 passed** |
| `modem_roundtrip` | 17 | **17 passed** |

Aggregate: **546 passed, 0 failed, 0 ignored** (lib 247 + main 14 + 44 integration/doc suites).

---

## STAGE 2 — BEHAVIOR-NEUTRALITY JUDGMENT (load-bearing)

**Verdict: every removal is provably behavior-neutral. Not one selection moves.**

### Schema-block removal (`instrument_section`, `fine_detail`)
- `git diff assets/mappings.json` removes exactly the two top-level blocks; `git diff
  src/mapping_loader.rs` removes their two structs (`InstrumentSectionMapping`,
  `FineDetailMapping`), their two fields on `MappingTable`, their two clone-arms in
  `rebuild_mapping_table`, AND the two JSON sub-objects in the in-file unit-test fixture —
  fully lockstep.
- `grep -rn "instrument_section|fine_detail|InstrumentSection|FineDetail" src/ assets/ tests/`
  returns ONLY one hit: a descriptive `//!` doc-comment line in the new test that *names* the
  removed blocks. Zero code references remain.
- Serde consistency: no struct field is left without its JSON and no JSON is left without its
  field. Deserialization of `assets/mappings.json` cannot break — the blocks are gone from both
  sides simultaneously, and the runtime never consumed them (no engine read path existed).

### The two `palette_bimodality le 0.3` conjuncts
- In HEAD, `palette_bimodality` appeared in EXACTLY two places (form `aaba` @ :156,
  key_scheme `aaba_excursion` @ :256), each as a `le 0.3` term. Both are removed; the
  `aspect_ratio ge 1.6` gate (and, for `aaba_excursion`, the `fg_bg_contrast ge 0.25` gate)
  and every other term are intact. No other rule referenced the knob, so its complete
  absence from the working tree is correctly scoped — only the two intended conjuncts went.
- Independent proof of no-op: the new identity test captures `palette_bimodality = 0.0` for
  ALL 6 probes (hard-pinned golden). `0.0 <= 0.3` is always true, so removing the conjunct
  cannot change whether either rule fires. Confirmed by the s52 test passing.

### `foreground_energy ge 0.015` — PRESERVED (correctly)
- `grep` confirms the term is STILL PRESENT in the TEXTURE SelectTable (`pad_bed_counter`
  rule), unchanged. The mappings.json diff does not touch the texture table at all — only the
  two schema blocks and the two form/key_scheme rules appear in the diff.
- Keeping it is correct. The identity test's golden pins `magicstudio-art.jpg` at
  `foreground_energy = 0.00308` (below the 0.015 floor) with `texture = pad_bed`, and the test
  carries an explicit D-FE guard that FAILS LOUDLY if any probe sits below the floor while the
  term is removed. Whether the floor is strictly load-bearing or (per the table's own
  `_comment`) a token floor superseded by `fg_bg_contrast ge 0.15`, the safe action — KEEP —
  was taken, and the gate pins the outcome either way.

### Identity gate is genuine (not vacuous)
- `tests/s52_probe_identity.rs` pins, per probe, real per-image values via the PUBLIC
  `SelectTable::select(&u)` surface over `pm.form` / `pm.key_scheme` / `pm.texture`, plus raw
  `foreground_energy` and `palette_bimodality`, against golden constants captured at clean HEAD.
  Hard `assert_eq!` / `assert!` on every row — no `assert!(true)`. magicstudio texture golden
  = `pad_bed`, as required. It passes on the cleaned-up tree, which is the proof.

### Dead-code removals (`seq`, `encoding`)
- `seq` (modem_encode): a write-only accumulator (`seq += 1`, never read). `grep seq` returns
  nothing post-removal. Cannot affect `enc_bytes` or any emitted output. Neutral.
- `encoding` (unpack_tiled_payload `TileEntry`): `TileEntry` derives `Deserialize` and is built
  from `serde_json::from_slice`. The field was `never read` (clippy dead_code at HEAD). Removal
  is safe because serde has no `deny_unknown_fields`, so a manifest still carrying an
  `"encoding"` key deserializes fine (unknown key silently ignored) and no code path consumed
  the value. Deserialization cannot break. Neutral.

**Reasoning conclusion:** the only removed selector terms are always-true (`palette_bimodality`
pinned 0.0) ; the preserved-token floor stays; the schema blocks and dead vars were unread.
No removal can change a runtime selection or a rendered note. The s52 identity gate independently
witnesses this for all 6 shipped probes (form/key_scheme/texture + raw knobs unchanged).

---

## STAGE 3 — MODULE BOUNDARIES & SCOPE

- `src/engine.rs` UNTOUCHED:
  `sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`
  — matches the expected hash exactly (verified at start and end of review).
- `src/modem.rs` is COMMENTS-ONLY: a filter over `git diff src/modem.rs` for any changed line
  that is not a comment (`*`, `///`, `//`, `/*`) or blank returns EMPTY. The S7 block header,
  the `SyncMode` doc, the `recover_symbol_timing` docstring, and the two 2a/2b section banners
  were rewritten to match live code; no executable modem line changed. `RsRate`, the
  burst/stride recovery, and the chirp path are unchanged (modem_realair 10/10 still green).
- No out-of-scope file modified: `git status --short` shows exactly the 5 listed files + the
  1 new test, nothing else.

---

## BLOCKING ISSUES

None.

## NON-BLOCKING NOTES

1. The task brief framed `foreground_energy ge 0.015` as load-bearing because magicstudio sits
   below the floor. The TEXTURE table's own `_comment` (and the s52 test golden) show magicstudio
   would resolve to `pad_bed` regardless, because the `pad_bed_counter` rule also requires
   `fg_bg_contrast ge 0.15`, which magicstudio (ct=0.084) fails independently. The term may be a
   token floor rather than strictly biting. This does not change the verdict — KEEPING it is the
   correct, conservative action and the test pins the outcome either way. Flagged only so the
   operator knows the "load-bearing" rationale rests on the fg_bg_contrast gate, not solely on fe.
2. Pre-existing clippy style warnings (doc_lazy_continuation, needless_range_loop, etc.) remain
   across the crate; they predate this session and are out of scope. No correctness-class
   (`-W clippy::correctness`) warning is present on the working tree.
