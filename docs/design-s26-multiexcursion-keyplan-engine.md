# Design S26 — Multi-excursion structural key plan: the GENERALIZED model + the unlocked realizer pivot

**Author role:** Rust Architect. **DESIGN ONLY.** This document modifies no source, test, or
asset. It produces exactly one artifact — this file. It specifies the machinery that takes the
S24/S25 key plan from an *abac-only example* to a GENERAL form/key-scheme catalogue that can carry
genuine tonal travel across the whole form vocabulary, adds the minimal per-region affect needed to
make multiple excursions genuinely distinct, and unlocks the realizer pivot/cadence (the K3 work the
operator has now scoped).

**Date:** 2026-06-16. **HEAD:** `9cd9681` (K1 BUILT & CLOSED; the K1 design docs were authored
against `ea99165`/the K1 work tree, now merged at this HEAD).

**Builds on (do not restate):**
- `docs/design-s24-image-as-form-key-plan.md` — the LOCKED buildable spec. Especially Decision 1
  (FORM + **Invariant A**: a Coda on new material still resolves to the HOME key), Decision 2
  (region→key, energy-ordered), Decision 3 (the v1 menu `{+7,+5,+3,−3}`), Decision 5 (direct
  modulation in K1; pivot deferred), Decision 6 (mode-constant relative), §2.5(a) the `key_scheme`
  catalogue rows, §4 Slices K2/K3, §5 the property tests (esp. §5.3 `resolves_home`).
- `docs/input-s25-k1-keyplan-harmony.md` — the Music Theory lane's pinned menu numbers.
- `docs/review-S25.md` — the K1 Quality Gate PASS + the two non-blocking re-listen notes (the
  whole-image-valence limitation is the one this arc resolves).

**The operator's resolved decision (verbatim intent), which this design is built against:**
1. ABAC was only ONE example. The target is the GENERAL machinery so the WHOLE catalogue
   (`rounded_binary`, `ternary_aba`, `aaba`, `abac`, `abbac`, `theme_and_variations`) can carry
   tonal travel — not an abac-only hack.
2. **Option A.** C becomes a GENUINE second excursion (a real, distinct key destination — NOT
   collapsed to home), AND the realizer gets a **pivot/cadence** so an off-home journey can resolve
   home where the form calls for it. This UNLOCKS the realizer (the K3 work) — `chord_engine.rs` is
   no longer frozen for this arc; it carries its OWN byte-freeze argument.
3. **Open / off-home endings are a DELIBERATE FEATURE.** Some forms/images should legitimately end
   unresolved. The design carries a per-form/per-scheme "lands home vs stays open" decision rather
   than forcing home universally.
4. Excursions are made genuinely distinct via **per-region affect** — a small PURE-RUST per-region
   brightness pass built from the EXISTING kernels (no new dependency, no ML, deterministic) so B
   and C read DIFFERENT region affect and travel to genuinely different keys. The "nudge C to the
   next menu entry" guard is documented as a FALLBACK only.

---

## 1. CURRENT STATE ANALYSIS (verified against the working tree at `9cd9681`)

### 1.1 The key-scheme types, functions, and wiring (all in `src/composition.rs`)

| Element | Location | Signature / shape (verbatim from tree) |
|---|---|---|
| `KeySchemeSection` | `composition.rs:449–456` | `pub struct { pub label: String, pub offset_rule: String }` (`serde::Deserialize`). `offset_rule` ∈ `"home" | "region_related:b" | "region_related:c"`; unknown → 0. |
| `KeyScheme` | `composition.rs:460–465` | `pub struct { pub id: String, #[serde(default)] pub sections: Vec<KeySchemeSection> }`. `"home_only"` (empty `sections`) is the identity anchor. |
| `key_scheme_catalogue` on `PlanMappings` | `composition.rs:717–721` | `#[serde(default)] pub key_scheme_catalogue: Vec<KeyScheme>` |
| `From<CompositionMappings>` arm | `composition.rs:742` | `key_scheme_catalogue: c.key_scheme_catalogue` (explicit struct literal, NO `..Default`) |
| `key_scheme: SelectTable` (id selector) | `composition.rs:683` (on `PlanMappings`); `select` at `:660` | first-match-wins rule scan → a scheme id string |
| `lookup_key_scheme` | `composition.rs:1175–1177` | `fn lookup_key_scheme<'a>(catalogue: &'a [KeyScheme], id: &str) -> Option<&'a KeyScheme>` |
| `relative_offset` | `composition.rs:1183–1197` | `fn relative_offset(home_mode: &str) -> i8` — substring match (`aeolian/minor/dorian/phrygian/locrian` → `+3`, else `−3`) |
| `excursion_offset` | `composition.rs:1215–1245` | `fn excursion_offset(u: &ImageUnderstanding, home_mode: &str) -> i8` — hue-distance (≥60° → relative) then a SINGLE `affect_valence > 0.40` split (`+7` else `+5`) |
| `resolve_key_scheme` | `composition.rs:1260–1304` | `fn resolve_key_scheme(scheme: Option<&KeyScheme>, sections: &[SectionTemplate], u: &ImageUnderstanding, home_mode: &str) -> Vec<i8>` |
| The §2.4 wiring (the only behavior change K1 made) | `composition.rs:945` (un-discard `key_scheme_id`), `:971–976` (resolve once per plan), `:1077` (`key_offset_semitones: offsets.get(i).copied().unwrap_or(0)`), `:1092` (`let key_scheme = offsets.clone()`) | — |
| `CompositionPlan` / `Section` / `KeyTempoPlan` spine | `composition.rs:823–839` / `:785–819` / `:769–781` | `Section.key_offset_semitones: i8` (`:794`); `KeyTempoPlan { home_root_midi: u8 (:772), key_scheme: Vec<i8> (:778), tempo_scheme: Vec<u64> (:780) }` |
| Form ladder + `form_catalogue` | `composition.rs:677` (`form: SelectTable`), `:693` (`form_catalogue: Vec<FormSpec>`); `FormSpec`/`SectionTemplate` at `:529–535` / `:511–525`; `ThematicRole` at `:275–283` = `{Statement, Contrast, Return, Development, Coda}` | — |
| `SelectTable`/`Predicate`/`Knob` machinery | `SelectTable` `:648–668`; `Predicate::holds` `:619–631`; `Knob`/`Knob::read` `:539–592` | `Knob::read` is the ONE field-name → scalar map; a new knob adds exactly one arm |

### 1.2 Which affect/region fields EXIST in `ImageUnderstanding` today (verified `composition.rs:39–98`)

