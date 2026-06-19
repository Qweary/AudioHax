# S42 Ground-Truth Trace — Per-Role Realized Output for `example.jpg` vs `Lena.png`

DESIGN/DIAGNOSIS DOCUMENT. No production code was changed to produce it; the
empirical data below came from throwaway `eprintln!` instrumentation that was
reverted before this document was finalized (working tree clean; the frozen
realization kernel `src/engine.rs` sha256 is unchanged at
`e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`).

This is the shared evidence base for the S42 diagnosis. Three other diagnostic
lenses (Music Theory, Aesthetics, Affect) read from here.

---

## Method

Both images rendered at `--seed 42` (the deterministic A/B operating point) on
the default pure-Rust build. Instrumentation captured, for **every
instrument-step decision**, the resolved orchestral role, the section
label/variation/thematic-role, the per-step normalized edge activity, the
section density bias, the resolved per-role prominence (saliency) weight,
whether a theme is present and whether a theme note is active on that step, the
base note, and the full realized `NoteEvent` list (onset offset ms, hold ms,
pitch, velocity). The image-understanding knob vector was dumped once per image.

- `example.jpg`: 42 steps × 4 instruments = 165 role-step decisions captured
  (3 instruments skip a couple of rest steps).
- `Lena.png`: 36 steps × 4 instruments = 144 role-step decisions.

The four sounding roles on both renders are **Bass, HarmonicFill, Melody, Pad**
(layer set `[Bass, Pad, HarmonicFill, Melody]`; no CounterMelody on either).

---

## HEADLINE ANSWER

**The melody is NOT register-buried — but it is dynamically/figurally
indistinct, and the two pieces share a near-identical accompaniment bed.
`example` and `Lena` differ in key, in a small set of accompaniment-figuration
details, and in whether the melody line carries a real theme — but NONE of
those differences is foregrounded, so the surface the ear hears is essentially
the same accompaniment gait in two keys.**

Two structural facts, both confirmed empirically, explain why the operator
could not tell theme-bearing from theme-less:

1. **The per-role saliency / prominence weight resolves to NEUTRAL (0.5) on
   every step of BOTH images.** The entire foregrounding system (the lift that
   would make the melody louder, higher, and rhythmically freer than the bed) is
   present in the code and is freeze-safe, but it is *gated off* for these two
   images by a selection threshold (see Part B §3). With prominence neutral, the
   melody sits only +2 velocity above neutral and on its register floor — it
   blends into a bed that is, role for role, just as loud (the HarmonicFill is
   actually the *loudest* role on both renders).

2. **The audible recurring gait is the accompaniment** — the Pad's broken-chord
   burst plus the Bass/Fill sustained bed — and that bed is driven by a single
   global activity scalar and a coarse per-section figuration profile, neither of
   which encodes the theme. `example` carries a real melodic theme (stated and
   recalled); `Lena` carries NO theme (every step free-selects the top chord
   tone). But because the theme voice is dynamically level with the bed, swapping
   a real theme in for a free-selected top note changes nothing the ear can
   isolate.

So the operator's report is precisely diagnosed: the **theme voice is present
and in the right register, but it is not foregrounded**, and the **accompaniment
is the perceptual subject** and barely differentiates the two images at the
gross-feel level.

---

# PART A — Empirical Per-Role Realized Output

## A.0 Section plans & theme status (the key structural difference)

| | `example.jpg` | `Lena.png` |
|---|---|---|
| Sections (label / variation / thematic-role) | `T` Identity Statement, `V1` Fragmented Development, `V2` Identity Development | `A` Identity Statement, `A'` Identity Return, `B` Identity Contrast |
| Theme present? | **YES** (`theme_present=true` everywhere) | **NO** (`theme_present=false` everywhere) |
| Melody steps actually playing a theme note | **11** (in the `T` Statement section) | **0** (every melody step free-selects) |
| Per-step prominence (saliency) weight | **0.500 (NEUTRAL) on every step** | **0.500 (NEUTRAL) on every step** |
| Per-section density bias | **0.500 (NEUTRAL) on every step** | **0.500 (NEUTRAL) on every step** |

The gait-probe is corroborated: `example` has a real motif; `Lena` has an empty
theme. The *only* place that difference can surface is the Melody voice's pitch
content — and that voice is not foregrounded (prominence neutral), so the
difference is inaudible as a figure-vs-ground change.

