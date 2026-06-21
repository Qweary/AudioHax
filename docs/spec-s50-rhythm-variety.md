# Spec S50 — RHYTHM-VARIETY RE-RANGE (fix-direction 1)

Author: Rust Architect (DESIGN ONLY — no src/asset/test file modified)
Status: design complete, ready for the Music Theory Specialist (sole writer of `chord_engine.rs`/`composition.rs`/`mappings.json`) + the Affect/Aesthetics taste gate (sole owners of the cut VALUES)
Blueprint: `docs/diag-s49-recurring-rhythm.md` (fix-direction 1, §6 Rank 1)
Scope: RE-RANGE the rhythm selectors so distinct real images SPREAD across the rhythmic surfaces instead of all collapsing onto DOTTED + cell-0 + ballad + 4/4. Cross-piece sameness is the target; within-piece monotony is fix-direction-2 (deferred — §7).

---

## 0. Verification of the live source (drift corrected from the brief)

Every selector named in the brief was read against the live tree. Corrections:

| Item | Brief said | LIVE (verified) | File:line |
|---|---|---|---|
| Band cutoffs | `~:1135-1138` | `MELODY_ARP_CUTOFF=0.80`, `MELODY_SYNC_CUTOFF=0.55`, `MELODY_DOTTED_CUTOFF=0.25` | `src/chord_engine.rs:1061-1063` |
| Edge normalization divisor | `~:2038` / `pure_analysis.rs:760` | `EDGE_ACTIVITY_RANGE_MAX=0.05`; used `(features.edge_density / 0.05).clamp(0,1)` | const `:1939`, used `:2038`; mirror `src/pure_analysis.rs:760` `(g.edge_density / 0.05)` |
| Per-role bias | `~:1137-1193` | `MELODY_RHYTHM_BIAS=+0.06`, `FILL=-0.05`, `PAD=-0.05`, `BASS=-0.10`; `role_rhythm_bias` `:1188`; `melody_total_rhythm_shift` `:1206` | `src/chord_engine.rs:1137-1213` |
| Cell cuts | `~:1770-1811` | `CELL_EDGE_BROAD=0.33`, `CELL_EDGE_BUSY=0.66`, `CELL_COMPLEXITY_PROFILED=0.66`; `pick_rhythm_cell` `:1791` | `src/composition.rs:1770-1811` |
| Arousal composite | `~:328-360` | `affect_composite` `:337`; weights live in `mappings.json composition/affect` (NOT inline) | `src/composition.rs:337-366` + `assets/mappings.json` |
| Character gate | arousal≥0.6 for scherzo/march | confirmed: `composition/character` rules require `arousal ge 0.6`; default `"ballad"` | `assets/mappings.json composition/character`; selected `src/composition.rs:1408` |
| Meter | `~:15,441,2219`, rules `[]` | confirmed default `"four4"`, `rules: []`; parsed `:1409`, stored `:995`/`:1232`; **NO downstream rhythm consumer** | `src/composition.rs`; `assets/mappings.json composition/meter` |

**Critical scale clarification the brief blurred.** There are TWO edge scales and they must not be conflated:

1. `ImageUnderstanding.edge_activity` = `(g.edge_density / 0.05).clamp(0,1)` (`pure_analysis.rs:760`) — a WHOLE-IMAGE, already-normalized 0..1 value. **The measured table (0.301, 0.509, 0.475, 0.719, 0.471, 0.106) is THIS value** — already post-`/0.05`. The cell selector (`pick_rhythm_cell` reads `u.edge_activity`) and the arousal composite (`affect_composite` reads `u.edge_activity`) both consume THIS scale.
2. `PerfFeatures.edge_density` — a PER-BAR RAW edge density (~0.005..0.05) carried per step, re-normalized INSIDE `realize_rhythm` at `:2038` by the SAME `/0.05`. The Melody band ladder (`:2625/2639/2649`) compares THIS post-`/0.05` `edge_activity` against the cut constants.

