# spec-s46-figure-ground-metrics.md — Figure-Ground / Role-Balance Metric Specification (S46)

**Author:** Rust Architect (SYNTHESIS round, S46). **Consumer:** Test Engineer (extends the harness).
**Status:** design / spec only — no code in this doc. **Companion:** `docs/design-s46-figure-ground.md`
(the unified work order), `docs/spec-s45-variety-metrics.md` (the per-layer variety spec this EXTENDS),
`tests/variety_scorecard_s45.rs` (the harness this spec adds metrics INTO — not a new harness).

**Freeze:** `src/engine.rs` is BYTE-FROZEN at sha256
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` (re-verified UNCHANGED at the
start AND end of this session). This spec is test-only; the existing `ENGINE_SHA256` guard
(`tests/variety_scorecard_s45.rs:68`) is unchanged and still asserts the sha.

---

## 0. Purpose and the boundary it draws

The S45 variety scorecard answers, per layer, *"does this layer MOVE (vary), or is it
flat/dormant/cloned?"* It does **not** answer the orthogonal S46 question: *"do the moving layers
stand in the right figure-ground RELATIONSHIP — is the melody the FIGURE and the rest the GROUND?"*
A layer can PASS S45 (it varies) and FAIL S46 (it varies as much as the figure → it competes; the
S45 CounterMelody is exactly this case). This spec adds the **cross-layer figure-ground HIERARCHY**
metrics F1–F5 that measure that relationship.

It EXTENDS spec-s45; it does **not** duplicate the S45 layers M1–M7. The S45 per-layer rows stay
as-is; F1–F5 are a NEW cross-layer block in the same harness. The S43 LEVEL invariants (resolved
Melody prominence > 0.5; melody velocity ≥ loudest bed role) are the level-only floor F1–F5 add the
ACTIVITY/REGISTER companions to.

### 0.1 The data + render path — REUSED VERBATIM from the S45 harness (no new render path)

F1–F5 are computable from the **exact** stream the S45 harness already collects. Per the
Architecture lens, they SLOT INTO the same `scorecard_for` body
(`tests/variety_scorecard_s45.rs:317`) and the same `LayerVerdicts` struct (`:305`), reusing:

- `render(&plan, &perf) -> RenderStreams` (`:173`) — the per-role `Vec<StampedEvent>` map
  (`by_role`) + per-section role sets, already collected for ALL roles each step.
- `per_step_pitch(events) -> Vec<(usize, u8)>` (`:268`) — per-step representative pitch per role.
- `motion_dir(a, b) -> i32` (`:262`) — motion sign, already used by M1.4.
- `step_shape_key(step_evs, ms_per_step) -> (usize, Vec<(u64,u64)>)` (`:288`) — onset count +
  sorted (offset,hold) fractions; the onset COUNT is the first tuple element, exactly the
  per-step onset density F1/F5 need.
- the per-(step,role) onset grouping already built for M5.1 (`step_shape_key` over the step's
  events) and the per-step `offset_ms` maps already built for M1.3 (`pad_off_by_step`,
  `counter_off_by_step`, `:402-415`).

**No new render path, no new image type, no audio, no OpenCV** — the same headless discipline as
S45. F1–F5 read the streams `scorecard_for` already has in hand.

### 0.2 The RNG boundary — REUSED VERBATIM from spec-s45 §0.2

Same `set_composition_seed(Some(SEED))` before every `plan()` (`tests/variety_scorecard_s45.rs:322`),
same `SEED = 42` (`:62`). Each F-metric is tagged **DETERMINISTIC** (structural / RNG-free — onset
count, offset phase, register FLOOR ordering) or **SEEDED** (absolute pitch — the chord draw),
exactly per the spec-s45 boundary. The seeded parts replay byte-identically under SEED 42.

### 0.3 The reference image set + the routing asymmetry — REUSED from spec-s45 §0.3

Same 6 probe images. The figure-ground hierarchy between the **Melody and the CounterMelody** is
only measurable where the counter is routed — **AudioHaxImg1/2/3** (`pad_bed_counter`). On the
non-counter routes (example, Lena, magicstudio) the counter is absent, so the counter-relative
parts of F1/F2/F5 report `N/A — counter not routed` (NOT a failure), exactly as M1 does. The
melody-vs-Pad/Fill parts of F1/F3/F4/F5 ARE measurable on all six.

### 0.4 The IMAGE-CONDITIONING discipline (load-bearing — the metric-rigidity guard)

This is the single most important instruction, reconciling the Affect hard-fail metrics with the
Aesthetics metric-rigidity caution (work-order tension #2). **The figure-ground bar is RELATIONAL
and IMAGE-CONDITIONED, never absolute.** A first-class engine does NOT make the melody mechanically
busiest/highest/loudest on every image — a flat-maximum figure-ground is itself a defect (a melody
that shouts equally hard on a still ambient image is as mechanical as one that never leads). The
encoding:

- The per-image **figure-strength** signal is `fg_bg_contrast` (modulated by `subject_size`),
  already on `ImageUnderstanding` (read in `scorecard_for` for the printed knobs row, `:335-336`).
  Define a binned figure-strength class per image:
  - **SUBJECT** (high `fg_bg_contrast`): a strong lead is justified — the melody MUST clearly win.
  - **MID**: a moderate lead.
  - **FIELD** (low `fg_bg_contrast`): an even texture is CORRECT — the melody need not win, only
    not-lose.
- Each F-metric whose first-class threshold is a MARGIN is gated by a function of figure-strength:
  large required margin on SUBJECT, ≈0 on FIELD. The **SIGN** (melody not strictly out-competed by
  the bed) is the floor on EVERY image; the **MARGIN** scales with the image.
- The rollup (§3) weights the NON-LEVEL cues (F1 activity, F2 recession, F3 highest, F5
  rhythm-distinctness) ABOVE the level floor, so a build that wins by "turning the melody up"
  scores WORSE than one that wins by making the melody move more and sit on top — the scorecard
  encoding of the operator's 90/10.

On the 6-image set, `fg_bg_contrast` ranges 0.052–0.341 (spec-s45 §0.3); AudioHaxImg1 (0.341) is
the strongest figure-strength, magicstudio (0.084 region) / Lena the weakest — so the conditioning
visibly differs across the columns.

---

## 1. The five figure-ground metrics

Each is a pure function of the streams `scorecard_for` already collects. Where a metric reuses an
S45 quantity, the reuse is named. "onset count for role R on step s" = the first element of
`step_shape_key(step_evs_of(R,s), ms_per_step)` — the count of `NoteEvent`s role R emitted on
step s, RNG-free.

### F1 — MELODY-IS-MOST-ACTIVE (operator signal 2; the strongest cue, Affect rank 1–2)

**Perceptual definition (Affect FG-1):** the figure must generate the densest onset stream; the
melody's onset density must exceed every other concurrent role's by an image-justified margin. The
DIRECT figure-ground correctness gate.

- **(a) Metric.** Per role, `onset_density(R) = total onsets of R / sounding steps of R` over the
  render (onsets per step, reusing the per-step onset COUNT). Then
  `F1_margin = onset_density(Melody) − max over other present roles of onset_density(other)`.
  Also the per-step form for the hard sign: on each step where the melody and a bed role
  co-sound, `melody_onsets(step) ≥ bed_onsets(step)`. *DETERMINISTIC* — onset count is the rhythm
  template (RNG-free), the same data M5.1/M3.2 read.
- **(b) FIRST-CLASS THRESHOLD — image-conditioned (§0.4).** `F1_margin ≥ f(fg_bg_contrast)` where
  `f` is large on SUBJECT images (≈ +0.3 onsets/step, the Affect "bed sustains ~1/step, figure
  averages ≥ ~1.3" bar), ≈ 0 on FIELD images. **HARD floor on EVERY image:** `F1_margin ≥ 0` —
  i.e. the melody is NEVER strictly less active than the busiest bed role. A NEGATIVE margin (the
  literal operator signal 2 — background busier than foreground) is a HARD figure-ground failure.
- **(c) FAILING signature + scorecard line.** `F1_margin < 0`:
  `Figure-ground F1 melody-most-active: INVERTED — bed busier than melody (margin −0.42, counter
  1.00 vs melody 0.58 onsets/step) [HARD FAIL]`. A near-zero-but-nonneg margin on a SUBJECT image:
  `F1: WEAK — melody only marginally most-active on a subject image (margin +0.05, thr +0.30)`.
- **DET/SEEDED:** **DETERMINISTIC.**
- **Expected today:** on AudioHaxImg1/2/3 the counter has a guaranteed off-beat onset on
  calm/static steps while the melody falls to SUSTAINED (one onset) → `F1_margin` ≈ 0 or NEGATIVE
  → reveals the inversion. REPORTED pre-fix (see §2 on which metric carries the assertion).

### F2 — BACKGROUND-RECESSION (activity) (operator signal 5; the §2 activity finding)

**Perceptual definition (Affect FG-2):** background roles recede in ACTIVITY, not only level. The
bed generates fewer onsets and moves less than the figure.

- **(a) Metric.** For each bed role (Pad, HarmonicFill, CounterMelody-when-routed):
  `activity_ratio(bed) = onset_density(bed) / onset_density(Melody)` (reuse F1's densities) and
  `motion_ratio(bed) = motion_fraction(bed) / motion_fraction(Melody)`, where motion_fraction
  reuses the M1.2 per-step-pitch-change computation (`per_step_pitch` + the moves/holds windows,
  `:381-398`) applied per role. Plus the carried-forward S43 LEVEL floor: bed velocity < melody
  velocity on accented steps (reuse the resolved `realize_velocity` output already in the
  stream's `ev.velocity`). *activity_ratio DETERMINISTIC; motion_ratio SEEDED* (pitch-change
  needs absolute pitch).
- **(b) FIRST-CLASS THRESHOLD — image-conditioned.** `activity_ratio(bed) ≤ g(fg_bg_contrast)`
  with `g ≈ 0.7` on SUBJECT images (the bed generates ≤ ~70% of the figure's onsets), relaxing
  toward `≈ 1.0` on FIELD images (an even texture is allowed). **HARD floor on EVERY image:**
  `activity_ratio(bed) ≤ 1.0 + ε` for every bed role — no bed role is strictly MORE active than
  the melody (this is the F5 invariant in ratio form; see §2). The CounterMelody on counter-routed
  images is the live watch-item.
- **(c) FAILING signature + scorecard line.** `Figure-ground F2 bg-recession: COUNTER COMPETES —
  counter activity_ratio 1.18 (thr ≤0.70 subject / ≤1.0 hard), motion_ratio 1.40 [HARD via F5]`.
- **DET/SEEDED:** activity ratio **DETERMINISTIC**; motion ratio **SEEDED**.
- **Expected today:** the counter's guaranteed onset gives it `activity_ratio ≥ 1.0` against a
  SUSTAINED melody on calm images → reveals the competition.

### F3 — MELODY-IS-HIGHEST (operator signal 3; register, Affect rank 3)

**Perceptual definition (Affect FG-3, Aesthetics §5.1.2):** the figure occupies the top of the
texture; the melody's pitch is the highest sounding pitch on the vast majority of steps —
RELATIONAL (melody vs the actual concurrent voices), not a fixed pitch floor.

- **(a) Metric.** On each step where the melody and ≥1 other role co-sound, compare the melody's
  representative pitch (`per_step_pitch` for Melody) against each other role's representative pitch
  this step (reuse the per-step pitch maps M1.4 already builds, `mel_by_step` `:434-440` and the
  analogous counter/pad maps). `F3_frac = steps where melody_pitch ≥ max(other concurrent
  pitches) / co-sounding steps`. *SEEDED* — absolute pitch (chord draw); pin under SEED 42 exactly
  as M1.2/M4.1 are. (An RNG-INVARIANT floor exists via the register floors: melody floor 67 >
  fill/counter floor 55 → structurally melody-on-top UNLESS the dark-image lift drops the melody
  or a counter tone seats high — exactly the failure modes F3 catches.)
- **(b) FIRST-CLASS THRESHOLD.** `F3_frac ≥ 0.95` (Aesthetics §5.1.2; brief deliberate crossings
  allowed as flagged exceptions). RELATIONAL, so it does not freeze the melody into a register.
  Not image-conditioned by margin (melody-on-top is a near-universal arrangement value), but the
  seat-order GUARD in slice 1 makes it structural rather than emergent.
- **(c) FAILING signature + scorecard line.** `Figure-ground F3 melody-highest: 0.78 of steps
  (thr ≥0.95) — melody dips below counter on dark-image steps`.
- **DET/SEEDED:** **SEEDED** (with an RNG-invariant floor on the register-band ordering).
- **Expected today:** on a dark image (`bright_octaves < 0`) the melody floor can drop into the
  counter band (§2.1B of the work order) → `F3_frac < 1.0` → reveals the register-order
  fragility. On bright images F3 ≈ 1.0 already.

### F4 — INVERSE-COMPENSATION PRESENT (operator signal 4; the subtle one, §3)

**Perceptual definition (Affect FG-4):** the NON-LEVEL help the melody receives should be INVERSE
to its realized register height relative to the bed — a low-seated melody gets MORE
activity/articulation separation, a high-seated melody less. A first-class engine shows a NEGATIVE
correlation between the melody's register gap to the bed and its applied separation.

- **(a) Metric.** Per section (or per register-bin partition of steps): the realized
  `melody_register_gap = melody_pitch − max(bed_pitch)` (reuse F3's per-step pitch maps) vs the
  melody's applied onset-distinctness/articulation-separation from the bed on those steps (reuse
  the M1.3-class onset-distinct computation, `:400-430`, computed melody-vs-bed instead of
  counter-vs-pad). `F4 = sign of correlation(register_gap, separation)` across the partitions; a
  first-class engine is NEGATIVE (smaller gap → more separation). *SEEDED for the gap partition;
  the onset-distinctness WITHIN a partition is DETERMINISTIC.*
- **(b) FIRST-CLASS THRESHOLD.** Sign NEGATIVE (load-bearing); magnitude ear-tuned. Today the
  correlation is ≈ 0 (help is uniform — `prom_shift` is register-blind), so F4 makes the MISSING
  compensation visible.
- **(c) FAILING signature + scorecard line.** `Figure-ground F4 inverse-compensation: ABSENT —
  separation uncorrelated with register gap (corr +0.02, thr negative) — low melody gets no extra
  help`.
- **DET/SEEDED:** **SEEDED** (gap partition) + **DETERMINISTIC** (within-partition separation).
- **Expected today:** ≈ 0 correlation → ABSENT → REPORTED (the metric is the before/after
  instrument for slice 3, the inverse-register slice).

### F5 — PER-ROLE-RHYTHM-DISTINCTNESS + BACKGROUND-ACTIVITY-RECESSION (signal 7 + the hard gate)

This metric carries TWO faces because the Architecture lens places the HARD regression assertion
here (the S46 analogue of M1.4): the anti-fusion distinctness (signal 7) AND the
background-must-never-out-move-the-foreground invariant (the regression gate).

- **(a) Metric.**
  - **F5a per-role rhythm distinctness (signal 7, Affect FG-5):** for every pair of concurrently
    sounding roles on a step, their `offset_ms` sets must NOT be identical (the S42 fusion
    signature was identical grids). `F5a = fraction of step×role-pairs with distinct onset
    offsets` — the M1.3 metric (`:400-430`) generalized to ALL role pairs. Plus the
    cross-section per-role density spread (reuse M5.2): `max_section_density − min_section_density`
    per role. *DETERMINISTIC* — offsets + onset counts are RNG-free templates.
  - **F5b background-activity-recession invariant (the HARD gate):** on every step a bed role
    co-sounds with the melody, `bed_onsets(step) ≤ melody_onsets(step)`. Count
    `bg_recession_violations = step×bed-role pairs where bed_onsets > melody_onsets`. *DETERMINISTIC*
    (onset counts). This is the activity-recession invariant that pins S46's gain and forbids
    re-inversion.
- **(b) FIRST-CLASS THRESHOLD.** F5a: all-pair onset-distinctness ≥ ~0.5 of concurrent step-pairs;
  per-role between-section density spread ≥ ~0.2. F5b: `bg_recession_violations ≤
  documented_residual` — the regression gate (see §2).
- **(c) FAILING signature + scorecard line.** `Figure-ground F5 rhythm-distinct/bg-recession:
  distinct 0.41 (thr ≥0.5), bg_recession_violations 37 (counter out-moves melody on 37 held steps)
  [REGRESSION GATE]`.
- **DET/SEEDED:** **DETERMINISTIC** (both faces).
- **Expected today:** F5a partially present (the counter's off-beat vs melody downbeat IS distinct
  — the one cue partly working, which must be PRESERVED); F5b VIOLATED on counter-routed calm
  images (the counter out-moves the held melody) → reveals the inversion and seeds the regression
  baseline.

---

## 2. The HARD regression assertion (the S46 analogue of M1.4) — F5b carries it

**F5b (background-activity-recession) carries the single hard assertion**, mirroring the M1.4
parallel-perfect regression guard's pattern (`tests/variety_scorecard_s45.rs:1035-1043`). Rationale
(Architecture lens): F1/F2/F3/F4 are EXPECTED to FAIL or be ABSENT on the counter-routed calm
images TODAY (the inversion is live), so per the scorecard's report-don't-red-bar discipline for
known-inverted layers (`:38-39`), **F1–F4 are REPORTED, not asserted**, pre-fix. F5b is the one
that becomes the regression GATE because it is the precise encoding of the "do not walk back S45
but do not let it re-invert" discipline: it forbids the background from out-MOVING the foreground.

### How it baselines pre-fix (the staged-bound discipline, exactly like M1.4)

1. **Measure the residual on the current (pre-fix) tree.** Run the harness on the counter-routed
   images (AudioHaxImg1/2/3) and record `bg_recession_violations` per image. This is the live
   inverted count (the counter out-moving the held melody on calm steps). On non-counter routes
   the Pad/Fill-vs-melody violations are recorded too (these are measurable on all six).
2. **Pin the bound to that measured residual**, image-keyed exactly as M1.4 pins `forced_residual_bound`
   per image (`:1029-1034`):

   ```rust
   // tests/variety_scorecard_s45.rs — the F5b assertion, mirroring the M1.4 block at :1035.
   // bg_recession_violations: per-step bed-role pairs where bed_onsets > melody_onsets.
   // PRE-FIX bound = the MEASURED residual on the current validated tree (the inverted count);
   // the S47 hierarchy slice TIGHTENS the bound toward 0 as it lands (the gate brackets the gain).
   // assert!(v.bg_recession_violations <= s46_recession_bound(name), "...REGRESSED...");
   ```

   where `s46_recession_bound(name)` returns the per-image measured pre-fix residual (the
   conservative max for any newly-routed image), and the assertion FAILS if a future change
   INTRODUCES NEW violations beyond the documented residual — i.e. it is a regression gate, not a
   pass gate, exactly like M1.4.
3. **The build slice tightens it toward zero.** The S47 hierarchy slice (work-order §5) drives
   `bg_recession_violations` down (the governor stops the counter out-moving a holding melody);
   each tightening of the bound is committed WITH the slice that earns it, so the gate brackets the
   gain. This is the spec-s45 §8c "objective before/after instrument" intent, applied to S46.

The other four (F1/F2/F3/F4) are REPORTED with their values + thresholds + a `[FAIL]/[WEAK]/[OK]`
tag in the printed row, so the scorecard is the visible before/after instrument while only F5b
red-bars. As each downstream slice lands (slice 1 fixes F1/F5b; the seat guard fixes F3; slice 3
fixes F4), the corresponding F-metric can be PROMOTED from reported to asserted in that slice,
following the M1.4 precedent of relaxing/tightening an assertion in lockstep with the build.

---

## 3. The figure-ground rollup (extending spec-s45 §8)

Add a NEW cross-layer block to the per-image scorecard and to `LayerVerdicts`:

```rust
// tests/variety_scorecard_s45.rs — additive fields on LayerVerdicts (:305), mirroring
// parallel_perfect_count (:313). Test-only; engine.rs untouched.
struct LayerVerdicts {
    // ... existing S45 fields unchanged ...
    figure_ground: Verdict,           // the F1–F5 rollup verdict (Varied/Partial/Flat/Na/Crash)
    melody_most_active_margin: f32,    // F1_margin (printed; image-conditioned thr alongside)
    melody_highest_frac: f32,          // F3_frac
    bg_recession_violations: usize,    // F5b — the regression-gated count
    rhythm_distinct_frac: f32,         // F5a all-pair onset-distinctness
}
```

- **Per-image figure-ground verdict:** `VARIED` iff F1 (margin ≥ image-conditioned thr AND ≥ 0),
  F2 (every bed activity_ratio ≤ image-conditioned thr), F3 (≥ 0.95), F4 (correlation negative),
  and F5 (F5a ≥ 0.5 AND F5b violations ≤ residual) all hold; `PARTIAL` if some hold; `FLAT` if the
  melody is out-competed (F1 < 0 or F5b violated beyond residual); `N/A` only where the counter is
  unrouted AND the melody-vs-Pad/Fill parts are also unmeasurable (never — Pad/Fill always
  present, so figure-ground is measurable on all six, counter-relative parts reported `N/A` per
  metric).
- **The rollup weighting (the metric-rigidity guard, Aesthetics §5.2):** the figure-ground verdict
  weights the NON-LEVEL cues (F1, F2, F3, F5) ABOVE the S43 level floor, so a build that improves
  only the level gap (the "turn it up" win) does NOT flip the figure-ground verdict to VARIED — it
  must improve activity/register/rhythm-distinctness. This is the scorecard encoding of the 90/10.
- **Whole-instrument bar:** a render is FIGURE-GROUND FIRST-CLASS only when F1–F5 pass
  simultaneously alongside the carried-forward S43 level floors and the S45 variety rows. **No
  current render passes** — and naming WHICH cue each image fails on is the value, per the
  spec-s45 §8c discipline.

---

## 4. Expected scorecard states (today's build) — all should reveal the inversion

| Metric | Counter-routed (Img1/2/3) | Non-counter (example/Lena/magicstudio) | Why |
|---|---|---|---|
| **F1 melody-most-active** | **INVERTED / WEAK** (margin ≈ 0 or negative) | margin vs Pad ≈ 0 or slightly positive | counter's guaranteed onset vs melody SUSTAINED on calm steps; Pad's downbeat figure vs sustained melody |
| **F2 bg-recession (activity)** | **COUNTER COMPETES** (activity_ratio ≥ 1.0) | Pad activity_ratio reported | no activity recession exists yet; bed has onsets the melody lacks |
| **F3 melody-highest** | < 0.95 on dark-image steps | < 1.0 only if a dark image drops the melody | seat order emergent, not enforced; dark `bright_octaves<0` drop crosses into counter band |
| **F4 inverse-compensation** | **ABSENT** (corr ≈ 0) | **ABSENT** (corr ≈ 0) | `prom_shift` is register-blind; help is uniform |
| **F5a rhythm-distinct** | partially present (counter off-beat vs melody downbeat IS distinct) | Pad-vs-melody distinctness reported | the one cue partly working — PRESERVE it |
| **F5b bg-recession violations** | **NONZERO** (counter out-moves held melody) → the regression baseline | Pad/Fill-vs-melody violations recorded | the live inversion; this is the count the S47 slice drives to 0 |
| **figure_ground rollup** | **FLAT/PARTIAL** | **PARTIAL** | no image is figure-ground first-class; the melody is not the figure on the counter-routed images |

These expected states ALL reveal the inversion the work order diagnoses — the scorecard's job is to
make each cue's failure a named, sized, before/after-measurable cell, so each S47+ slice has an
objective target (drive F5b → 0; lift F1_margin above `f(fg_bg_contrast)`; F3 → 1.0 via the seat
guard; F4 → negative via the inverse-comp slice) and a regression gate (F5b) to hold the gain.

---

*End of S46 figure-ground metric spec. Design-only: no source, test, or asset modified. EXTENDS
`spec-s45-variety-metrics.md`; lands in `tests/variety_scorecard_s45.rs` via `scorecard_for` /
`LayerVerdicts` with NO new render path. `src/engine.rs` sha256 re-verified UNCHANGED at session
end: `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.*
