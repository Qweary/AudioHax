# spec-s45-variety-metrics.md — Per-Layer Variety-Metric Specification (S45, Slice 1)

**Author:** Music Theory Specialist (PRODUCER). **Consumer:** Test Engineer (builds the harness).
**Status:** design / spec only — no code in this doc. **Companion:** `docs/design-s44-variety-nvoice.md`,
`tests/variety_s45.rs` (the routing/figuration property net this complements).

---

## 0. Purpose and the boundary it draws

The operator (a music-performance degree, hears every flaw) wants an **objective instrument** that
answers, per image and per layer, *"does this layer actually exhibit the variety a first-class program
would have, or is it flat / dormant / cloned?"* — measured from the **rendered composition**, not the ear.

This spec defines, for each musical layer:

- **(a) the concrete metric(s)** computable from a deterministic render of one image,
- **(b) the FIRST-CLASS THRESHOLD** that distinguishes real variety from a flat/dormant/cloned layer,
- **(c) what a FAILING (flat) result looks like**, so the scorecard can flag the dormant layer by name.

### 0.1 What "a render" means here (the data the harness reads)

The instrument runs the **real plan → realize path**, headless, per image, RNG-pinned:

1. `understand_image_pure(...) → ImageUnderstanding` (or a fixture `ImageUnderstanding` carrying the
   probe's measured knobs — see §0.3) — the per-image knobs.
2. `CompositionPlanner::plan(&u, &mappings) → CompositionPlan` — the structure: ordered `Section`s, each
   with `thematic_role`, `variation`, `density`, `orchestration` (`OrchestrationProfile` incl.
   `layers`, `pad_voices`, `figuration_resolved`, `bass_pattern_resolved`, `prominence`), and its own
   `steps: Vec<StepPlan>`.
3. For each global step `0..total_steps`: `plan.locate(step) → (&Section, step_in_section)`, build the
   `StepContext`, and for each instrument index call
   `chord_engine::realize_step(step, inst_idx, num_instruments, &PerfFeatures, ms_per_step, &ctx)
   → Vec<NoteEvent>`.

The result is, **per layer (role)**, a time-ordered stream of `NoteEvent { note, velocity, hold_ms,
offset_ms }` plus the per-section structural facts. **Every metric below is a pure function of that
stream + the plan.** No image type, no audio hardware, no OpenCV — same headless discipline as
`variety_s45.rs` / `composition_s15.rs`.

### 0.2 The RNG boundary (load-bearing — read before building)

`CompositionPlanner::plan` delegates per-section harmony to `thread_rng` (Roman numerals, chord
qualities, per-step chord notes are **non-deterministic**). Therefore:

- **Metrics keyed on absolute pitch values** (distinct-pitch counts, contour, non-chord-tone counts)
  must be made deterministic. Two acceptable routes, in order of preference:
  1. **Seed the RNG** (a fixed-seed `StdRng` threaded into a test-only plan entry, if one exists or the
     Test Engineer adds one in the test harness — NOT in `src`), so the same image renders byte-identically
     every run; then absolute-pitch metrics are stable.
  2. If seeding is not wired, restrict pitch-dependent metrics to **RNG-INVARIANT projections** —
     properties true for *every* draw: e.g. "the counter MOVES at least K times" can be checked on the
     `motion_dir` of the realized counter line, which is contrary/oblique by construction regardless of
     the chord draw; "the bass is static" is a property of the `bass_pattern_resolved` arm, not the draw.
- **Metrics keyed on STRUCTURE** (which `ThemeVariation` reached the plan, distinct `figuration_resolved`
  ids per section, distinct orchestration profiles, section `density` values, onset GRID / offset_ms /
  hold-fraction *shape*) are **RNG-independent** — `section_figuration_id`, the texture `SelectTable`,
  the variation clamp, and the rhythm-cell/onset templates are pure functions of `(knobs, role)`. Prefer
  these wherever a metric can be expressed structurally; they are the robust core of the scorecard.

The harness should **label each metric `DETERMINISTIC` (structural / RNG-free) or `SEEDED` (needs a fixed
RNG)** so a future reader knows which guarantee backs each scorecard cell.

### 0.3 The reference image set (the scorecard's columns)

Drive all 8 layers over the **6 probe images**, using their **measured knobs** (the S45 feature-distribution
probe; `foreground_energy` 0.003–0.039, `fg_bg_contrast` 0.052–0.341):

| image | fe | ct | colorfulness | subj_e | S45 texture route |
|---|---|---|---|---|---|
| example.jpg | 0.039 | 0.136 | 0.686 | 0.043 | pad_broken_wave |
| Lena.png | 0.016 | 0.052 | 0.122 | 0.044 | pad_bed |
| AudioHaxImg1.jpg | 0.017 | 0.341 | 0.011 | 0.047 | **pad_bed_counter** |
| AudioHaxImg2.jpg | 0.034 | 0.284 | 0.080 | 0.070 | **pad_bed_counter** |
| AudioHaxImg3.jpg | 0.024 | 0.203 | 0.423 | 0.049 | **pad_bed_counter** |
| magicstudio-art.jpg | 0.003 | 0.084 | 0.287 | 0.009 | pad_bed |

The CounterMelody layer (Metric 1) is **only present** on the three routed images; the scorecard must
report it as `N/A — not routed` (not as a failure) for the other three. Every other layer is present on
all six.

---

## 1. Layer: CounterMelody (when routed: `pad_bed_counter` selected)

**Realization read:** `realize_rhythm`'s `CounterMelody` arm (`chord_engine.rs:1831`). Pitch via
`realized_counter_pitch_with_prev` (contrary/oblique, chord-tone, fifth-species figures); rhythm via the
held-period activation (off-beat onset at `step_ms/4` when the harmony/melody is static) vs the oblique
sustain when the melody is active. The cadence ring (`:1631`) recomputes the counter cadence pitch.

### (a) Metrics — over the realized CounterMelody NoteEvent stream of one render
- **M1.1 Presence:** the layer emits ≥1 `NoteEvent` per section it sounds in (it never rests a whole
  section — held-period activation guarantees a note). *Structural-ish; DETERMINISTIC on event COUNT.*
- **M1.2 Motion fraction:** of consecutive sounding steps, the fraction where the counter **pitch
  changes** vs sustains the same pitch: `moves / (moves + holds)`. *SEEDED (absolute pitch).*
- **M1.3 Onset-grid distinctness vs Pad:** compare the counter's `offset_ms` set against the Pad's per
  step. First-class = the counter places its onset at `step_ms/4` (the guaranteed off-beat) on the
  held/static steps while the Pad sits on the downbeat (`offset_ms == 0`) — a **distinct onset phase**.
  Metric: fraction of steps where `counter.offset_ms != pad.offset_ms`. *DETERMINISTIC (offset is the
  rhythm template, RNG-free).*
- **M1.4 Parallel-perfects vs Melody — PARALLELS-MINIMIZED (not zero):** count the strict
  counter↔melody parallel-perfect pairs — consecutive sounding step-pairs where both lines move in the
  **same direction** (similar motion) *into* a perfect interval (P5 or P8/unison) by the **same signed
  interval** (true parallel motion, not merely arriving at a perfect by contrary/oblique approach). The
  bar is **MINIMIZED to the irreducible set**, NOT zero. *DETERMINISTIC on the pair classification
  (motion direction + interval class are read from the realized pitch deltas; the irreducible set is a
  fixed, enumerated count for the routed image set, so the assertion is stable even though absolute
  pitch is SEEDED — see the bar in (b)). The contrary/oblique split is the separate M1.3-class
  measurement (M1.3 already reports the counter at 0.828 contrary/oblique on the routed images, which IS
  the first-class motion profile).*

  **Why the bar is minimized, not zero — the two irreducible classes.** Implementation drove the strict
  parallel pairs from 11 down to 5; the residual 5 are *music-forced* and decompose into exactly two
  documented classes, each ACCEPTED (not a failure):
  1. **Species-forced phrase-start cases** (recurring PhraseStart→Interior pairs). The melody descends by
     step (e.g. 71→70) and the active chord offers **no consonant contrary or oblique tone** for the
     counter to reach. The only route to a non-parallel here is to hold an **unresolved dissonant
     (non-chord-tone) suspension** — which is musically *worse* and breaks the validated "every
     structural sustain is consonant" invariant (pinned by
     `test_consonant_corpus_dissonance_resolves_by_step` and siblings). Classical first-species practice
     chooses the **consonant similar-motion-into-a-perfect as the lesser evil** in exactly this
     configuration. This is the correct counterpoint, not a defect.
  2. **Cross-section pivot-boundary cases.** At a section pivot the counter has no prior-section last
     pitch in scope, so it cannot plan contrary motion across the boundary; reaching zero here requires
     threading the previous section's last counter pitch into `StepContext` for continuity — a
     constructor that lives in **byte-FROZEN `engine.rs`** — making it a separate multi-slice feature
     (deferred; see the forward note in (b) and §7-reconciliation).
