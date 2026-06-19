# Quality-Gate Review — S41 Finding-B "image-selected rhythm cells"

**Reviewer:** Quality Gate (Specialist 6 — verify, do not implement).
**Scope reviewed:** the uncommitted Finding-B slice on top of committed hue-gap fix `9d97d14`.
**Verdict:** **PASS**
**Date:** 2026-06-19

This review judges CORRECTNESS, FREEZE INTEGRITY, and TEST QUALITY only. The
taste/affect gate (Spec 8 ∥ Spec 9 — the band-edge cuts and the gait-vocabulary
musicality) runs AFTER this verdict. One precedence observation below is flagged
for the taste reviewers.

---

## 1. Freeze integrity — CONFIRMED

- `sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` —
  EXACT match to the asserted hash.
- `git diff --stat` changeset: `assets/mappings.json`, `src/chord_engine.rs`,
  `src/composition.rs`, `tests/composition_s15.rs`, `tests/motif_s39.rs`.
  **engine.rs is NOT in the changeset.** Freeze-safe.
- `cargo test --test engine_equivalence` → **9/9 PASS**, including
  `test_full_golden_sweep_is_byte_identical`. The realizer golden is byte-stable.

## 2. Cell-0 byte-identity hinge (the load-bearing freeze claim) — CONFIRMED

Independently re-derived: `git show HEAD:src/chord_engine.rs` old `rhythm_profile()`
vs. new `rhythm_cells()` cell 0, all 8 archetypes:

| Archetype | Old profile (S39) | New cell 0 | Match |
|---|---|---|---|
| Arch | `[2,1,1,2]` | `[2,1,1,2]` | ✓ |
| InvertedArch | `[2,1,1,2]` | `[2,1,1,2]` | ✓ |
| Descent | `[1,1,1,1,2]` | `[1,1,1,1,2]` | ✓ |
| Ascent | `[1,1,2]` | `[1,1,2]` | ✓ |
| NeighborTurn | `[1,1,1,1,2]` | `[1,1,1,1,2]` | ✓ |
| LeapStep | `[2,1,1,1,1]` | `[2,1,1,1,1]` | ✓ |
| Pendulum | `[2,2]` | `[2,2]` | ✓ |
| RisingSequence | `[1,1,2]` | `[1,1,2]` | ✓ |

`resolve_motif` is now `resolve_motif_celled(.., 0)`; every line below the
`let profile = archetype.rhythm_cell(cell_index)` substitution is byte-unchanged
from the S39 body (the cycle-and-cap accumulation loop is untouched). With cell 0
identical and the body identical, `resolve_motif` is behavior-preserving — the 11
existing call sites and all goldens stay byte-identical. **P5 pins this in-tree.**

## 3. Full test net — ALL GREEN

| Run | Result |
|---|---|
| `cargo build` (default) | OK (pre-existing unrelated warnings only) |
| `cargo test --lib --no-default-features` | 180/180 PASS |
| `cargo test --test engine_equivalence` | 9/9 PASS |
| `cargo test --test motif_s41` | 6/6 PASS |
| `cargo test --test motif_s39` | 5/5 PASS |
| `cargo test --test composition_s15` | 5/5 PASS |
| `cargo test` (default, ALL targets) | every target PASS; **0 failed workspace-wide** (223 lib + all integration suites) |

No failures anywhere.

## 4. Tests are real, not gamed — CONFIRMED

- **P1** drives `resolve_motif_celled(a, range, len, c)` on a FIXED archetype `a`,
  sweeping only the cell index. The contour is held by construction; only the gait
  varies. Asserts ≥2 distinct gaits AND that ≥1 cell escapes cell 0 (not an
  all-duplicate vocabulary). This is genuine same-contour-different-gait, NOT the
  trivial different-image-different-archetype.
- **P4** (clap-test proxy) holds a bright+flat affect corner so `pick_archetype`
  returns ONE archetype, asserts the archetype stays constant across the selector
  sweep, then asserts ≥3 distinct gaits on that one contour. `--nocapture` confirms:
  all 4 rows are `Arch`, cells 1/0/2/3, four distinct gaits
  (`[2,2,1]`/`[2,1,1,1]`/`[1,1,1,1,1]`/`[3,1,1,1,1]`). Real.
- **P2** threshold ≥10 vs the S39 ceiling of ~6. S39 ceiling re-derived as sound:
  pre-S41 only 5 distinct base profiles existed across 8 archetypes (Arch=InvertedArch,
  Ascent=RisingSequence, Descent=NeighborTurn shared), ~6 effective after range/length
  cap variation. The ≥10 floor is comfortably above 6 and the actual realized count
  is **23** (independently confirmed — see §4a). Meaningful.
