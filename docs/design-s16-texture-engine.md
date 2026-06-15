# S16 — The Texture / Density Engine (saliency-driven layering, engineering side)

**Author role:** Rust Architect (DESIGN ONLY — no source, test, or asset modified by this document; `docs/` only).
**Date:** 2026-06-15.
**Status:** PROPOSE-FOR-ITERATION. A Music Theory Specialist is, in parallel, designing the MUSICAL texture model (what the layers *are* — melody / counter-melody / pad / bass / background — and the saliency→layer assignment). This document owns the ENGINE side: where sparsity lives in the as-built spine, the pure-Rust saliency extension to `ImageUnderstanding`, how the new layers thread additively through `plan → StepContext → realize_step` without a rewrite or a byte-freeze break, and the staging.
**Grounded against** the S15 head: `src/engine.rs`, `src/composition.rs`, `src/pure_analysis.rs`, `src/chord_engine.rs`, `tests/engine_equivalence.rs`, and the docs `assessment-composition-architecture.md` (10-stage roadmap; Stage 9 = saliency), `design-s15-variety-engine.md` (the data-driven `FormSpec`/`SelectTable`/`PlanMappings` mechanism every later stage extends), `spec-s15-slice1-build.md` (the as-built S15 contract).

---

## 0. Executive summary (read first)

The S15 output sounds **near-monophonic** — the operator hears "at least half the melody missing and ALL counterpoint, ALL harmony, ALL background gone." That is not a bug in any one function; it is **structural**: the as-built ensemble is a fixed `num_instruments` (default **4**) split by `instrument_role` into **one Bass + one Melody + the rest HarmonicFill**, and each role emits **one sustained tone per step** in the common case. There is architecturally *no* counter-melody role and *no* sustained-pad role, the default width is small, and on a real photo almost every step falls into the single-sustained-note branch of `realize_rhythm`. Density is gated at three seams: the **ensemble width** (`EngineConfig.num_instruments`), the **role taxonomy** (`OrchestralRole` has 3 arms, only 1 of which is the tune), and the **per-role realizer branch** (HarmonicFill/Bass mostly emit a single note). The cure is to make density a **planned, saliency-driven property**: extend `ImageUnderstanding` with a pure-Rust region/subject-foreground-background reading (zero new dependency, deterministic, reviving Stage 9), add **two new `OrchestralRole` arms (CounterMelody, Pad)** plus a **plan-supplied layer/voice-count controller**, and thread both into the realizer **additively** so the behaviour-neutral default `StepContext` selects none of it — keeping `tests/engine_equivalence.rs` byte-green exactly as S9/S13/S15 did. **Recommended Slice 1:** the saliency reader + a `TextureProfile` (data-driven, from `mappings.json`) that the planner attaches per-section and that *widens the realized voice set* — the biggest hearable density win (a real harmonic pad + a counter-line under the melody) for the smallest new mechanism (one image-reader + one realizer fan-out), with the cadence branch frozen. Texture/density and Stage 9 saliency are **deeply coupled and should MERGE** into one stage (S16): the saliency reading is precisely what *assigns* the texture layers.

---

## 1. Where sparsity lives architecturally — the per-step trace

Follow one step from time-cursor to sounded notes, and the thinness is over-determined at three independent seams.

### 1.1 The trace

`PipelineEngine::tick` (`engine.rs:458`) → `decide_step` (`engine.rs:526`) pulls **one `ScanBarFeatures` row of width `num_instruments`** from the `FeatureSource`, then loops `row.iter().enumerate()` calling `decide_instrument_action` **once per instrument** (`engine.rs:553`). Each call (`engine.rs:694`) projects `ScanBarFeatures → PerfFeatures`, selects `plan_steps[step_idx % plan_steps.len()]`, and calls `chord_engine::realize_step` (`chord_engine.rs:897`). Inside `realize_step`:

1. `instrument_role(inst_idx, num_instruments)` (`chord_engine.rs:863`) assigns the orchestral role.
2. `role_pitch` / `theme_melody_pitch` picks ONE base pitch.
3. `realize_rhythm` (`chord_engine.rs:1156`) emits the `Vec<NoteEvent>` — for most steps, **exactly one `NoteEvent`** per instrument.

So the total sounding density of a step is `Σ over instruments of |realize_rhythm(...)|`, and the dominant term is **the number of instruments**, not anything musical.

### 1.2 Seam A — the ensemble is small and fixed

`EngineConfig::num_instruments` defaults to **4** (`engine.rs:202`). `instrument_role` (`chord_engine.rs:863`) maps a 4-wide ensemble to **{Bass, HarmonicFill, HarmonicFill, Melody}** — one bass, **one** melodic line, two inner sustained tones. There is no plan input that *raises* the active voice count for a busy/high-contrast image: the width is a CLI/`EngineConfig` knob set once, never a function of the image or the section. A "full" texture (melody + counter-melody + pad + bass + background) needs **at least 5 differentiated lines**; the default ensemble cannot express it because it has neither the width nor the role vocabulary.