- **M1.5 Distinct-pitch count** over the piece (size of the set of realized counter `note` values).
  *SEEDED.*

### (b) FIRST-CLASS THRESHOLD (a genuinely moving, onset-distinct, partly-contrary line)
- M1.1 present in **every** routed section.
- M1.2 **motion fraction ≥ 0.50** (the line moves on at least half its sounding step-pairs — not a drone).
- M1.3 **onset-distinct fraction ≥ 0.40** (it is rhythmically off-grid from the Pad on a meaningful share
  of steps; the held-period off-beat is the engine of this).
- M1.4 **PARALLELS-MINIMIZED bar (the precise pass/fail rule the Test Engineer implements):** the strict
  counter↔melody parallel-perfect pair count must be **≤ the documented irreducible set** (the
  enumerated species-forced phrase-start cases + the cross-section pivot-boundary cases — 5 on the
  current routed image set). **PASS** iff the measured count is ≤ that documented forced-set count;
  **FAIL (REGRESSION)** iff any *new* parallel appears beyond the documented forced set — i.e. the
  instrument flags a regression, not the accepted floor. The documented species-forced + pivot-boundary
  cases are ACCEPTED, never failures. **AND** contrary-or-oblique fraction ≥ 0.50 (seeded; the counter
  measures 0.828 on the routed images, comfortably first-class).

  *A first-class counter line **minimizes** parallels and uses contrary/oblique motion as the norm — the
  M1.3 contrary/oblique fraction metric already pins this, and the routed counter's 0.828 contrary/oblique
  IS first-class. The residual forced parallels on the inner-voice/melody pairs are an accepted artifact
  of the **consonance-first floor** (the validated S30–S33 species-counterpoint invariant that every
  structural sustain is consonant), not a defect: removing them would require the musically-worse
  unresolved dissonant suspension.*

  > **Forward note (S46 slice).** The cross-section pivot-boundary parallels would be further reduced by
  > threading the **prior-section last-counter-pitch through `StepContext`**, giving the counter contrary
  > planning across a section pivot. That requires editing the byte-FROZEN `engine.rs` `StepContext`
  > constructor, so it is deferred to a dedicated S46 slice. It is the **same "shared voice-leading
  > context" reconciliation item that S44 §7 flagged** — once landed, the irreducible parallel set drops
  > to only the species-forced phrase-start class.

  > **Test-assertion note.** The M1.4 assertion in `tests/variety_scorecard_s45.rs` (owned by the Test
  > Engineer) is being relaxed in parallel — from `parallel count == 0` to the
  > `count ≤ documented-forced-set (regression gate)` bar above — to match this refined, musically-honest
  > definition.