## A.1 `example.jpg` — per-role realized output

| Role | Steps | Notes | Pitch (MIDI) range / median | Velocity range / median | Hold ms range / median | Onsets-per-step | Dominant rhythm (onset offsets ms → count) |
|---|---|---|---|---|---|---|---|
| **Bass** | 42 | 50 | 25–47 / 32 | 60–106 / 86 | 110–750 / 383 | 1× ≈34, 2× ≈8 | one sustained root `(0,)` ×34; pre-cadence pickup `(0,468)` ×8 |
| **HarmonicFill** | 42 | 41 | 54–66 / 61 | 77–105 / **101** | 344–750 / 510 | 1× ≈41, rest ×1 | sustained inner tone `(0,)` ×41 |
| **Melody** | 39 | 86 | 56–78 / **69** | 63–109 / 88 | 62–750 / **83** | 1×16, 2×7, 3×8, 4×8 | sustained `(0,)` ×16; arpeggio `(0,156,312,468)` ×8; syncopated `(0,208,416)` ×8; dotted `(0,416)` ×4 |
| **Pad** | 42 | 138 | 56–66 / 62 | 58–106 / 82 | 156–750 / 156 | 4× ≈32, 1× ≈10 | **broken-chord burst `(0,156,313,469)` ×32**; block `(0,)` ×10 |

## A.2 `Lena.png` — per-role realized output

| Role | Steps | Notes | Pitch (MIDI) range / median | Velocity range / median | Hold ms range / median | Onsets-per-step | Dominant rhythm (onset offsets ms → count) |
|---|---|---|---|---|---|---|---|
| **Bass** | 36 | 45 | 37–47 / 42 | 74–102 / 82 | 110–750 / 523 | 1× ≈27, 2× ≈9 | one sustained root `(0,)` ×27; pre-cadence pickup `(0,468)` ×9 |
| **HarmonicFill** | 36 | 35 | 63–68 / 64 | 66–102 / **94** | 344–750 / 565 | 1× ≈35, rest ×1 | sustained inner tone `(0,)` ×35 |
| **Melody** | 36 | 73 | 70–83 / **82** | 74–105 / 81 | 62–750 / **62** | 1×17, 2×10, 4×9 | sustained `(0,)` ×17; arpeggio `(0,156,312,468)` ×9; dotted `(0,416)` ×9 |
| **Pad** | 36 | 90 | 56–71 / 63 | 68–102 / 88 | 688–750 / 688 | 3× ≈27, 1× ≈9 | **block triad `(0,0,0)` ×27** (3 simultaneous voices); single `(0,)` ×9 |

## A.3 Reading of the tables

- **Is the melody present? YES.** It plays on every step and sits in the
  *highest* register band of all four roles (median MIDI 69 example / 82 Lena,
  vs Fill ~61/64 and Bass ~32/42). It is not buried by register.
- **Is the melody foregrounded by loudness? NO.** Its velocity median (88 / 81)
  is level with the Pad (82 / 88) and *below* the HarmonicFill (101 / 94). The
  loudest sustained voice on both renders is the inner HarmonicFill, not the
  melody. The melody gets only its fixed +2 role bias; the saliency lift that
  would open a real gap is neutral (0.5 → 0 nudge).
- **The recurring gait the operator describes = the accompaniment, not the
  melody.** On `example` the Pad runs an even 4-onset broken-chord burst
  `(0,156,313,469)` on 32 of 42 steps; the Bass holds a single sustained root.
  That even quarter-subdivision bed, under sustained Fill, IS the "pleasant
  chords + recurring rhythm" surface. The melody's own dotted/syncopated/
  arpeggio figures (the lines that DO carry the per-image variation) are riding
  *on top of and at the same dynamic level as* that bed, so they read as part of
  the texture rather than as a distinct tune.
- **Do the two pieces differ only in key? NEARLY — at the level the ear weights
  most.** They differ in (a) key/register offset, (b) the Pad figuration profile
  (`example` got the animated broken-chord bed, `Lena` got the plain block
  triad), and (c) theme presence in the melody pitch content. But (b) is a quiet
  inner-voice detail and (c) is not foregrounded, so the **dominant shared
  feature — sustained Bass + sustained Fill + a Pad broken-chord/block bed +a
  same-level melody — is what carries across both**, exactly as reported.