Present, per-region **ENERGY**: `subject_energy` (`:83`), `foreground_energy` (`:85`),
`background_energy` (`:87`). Whole-image **affect**: `affect_arousal` (`:95`), `affect_valence`
(`:97`) (planner-computed, sentinel `-1.0` from the pixel producer). Whole-image palette/value:
`dominant_hue`, `secondary_hue`, `value_key`, `avg_brightness`, `avg_saturation`, `colorfulness`,
`subject_hue` (`:77`), `subject_saturation` (`:79`), `fg_bg_contrast` (`:81`). Whole-image balance:
`quadrant_contrast`, `vertical_emphasis`, `mass_centroid`.

**Per-region brightness / valence / hue does NOT exist** as a first-class field. This is the
load-bearing gap. **It must be ADDED** (§3). Note carefully: `pure_analysis.rs::analyze_regions_pure`
(`:451–512`) DOES compute per-cell `RegionStats { mean_value (brightness 0..100), mean_saturation,
dominant_hue, edge_energy, … }`, and `understand_image_pure` (`:639–764`) DOES split the 3×3 grid
into a subject cell, a foreground band `{1,3,5,7}` and a background band `{0,2,6,8}` — but it then
**collapses** those bands to a single scalar each (`foreground_energy`/`background_energy` =
`edge_energy` means at `:734–735`) and reads `subject_hue`/`subject_saturation` off the subject cell
only. The brightness/hue/saturation of the foreground and background BANDS is computed and then
discarded. **The per-region affect add is therefore a pure re-surfacing of values already computed
in the existing pass — no new pixel work, no new kernel, no new dependency.**

The consequence today (review-S25 note 2, design Risk 3): `resolve_key_scheme`'s energy-order picks
which region label is B vs C, but `excursion_offset` reads only WHOLE-IMAGE `affect_valence` and
`subject_hue`/`secondary_hue`, so **B and C resolve to the SAME offset** — there is no genuine second
excursion. The `region_related:b` and `region_related:c` arms of `resolve_key_scheme` (`:1281`)
literally both call `excursion_offset(u, home_mode)` with identical arguments.

### 1.3 The realizer transposition + cadence seam (`src/chord_engine.rs`, READ-ONLY today)

- **The transpose seam consumes `key_offset_semitones` in ONE place: melody only.** `theme_pitch`
  (`chord_engine.rs:2098–2121`): `tonic_pc = ((ctx.key_tempo.home_root_midi as i16 +
  ctx.section.key_offset_semitones as i16).rem_euclid(12)) as u8` (`:2105–2106`). This shifts the
  THEME melody's degree-to-pitch tonic.
- **CRITICAL — the harmony does NOT currently transpose.** The per-section chords are pre-generated
  in the planner: `chord_engine.generate_chords(&progression, home_root_midi, &home_mode, …)`
  (`composition.rs:1062–1070`) is called with the literal `home_root_midi` (60), NOT
  `home_root_midi + key_offset`. So under K1, a B section's *chords* sound in the home key and only
  the THEME melody's tonic shifts. For closely-related K1 menu moves this reads as a leaning melody
  over home harmony; for genuine multi-excursion travel (Option A) the harmony must move too. This is
  a primary K3 seam decision (§4) and a primary RISK (§7).
- `generate_chords` signature: `pub fn generate_chords(&self, progression: &[String], root_midi: u8,
  mode: &str, edge_complexity: f32, brightness_drop: f32, saturation01_raw: f32, colorfulness_raw:
  f32) -> Vec<Chord>` (`chord_engine.rs:170–179`).
- The cadence realization is DATA-DRIVEN and already present: `plan_phrases` (`:637`) stamps
  `PhrasePosition::{HalfCadence, PerfectAuthenticCadence, PhraseStart, Interior}` (`:476–482`) at
  phrase boundaries and re-spells the boundary chord to the exact `V`/`I` the cadence requires; the
  non-final phrase rests on a Half cadence, the final closes V→I. `realize_step` reads
  `is_cadence`/`is_phrase_start` off `step.position` (`:1041–1045`). The `boundary_cadence`
  (`CadenceStrength` `:302–310`) lives on `SectionTemplate` and is carried onto `Section`.
- `realize_step` PUBLIC signature (frozen): `pub fn realize_step(step: &StepPlan, inst_idx: usize,
  num_instruments: usize, features: &PerfFeatures, ms_per_step: u64, ctx:
  &crate::composition::StepContext) -> Vec<NoteEvent>` (`chord_engine.rs:1021–1028`). It receives
  `ctx.section` (carries `key_offset_semitones`, `boundary_cadence`, `orchestration`) and
  `ctx.key_tempo` zero-copy. **Prior slices (S23 prominence, S17 pad) deepened behavior WITHOUT
  changing this signature** — they threaded data via `ctx`/the section and recomputed inside. That is
  the blessed extension route.
- **The section-seam point.** The engine builds ONE `StepContext` per global step in
  `decide_step` (`engine.rs:543–564`), resolving `(section, step_in_section)` via `comp.locate`
  (no modulo). A "section boundary" is exactly the step where `step_in_section == 0` for a section
  index > 0 — and the realizer can see this because `ctx.step_in_section` and `ctx.section` are both
  present. The PIVOT therefore hooks at the FIRST step of a modulating section (or the LAST step of
  the section preceding it). No new engine plumbing is required to reach the seam; `ctx` already
  carries everything.

### 1.4 The byte-freeze anchors

- **`engine.rs`** sha256 (verified now) = `7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23`
  (matches the K1 witness). The freeze anchor is the `engine_equivalence` golden sweep
  (`tests/engine_equivalence.rs`): `single_section_default` identity path (`StepContext::
  single_section_default`, `composition.rs:884–894`), goldens **240** (cadence hold `1.20*200`),
  **114/84** (bass velocity sat100/sat0), **36** (`G_BASS_NOTE`, C2 floor), **79** (`G_MELODY_NOTE`,
  G5-area). `engine.rs:750` and the equivalence net hand-build `Section` with
  `key_offset_semitones: 0`. **Do NOT move these goldens.**
- **`chord_engine.rs`** sha256 (verified now) = `b448d9363499234e7e5ddce18fbb3017b754acbea3af51126cd5e51b1215e39b`.
  This file is UNLOCKED for this arc (the K3 slice only), under its own byte-freeze argument (§4.3).
  K2a/K2b do NOT touch it.

---

## 2. THE GENERALIZED KEY-SCHEME MODEL

The change is from "schemes are abac-only data" to "any form in `form_catalogue` can declare an
N-excursion key plan with an explicit resolution policy." Three additions, all backward-compatible.

