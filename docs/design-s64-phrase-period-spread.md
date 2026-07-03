# S64 Design — Phrase-Period / Cadence-Scheme Spread (STEP 2)

**Session 64, STEP 2 — production change.** Owner: Music Theory Specialist. Files touched:
`src/chord_engine.rs` (the `PhraseScheme` catalogue + `plan_phrases_scheme`), `src/composition.rs`
(feature-driven scheme resolution in `plan`, threaded into `plan_phrases_scheme`),
`src/mapping_loader.rs` (loader field), `assets/mappings.json` (the `phrase_scheme` selector).
`src/engine.rs` is **FROZEN and untouched** (sha256 `e50c7db…2348261`, verified).

This slice implements the STEP-1 diagnostic's single deepest recommendation: break the universal
4-step phrase *period* so the **cadence periodicity the ear locks onto varies across pieces**,
deterministically, from an image feature, over a curated authored menu.

---

## 0. The problem this fixes (from `docs/diagnostic-s64-fixed-rhythm-key.md`)

The operator (trombone performance degree) transcribed a FIXED cross-piece rhythmic skeleton in
one layer of every output: `dotted-quarter · eighth · quarter-triplet · half-note`, repeating every
four steps, then a `dotted-quarter · eighth` tail. The diagnostic located this as the **phrase /
cadence scaffold**, not the motto and not a mappings template:

- `PHRASE_LENGTHS = [4,8]` + the `plan_phrases` "largest length that divides evenly AND yields >= 2
  phrases" rule collapse **every** section of **every** image to a universal 4-step period with the
  layout `[Start · Interior · Interior · Cadence]` → the recurring `S..h S..P`.
- The two rhythmic gestures that realize it — the **pre-cadence acceleration** (the "triplet") and
  the **cadence ring** (the "half-note") — are structural constants in `realize_rhythm`, gated on
  `position_in_phrase` / `phrase_len` read off the `StepPlan`. They are NOT image-driven, and NOT
  form-gated (proven: `example.jpg` on `theme_and_variations` still realizes `S..hS..P`).

**The lever:** the gestures read their placement off the `StepPlan`'s phrase geometry. So if the
phrase geometry (period length → cadence positions) varies per piece, the pre-cadence lift and the
cadence ring **move with it, for free** — no change to `realize_rhythm`, no risk to the byte-frozen
realizer kernel. That is the whole design.

---

## 1. The curated phrase-scheme catalogue (`chord_engine.rs::PhraseScheme`)

Four real common-practice phrase/cadence archetypes. Each is defined by the nominal phrase length it
feeds the SHARED partition/cadence-stamping logic in `plan_phrases_scheme`; the last phrase of a
section absorbs any remainder (a musically honest "long phrase + short tail"). Cadence positions
below are for the recurring **8-step** section (the "B"/"A′"/"V2" the operator was hearing).

| scheme | structure | 8-step layout | cadence positions | cadence density | theory |
|---|---|---|---|---|---|
| `period_4` **(LEGACY / default / freeze anchor)** | 4+4 antecedent–consequent PERIOD | `S..h S..P` | **{3, 7}** | 2 / 8 | The normative period: antecedent on a half cadence, consequent on a PAC. Byte-identical to pre-S64. |
| `phrase_5` | **5+3 asymmetric period** | `S...h S.P` | **{4, 7}** | 2 / 8 | A 5-step antecedent answered by a compressed 3-step consequent — a genuine asymmetric period. The interior cadence (and its lift + ring) walk one step later; the two phrases are unequal. |
| `group_6` | a broad **6-step phrase** + 2-step tail | `S....h SP` | **{5, 7}** | 2 / 8 | One long, unhurried phrase that defers the first cadence to step 5, then a short codetta. The "long line" reading. |
| `sentence_8` | one **8-step Schoenberg sentence** (basic idea 2 + repetition 2 + continuation-with-fragmentation 4) | `S......P` | **{7}** | **1 / 8** | The sentence drives to a *single* terminal cadence; the `S..hS..P` double-period collapses to one long push, and the pre-cadence lift lands once (step 6→7) instead of on every 3rd step. |

**Scope of the `sentence_8` label (no overclaim).** What this scheme actually realizes is the
sentence's **cadence PLACEMENT** — a single terminal cadence at the phrase end (step 7) with *no*
interior cadence — which is the whole point here (it is the cadence periodicity, not the surface
motif, that the operator transcribed). It does **not** encode the sentence's internal 2+2+4
basic-idea / repetition / continuation-with-fragmentation grouping at the `StepPlan` level: the
interior steps 0–6 are stamped as one undifferentiated run of `PhraseStart`/`Interior` positions, not
a 2-bar / 2-bar / 4-bar sub-grouping with accelerating harmonic rhythm. The "2+2+4" in the table is
the *name* of the archetype the single-terminal-cadence layout is drawn from, not a claim that the
fragmentation/acceleration is modeled. (Encoding the internal grouping would need per-sub-phrase
velocity/harmonic-rhythm structure — a candidate S65 refinement, out of scope for the periodicity
spread.)

