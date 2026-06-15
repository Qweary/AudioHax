# S15 Slice 1 — The Buildable Per-File Spec

**Author role:** Rust Architect (DESIGN ONLY — no source/test/asset modified by this document; `docs/` only).
**Date:** 2026-06-15.
**Status:** LOCKED for build. This is the single contract the two implementers (Rust Implementer + Music Theory Specialist) build against. The operator's seven confirmed decisions (below) are settled — this doc encodes them; it does not re-litigate them.
**Grounded against** the S13 head working tree: `src/engine.rs` (PipelineEngine struct fields ~`:270`, `tick` `:405`, `decide_step` `:472`, `decide_instrument_action` `:562`, `set_features_global` `:328`, `GlobalFeatures` `:33`, `ScanBarFeatures` `:59`), `src/chord_engine.rs` (`StepPlan` `:489`, `plan_phrases` `:631`, `realize_step` `:891`, `PhrasePosition` `:469`, `OrchestralRole` `:795`, `PerfFeatures` `:815`, `NoteEvent` `:833`, `Chord` `:30`, the articulation clamp `:1156`–`:1164`), `src/pure_analysis.rs` (`analyze_global_pure` `:423`, the dead features), `tests/engine_equivalence.rs` (the 6-arg byte-freeze net).

This spec supersedes the structural shells in `assessment-composition-architecture.md` §C and the two parallel S15 design docs where they conflict; it MERGES them into one buildable form. Where it says **MODIFIED-from-assessment** it points at the §C signature it changes.

---

## 0. The operator's confirmed decisions (settled — build to these)

1. **Form-as-data.** Form catalog is data rows (`FormSpec`) in `mappings.json`, added without recompile. `CompositionPlan.form: String` (the selected `FormSpec.id`). `ThematicRole`/`ThemeVariation`/`CadenceStrength` stay **closed enums inside the rows**.
2. **Slice 1 ships the multi-form catalog**: rounded-binary (default), ternary ABA, AABA, ABAC, ABBAC, theme-and-variations-as-section-list — selected by image balance/symmetry/complexity condition ladders. All 4/4 / home-key / Ballad.
3. **Character = Ballad-only** in slice 1. The other Tier-1 characters + meter/modulation/climax/variation-techniques/saliency defer to their roadmap stages; their `mappings.json` tables ship now as schema, default-pinned.
4. **Motif encoding = contour-from-8-archetypes**, RESOLVED to the engine's existing degree+duration `MotifNote` at plan-build time. `MotifNote` does NOT change shape — see §1.5, locked.
5. **Multi-return forms (rondo/AABA) gated behind episode-contrast** — slice 1 does not ship raw rondo; AABA ships because its single bridge is contrast enough, and ABBAC's doubled-B is the contrast. (See §5 catalog notes.)
6. **`StepContext` borrowed** (`StepContext<'a>`) with the small `tick` restructure (resolve section/phrase before the mutable step advance).
7. **Selection = per-axis `SelectTable { default, rules }`, first-match-wins** predicate ladders (bounded `Knob × CmpOp × value`, AND-of-predicates — NOT an expression DSL), deterministic given `(understanding, mappings.json)`.

---

## 1. LOCKED TYPE DEFINITIONS

All NEW types live in a **new module `src/composition.rs`** (pure-Rust, `--no-default-features`-clean, NO image types, NO pixel math — reads perceptual scalars and emits structure) unless marked otherwise. `engine.rs` re-exports them (`pub use composition::*`) so call sites and the equivalence test import from `audiohax::engine` exactly as today. `chord_engine.rs` types (`StepPlan`/`PhrasePosition`/`OrchestralRole`/`PerfFeatures`/`NoteEvent`/`Chord`) are unchanged and re-used.

### 1.1 `ImageUnderstanding` — the planner's input (NEW; in `composition.rs`)

Image-free mirror, same discipline as `engine::GlobalFeatures`. Populated by a producer in `pure_analysis.rs` at the boundary (field-copy, no `Mat`). Slice 1 only **reads** the subset the form/theme ladders need; the rest are present (so later stages fill values, not types) and default to the whole-image / sentinel value.