### 2.1 A per-scheme RESOLUTION POLICY (lands-home-vs-open) — new types in `composition.rs`

```rust
/// How a key scheme ENDS — the operator's "lands home vs stays open" decision, per scheme
/// (S26). Open/off-home endings are a DELIBERATE feature, not a defect: some forms (and some
/// images routed onto them) legitimately end unresolved. The policy is DATA (a JSON enum tag),
/// resolved by the planner; the realizer reads only the per-section offsets + the pivot/land
/// flags derived from it. `Resolve` is the byte-stable default (it is what every K1 scheme does
/// implicitly today — the final section is "home"). NEW S26.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionPolicy {
    /// The FINAL section's offset is forced to 0 (home) regardless of its `offset_rule`, and the
    /// realizer's land-home cadence (§4) is armed for that boundary. This realizes Invariant A:
    /// a Coda on new material still resolves to the HOME key. The K1 / default behavior.
    Resolve,
    /// The final section keeps its own `offset_rule`-derived offset (may be non-zero → ends
    /// OFF-home). The land-home cadence is NOT armed. This is the deliberate open ending.
    Open,
}

impl Default for ResolutionPolicy {
    /// Absent in JSON → `Resolve` (the byte-stable, ends-home default; matches every K1 scheme).
    fn default() -> Self { ResolutionPolicy::Resolve }
}
```

`KeyScheme` gains the policy plus an opt-in pivot flag (additive, `#[serde(default)]` → old JSON
parses byte-identically):

```rust
/// A named per-section offset rule set (S24, GENERALIZED S26). "home_only" (empty `sections`) is
/// the identity anchor. Resolved once per plan by `resolve_key_scheme`. NOW carries a resolution
/// policy (lands-home vs open) and an opt-in realizer-pivot flag.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct KeyScheme {
    pub id: String,
    #[serde(default)]
    pub sections: Vec<KeySchemeSection>,
    /// NEW S26 — how the scheme ends. `#[serde(default)]` → `Resolve` (ends home; the K1
    /// behavior). `Open` lets the final section stay off-home (the deliberate open ending).
    #[serde(default)]
    pub resolution: ResolutionPolicy,
    /// NEW S26 — opt-in: when `true`, the realizer (K3) inserts a witnessed pivot chord at each
    /// modulating section boundary and a land-home cadence at a `Resolve` final return. `false`
    /// (the default) keeps the K1 direct-modulation behavior AND is the realizer byte-freeze
    /// gate — with `pivot == false` the realizer inserts NOTHING. `#[serde(default)]` → `false`.
    #[serde(default)]
    pub pivot: bool,
}
```

### 2.2 The generalized N-excursion `offset_rule` set (a closed, parsed vocabulary)

K1 parses `offset_rule` as ad-hoc strings inside `resolve_key_scheme` (`composition.rs:1279–1283`).
S26 keeps the JSON string shape (no schema break) but widens the recognized set so a scheme can
declare more than two excursions and bind each to an energy-RANKED non-subject region. The rule
grammar (documented; parsed in the planner, NOT a new serde enum so unknown still degrades to home):

```rust
/// The recognized `offset_rule` grammar (S26). Parsed in `resolve_key_scheme`; an unrecognized
/// string degrades to `Home` (offset 0 — byte-stable). This is documentation of the string
/// contract, expressed as an internal enum the planner maps the string onto; it is NOT a serde
/// type (the JSON stays a string so old/unknown rules degrade rather than fail to parse).
///
///   "home"               → Home              (offset 0; binds a Statement/Return/home role)
///   "region_related:b"   → Excursion(rank 0) (the MOST-energetic non-subject region)
///   "region_related:c"   → Excursion(rank 1) (the SECOND-most-energetic non-subject region)
///   "region_related:d"   → Excursion(rank 2) (… extends to N excursions; reserved)
enum OffsetRule {
    Home,
    /// `rank` indexes into the energy-DESCENDING ordering of the non-subject regions: rank 0 =
    /// most energetic (today the eye's first stop), rank 1 = next, … Each rank reads THAT
    /// region's own affect (after §3) so distinct ranks travel to genuinely distinct keys.
    Excursion(u8),
}

/// Parse an `offset_rule` string into the closed grammar (S26). Unknown → `Home` (byte-stable).
/// Pure, total.
fn parse_offset_rule(s: &str) -> OffsetRule;
```

The generalized resolve replaces the K1 body of `resolve_key_scheme` while keeping its signature and
its zero-pad/truncate totality. New doc comment + signature (the policy + ranked regions are the only
new behavior):

```rust
/// Resolve a `KeyScheme`'s per-section offset RULES into concrete `key_offset_semitones` (S24,
/// GENERALIZED S26). Returns one `i8` per section IN ORDER, length == `sections.len()`.
///
/// - "home" → 0 (binds a home role: Statement/Return).
/// - "region_related:b|c|d" → `Excursion(rank)`: the menu offset computed from the rank-th
///   most-energetic NON-SUBJECT region's OWN affect (per-region brightness/hue from §3), so
///   distinct ranks travel to genuinely distinct keys (the "eye sweeps twice" intent).
/// - The `scheme.resolution` policy is applied LAST: `Resolve` forces the FINAL section's offset
///   to 0 (Invariant A — a Coda on new material still lands home); `Open` leaves it as resolved
///   (the deliberate off-home ending).
/// - A `None`/empty (`home_only`) scheme, or any unknown rule, yields all-zero (the identity /
///   byte-freeze path). PURE: no clock, no RNG.
///
/// The energy-DESCENDING region ranking (Decision 2 generalized) is computed once from the
/// per-region energies; rank `k` selects the k-th region. The returned length always equals the
/// form's section count (zero-pad/truncate on mismatch; the debug-only role-alignment assertion
/// fires per Risk 6).
fn resolve_key_scheme(
    scheme: Option<&KeyScheme>,
    sections: &[SectionTemplate],
    u: &ImageUnderstanding,
    home_mode: &str,
) -> Vec<i8>;
```

The per-rank, per-region menu computation is a generalization of `excursion_offset` that takes the
chosen region's OWN affect rather than whole-image affect:

```rust
/// One non-subject region's affect, energy-ranked. The planner builds an energy-DESCENDING list
/// of these from the per-region fields (§3) so `resolve_key_scheme` can address "the rank-th
/// region." Pure data; no music. NEW S26.
struct RegionAffect {
    /// Region brightness 0..1 (per-region valence proxy; from §3 `*_brightness`/100).
    valence: f32,
    /// Region dominant hue 0..360 (from §3 `*_hue`).
    hue: f32,
    /// Region energy 0..1 (the existing `foreground_energy`/`background_energy`), the rank key.
    energy: f32,
}