- **Articulation note:** the melody's hold-ms median is very short (83 / 62 ms)
  with a wide spread up to 750 ms. The short median is the busy-band staccato
  figures dominating note count; the long tail is the sustained/cadence notes.
  This is the "note-length extremes" surface signature.

---

# PART B — Signal-Flow Code Trace + Freeze-Status of Candidate Levers

All citations are to the current tree. **`src/engine.rs` is BYTE-FROZEN.** The
realization functions below all live in `src/chord_engine.rs`, which is
freeze-SAFE to edit; the selection tables live in `assets/mappings.json`, also
freeze-safe. The frozen kernel only *calls* these.

## B.1 Where each role's rhythm / figuration is chosen

The single realization entry point is
`chord_engine::realize_step` (`src/chord_engine.rs:1076`). It computes the role,
prominence weight, base note, velocity, then dispatches to
`realize_rhythm` (`src/chord_engine.rs:1481`). **All rhythm-pattern selection is
inside `realize_rhythm`.**

- **The one global activity scalar.** `realize_rhythm` first computes
  `edge_activity` (`src/chord_engine.rs:1504-1515`) = normalized
  `features.edge_density / 0.05`, plus a `(ctx.section.density - 0.5) * GAIN`
  nudge. **Empirically `section.density == 0.5` on every step of both images, so
  the density term is exactly 0** — `edge_activity` is purely the per-step edge
  density. This *same scalar* gates every role's rhythm. It varies per step and
  per image, but it is one number shared across Bass/Fill/Melody/Pad.

- **Bass rhythm** (`src/chord_engine.rs:1635-1714`): branches on
  `ctx.section.orchestration.bass_pattern_resolved`. With the resolved pattern
  `None`/`Sustained` (the case on both images — neither selected a walking/pedal
  profile) it falls to the legacy body: one sustained root `(0,)` per step, or a
  `(0, ¾)` pickup before a cadence. **No melody/image feature read beyond the
  shared `edge_activity` setting the hold length.**

- **HarmonicFill rhythm** (`src/chord_engine.rs:1716-1737`): one sustained inner
  tone `(0,)`, with a rest-as-gesture only when `edge_activity <
  FILL_REST_ACTIVITY (0.10)` on a weak beat. Image-invariant in shape.

- **Pad rhythm / the broken-chord burst** (`src/chord_engine.rs:1739-1816`):
  holds `pad_voices` inner tones. If
  `ctx.section.orchestration.figuration_resolved` is present and non-empty it
  calls `figured_bed` (`src/chord_engine.rs:2278`) to expand the held bed into
  the figure's onset burst; otherwise it emits the sustained block bed. **This is
  the source of the recurring "pleasant changes + even gait."** The `alberti`
  figure (`assets/mappings.json` `figuration_catalogue`) is the even
  `(0, 0.25, 0.5, 0.75)` onset table that renders as `(0,156,313,469)` — the
  burst seen 32× on `example`.

- **Melody rhythm / the dotted + arpeggio figures**
  (`src/chord_engine.rs:1896-1960`): four bands selected by `edge_activity`
  against cutoffs 0.80 / 0.55 / 0.25 (each shifted by `prom_shift`, which is **0
  here** because prominence is neutral):
  - `> 0.80` or pre-cadence → **arpeggio**: `n` even onsets (`src/chord_engine.rs:1912-1924`) — the `(0,156,312,468)` triad arpeggiation the operator names.
  - `> 0.55` → **syncopated**: `(quarter, ¾)` onsets (`src/chord_engine.rs:1925-1933`).
  - `> 0.25` → **dotted**: long-short `(0, ⅔)` pair (`src/chord_engine.rs:1934-1942`) — the dotted figure the operator names.
  - else → **sustained** single tone (`src/chord_engine.rs:1943-1960`).

  **Where the dotted→eighth→triplet figure and the triad arpeggio come from:**
  the dotted figure is the Melody mid-activity band (line 1934); the even triad
  arpeggio is BOTH the Melody high-activity band (line 1912) AND the Pad's
  `alberti`/`broken_chord` figuration burst (lines 1797-1799 → `figured_bed`).
  **Selection reads only the global `edge_activity`** (per-step, image-derived
  but a single scalar) for the Melody, and the **per-section figuration profile**
  for the Pad. Neither reads any sense of "is this the theme" or "how different is
  this image from another."