```rust
/// Whole-image perceptual understanding — the COMPOSER'S input. Computed once per image,
/// whole-image, all plain values. NEW. Image-free (no `Mat`, no pixel type).
#[derive(Debug, Clone, PartialEq)]
pub struct ImageUnderstanding {
    // ── Energy (0..1; the dead S13 features re-exposed via pure_analysis) ──
    pub edge_activity: f32,        // clamp(global.edge_density / 0.05, 0, 1)   [EDGE_ACTIVITY_RANGE_MAX]
    pub texture: f32,              // clamp(global.texture_laplacian_var / 2000, 0, 1)
    pub complexity: f32,           // clamp(global.shape_complexity / 2, 0, 1)
    // ── Palette ──
    pub dominant_hue: f32,         // 0..360 — slice 1: == global.avg_hue (argmax upgrade is Stage 8)
    pub dominant_hue_mass: f32,    // slice 1 default 1.0
    pub secondary_hue: f32,        // slice 1 default == dominant_hue
    pub palette_bimodality: f32,   // 0..1 — slice 1 default 0.0
    pub colorfulness: f32,         // == global.hue_spread
    pub value_key: f32,            // 0..1 toward dark — slice 1: clamp(1 - avg_brightness/100, 0, 1)
    pub avg_brightness: f32,       // 0..100 (mirror of global)
    pub avg_saturation: f32,       // 0..100 (mirror of global)
    // ── Composition balance ──
    pub mass_centroid: (f32, f32), // slice 1 default (0.5, 0.5)
    pub quadrant_contrast: f32,    // 0..1 — slice 1 default 0.0
    pub aspect_ratio: f32,         // == global.aspect_ratio (w/h)
    pub vertical_emphasis: f32,    // 0..1 (upper-mass) — slice 1 default 0.5
    // ── Subject / region-saliency (defaults = whole-image; saliency is Stage 9) ──
    pub subject_size: f32,         // slice 1 default 1.0
    pub subject_hue: f32,          // slice 1 default == dominant_hue
    pub subject_saturation: f32,   // slice 1 default == avg_saturation
    pub fg_bg_contrast: f32,       // slice 1 default 0.0
}
```

**`pure_analysis.rs` producer (NEW fn — Rust Implementer owns).** Add alongside `analyze_global_pure`:

```rust
/// Build the whole-image `ImageUnderstanding` from the same RGB image the
/// `GlobalFeatures` producer reads. Slice 1: derives the four energy knobs from the
/// (currently dead) S13 features + the cheap palette/balance defaults. NO music logic.
pub fn understand_image_pure(img: &RgbImage) -> Result<ImageUnderstanding, AnalysisError>;
```

The clamp formulas (assessment §C.2, restated as the LOCKED mapping — dead-feature → field):

| `ImageUnderstanding` field | source | formula |
|---|---|---|
| `edge_activity` | `global.edge_density` | `clamp(edge_density / 0.05, 0.0, 1.0)` (`0.05` == `chord_engine::EDGE_ACTIVITY_RANGE_MAX`; keep in sync) |
| `texture` | `global.texture_laplacian_var` (dead) | `clamp(var / 2000.0, 0.0, 1.0)` |
| `complexity` | `global.shape_complexity` (dead) | `clamp(shape / 2.0, 0.0, 1.0)` |
| `aspect_ratio` | `global.aspect_ratio` (dead) | passthrough (`w/h`) |
| `colorfulness` | `global.hue_spread` | passthrough |
| `value_key` | `global.avg_brightness` | `clamp(1.0 - avg_brightness/100.0, 0.0, 1.0)` |
| `dominant_hue` | `global.avg_hue` | passthrough (argmax upgrade deferred to Stage 8) |

All other fields take the slice-1 default in the table comments above. **The planner MUST treat a default/sentinel field as "condition not met"** so a ladder rule reading a not-yet-extracted knob simply falls through to the axis default.

### 1.2 `CompositionPlan` / `Section` / `KeyTempoPlan` (NEW; `composition.rs`) — MODIFIED-from-assessment §C.3

```rust
/// The up-front architectural plan for one piece — computed ONCE by `CompositionPlanner`
/// from an `ImageUnderstanding`, then DRIVES per-step realization. NEW.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionPlan {
    pub form: String,              // MODIFIED §C.3: the selected FormSpec.id (was `form: Form` enum)
    pub character: Character,      // closed enum — slice 1 always Character::Ballad
    pub meter: Meter,              // closed enum — slice 1 always Meter::Four4
    pub key_tempo: KeyTempoPlan,
    pub sections: Vec<Section>,    // the EXPANDED, concrete ordered sections — THIS IS THE PIECE
    pub themes: Vec<ThemeSeed>,    // returning theme(s); a section with theme:None is valid
    pub total_steps: usize,        // == sum of section.step_len; the time cursor's N
}

/// One section — a span of steps with a local identity and a theme ref. The unit the
/// time cursor walks; the per-step realizer is parameterized by the CURRENT section. NEW.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub label: String,                 // "A" / "B" / "A'" — carried to the snapshot/observer
    pub step_len: usize,
    pub thematic_role: ThematicRole,
    pub key_offset_semitones: i8,      // slice 1: ALWAYS 0 (home key) — modulation is Stage 5
    pub ms_per_step: u64,              // slice 1: == key_tempo.base_ms_per_step (section-stable)
    pub mode: String,                  // slice 1: == key_tempo.home_mode (no modal plan yet)
    pub progression: Vec<String>,      // Roman numerals for this section (filled by chord_engine)
    pub theme: Option<usize>,          // index into themes[] this section states/recalls, or None
    pub variation: ThemeVariation,     // slice 1: Identity or Fragmented only
    pub boundary_cadence: CadenceStrength,
    pub density: f32,                  // local density bias, 0..1; slice 1 default 0.5 (no-op)
    /// The section's own FILLED phrase plan (chord_engine output). NOT in §C.3 — this is
    /// where the per-section StepPlans live so the realizer reads the section's own steps,
    /// never `plan[step_idx % len]`. See §3.
    pub steps: Vec<StepPlan>,
}

/// The piece's structural key + tempo SPINE — computed once, section-stable. NEW.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyTempoPlan {
    pub home_root_midi: u8,        // tonal home (from dominant-hue lookup; seeds, then offsets apply)
    pub home_mode: String,
    pub base_ms_per_step: u64,     // base tempo (brightness→BPM, clamped by character window)
    pub key_scheme: Vec<i8>,       // section_index → key_offset; slice 1: ALL ZEROS
    pub tempo_scheme: Vec<u64>,    // section_index → ms_per_step; slice 1: all == base_ms_per_step
}
```

