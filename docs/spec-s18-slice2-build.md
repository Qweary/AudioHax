# S18 Slice 2 — The Buildable Per-File Spec (Saliency Region Reader + Real Counter-Melody)

**Author role:** Rust Architect (DESIGN ONLY — no source/test/asset modified by this document; `docs/` only).
**Date:** 2026-06-15.
**Status:** LOCKED for build. This is the single contract the two implementer lanes build against. It turns the two S16 texture/density design docs (`design-s16-texture-musical.md` §2.3/§3, `design-s16-texture-engine.md` §2/§3.5) into a per-file build spec against the **as-built S17 seam** (`spec-s17-slice1-build.md` shipped at HEAD `0f98d0f`), and folds the operator's S17 re-listen verdict (the HARMONIC-RHYTHM "empty periods" complaint) into the counter-melody contract.

**Grounded against the actual head working tree** (verified, NOT trusted from the S16 design docs which predate the build and drifted in naming):
- `src/pure_analysis.rs`: `understand_image_pure` `:469` (field-copies the S13 globals into the reserved saliency fields with whole-image DEFAULTS — `subject_size: 1.0` `:491`, `subject_hue == dominant_hue` `:492`, `subject_saturation == g.avg_saturation` `:493`, `fg_bg_contrast: 0.0` `:494`, `mass_centroid: (0.5,0.5)` `:487`, `quadrant_contrast: 0.0` `:488`, `vertical_emphasis: 0.5` `:490`); kernels `to_gray` `:286`, `hsv_means` `:169`, `edge_density_pure` `:310`, `rgb_to_hsv` `:122`; the owned-`RgbImage` boundary discipline (no music type enters the module).
- `src/composition.rs`: `ImageUnderstanding` `:39` (the reserved saliency fields `:73`–`:81`; NOTE it does NOT yet carry the `subject_energy`/`foreground_energy`/`background_energy` triplet — S18 adds it); `Knob` `:272` + `Knob::read` `:292` (already has `SubjectSize` `:285` + `FgBgContrast` `:286` arms — S18 adds the three energy variants); `SelectTable` `:367` + `select` `:378`; `OrchestrationProfile`/`LayerRole` `:185`–`:213` (shipped S17); `PlanMappings.texture`/`texture_catalogue` `:404`/`:412` (shipped); the planner's `texture.select(u)` `:692`; `ImageUnderstanding::neutral` `:87` (every field must get a default).
- `src/chord_engine.rs`: `OrchestralRole::{Pad, CounterMelody}` `:813`/`:818` (shipped); `assign_role` `:918`; `realize_step` `:956` (signature FROZEN; reads `pad_voices` `:968`; calls `realize_rhythm` `:1020` with `pad_voices` `:1029`); the `CounterMelody` realize STUB `:1474`–`:1484` (delegates to the rest-fixed HarmonicFill figure — this is what Slice 2 fills); `realize_rhythm` `:1258` (private free fn; receives `step: &StepPlan` + `pad_voices: u8`, NOT `ctx`); `role_pitch` `:1044` (private; Pad/Counter share the inner-tone seat `:1088`); `theme_melody_pitch` `:1737` (**`pub`**); `voice_lead_one` `:2004` (private); `has_parallel_perfects` `:1970` (private); `upper_voice_candidates` `:1936` (private); `interval_class` `:1959` (private); `seat_pc_in_register` `:1112` (private); `degree_to_pitch` `:1691` (private); `FILL_REST_ACTIVITY` `:1241`; `PAD_OVERLAP_FRAC` `:1252`; `MAX_UPPER_VOICE_MOTION` `:518` (`pub const`, == 7); register floors `:1038`–`:1040`; the `sustained` closure `:1343` (the `(frac*rit).min(1.20)` cap); `Chord` `:34` (`notes: Vec<u8>`); `StepPlan` `:495` (`chord`, `position_in_phrase`, `phrase_len`, `position`).
- `src/engine.rs`: `decide_step` compose path `:542`–`:564` (builds ONE `StepContext` per step, borrowed by each per-instrument `decide_instrument_action`; the cursor is non-looping); the legacy flat path `:579`–`:592` (identity profile). **No engine.rs change in Slice 2.**
- `tests/engine_equivalence.rs`: `default_section` `:81` carries `OrchestrationProfile::identity()` `:97`; goldens `G_BASS_NOTE=36` `:130`, `G_MELODY_NOTE=79` `:135`, cadence vel `114`/`84` + hold `240`, `MS_PER_STEP=200` `:124`.
- `tests/texture_s17.rs`: the S17 net (Pad bed, rest fix, assign_role witness, CounterMelody-stub-equals-HarmonicFill `:398`). **The S18 counter-line REPLACES the stub, so the §11.7 `test_countermelody_stub_equals_harmonicfill` assertion is the ONE S17 property that S18 deliberately supersedes — see §6.4.**
- `assets/mappings.json`: `texture_catalogue` `:137`–`:140` + `texture` SelectTable `:141`–`:144` (`default: "pad_bed"`, `rules: []`).

**Scope is LOCKED** and is NOT widened: Slice 2 = the **pure-Rust SALIENCY REGION READER** (fills the reserved saliency fields + the energy triplet, adds the new saliency `Knob` variants) + the **real COUNTER-MELODY line** (replaces the S17 stub) + the **`mappings.json` wiring** that ADDS a CounterMelody-bearing orchestration profile selected by the new saliency knobs. Two coupled mechanisms that LAND TOGETHER — the saliency reader is what ASSIGNS the counter-melody layer.

---

## 0. The deliverables + the operator verdict (settled — build to these)