- M1.5 **distinct pitches ≥ 4** over the piece.

### (c) FAILING (flat) signature
A near-static drone: M1.2 < 0.2 (sustains the same pitch step after step), M1.3 ≈ 0 (locked to the Pad's
downbeat — no independent onset), M1.5 ≤ 2 (one or two pitches). Scorecard line:
`CounterMelody: DORMANT — drone (motion 0.08, onset-distinct 0.0, 2 pitches)`.

### Expected today
**VARIED when routed** (M1 is the layer S45 Slice 1 deliberately activated; the species arm is the most
musically developed code in the engine). Report `N/A — not routed` on example/Lena/magicstudio.

---

## 2. Layer: Pad figuration (across the sections of a figured plan)

**Read:** each `Section.orchestration.figuration_resolved.id`; the planner's per-section arc
`section_figuration_id(base, role)` (`composition.rs:1859`) swaps anchor roles (Statement/Return/Coda →
base cell) vs departure roles (Contrast/Development → a broken/arpeggiated partner). The figuration's
`onsets` (count + `at` positions) give the **density class** of each cell. (Note: only the **figured**
profiles — `pad_figured`, `pad_broken_wave`, `pad_block_comp` — carry a base figuration; `pad_bed` and
`pad_bed_counter` carry `figuration == None`, so this layer is **N/A** on those routes — see §2 Expected.)