**Why these four and not the diagnostic's literal `asymmetric_3` (3-step groups):** see §4. A
symmetric 3-group scheme (cadence periodicity 3) was designed, built, and **rejected on measurement**
because it *tripled* the cadence rate and regressed the S49 F5a within-piece anti-fusion floor.

### 1.1 The freeze anchor

`PhraseScheme::Period4` reproduces the pre-S64 `plan_phrases` output **byte-for-byte**: the legacy
length rule is extracted verbatim into `legacy_period_len(n)`, and `Period4::phrase_len_for` calls it
unchanged. Each non-legacy scheme also **falls back to `legacy_period_len`** below its minimum viable
span (e.g. `phrase_5` requires `n >= 8`; `group_6` requires `n >= 6`), so the very short sections (the
rounded-binary "A" of ~3 steps) stay identical across all schemes — the audible spread is confined to
the longer sections where periodicity is actually perceptible.

---

## 2. Feature-driven selection (`assets/mappings.json` → `composition.phrase_scheme`)

A first-match-wins `SelectTable` over **`complexity` (primary) + `edge_activity` (secondary)** — the
same knobs the `form` gate reads — resolved **once per image** in `composition.rs::plan`
(uniform-per-piece, mirroring the S53 motto grain) and threaded into `plan_phrases_scheme`.

```
default: period_4
1. complexity >= 0.6  AND edge_activity >= 0.5   → sentence_8   (busy + energetic: one driven arc)
2. complexity >= 0.85                            → period_4     (very dense but STATIC outlier — see §4.1)
3. complexity >= 0.2                             → group_6      (mid-high detail: a broad long phrase)
4. complexity >= 0.1                             → phrase_5     (low-mid detail: an off-square period)
```

Musical logic: phrasing variety **tracks the image's sense of structural motion**. A busy, energetic
surface (high complexity *and* high edge) drives to one goal → the sentence. A mid-detail surface
gets an off-square period or a broad long phrase. The simplest (near-zero-complexity) images keep the
normative period. The one high-complexity/low-motion outlier keeps the legacy period (§4.1).

BALANCE LAW compliance: expansion is *only* by opening routes to authored structures selected by a
real image feature — curated menu + feature seed, fully deterministic, **no RNG, no hash-synthesis**.

---

## 3. The per-corpus spread achieved

Measured with `cargo test --release --test keydiag_s64 -- --nocapture` (seed 42). Cadence positions
are for the recurring 8-step section.

| image | complexity | edge_activity | scheme | 8-step layout | cadence positions |
|---|---|---|---|---|---|
| AudioHaxImg1 | 0.005 | 0.301 | `period_4` | (short sections only: `S.h`/`Sh`/`S..h`) | {legacy} |
| AudioHaxImg2 | 0.015 | 0.509 | `period_4` | `S..hS..P` | {3, 7} |
| **AudioHaxImg3** | 0.229 | 0.475 | **`group_6`** | **`S....hSP`** | **{5, 7}** |
| **example** | 0.905 | 0.719 | **`sentence_8`** | **`S......P`** | **{7}** |
| **Lena** | 0.164 | 0.471 | **`phrase_5`** | **`S...hS.P`** | **{4, 7}** |
| magicstudio | 1.000 | 0.106 | `period_4` | `S..hS..P` | {3, 7} |

**All four schemes are reached; no collapse onto one.** The universal `S..hS..P` ({3,7}) — which
previously appeared on **every** section of **every** image — now appears on only 3 of 6 pieces, and
is no longer universal: the corpus carries **four distinct cadence patterns** ({3,7}, {4,7}, {5,7},
{7}). Two pieces played back-to-back no longer share the interior-cadence beat, so the pre-cadence
"triplet" and the cadence "half-note" the operator transcribed stop recurring at a fixed place.

The threshold `complexity >= 0.1` (rule 4) deliberately keeps Img1/Img2 (complexity 0.005/0.015) on
`period_4`; Img1 is thereby untouched, protecting its razor-thin F5a baseline (§4).

---

## 4. Cadence-density and F5a — the design constraint that shaped the roster

The S49 **F5a within-piece anti-fusion floor** (`variety_scorecard_s45`) measures the fraction of
co-sounding role-pairs whose onset grids are *distinct*, ratcheted per-image against a pre-slice-2
baseline (tolerance 0.050). A **cadence ring collapses every role to the same single sustained note
at offset ~0**, so it is a homophonic step where role-pairs FUSE. Therefore:

> **A scheme that increases a section's cadence COUNT lowers F5a** by diluting the distinct-role-pair
> fraction with extra fused ring steps.

The diagnostic's literal `asymmetric_3` (3-step groups → cadence periodicity 3) has cadence density
**3/8** vs the legacy 2/8. Built and measured, it dropped **Lena** from baseline 0.743 to **0.686**
(below the 0.693 ratchet floor) — a role pair fused onto the added ring. Both 0.743-baseline images
(Lena, Img3) would regress. **Rejected.**

The shipped roster is therefore **cadence-density-bounded relative to legacy for every
corpus-reachable section length**: for every `n` the corpus actually produces (`n ∈ {9,10,12,14}`,
max 14 — see the reachability note in §7), every non-legacy scheme's cadence count is `<=` the
`Period4` count for the *same* `n`, so none adds a homophonic ring step over the legacy baseline. At
the canonical 8-step section: `phrase_5` and `group_6` are density-neutral (2 cadences, repositioned)
and `sentence_8` is density-*reducing* (1 cadence) — F5a actually *rises* on `example`
(0.654 → 0.704). (This "≤ legacy for the same reachable `n`" invariant has ONE unreachable exception at
`n=16`, documented in §7; it exposes no shipped gate.) All six images HOLD at or above baseline;
`variety_scorecard_s45` 3/3.

### 4.1 The magicstudio F4 exception

`magicstudio-art` is complexity 1.000 but **edge_activity 0.106 — the lowest in the corpus** (a
saturated, static field with no linear motion). Empirically, *any* cadence-repositioning scheme
(`phrase_5`, `group_6`, `sentence_8` all tested) drops its already-low-seated melody below the S48
**F4 inverse-register-compensation** threshold (seat < 69) on a degenerate 4-step sample where
`corr(register_gap, separation) = +0.000` (the comp gives constant separation → zero variance),
tripping the F4 defect gate. This is a comp-layer register quirk exposed by magic's low seat, not a
phrase-layer fault, and fixing it would mean out-of-scope realizer surgery.

**Resolution:** magic keeps `period_4` (rule 2, `complexity >= 0.85`), which holds its melody register
byte-identical to the passing baseline. Musically honest: a motionless field has no internal
articulation to hang off-square cadences on, so the densest-but-most-static image keeps the normative
period while the phrase spread is carried by the images that have real structural motion.

---

## 5. Freeze-safety argument (why identity / neutral stays byte-frozen)

1. **`engine.rs` is untouched.** Its frozen call `chord_engine.plan_phrases(&chords)` (line 431) still
   exists and now delegates to `plan_phrases_scheme(chords, PhraseScheme::Period4)` → the legacy
   length rule → byte-identical output. No signature changed on the method `engine.rs` calls.
2. **`engine_equivalence` (9/9) is structurally immune.** It builds a hand-written `fixed_plan()` and
   realizes it under `OrchestrationProfile::identity()` — it never calls `plan_phrases` *or*
   `composition::plan`. Confirmed 9/9 after the change.
3. **The identity/neutral plan path resolves to `Period4`.** `ImageUnderstanding::neutral()` carries
   `complexity = 0.0`, which matches no `phrase_scheme` rule → the `default` `"period_4"` →
   `PhraseScheme::from_id` → `Period4`. Every neutral witness (`variety_s45` Property E,
   `s52_probe_identity`, the `variety_scorecard` neutral fixtures) takes the legacy period.
4. **No `StepPlan` field was added.** The realizer's cadence/pre-cadence gates read the *existing*
   `position_in_phrase` / `phrase_len` fields; the scheme only changes how `plan_phrases_scheme`
   *stamps* them. (Adding a field would have broken the test fixtures that construct `StepPlan`
   literally — `engine_equivalence::fixed_plan`, `prominence_s43::interior_step` — which are off-limits.)
5. **Back-compat.** `phrase_scheme` is `#[serde(default)]` on both `CompositionMappings` and
   `PlanMappings`; an old `mappings.json` with no such block yields an empty `SelectTable` → `""` →
   `Period4`.

**Full suite: 597 passed, 0 failed.** `engine.rs` sha256 unchanged. `clippy --all-targets -D
clippy::correctness` clean on the touched files.

---

## 6. Cadence-gesture variation — DEFERRED to S65 (with the reason)

The task allowed a *light* fold-in of cadence-GESTURE-TYPE variation (feminine ending vs. accel-run
vs. suspension) if freeze-safe. **Deferred**, because:

- Varying the gesture *type* requires the realizer to know the scheme, which means threading a scheme
  handle into `realize_rhythm` — and the only clean carrier is a new `StepPlan` field. **A new
  `StepPlan` field is impossible this slice**: `engine_equivalence::fixed_plan` and
  `prominence_s43::interior_step` construct `StepPlan` with explicit fields and are test files I must
  not edit — a new field breaks their compile.
- Touching the cadence ring (`:2252`) or the pre-cadence arpeggio (`:2728`) directly is exactly the
  `engine_equivalence` realizer surface; not worth the freeze risk for a secondary win.

**What we got for free instead:** the gesture *placement* already varies per scheme, because the
pre-cadence gate `position_in_phrase + 2 >= phrase_len` and the cadence ring both read the StepPlan
geometry. So across the corpus the pre-cadence "triplet" now lands at step 6 (`sentence_8`), step 5
(`group_6`), step 4 (`phrase_5`) or step 3 (`period_4`), and the cadence "half-note" recurs at a
different periodicity per piece. The *placement* spread is the substantive win; the *type* catalogue
is the clean S65 follow-on (it should land together with a `StepPlan` schema change the Test Engineer
can re-baseline).

---

## 7. Public API surface (for the Test Engineer)

- `chord_engine::PhraseScheme` — `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]` enum;
  variants `Period4` (`#[default]`), `Phrase5`, `Group6`, `Sentence8`.
- `PhraseScheme::from_id(&str) -> PhraseScheme` — `"period_4"`/unknown/`""` → `Period4`;
  `"phrase_5"` → `Phrase5`; `"group_6"` → `Group6`; `"sentence_8"` → `Sentence8`.
- `ChordEngine::plan_phrases(&self, &[Chord]) -> Vec<StepPlan>` — UNCHANGED signature; now a freeze
  delegate to `plan_phrases_scheme(chords, Period4)`.
- `ChordEngine::plan_phrases_scheme(&self, &[Chord], PhraseScheme) -> Vec<StepPlan>` — the scheme-aware
  planner. `Period4` == legacy byte-for-byte.
- `composition::PlanMappings.phrase_scheme: SelectTable` — the selector (loaded from
  `mappings.json`).

**Assertions the Test Engineer should add (phrase-period-variety net + identity-neutrality witness):**

- **Identity neutrality:** `plan_phrases(chords) == plan_phrases_scheme(chords, PhraseScheme::Period4)`
  for arbitrary chord inputs (byte-identical `Vec<StepPlan>`); and
  `PhraseScheme::from_id("") == PhraseScheme::Period4`, `from_id("period_4") == Period4`.
- **Per-image scheme assignment (seed 42, real corpus):**
  Img1 → `period_4`, Img2 → `period_4`, Img3 → `group_6`, example → `sentence_8`, Lena → `phrase_5`,
  magicstudio → `period_4` — assert via `pm.phrase_scheme.select(&understand(name))` mapped through
  `PhraseScheme::from_id`.
- **Cadence-periodicity spread:** over the corpus, the set of interior-cadence positions in the 8-step
  sections has **>= 3 distinct patterns** (observed {3,7}, {4,7}, {5,7}, {7}) — i.e. `S..hS..P` is no
  longer universal.
- **Cadence-density bound (freeze/F5a guard):** for every scheme and every **corpus-reachable**
  section length `n`, the number of `HalfCadence`/`PerfectAuthenticCadence` steps `<=` the `Period4`
  count for the same `n` (no scheme adds cadence rings over legacy). Reachable `n = 8 +
  round(edge_activity·8)`; the corpus max `edge_activity` is `0.719` → `n <= 14`, so the reachable set
  is `{9,10,12,14}`, verified: at each, both `phrase_5` and `group_6` yield `<=` legacy's cadence
  count (legacy itself already runs 3–4 cadences at the non-power-of-two `n=9/10/12/14`).
  **Documented exception — `n=16` (UNREACHABLE):** at `n=16` legacy takes `phrase_len 8` → **2**
  cadences, whereas `phrase_5` → **4** and `group_6` → **3** (both exceed). But `n=16` requires
  `round(edge_activity·8) = 8`, i.e. `edge_activity >= ~0.94` — above the corpus max (0.719, `n=14`),
  so **no shipped section ever hits it and no F5a gate leaks**. The exposure is purely hypothetical
  (a future non-corpus image busier than any in the set); at worst it adds one extra cadence ring
  (`group_6`) or two (`phrase_5`) on that image — **no freeze break, no determinism break**, just a
  mild F5a re-check owed if such an image ever appears (candidate S65 hardening: cap non-legacy
  phrase counts at the legacy count for the same `n`). `sentence_8` never exceeds legacy at any `n`.
- **Length bound:** every stamped `phrase_len` (this_len) ∈ `1..=8` (observed set {2,3,4,5,6,8}).