### 1.3 `ThemeSeed` / `MotifNote` (NEW; `composition.rs`) — `MotifNote` UNCHANGED, locked

```rust
/// A returning-theme seed (§A.6). The motif is KEY-RELATIVE (degree+duration) so a section
/// could transpose it by key_offset (slice 1 stays home, so it never does). NEW.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeSeed {
    pub id: usize,
    /// The EXPANDED concrete motif the realizer reads — degree+duration, key-relative.
    /// Produced at PLAN-BUILD time by the contour-archetype resolver (§1.5). The archetype
    /// is NOT stored on the seed in slice 1 — resolution is one-way at build (see §1.5/§2).
    pub motif: Vec<MotifNote>,
}

/// One motif note — scale/key-relative so it transposes cleanly. UNCHANGED from assessment
/// §C.3. THE CONTOUR-ARCHETYPE IS RESOLVED INTO THIS at plan-build time (operator decision 4
/// — locked: MotifNote keeps its degree+duration shape; the 8-archetype encoding is a
/// build-time INPUT to a resolver, never a runtime type the realizer reads).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotifNote {
    pub degree: i8,       // scale degree relative to the section tonic (0 == tonic)
    pub dur_steps: u8,    // duration in steps (>=1)
}
```

### 1.4 The serde mapping structs (NEW; `composition.rs`) — load from `mappings.json`

```rust
/// One section's role in a FORM TEMPLATE — pure structure, no music content. The planner
/// expands these into concrete `Section`s. NEW. Loaded from mappings.json.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SectionTemplate {
    pub label: String,                  // "A" / "B" / "A'" / "T" / "V1" …
    pub role: ThematicRole,             // closed enum (serde rejects unknown variant)
    pub rel_len: f32,                   // relative weight; scaled to fill total_steps
    pub theme: Option<usize>,           // which theme slot this section states/recalls
    pub variation: ThemeVariation,      // slice 1 set: {Identity, Fragmented}
    pub boundary_cadence: CadenceStrength,
}

/// A FORM = an ordered section-template list + a stable id handle. THE FORM VOCABULARY
/// LIVES HERE, IN mappings.json. Adding a form is a JSON row, not a Rust enum edit. NEW.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct FormSpec {
    pub id: String,                     // "rounded_binary" / "ternary_aba" / "aaba" / …
    pub sections: Vec<SectionTemplate>,
}

/// Curated plan-selection tables over the ImageUnderstanding knobs. Loaded from
/// mappings.json. Each axis: a default id + ordered first-match-wins rules. NEW.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct PlanMappings {
    pub form: SelectTable,            // → a FormSpec.id from form_catalogue
    pub character: SelectTable,       // → a Character variant name; slice 1 pinned "ballad"
    pub meter: SelectTable,           // → a Meter name; slice 1 pinned "four4"
    pub key_scheme: SelectTable,      // → a key-scheme id; slice 1 pinned "home_only"
    pub theme_behaviour: SelectTable, // → "absent" | "fragment" | "second_theme"
    pub form_catalogue: Vec<FormSpec>,
    // character_overlays / key_schemes ship as schema later; slice 1 omits or default-pins.
}

/// One axis's "default + ordered conditional departures." `pick`/`default` are string ids. NEW.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SelectTable {
    pub default: String,
    pub rules: Vec<SelectRule>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct SelectRule {
    pub when: Vec<Predicate>,   // ALL must hold (AND); rules tried in order, first match wins
    pub pick: String,
}

/// A single threshold/range test over one ImageUnderstanding knob. Closed op set — NOT an
/// expression language. NEW.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Predicate {
    pub knob: Knob,
    pub op: CmpOp,              // Lt | Le | Gt | Ge | InRange
    pub lo: f32,
    pub hi: f32,               // used only by InRange (lo..=hi)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CmpOp { Lt, Le, Gt, Ge, InRange }

/// Closed handle naming a selectable ImageUnderstanding knob. New knob → enum variant + a
/// getter arm in `Knob::read(&ImageUnderstanding) -> f32`. serde rejects unknown variants. NEW.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Knob {
    EdgeActivity, Texture, Complexity, Colorfulness, ValueKey, AvgBrightness, AvgSaturation,
    DominantHue, PaletteBimodality, QuadrantContrast, VerticalEmphasis, AspectRatio,
    SubjectSize, FgBgContrast,
}
```

### 1.5 `StepContext<'a>` (NEW; `composition.rs`) + the closed enums