## B.2 Where the melody is realized and what governs its salience

- **Melody pitch.** `realize_step` (`src/chord_engine.rs:1134-1145`) first asks
  `theme_melody_pitch` (`src/chord_engine.rs:2765`): in a Statement/Return
  section with a non-empty theme it returns the motif pitch (this is the 11
  theme-active melody steps on `example`); otherwise it returns `None` and the
  melody free-selects the **top chord tone** via `role_pitch`
  (`src/chord_engine.rs:1248-1263`). On `Lena` there is no theme, so 100% of
  melody steps free-select.
- **Register floors** (`src/chord_engine.rs:1210-1212`): Bass C2 (36), Fill G3
  (55), Melody G4 (67). The melody floor is already the highest, plus a
  brightness octave lift. So **register puts the melody on top** — confirmed by
  the A-tables. Register is NOT the burial mechanism.
- **Velocity / salience.** `realize_velocity` (`src/chord_engine.rs:1313-1395`):
  level from saturation, a messa-di-voce swell, metric accent, and a fixed
  per-role bias of only **+2 for Melody / −3 for Pad / −1 for Bass**
  (lines 1371-1381). Then a saliency nudge `(prominence_w - 0.5) *
  PROMINENCE_VEL_SPAN(18)` (lines 1390-1392). **With `prominence_w == 0.5`
  (the measured case) this nudge is exactly 0**, so the melody's only edge over
  the bed is +2 velocity — and the HarmonicFill carries no negative bias, so it
  ends up the loudest role.
- **The dormant foregrounding system.** `prominence_weight`
  (`src/chord_engine.rs:1013-1022`) returns `PROMINENCE_NEUTRAL (0.5)` when the
  section's `prominence` Vec is empty. The planner fills that Vec from a
  `prominence` SelectTable (`src/composition.rs:1543-1545`). A `subject_melody`
  profile EXISTS in `assets/mappings.json` (`prominence_catalogue`) and would set
  Melody weight 1.0 (→ +9 velocity, +2 register, lowered rhythm cutoffs so the
  melody subdivides more freely), CounterMelody 0.6, Fill 0.4, Pad 0.3, Bass 0.5
  — i.e. a real figure-over-ground separation. **It is gated off:** its rule
  fires only when `subject_size ∈ [0.05, 0.55]` AND `fg_bg_contrast ≥ 0.25`.

  Measured understanding knobs at `--seed 42`:

  | knob | `example.jpg` | `Lena.png` | `subject_melody` needs |
  |---|---|---|---|
  | `subject_size` | 0.111 | 0.110 | 0.05–0.55 ✓ (both pass) |
  | `fg_bg_contrast` | **0.136** | **0.052** | ≥ 0.25 ✗ (both FAIL) |

  Both images fail the `fg_bg_contrast ≥ 0.25` gate, so prominence falls to the
  empty `uniform` profile → neutral → the foregrounding system never engages.
  **This single threshold is why the melody is dynamically indistinct on these
  images.**

## B.3 Per-lever FREEZE verdicts

The frozen kernel `src/engine.rs` is **not touched by any lever below** — it
only calls `realize_step`/`decide_step` and resolves sections. Every lever lands
in `chord_engine.rs`, `composition.rs`, or `mappings.json`.

### Fix family 1 — SALIENCE (foreground the melody over the bed)