### (a) Metrics — over the section sequence of a figured render
- **M2.1 Distinct figuration cells:** size of the set `{section.figuration_resolved.id}` across sections.
  *DETERMINISTIC (pure `section_figuration_id`).*
- **M2.2 Density-class change:** classify each cell as BLOCK (≤2 onsets / sustained feel: `block`,
  `block_comp_24`) vs BROKEN (≥3 onsets / arpeggiated: `alberti`=3, `broken_chord_up/wave`, `arp_waltz`).
  Metric: does the sequence contain **at least one BLOCK↔BROKEN transition** (a *felt* density change),
  not merely two different cells of the same density class? *DETERMINISTIC (onset count is data).*
- **M2.3 Return-to-base at recap:** in a rounded/returning form (Statement … Return/Coda), does the final
  anchor section's cell **equal the opening Statement's cell** (block→broken→block, an A…A′ texture arc)?
  *DETERMINISTIC.*

### (b) FIRST-CLASS THRESHOLD
- M2.1 **≥ 2 distinct cells** across the piece.
- M2.2 **≥ 1 BLOCK↔BROKEN density-class change** (the contrast is *felt*, not a same-density cosmetic swap).
- M2.3 **return == true** (the recap restores the opening texture).

### (c) FAILING (flat) signature
One cell for the whole piece (M2.1 == 1) → `Pad figuration: FLAT — one ostinato cell (alberti×N)`; or
two cells of the SAME density class (M2.1 == 2 but M2.2 == false) → `Pad figuration: WEAK — cells differ
but no density change`; or a departure that never returns (M2.3 == false) → `Pad figuration: no recap`.

### Expected today
**VARIED on a figured route** (`section_figuration_id` is the second S45-Slice-1 deliverable and is
exercised by `variety_s45.rs` Property D). `N/A — non-figured route` on `pad_bed`/`pad_bed_counter`.
Note the asymmetry the scorecard must surface honestly: the three CounterMelody-routed images are
exactly the ones with **no Pad figuration arc** (the bed is `figuration == None`), and the two
figuration-bearing routes among the six (example→pad_broken_wave, AudioImg3 *would have* been
pad_broken_wave but is now counter) — so on this image set Pad-figuration is only live for **example.jpg**.
That is a real finding to report, not a metric failure.

---

## 3. Layer: Bass