/// The B/C/… excursion offset for ONE specific non-subject region (S24 Decisions 3/4, REGIONALIZED
/// S26). Same menu math as the K1 `excursion_offset`, but reads the GIVEN region's own
/// valence/hue (from §3) instead of whole-image affect, and the hue distance is measured against
/// the SUBJECT hue. Direction: high region-valence → dominant +7 (near) / relative on strong hue
/// contrast; low → subdominant +5 (near) / relative on strong contrast. Returns a value in the v1
/// menu `{+7,+5,+3,−3}`. PURE.
fn region_excursion_offset(region: &RegionAffect, subject_hue: f32, home_mode: &str) -> i8;
```

`region_excursion_offset` SUBSUMES the K1 `excursion_offset` — when called with a `RegionAffect`
built from whole-image affect (the §3-absent fallback) it reproduces K1 exactly, so the K1 tests
stay green during K2a. The K1 `excursion_offset` becomes a thin shim that builds a whole-image
`RegionAffect` and delegates (or is removed once §3 lands and the energy-ranked path is the only
caller — the Implementer's choice, recorded as a non-blocking cleanup).

### 2.3 How Section roles bind to offsets (the role/offset contract, unchanged shape, widened)

The binding is by **section ORDER within the chosen form**, with `thematic_role` as the safety
check (the existing K1 discipline, `composition.rs:1291–1301`). The generalized contract:

| `ThematicRole` | Intended `offset_rule` | Offset under `Resolve` | Offset under `Open` |
|---|---|---|---|
| `Statement` | `home` | 0 | 0 |
| `Return` | `home` | 0 | 0 |
| `Contrast` | `region_related:b` (rank 0) | menu | menu |
| `Development` | `region_related:c` (rank 1) | menu | menu |
| `Coda` (final) | `region_related:c`/`d` (Option A) | **forced 0** | **kept (off-home)** |

The role-alignment debug assertion widens: `home` rules must land on `Statement`/`Return`;
`region_related:*` rules must land on `Contrast`/`Development`/`Coda`. (`Coda` is allowed a
non-home rule now — that is the Option A change.)

### 2.4 How `abac_rondo`'s C row changes under Option A, and the other forms in ONE machinery

The contradiction in S24 §2.5(a) (`abac_rondo` C = `region_related:c` ends off-home, vs Invariant A)
is resolved by the `resolution` policy, NOT by editing the C row. The C row STAYS
`region_related:c` (C is a genuine second excursion — Option A); the policy decides the landing:

```jsonc
// abac_rondo under Option A — C is a GENUINE second excursion; resolution decides the landing.
{ "id": "abac_rondo",
  "resolution": "resolve",                 // Invariant A: C resolves to HOME even on new material
  "pivot": false,                          // K2b ships pivot:false (byte-safe); K3 flips it on
  "sections": [
    { "label": "A", "offset_rule": "home" },
    { "label": "B", "offset_rule": "region_related:b" },   // rank-0 region
    { "label": "A", "offset_rule": "home" },
    { "label": "C", "offset_rule": "region_related:c" } ] } // rank-1 region (DISTINCT key, then home)
```

Because `resolution: "resolve"` forces the FINAL section's offset to 0, the resolved offsets are
`[0, B, 0, 0]` — C travels to its own key in PLANNING intent and (under K3 `pivot:true`) the
realizer journeys there mid-section and lands home on the Coda cadence. Under `pivot:false` (K2b),
C's resolved offset is forced 0 (it ends home as K1 did) but the `region_related:c` rule is now
*present and rank-1*, so the moment §3 + K3 land, C becomes a real distinct excursion with no
catalogue edit. If the operator instead wants this form to END OPEN on a particular routing, a
sibling scheme `abac_open` with `"resolution": "open"` keeps C's offset non-zero.

The SAME machinery expresses every other form — these are JSON rows, no Rust change:

```jsonc
{ "id": "rounded_binary_excursion", "resolution": "resolve", "pivot": false, "sections": [
    { "label": "A",  "offset_rule": "home" },
    { "label": "B",  "offset_rule": "region_related:b" },
    { "label": "A'", "offset_rule": "home" } ] },

{ "id": "ternary_aba_excursion",  "resolution": "resolve", "pivot": false, "sections": [
    { "label": "A", "offset_rule": "home" },
    { "label": "B", "offset_rule": "region_related:b" },
    { "label": "A", "offset_rule": "home" } ] },

{ "id": "aaba_excursion", "resolution": "resolve", "pivot": false, "sections": [
    { "label": "A", "offset_rule": "home" },
    { "label": "A", "offset_rule": "home" },
    { "label": "B", "offset_rule": "region_related:b" },   // the bridge departs
    { "label": "A", "offset_rule": "home" } ] },

{ "id": "abbac_excursion", "resolution": "resolve", "pivot": false, "sections": [
    { "label": "A", "offset_rule": "home" },
    { "label": "B", "offset_rule": "region_related:b" },   // rank-0
    { "label": "B", "offset_rule": "region_related:c" },   // rank-1 — the two B's now DIVERGE
    { "label": "A", "offset_rule": "home" },
    { "label": "C", "offset_rule": "region_related:c" } ] },

{ "id": "theme_and_variations_excursion", "resolution": "open", "pivot": false, "sections": [
    { "label": "T",  "offset_rule": "home" },
    { "label": "V1", "offset_rule": "region_related:b" },
    { "label": "V2", "offset_rule": "region_related:c" },
    { "label": "V3", "offset_rule": "region_related:b" } ] }  // a T&V may legitimately drift / end open