| Sub-lever | Edit site | Frozen `engine.rs`? | Verdict |
|---|---|---|---|
| **Lower/relax the `subject_melody` gate** so it actually fires (e.g. drop the `fg_bg_contrast ≥ 0.25` floor toward ~0.10, or add a default-foreground rule) | `assets/mappings.json` → `composition.prominence.rules` / add a non-empty default | NO | **FREEZE-SAFE — JSON only.** Highest leverage, zero Rust. Turns on the entire dormant lift (+9 vel, +2 reg, freer rhythm) with no code edit. |
| **Add a new prominence profile** (e.g. always-mild melody foreground) | `assets/mappings.json` → `prominence_catalogue` | NO | **FREEZE-SAFE — JSON only.** |
| **Widen the velocity gap constants** (raise `PROMINENCE_VEL_SPAN`, or the fixed `+2`/`-3` role biases) | `src/chord_engine.rs:1372-1379, 985` | NO | **FREEZE-SAFE.** Edits `realize_velocity`/consts in `chord_engine.rs`. |
| **Reduce the HarmonicFill loudness** (it is currently the loudest role) — add a small negative Fill bias like Pad's | `src/chord_engine.rs:1371-1381` (`realize_velocity` per-role match) | NO | **FREEZE-SAFE.** Pure addition to the role-bias match. |
| **Raise the melody register lift** (`PROMINENCE_REG_SPAN`) | `src/chord_engine.rs:995, 1259-1261` | NO | **FREEZE-SAFE** (within the existing sum-clamp). |

### Fix family 2 — ACCOMPANIMENT VARIATION (stop pieces sharing a gait)

| Sub-lever | Edit site | Frozen `engine.rs`? | Verdict |
|---|---|---|---|
| **Make figuration selection more discriminating** (so two different images pick visibly different Pad figures, not the same alberti/block) | `assets/mappings.json` → `composition.texture.rules` (and/or `prominence`/`figuration` knob thresholds) | NO | **FREEZE-SAFE — JSON only.** The `texture` SelectTable already reads image knobs (`arousal`/`valence`/`colorfulness`/`subject_energy`); retune its rule thresholds so distinct images diverge. |
| **Add new figuration patterns** to vary the bed | `assets/mappings.json` → `figuration_catalogue` (+ `figured_bed` already supports them) | NO | **FREEZE-SAFE — JSON only.** |
| **Drive `section.density` off the image** so the `(density-0.5)*GAIN` term in `edge_activity` actually moves the accompaniment busyness per image (it is currently pinned at 0.5) | `src/composition.rs` (planner — where per-section `density` is set) | NO | **FREEZE-SAFE.** Planner change in `composition.rs`; the realizer already reads `ctx.section.density` zero-copy. |
| **Image-drive the bass pattern** (walking/pedal) so the bass gait differs per image | `assets/mappings.json` → bass-pattern selection (+ the realizer's Bass dispatch at `src/chord_engine.rs:1646-1691` already handles resolved patterns) | NO | **FREEZE-SAFE — JSON only** for selection; the Rust dispatch already exists and is freeze-safe. |
| **Make the Pad figuration rhythmically non-uniform** (alberti/broken-chord are even `(0,.25,.5,.75)`; add dotted/swing onset tables) | `assets/mappings.json` → `figuration_catalogue` onset tables | NO | **FREEZE-SAFE — JSON only.** |

**No candidate sub-lever in either family touches the frozen `engine.rs`.** All
fixes are reachable in `assets/mappings.json` (selection-table retuning + new
catalogue rows — the highest-leverage, lowest-risk path) and in
`chord_engine.rs`/`composition.rs` (constants, the velocity role-bias match, and
the planner's per-section density).

## B.4 Highest-leverage freeze-safe lever

**Relax the `subject_melody` prominence gate in `assets/mappings.json` (the
`fg_bg_contrast ≥ 0.25` floor that both images fail), or add a non-empty default
prominence profile.** This is a JSON-only change that activates the entire
already-built foregrounding system (+9 velocity, +2 register, freer melody
rhythm, recessed Pad/Fill). It directly attacks the root cause the trace
isolates: the melody is in the right register but dynamically level with — and
quieter than — the accompaniment bed, so it never reads as the subject. A close
second is reducing the HarmonicFill velocity bias (it is currently the loudest
role), which is a one-line freeze-safe edit to the `realize_velocity` role match.

---

## Appendix — verification

- Render command: `cargo run -- render assets/images/<img> --seed 42 --wav /tmp/x.wav` (pure-Rust default build).
- Instrumentation was throwaway `eprintln!` in `realize_step` (realized-event dump) and after the image-understanding extraction (knob dump), both env-gated and both reverted.
- Working tree clean after revert; `src/engine.rs` sha256 unchanged: `e50c7db189a1585102a885fd1e975bf378b06a9ed56ce26993c6c767a2348261`.