```rust
/// The plan-relative context for one scan step — WHICH section, its theme/key/tempo, and
/// the step's offset within the section. Threaded into the realizer so realization is DRIVEN
/// BY the plan. BORROWED (zero-copy) — operator decision 6. NEW.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepContext<'a> {
    pub section: &'a Section,
    pub step_in_section: usize,
    pub theme: Option<&'a ThemeSeed>,   // resolved from section.theme against plan.themes
    pub key_tempo: &'a KeyTempoPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ThematicRole { Statement, Contrast, Return, Development, Coda }

/// Slice 1 USES only Identity + Fragmented; the rest ship as schema (later stages).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ThemeVariation {
    Identity, Transposed, Reharmonized, Augmented, Diminished, Ornamented, Fragmented,
    Inverted, Retrograde,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CadenceStrength { Half, Imperfect, Perfect, Deceptive, Plagal }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Meter { Four4, Three4, Six8, Two4 }   // slice 1 always Four4

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Character { Ballad, Hymn, Nocturne, Drone, March, Lament, Waltz, Scherzo, Lilt, Gigue }
// slice 1 always Ballad; the rest ship as schema (default-pinned), realized in later stages.
```

**The contour-archetype resolver — the RESOLUTION of the one open reconciliation flag (operator decision 4).**

`MotifArchetype` is **NOT a stored field on `ThemeSeed` or `MotifNote`** in slice 1. It is a closed enum used **only at plan-build time** as the input to a one-way resolver that emits `Vec<MotifNote>`. This is the lock: the 8-archetype encoding lives in `chord_engine.rs` (Music Theory owns it) as a build-time function; its output is the existing `MotifNote { degree, dur_steps }` the realizer already reads. The realizer boundary does not change at all.

```rust
// In chord_engine.rs — Music Theory Specialist owns. Build-time only; NOT read at runtime.
/// The 8 curated melodic-shape archetypes (Arch default + Inverted-Arch, Descent, Ascent,
/// Neighbor-turn, Leap-and-step, Pendulum, Rising-sequence). Slice-1 ACTIVE subset is the
/// original four (Arch, Descent, Ascent, Neighbor-turn); the other four ship as variants.
pub enum MotifArchetype { Arch, InvertedArch, Descent, Ascent, NeighborTurn, LeapStep, Pendulum, RisingSequence }

/// Resolve a chosen archetype + image-derived range/rhythm params into the concrete
/// key-relative degree+duration sequence the realizer reads. THE ONE PLACE contour →
/// MotifNote happens. Called by composition.rs at plan build (§2), never at tick time.
pub fn resolve_motif(archetype: MotifArchetype, range_degrees: u8, length_steps: usize) -> Vec<MotifNote>;
```

---

## 2. THE CROSS-FILE CONTRACT (the most important section)

The exact boundary between `composition.rs` (Rust Implementer) and `chord_engine.rs` (Music Theory Specialist). Pinned so neither both-owns nor both-skips any piece.

**One paragraph, locked.** `composition.rs` owns the **planner**: it selects form/character/meter/key/theme-behaviour via the `SelectTable` ladders, expands the chosen `FormSpec.sections` into concrete `Section`s (scaling `rel_len` to fill `total_steps`), and **per section** calls the EXISTING `chord_engine` craft (`pick_progression` → `generate_chords` → `plan_phrases`) to fill that section's `progression` and its `steps: Vec<StepPlan>`. For the **theme**, `composition.rs` chooses the `MotifArchetype` and the range/length params from the image knobs (hue + edge_activity), then **calls `chord_engine::resolve_motif(...)`** to turn that archetype into the concrete `Vec<MotifNote>` it stores on `ThemeSeed.motif`. `chord_engine.rs` (Music Theory) owns `resolve_motif` (the contour→degree resolver), the 8 archetypes' shapes, the articulation clamp (§4.4), and — the realizer side — `decide_instrument_action`/`realize_step` gain a `ctx: &StepContext` parameter and, on a step whose `ctx.section.theme` is `Some` and whose role is Melody, the realizer **reads `ctx.theme.motif`** and plays that degree (mapped to a chord tone / NCT in the section's chord) instead of free-selecting the top chord tone; on `Fragmented` it plays only the first half of the motif then the melody role rests; on a `theme: None` / Contrast section it free-selects exactly as today. So: **Music Theory produces `ThemeSeed.motif` (via `resolve_motif`, called by composition.rs) and consumes it (the realizer's theme-replay); composition.rs produces the archetype choice + params and the section/plan structure; the realizer's theme-replay logic is Music Theory's, threaded by the Implementer.**

**The decision-kernel signature change (LOCKED).** `decide_instrument_action` gains exactly one trailing parameter:

```rust
pub fn decide_instrument_action(
    f: &ScanBarFeatures,
    inst_idx: usize,
    step_idx: usize,
    num_instruments: usize,
    plan_steps: &[StepPlan],   // UNCHANGED type — now the CURRENT SECTION's filled steps (ctx.section.steps)
    ms_per_step: u64,          // now the SECTION's tempo (ctx.section.ms_per_step)
    ctx: &StepContext,         // NEW — the plan-relative context
) -> InstrumentDecision;
```

