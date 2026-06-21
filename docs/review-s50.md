# Quality Gate Review — S50 RHYTHM-VARIETY RE-RANGE

**Reviewer:** Quality Gate (AudioHax)
**Date:** 2026-06-20
**Scope:** Uncommitted working-tree slice killing the cross-piece rhythmic SAMENESS defect (every input image produced the same `[dotted-quarter, eighth]→triplet→long-note` motif).
**Files reviewed:** `src/chord_engine.rs`, `src/composition.rs`, `assets/mappings.json`, `tests/rhythm_variety_s50.rs` (new), `tests/affect_s22.rs`, `tests/keyplan_s29.rs`, `tests/motif_s41.rs` (re-blesses). `src/engine.rs` confirmed FROZEN/untouched.

**VERDICT: PASS WITH ISSUES** (issues are documentation-staleness only; no blocking issues, no functional defects).

---

## Compilation Status

`cargo build --release` — **PASS.** Finished clean. The only warnings are pre-existing modem-binary warnings (`unused variable seq`, `value assigned to seq never read` in `src/bin/modem_encode.rs`) unrelated to this slice.

## Lint Status

- `cargo fmt -- --check` — **PASS** (exit 0, no diff).
- `cargo clippy --release -- -W clippy::all` — **no correctness warnings (non-blocking).** All clippy output is style-level (doc-list indentation, `div_ceil`/`is_multiple_of` idiom suggestions, loop-index hints) or pre-existing modem/FEC/synth warnings. None are in the new `band_activity_spread` code or the changed selector regions; the chord_engine/composition hits are all at lines far from the S50 edits (893, 3956, 4022, 259, 1694) and predate this slice.

## Test Results (per binary)

`cargo test --release` — **ALL GREEN.** 34 integration binaries + lib/main unit tests. Highlights:

| Binary | Result | Notes |
|---|---|---|
| lib (`audiohax`) unit | 247 passed / 0 failed | incl. the `s47_melody_activity_class_prom_shift_lowers_cutoffs` re-bless |
| `engine_equivalence` | **9 passed / 0 failed** | HARD GATE 2 |
| `diversity_s13` | **10 passed / 0 failed** | HARD GATE 3 (articulation goldens byte-identical) |
| `variety_scorecard_s45` | passed (F5b==0 per image) | HARD GATE 4 |
| `variety_s45` | passed | S45 counter still moves |
| `rhythm_variety_s50` (new) | **4 passed / 0 failed** | the decisive scorecard |
| `affect_s22` | 8 passed / 0 failed | 2 re-blesses |
| `keyplan_s29` | 4 passed / 0 failed | 1 re-bless |
| `motif_s41` | 6 passed / 0 failed | test_p4 re-author |
| `modem_realair` | 10 passed | unaffected (37s) |
| `modem_roundtrip` | 17 passed | unaffected |

No skipped/ignored tests. Every `tests/*.rs` file ran (cross-checked file list vs. run list — zero gaps).

## Module Boundary Audit

- **`chord_engine.rs`** — no image/MIDI logic. Grep for `pure_analysis`/`midi`/`image::`/`midir` imports returns empty. The new `band_activity_spread` is a pure, RNG-free free fn receiving an `f32` parameter. ✓
- **`mapping_loader.rs`** — no hardcoded musical/character values. The floats present are normalization divisors and the empty default-template skeleton; no `Character::`/character-name literals. The S50 change is entirely in the JSON data, not the loader. ✓
- **`composition.rs`** — changes are selector-logic constants + comments + one test re-bless. No seam/contract change. ✓
- **`assets/mappings.json`** — **data-only, schema-compatible.** New rules use the identical SelectTable schema: same op set {ge, lt, le, in_range}, same `{when, pick}` structure, same rule count (5), same knobs (arousal/valence). Only numeric `lo` thresholds moved. The OLD mappings.json parses under the unchanged loader → backward compatible content change, NOT a schema change. ✓
- **File ownership** — exactly the expected files are dirty; no file modified outside its owning agent's lane. ✓

## Musical Logic Review

**`band_activity_spread` (the band side).** Verified by direct math evaluation over [0,1]:
- **Monotone non-decreasing** ✓
- **Fixed point at CENTER**: `spread(0.40) == 0.40` ✓ (the band ladder is byte-neutral at the reference activity — the freeze hinge)
- **Identity at gain == 1.0** ✓ (the gate can disable the spread without disturbing goldens)
- **Genuinely re-spreads, no over-drive**: on the real six-image cluster the calm tail (magic 0.106→0, Img1 0.301→0.222) reaches SUSTAINED; the mid-cluster (Img2 0.509→0.553, Img3 0.475→0.505, Lena 0.471→0.499) lands SYNC/DOTTED; and ONLY the genuinely-busy `example` (0.719→0.847) reaches ARPEGGIO. The asymmetric slope (GAIN_LOW 1.8 > GAIN_HIGH 1.4) is the intended over-drive guard — mid-cluster images are NOT flung into ARPEGGIO. ✓

**Character gate change.** Real planner output shows genuine character spread across the six: Lament / Hymn / Scherzo / March (4 distinct) — no longer the universal Ballad pin. The deadzone closure (`march valence lt 0.55 | scherzo valence ge 0.55`) partitions the energetic band exactly at 0.55 with **no gap and no overlap** (lt vs ge). Sound. ✓

