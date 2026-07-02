# QG Review — S57 Lane G (Reed-Solomon flag ergonomics)

**Verdict: PASS**

Independent Quality-Gate verification of the Signal Processing Specialist's S57 change. Every
claim below is re-derived from evidence produced by this reviewer (build/test/clippy/fmt output,
git scope, engine sha, and live bin invocations) — not from the implementer's report.

Change under review: `modem_encode` now honors bare `--rs-data`/`--rs-parity` (defaulting shard
size to 128 → `RS01` stream) instead of silently emitting a non-RS `PKT1` stream; hard-errors on
partial RS spec and on RS-flags-vs-non-legacy-`--coding-profile` conflict; `modem.rs` gains
`stream_is_rs01`; `modem_decode` gains an `RS01` auto-detect branch. +3 CLI round-trip tests.

---

## Stage 1 — Mechanical

| Check | Result |
|---|---|
| 1a `cargo build --release` | **PASS** — `Finished release profile`, no errors |
| 1b `cargo test` | **PASS** — every suite green, 0 failed |
| 1c `cargo clippy --all-targets` | **PASS** — 0 net-new warnings in changed files |
| 1d `cargo fmt --check` | **PASS (for this task)** — only pre-existing drift in untouched `tests/qg_s53_review.rs` |

### 1b — per-suite test counts (all `0 failed`)
- lib (`src/lib.rs`): **253 passed**
- bin `src/main.rs`: 14 passed
- `tests/modem_cli_roundtrip.rs`: **12 passed** (was 9 pre-S57; +3 new tests, confirmed)
- `tests/modem_roundtrip.rs`: 17 passed
- `tests/modem_realair.rs`: 10 passed
- `tests/cli_parse.rs`: 24 passed
- (all remaining music-pipeline / harness suites green: ab_harness_s31 3, affect_s22 8,
  cell_distinctness_s53 6, composition_s15 5, counterpoint_s30 13, diversity_s13 10,
  engine_equivalence 9, engine_seam 10, figuration_s20 8, home_s40 9, hue_gap_s41 11,
  keyplan_* 42 total, motif_s39 5, motif_s41 6, pattern_library_s34 16, phase2_pure_pipeline 7,
  prominence_s23 5, prominence_s43 6, qg_probe_band_isolation 1, qg_s53_review 8,
  render_cli_determinism_s55 2, rhythm_variety_s50 4, runtime_reachability_s37 3,
  s52_probe_identity 1, saliency_s18 12, seed_s41 5, texture_s17 7, tui_render 13,
  valence_mode_s36 7, variety_s45 5, variety_scorecard_s45 3)

### 1c — clippy net-new analysis
Total workspace warnings: 178, all pre-existing style/pedantic/nursery classes (doc-list
overindent, cast-same-type, div_ceil, loop-index, clamp pattern, etc.). No correctness-class
warnings anywhere.

Warning line locations in the four changed files were cross-checked against the inserted line
ranges (modem_encode.rs ~186–232, modem_decode.rs ~314–340, modem.rs ~785–800, test helper
~80–92 and new tests ~559+). **Zero warnings fall inside any inserted range.** The nearest hit,
`modem_encode.rs:182 (unwrap after is_some on preset_params)`, sits *above* the insertion point
(line 186) and is pre-existing code unrelated to the RS flags. **Net-new clippy warnings = 0.**