1. **SALIENCY REGION READER** in `pure_analysis.rs`: `analyze_regions_pure` (a 3×3 region pass reusing `to_gray`/`hsv_means`/`edge_density_pure`, ZERO new dependency, deterministic, no ML) + `pick_subject_region` (center-surround contrast blend). FILL the reserved `ImageUnderstanding` saliency fields from real region values + add the `subject_energy`/`foreground_energy`/`background_energy` triplet. (§1.)
2. **New saliency `Knob` variants** + `Knob::read` arms + the `texture` SelectTable rules + a CounterMelody-bearing `OrchestrationProfile` row, all in `composition.rs` + `mappings.json`. (§2.)
3. **Real COUNTER-MELODY line** replacing the S17 stub in `chord_engine.rs`: chord-tone-nearest-previous-pitch (reuse the `voice_lead_one`/`upper_voice_candidates` nearest-tone search), contrary/oblique motion vs the melody (extend `has_parallel_perfects` to the (melody,counter) pair), rhythmically complementary, **preferentially active during held-chord / melody-static stretches** (the operator's "empty periods" answer). (§3.)

**OPERATOR RE-LISTEN VERDICT (load-bearing — folded into §3.4).** The operator re-listened to the S17 pad output: *better — chords now sound at relevant times, harmony fills in; no pad micro-fix needed.* The residual complaint is **HARMONIC RHYTHM**: when one chord holds for several steps, *nothing moves underneath it* — "empty periods" where no other chord support plays. The operator explicitly does NOT want naive extra chord stabs (sounds sloppy). **The CounterMelody is the principled first lever against exactly this**, so §3.4 makes the counter-line's rhythmic-complementarity rule **preferentially ACTIVATE during held-chord / melody-static stretches** — a moving line weaving through the held harmony is the musical answer to "empty periods."

**OUT OF SCOPE (the designed Slice 3, design-first — do NOT let the counter-melody grow into it):** the fuller accompaniment-figuration system — Alberti bass, comping patterns, on/off-beat placement, beat-position- and style-dependent figuration. **One moving line, rhythmically complementary, is the Slice-2 ceiling.** Also out of scope: the DoG saliency-mask upgrade (the region proxy ships; the mask is a later refinement into the same fields), per-phrase density modulation, and any `num_instruments` widening. (§7.)

---

## 1. DELIVERABLE 1 — THE SALIENCY REGION READER (Rust Implementer lane; `pure_analysis.rs`)

### 1.1 Region-grid geometry — LOCKED: 3×3 rule-of-thirds, center cell = subject candidate, border ring = background

**Decision: a fixed 3×3 rule-of-thirds grid**, NOT a center-disc-vs-border split. Rationale: (a) it reuses the existing kernels per sub-rectangle with trivial integer crop math (no disc-mask arithmetic), exactly the `scan_steps` rect discipline already in the module; (b) it yields BOTH the center-vs-border contrast the subject reader needs AND the quadrant/vertical means the `mass_centroid`/`vertical_emphasis`/`quadrant_contrast` fields want, from one pass; (c) it is deterministic and parity-free (no learned model, no RNG, no clock). The grid partitions `[0,w)×[0,h)` into 9 cells by thirds (the last row/col absorbs the rounding remainder, the same last-section rule `scan_steps` uses at `:665`/`:681`):

```
 cell index (row-major):   0 1 2      center  = cell 4
                           3 4 5      border ring = cells {0,1,2,3,5,6,7,8}
                           6 7 8
```

```rust
// ── src/pure_analysis.rs (NEW, private) ────────────────────────────────────
/// One region's cheap perceptual stats — the SAME kernels analyze_global_pure uses,
/// computed over a sub-rectangle. Pure-Rust; no new dependency. Deterministic.
struct RegionStats {
    /// Region centroid in normalized image coords (0..1, 0..1).
    center: (f32, f32),
    /// Area fraction of the whole image, 0..1.
    area_frac: f32,
    mean_value: f32,      // luminance 0..100 (to_gray mean over the cell)
    mean_saturation: f32, // 0..100 (hsv_means over the cell)
    edge_energy: f32,     // 0..1 (edge_density_pure over the cell's gray)
    dominant_hue: f32,    // 0..360 (hsv_means circular hue over the cell)
}

/// Decompose `img` into a `(cols, rows)` rule-of-thirds grid (LOCK: (3,3)) and compute
/// each cell's stats by cropping the sub-rectangle (image::imageops::crop_imm(..).to_image(),
/// the same owned-buffer path analyze_section_pure already consumes) and running the existing
/// kernels. Returns the 9 cells in row-major order. ONE extra pass over the pixels. Pure.
fn analyze_regions_pure(img: &RgbImage, grid: (u32, u32)) -> Vec<RegionStats>;

/// The center-surround saliency blend → (subject_region_index, saliency_score).
/// LOCK: subject_region = argmax over cells of
///   score(cell) = W_CENTER * center_bias(cell)
///               + W_CONTRAST * local_contrast(cell, neighbours)
///               + W_SAT      * (cell.mean_saturation/100 - border_mean_saturation/100).clamp(0,1)
/// where center_bias(cell)   = 1.0 for the center cell (idx 4), 0.5 for the 4 edge-mid cells
///                             (1,3,5,7), 0.0 for the 4 corners (0,2,6,8) — the rule-of-thirds
///                             prior (a subject is usually central/at a third intersection);
///       local_contrast(cell) = (|cell.mean_value - mean_value_of_8_neighbours|/100)
///                             + cell.edge_energy, clamped 0..1 (luminance pop + detail pop);
///       border_mean_saturation = mean of mean_saturation over the 8 border cells.
/// First-match-wins argmax; on a tie pick the MOST-CENTRAL cell (lowest |center-(0.5,0.5)|),
/// so a flat field deterministically resolves to the center. Pure arithmetic, NO learned model.
/// LOCKED weights: W_CENTER = 0.5, W_CONTRAST = 0.35, W_SAT = 0.15 (center-bias dominant — the
/// proxy is "is there a prominent central/thirds region", per design-s16-engine §2.1).
fn pick_subject_region(regions: &[RegionStats]) -> (usize, f32);
```

**Honest fidelity note (carry the `pure_analysis.rs:13` discipline):** this is a *contrast/center-bias proxy, not segmentation*. It answers "is there a prominent region and how prominent" well and "what object is it" not at all. The owner's ear is the gate; the DoG-mask upgrade into the same fields is a later slice.

### 1.2 Mapping region values → the reserved `ImageUnderstanding` fields (LOCKED formulas)

`understand_image_pure` (`:469`) gains, BETWEEN the `analyze_global_pure` call (`:472`) and the struct literal (`:475`):

```rust
let regions = analyze_regions_pure(img, (3, 3));
let (subj_idx, _score) = pick_subject_region(&regions);
let subj = &regions[subj_idx];
// border ring = all cells except the chosen subject cell (NOT just the geometric border —
// the subject may have resolved to an edge-mid cell; "background" is everything-but-subject).
let border: Vec<&RegionStats> = regions.iter().enumerate()
    .filter(|(i, _)| *i != subj_idx).map(|(_, r)| r).collect();
let border_value      = mean(border.iter().map(|r| r.mean_value));      // 0..100
let border_saturation = mean(border.iter().map(|r| r.mean_saturation)); // 0..100
let border_edge       = mean(border.iter().map(|r| r.edge_energy));     // 0..1
```

The struct literal's reserved-field defaults are REPLACED by:

| Field | LOCKED formula | Range |
|---|---|---|
| `subject_size` | `subj.area_frac` (1/9 for the 3×3 grid when a single cell wins; a uniform-field tie still resolves to one cell → ~0.11, correctly "small subject"; the field's "no subject → ~1.0" intent is carried by `fg_bg_contrast → 0` gating the counter-melody OFF, not by `subject_size`) | 0..1 |
| `subject_hue` | `subj.dominant_hue` | 0..360 |
| `subject_saturation` | `subj.mean_saturation` | 0..100 |
| `fg_bg_contrast` | `((subj.mean_value - border_value).abs()/100 + (subj.mean_saturation - border_saturation).abs()/100 + (subj.edge_energy - border_edge).abs()).clamp(0,1)` — the value/saturation/edge contrast of the subject cell vs the border ring. **The single most load-bearing knob** (it gates counter-melody presence). 0 on a flat field, high on a strong subject. | 0..1 |
| `mass_centroid` | luminance-weighted centroid of the 9 cell `mean_value`s over their `center` coords: `(Σ v_i·x_i / Σ v_i, Σ v_i·y_i / Σ v_i)` | (0..1, 0..1) |
| `vertical_emphasis` | upper-third mass fraction: `(v0+v1+v2) / (Σ all v_i)` (mean values of the top row over the total) — high when the bright/heavy mass sits high in frame | 0..1 |
| `quadrant_contrast` | population std-dev of the 9 cell `mean_value`s, normalized: `(stddev(v_i)/50).clamp(0,1)` — spread of luminance across the grid (a flat field → 0, a high-contrast composition → high) | 0..1 |

### 1.3 The energy triplet (NEW fields on `ImageUnderstanding`) — definitions

`ImageUnderstanding` (`composition.rs:39`) gains THREE fields (Rust Implementer adds them in `composition.rs`; `pure_analysis.rs` fills them; `ImageUnderstanding::neutral` `:87` gets the `0.0` defaults):

```rust
// added after fg_bg_contrast (composition.rs:81), all 0..1, default 0.0:
/// Energy in the salient (subject) region, 0..1. NEW S18.
pub subject_energy: f32,
/// Energy in the foreground band (the non-subject central cells: the 4 edge-mid
/// cells 1,3,5,7 minus the subject cell), 0..1. NEW S18.
pub foreground_energy: f32,
/// Energy in the background band (the 4 corner cells 0,2,6,8 minus the subject), 0..1. NEW S18.
pub background_energy: f32,
```

LOCKED definitions ("energy" == the region's `edge_energy`, the cheap activity proxy):
- `subject_energy` = `subj.edge_energy`.
- `foreground_energy` = mean `edge_energy` over the edge-mid cells {1,3,5,7} excluding the subject cell (if all four ARE the subject — impossible for a single argmax — fall back to `border_edge`).
- `background_energy` = mean `edge_energy` over the corner cells {0,2,6,8} excluding the subject cell.

These three are what the texture SelectTable reads to decide counter-melody presence (`foreground_energy` busy → counter present) and pad depth (`background_energy`/`value_key` → pad voices) — §2.

### 1.4 Tests (Test Engineer; add to a NEW `tests/saliency_s18.rs` + 2 in-module unit tests)

In-`pure_analysis.rs` module unit tests (hand-built `RgbImage`s, no planner, no music):
1. **`analyze_regions_pure` cell count + geometry**: a 30×30 image → 9 cells, areas sum to ~1.0, `center` coords are the 9 thirds-centroids, the last row/col absorbs the remainder on a non-divisible size (31×31 → still 9 cells, areas sum to 1.0).
2. **`pick_subject_region` center-surround**: a uniformly flat field → the CENTER cell (idx 4) wins by the center-bias tie-break (and `fg_bg_contrast` computed from it ≈ 0); a single bright/high-edge blob in the center → center cell wins with a HIGH score; a single bright blob in a corner → that corner cell wins over the flat center (contrast beats center-bias when contrast is strong).

In `tests/saliency_s18.rs` (whole-image `understand_image_pure` over hand-built images):
3. **Saliency spread**: a flat solid-color image → `fg_bg_contrast ≈ 0`, `subject_energy ≈ 0`, `foreground_energy ≈ 0`; a "subject on a quiet ground" image (central textured patch on a flat field) → `fg_bg_contrast` strictly > the flat image's, `foreground_energy` < `subject_energy` (the action is in the subject); a busy-everywhere image → `foreground_energy` HIGH. (This is the design-s16-engine §5.3 "saliency spread across distinct images" net.)
4. **Field ranges + determinism**: every new field is in its documented range for a sweep of hand-built images, and `understand_image_pure(img) == understand_image_pure(img)` (pure).

### 1.5 Byte-freeze guarantee for Deliverable 1

The saliency reader runs ONLY inside `understand_image_pure`, on the **compose path** the equivalence net never calls (`engine_equivalence.rs` builds a FIXED plan by hand and never invokes the planner/analysis — verified header 7–11). The three new `ImageUnderstanding` fields are additive; the net does not construct an `ImageUnderstanding`. **`engine_equivalence` stays BYTE-GREEN with goldens 36/79/114/84/240 unmoved** — same boundary S13/S17 used.

---

## 2. DELIVERABLE 2 — SALIENCY KNOBS + SELECTION (Rust Implementer lane; `composition.rs` + `mappings.json`)

### 2.1 New `Knob` variants (additive; serde + one getter arm each)

`Knob` (`composition.rs:272`) gains THREE variants; `Knob::read` (`:292`) gains one arm each. `SubjectSize`/`FgBgContrast` already exist (S17 `:285`/`:286`).

```rust
// added to the Knob enum (after FgBgContrast):
SubjectEnergy,
ForegroundEnergy,
BackgroundEnergy,
```
```rust
// added to Knob::read's match (one arm each):
Knob::SubjectEnergy => u.subject_energy,
Knob::ForegroundEnergy => u.foreground_energy,
Knob::BackgroundEnergy => u.background_energy,
```

Serde naming follows the existing `#[serde(rename_all = "snake_case")]` on `Knob` — so the JSON keys are `subject_energy` / `foreground_energy` / `background_energy` (matching `subject_size`/`fg_bg_contrast`). These are serde/getter-only; the equivalence net never reads them.

### 2.2 The new CounterMelody-bearing `OrchestrationProfile` + the texture SelectTable rules

The shipped `texture_catalogue` (`mappings.json:137`) has `identity` + `pad_bed`. Slice 2 ADDS one profile, `pad_bed_counter`, and CONVERTS the texture SelectTable from `default: "pad_bed", rules: []` into a saliency-conditioned ladder. LOCKED rows (Music Theory Specialist authors the `layers`/`pad_voices` musical values; Rust Implementer is the SOLE writer of `mappings.json`):

```jsonc
  "texture_catalogue": [
    { "id": "identity", "layers": [], "density": 0.5, "pad_voices": 0 },
    { "id": "pad_bed",  "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.55, "pad_voices": 3 },
    // NEW S18 — same bed, but inst 2 becomes a CounterMelody (a busy foreground earns a
    // second moving line). Selected when fg is busy AND there is a real subject.
    { "id": "pad_bed_counter", "layers": ["Bass","Pad","CounterMelody","Melody"], "density": 0.6, "pad_voices": 3 }
  ],
  "texture": {
    "default": "pad_bed",
    "rules": [
      // subject → counter-melody PRESENCE: a busy foreground over a real subject earns the
      // second line. Both predicates must hold (AND). foreground_energy busy AND fg_bg_contrast
      // shows a real subject-vs-ground. First-match-wins → this beats the pad_bed default.
      { "when": [ { "knob": "foreground_energy", "op": "ge", "lo": 0.35 },
                  { "knob": "fg_bg_contrast",    "op": "ge", "lo": 0.20 } ],
        "pick": "pad_bed_counter" }
    ]
  }
```

Selection mapping (the design-s16 §3.2 throughline, now grounded): **subject → melody** (the melody layer is always present; `subject_size`/`fg_bg_contrast` already feed melody register/prominence via the existing form/character ladders — unchanged here); **foreground → counter-melody presence** (`foreground_energy` busy ⇒ `pad_bed_counter`, else the quiet-foreground `pad_bed` without a counter); **background → pad depth** (`pad_voices` rides on the profile; Slice 2 keeps it at 3 for both bed profiles — a `background_energy`/`value_key`-conditioned `pad_voices` is the documented Slice-3 finer-threshold work, NOT this slice, to keep the slice to ONE new musical mechanism). The `BackgroundEnergy`/`SubjectEnergy` knob variants ship now (Deliverable-1 fields + getters) so Slice 3's pad-depth rules are a pure JSON add with no Rust edit.

**Why the counter rule is gated on TWO predicates:** `foreground_energy` alone would turn the counter on for a busy abstract with no subject (where the bed already carries it); requiring `fg_bg_contrast ≥ 0.20` ensures a counter-line only enters when there's a real subject/ground stratification for it to weave through — the design-s16 §3.2 "fg → counter-melody presence … absent when the foreground is quiet" rule, made conjunctive so it tracks STRUCTURE not average busyness.

### 2.3 Byte-freeze + back-compat for Deliverable 2

- The equivalence net builds its plan by hand and never calls `texture.select(u)` — the new rules are unreachable on the freeze path. GREEN.
- `mappings.json` back-compat: the `texture`/`texture_catalogue` fields are already `#[serde(default)]` (S17 `:408`/`:414`), and the new `Knob` variants are additive — an OLD mappings.json with no `pad_bed_counter`/rules still parses; a rule naming a missing profile id falls back to `lookup_orchestration → identity` (the S17 path `:694`). **Optional ride-along (§7):** the S17 §11.7 back-compat test (old mappings → identity fallback) can be re-asserted here while in `mappings.json` — flag it, not required.
- A NEW `composition.rs` unit test pins the selection: a hand-built `ImageUnderstanding` with `foreground_energy: 0.5, fg_bg_contrast: 0.3` selects `pad_bed_counter`; one with `foreground_energy: 0.1` selects `pad_bed`; the default holds when neither fires. (RNG-free, no planner — directly over `SelectTable::select`.)

---

## 3. DELIVERABLE 3 — THE REAL COUNTER-MELODY (Music Theory Specialist lane; `chord_engine.rs` only)

### 3.1 What the realize branch can SEE — the decisive architectural read

`realize_step` (`:956`) is **stateless per-instrument-per-step** and its signature is FROZEN. It does NOT receive the melody's sounded pitch, nor the counter's previous pitch, nor the previous chord — and `realize_rhythm` (`:1258`) receives even less (`step: &StepPlan` + `pad_voices`, NOT `ctx`). BUT every datum the counter-line needs is **deterministically RE-COMPUTABLE from plan data already borrowed in `ctx`**, with no new cross-step state and no signature change to `realize_step`:

- **The melody's pitch THIS step** = the same value the Melody instrument computes: `theme_melody_pitch(ctx, OrchestralRole::Melody, &step.chord, features)` (pub `:1737`) → if `Some(Some(p))` the theme pitch, if `Some(None)` the melody RESTS this step, if `None` the free-select `role_pitch(Melody, &step.chord, melody_inst_idx, num, features)`. The counter arm reuses this exact path.
- **The melody's pitch / chord the PREVIOUS step** = `ctx.section.steps[ctx.step_in_section - 1]` (the section carries its own filled `StepPlan` list; `step_in_section` locates the current one). When `step_in_section == 0` there is no prior step — treat as "phrase opening, no contrary constraint yet."
- **The counter's OWN previous pitch** = recompute the counter arm's own output for `step_in_section - 1` (it is a pure function of the prior `StepPlan` + `ctx`), OR — the cheaper LOCK — seed the counter line's "previous pitch" from the prior step's chord using the same nearest-tone seating, so the line is *connected* without a recursive self-call. **LOCK: seed `prev_counter_pitch` = the counter pitch the arm WOULD pick for `steps[step_in_section-1].chord` from a neutral anchor** (one non-recursive nearest-tone pick off the prior chord), keeping the arm O(1) per step.

**THREADING (the ONE additive change to reach this):** the CounterMelody arm needs `ctx` (for `ctx.section.steps` + `ctx.step_in_section` + the melody recompute). `realize_rhythm` does not receive `ctx`. Mirror the S17 `pad_voices` precedent: thread `ctx: &StepContext` into the private `realize_rhythm` as an additive parameter (it is a `fn`, not `pub fn` — NOT a public-seam change; `realize_step`'s signature is unchanged). `realize_step` already holds `ctx` and passes `pad_voices` down — it now also passes `ctx`. Under the identity profile no instrument is ever a CounterMelody, so the new `ctx` reader in the Counter arm is inert on the freeze path. **This is a one-way dependency note for lane sequencing — see §5.**

### 3.2 The counter-line PITCH contract (LOCKED — reuse the existing craft)

For a CounterMelody step (non-cadence — the cadence early-return `:1364` still fires first, ringing a single sustained note, byte-stable):

1. **Compute the melody's move.** `m_now` = the melody pitch this step (via the §3.1 recompute); `m_prev` = the melody pitch the prior step (via `steps[step_in_section-1]`). The melody's motion direction `mel_dir ∈ {Up, Down, Hold}` = `sign(m_now - m_prev)` (Hold when equal OR when `m_now` is a rest — a rest is treated as Hold for the contrary rule, and as a "gap" for the rhythm rule §3.4).
2. **Build the counter's candidate set.** The chord-tone candidates near the counter's previous pitch: reuse `upper_voice_candidates(pc, prev_counter_pitch, MAX_UPPER_VOICE_MOTION)` (`:1936`, the same ≤P5 nearest-tone search `voice_lead_one` uses) over each pitch class of `step.chord` (skip the root pc — the counter is an inner/upper line, not a bass double), seated in the FILL/COUNTER register (between `FILL_REGISTER_FLOOR` 55 and `MELODY_REGISTER_FLOOR` 67, so it sits under the melody and above the pad bed). Dedup.
3. **Score for CONTRARY/OBLIQUE motion (the core counterpoint rule).** Prefer the candidate whose motion direction `cnt_dir = sign(cand - prev_counter_pitch)` OPPOSES `mel_dir` (contrary) or is `Hold` when the melody moves (oblique); penalize SIMILAR motion (same direction as the melody). Score (lower wins): `motion = |cand - prev_counter_pitch|` (conjunct preference, like `voice_lead_one`), PLUS a contrary-motion bonus subtracted when `cnt_dir` opposes/obliques `mel_dir`, PLUS a heavy penalty if the candidate is identical to `m_now`'s pitch class in the same octave (no unison-double of the melody).
4. **HARD-REJECT similar motion INTO a perfect fifth/octave** between melody and counter. Extend `has_parallel_perfects` to the **(melody, counter) pair across T→T+1**: build the two 2-voice "voicings" `[m_prev, prev_counter_pitch]` and `[m_now, cand]` and reject `cand` if `has_parallel_perfects(&[m_prev, prev_counter_pitch], &[m_now, cand])` is true (the existing `:1970` checker already returns true exactly when both voices move into the same perfect interval class — it works as-is on the 2-element slices; **no edit to `has_parallel_perfects` itself is needed, only a NEW call site** in the counter arm). If every candidate is rejected (rare), fall back to the nearest oblique candidate (the counter HOLDS its previous pitch if it is still a chord tone, else the nearest chord tone — never emit a parallel).
5. **Conservatism (first-species floor).** Chord tones only in the first cut (passing tones on weak beats are the design-s16 §2.3 rule-4 refinement — **deferred to keep the slice to one mechanism**; the counter sounds a chord tone every time it sounds). No suspensions/appoggiaturas.

The chosen pitch is the counter's `note` for the step; `prev_counter_pitch` for the next step is this chosen pitch (or, per the §3.1 LOCK, the non-recursive prior-chord seed — pick ONE and document it; the seed is cheaper and sufficient for a connected line).

### 3.3 The counter-line RHYTHM contract — complementary to the melody

The counter is rhythmically COMPLEMENTARY: it preferentially sounds where the melody RESTS or HOLDS, and stays out of the melody's way where the melody is active. Concretely, off the data the arm sees:

- **Melody active this step** (the melody's `realize_rhythm` figure subdivides — i.e. `edge_activity > 0.55`, the arpeggio/syncopated/dotted bands `:1492`/`:1505`/`:1514`): the counter sounds ONE sustained tone for the full step (`sustained(0, step_ms, base_frac)` with the connected-leaning `base_frac`), staying underneath without competing — it holds while the melody moves (the oblique case).
- **Melody resting or holding this step** (`m_now` is a rest `Some(None)`, OR `mel_dir == Hold`, OR low melody activity `edge_activity ≤ 0.55` so the melody sustains): the counter MOVES — it is the active voice this step, sounding its chosen contrary-motion pitch, and may place its onset OFF the downbeat (a single delayed onset at `step_ms/4`, one note, NOT an arpeggio) so it fills the melody's gap rhythmically as well as harmonically. **This single off-beat placement is the rhythmic-complementarity lever; it is NOT a comping/Alberti figure (that is Slice 3, §7) — it is one note, possibly delayed.**

### 3.4 THE HELD-PERIOD ACTIVATION RULE (the operator "empty periods" verdict — load-bearing)

The operator's residual complaint: a chord that HOLDS across several steps has "empty periods" where nothing moves underneath. The counter-line is the lever. **The rhythmic-complementarity rule of §3.3 must preferentially ACTIVATE during held-chord / melody-static stretches.** Detection, off the data the arm sees at `realize_step` time:

- **Held-chord detection.** Compare `step.chord` to the prior step's chord `ctx.section.steps[step_in_section - 1].chord`: a "held period" is when `step.chord.notes == prev.chord.notes` (the same voiced chord persists) — i.e. the harmony is static across this step boundary. (At `step_in_section == 0`, or when the prior chord differs, it is NOT a held period.)
- **Melody-static detection.** `mel_dir == Hold` (the melody repeats/sustains its pitch) OR `m_now` is a rest.

**Activation rule (LOCK):** when EITHER held-chord OR melody-static is true, the counter is in its **MOVING** mode (§3.3 "melody resting or holding" branch) AND takes a **guaranteed onset** — it does NOT rest-as-gesture even on a weak interior beat (the §3.3 rest path is suppressed during a held/static period), AND it places its onset OFF the downbeat (`step_ms/4`) so a moving counter-line audibly weaves through the held harmony. This is the direct musical answer to "empty periods": across a multi-step held chord, the melody may sustain but the counter steps to a NEW contrary-motion chord tone each step (selected fresh per §3.2 against the held chord's tones), so something always moves underneath. Because the counter picks a *different nearest chord tone* as `prev_counter_pitch` advances and contrary motion is scored against the (held or moving) melody, the held chord gets an internal moving line rather than a re-struck stab — exactly the operator's "no naive extra chord stabs" constraint.

When NEITHER held nor static is true (the chord is changing AND the melody is moving), the counter takes the §3.3 "melody active" branch (one sustained underneath) OR, on a weak interior beat, MAY rest-as-gesture (gate on `FILL_REST_ACTIVITY` like the fill, so a genuinely near-static image still gets occasional counter-rests) — the texture breathes when both other voices are already busy.

### 3.5 The arm, assembled (Music Theory owns the body; replaces the `:1474` stub)

```rust
OrchestralRole::CounterMelody => {
    // ctx is now threaded in (additive param on this private fn — §3.1). The melody
    // recompute, prior-step lookup, and contrary-motion search are all pure functions of
    // (ctx, step, features) — no new cross-step state, no realize_step signature change.
    let si = ctx.step_in_section;
    let prev = si.checked_sub(1).and_then(|p| ctx.section.steps.get(p)); // None at section start
    // melody now / prev (reuse the Melody role's pitch path):
    let m_now = melody_pitch_for(ctx, step, features);     // Option<u8>: None == melody rests
    let m_prev = prev.and_then(|p| melody_pitch_for(ctx, p, features));
    let mel_dir = direction(m_prev, m_now);                // Up / Down / Hold (rest -> Hold)
    let held_chord = prev.map_or(false, |p| p.chord.notes == step.chord.notes);
    let melody_static = mel_dir == Hold || m_now.is_none();
    // PITCH (contrary/oblique, ≤P5, no parallel perfects vs melody — §3.2):
    let prev_counter = seed_prev_counter(ctx, prev, step);  // non-recursive prior-chord seed
    let cnt = pick_counter_pitch(&step.chord, prev_counter, m_prev, m_now, mel_dir); // §3.2
    // RHYTHM (complementary + held-period activation — §3.3/§3.4):
    if held_chord || melody_static {
        // MOVING mode: guaranteed off-beat onset, no rest — fills the "empty period".
        vec![NoteEvent { note: cnt, velocity, hold_ms: hold(step_ms, base_frac),
                         offset_ms: step_ms / 4 }]
    } else if melody_active(edge_activity) {
        // OBLIQUE hold underneath the moving melody.
        vec![sustained(0, step_ms, base_frac).with_note(cnt)]
    } else {
        // both calm: may rest-as-gesture on a weak interior beat (texture breathes).
        let weak_interior = !step.position_in_phrase.is_multiple_of(2);
        if edge_activity < FILL_REST_ACTIVITY && weak_interior { Vec::new() }
        else { vec![sustained(0, step_ms, base_frac).with_note(cnt)] }
    }
}
```
(`melody_pitch_for`, `direction`, `seed_prev_counter`, `pick_counter_pitch`, `melody_active`, and a `.with_note(cnt)` helper are NEW private helpers in `chord_engine.rs`, Music-Theory-owned. `sustained`/`base_frac`/`edge_activity`/`velocity` are already in `realize_rhythm`'s scope. The `note` param the other arms thread is the counter's `base_note` anchor — the arm overrides it with `cnt`, exactly as the Pad arm overrides `note` with its seated bed tones.)

### 3.6 Tests (Test Engineer; add to a NEW `tests/counter_s18.rs`, hand-built RNG-free)

1. **Contrary/oblique motion**: build two consecutive steps where the melody moves UP (m_prev < m_now); assert the realized counter pitch moves DOWN or HOLDS relative to its seeded previous pitch (never strictly up with the melody). Symmetric for melody-down. (Pin the core counterpoint rule.)
2. **No parallel perfects**: construct a (m_prev, prev_counter) → (m_now, cand) configuration where the only "nearest" candidate would form a parallel fifth; assert the realized counter pitch is NOT that candidate (the `has_parallel_perfects` reject fired). 
3. **Chord-tone membership**: the realized counter pitch is always a chord-tone pitch class of `step.chord`, seated between FILL and MELODY registers (55 ≤ note < melody floor).
4. **Held-period activation (the operator verdict)**: two consecutive steps with the SAME voiced chord (`held_chord == true`) and a SUSTAINING melody → the counter SOUNDS on BOTH steps (no rest), with onset OFF the downbeat (`offset_ms == step_ms/4`), and the two counter pitches DIFFER (something moves underneath the held chord). This is the "empty periods" regression guard.
5. **Complementary rhythm**: when the melody is ACTIVE (high edge_activity → arpeggio), the counter sounds ONE sustained tone (len 1, offset 0); when the melody RESTS/HOLDS, the counter is the moving voice (offset == step_ms/4).
6. **Section-start guard**: at `step_in_section == 0` (no prior step) the counter still produces a valid chord-tone note (no panic, no parallel check against a missing prior).
7. **Slice-3 ceiling guard**: the counter emits AT MOST one NoteEvent per step (never an arpeggio/comping figure) — pins the Slice-2 ceiling so Slice-3 figuration is a clean future diff.

### 3.7 Byte-freeze guarantee for Deliverable 3

The CounterMelody arm is reached ONLY when `assign_role` returns `CounterMelody`, which happens ONLY under a non-identity profile naming a `CounterMelody` layer (`pad_bed_counter`) — never under the identity profile the equivalence net carries. The new `ctx` param on the private `realize_rhythm` is inert on the freeze path (`pad_voices == 0`, no Pad/Counter instrument). `realize_step` signature UNCHANGED. The cadence early-return `:1364` and the `sustained` `(frac*rit).min(1.20)` cap are UNTOUCHED. `has_parallel_perfects` itself is not edited (only a new call site). **`engine_equivalence` stays BYTE-GREEN; goldens 36/79/114/84/240 unmoved.**

---

## 4. PER-FILE BUILD SUMMARY

| File | Lane | Change | Freeze impact |
|---|---|---|---|
| `src/pure_analysis.rs` | **Rust Implementer** | `RegionStats` + `analyze_regions_pure` + `pick_subject_region`; fill the reserved saliency fields + the energy triplet in `understand_image_pure`; 2 in-module unit tests | GREEN — compose-path only; net never calls it |
| `src/composition.rs` | **Rust Implementer** | 3 new `ImageUnderstanding` fields + `neutral()` defaults; 3 new `Knob` variants + `read` arms; 1 selection unit test | GREEN — additive fields/variants, net never reads them |
| `assets/mappings.json` | **Rust Implementer** (SOLE writer; Music Theory hands the `pad_bed_counter` values) | add `pad_bed_counter` profile + the `foreground_energy ∧ fg_bg_contrast` texture rule | GREEN — `#[serde(default)]`; net builds its plan by hand |
| `src/chord_engine.rs` | **Music Theory Specialist** | replace the `:1474` CounterMelody stub with the real counter-line (§3.2–§3.5); the NEW private helpers; thread `ctx` into the private `realize_rhythm` + its read in `realize_step` | GREEN — Counter arm unreachable under identity; `realize_step` sig + cadence ring + `has_parallel_perfects` untouched |
| `tests/saliency_s18.rs` | **Test Engineer** | §1.4 net | — |
| `tests/counter_s18.rs` | **Test Engineer** | §3.6 net | — |
| `tests/texture_s17.rs` | **Test Engineer** | SUPERSEDE `test_countermelody_stub_equals_harmonicfill` `:398` — the stub is gone; replace with a "counter is no longer a HarmonicFill delegate" assertion (§6.4) | GREEN — the only S17 net touch |

**OFF-LIMITS to BOTH lanes:** `src/main.rs`, `src/modem.rs`, `src/bin/modem_*`, `src/synth_sink.rs`, `src/midi_output.rs`, `src/cli.rs`, `src/tui.rs`, `src/engine.rs` (no engine change this slice — the `ctx` the Counter arm needs is already built per-step on the compose path). The S17 Pad / rest-fix logic is TOUCH-ONLY-ADDITIVELY (the Counter arm is a sibling `match` arm; do not reshape the Pad/Fill/Bass/Melody arms).

---

## 5. LANE SEQUENCING — VERDICT: ∥ PARALLEL, with ONE non-blocking type dependency

**The two lanes run in PARALLEL.** The Music Theory lane (`chord_engine.rs`, the counter-line) depends ONLY on the S17 enum arm `OrchestralRole::CounterMelody` (already present at HEAD `:818`) and on `ctx.section.steps`/`ctx.step_in_section` (already on `StepContext` at HEAD). It does NOT depend on the saliency reader's OUTPUT: the counter-line's pitch and rhythm are pure functions of `step.chord` + the recomputed melody + the prior `StepPlan` — none of which read `subject_energy`/`foreground_energy`/`fg_bg_contrast`. The Implementer lane (`pure_analysis.rs` + `composition.rs` + `mappings.json`, the saliency reader + the knobs + the selection rule) does NOT depend on the counter-line's realization. The two mechanisms are SELECTION (Implementer: when does a CounterMelody instrument exist) vs REALIZATION (Music Theory: what does a CounterMelody instrument sound) — file-disjoint and behavior-disjoint until they meet at the already-shipped `OrchestralRole::CounterMelody`/`LayerRole::CounterMelody` bridge.

**The ONE-WAY DEPENDENCY (non-blocking, both lanes can build against it today):** the new `ImageUnderstanding` energy fields (`subject_energy`/`foreground_energy`/`background_energy`) are DEFINED by the **Implementer** lane (in `composition.rs`) and READ by the **Implementer's own** `mappings.json` rule + `Knob::read` — they are entirely inside the Implementer lane, so they do NOT cross to the Music Theory lane. There is therefore NO Implementer-first serialization required (unlike S17, where the realize branch needed a type the Implementer defined). The Music Theory lane compiles and tests against HEAD's `chord_engine.rs` + `StepContext` alone.

**The single coordination point** is integration, not authorship order: a full compose-path listen of the counter-line only happens once BOTH lands (the Implementer's `pad_bed_counter` rule must select for the Music Theory arm to ever be reached at runtime). For HEADLESS testing each lane is fully self-contained — the Music Theory `tests/counter_s18.rs` builds a `pad_bed_counter`-style profile BY HAND (the `texture_s17.rs` `pad_bed()` fixture pattern `:114`), never touching the loader, so it does not wait on the Implementer's `mappings.json`. **Build order: either lane first or both at once; merge order: either. No Implementer-first gate.**

---

## 6. BYTE-FREEZE ARGUMENT (consolidated)

| S18 change | Default-path behaviour | Net impact |
|---|---|---|
| `analyze_regions_pure`/`pick_subject_region` + filled saliency fields | the net never calls `understand_image_pure`/the planner (fixed plan by hand) | GREEN — compose-path only |
| 3 new `ImageUnderstanding` fields + `neutral()` defaults | additive fields; the net constructs no `ImageUnderstanding` | GREEN |
| 3 new `Knob` variants + `read` arms | serde/getter-only; not read at the default operating point | GREEN |
| `pad_bed_counter` profile + texture rule | `#[serde(default)]`; net builds its plan by hand, never loads them | GREEN |
| CounterMelody arm filled (replaces stub) | unreachable under identity (`assign_role` never returns `CounterMelody`); cadence ring untouched | GREEN |
| `ctx` threaded into private `realize_rhythm` | additive param on a `fn` (not `pub`); inert under identity (`pad_voices == 0`, no Counter inst); `realize_step` sig unchanged | GREEN |
| new `has_parallel_perfects` call site (no edit to the fn) | reached only in the Counter arm, never under identity | GREEN |

**No golden moves in Slice 2.** Goldens 240/114/84/36/79 UNMOVED. The cadence branch and the Pad/Fill/Bass/Melody arm bodies are byte-stable.

### 6.4 The ONE deliberate S17-net supersession

`tests/texture_s17.rs::test_countermelody_stub_equals_harmonicfill` (`:398`) asserts the CounterMelody arm is byte-equal to the HarmonicFill figure. Slice 2 REPLACES the stub with a real counter-line, so this assertion is now FALSE BY DESIGN — it is the S17 property Slice 2 consciously retires (the S17 spec §11.6 named it "so Slice 2's replacement is a clean diff"). The Test Engineer replaces it with the inverse: a `tests/counter_s18.rs` assertion that the counter-line is NO LONGER a HarmonicFill delegate (e.g. on a held-chord/static-melody step the counter onsets OFF the downbeat at `step_ms/4` where the HarmonicFill figure would onset at 0). This is a deliberate, documented test edit — NOT a freeze relaxation (the freeze is `engine_equivalence.rs`, which is untouched).

---

## 7. OUT OF SCOPE FOR SLICE 2 (explicit)

- **The fuller accompaniment-figuration system** (the designed Slice 3, design-first): Alberti bass, comping patterns, on/off-beat placement, beat-position- and style-dependent figuration. The counter-melody is ONE moving line, at most one NoteEvent per step (§3.6 test 7 pins this ceiling). Do NOT let it grow into figuration.
- **Passing tones / suspensions / appoggiaturas** in the counter-line (design-s16 §2.3 rule-4) — chord tones only this slice.
- **`pad_voices` conditioned on `background_energy`/`value_key`** (the bg → pad-depth finer thresholds) — the knobs ship now (Deliverable 1) so it is a pure JSON add in Slice 3; the rule is not written this slice.
- **Per-phrase density modulation** and the `OrchestrationProfile.density` band-wiring (still reserved schema since S17).
- **The DoG saliency-mask upgrade** — the 3×3 contrast proxy ships; the `imageproc::gaussian_blur_f32` center-surround mask into the same fields is a later refinement.
- **Any `num_instruments` widening** for more independent lines — the counter rides the existing width (`pad_bed_counter` swaps one inner instrument to a CounterMelody, like `pad_bed` swaps one to a Pad).
- **Any `main.rs` / scheduler change** — the counter is one note per step within the legato cap; no true-cross-step-sustain question arises.

---

## 8. DIVERGENCES FROM THE TWO S16 DESIGN DOCS (found against the real S17 code)

1. **`TextureProfile`/`Section.texture` → `OrchestrationProfile`/`Section.orchestration`.** The S16 docs name the per-section profile `TextureProfile`; S17 renamed it `OrchestrationProfile` (to avoid colliding with `ImageUnderstanding.texture: f32`). This spec uses the as-built names throughout.
2. **`SubjectSize`/`FgBgContrast` Knob variants already exist** (S17 `:285`/`:286`); S18 adds only the three ENERGY variants, not the subject/contrast pair the S16 engine doc §2.2 implied were new.
3. **`realize_step` already threads `ctx`; `realize_rhythm` does NOT.** The S16 engine doc §3.5 treats reaching the chord/ctx as the seam question; in the as-built tree the chord is already reachable via `step.chord` and the ONLY additive thread the counter needs is `ctx` into the private `realize_rhythm` (mirroring the S17 `pad_voices` precedent) — NOT a `realize_step` signature change.
4. **`has_parallel_perfects` works on the (melody,counter) 2-voice pair AS-IS** — the S16 musical doc §2.3 said "extend `has_parallel_perfects`"; in the as-built tree the fn already checks every voice pair across T→T+1 over arbitrary-length slices, so the (melody,counter) check is a NEW CALL SITE on 2-element slices, not an edit to the fn.
5. **The counter-line cannot observe live melody/counter state** (`realize_step` is stateless per-instrument); the as-built fix is to RE-COMPUTE the melody pitch and prior-step context deterministically from `ctx.section.steps` + `theme_melody_pitch`/`role_pitch` — a divergence from the S16 docs' implicit assumption that "both lines are known at plan time" (they are RE-DERIVABLE at realize time, which is equivalent and needs no plan-time counter-line storage). §3.1.
6. **The operator HELD-PERIOD verdict is new** (post-dates both S16 docs) and is folded into §3.4 as the counter-line's primary activation driver — the S16 §2.3 rule-3 "rhythmic complementarity" is sharpened from "fill the melody's gaps" to "fill the held-chord empty periods," the operator's actual residual complaint.

---

*Design-only. No source, test, or asset modified by this document.*