What the realizer reads from `ctx`: `ctx.section.thematic_role` (Statement/Return → play theme; Contrast → free-select), `ctx.theme` (the `Vec<MotifNote>` to replay when present), `ctx.step_in_section` (which motif note this step carries / theme-statement timing), `ctx.section.variation` (Identity vs Fragmented truncation). `ctx.section.key_offset_semitones` and `ctx.key_tempo` are read but are **always 0 / home in slice 1** (no transposition fires) — they exist so Stage 5 needs no signature change.

**Where contour-archetype resolution happens (LOCKED):** at **plan-build time, in `composition.rs`, by calling `chord_engine::resolve_motif`**. NOT at tick time, NOT in the realizer. The realizer only ever reads already-resolved `MotifNote`s. This is what keeps `MotifNote` unchanged and the freeze safe.

**The empty-plan guard stays:** when `plan_steps.is_empty()`, return a silent `InstrumentDecision` (P2). Unchanged.

---

## 3. THE NON-LOOPING PLAN EXPANSION (death of `plan[step_idx % len]`)

Today the engine holds one flat `plan: Vec<StepPlan>` (`engine.rs:274`) and the realizer indexes `plan[step_idx % plan.len()]` (`engine.rs:580`), and `tick` reads `self.plan[step_idx % self.plan.len()].position` for the phrase snapshot (`engine.rs:448`). Slice 1 replaces the single flat looped plan with a **`CompositionPlan` whose sections each carry their own `steps`, concatenated and played once 0→`total_steps`.**

**Flow:** `compose_from_image(&understanding)` → `CompositionPlanner::plan(&understanding)` returns a `CompositionPlan` →

1. `form_id = plan_mappings.form.select(&u)` (first-match-wins, else default).
2. character / meter / key_scheme / theme_behaviour likewise (slice 1: pinned to ballad / four4 / home_only).
3. `form_spec = lookup(form_catalogue, form_id)`.
4. `KeyTempoPlan`: `home_root_midi` from dominant-hue lookup (reuse existing root logic / `EngineConfig.root_midi` seed); `base_ms_per_step` from brightness→BPM (the existing `interp_tempo_bpm` path) clamped to the Ballad window; `key_scheme`/`tempo_scheme` all-home / all-base.
5. `total_steps` from a base budget × section count (image-influenced, deterministic).
6. Themes: choose archetype + params from `u` (hue + edge_activity), call `chord_engine::resolve_motif` → `ThemeSeed.motif`, per the `theme_behaviour` selection.
7. **Expand** `form_spec.sections` → `Vec<Section>`: scale each `rel_len` to a `step_len` summing to `total_steps`; for each section call `pick_progression(home_mode)` → `generate_chords(...)` → `plan_phrases(chords)` to fill `progression` and `steps: Vec<StepPlan>`; copy `theme`/`variation`/`boundary_cadence` from the `SectionTemplate`.
8. Assemble `CompositionPlan { form: form_id, character, meter, key_tempo, sections, themes, total_steps }`.

**Engine fields/methods that change (`engine.rs` — Rust Implementer):**

- **`PipelineEngine.plan: Vec<StepPlan>`** (`:274`) → **`PipelineEngine.composition: Option<CompositionPlan>`** plus a retained `plan: Vec<StepPlan>` ONLY as the legacy/back-compat single-section path (see §4). Recommended: keep the `plan` field for the `set_features_global` legacy path bit-for-bit, and ADD `composition: Option<CompositionPlan>`. When `composition` is `Some`, `tick`/`decide_step` walk it; when `None`, the legacy flat path runs unchanged.
- **`set_plan(&mut self, plan: CompositionPlan)`** — NEW. Installs a precomputed `CompositionPlan` (sets `composition = Some(plan)`).
- **`compose_from_image(&mut self, understanding: &ImageUnderstanding)`** — NEW. Calls `CompositionPlanner::plan` then `set_plan`.
- **`decide_step`** (`:472`): when `composition` is `Some`, resolve `(section, step_in_section)` from the global `step_idx` (walk section boundaries, NO modulo), build the `StepContext`, and call `decide_instrument_action(f, inst, step_idx, num, &section.steps, section.ms_per_step, &ctx)`. The section's local step index into `section.steps` is `step_in_section` (the realizer still wraps WITHIN a section's own filled steps if `section.steps.len() < step_len`, but the engine never wraps the global cursor).
- **`tick`** (`:405`): the phrase snapshot at `:448` (`self.plan[step_idx % self.plan.len()].position`) becomes a `self.snapshot_phrase(step_idx)` helper that, in composition mode, reads the resolved section's step position; the `tick` restructure for the borrow is §4.2.
- **`total_steps`** drives the time cursor: `tick` advances `step_index` 0→`total_steps` once. `step_count()` from the `FeatureSource` still bounds the scan; when composing, `total_steps` is the authoritative N (the planner sizes sections to it).

**New `CompositionPlanner` (`composition.rs` — Rust Implementer; calls chord_engine, does not duplicate it):**