```

`theme_and_variations_excursion` is the natural showcase of the OPEN policy (a variation set that
wanders is idiomatic). `abbac_excursion` is the natural showcase of the N-excursion ranking (its two
B's read rank-0 and rank-1 regions and therefore DIVERGE — the "eye sweeps twice" made audible).

### 2.5 The `key_scheme` SelectTable rules that route forms onto schemes (Aesthetics-lane data)

The selector (`composition.rs:683`, `assets/mappings.json` `key_scheme`) gains rules picking the
new schemes when the corresponding form is selected AND a real subject/ground stratification exists
(`fg_bg_contrast ≥ 0.25`, the K1 gate), ordered so the more-specific multi-excursion schemes are
tried before the single-excursion ones. `home_only` stays the byte-stable default. (Data shape;
the Aesthetics lane owns these rows, the Implementer is the sole committer — §6 ownership.)

---

## 3. PER-REGION AFFECT ADD (`src/pure_analysis.rs` only — ZERO new dependency, no realizer touch)

The minimal change so B and C read DISTINCT region affect. **It re-surfaces values
`analyze_regions_pure` ALREADY computes** (per-cell `mean_value`, `dominant_hue`,
`mean_saturation`) and currently discards for the foreground/background bands.

### 3.1 New `ImageUnderstanding` fields (`composition.rs:39–98` struct + `neutral()` + `Knob`)

```rust
// Added to `ImageUnderstanding` (composition.rs), after the energy triplet (:87):
/// NEW S26 — mean brightness (0..1) of the foreground band (the non-subject edge-mid cells
/// {1,3,5,7} minus the subject cell). Per-region valence proxy: lets the planner travel the
/// foreground excursion by the foreground's OWN brightness, not the whole image. Pure pixel
/// stat from the existing region pass. Defaults to whole-image `avg_brightness/100` in
/// `neutral()` and when the band is degenerate (honest fallback → K1 whole-image behavior).
pub foreground_brightness: f32,
/// NEW S26 — mean brightness (0..1) of the background band (corner cells {0,2,6,8} minus
/// subject). Same discipline.
pub background_brightness: f32,
/// NEW S26 — dominant hue (0..360) of the foreground band. Per-region hue, so the near-vs-
/// relative hue-distance test (§2.2 `region_excursion_offset`) measures the FOREGROUND's hue
/// against the subject, not the whole image. Defaults to `secondary_hue`.
pub foreground_hue: f32,
/// NEW S26 — dominant hue (0..360) of the background band. Same discipline.
pub background_hue: f32,
```

Each gets a `Knob` variant + a one-line `Knob::read` arm (`composition.rs:539–592`) so SelectTable
rules can read them, and a field in `neutral()` (`:103–130`) set to the whole-image fallback
(`avg_brightness/100` for brightness, `secondary_hue` for hue) so an absent producer degrades to K1.

### 3.2 The pure region pass (re-surfacing, in `understand_image_pure`, `pure_analysis.rs:639–764`)

No new function is strictly required — the values already exist inside `understand_image_pure` where
`foreground_energy`/`background_energy` are computed (`:720–735`). The change is to compute the band
MEANS of `mean_value` and the circular hue mean of `dominant_hue` over the same band index sets and
assign them to the new fields. Expressed as a small pure helper for clarity (signature + doc only):

```rust
/// Mean brightness (0..1) and circular-mean dominant hue (0..360) over a band of region cells,
/// EXCLUDING the subject cell (S26). Reuses the per-cell `RegionStats.mean_value` /
/// `RegionStats.dominant_hue` that `analyze_regions_pure` already produced — NO new pixel pass,
/// NO new dependency. Hue is averaged circularly (the same unit-vector mean `hsv_means` uses) so
/// the red wrap is handled. Returns `(brightness01, hue_deg)`; a fully-degenerate band (all cells
/// are the subject — impossible for a single argmax) falls back to the whole-image values the
/// caller passes. `idxs` is the band's cell indices ({1,3,5,7} foreground, {0,2,6,8} background).
fn band_affect(
    regions: &[RegionStats],
    idxs: &[usize],
    subj_idx: usize,
    fallback_brightness01: f32,
    fallback_hue_deg: f32,
) -> (f32, f32);
```

Existing kernels reused: `analyze_regions_pure` (the 3×3 pass, already called at `:645`), each
cell's `mean_value`/`dominant_hue` (already on `RegionStats`, `:434–442`), and the circular-mean
arithmetic pattern from `hsv_means` (`:169–209`). The assignment lines added to the
`ImageUnderstanding { … }` literal at `:738–763`:

```rust
// foreground band {1,3,5,7}, background band {0,2,6,8}, both minus the subject cell:
let (fg_brightness, fg_hue) =
    band_affect(&regions, &[1,3,5,7], subj_idx, g.avg_brightness/100.0, dominant_hue);
let (bg_brightness, bg_hue) =
    band_affect(&regions, &[0,2,6,8], subj_idx, g.avg_brightness/100.0, dominant_hue);
// … foreground_brightness: fg_brightness, background_brightness: bg_brightness,
//     foreground_hue: fg_hue, background_hue: bg_hue,
```

**Module boundary held:** this lives entirely in `pure_analysis.rs` (pixels in, image-free scalars
out); it touches NO music logic and NO realizer. The planner's `region_excursion_offset` (§2.2)
consumes these scalars, the same way it consumes the existing energy fields. **Zero new
dependency** (everything uses `image 0.24` + the already-present kernels).

### 3.3 The guard (documented FALLBACK only, per operator)

If, for some image, the rank-0 and rank-1 regions resolve to the SAME menu offset (e.g. both bands
are mid-bright and hue-near), the offsets collapse and the "two excursions" are not distinct. The
per-region affect of §3.1–§3.2 is the PRIMARY mechanism and usually prevents this. As a documented
FALLBACK (NOT the default path), `resolve_key_scheme` MAY nudge the rank-1 offset to the
next-ranked menu entry when it would equal the rank-0 offset. This is recorded as an OPTIONAL guard,
ear-gated, and is the cheap mechanism the operator explicitly de-prioritized in favor of per-region
affect. It must NOT be the only thing making excursions distinct.

---

## 4. THE REALIZER PIVOT / CADENCE (K3 — now unlocked; `chord_engine.rs` carries its own freeze)

The Music Theory lens owns the actual harmonic RULES of the pivot (which common-tone/pivot chord,
its voicing, its duration). This design specifies the SEAM, the signatures, where it hooks, the data
it consumes, and — load-bearingly — the byte-freeze argument. **The pivot is reachable ONLY when a
non-`home_only` scheme has `pivot: true`.**

### 4.1 The two harmony decisions K3 must make first (the harmony-transposition gap)

Because today the harmony is generated at `home_root_midi` (§1.3) and does not move, K3 must decide
HOW a B/C section's chords sound in their excursion key. Two options for the Music Theory lens
(this design recommends Option (i)):

- **(i) Re-root the section's chords at the offset key (recommended).** In the planner, generate the
  section's chords at `home_root_midi + key_offset_semitones` instead of the literal `home_root_midi`
  — i.e. pass the per-section root into `generate_chords` (`composition.rs:1062`). This is a PLANNER
  change (NOT a `chord_engine` body change) and is itself byte-safe for `home_only` (offset 0 → same
  root → byte-identical chords). It makes the harmony genuinely travel with the melody. **This part
  can ship in K2b/K3-planner-half without touching `chord_engine.rs` at all** — flagged as the
  cheapest way to make travel audible.
- **(ii) Transpose at realize time in `chord_engine`.** Heavier; freeze-sensitive; not recommended
  given (i) exists.

### 4.2 The pivot seam (signature + hook + consumed data; bodies are the Music Theory lens')

```rust
/// A witnessed pivot/common-tone chord inserted at a MODULATING section boundary (S26/K3). The
/// chord prepares the move from the previous section's key to this section's `key_offset_semitones`
/// so a direct modulation no longer sounds like a splice. Returns the pivot's note events for this
/// step, or `None` when no pivot applies (the byte-freeze gate — see §4.3).
///
/// Reachable ONLY when: (a) the active scheme is non-`home_only` AND has `pivot == true` (carried
/// onto the section via a new `Section` flag the planner sets), AND (b) this is the FIRST step of a
/// section whose `key_offset_semitones` differs from the previous section's. Under the identity /
/// `home_only` / `pivot:false` path this returns `None` and inserts NOTHING.
///
/// Data consumed (all already on `ctx`, zero-copy): `ctx.section.key_offset_semitones` (the
/// destination key), the previous section's offset (threaded via a new `StepContext` field
/// `prev_key_offset_semitones: Option<i8>`, `None` on the first section / identity), the home root
/// and mode (`ctx.key_tempo`), and `ctx.step_in_section` (== 0 at the boundary). The pivot's
/// harmonic RULE (which common-tone chord, voicing) is the Music Theory lens' to specify.
fn pivot_chord_events(
    ctx: &crate::composition::StepContext,
    features: &PerfFeatures,
    ms_per_step: u64,
) -> Option<Vec<NoteEvent>>;

