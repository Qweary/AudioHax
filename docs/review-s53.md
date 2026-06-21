# Quality Gate Review — S53 slice 1 (fix-direction-2 / D-CELL)

**Reviewer:** AudioHax Quality Gate (independent — not the Producer).
**Slice:** Un-gate the rhythm-cell axis to run per-piece — every real image gets a `RhythmMotto`
(an `(archetype, cell_index)` gait identity) that biases melody onset placement, decoupled from the
dead theme gate. engine.rs FROZEN.
**HEAD reviewed against:** `64b6883b058a55987c11a7e4eb0da8234babc51c`
**Verdict:** **PASS** (no blocking issues; two non-blocking notes).

---

## 1. Compilation / Lint / Test Results

| Gate | Result | Notes |
|---|---|---|
| `cargo build --release` | **PASS** | clean, 16.5s |
| `cargo fmt -- --check` | **PASS** | no diff |
| `cargo clippy -- -W clippy::all` | **PASS (no net-new)** | 41 lib warnings — VERIFIED pre-existing: HEAD (via `git stash`) also reports exactly 41. Zero warnings cite any slice line (`pick_piece_cell`, `apply_motto_onset_bias`, `RhythmMotto`, `MOTTO_*`, `PIECE_*`, `.motto`). No net-new correctness warning. |
| `cargo test` (default features) | **PASS** | **557 passed, 0 failed** across the full suite. |
| `cargo test --no-default-features` | n/a (pre-existing breakage) | Confirmed broken repo-wide at HEAD — test binaries import `pure_analysis`/`synth_sink`/`image`/`rustysynth`, all feature-gated out (the documented Cargo.toml:20 issue). NOT introduced by this slice; noted as observation only per the review brief. |

New/changed tests all green: `cell_distinctness_s53` (6/6), `rhythm_variety_s50` (4/4), the
`pick_piece_cell` unit net in `chord_engine::tests` (5/5), and the QG independent net (8/8, below).

---

## 2. Freeze Verification — STAGE 2 (rigorous)

**engine.rs sha256 — EXACT MATCH.**
`sha256sum src/engine.rs` = `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` —
byte-for-byte equal to the freeze anchor. `git diff HEAD -- src/engine.rs` is EMPTY. engine.rs
contains zero occurrences of `motto` (`grep -c` = 0). **The freeze holds.**

**Class-A engine-kernel goldens — all byte-green, none re-baselined:**
- `engine_equivalence` — **9/9** byte goldens pass.
- `seed_s41` — pass incl. `test_pt_seed_5_engine_frozen` (the ENGINE_SHA256 witness) and
  `test_pt_seed_4_none_preserves_legacy_path`.
- `affect_s22` — pass incl. `byte_freeze_witness_locked_files_unmoved`.
- `variety_scorecard_s45` — pass incl. `scorecard_engine_frozen` (and the F5b hard-gate, §4 below).
- `s52_probe_identity` — pass.
None of these test files appear in the diff; their goldens are untouched.