```rust
pub struct CompositionPlanner { plan_mappings: PlanMappings }
impl CompositionPlanner {
    pub fn new(plan_mappings: PlanMappings) -> Self;
    /// Deterministic given (understanding, plan_mappings) EXCEPT the delegated pick_progression
    /// thread_rng (the documented S9 boundary; the equivalence net never calls this path).
    pub fn plan(&self, understanding: &ImageUnderstanding) -> CompositionPlan;
}
impl SelectTable {
    /// Pure scan: first rule whose every Predicate holds against `u`, else `default`.
    pub fn select(&self, u: &ImageUnderstanding) -> String;
}
```

---

## 4. THE BACK-COMPAT LANDING ORDER (each step keeps `engine_equivalence` byte-green)

The freeze (from the actual test): `decide_instrument_action(&f, inst, step, num, &plan, MS_PER_STEP)` — **6 args** — pins `plan[step_idx % plan.len()]` (P3 modulo wrap), `G_BASS_NOTE=36` / `G_MELODY_NOTE=79` (P5), cadence velocity `114`/`84` (P6), cadence hold `240 ms` (P6), `MS_PER_STEP=200`.

### 4.1 Three landing steps

1. **Types only.** Add `composition.rs` (all §1 types) + `engine.rs` re-export + the `pure_analysis::understand_image_pure` producer. **No behaviour change → net GREEN.**
2. **`ctx` parameter, defaulted everywhere.** Add `ctx: &StepContext` to `decide_instrument_action` (now 7 args). Provide `StepContext::single_section_default(...)` (§4.3). Wire the default at the one production call site (`decide_step` `:481`) **and** update `tests/engine_equivalence.rs`'s call sites to pass the default `ctx`. Because the default applies zero transposition / no theme / home key, **the goldens (240, 114/84, 36/79) do not move; net GREEN.** This is the ONE allowed touch of `engine_equivalence.rs` in slice 1 — it adds an argument to each `decide_instrument_action(...)` call; **it does not relax a single assert.** (The P3 modulo-wrap test still passes the legacy `fixed_plan()` and exercises the wrap — see §4.5.)
3. **`compose_from_image` / `CompositionPlanner` producing >1 section ONLY when called.** Reachable only via the new compose path (analogue of `set_features_global`'s boundary — the net never calls it). The legacy `plan` field + `set_features_global` still produce the single flat looped plan. The golden moves only where a slice deliberately re-derives it — in slice 1 that is **only the §4.4 articulation clamp.**

### 4.2 The `tick` borrow restructure (operator decision 6 — borrowed)

`tick` (`:405`) is `&mut self`; it must build `ctx` from an immutable borrow of `self.composition`, hand it to `decide_step`, and only THEN write `self.step_index` (`:457`). Because `decide_step` returns an OWNED `Vec<InstrumentDecision>`, the borrow held by `ctx` ends at the end of the `decide_step` call expression — well before the `&mut` field write. Compiles under NLL, no `unsafe`/`RefCell`/clone. Locked structure:

```text
fn tick(&mut self, source, sink):
    if paused { return … }                       // unchanged early-return
    let step_idx = self.step_index;
    let decisions = {
        // immutable borrow of self.composition, fully scoped to this block:
        // resolve (section, step_in_section), build StepContext, call the kernel.
        self.decide_step(source, step_idx)        // returns OWNED Vec; borrow ends here
    };
    // sink sends (uses owned `decisions`) — unchanged loop
    // phrase snapshot via a SHORT immutable borrow, then drop, then advance:
    let phrase = self.snapshot_phrase(step_idx);  // returns owned PhrasePosition
    self.last_phrase = phrase;
    let total = self.total_steps_or(source);      // total_steps when composing, else step_count()
    self.step_index = step_idx + 1;               // &mut field write — no live borrow now
    self.scan_position = …
```

`decide_step` itself becomes the place that resolves the section and builds the `StepContext` per step (built ONCE per step, borrowed by each per-instrument `decide_instrument_action` — it is step-relative, not instrument-relative).

### 4.3 `StepContext::single_section_default` — the exact construction the net uses

The equivalence test constructs a single behaviour-neutral `Section` + `KeyTempoPlan` as locals (the test owns the lifetimes, exactly as it already owns `fixed_plan()`), and borrows them into the `ctx`:

```rust
impl<'a> StepContext<'a> {
    /// The behaviour-neutral default: one section, no theme, home key, identity variation.
    /// Under it the kernel does EXACTLY what it does today — no transposition, no theme,
    /// home mode, same ms_per_step. Used by engine_equivalence.rs and the legacy flat path.
    pub fn single_section_default(section: &'a Section, key_tempo: &'a KeyTempoPlan) -> StepContext<'a>;
}
```

The test builds: `Section { label:"A", step_len:<plan.len()>, thematic_role:Statement, key_offset_semitones:0, ms_per_step:MS_PER_STEP, mode:"Ionian", progression:vec![], theme:None, variation:Identity, boundary_cadence:Perfect, density:0.5, steps:<fixed_plan()> }` and `KeyTempoPlan { home_root_midi:60, home_mode:"Ionian", base_ms_per_step:MS_PER_STEP, key_scheme:vec![0], tempo_scheme:vec![MS_PER_STEP] }`, then `StepContext::single_section_default(&section, &key_tempo)`, and passes `&ctx` as the new 7th arg. **`theme:None` + `key_offset:0` ⇒ the realizer takes its existing free-select path ⇒ goldens unchanged.**

### 4.4 The S13 articulation clamp (the one deliberate golden move — Music Theory owns)

Live at `chord_engine.rs:1156`–`:1164`: `curve_frac = LEGATO_FRAC_HI(1.05) + (STACCATO_FRAC(0.40) - 1.05)*edge_activity`, then `.clamp(0.30, 1.20)`. Narrow the **non-cadence** window to **`0.55 ≤ base_frac ≤ 1.10`** by RE-SCALING the curve into the narrower range (not truncating — a calm image stays more legato than a busy one, within musical bounds). **The cadence branch stays byte-stable:** the `is_cadence` early return at `:1194`–`:1196` (`sustained(0, step_ms, LEGATO_FRAC)`) and the `sustained` helper's `(frac*rit).min(1.20)` and the `240 ms` cadence hold are UNTOUCHED. This deliberately moves the **non-cadence** articulation goldens — re-derive the affected constants/expected values **by hand in the same commit** with a derivation comment pointing here (S13 §7 discipline). Never loosen an assert to silence it. The per-character `articulation_mult` rides on top in later stages, defaulting to ×1.0 (no-op) in slice 1 (Ballad).

### 4.5 What stays GREEN unchanged

`engine_equivalence.rs` P3 (`test_step_idx_wraps_via_modulo`) passes the legacy `fixed_plan()` directly as `plan_steps` and the default `ctx` — the modulo wrap WITHIN the supplied `&[StepPlan]` is the realizer's, unchanged; the engine's non-looping cursor (§3) is a separate, compose-path concern the net never touches. P1/P2/P4/P5/P6/P7 are all byte-stable under the default `ctx`. Only the non-cadence articulation goldens (if the net pins any — verify; the current net pins only the cadence branch at 240 ms, which is frozen) move, and only via §4.4.

---

## 5. THE `mappings.json` ADDITIONS

A new top-level block `composition` (additive — the OLD `mappings.json` must still parse with the new loader; verify the loader treats the block as optional or ships it). **Ownership inside the block is split** — see §6.

### 5.1 FormSpec rows for the 6 slice-1 forms (Music Theory authors the rows; Implementer wires the loader)

```jsonc
"composition": {
  "form_catalogue": [
    { "id": "rounded_binary", "sections": [
        {"label":"A",  "role":"Statement","rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Half"},
        {"label":"B",  "role":"Contrast", "rel_len":1.0,"theme":null,"variation":"Identity","boundary_cadence":"Half"},
        {"label":"A'", "role":"Return",   "rel_len":0.75,"theme":0,"variation":"Identity","boundary_cadence":"Perfect"} ] },
    { "id": "ternary_aba", "sections": [
        {"label":"A","role":"Statement","rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Perfect"},
        {"label":"B","role":"Contrast", "rel_len":1.0,"theme":null,"variation":"Identity","boundary_cadence":"Half"},
        {"label":"A","role":"Return",   "rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Perfect"} ] },
    { "id": "aaba", "sections": [
        {"label":"A","role":"Statement","rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Imperfect"},
        {"label":"A","role":"Statement","rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Imperfect"},
        {"label":"B","role":"Contrast", "rel_len":1.0,"theme":null,"variation":"Identity","boundary_cadence":"Half"},
        {"label":"A","role":"Return",   "rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Perfect"} ] },
    { "id": "abac", "sections": [
        {"label":"A","role":"Statement","rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Half"},
        {"label":"B","role":"Contrast", "rel_len":1.0,"theme":null,"variation":"Identity","boundary_cadence":"Imperfect"},
        {"label":"A","role":"Return",   "rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Half"},
        {"label":"C","role":"Coda",     "rel_len":1.0,"theme":null,"variation":"Identity","boundary_cadence":"Perfect"} ] },
    { "id": "abbac", "sections": [
        {"label":"A", "role":"Statement","rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Half"},
        {"label":"B", "role":"Contrast", "rel_len":1.0,"theme":null,"variation":"Identity","boundary_cadence":"Deceptive"},
        {"label":"B'","role":"Contrast", "rel_len":1.0,"theme":null,"variation":"Fragmented","boundary_cadence":"Half"},
        {"label":"A", "role":"Return",   "rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Imperfect"},
        {"label":"C", "role":"Coda",     "rel_len":0.75,"theme":null,"variation":"Identity","boundary_cadence":"Plagal"} ] },
    { "id": "theme_and_variations", "sections": [
        {"label":"T", "role":"Statement",  "rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Perfect"},
        {"label":"V1","role":"Development","rel_len":1.0,"theme":0,"variation":"Fragmented","boundary_cadence":"Imperfect"},
        {"label":"V2","role":"Development","rel_len":1.0,"theme":0,"variation":"Identity","boundary_cadence":"Perfect"} ] }
  ],
```

### 5.2 The form-selection SelectTable + theme-behavior table (Music Theory authors thresholds)

```jsonc
  "form": {
    "default": "rounded_binary",
    "rules": [
      { "when": [ {"knob":"complexity","op":"ge","lo":0.66,"hi":0.0}, {"knob":"edge_activity","op":"ge","lo":0.6,"hi":0.0} ], "pick": "theme_and_variations" },
      { "when": [ {"knob":"quadrant_contrast","op":"le","lo":0.2,"hi":0.0} ], "pick": "ternary_aba" },
      { "when": [ {"knob":"aspect_ratio","op":"ge","lo":1.6,"hi":0.0}, {"knob":"palette_bimodality","op":"le","lo":0.3,"hi":0.0} ], "pick": "aaba" },
      { "when": [ {"knob":"vertical_emphasis","op":"ge","lo":0.6,"hi":0.0} ], "pick": "abac" },
      { "when": [ {"knob":"edge_activity","op":"ge","lo":0.7,"hi":0.0}, {"knob":"value_key","op":"ge","lo":0.6,"hi":0.0} ], "pick": "abbac" }
    ]
  },
  "theme_behaviour": {
    "default": "absent",
    "rules": [ { "when": [ {"knob":"complexity","op":"ge","lo":0.4,"hi":0.0} ], "pick": "fragment" } ]
  },
```

### 5.3 Schema-present, default-pinned tables (slice 1 inert)

```jsonc
  "character":   { "default": "ballad",    "rules": [] },
  "meter":       { "default": "four4",     "rules": [] },
  "key_scheme":  { "default": "home_only", "rules": [] }
}
```

`character_overlays` / `key_schemes` may be omitted in slice 1 (the loader defaults them) or shipped empty; their full schema lands at Stage 4 / Stage 5. **Determinism note:** `op` values are snake_case (`ge`/`le`/`lt`/`gt`/`in_range`) to match the `#[serde(rename_all="snake_case")]` on `CmpOp`. The slice-1 ladder rules read `complexity`/`edge_activity` (live, well-spread) and the default-pinned knobs (`quadrant_contrast`/`vertical_emphasis`/`palette_bimodality`/`aspect_ratio`) — the latter sit at their slice-1 defaults, so those rungs are effectively dormant until Stage 8/9 fills the knobs, falling through to `rounded_binary`. That is honest degradation, not breakage.

---

## 6. FILE-OWNERSHIP + OFF-LIMITS MAP

| Area | Owner |
|---|---|
| `src/composition.rs` (NEW: all §1 types, `CompositionPlanner`, `SelectTable::select`, plan expansion §3) | **Rust Implementer** |
| `src/pure_analysis.rs` (`understand_image_pure` producer; dead-feature re-exposure) | **Rust Implementer** |
| `src/engine.rs` (the `ImageUnderstanding` mirror re-export, `composition` field, `set_plan`/`compose_from_image`, the `ctx` threading on `decide_step`/`decide_instrument_action`, the `tick` restructure §4.2, `snapshot_phrase`) | **Rust Implementer** |
| `src/chord_engine.rs` (the `resolve_motif` contour→degree resolver + 8 archetypes; the realizer's theme-replay reading `ctx.theme`; the §4.4 articulation clamp + hand-re-derived non-cadence goldens) | **Music Theory Specialist** |
| `src/mapping_loader.rs` (deserialize the new `composition` block; keep OLD mappings.json parsing) | **Rust Implementer** |
| `assets/mappings.json` `composition.form_catalogue` rows + `form`/`theme_behaviour` rule THRESHOLDS | **Music Theory Specialist** (musical content) |
| `assets/mappings.json` `composition` block STRUCTURE / the default-pinned `character`/`meter`/`key_scheme` stubs + loader wiring | **Rust Implementer** (schema) |
| `tests/engine_equivalence.rs` (add the `ctx` 7th arg at each call site — NO assert relaxed; §4.3) | **Rust Implementer** |

**OFF-LIMITS to BOTH:** `src/modem.rs`, `src/bin/modem_*`, `src/synth_sink.rs`, `src/midi_output.rs`, `src/cli.rs`, `src/tui.rs`. And: `decide_instrument_action`'s musical DECISION logic except the **additive** `StepContext` threading + the plan expansion + the theme-replay branch (Music Theory) — i.e. neither implementer reshapes the existing role-pitch / velocity-contour / rhythm-pattern bodies; they ADD the theme branch and thread `ctx`, nothing more.

---

## 7. RESOLVED RECONCILIATION FLAG

**Motif encoding ↔ MotifNote shape (the one open flag).** RESOLVED, locked: `MotifNote { degree: i8, dur_steps: u8 }` is **UNCHANGED**. The contour-from-8-archetypes encoding is a **build-time input** to `chord_engine::resolve_motif(archetype, range, length) -> Vec<MotifNote>`, called by `composition.rs` at plan build; `ThemeSeed.motif` stores the resolved `Vec<MotifNote>`; the realizer reads only that. `MotifArchetype` is a closed enum used at build time only — it is NOT stored on `ThemeSeed` or `MotifNote` and is NOT read at tick time. This is the cleanest seam: rich/safe contour vocabulary at build, the frozen degree+duration boundary at the realizer. No realizer signature changes from the motif encoding (only the additive `ctx`).

*Design-only. No source, test, or asset modified by this document.*