/// The land-home cadence arming flag at the final return (S26/K3). When the scheme's
/// `ResolutionPolicy::Resolve` forced the final section to offset 0 AND `pivot == true`, the
/// realizer strengthens the final boundary's existing Perfect cadence into an explicit V→I in the
/// home key (it does not re-author the cadence DATA — `plan_phrases` already stamps Perfect; this
/// only ensures the journey-home is voiced as a true authentic cadence). Pure; reads
/// `ctx.section.boundary_cadence` + the `resolution`/`pivot` flags carried onto the section.
fn land_home_is_armed(ctx: &crate::composition::StepContext) -> bool;
```

**Where it hooks:** inside `realize_step` (`chord_engine.rs:1021`), guarded at the very top:
`if let Some(pivot) = pivot_chord_events(ctx, features, ms_per_step) { return pivot; }` placed BEFORE
the existing `base_note`/velocity/rhythm path, so the identity path falls straight through to the
frozen code. `realize_step`'s PUBLIC signature is UNCHANGED (the new data rides `ctx` — the S23
prominence / S17 pad precedent). The only `StepContext` change is an additive field
`prev_key_offset_semitones: Option<i8>` (and the planner-set `pivot`/`resolution` flags carried onto
`Section`), set by the engine's per-step `ctx` build (`engine.rs:547–552`) and defaulted to `None`
in `single_section_default` (`composition.rs:884–894`) so the equivalence net stays on the identity
path.

### 4.3 THE BYTE-FREEZE ARGUMENT (explicit — engine.rs goldens 240/114/84/36/79 cannot move)

The realizer change is reachable ONLY through a `pivot:true`, non-`home_only` scheme. The identity
path is byte-identical, by these guarantees:

1. **The pivot is gated on `pivot == true` AND a key change.** `pivot_chord_events` returns `None`
   unless the active scheme has `pivot: true` (defaulted `false`, §2.1) AND
   `ctx.section.key_offset_semitones != prev_key_offset_semitones`. Under `home_only` every offset is
   0 and every section equals its predecessor → `None` → nothing inserted.
2. **`engine.rs` is NOT touched in a way that moves its goldens.** The only `engine.rs` change is
   building the additive `prev_key_offset_semitones` into the per-step `ctx` (`engine.rs:547–552`),
   which is `None` on the legacy/equivalence path (`single_section_default` sets it `None`). The
   equivalence net hand-builds `Section { key_offset_semitones: 0, … }` and uses
   `single_section_default` (`tests/engine_equivalence.rs:147/164/190/225/267/…`), so its `ctx` has
   `prev: None` and `pivot:false` → the pivot guard is dead → the realizer takes the frozen path.
   **engine.rs sha256 stays `7a07fb…343c23` IF the `ctx`-build edit is byte-irrelevant to the golden
   sweep** — which it is, because the golden sweep runs the identity `ctx`. (If the Implementer finds
   the `engine.rs` ctx-build edit perturbs the sha, the additive field can be defaulted at the
   `StepContext` constructor so `engine.rs`'s compose-path build is the only writer and the legacy
   build is untouched — recorded as the byte-freeze contingency.)
3. **`chord_engine.rs` carries its OWN freeze re-witness.** Its sha256 WILL change (new functions
   added). The freeze argument is BEHAVIORAL not byte-level for this file: the
   `no_inversion_invariant` sweep and a new `pivot_inserts_nothing_on_identity` test prove that for
   every `home_only`/`pivot:false` plan the realized note stream is byte-identical to pre-K3, and the
   `engine_equivalence` goldens (which exercise only the identity `ctx`) stay 240/114/84/36/79.
4. **The land-home cadence inserts nothing new structurally.** `land_home_is_armed` only strengthens
   the VOICING of an already-stamped Perfect cadence (it does not add a step or move a boundary), and
   it is armed only under `pivot:true` + `Resolve`. Under identity it is `false`.

**Witnesses (machine-checkable):** `sha256sum src/engine.rs` ==
`7a07fb8568fcd94536dd3ec2e4dc71c8154f9d9678599a6106e3a542a4343c23` after K3 (or the contingency in
guarantee 2 applied); `cargo test --test engine_equivalence` 9/9 green, goldens unmoved;
`pivot_inserts_nothing_on_identity` green; `no_inversion_invariant` green across all menu offsets.

### 4.4 The `main.rs` note_off / cross-step-sustain seam (flagged)

The adapter schedules per-step `note_on`/`note_off` pairs, each note bounded within its own step's
`hold_ms` (`main.rs:494–534`). A pivot chord that wants to SUSTAIN across the section boundary (a
true common-tone hold) cannot extend a note past its step under the current scheduler. **Per the
prior-slice precedent, K3 must realize the pivot WITHIN one step (the boundary step) using
legato-overlap — i.e. the pivot's notes hold to the end of their step and the next section's downbeat
overlaps — rather than changing the scheduler.** A scheduler change (cross-step sustain) is
explicitly OUT OF SCOPE for K3 and would be its own freeze-sensitive slice if the ear demands it.
This keeps `main.rs` untouched.

---

## 5. DATA FLOW DIAGRAM (module boundaries)

```
  image (RgbImage)
      │
      ▼