### 1d — fmt scope
`cargo fmt --check` reports diffs ONLY in `tests/qg_s53_review.rs` (a music-pipeline test NOT in
this change's scope — pre-existing drift, exactly as the implementer noted). **No fmt diff in any
of the four changed files.** Not this task's regression; noted for hygiene.

---

## Stage 2 — Boundary / Scope Audit

- `git status --porcelain` + `git diff --stat`: only modified files are
  `src/bin/modem_encode.rs` (+47), `src/bin/modem_decode.rs` (+25), `src/modem.rs` (+16),
  `tests/modem_cli_roundtrip.rs` (+168). All insertions, 0 deletions. **In-scope; no stray
  src/music-pipeline file touched.** (`cli.rs` unchanged — expected.)
- `src/engine.rs` sha256 = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`
  — **matches the required frozen hash.**
- Subsystem independence: `grep` for music-pipeline module imports (engine/render/music/
  composition/melody/harmony/rhythm/motif/keyplan/texture) in the three modem src files →
  **none found.** Modem subsystem is independent of the music pipeline.

---

## Stage 3 — Behavioral Verification (re-derived by running the bins)

Payload: 320 random bytes (`in.bin`, sha256 `2579f654…`). All invocations via
`cargo run --release --bin`, default features.

| # | Check | Result |
|---|---|---|
| (i) | Bare `--rs-data 4 --rs-parity 2` (no `--rs-shard-size`) → captured packet-stream magic | **`RS01`** (hex `5253 3031`) — encoder honored the flags, did NOT emit PKT1. Also emits the "defaulting shard size to 128 bytes" notice. |
| (ii) | Flagless `modem_decode` on that WAV | **RS01 auto-detect fired** ("Detected RS01 Reed-Solomon packet stream; auto-depacketizing… succeeded: 341 bytes"); recovered file **BYTE-EXACT** to input (`cmp` clean, 320 bytes). |
| (iii) | Partial `--rs-data 4` alone | **Exit 1**; stderr = "incomplete Reed-Solomon spec: --rs-data and --rs-parity must be supplied together…"; **no WAV written.** |
| (iv) | Conflict `--rs-data 4 --rs-parity 2 --coding-profile rs-high` | **Exit 1**; stderr = "conflicting FEC selection: …request a header-less RS01 stream, but --coding-profile RsHigh requests an in-band CDG1 stream…"; **no WAV written.** |
| (v) | Legacy no-flag byte-identity | Two no-flag encodes → **WAV byte-identical** and **packet-stream byte-identical**; no-flag stream magic = **`PKT1`** (unchanged). Exit 0. Diff confirms all new logic is gated on `rs_flags_present = cli.rs_data.is_some() \|\| cli.rs_parity.is_some()` (raw CLI flags), so the no-flag path never enters the new blocks. |
| (vi) | Regression strength | **Strong — tests genuinely lock the fix, not assert-true-ish.** Test 10 asserts captured magic `== RS01` (pre-fix = PKT1 → fails) AND decode log contains `RS01`+`auto` (pre-fix branch absent → fails) AND byte-exact recovery. Test 11 asserts non-zero exit + "conflicting FEC selection" substring + no WAV (pre-fix silently succeeded, profile won → fails). Test 12 asserts non-zero exit + "incomplete Reed-Solomon spec" + no WAV (pre-fix silently emitted PKT1 and succeeded → fails). The `run_expect_fail` helper asserts non-zero exit with a debuggable panic on unexpected success. Each of the three would fail on pre-fix code. |

Decode branch ordering was inspected: the new `RS01` branch is an `else if` placed after the
`expected_repeats` and `PKT1` branches. Because `RS01`/`PKT1`/`CDG1`/`AHX1` prefixes are disjoint,
the branch cannot displace the legacy or PKT1 paths — confirmed both by reading the code and by
(v) still producing/decoding PKT1 unchanged.

---

## Issues

**Blocking:** none.

**Non-blocking (pre-existing, not introduced by S57):**
1. `cargo fmt --check` drift in `tests/qg_s53_review.rs` (untouched music-pipeline test).
2. 178 pre-existing style/pedantic clippy warnings workspace-wide (0 in changed files).
3. Out-of-scope observation (not a regression): `--rs-shard-size` supplied *alone* (without
   data/parity) does not trigger RS, since `rs_flags_present` keys only on `rs_data`/`rs_parity`.
   Consistent with pre-S57 behavior; the S57 charter targets the bare data+parity path only.

---

## Verdict

**PASS.** Build green, full test suite green (12 modem_cli_roundtrip tests incl. the 3 new ones),
0 net-new clippy, fmt clean within scope, git scope limited to the four declared files,
`engine.rs` frozen at the required hash, modem subsystem independent of the music pipeline, and
all six Stage-3 behavioral properties independently confirmed (RS01 emission, flagless byte-exact
round-trip, both hard-errors with no WAV side effect, legacy PKT1 byte-identity preserved, and
regression-strong tests). The RS request no longer silently degrades to a non-RS stream.