**Cell revert.** Correct given the theme gate. `pick_rhythm_cell` runs only on the theme path (complexity ≥ 0.4); a PROFILED gate ≤ 0.4 would force-pin every themed image onto cell 3 and kill the cells 0/1/2 edge ramp. Reverting PROFILED to 0.66 (above the 0.4 theme gate) keeps the edge ramp reachable for themed images in [0.4, 0.66). The invariant BROAD(0.33) < BUSY(0.66) holds. ✓

## Test Quality Assessment

`tests/rhythm_variety_s50.rs` validates **meaningful properties**, not weak `is_ok`/non-empty checks:
- (1) ≥4 distinct (band, cell, character) tuples (floor strictly above the pre-S50 collapse of 1; below the achievable 5) PLUS a directional pin: busiest `example` == ARPEGGIO, calmest `magic` == SUSTAINED.
- (2) ≥3 distinct tempos AND ≥3 distinct characters (un-pin witnesses).
- (3) ≥3 distinct dominant bands AND ≤1 documented signature collision (the genuinely-similar Img3~Lena mid-cluster pair, explicitly pinned).
- (4) determinism (two passes byte-identical).

Real `--nocapture` run confirms the floors bite well below the achieved spread: 4 bands, 4 characters, 6 tempos, single documented collision. Observables are read RNG-free off the pure realizer / seeded planner. Strong net.

**Re-blessed fixtures** — all four preserve their INTENDED property rather than rubber-stamping the new code:
- `affect_s22` monotone-tempo: **legitimate exercise, not a dodge.** The old fixture sat at a brightness where the raw BPM was already inside both March and Scherzo windows, so the character crossing changed nothing — a non-exercising fixture. Re-calibrated to a low-brightness base (raw BPM 87.2) where the Lament ceiling (66) and March floor (96) actually bite, so BPM genuinely rises 66→96 as saturation/arousal rises. This exercises "more arousal → faster" across a real character-window crossing.
- `affect_s22` fall-through: correctly re-targets the surviving (0.30, 0.34) unclassified deadzone with valence outside nocturne's band → Ballad. Property intact.
- `keyplan_s29` density: re-calibrated edge_density 0.04→0.033 so HOME isn't saturated at ARPEGGIO by the spread; home (spread 0.764, SYNCOPATED, 2 onsets) vs excursion (spread 0.869, ARPEGGIO, 3 onsets) still proves "higher-density excursion is busier" across a real band boundary.
- `motif_s41` test_p4: re-authored to an edge sweep at FIXED complexity 0.5 (in [0.4, 0.66) → theme present, PROFILED divert not fired). 0.10→cell1, 0.50→cell0, 0.80→cell2 genuinely proves ≥3 distinct gaits on the single held Arch archetype — directly proving the cell revert restored edge-ramp reachability for themed images.

## Integration Assessment

The band/character/cell changes compose without type or contract breaks (build + full suite green). No leftover TODO indicating incomplete integration (the cell-axis revert is a deliberate, documented decision pending fix-direction-2, not an incomplete edit). The decisive defect IS addressed: the six bundled images now spread across 4 bands, 4 characters, and 6 tempos, with only the documented Img3~Lena mid-cluster tie remaining.

## HARD-GATE Results

1. **engine.rs FROZEN** — **PASS.** `sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` (matches exactly).
2. **engine_equivalence 9/9** — **PASS.**
3. **Freeze discipline** — **PASS.** `band_activity_spread` is applied at exactly three sites: its definition (chord_engine.rs:1097), `melody_activity_class` (1147), and the band-ladder comparison `band_edge` (2677). It is NOT applied to the articulation curve (line 2140 uses raw normalized `edge_activity`), the FILL_REST check (reads raw `edge_activity`), or the `/0.05` (EDGE_ACTIVITY_RANGE_MAX) normalization. `diversity_s13` 10/10 byte-identical. ✓
4. **S46/S49 figure-ground preservation** — **PASS.** `variety_scorecard_s45` green (F5b `bg_recession_violations == 0` per image across all six; melody stays most-active/on-top/loudest because the spread is mirrored inside `melody_activity_class` so the governor sees the true spread class); `variety_s45` green (the S45 counter still moves).

## Blocking Issues

**None.**

## Non-Blocking Issues

1. **NB-1 (doc staleness in the new test header).** `tests/rhythm_variety_s50.rs` lines 10-11 still describe "TIGHTENED cell cuts (BROAD 0.38 / BUSY 0.50 / PROFILED 0.20)" — the values from the abandoned cell-side re-range. The cell axis was REVERTED in composition.rs (0.33/0.66/0.66). The test BODY reads live planner values and is correct; only the module-header narrative is stale. Cosmetic; recommend a one-line correction.
2. **NB-2 (pre-existing).** Unused-variable warnings in `src/bin/modem_encode.rs` (`seq`). Not in scope for this slice; pre-existing.
3. **NB-3 (clippy backlog, pre-existing).** ~40 style-level clippy warnings across lib + bins (doc-list indentation, idiom suggestions). None correctness, none in S50 code; tracked separately.

## Overall Verdict

**PASS WITH ISSUES.** All four hard gates pass. The slice is functionally correct, the band spread math is sound (monotone, fixed-point-neutral, gain-1.0-identity, no over-drive), module boundaries are respected, the mappings.json change is data-only/backward-compatible, and the test net validates meaningful cross-piece-distinctness properties with floors strictly above the pre-S50 collapse value. The four re-blessed fixtures each preserve their intended property (notably the affect_s22 monotone-tempo, which is genuinely re-exercised rather than dodged). The only issues are documentation staleness (NB-1) and pre-existing lint (NB-2/NB-3) — none blocking.