┌─────────────────────────────── pure_analysis.rs (PIXELS → image-free scalars; no music) ──┐
│  analyze_regions_pure (3×3)  ──►  RegionStats[ mean_value, dominant_hue, edge_energy, … ]   │
│            │                                                                                │
│            ├─ pick_subject_region ─► subj_idx                                               │
│            ├─ band energies {1,3,5,7}/{0,2,6,8}  ─► foreground_energy / background_energy   │
│            └─ NEW S26 band_affect ─► foreground_brightness/_hue, background_brightness/_hue │
│                                                                                            │
│  understand_image_pure  ─────────────────────────► ImageUnderstanding (image-free mirror)  │
└────────────────────────────────────────────────────────────────────────────────────────┘
      │ (field-copy at the boundary; NO image type crosses)
      ▼
┌─────────────────────────────── composition.rs (PLANNER; no pixels, no realize) ───────────┐
│  affect_composite ─► affect_arousal/valence (whole-image)                                  │
│  key_scheme.select(u) ─► scheme id ─► lookup_key_scheme ─► KeyScheme{sections,resolution,pivot}│
│  resolve_key_scheme:                                                                       │
│     energy-DESCENDING rank of non-subject regions (RegionAffect from §3 fields)            │
│     per rank → region_excursion_offset(region, subject_hue, home_mode)  ∈ {+7,+5,+3,−3}    │
│     apply ResolutionPolicy (Resolve → force final 0 / Open → keep)                         │
│        ─► Section.key_offset_semitones[i] + KeyTempoPlan.key_scheme: Vec<i8>               │
│     (K3) generate_chords(progression, home_root_midi + offset, mode, …) per section §4.1   │
│  carry scheme.pivot / scheme.resolution onto each Section                                   │
└────────────────────────────────────────────────────────────────────────────────────────┘
      │ (CompositionPlan installed on the engine)
      ▼
┌─────────────────────────────── engine.rs (CURSOR; no note selection) ──────────────────────┐
│  decide_step: comp.locate(step_idx) ─► (section, step_in_section)                          │
│     build StepContext { section, step_in_section, theme, key_tempo,                        │
│                         NEW prev_key_offset_semitones }  (per global step, zero-copy)       │
└────────────────────────────────────────────────────────────────────────────────────────┘
      │ ctx
      ▼
┌─────────────────────────────── chord_engine.rs (REALIZER; no image logic) ─────────────────┐
│  realize_step(step, …, ctx):                                                               │
│     (K3) pivot_chord_events(ctx,…) ─Some─► return pivot events (boundary step, pivot:true)  │
│         └─None (identity / home_only / pivot:false) ─► FROZEN free-select/theme path        │
│     theme_pitch: tonic_pc = (home_root_midi + key_offset).rem_euclid(12)                    │
│     (K3) land_home_is_armed(ctx) ─► strengthen final Perfect cadence voicing                │
└────────────────────────────────────────────────────────────────────────────────────────┘
      │ NoteEvent[]
      ▼
  main.rs adapter (per-step note_on/note_off scheduling; legato-overlap, no scheduler change)
```

---

## 6. MULTI-SESSION DECOMPOSITION

Cadence per slice (the S21/S24 pattern, BINDING): **Architect spec → Implementer ∥ Music-Theory
input (file-disjoint) → Test Engineer → Quality Gate LAST.** Each slice independently shippable AND
independently HEARABLE. File-disjoint owners; single-writer `assets/mappings.json` (Implementer is
the sole committer for the slice).

### Slice K2a — per-region affect + generalized multi-excursion planner *(RECOMMENDED FIRST)*
- **Scope:** the §3 per-region affect add (4 fields + `band_affect` + 4 `Knob` arms); the §2.1
  `ResolutionPolicy` + `KeyScheme.{resolution,pivot}` types; the §2.2 generalized `resolve_key_scheme`
  + `region_excursion_offset` (energy-DESCENDING rank, per-region affect). **Planner + pure_analysis
  only.** Direct modulation still (no pivot). The `region_related:c` arm now genuinely diverges from
  `:b` because each reads its own region.
- **Files & owners (file-disjoint):** Implementer owns `src/pure_analysis.rs` (the §3 add) +
  `src/composition.rs` (the types + resolve + `Knob` arms) + `assets/mappings.json` (the new
  per-region `Knob` plumbing if any rule reads them; sole committer). Music-Theory lens owns a
  file-disjoint input doc: confirm the per-region direction mapping (region-brightness → V/IV/relative)
  meets a trained ear, and confirm the energy-DESCENDING ranking is the right "eye travel" order.
  **`chord_engine.rs` / `engine.rs` NOT touched.**
- **Byte-freeze posture:** SAFE. New fields are `#[serde]`-defaulted to whole-image fallbacks; the
  realizer is untouched; `home_only` still all-zero; engine.rs sha256 unchanged; goldens unmoved.
- **What the owner HEARS:** *two images with the same whole-image affect but different foreground/
  background brightness now travel to DIFFERENT B keys; a multi-excursion form's B and C land in
  GENUINELY distinct related keys (the eye sweeps twice) — still all in direct modulation, still
  lands home.*

### Slice K2b — the generalized catalogue rows for the whole form vocabulary *(planner/data only)*
- **Scope:** add the §2.4 catalogue rows (`rounded_binary_excursion`, `ternary_aba_excursion`,
  `aaba_excursion`, `abac_rondo` under Option A, `abbac_excursion`, `theme_and_variations_excursion`)
  + the `key_scheme` SelectTable rules routing each form onto its scheme; ship `theme_and_variations`
  with `resolution: "open"` to prove the open-ending policy end-to-end (the final section's offset
  stays non-zero, lands off-home). Optionally land §4.1 (i) re-rooting in the planner so the harmony
  travels (recommended; still no `chord_engine` touch).
- **Files & owners:** Aesthetics lane owns the scheme rows + SelectTable rules (structural shape);
  Music-Theory lane confirms each form's resolution policy (which forms SHOULD land home vs may end
  open) against a trained ear; Implementer is the sole committer of `assets/mappings.json` (+ the
  §4.1(i) planner re-root in `composition.rs` if included). **`chord_engine.rs`/`engine.rs` NOT
  touched.**
- **Byte-freeze posture:** SAFE (data + planner; `home_only` default; §4.1(i) is byte-identical at
  offset 0).
- **What the owner HEARS:** *every form in the catalogue now carries tonal travel; a T&V piece
  legitimately ENDS OPEN (off-home) where the policy says so; the open ending is a deliberate color,
  not a bug.* And — the `resolves_home` test becomes CONDITIONAL on the scheme's policy (§7).