- **P3** 40% single-gait cap. NOT a smell: the fixture set is deliberately skewed
  (2 of 4 edge points → cell 1; half the complexity points → cell 3), and the looser
  40% (vs motif_s39's 30%) is documented to accommodate that construction skew while
  still proving non-collapse. The actual dominant share is **17/128 = 13.3%** — vast
  margin under the cap, so it is not masking a near-collapse.
- **P5** is a real byte-identity check: `assert_eq!(celled, s39)` (full `MotifNote`
  equality, not degree-only) over a range×length grid for every archetype.
- **P6** is a real contract sweep: every archetype × every cell × a length grid
  (incl. degenerate 0/1/2 and over-long 12) asserting `dur_steps >= 1`, motif
  non-empty, and `Σ dur_steps == floored budget` exactly.

### 4a. "Realized gait count = 23" — INDEPENDENTLY RE-DERIVED

`cargo test --test motif_s41 -- --nocapture` P3 log:
`P3: n=128 cap=52 dominant gait [2, 2, 1] share=17 (of 23 distinct)`.
The 128-fixture planner sweep realizes **23 distinct gaits**, confirming the claim
and clearing both the ≥10 P2 floor and the S39 ceiling of 6 with large margin.

## 5. DP-B re-bless faithfulness — CONFIRMED (one strengthened)

- `tests/motif_s39.rs:test_vocabulary_spread_reaches_most_archetypes` — relaxed the
  identify loop from cell-0-only `resolve_motif` to ANY cell via
  `resolve_motif_celled(a, .., cell)`. This is a STRICT SUPERSET of the old match
  (cell 0 still covered) — the planner now selects non-zero cells, so the old match
  would mis-fail. Intent preserved: still asserts the vocabulary reaches most
  archetypes. Not weakened to a tautology.
- `tests/composition_s15.rs:test_returning_theme_is_identity_recall_not_fresh` —
  finds the planner-selected cell by matching the stored line against the vocabulary,
  then asserts byte-equality. The `.expect()` (fail if NO cell matches) carries the
  teeth — it still proves the stored theme IS a deterministic resolver line (a recall,
  not a fresh/random pick). It is actually STRENGTHENED: the old check compared
  degree-only; the new one asserts full degree+duration equality. The downstream
  "not flat / not degenerate" assertions remain intact. Not a tautology.

## 6. mappings.json mirror inert — CONFIRMED

`grep -rn "motif_rhythm\|cells\|rhythm_cells"` over `src/` returns ONLY chord_engine.rs's
own inline `rhythm_cells()` definitions; `src/mapping_loader.rs` has ZERO `motif`
references. The JSON `composition.motif_rhythm.cells` block is NOT loader-wired — inline
Rust is authoritative (S39 DP-6 discipline retained). The mirror was cross-checked: every
cell of every archetype matches the inline Rust byte-for-byte, and cell 0 of each matches
the S39 profile.

## 7. Selection-precedence question — SOUND (one item flagged to taste reviewers)

`pick_rhythm_cell` checks `complexity >= CELL_COMPLEXITY_PROFILED (0.66)` FIRST and, if
true, returns cell 3 (the character/syncopated gait) BEFORE the `edge_activity` density
ramp. **As CORRECTNESS this is sound:** the precedence is intentional and documented (the
"decorrelating tiebreak"), it is total/deterministic, the index is clamped via
`index.min(cell_count.saturating_sub(1))` (and again defensively in
`MotifArchetype::rhythm_cell`), and the `cell_count > 3` guard prevents indexing past a
shorter vocabulary. No panic path; no out-of-range. The `_archetype` parameter is
intentionally unused (the character tiebreak is archetype-independent — always cell 3).

**Flagged for the taste/affect reviewers (DP-A), NOT a correctness defect:** because the
complexity gate runs first, a high-complexity + high-edge_activity (busy) image is diverted
to cell 3 (profiled) and NEVER reaches cell 2 (the busy/even-subdivided gait). i.e. cell 2
can be starved for the high-complexity-high-activity quadrant. Whether that is the desired
affect mapping (a visually-intricate AND energetic image should sound "characterful/
syncopated" rather than "moto-perpetuo even") is a taste judgment, not a correctness one —
it is left to the Affect/Aesthetics gate to confirm or refine the band edges and precedence.

---

## Verdict: PASS

Freeze intact (engine.rs unchanged + 9/9 golden equivalence). Cell-0 byte-identity hinge
verified for all 8 archetypes and pinned by P5. Full workspace test net green (0 failures).
The S41 property net genuinely proves rhythm-from-contour decoupling (23 realized gaits vs
the S39 ceiling of 6; P1/P4 hold the contour fixed). Both sibling re-blesses preserve (one
strengthens) original intent. The JSON mirror is confirmed inert. The selection precedence
is correct and total; the one affect consequence (cell-2 starvation for the busy+intricate
quadrant) is correctly a taste call and is flagged forward.