So the band ladder and the cell selector operate on the SAME nominal 0..1 normalized scale, derived from the same `/0.05` calibration but at two granularities (per-bar vs whole-image). The re-range targets the **cut POSITIONS** on that 0..1 scale; it does NOT change `/0.05` (constraint, §3.1).

Verified arousal/valence for the six images (recomputed from the live weights):

| image | edge_activity | complexity | arousal | valence |
|---|---|---|---|---|
| AudioHaxImg1 | 0.301 | 0.005 | 0.211 | 0.321 |
| AudioHaxImg2 | 0.509 | 0.015 | 0.259 | 0.678 |
| AudioHaxImg3 | 0.475 | 0.229 | 0.391 | 0.584 |
| example | 0.719 | 0.905 | 0.696 | 0.528 |
| Lena | 0.471 | 0.164 | 0.375 | 0.647 |
| magicstudio-art | 0.106 | 1.000 | 0.389 | 0.454 |

engine.rs sha256 (verified at design time): `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261` — **every change below is engine.rs-free** (§3).

---

## 1. CURRENT-STATE DATA FLOW (per selector family)

### Family A — the per-step MELODY band (DOTTED collapse)
```
PerfFeatures.edge_density (per-bar raw ~0.005..0.05)
  → realize_rhythm:2038   base = (edge_density / EDGE_ACTIVITY_RANGE_MAX=0.05).clamp(0,1)
  → +density_nudge (ctx.section.density-0.5)*0.5   (0 on home/identity)
  → edge_activity ∈ 0..1
  → prom_shift = melody_total_rhythm_shift(melody_w)  (0 at neutral 0.5; +bias if foreground)
  → ladder:  pre_cadence||ev>ARP(0.80-shift) → ARPEGGIO (n=3/4)         ← "triplet"
             ev>SYNC(0.55-shift)             → SYNCOPATED
             floor_to_dotted||ev>DOTTED(0.25-shift) → DOTTED (long-short) ← "dotted-q→8th"
             else                            → SUSTAINED (one long tone)  ← "long note"
```
Real photos: per-bar edge re-normalizes into the 0.30–0.55 neighborhood → DOTTED for nearly all (the `floor_to_dotted` foreground floor also routes calm foreground INTO DOTTED). ARPEGGIO needs >0.80, only `example.jpg` approaches it; the "triplet" therefore comes from the STRUCTURAL `pre_cadence` burst, which fires every phrase regardless of image.

### Family B — the theme rhythm-CELL (cell-0 collapse)
```
ImageUnderstanding {edge_activity (whole-image 0..1), complexity}
  → pick_rhythm_cell (composition.rs:1800-1808):
      complexity >= CELL_COMPLEXITY_PROFILED(0.66) && K>3 → cell 3 (profiled/dotted character)
      else edge_activity <  CELL_EDGE_BROAD(0.33)         → cell 1 (broad/augmented)
      else edge_activity <  CELL_EDGE_BUSY(0.66)          → cell 0 (S39 anchor: broad-but-moving)
      else                                                → cell 2 (busy/even-subdivided)
  → resolve_motif_celled cycles the cell's dur_steps → the macro "long notes"
```
Real photos: complexity 0.005–0.23 NEVER reaches 0.66 → cell 3 dead; edge_activity 0.30–0.51 sits inside `[0.33, 0.66)` → **cell 0** for four of six (Img1 at 0.301 < 0.33 → cell 1; magic at 0.106 → cell 1). Effective vocabulary ≈ {cell 0, cell 1}; cells 2 and 3 unreachable.