### Slice K3 — the realizer pivot/cadence + open-ending realization *(freeze-sensitive, ear-gated)*
- **Scope:** §4.2 `pivot_chord_events` + `land_home_is_armed` in `chord_engine.rs`; the additive
  `StepContext.prev_key_offset_semitones` + the per-section `pivot`/`resolution` carry; flip the
  desired schemes to `pivot: true`; the §4.4 legato-overlap realization (no scheduler change). The
  Music-Theory lens specifies the pivot's harmonic rule + voicing.
- **Files & owners:** Implementer owns `src/chord_engine.rs` (the pivot fns + hook) + `src/engine.rs`
  (the additive ctx field, byte-irrelevant to goldens) + `src/composition.rs` (the section carry) +
  `assets/mappings.json` (the `pivot:true` flips). Music-Theory lens owns the pivot/cadence harmonic
  rules input doc. **THE ONE freeze-sensitive slice** — carries its own byte-freeze argument (§4.3)
  + its own `pivot_inserts_nothing_on_identity` + `no_inversion` re-witness; engine.rs goldens
  240/114/84/36/79 cannot move.
- **Byte-freeze posture:** SENSITIVE — gated, witnessed (§4.3). `chord_engine.rs` sha changes;
  `engine.rs` sha must stay frozen (contingency in §4.3 guarantee 2 if needed).
- **What the owner HEARS:** *the key changes stop being abrupt jumps and become prepared, hinged
  modulations; an off-home journey resolves home with a true V→I where the policy lands home, and
  stays open where it doesn't.*

**Recommended FIRST slice this engagement: K2a.** It is the decisive, byte-SAFE win that makes the
operator's core complaint ("B and C don't actually go anywhere distinct") audibly false, and it is
the prerequisite the K1 Quality Gate explicitly flagged (review-S25 note 2). It proves the per-region
affect + the generalized N-excursion planner before any realizer code moves. K2b is the catalogue
breadth (still safe). K3 is the smoothness/landing depth and the only freeze-sensitive change.

---

## 7. RISKS & TRADE-OFFS

1. **Byte-freeze risk once the realizer is unlocked (K3).** The mitigation is the §4.3 gate (pivot
   reachable only via `pivot:true` + a key change), the additive-`ctx` route (no `realize_step`
   signature change), and the explicit engine.rs-sha + golden re-witness. The residual risk is that
   the `engine.rs` `ctx`-build edit perturbs the golden sha; the §4.3 guarantee-2 contingency
   (default the field at the constructor, legacy build untouched) neutralizes it. K2a/K2b are
   byte-SAFE and carry no realizer risk.
2. **The `resolves_home` property test must become CONDITIONAL, not universal.** S24 §5.3 asserts the
   FINAL section's offset == 0 for every form × scheme. Under Option A + the open-ending policy this
   is FALSE for `resolution: "open"` schemes (they legitimately end off-home). The test must be
   re-scoped: `resolves_home` asserts final-offset==0 ONLY for `ResolutionPolicy::Resolve` schemes,
   and a NEW `open_schemes_may_end_off_home` asserts that an `Open` scheme on a firing image CAN
   resolve a non-zero final offset (the deliberate-feature witness). Landing this in K2b (where the
   first `Open` scheme ships) is mandatory — otherwise the open ending trips the universal test.
3. **The harmony-transposition gap (the subtle one).** Today only the THEME melody reads the offset;
   the chords sound in the home key (§1.3). Until §4.1(i) lands (the planner re-root), a "travel" is a
   melodic lean over home harmony, not a true modulation — which may underwhelm a trained ear in K2a.
   Mitigation: land §4.1(i) re-rooting in K2b (it is byte-safe at offset 0 and needs no `chord_engine`
   touch), so the harmony travels before the ear test that matters. Flag to the operator: K2a alone
   makes the *plan* distinct; K2b's re-root makes the *harmony* distinct; K3 makes the seams smooth.
4. **Per-region affect is still a 3×3-band proxy, not segmentation.** The band brightness/hue means
   inherit the existing center-surround saliency proxy's coarseness (pure_analysis `:13`/`:427`
   honesty note). It is deterministic and dependency-free, but a true DoG/segmentation mask is a later
   slice. Accepted; the owner's ear is the gate.
5. **N-excursion ranking on degenerate images.** When both non-subject bands have near-equal energy,
   the rank-0/rank-1 split is arbitrary-but-deterministic (stable tiebreak on band index). The §3.3
   fallback guard exists for the rare case both resolve to the same menu offset, but it is explicitly
   the documented fallback, not the primary mechanism.
6. **Module-boundary risk.** The per-region affect MUST stay in `pure_analysis.rs` (pixels) and the
   menu/direction logic MUST stay in `composition.rs` (planner). `region_excursion_offset` reads
   image-free scalars only; it names no pixel type. The realizer pivot reads `ctx` only; it names no
   image type. The single-writer `assets/mappings.json` discipline (Implementer sole committer per
   slice) holds, with disjoint keys (`key_scheme`/`key_scheme_catalogue` only).
7. **Mirror-risk (recurring).** The 4 new `ImageUnderstanding` fields are NOT serde-loaded (computed
   by the producer), so no `mapping_loader` mirror is needed for them — but the new `Knob` arms must
   land or a rule reading them fails to parse. The `KeyScheme.{resolution,pivot}` fields ARE serde
   and `#[serde(default)]`, parsed via the EXISTING `key_scheme_catalogue` mirror (already wired on
   all three touch-points per review-S25 criterion 3) — no new mirror touch-point, the round-trip
   witness still covers it.

---

*Design-only. No source, test, or asset modified by this document. Seam citations
(`composition.rs:39–98/275–283/302–310/449–465/529–535/539–592/648–668/677/683/693/717–742/769–839/
884–894/945/971–976/1062–1070/1077/1092/1175–1304`, `pure_analysis.rs:169–209/434–512/639–764`,
`chord_engine.rs:119/170–179/476–482/637/1021–1110/2098–2121`, `engine.rs:526–594/694–737/750`,
`main.rs:494–534`, `tests/engine_equivalence.rs` goldens 240/114/84/36/79) and the two sha256 anchors
are verified against the working tree at HEAD `9cd9681`. Signatures + types + doc comments are
binding shapes; bodies are the slice implementers'. The harmonic RULES of the pivot are the Music
Theory lens' to specify. The build-role titles (Architect, Implementer, Music Theory lens, Test
Engineer, Quality Gate) are the S21/S24 domain titles.*