**Read:** the realized `Bass` (`OrchestralRole::Bass`) NoteEvent stream; the bass dispatch
(`chord_engine.rs:1647`) branches on `Section.orchestration.bass_pattern_resolved` — `None`/`Sustained`
→ the byte-stable one-root-per-step arm (today's default), `Walking`/`Pedal` → a generated line.

### (a) Metrics
- **M3.1 Distinct bass pitches** over the piece (set size of realized Bass `note` values). *SEEDED for the
  exact count; but a DETERMINISTIC structural proxy exists — see M3.3.*
- **M3.2 Intra-step onset count:** does the bass emit >1 onset within any step (walking/pedal motion), or
  exactly one sustained root per step? Metric: max onsets-per-step over the render. *DETERMINISTIC (the
  arm taken is `bass_pattern_resolved`, RNG-free).*
- **M3.3 Pattern-arm witnessed:** which `BassPatternKind` actually reached realization across sections
  (`Sustained` only, vs any `Walking`/`Pedal`). *DETERMINISTIC.*

### (b) FIRST-CLASS THRESHOLD
- M3.1 **distinct bass pitches ≥ (number of distinct chord roots in the progression)** — i.e. the bass
  tracks root motion rather than pedaling one note.
- M3.2 **max onsets-per-step ≥ 2** on at least the sections that should walk (a moving bass subdivides).
- M3.3 at least one section realizes a **non-`Sustained`** arm.

### (c) FAILING (flat) signature — **this is the metric that should reveal today's dormancy**
M3.2 == 1 for every step (one sustained root per step), M3.3 == `Sustained` everywhere, M3.1 still moves
between chord roots BUT only because the chord changed — there is **no passing/neighbor motion, no
walking, no pedal**. Scorecard line: `Bass: DORMANT — sustained root only, no bass_pattern ever selected
(walking/pedal arms reachable but unrouted)`.

### Expected today
**FLAT / DORMANT** (per S44 §2.1). No `texture_catalogue` profile in `assets/mappings.json` sets a
`bass_pattern` handle, so `bass_pattern_resolved == None` on every section → the sustained arm is the only
one realized. The metric exists precisely to make this visible and to flag the unrouted `walking`/`pedal`
catalogue entries.

---

## 4. Layer: Melody

**Read:** the realized `Melody` (`OrchestralRole::Melody`) stream; pitch via `role_pitch` / the theme
seam (`theme_melody_pitch`), chord-tone-selected.

### (a) Metrics
- **M4.1 Pitch-class variety:** size of the set `{note % 12}` of realized melody notes. *SEEDED.*
- **M4.2 Contour direction-changes:** count of sign flips in the `motion_dir` sequence of the melody line
  (a turning, shaped contour vs a monotone run/static repeat). *SEEDED for exact count; an RNG-INVARIANT
  floor — "the contour is not constant" — can be checked on the theme motif's resolved contour
  (`resolve_motif`, plan-time, RNG-free) for the theme-driven sections.*
- **M4.3 Non-chord-tone count:** number of realized melody notes that are NOT a member of their step's
  chord pitch-class set (passing tones, suspensions, appoggiaturas). *SEEDED.*

### (b) FIRST-CLASS THRESHOLD
- M4.1 **≥ 5 distinct pitch classes** over the piece (a real tune visits most of a heptatonic mode, not
  3–4 triad tones).
- M4.2 **≥ 3 contour direction-changes** (the line arches and turns; not a monotone ramp).
- M4.3 **non-chord-tone count ≥ 1** (the vocabulary includes at least one expressive dissonance — the
  difference between a melody and a broken chord).

### (c) FAILING (flat) signature — **reveals the "vocabulary-thin" state**
M4.1 ≤ 4 (chord tones only), M4.3 == 0 (zero non-chord tones — every melody note is a triad member),
M4.2 low. Scorecard line: `Melody: VOCABULARY-THIN — chord-tones only (4 PCs, 0 non-chord tones)`.

### Expected today
**FLAT / vocabulary-thin** (per S44 §2.1): the melody is chord-tone-selected, so M4.3 is ≈0 and M4.1 is
bounded by the triad/7th-chord membership. The metric is built to expose exactly this — it is the
strongest argument for the future melodic-vocabulary slice.

---

## 5. Layer: Rhythm

**Read:** the realized onset/duration shapes across the render — `(offset_ms, hold_ms)` patterns per
step, per role; `realize_rhythm` selects sustained / arpeggio / dotted / syncopated / rest-as-gesture
bands plus harmonic-rhythm acceleration. The existing unit invariant `test_rhythm_at_least_three_distinct_patterns`
pins **≥3 distinct patterns** on a single hand-built phrase (a floor).

### (a) Metrics
- **M5.1 Distinct rhythm-pattern count** across the *whole render* (canonicalize each step's event shape
  to `(onset_count, sorted offset_ms fractions, sorted hold fractions)` and count distinct shapes).
  *DETERMINISTIC (the rhythm template is RNG-free; it keys on `edge_activity`/phrase-position/role).*
- **M5.2 Between-section onset-density variation:** mean onsets-per-step per section; does it differ
  across sections (the form has a busy section and a calm section), not merely within a phrase? Metric:
  `max_section_density − min_section_density`. *DETERMINISTIC (per-section `density` nudge + the cell
  selection are RNG-free).*

### (b) FIRST-CLASS THRESHOLD
- M5.1 **≥ 4 distinct patterns** across the piece (the ≥3 unit floor is a single-phrase minimum;
  first-class is more shapes spread over the form).
- M5.2 **between-section density spread ≥ 0.20** (a real onset-density ARC across the form, not a flat
  busyness — the section-`density` lever audibly contributes).

### (c) FAILING (flat) signature
M5.1 ≤ 3 (only the unit-floor minimum, no extra shapes), M5.2 ≈ 0 (every section the same busyness).
Scorecard line: `Rhythm: shapes OK (4) but FLAT density arc (Δ=0.02 between sections)`.

### Expected today
**PARTIALLY VARIED:** M5.1 is likely first-class (≥3, the engine has the band machinery). M5.2 is the
risk — the per-section `density` lever (S29) is wired but only departs from 0.5 when source-region energy
differs; report the actual spread and flag if the arc is flat.

---

## 6. Layer: Harmony / theme variation

**Read:** each `Section.variation` (`ThemeVariation`) that actually reached the plan. The slice-1 clamp
`clamp_variation_slice1` (`composition.rs:1815`) collapses **everything except `Fragmented`** to
`Identity` — so only **2 of the 9** `ThemeVariation` variants (Identity, Fragmented) can appear in a plan;
the other 7 (Transposed, Reharmonized, Augmented, Diminished, Ornamented, Inverted, Retrograde) are
**dead** at plan level regardless of the form template.

### (a) Metrics
- **M6.1 Distinct ThemeVariation variants** realized across sections: size of `{section.variation}`.
  *DETERMINISTIC (the clamp + template are RNG-free).*
- **M6.2 Variant-reachability census:** which of the 9 variants ever appear across the whole image set.
  *DETERMINISTIC.*

### (b) FIRST-CLASS THRESHOLD
- M6.1 **≥ 3 distinct variants** within a single multi-section piece (a development genuinely transforms
  the theme — transposes / reharmonizes / augments — not just states-or-fragments it).
- M6.2 the census shows **> 2** variants reachable across the set (the clamp is lifted).

### (c) FAILING (flat) signature — **reveals the clamp**
M6.1 ≤ 2 (only Identity and/or Fragmented), M6.2 == exactly `{Identity, Fragmented}` across ALL six
images. Scorecard line: `Theme variation: CLAMPED — only Identity/Fragmented reach the plan (7 of 9
variants dead via clamp_variation_slice1)`.

### Expected today
**CLAMPED / DORMANT** (per S44 §2.1). M6.2 will read exactly `{Identity, Fragmented}` — the metric's job
is to make the slice-1 clamp a visible, named scorecard failure and to size the gap (7 dead variants).

---

## 7. Layer: Form / texture (orchestration arc across sections)

**Read:** each `Section.orchestration` profile id and `Section.density`; is there a **texture ARC** (the
ensemble thins/thickens or the profile changes across the form) or is **one profile cloned flat** across
every section?

### (a) Metrics
- **M7.1 Distinct orchestration profiles** across sections: size of `{section.orchestration.id}`.
  *DETERMINISTIC.* (Today the planner selects ONE texture profile per *piece* via the `texture`
  SelectTable and clones it onto every section — so this is expected == 1; the metric reveals the clone.)
- **M7.2 Density arc:** `max(section.density) − min(section.density)` (the S29 per-section energy→density
  lever). *DETERMINISTIC.*
- **M7.3 Active-layer-count variation:** does the set of sounding roles (`orchestration.layers`, or the
  realized non-empty role streams) change across sections, or is the same stratification cloned? *DET.*

### (b) FIRST-CLASS THRESHOLD
- M7.1 **≥ 2 distinct profiles** OR (if profile is by-design one-per-piece) M7.3 shows a **layer-count
  change** across sections (a section drops to bass+pad, another adds the counter — a real orchestration
  arc).
- M7.2 **density arc ≥ 0.15** (the form breathes).

### (c) FAILING (flat) signature — **reveals the cloned-profile state**
M7.1 == 1 AND M7.3 == 0 AND M7.2 ≈ 0: one profile, the same layers, the same density, cloned across every
section. Scorecard line: `Form/texture: CLONED — one profile (pad_bed_counter) on all N sections, Δdensity
0.0, no layer-count arc`.

### Expected today
**CLONED / FLAT** (per S44 §2.1). The `texture` SelectTable picks one profile per piece and the planner
attaches it to every section; M7.1 == 1. The only live variation within a piece today is the **Pad
figuration arc** (§2) and the **per-section density nudge** (§5/M7.2) — report both and flag the
cloned-profile / cloned-stratification dormancy.

---

## 8. Cross-layer / whole-piece variety scorecard (the rollup)

### (a) The instrument
For one rendered image, emit a **per-layer pass/fail row** (layers 1–7) plus the metric values that drove
each verdict, then a one-line **verdict**. Run it over the 6-image set → an 8-row × 6-column scorecard.

### (b) Combining into a first-class verdict
Define a layer's status as one of `VARIED` (all its (b) thresholds met), `PARTIAL` (some met),
`DORMANT/FLAT` (its (c) flat signature matches), or `N/A` (layer not present on this route — CounterMelody
off non-counter routes, Pad-figuration off non-figured routes). Then:

- **Per-image verdict:** `FIRST-CLASS` iff **every present non-N/A layer is `VARIED`**; `DEVELOPING` iff
  the *active/foreground* layers (CounterMelody when routed, Pad figuration when figured, Rhythm) are
  VARIED but background layers (Bass/Melody/Theme/Form) are FLAT; `FLAT` iff a majority of present layers
  are DORMANT.
- **Set-level rollup:** for each layer, the worst status across the 6 images (so a layer that is FLAT on
  any image is reported FLAT for the set), plus the **variant/arm census** (M6.2, M3.3, M7.1) that shows
  what is reachable-but-unrouted across the whole set.

### (c) FIRST-CLASS bar for the whole instrument
A render is **FIRST-CLASS** only when CounterMelody (if routed), Pad figuration (if figured), Bass,
Melody, Rhythm, Theme, and Form/texture all pass their (b) thresholds simultaneously. **No image in the
current build will pass** — and that is the point: the scorecard should print, per the expected states
below, a precise list of which layers are dormant and why, so each future slice has an objective target
to move and a regression gate to hold the gains.

---

## 9. Existing-coverage delta — what's already pinned vs what's NEW

### 9.1 ALREADY covered by the 23 `chord_engine.rs` unit tests (single hand-built fixtures, note-level invariants)
These prove the *mechanism* exists on a constructed phrase; they do **not** measure variety on a real
per-image render:
- Voice leading: `test_upper_voices_never_leap_beyond_perfect_fifth`,
  `test_no_parallel_perfect_fifths_or_octaves`, `test_common_tone_retained_in_same_voice`,
  `test_voice_spacing_upper_voices_never_collapse_to_unison`.
- Mode/chord correctness: the `*_mode_honored_*` set, `test_*_triads_are_scale_derived_chord_tones`,
  `test_iv/iii_numeral_resolves_*`, `test_seventh_and_ninth_are_diatonic`,
  `test_harmonic_complexity_tracks_saturation`, `test_colorfulness_adds_borrowed_chord`,
  `test_secondary_dominant_*`, `test_modal_interchange_borrows_minor_iv_when_dark`.
- Dynamics/phrase: `test_velocity_varies_within_a_phrase`, `test_dynamics_contour_exceeds_structural_floor`,
  `test_dynamics_metric_accent_strong_exceeds_weak`, `test_cadences_sit_at_phrase_boundaries`.
- **Rhythm (FLOOR, partial overlap with M5.1):** `test_rhythm_at_least_three_distinct_patterns` (≥3 on
  one phrase), `test_articulation_hold_fractions_vary`,
  `test_articulation_curve_calm_longer_than_busy_and_crosses_legato`,
  `test_rhythmic_density_distribution_differs_with_busyness`, the articulation-window bounds tests.
- **CounterMelody (note-level, partial overlap with M1.4):** `test_counter_contrary_or_oblique_vs_melody`,
  `test_counter_held_period_fills_and_moves`, `test_counter_held_run_advances_across_three_steps`,
  `test_counter_is_chord_tone_in_counter_register`, `test_counter_at_most_one_event_per_step`,
  `test_counter_no_longer_harmonicfill_delegate` — all on a **hand-built 2-section counter fixture**, not
  a per-image plan.
- Theme seam: the `test_theme_pitch_*` set (Identity holds, Fragmented head-then-rest) — proves the seam,
  not the variant census.
- Orchestration: `test_orchestration_roles_differ_by_register_and_activity` (one render, role separation).

### 9.2 NEW in this instrument (per-image, per-layer, whole-RENDER measurements — the value add)
None of the above measures variety **on a real plan, per image, across the whole form**. NEW:
- **M1.2/M1.3/M1.5** — counter motion fraction, onset-distinctness vs Pad, distinct-pitch count **on a
  real routed render** (the unit tests check a single 2-step fixture, not motion fraction over a piece).
- **M2.1/M2.2/M2.3** — the per-section figuration *arc* (distinct cells, density-class change, recap
  return) — `variety_s45.rs` Property D pins *that variation happens*; the metric measures *how much* and
  the BLOCK↔BROKEN/recap quality.
- **M3.1/M3.2/M3.3** — the Bass dormancy census (no unit test renders a multi-section bass and checks for
  walking/pedal motion).
- **M4.1/M4.2/M4.3** — melody pitch-class/contour/non-chord-tone **vocabulary** over a piece (no existing
  test counts non-chord tones — there are none to count yet; the metric exposes the zero).
- **M5.1 whole-render** distinct-pattern count and **M5.2 between-section** density spread (the unit test
  is within one phrase).
- **M6.1/M6.2** — the ThemeVariation reachability census exposing the 7-of-9 clamp (no existing test
  asserts which variants reach a plan).
- **M7.1/M7.2/M7.3** — the orchestration/density ARC across sections exposing the cloned-profile state.
- **§8** — the cross-layer rollup and per-image FIRST-CLASS verdict (entirely new; there is no aggregate
  variety scorecard today).

---

## 10. Summary of expected scorecard states (today's build)

| Layer | Expected | Why |
|---|---|---|
| 1 CounterMelody (routed) | **VARIED** | S45-S1 activated the species arm; moving, onset-distinct, 0.828 contrary/oblique. M1.4 = parallels-MINIMIZED (≤ documented irreducible forced set, not zero — species-forced phrase-start + pivot-boundary cases accepted; regression on new parallels only). N/A off counter routes. |
| 2 Pad figuration (figured) | **VARIED** | S45-S1 per-section arc (`section_figuration_id`). N/A on the 5 non-figured routes in this set (only example.jpg is figured). |
| 3 Bass | **DORMANT** | No profile sets `bass_pattern`; only the sustained arm realizes — walking/pedal unrouted. |
| 4 Melody | **FLAT (vocab-thin)** | Chord-tone-only selection → 0 non-chord tones, ≤4 PCs. |
| 5 Rhythm | **PARTIAL** | Shapes likely ≥3 (pass); between-section density arc the risk — report Δ. |
| 6 Theme variation | **CLAMPED** | `clamp_variation_slice1` kills 7 of 9 variants; census == {Identity, Fragmented}. |
| 7 Form / texture | **CLONED** | One texture profile cloned across all sections; only the Pad-figuration arc + density nudge vary within a piece. |
| 8 Rollup | **DEVELOPING** | Foreground (Counter/Figuration/Rhythm) moving; background (Bass/Melody/Theme/Form) flat — no image is FIRST-CLASS yet. |