### Family C — character/arousal (ballad pin → tempo pin)
```
ImageUnderstanding → affect_composite (weights in mappings.json) → arousal, valence
  → composition/character SelectTable.select(u):
      arousal ge 0.6 & valence ge 0.55 → scherzo
      arousal ge 0.6 & valence lt 0.45 → march
      arousal le 0.3 & valence lt 0.35 → lament
      arousal le 0.3 & valence ge 0.55 → hymn
      arousal le 0.35 & valence in[0.35,0.47] → nocturne
      else → DEFAULT "ballad"
  → character_tempo window clamps BPM (ballad 56..96)
```
Real arousal 0.21–0.39 (verified). It never reaches the 0.6 scherzo/march gate AND mostly sits ABOVE the 0.30/0.35 lament/hymn/nocturne gates → falls through to **ballad** for nearly all (only Img1 at arousal 0.211/valence 0.321 satisfies `arousal le 0.3 & valence lt 0.35` → lament). Tempo therefore pins to the slow 56–96 ballad window.

### Family D — meter (hard 4/4)
```
ImageUnderstanding → composition/meter SelectTable (default "four4", rules: []) → ALWAYS Meter::Four4
  → stored on CompositionPlan.meter (:995/:1232)
  → *** NO READER *** : grep confirms no downstream code branches on Meter::Three4/Six8/Two4.
```
Meter is inert schema. Activating it is NOT a cut re-range — it needs a new CONSUMER (bar grid → step-per-bar → beat-strength → onset placement). **Out of scope for this slice (§2.D).**

---

## 2. THE RE-RANGE MECHANISM

Design principle shared by all families: **separate the MECHANISM (a data-relative remap of the selector INPUT, owned by Architecture/Music-Theory) from the CUT VALUE (a perceptual placement, owned by Affect/Aesthetics).** The mechanism re-centers/re-scales the real-image cluster across the full decision range so the EXISTING cut constants once again "bite"; the taste gate then sets where, perceptually, each cut should sit on the re-spread axis. Every mechanism is **monotone** (preserves all direction/ordering property tests) and **freeze-gated at a reference input** (preserves the goldens).

### 2.A — Melody band: a data-relative SPREAD on the band-input edge_activity

**Seam (NEW, in `chord_engine.rs`, inside `realize_rhythm` between the `edge_activity` computation at `:2048` and the band ladder at `:2625`):**

```rust
/// S50 — re-range the BAND-INPUT activity onto the real-image distribution. Maps the measured
/// natural-photo cluster across the full 0..1 decision range so the EXISTING band cuts bite
/// again (instead of all photos landing DOTTED). Piecewise-linear about a center, monotone
/// non-decreasing, fixed-point at the reference activity so the band ladder is byte-neutral
/// there. RNG-free, pure. Applied ONLY to the band ladder's comparison input — the articulation
/// curve and the FILL_REST check keep the UNMAPPED edge_activity (so diversity_s13's articulation
/// goldens do not move; §3.2).
///
/// theory: rhythmic subdivision should track visual activity, but natural photos occupy a
/// COMPRESSED activity sub-band; a one-knee linear stretch about the cluster center re-expands it
/// so SUSTAINED..ARPEGGIO are all reachable. Slope/center are TASTE-OWNED (§6).
fn band_activity_spread(edge_activity: f32) -> f32 { /* signature + doc only */ }
```