### 1.3 Seam B — the role taxonomy has no counter-melody and no pad

`OrchestralRole` (`chord_engine.rs:801`) is a **closed 3-arm enum**: `Bass`, `HarmonicFill`, `Melody`. The operator's missing layers map onto roles that **do not exist**:

- **counter-melody / counterpoint** — there is no second melodic role. Every non-Bass-non-Melody instrument is `HarmonicFill`, which by design (`chord_engine.rs:1283`) is the *least* active voice and may even **rest** (`edge < 0.15 && weak_interior → Vec::new()`, `chord_engine.rs:1290`). So additional instruments do not thicken into a counter-line; they thin toward silence.
- **sustained pad / harmony bed** — `HarmonicFill` is the closest thing, but it sounds at most one inner chord tone per step and is the role most likely to rest. There is no role whose contract is "hold the full chord underneath as a bed."
- **background** — no role at all.

`instrument_role` is a pure total function of `(inst_idx, num_instruments)` with no image/plan input, so even widening the ensemble only multiplies `HarmonicFill`, not the *kinds* of line.

### 1.4 Seam C — the per-role realizer collapses to one note

Even at the current width, `realize_rhythm` (`chord_engine.rs:1261`) is structurally sparse on real photos:

- **Bass:** one sustained root (`chord_engine.rs:1279`), two notes only in the narrow `pre_cadence` window.
- **HarmonicFill:** one sustained inner tone, **or a rest** (`chord_engine.rs:1290`).
- **Melody:** the only role that can subdivide — but the arpeggio/syncopation branches are gated on `edge_activity > 0.55/0.80` (`chord_engine.rs:1304/1317`), and a real photo's normalized per-bar `edge_activity` typically lands in the **SUSTAINED** branch (`chord_engine.rs:1335`), i.e. **one long note**.

So in the common case a 4-instrument step sounds **≈ 3 notes** (bass root + one fill tone + one melody tone, one fill possibly resting) — a chord skeleton, not a texture. Nothing in the chord harmony is lost (`generate_chords` builds full 3–5-note chords); it is the **realization** that picks one tone per instrument and the **ensemble** that has too few instruments and too few *kinds* of instrument. That is exactly "all the harmony / all the background / all the counterpoint missing": the chord exists in the plan but is never *voiced as a bed*, and the lines that would sit between bass and melody don't exist.

### 1.5 Diagnosis, stated for the build