**The carrier deviation — INDEPENDENTLY JUDGED CORRECT.**
The Producer reports the seam spec's `Section.motto` field was incompatible with the frozen
engine.rs (the engine's `legacy_default_section` open-codes a `Section {…}` literal), so the motto
rides on `OrchestrationProfile` (a `#[serde(skip)]` planner-set field) instead, read via
`ctx.section.orchestration.motto` with a `Section::motto()` accessor restoring the design ergonomics.
I verified all three sub-claims independently:

- **(a) engine.rs compiles unchanged + sha-identical.** Confirmed (above). The mechanism is real:
  `engine.rs:769` builds its frozen `Section` literal via `OrchestrationProfile::identity()` — a
  *constructor call*, not an open-coded `OrchestrationProfile {…}` literal. Adding a field to
  `OrchestrationProfile` and seating it inside `identity()` therefore leaves the engine.rs literal
  textually unchanged. A bare `Section.motto` field would have forced an edit to that literal. The
  deviation is not a workaround of convenience — it is the *only* placement that preserves the freeze.
- **(b) The freeze hinge is real — neutral motto ⇒ byte-identical output.** `RhythmMotto::neutral()`
  has `cell_index == None`; `is_neutral()` short-circuits the realizer read; `identity()` /
  `single_section_default` / `legacy_default_section` all carry `neutral()`. The 9/9
  engine_equivalence goldens (which flow through these) are byte-green, proving the hinge. My own
  net additionally locks neutral ≡ cell-0 ≡ cell-2 (the authored zero-displacement gaits).
- **(c) `OrchestrationProfile` is a sound home — NOT a boundary violation.** It is the established
  carrier for planner-set, per-section-cloned realization params (`prominence`,
  `bass_pattern_resolved`, `density`). The motto is exactly that class of value. `Copy`, rides the
  existing `orchestration.clone()` with zero allocation. Sound.

---

## 3. Musical Logic Review — STAGE 3 (load-bearing scrutiny)

**`pick_piece_cell(edge_activity, complexity, archetype, cell_count)` — correct.**
- Driver is **`complexity`** (NOT `affect_arousal`) — matches the Affect specialist's adjudicated
  contradiction of the seam spec's original proposal. The const is `PIECE_COMPLEXITY_PROFILED = 0.20`
  (NOT the reverted themed-path `CELL_COMPLEXITY_PROFILED = 0.66`, which is untouched in composition.rs).
- PRIMARY axis uses `band_activity_spread(edge_activity)` against `PIECE_EDGE_BROAD/BUSY` (0.33/0.66)
  on the BROAD→BUSY ramp (cells 1/0/2).
- SECONDARY cell-3 divert guarded by `complexity >= 0.20 && primary != 2 && cell_count > 3` — the
  cell-2 guard keeps the busy outlier (example) even. Verified the guard is load-bearing (without it,
  distinct-cell count drops 4→3).
- **Takes scalars, not an image type** — `chord_engine` imports no `ImageUnderstanding`. Module
  boundary preserved.

**6-probe cell table — EXACT MATCH to the Affect target, verified TWO ways.**
The Producer's unit test reproduces it off the isolated selector; I additionally drove the **real
planner path** (`CompositionPlanner::plan` over the six shipped images, reading the stamped
`Section.motto.cell_index`) in `qg_six_probe_table_off_real_planner` — both agree:

| image | cell | image | cell |
|---|---|---|---|
| AudioHaxImg1 | 1 | example | 2 |
| AudioHaxImg2 | 0 | Lena | 0 |
| AudioHaxImg3 | 3 | magicstudio | 3 |

4 distinct cells {0,1,2,3}; exactly 2 on cell 3 (GR-3 selective). The Img3~Lena watch-pair is split
(Img3→3, Lena→0) and magicstudio's paradox-gait is resolved (→3). No probe lands elsewhere.

**GR-1 (figure-ground) — structurally enforced, not merely asserted.**
The motto application (`chord_engine.rs:2804`) is gated `matches!(role, OrchestralRole::Melody)` and
calls `apply_motto_onset_bias`, which is **count-preserving by construction** (it walks existing
interior onsets and re-places them; it never `push`es/`pop`s). It touches only the Melody arm — the
bed roles never enter the code path. So bed onset counts are invariant and no voice's `ActivityClass`
can be promoted ⇒ the spec-s46 F5b background-recession metric is untouched by construction. I
verified this in production (not just by test): my `qg_bed_roles_untouched_only_melody_biased` drives
a 4-instrument ensemble and confirms inst 0 (Bass) + inst 1/2 (HarmonicFill) are **byte-identical**
under neutral vs cell-3, while only inst 3 (the Melody, the figure) is biased. The F5b==0 hard gate
in `variety_scorecard_s45` (real whole-plan render over all six images) still passes (§4).

> Note: under `instrument_role`, the *highest* instrument is the Melody (not inst 0). The bias
> correctly lands there. This is the intended figure-ground location.

**GR-2 (cadence relaxation) — structurally enforced.**
The `if is_cadence { … return … }` cadence ring (`chord_engine.rs:2251–2280`) early-returns BEFORE
the `match role` block that contains the motto application — so the motto is **structurally zero at a
cadence**, and the single sustained ritardando ring is unperturbed. The pre-cadence approach applies
`MOTTO_PRECADENCE_ATTEN = 0.5` (the bias is halved). The downbeat anchor (first onset, index 0) is
explicitly skipped (`for i in 1..n`). I verified all three independently:
- `qg_cadence_step_ignores_motto`: cell-3 motto == neutral at a PAC step (single sustained ring).
- `qg_precadence_attenuates_bias`: pre-cadence displacement ≤ full-strength interior displacement,
  and full-strength displacement is genuinely non-zero.
- The downbeat anchor is fixed in every cell-1/cell-3 case.

**Onset-bias REAL, not gamed.** `apply_motto_onset_bias` displaces each interior onset by a signed
fraction (`MOTTO_ONSET_BIAS_DEPTH = 0.18` × per-cell character weight × atten) of the *gap* to its
neighbour, clamped to keep ≥1ms separation (strict ordering, never crosses a boundary, never
re-orders). cell 1 = −0.6 (earlier pull), cell 3 = +1.0 (later lean), cells 0/2 = 0.0 (keep grid).
I confirmed the *audio observable actually moves*: `qg_cell3_displaces_interior_preserves_count_anchors_downbeat`
(cell 3 moves an interior onset, count preserved, ordered, in-step) and `qg_cell1_biases_distinctly_from_cell3`
(cell 1 is a distinct, live, earlier-pulling walk). A motto that "selected a cell" but produced
identical audio would be gamed — it is not; the audio genuinely moves for non-neutral mottos, and
neutral/cell-0/cell-2 are genuine byte-identical no-ops.

**`RhythmMotto.cell_index: Option<usize>` (None = neutral) — sound.** cell 0 is a real, selectable
value (it is the S39 anchor and a legitimate per-piece motto), so neutrality MUST be distinct from
every real cell. `None` is that distinct sentinel; a "cell 0 == neutral" encoding would have made a
genuine cell-0 piece indistinguishable from the freeze path. Correct design.

---

## 4. Test Quality + Golden Handling — STAGE 4

**Class-A goldens unchanged/frozen.** Confirmed — see §2. None re-baselined.

**Class-B goldens re-pointed with justification.**
- `rhythm_variety_s50.rs`: the diff touches ONLY the `realized_cell` helper body (re-pointed from
  the always-empty `themes[0].motif` to `section.motto().cell_index`, the live engine-read
  observable) and its doc comments. **The §5.2-B justification hunk is present** ("…the no-theme
  rhythmic observable is now `section.motto().cell_index`… the S52 honesty invariant is
  PRESERVED…"). Critically, **no assertion floor was loosened** — the `>= 4` tuple-distinctness,
  `<= 1` collision, and `>= 3` band/tempo/character thresholds are byte-unchanged and still pass with
  the now-live cell axis. The Img3~Lena permitted-tie logic is intact. This is the *honesty
  strengthening* the design promised: the observable moved from a cell the realizer never read to one
  it actually reads.
- `cell_distinctness_s53.rs` is a NEW file (untracked, not a re-baseline of an existing golden). Its
  assertions check REAL properties — the exact 6-probe table, ≥3 distinct cells, the {0,1,2,3} set,
  matching-pairs == 2 (down from the dormant 7), and a live onset-bias test that confirms the motto
  displaces an interior onset count-preservingly with the downbeat anchored and a neutral no-op. No
  `assert!(true)` placeholders.

**S50 cross-piece spread preserved.** `rhythm_variety_s50` 4/4 green — band/character/tempo spread
floors hold; the motto did not collapse the existing three axes.

**Mechanical test edits (9 files, +2/+3 lines each).** `counterpoint_s30`, `figuration_s20`,
`keyplan_s25`, `motif_s39`, `pattern_library_s34`, `prominence_s23`, `prominence_s43`, `saliency_s18`,
`texture_s17` each gained only `use … RhythmMotto;` + `motto: RhythmMotto::neutral(),` field
additions to their test-local `OrchestrationProfile` literals — forced by the new non-defaulted
field. Pure mechanical compliance, NOT re-baselines.

**Independent QG net (`tests/qg_s53_review.rs`, added by this review).** 8/8 green — re-derives the
freeze hinge, onset-bias reality/ordering/bounds, GR-1 bed-invariance + melody-only bias, GR-2
cadence/pre-cadence relaxation, and the 6-probe table off the real planner. Caught (and corrected in
my own assertion) the inst-3-is-melody stratification fact, which confirmed the bias lands on the
figure.

---

## 5. Integration — STAGE 5

- `composition.rs` imports `RhythmMotto`, stamps the motto once in `plan()` (after the affect seat,
  alongside the once-per-plan resolves) onto `orchestration`, which is `.clone()`d onto every section
  (uniform-per-piece grain). `Section::motto()` accessor reads it back. Type-coherent across
  `composition.rs ↔ chord_engine.rs`.
- `pick_piece_cell` is called with `(u.edge_activity, u.complexity, piece_archetype,
  piece_archetype.rhythm_cell_count())` — scalars only; boundary clean.
- No broken callers; full suite (557) compiles and passes. No stray TODO/incomplete-integration
  markers in the slice.

---

## 6. Blocking Issues

**None.**

## 7. Non-Blocking Issues / Observations

1. **`--no-default-features` is broken repo-wide** (pre-existing Cargo.toml:20 issue; the headless
   test path the design's §5.1 step 7 names). Not a slice defect — `cell_distinctness_s53.rs`'s
   header claims it "runs under `--no-default-features`," but that path does not currently build for
   *any* test binary. The file does run fine under default features. Worth fixing the headless path
   separately so the design's intended headless determinism guarantee is actually exercisable.
2. **`pick_piece_cell`'s `archetype` parameter is unused** (`_archetype`) — the selector keys only on
   the two scalars + `cell_count`. This is fine (the archetype names which vocabulary the cell
   indexes downstream, and is carried on the `RhythmMotto`), but the parameter is currently vestigial
   in the selector body. Harmless; flagged only for awareness.

---

## 8. Overall Verdict

**PASS.** The freeze is intact (engine.rs sha-exact, 9/9 + all kernel goldens byte-green, carrier
deviation independently judged correct and in fact the only freeze-preserving placement). The musical
logic is correct and not gamed: the driver is the adjudicated `complexity` axis, the 6-probe table
matches exactly off both the unit selector and the real planner, the onset bias genuinely moves audio
count-preservingly, and GR-1/GR-2 are *structurally* enforced (melody-only, count-preserving,
cadence early-return, downbeat anchored) rather than merely test-asserted. Golden handling is clean —
Class-A frozen, Class-B re-pointed to the honest live observable with justification and no floor
loosening. Ready for the lead to integrate.