Mechanism (the body is the Music Theory Specialist's to write; the SHAPE is specified): a one-knee piecewise-linear stretch
```
out = clamp(BAND_SPREAD_CENTER + (edge_activity - BAND_SPREAD_CENTER) * slope, 0, 1)
where slope = BAND_SPREAD_GAIN_LOW  for edge_activity < BAND_SPREAD_CENTER
            = BAND_SPREAD_GAIN_HIGH for edge_activity >= BAND_SPREAD_CENTER
```
Fixed point at `edge_activity == BAND_SPREAD_CENTER` (the remap is identity there → freeze witness, §3.2). With `GAIN > 1` the cluster (≈0.30–0.55) fans out across the SUSTAINED/DOTTED/SYNC boundaries instead of pancaking onto DOTTED.

It plugs in as: change the band ladder's three comparisons from `edge_activity > (CUTOFF - prom_shift)` to `band_activity_spread(edge_activity) > (CUTOFF - prom_shift)`. **The cut CONSTANTS (0.80/0.55/0.25) are UNCHANGED** — the spread does the re-positioning, which keeps the named cuts stable for `melody_activity_class` (which MUST stay 1:1 with the arm, `:1095`) by applying the SAME `band_activity_spread` inside `melody_activity_class` too (one shared transform, mirroring the shared-cutoff discipline of `:1093`).

TASTE-OWNED VALUES (Affect/Aesthetics to set; placeholders + ranges):
- `BAND_SPREAD_CENTER` — **TASTE-OWNED VALUE** placeholder `0.40`, valid `[0.30, 0.50]`. The activity the cluster pivots about; also the freeze-neutral reference (must equal the reference input chosen in §3.2).
- `BAND_SPREAD_GAIN_LOW` / `BAND_SPREAD_GAIN_HIGH` — **TASTE-OWNED VALUES** placeholders `1.8` / `1.4`, valid `[1.0, 3.0]`. >1 spreads; =1 is a no-op (lets the gate disable the spread). Asymmetric so the calm side opens toward SUSTAINED without over-driving the busy side into a wall of ARPEGGIO.

Alternative the gate may prefer: leave the spread identity and instead MOVE the three cut constants down (`ARP→~0.62`, `SYNC→~0.45`, `DOTTED→~0.33`). Mechanically identical result; the spread-function form is recommended because it keeps `melody_activity_class`/arm coupling automatic and keeps the cut constants at their freeze-documented values. **Both are taste-owned at the value layer.**

### 2.B — Cell selection: re-positioned cuts + a reachable complexity gate

`pick_rhythm_cell` (`composition.rs:1791`). Two problems: (i) complexity 0.66 gate is unreachable (real complexity 0.005–0.23 except the two synthetic-art images), so cell 3 is dead; (ii) the edge cuts 0.33/0.66 put four photos in `[0.33,0.66)` → cell 0.

Mechanism — purely re-position the existing three constants (no new code path; the selector logic at `:1800-1808` is unchanged in SHAPE):
- `CELL_COMPLEXITY_PROFILED` — **TASTE-OWNED VALUE** placeholder `0.20`, valid `[0.12, 0.66]`. Lowered so visually-intricate-but-not-saturated photos (Img3 0.229, magic 1.0) reach the profiled/character cell 3. NOTE: at 0.20, Img3 (0.229) and magic (1.0) take cell 3; Img1/Img2/Lena stay on the density ramp — exactly the decorrelating tiebreak the comment at `:1772` intends, now actually reachable.
- `CELL_EDGE_BROAD` / `CELL_EDGE_BUSY` — **TASTE-OWNED VALUES** placeholders `0.38` / `0.50`, valid BROAD `[0.25,0.42]`, BUSY `[0.45,0.66]`, with the invariant `BROAD < BUSY`. Tightened around the cluster so the four mid-cluster photos SPLIT: edge<0.38 → cell 1 (Img1 0.301, magic 0.106 — but magic is diverted to cell 3 first), `[0.38,0.50)` → cell 0 (Img3 0.475 absent the cell-3 divert; Lena 0.471), `>=0.50` → cell 2 (Img2 0.509, example 0.719). Result across the six: cells {1,0,2,3} all occupied.

Optionally a data-relative spread analogous to 2.A could be applied to `u.edge_activity` before the cell cuts; the simpler re-position is recommended here because the cell cuts are NOT shared with another reader (no `melody_activity_class` coupling), so moving the constants is clean and the taste gate sets them directly.

### 2.C — Character/arousal: re-center the composite OR lower the gates

The honest mid-arousal band is 0.21–0.39. Two interchangeable mechanisms (the gate picks one):

**Option C1 (recommended) — lower the character gates in `mappings.json composition/character`.** Pure data edit, no code. Move the high gates DOWN onto the real band: scherzo/march `arousal ge 0.6 → ge ~0.34`; keep valence splits. This lets Img2/Img3/Lena/example reach scherzo/march (faster, brighter feel) and leaves Img1 (0.211) at lament, so character — and thus tempo via the per-character window — SPREADS.
- TASTE-OWNED VALUES: the new arousal gates (`scherzo/march ge` — **TASTE-OWNED VALUE** placeholder `0.34`, valid `[0.28, 0.60]`) and the existing valence splits. Affect/Aesthetics owns "how energetic must a photo feel to march."

**Option C2 — re-center/expand the arousal composite dynamic range.** Re-scale `arousal` about its real-image mean (≈0.32) so the 0.21–0.39 band stretches across 0.1–0.9, then the EXISTING 0.6 gate bites. Implemented as a post-blend remap in `affect_composite` (`composition.rs:362`) — but this changes the arousal NUMBER that `affect_s22.rs` pins (Scherzo at 0.86, Lament at 0.20), so it requires re-blessing those expected values. C1 (gate move) is preferred: it changes WHERE the cut sits without changing the composite the tests pin.

Either way the **tempo de-cap already exists** (`character_tempo_bpm`, `:374`) — once a photo selects a non-ballad character it gets that character's window automatically. No tempo-path code change needed.

### 2.D — Meter: DEFER (honest recommendation)

**DEFER to fix-direction-2.** Meter is dormant schema with ZERO downstream consumer (verified grep: nothing branches on `Meter::Three4/Six8/Two4`). Activating image-derived meter is NOT a cut re-range — it requires building an entirely new consumer chain: meter → steps-per-bar → beat-strength map → phrase-length plan → onset-placement that honors the bar. That is the per-piece RHYTHMIC-IDENTITY architecture (diagnosis §6 Rank 2), which the diagnosis itself sequences as the structural fix AFTER the re-range. Forcing a half-meter into this slice would either (a) be cosmetic (a meter field nobody reads) or (b) drag the whole Rank-2 build into a slice scoped as tuning, blowing the freeze surface. Recommendation: leave `meter.rules: []` this slice; file the meter-consumer as the opening move of the fix-direction-2 arc.

---

## 3. FREEZE & EQUIVALENCE PRESERVATION

### 3.1 engine.rs untouched
Every edit lands in `chord_engine.rs` (2.A: new `band_activity_spread` fn + its call inside `realize_rhythm` and `melody_activity_class`), `composition.rs` (2.B: three const values; optionally 2.C2), and `mappings.json` (2.C1: character gate numbers). **None touches `src/engine.rs`.** engine.rs only CONSTRUCTS `PerfFeatures` (`:728`) and calls the public `decide_instrument_action`/`realize_step` seam — neither signature changes (2.A adds a private free fn and an internal call; the precedent is the existing `melody_total_rhythm_shift` private helper). sha256 stays `e50c7db1…348261`.

### 3.2 engine_equivalence 9/9 preserved
The goldens (`tests/engine_equivalence.rs`) are pinned on (a) **cadence-step rendering** — `if is_cadence { return vec![sustained(0, step_ms, LEGATO_FRAC)] }` at `:2188/2216` returns BEFORE the band ladder, so the band re-range (2.A) is unreachable on every cadence golden; and (b) the **neutral path** — `StepContext::single_section_default` → `prominence_weight == 0.5`, `theme: None`, `counter_present == false`.

Per change:
- **2.A band spread** — `band_activity_spread` is **freeze-neutral by construction at its fixed point**: at `edge_activity == BAND_SPREAD_CENTER` it returns its input. The goldens never reach the band ladder (cadence path), so there is no golden to move. The non-cadence diversity tests (which DO reach the ladder) are direction/count tests, not byte goldens (§3.3). FURTHER GATE: to be doubly safe on any future non-cadence golden, the spread is identity when `BAND_SPREAD_GAIN_* == 1.0`, and the recommended default keeps the cut CONSTANTS unmoved (the diagnosis-documented `:1061-1063` values), so `melody_activity_class` and the arm stay 1:1. Mirror the S49 discipline: the spread is the only new term, it is identity at the reference activity, exactly as L1's bias is `0.0` at neutral weight.
- **2.B cell cuts** — `pick_rhythm_cell` is on the THEME path; the goldens use `theme: None` (`default_section` comment: "the goldens (240, 114/84, 36/79) do NOT move"), so the cell selector is never consulted by any golden. The cell-0 FREEZE ANCHOR (`motif_s41` P5: `resolve_motif_celled(..,0) == resolve_motif(..)`) is a VOCABULARY identity, untouched — moving the SELECTOR cuts never changes the cell-0 dur-sequence. Byte-safe.
- **2.C character gates** — the character SelectTable is on the PLAN path; the goldens hand-build a fixed plan and never run `CompositionPlanner::plan`, so the character/arousal selection is never consulted by `engine_equivalence`. Byte-safe. (The `affect_s22` property tests DO run the planner — see §3.3.)

### 3.3 Property/regression tests that MOVE (Test Engineer must re-bless EXPECTATIONS, not goldens)
These are direction/count tests, not byte goldens; the re-range is designed to keep them GREEN or to require a documented expected-value update:
- `tests/diversity_s13.rs::test_distinct_images_differ_in_3_dimensions` / `test_normalization_real_photo_edge_range_is_usable` — count/`>=` tests; a monotone spread only INCREASES the spread → stay green. **The articulation tests (`test_articulation_is_continuous_not_three_bands`, `test_articulation_calm_longer_than_busy`) read the UNMAPPED `edge_activity` for the curve (2.A applies the spread ONLY to the band-ladder comparison, NOT to the articulation curve at `:2063-2080` nor to `FILL_REST_ACTIVITY`) → byte-stable.** This separation is a HARD requirement of 2.A.
- `tests/motif_s41.rs` P1/P2/P4 — `>=` count tests on distinct gaits; re-positioned cuts keep cells reachable across the deliberately-wide sweeps (edge {0.10,0.45,0.80}, complexity {0.45,0.85}) → stay green. P5 anchor untouched (§3.2).
- `tests/affect_s22.rs` — if Option C1 (gate move) is taken, the Scherzo/Lament CORNER fixtures (arousal≈0.86 / ≈0.20) still select Scherzo/Lament under lowered gates (0.86 ≥ any gate in [0.28,0.60]; 0.20 still ≤ the lament gate) → stay green. If Option C2 (composite re-center) is taken, the asserted arousal numbers move → **re-bless required**; this is the reason C1 is recommended.
- `tests/prominence_s23.rs` — sweeps edge 0.30/0.85 for foreground/recessive ORDERING; monotone spread preserves ordering and the per-role bias signs are unchanged → stay green.

Decisive freeze rule for the build: **apply the spread to the band-ladder comparison input only; never to the articulation curve, the FILL_REST check, or `/0.05`.** That single discipline is what keeps the articulation goldens byte-identical while the bands re-spread.

---

## 4. THE DECISIVE METRIC / TESTABILITY

Property the Test Engineer encodes (new test, e.g. `tests/rhythm_variety_s50.rs`):

**Definition of "distinct images land on distinct rhythmic surfaces":** across the six bundled images, extract the per-image RHYTHMIC SIGNATURE and require the set of signatures to take ≥ N distinct values, AND require that no two images share the full `[dotted-q, eighth] → triplet → long` motif signature.

Concrete discriminating measurement (deterministic — use a FIXED progression / `realize_step`, NOT `set_features_global`, per the diversity_s13 RNG discipline):
1. **Selector tuple** (planner-level, cheapest, RNG-free): for each image compute `(cell, character, meter)` from `pick_rhythm_cell` / `CompositionPlanner::plan(u).character` / `.meter`, and the per-step `band` from `band_activity_spread(edge_activity)` against the cuts. Require the multiset of `(band, cell, character)` tuples over the six images to have **≥ 4 distinct values** (meter excluded — deferred; was 1 before, target ≥4). Directional sanity: the busiest image (`example`, edge 0.719) must NOT share a band with the calmest (`magic`, edge 0.106).
2. **Rendered onset signature** (audible-surface proof): render each image's melody line over a fixed plan and extract the **onset-count multiset per phrase** + the **first-onset-offset pattern** (which band each step took). Encode the operator's motif as a detector: a piece is "the signature" iff its dominant per-phrase pattern is `DOTTED-onset-pair → pre_cadence n=3 → cadential sustained`. Require **no two of the six images produce the same dominant pattern** (the cross-piece sameness gate), and require the **count of distinct dominant patterns across the six ≥ 3**.
3. **Tempo spread** (character proof): `base_ms_per_step` over the six must take ≥ 3 distinct values (was effectively pinned to the ballad window) — directly observes the character un-pin.

The N thresholds (4 / 3 / 3) are floors comfortably above the pre-S50 state (1 / 1 / ~1) and below the achievable count; pick the final floors with the gate but keep them strictly above the collapse value so a future re-collapse fails loudly. This mirrors `motif_s41`'s "`>=10` distinct gaits" discipline.

---

## 5. DATA-FLOW DIAGRAM (re-ranged path)

```
                        IMAGE
                          │
              understand_image_pure (pure_analysis.rs)
                          │  edge_density/0.05 → edge_activity (whole-image, UNCHANGED /0.05)
                          ▼
        ┌─────────────────────────────────────────────────────────┐
        │ ImageUnderstanding { edge_activity, complexity, sat, ... }│
        └───────┬──────────────────────┬──────────────────┬────────┘
                │                       │                  │
   (B) pick_rhythm_cell        (C) affect_composite   (per-step) PerfFeatures.edge_density
   CELL_EDGE_BROAD*▼           → arousal,valence       (per-bar raw)  /0.05  (UNCHANGED)
   CELL_EDGE_BUSY*             │                              │
   CELL_COMPLEXITY_PROFILED*   ▼                              ▼  edge_activity (band input)
        │              composition/character          ┌──────────────────────┐
        ▼              gates* (C1) → Character         │ (A) band_activity_    │
   cell 0..3          → character_tempo window         │     spread(edge)*     │ ← NEW seam
   (now all           → base_ms_per_step (spread)      │ CENTER*/GAIN_LOW*/    │
    reachable)                                         │ GAIN_HIGH*  (identity │
        │                                              │  at CENTER)           │
        │                                              └──────────┬───────────┘
        │                                                         ▼  (band ladder ONLY)
        │                                       ARP(0.80)/SYNC(0.55)/DOTTED(0.25) - prom_shift
        │                                       (cut CONSTANTS unchanged; spread re-positions)
        │                                                         │
        ▼                                                         ▼
   resolve_motif_celled                              ARPEGGIO / SYNC / DOTTED / SUSTAINED
   (macro durations,                                 (per-step onsets)
    cell-0 anchor frozen)         articulation curve + FILL_REST: UNMAPPED edge_activity (frozen)
        └───────────────────────────┬─────────────────────────────┘
                                     ▼
                          per-piece rhythmic surface  → now SPREADS across the six images

   (D) meter: SelectTable → Meter  ──X── NO downstream consumer  [DEFERRED to fix-direction-2]

   * = TASTE-OWNED cut value (Affect/Aesthetics sets; Architecture owns the mechanism/seam)
```

---

## 6. MIGRATION PATH

Single writer of `chord_engine.rs`/`composition.rs`/`mappings.json` = the Music Theory Specialist. Cut VALUES supplied by Affect/Aesthetics.

**Independent (parallelizable) changes:**
- 2.A (band spread) — chord_engine.rs only.
- 2.B (cell cuts) — composition.rs only.
- 2.C1 (character gates) — mappings.json only.

These touch disjoint files and disjoint selector families → no coordination needed between them; land in any order.

**Ordered steps:**
1. Affect/Aesthetics supply the cut values (see the TASTE-OWNED list): `BAND_SPREAD_CENTER`, `BAND_SPREAD_GAIN_LOW/HIGH`; `CELL_EDGE_BROAD/BUSY`, `CELL_COMPLEXITY_PROFILED`; the character arousal gate(s). Decide C1 vs C2 (recommend C1).
2. Music Theory Specialist: add `band_activity_spread` (signature + body) in chord_engine.rs; route the band-ladder comparison AND `melody_activity_class` through it; **leave the articulation curve, FILL_REST, and `/0.05` reading the unmapped value** (the freeze discipline, §3.3).
3. Music Theory Specialist: re-point the three cell constants in composition.rs.
4. Music Theory Specialist: lower the character gates in mappings.json (C1).
5. Test Engineer: add `tests/rhythm_variety_s50.rs` (§4); re-run `engine_equivalence` (must be 9/9), `diversity_s13`, `motif_s41`, `affect_s22`, `prominence_s23` (must stay green / re-bless only if C2 chosen).
6. Verify: `cargo build --release`, `cargo test`, `cargo clippy -- -W clippy::all`; confirm engine.rs sha256 unchanged.

**What Affect/Aesthetics must supply (the cut-point values):** every constant marked TASTE-OWNED above, with the perceptual rationale (where "busy enough to subdivide" / "energetic enough to march" sits). The taste/affect review is a STANDING gate beside correctness for this slice because the acceptance turns on a perceptual judgment (does the catalogue now sound varied?), exactly the Specialist Marshaling discipline.

---

## 7. RISKS & TRADE-OFFS

1. **Cross-piece vs within-piece (the diagnosis's own caveat).** Re-range fixes SAMENESS-ACROSS-PIECES; it does NOT fix MONOTONY-WITHIN-A-PIECE — each image still gets ONE band/cell/character. The recurring `[dotted→8th]→triplet→long` will spread across the catalogue (different images land in different bands) but a single piece can still repeat its one figure phrase-to-phrase, AND the `pre_cadence` "triplet" + cadential "long note" are STRUCTURAL (fire every phrase regardless of image), so those two elements of the heard motif survive this slice. Honest framing for the operator: this makes the SIX images sound like six different things; it does not yet make one image internally varied. The within-piece fix is fix-direction-2 (per-piece rhythmic identity: meter activation, per-piece phrase/harmonic-rhythm plan, varying the pre-cadence). **Recommend sequencing fix-direction-2 next, with meter activation as its opening move.**

2. **Spreading the bands could over-drive into ARPEGGIO/fragmentation.** A too-high `BAND_SPREAD_GAIN` pushes mid-cluster photos into ARPEGGIO (busy, "computer-like" again) — the inverse failure. Mitigation: asymmetric gains (calm side opens more than busy side), and the taste gate ear-tests on all six. The gain has a valid floor of 1.0 (no-op) so the gate can dial back.

3. **Regressing the S46/S49 figure-ground gains.** The re-range composes WITH (does not replace) the S49 per-role bias (L1), bed phase-sep (L2), articulation contrast (L3), between-section density (L4): 2.A applies the SAME spread inside `melody_activity_class`, so the governor still sees the melody's true (spread) class → F5b (bed ≤ melody onsets), "melody most-active/on-top", and the S45 counter still hold. The per-role bias signs and the `floor_to_dotted` foreground floor are untouched. Risk: if the spread lifts a calm FOREGROUND melody out of SUSTAINED before `floor_to_dotted` would, the floor becomes a rare no-op — benign (the melody is already moving), but the gate should confirm F5b stays satisfied (melody never QUIETER than the bed) on all six. Add an F5b re-check to the §4 test as a guard.

4. **Cell-3 reachability vs the decorrelation intent.** Lowering `CELL_COMPLEXITY_PROFILED` to ~0.20 makes cell 3 reachable but risks OVER-diverting (every mildly-textured photo → character cell). Mitigation: it is a single taste-owned value with a wide valid range; the gate sets it against the six so the divert is selective (Img3/magic divert, the rest stay on the density ramp).

5. **C2 (composite re-center) breaks affect_s22 expectations.** Mitigated by recommending C1 (gate move), which leaves the composite — and its pinned numbers — intact.