> Sparsity is a **density-and-role** problem, not a harmony problem. Three additive levers fix it, in increasing mechanism cost: **(C-fix)** make under-populated roles emit a fuller figure (a pad that holds the chord; a fill that doesn't rest when the image wants density); **(B-fix)** add the missing roles (`CounterMelody`, `Pad`); **(A-fix)** let the *plan* raise the active voice/layer count for an image/section that wants a full texture. The image driver for all three is a **saliency reading** — subject vs. foreground vs. background — because "how many independent lines, and how prominent each" is a perceptual property of the image's *structure*, not of its averages. This is why texture density and the deferred Stage-9 saliency are one problem.

---

## 2. The `pure_analysis` SALIENCY extension — region / foreground-background, pure-Rust

The S15 `ImageUnderstanding` already **reserves** the saliency fields with whole-image defaults (`composition.rs:73`): `subject_size: 1.0`, `subject_hue == dominant_hue`, `subject_saturation == avg_saturation`, `fg_bg_contrast: 0.0`, plus `mass_centroid: (0.5, 0.5)`, `quadrant_contrast: 0.0`, `vertical_emphasis: 0.5`. S16 **fills these with real values** and adds a small **region triplet** the texture controller reads — all in `pure_analysis.rs`, all pure-Rust on the `image`/`imageproc` 0.23 crates already in the tree (`pure_analysis.rs:28`), **zero new dependency**, deterministic (no RNG, no clock), honouring the module boundary (no music type enters `pure_analysis`).

### 2.1 The heuristic (no ML), matching how `understand_image_pure` already works

`understand_image_pure` (`pure_analysis.rs:469`) today calls `analyze_global_pure` once and field-copies/clamps the whole-image scalars. S16 adds a **single extra pass** — a 3×3 (or center/border) region decomposition over the SAME `RgbImage` — and derives the subject/fg-bg knobs from **region contrast**, the cheapest defensible saliency proxy (assessment §B.2.d's "3-region center/border/detail proxy first"):

- **Regions.** Partition the image into a coarse grid (rule-of-thirds 3×3, or a center disc vs. border ring). For each region compute the cheap stats `understand_image_pure` already knows how to compute per area: mean value (luminance via `to_gray`, `pure_analysis.rs:286`), mean saturation (`hsv_means`, `pure_analysis.rs:169`), and edge energy (`edge_density_pure`, `pure_analysis.rs:310`). These reuse the exact kernels already validated against the OpenCV path; no new vision code.
- **Subject region = the most salient cell.** Saliency proxy = a weighted blend of (a) **center bias** (rule-of-thirds / center weighting — a subject is usually framed central), (b) **local contrast against the surround** (`|region_value − mean_of_neighbours|` + edge energy: a subject pops in luminance/detail), and (c) **saturation pop** (`region_saturation − border_saturation`). First-pass: `subject_region = argmax` of that blend. This is the standard center-surround intuition done with arithmetic, not a learned model.
- **Derived knobs.** `subject_size` = the salient region's area fraction (a single central blob → small; a uniform field → ~1.0, i.e. "no subject, whole-image texture"); `fg_bg_contrast` = the value/saturation/edge contrast between the subject region and the border ring (0 = flat field, high = strong subject-vs-background); `subject_hue`/`subject_saturation` = the salient region's hue/saturation (vs. the whole-image `dominant_hue`/`avg_saturation` already carried). `mass_centroid`/`vertical_emphasis`/`quadrant_contrast` fall out of the same region pass (luminance-weighted centroid; upper-vs-lower mass; variance of the region means) — these were assessment §B.2.c and are free once the regions exist.

The honest fidelity note (same discipline as `pure_analysis.rs:13`'s OpenCV-parity deltas): this is a *contrast* proxy, not segmentation — it answers "is there a prominent region and how prominent" well, and "what object is it" not at all. The optional DoG upgrade (`imageproc::gaussian_blur_f32` center-surround → a real mask) is a later refinement *into the same fields*; the proxy ships first.

### 2.2 Concrete Rust sketch (illustrative signatures)

New private region type + a region pass in `pure_analysis.rs`, and the saliency fields populated in `understand_image_pure`. No public-API change to `ImageUnderstanding`'s *shape* — the fields already exist (`composition.rs:73`); S16 changes them from defaults to computed values, plus adds the small region triplet the controller needs.

```rust
// ── src/pure_analysis.rs (NEW, private) ────────────────────────────────────
/// One region's cheap perceptual stats — the SAME kernels analyze_global_pure
/// uses, computed over a sub-rectangle. Pure-Rust; no new dependency.
struct RegionStats {
    /// Region centroid in normalized image coords (0..1, 0..1).
    center: (f32, f32),
    /// Area fraction of the whole image, 0..1.
    area_frac: f32,
    mean_value: f32,       // luminance 0..100 (to_gray mean)
    mean_saturation: f32,  // 0..100 (hsv_means)
    edge_energy: f32,      // 0..1 (edge_density_pure)
    dominant_hue: f32,     // 0..360 (hsv_means circular)
}

/// Decompose `img` into a coarse region grid (rule-of-thirds 3×3 by default) and
/// compute each region's stats. ONE extra pass over the pixels already loaded.
/// Deterministic, pure.
fn analyze_regions_pure(img: &RgbImage, grid: (u32, u32)) -> Vec<RegionStats>;

/// The center-surround saliency blend → (subject_region_index, saliency_score).
/// score = w_center*center_bias + w_contrast*local_value_edge_contrast
///        + w_sat*saturation_pop. First-match-wins argmax; ties → most-central.
/// Pure arithmetic, NO learned model (assessment §B.2.d proxy).
fn pick_subject_region(regions: &[RegionStats]) -> (usize, f32);
```

```rust
// ── ImageUnderstanding (composition.rs) — fields ALREADY present (S15 :73);
//    S16 fills them + adds the region triplet the texture controller reads.    ─
pub struct ImageUnderstanding {
    // ... existing S15 fields unchanged ...

    // Saliency, S15-reserved (composition.rs:73) — S16 fills with real values:
    //   subject_size, subject_hue, subject_saturation, fg_bg_contrast,
    //   mass_centroid, quadrant_contrast, vertical_emphasis

    // NEW S16 region triplet — region-wise energy the layer controller reads to
    // decide how many lines and how prominent. All 0..1, default whole-image:
    /// Energy in the salient (subject) region, 0..1. Default 0.0.
    pub subject_energy: f32,
    /// Energy in the foreground (non-subject central) band, 0..1. Default 0.0.
    pub foreground_energy: f32,
    /// Energy in the background (border) band, 0..1. Default 0.0.
    pub background_energy: f32,
}
```

The boundary copy in `understand_image_pure` (`pure_analysis.rs:469`) gains:

```rust
let regions = analyze_regions_pure(img, (3, 3));
let (subj_idx, _score) = pick_subject_region(&regions);
let subj = &regions[subj_idx];
// ... field-copy: subject_size = subj.area_frac; subject_hue = subj.dominant_hue;
//     subject_saturation = subj.mean_saturation;
//     fg_bg_contrast = contrast(subj, border_ring(&regions));
//     subject_energy = subj.edge_energy; foreground_energy/background_energy = banded means;
//     mass_centroid / vertical_emphasis / quadrant_contrast = region-derived.
```

**`Knob` enum coupling (S15 `composition.rs:211`).** The `SelectTable` predicate layer reads `ImageUnderstanding` through the closed `Knob` enum. `SubjectSize` and `FgBgContrast` are **already `Knob` variants** (`composition.rs:226-227`) with getter arms (`composition.rs:247-248`) — so saliency-driven *form/character* selection is already expressible. S16 adds three `Knob` variants (`SubjectEnergy`, `ForegroundEnergy`, `BackgroundEnergy`) + their getter arms only if the texture `SelectTable` needs to branch on them — one variant + one match arm each, the documented "one deliberate coupling" (`design-s15-variety-engine.md` §2.1).

---

## 3. Threading texture + saliency through plan → realize, additively

The governing question: **does voice/layer COUNT and assignment belong in the plan (`composition.rs`) or in role assignment (`chord_engine::instrument_role`)?** The answer, consistent with the S15 split, is **both, at the right altitude:**

- **The plan decides the TEXTURE PROFILE** (how many active layers, how prominent each, whether a pad sounds) — this is a *structural/perceptual* decision driven by saliency, so it lives in `composition.rs` as **data** attached per `Section`, selected by a `SelectTable` over the new saliency knobs. This is the open-content-as-data discipline (`design-s15-variety-engine.md` §1.2): adding a texture profile is a `mappings.json` row, not a Rust edit.
- **The realizer assigns and renders the layers** — `instrument_role` and `realize_rhythm` are the *mechanism*, so the new roles (`CounterMelody`, `Pad`) are **closed-enum + new realize branch** (mechanism, like `Meter`/`Character` in S15 §1.3), and the per-section/per-phrase density variation is a **new bounded scalar threaded through `StepContext`** that the existing branches read.

This mirrors S15 exactly: **open CONTENT as data (texture/orchestration profiles, saliency→layer tables), bounded new MECHANISM as code (two new roles, the saliency reader, one density controller).**

### 3.1 New roles as additive enum arms + realize branches (Seam B fix)

`OrchestralRole` (`chord_engine.rs:801`) gains two arms. This is additive: existing `match role` sites (`role_pitch` `chord_engine.rs:998`, `realize_rhythm` `chord_engine.rs:1261`, `realize_velocity` `chord_engine.rs:1105`) get **new arms**; the existing `Bass/HarmonicFill/Melody` arms are **untouched and byte-stable.**

```rust
pub enum OrchestralRole {
    Bass,
    HarmonicFill,
    Melody,
    /// NEW S16 — a second melodic line (counterpoint) under/around the Melody.
    /// Stepwise, contrary-motion-biased; the Music Theory Specialist owns its
    /// pitch-selection + voice-leading-against-melody contract.
    CounterMelody,
    /// NEW S16 — a sustained HARMONY BED: holds the full chord (multiple chord
    /// tones) across the step, the "all the harmony missing" fix. Lowest rhythmic
    /// activity, never rests, widest simultaneous note count of any single role.
    Pad,
}
```

The **byte-freeze guarantee** is that the behaviour-neutral default `StepContext` (S15 `composition.rs:489`, `single_section_default`) never *selects* these roles. `instrument_role` is the gate: today it maps width→{Bass, Fill…, Melody}. S16 makes role assignment **plan-aware** so the new roles only appear when the texture profile asks for them.

### 3.2 Role assignment becomes plan-aware (Seam A + B fix), additively

The clean additive move: `instrument_role(inst_idx, num_instruments)` stays as the **default/back-compat assigner** (byte-stable), and a NEW plan-aware assigner consults the section's texture profile. The realizer calls the plan-aware one, which **falls back to `instrument_role` when the profile is the default** (no texture profile, or the identity profile that `single_section_default` carries) — so the equivalence net's default path hits exactly today's assignment.

```rust
// ── chord_engine.rs (NEW) ──────────────────────────────────────────────────
/// Plan-aware role assignment. When `ctx`'s section carries a non-default
/// TextureProfile, it maps the instrument index onto the profile's active layer
/// set (which MAY include CounterMelody/Pad and MAY widen the melodic count);
/// otherwise it DELEGATES to the legacy `instrument_role` (byte-stable). The one
/// place the new roles enter the realizer.
pub fn assign_role(
    inst_idx: usize,
    num_instruments: usize,
    ctx: &crate::composition::StepContext,
) -> OrchestralRole;
```

`assign_role` reads `ctx.section.texture` (the new per-section profile, §3.3). Default profile ⇒ `assign_role == instrument_role`. A "full" profile ⇒ e.g. for 5 instruments {Bass, Pad, CounterMelody, HarmonicFill, Melody}. **`num_instruments` flows unchanged** through `decide_step → decide_instrument_action → realize_step → assign_role`; the plan can ALSO request a wider realized voice set than `num_instruments` (Seam A) by having the realizer emit **multiple `NoteEvent`s for the `Pad` role** (a held chord = several simultaneous notes from one instrument) — so density rises **without** forcing the operator to raise the CLI instrument count, though raising it remains the cleaner path for truly independent lines.

### 3.3 The `TextureProfile` — open content as data, attached per `Section`

A texture profile is **structure, not music content**: which layers are active, each layer's prominence, the pad's density. It is data in `mappings.json` (like `FormSpec`), selected by a `SelectTable` over the saliency knobs, and attached to each `Section` by the planner. This is the S15 `composition.rs` content-as-data pattern applied to texture.

```rust
// ── composition.rs (NEW serde struct, loaded from mappings.json) ────────────
/// One named texture/orchestration profile — pure structure, no note content.
/// The planner attaches one per Section (saliency-selected); the realizer's
/// assign_role/realize_rhythm read it. Adding a profile is a JSON row, not a
/// Rust edit (the FormSpec discipline, design-s15 §1.2).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct TextureProfile {
    /// Stable id, e.g. "sparse" / "duet" / "full" / "pad_and_tune".
    pub id: String,
    /// Which roles sound, in inst-index order; the realizer maps instruments onto
    /// this list. Closed ThematicRole-style enum keeps it type-safe (serde rejects
    /// an unknown role). Default profile == today's {Bass, Fill.., Melody}.
    pub layers: Vec<LayerRole>,
    /// 0..1 density bias the realizer's existing edge_activity bands shift by
    /// (raises onset count / un-rests the fill). Default 0.5 == no-op.
    pub density: f32,
    /// How many chord tones the Pad holds simultaneously (1..=5). 0 == no pad.
    pub pad_voices: u8,
}

/// The layer vocabulary — closed (mechanism), mirrors OrchestralRole. serde-safe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LayerRole { Bass, HarmonicFill, Melody, CounterMelody, Pad }
```

`Section` (`composition.rs:399`) gains one field:

```rust
pub struct Section {
    // ... existing S15 fields unchanged ...
    /// NEW S16 — the saliency-selected texture profile for this section. Default
    /// (single_section_default / legacy_default_section) carries the IDENTITY
    /// profile == today's role split, so the realizer is byte-stable under it.
    pub texture: TextureProfile,
}
```

`StepContext` (`composition.rs:477`) needs **no new field** — it already borrows `&'a Section`, so `ctx.section.texture` is reachable zero-copy. This is the cleanest possible thread: the borrow is already in place (S15 operator-decision-6 BORROWED `StepContext`), and `texture` rides on the section the realizer already has.

`PlanMappings` (`composition.rs:330`) gains one `SelectTable` axis (`texture`) + one catalogue (`texture_catalogue: Vec<TextureProfile>`), exactly parallel to `form` + `form_catalogue`. The planner's `plan()` (`composition.rs:538`) selects a texture id per section the same way it selects the form — `self.plan_mappings.texture.select(u)` over the saliency knobs — and attaches the looked-up `TextureProfile` to each `Section`. Saliency drives it because the `SelectTable` predicates read `SubjectSize`/`FgBgContrast`/`SubjectEnergy` etc.

### 3.4 Where per-section / per-phrase density variation lives

- **Per-section** density is the `TextureProfile.density` scalar on the `Section` (saliency-selected). It feeds the existing `realize_rhythm` `edge_activity` band logic as an **additive bias** (e.g. a high-density profile shifts the SUSTAINED→DOTTED→SYNCOPATED→ARPEGGIO thresholds down so more steps subdivide, and disables the HarmonicFill rest). This re-uses the S13 band mechanism (`chord_engine.rs:1304-1340`) rather than adding a new one.
- **Per-phrase** density is already available structurally: `realize_rhythm` reads `step.position_in_phrase`/`phrase_len`/`pre_cadence` (`chord_engine.rs:1179`), so a profile's density bias can be modulated by phrase position (denser mid-phrase, thinning into the cadence) **without a new field** — the phrase plan the realizer already receives carries it.

The density controller is therefore **one bounded scalar** (`Section.texture.density`) read by the *existing* band logic, plus the pad's `pad_voices` count — minimal new mechanism, maximal reuse.

### 3.5 The Pad realize branch (Seam C fix) — the "all the harmony missing" cure

The single highest-value realizer addition: a `Pad` arm in `realize_rhythm` that emits **multiple simultaneous `NoteEvent`s** — the chord's tones held across the whole step at low velocity in the inner register — instead of one tone. This is the bed that makes the harmony *audible as harmony*. Sketch:

```rust
// in realize_rhythm's `match role` (chord_engine.rs:1261), a NEW arm:
OrchestralRole::Pad => {
    // Hold the chord as a sustained bed: pad_voices chord tones, all at offset 0,
    // held the full step (legato), low velocity so it supports not competes.
    // Multiple NoteEvents from one instrument == the simultaneous harmony layer.
    chord.notes.iter().take(pad_voices).map(|&n| sustained_note(n, ...)).collect()
}
OrchestralRole::CounterMelody => {
    // A second melodic line: stepwise, contrary-motion-biased against the Melody.
    // PITCH selection is the Music Theory Specialist's (their voice-leading-against-
    // melody contract); the ENGINE provides the role, the branch, and the seam.
    // Rhythmically it leans the Melody's dotted/sustained figures, offset to fill
    // the melody's rests (the counterpoint intuition).
}
```

Note the realizer signature `realize_step(step, inst_idx, num_instruments, &PerfFeatures, ms_per_step, ctx)` (`chord_engine.rs:897`) is **unchanged** — `Pad`'s `pad_voices` and the chord are already reachable via `ctx.section.texture` and `step.chord`. **No seam change**, purely a new `match` arm. This is the same additive discipline the S15 theme seam used (`theme_melody_pitch` added a branch, not a parameter).

### 3.6 Data-flow diagram

```
                        ┌─────────────────────────────────────────────┐
  RgbImage ──► understand_image_pure (pure_analysis.rs:469)            │
              │   analyze_global_pure  (existing whole-image scalars)  │
              │   analyze_regions_pure (NEW §2.2)  ──► RegionStats[]    │
              │   pick_subject_region  (NEW §2.2)  ──► subject idx      │
              └──── field-copy ─────────────────────────────────────────┘
                        │  (image-free boundary, no Mat crosses)
                        ▼
        ImageUnderstanding  { ... subject_size, fg_bg_contrast,
                              subject_energy/foreground/background (NEW) }
                        │
                        ▼
        CompositionPlanner::plan (composition.rs:538)
            form    = form.select(u)        (S15)
            texture = texture.select(u)  ◄── NEW: saliency-driven SelectTable
            per Section: attach TextureProfile (NEW field)   [open CONTENT = data]
                        │
                        ▼
        CompositionPlan ──► PipelineEngine::set_plan / compose_from_image (engine.rs)
                        │
                        ▼  tick → decide_step → decide_instrument_action (UNCHANGED seam)
        StepContext { section: &Section{ texture, .. }, .. }   (borrow already present)
                        │
                        ▼
        realize_step (chord_engine.rs:897, signature UNCHANGED)
            assign_role(inst_idx, num, ctx)  ◄── NEW: default-delegates to
                                                  instrument_role; profile ⇒ Pad/CounterMelody
            realize_rhythm: NEW Pad/CounterMelody arms; density bias on existing bands
                        │
                        ▼
        Vec<NoteEvent>  — now a TEXTURE (bass + pad-bed + counter + fill + melody)
                          [bounded new MECHANISM = code]

  DEFAULT StepContext (single_section_default) ⇒ texture == IDENTITY profile
     ⇒ assign_role == instrument_role, no Pad/CounterMelody, density 0.5 no-op
     ⇒ realize_step BYTE-IDENTICAL  ⇒  tests/engine_equivalence.rs GREEN
```

---

## 4. Byte-freeze strategy

The discipline is identical to S9/S13/S15: **every new parameter/field is behaviour-neutral at its default; the legacy single-line operating point is reproduced exactly; only one deliberate, hand-re-derived golden move per slice, if any.**

### 4.1 What keeps `tests/engine_equivalence.rs` byte-green

The net pins `decide_instrument_action` on a **fixed `&[StepPlan]`** with a `single_section_default` `StepContext` (S15 `composition.rs:489`), asserting the goldens (P3 modulo wrap, `G_BASS_NOTE=36`/`G_MELODY_NOTE=79`, cadence vel `114`/`84`, cadence hold `240 ms`). Each S16 change is neutral under that default:

| S16 change | Default-path behaviour | Net impact |
|---|---|---|
| `OrchestralRole::{CounterMelody, Pad}` (new arms) | `instrument_role` (unchanged) never returns them; `assign_role` delegates to it under the identity profile | GREEN — new arms unreachable at default |
| `assign_role(inst, num, ctx)` | identity `TextureProfile` ⇒ returns exactly `instrument_role(inst, num)` | GREEN — byte-identical role |
| `Section.texture: TextureProfile` | `legacy_default_section` (`engine.rs:745`) + `single_section_default` carry the **IDENTITY profile** (layers = today's split, density 0.5, pad_voices 0) | GREEN — additive field, no realizer effect |
| `realize_rhythm` density bias | identity profile density 0.5 ⇒ the bias term is the no-op center (the existing thresholds unchanged) | GREEN |
| `ImageUnderstanding` saliency fields filled | the net does NOT call `understand_image_pure`/the planner (it builds a fixed plan); saliency only affects the *compose* path | GREEN — same boundary as S13's `set_features_global`/RNG isolation |
| new `Knob`/`LayerRole` variants | serde-only; not read at the default operating point | GREEN |

The one test-file touch (if `assign_role` becomes the call site) is **adding the `ctx`-derived role**, which under the default ctx equals `instrument_role` — an argument plumb, **never an assert relaxation** (S15 §3.2 discipline: "it adds an argument, it does not relax an assert").

### 4.2 Which slices DELIBERATELY move a golden, and how

The texture spine itself moves **no** golden — it is purely additive behind the identity profile. A golden moves **only** if a slice changes the *default* realization, which S16 deliberately avoids. The one candidate, if the operator wants the default texture to be fuller (not just the saliency-selected one), would be making the **identity profile non-identity** — e.g. giving the default 4-ensemble a Pad. That is a **deliberate golden re-derivation**: the new pad NoteEvents change the default step's note set, so the affected `engine_equivalence` goldens must be **hand-re-derived from the new documented formula in the same commit, with a comment citing this section, cadence branch left byte-stable** (S13 §7 discipline). **Recommendation: do NOT do this in Slice 1** — ship the new texture behind the saliency-selected profile only, keep the default identity, keep the net green with zero golden moves. Promote the default to a fuller texture later as its own consciously-golden-moving slice if the operator wants every image (not just high-saliency ones) thicker.

---

## 5. Staging — byte-freeze-safe, one-per-session slices

The engagement builds **one stage per session**. Sequenced to climb out of sparsity fastest, each slice builds + tests headless + is hearable, `engine_equivalence` stays byte-green throughout.

### 5.1 Reconciliation with the 10-stage roadmap — MERGE texture into Stage 9

The assessment's Stage 9 is *"region-saliency upgrade … melody-vs-accompaniment color split + theme prominence from the actual subject."* The operator's texture/density requirement is *"drive the layers by a subject/foreground/background reading."* **These are the same stage:** the saliency reading is precisely the image input that *assigns the texture layers*. Sequencing them apart would build the saliency reader twice (once for the color split, once for the layer count) or build the layers blind. **Recommendation: MERGE — S16 *is* the realized Stage 9, pulled forward**, because the operator's verdict makes density the top priority, ahead of the roadmap's Stage 3 (meter) / Stage 4 (character). The roadmap's earlier stages (meter, character) remain valid and unblocked; S16 is the saliency+texture stage promoted to next, and it *subsumes* the deferred Stage 9 saliency reader. Later stages (variation techniques, semantic tier) are unaffected and ride on the now-richer texture.

### 5.2 The slice order

1. **Slice 1 — SALIENCY READER + Pad bed (the biggest hearable density win, smallest mechanism).** See §6.
2. **Slice 2 — CounterMelody role.** Add the `CounterMelody` realize branch + the Music Theory Specialist's pitch/voice-leading-against-melody contract; texture profiles that include it become audible as a second line. (Slice 1 ships the *role enum arm* and the profile *schema*; Slice 2 fills the counter-line's *realization*.)
3. **Slice 3 — saliency-driven texture SELECTION breadth + per-phrase density modulation.** Expand the `texture` `SelectTable` (more profiles, finer saliency thresholds) and wire the density bias to phrase position. Pure data + the existing band logic; no new role.
4. **Slice 4 (optional, deliberate golden move) — promote the DEFAULT texture.** If the operator wants *every* image thicker (not just high-saliency), make the identity profile a fuller default and hand-re-derive the moved goldens. Its own slice precisely because it moves the byte-freeze.
5. **Slice 5 (optional) — DoG saliency upgrade.** Replace the region proxy with a true center-surround mask (`imageproc::gaussian_blur_f32`) populating the same fields more accurately. Pure refinement; the texture controller is unchanged.

### 5.3 Per-slice net

| Slice | Files touched | New DATA vs new MECHANISM | Test net | Independence |
|---|---|---|---|---|
| **1** | `pure_analysis.rs` (region pass, fill saliency), `composition.rs` (`TextureProfile`/`LayerRole`/`Section.texture`/`texture` SelectTable + catalogue, attach per section), `chord_engine.rs` (`OrchestralRole::Pad`+`CounterMelody` arms, `assign_role`, Pad realize branch), `assets/mappings.json` (texture catalogue + texture SelectTable + Ballad-default identity profile), `tests/engine_equivalence.rs` (plumb default `ctx` role only) | **DATA:** texture profiles + saliency SelectTable rows. **MECHANISM:** region reader, 2 role arms, `assign_role`, Pad branch, density-bias term. | `engine_equivalence` GREEN (identity profile); NEW property tests: (a) distinct-saliency images get different `subject_size`/`fg_bg_contrast` (the region reader has spread across the 6 in-repo images); (b) a "full" profile step realizes **strictly more simultaneous NoteEvents** than the identity profile (density actually rises); (c) the Pad arm emits `pad_voices` chord tones, all chord members, at offset 0; (d) `assign_role` under default ctx == `instrument_role` for all (inst, num) (the freeze witness). | Self-contained: ships the spine + the Pad bed + the saliency reader; CounterMelody is *enum-present but realize-stub* (returns the HarmonicFill figure until Slice 2), so the role exists without forcing the counterpoint craft into Slice 1. |
| **2** | `chord_engine.rs` (CounterMelody realize + pitch contract), tests | MECHANISM: counter-line realization | property: counter-line is stepwise, contrary-motion-biased vs melody, fills melody rests | needs Slice 1's role arm |
| **3** | `composition.rs`/`mappings.json` (more profiles, finer thresholds), `chord_engine.rs` (phrase-position density) | mostly DATA + one density-modulation term | property: per-phrase density varies; saliency selects ≥3 distinct profiles across the 6 images | needs Slice 1 |
| **4** | `chord_engine.rs`/`mappings.json` (default profile), `tests/engine_equivalence.rs` (re-derived goldens) | DATA (default profile) + deliberate golden move | re-derived non-default goldens, hand-commented; cadence branch frozen | independent; deliberately moves the freeze |
| **5** | `pure_analysis.rs` (DoG mask) | MECHANISM: DoG saliency | parity: DoG fields track the proxy direction, sharper | needs Slice 1's fields |

---

## 6. Recommended Slice 1

**Build, as one landed unit, the pure-Rust saliency reader + a saliency-selected `TextureProfile` that adds a real HARMONY-BED (`Pad`) and widens the active layer set — the biggest hearable density win for the smallest new mechanism, with the cadence branch and the default operating point byte-frozen.**

- **Files.** `pure_analysis.rs`: `analyze_regions_pure` + `pick_subject_region` (§2.2), fill the S15-reserved saliency fields + add the `subject_energy`/`foreground_energy`/`background_energy` triplet in `understand_image_pure`. `composition.rs`: `TextureProfile`/`LayerRole` serde structs, `Section.texture` field, a `texture` `SelectTable` + `texture_catalogue` in `PlanMappings`, attach a profile per section in `plan()`, and the identity profile on `legacy_default_section`/`single_section_default` paths. `chord_engine.rs`: `OrchestralRole::{Pad, CounterMelody}` arms, `assign_role(inst, num, ctx)` (default-delegates to `instrument_role`), the `Pad` realize branch (multi-note held chord bed), the per-section density bias on the existing `realize_rhythm` bands; `CounterMelody` arm present but realize-stubbed to the HarmonicFill figure (Slice 2 fills it). `assets/mappings.json`: the texture catalogue, the texture SelectTable, the Ballad-default identity profile. `engine.rs`: thread `ctx` into the role-assignment call (already in place via `StepContext`). `tests/engine_equivalence.rs`: plumb the default-ctx role (argument only, no assert change). **Zero new dependency.**

- **Mechanism (bounded).** One image-side region reader; two new role enum arms; one plan-aware `assign_role`; one new `Pad` realize branch; one density-bias term on the existing bands. Everything else is **data** (profiles + saliency selection rows in `mappings.json`).

- **Tests.** `engine_equivalence` GREEN under the identity profile (no golden moves). New property net: saliency spread across the 6 in-repo images; a full-profile step realizes strictly more simultaneous notes than identity; the Pad arm holds `pad_voices` chord tones at offset 0; `assign_role(default ctx) == instrument_role` for all `(inst, num)` (the freeze witness).

- **Byte-freeze plan.** Every change is neutral under `single_section_default`'s identity profile (§4.1). **No golden moves in Slice 1** — the new texture is reachable only through the saliency-selected profile on the compose path, which the equivalence net never exercises (the same boundary S13 used for `set_features_global` and the `pick_progression` RNG). The cadence branch (`is_cadence` early return + the `sustained` `(frac*rit).min(1.20)` 240 ms ring) is untouched. Promoting the *default* texture to fuller — the only change that would move a golden — is deliberately deferred to Slice 4.

- **Becomes hearable.** For the first time the operator hears, on a high-saliency image, a **sustained harmonic bed under the tune** (the chord *as harmony*, not a skeleton) and a **wider active texture** that tracks the subject's prominence — directly attacking "ALL of the harmony and ALL of the background missing," with the counter-line landing in Slice 2 and zero new dependency.

---

*Design-only. No source, test, or asset modified by this document. Illustrative Rust signatures are non-binding; the Music Theory Specialist owns the layers' musical contracts (counter-line voice leading, pad voicing, saliency→layer musical mapping) and the Implementer owns the final signatures.*
