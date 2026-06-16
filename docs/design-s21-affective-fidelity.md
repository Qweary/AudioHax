# Design S21 — Affective Fidelity: the unified image→affect→character→sound design

**Author role:** Rust Architect — SYNTHESIS pass. DESIGN ONLY: this document modifies no
source, test, or asset. It reconciles three parallel S21 design streams into one build-ready
design and a staged, independently-hearable build plan. It writes Rust **signatures** (no bodies)
and `docs/` prose only; the single artifact this work produces is this file.
**Date:** 2026-06-15.
**Inputs reconciled:** `docs/design-s21-affect-mapping.md` ("the affect design" — the
valence/arousal bridge, character selection, tempo de-cap), `docs/design-s21-musical-craft.md`
("the music-craft design" — character-as-craft bundles, saliency→role, invariants/tests), and
`docs/design-s21-engine-reframe.md` ("the engine design" — the data threading, the byte-freeze
argument, the slicing).
**Verified against the working tree** (not trusted from the docs): `src/composition.rs` — the
`Character` enum at `:177–188` (ten variants, listed below), `parse_character` at `:919–933`
(all ten names wired, unknown→Ballad), the tempo clamp at `:727`, the `BALLAD_BPM_MIN/MAX` consts
at `:851–852`; `assets/mappings.json` — `brightness_to_tempo_bpm` at `:16–20` (tops at 120),
the empty-ruled `"character"` at `:134`, `feature_normalization` and the texture catalogue.

> Convention (carried from the three streams and `composition-architecture-engine.md`): signatures
> give the SHAPE of each seam, no bodies; new vocabulary lands as `mappings.json` rows parsed
> backward-compatibly via `#[serde(default)]`/`#[serde(skip)]`; pure-Rust default, ML opt-in only
> and NOT designed here; data-as-vocabulary; module boundaries (`pure_analysis` = pixels,
> `composition` = planner/affect, `chord_engine` = realizer, image-blind). The three streams are
> referred to as "the affect design", "the music-craft design", and "the engine design."

---

## 0. Executive summary (read first)

`example.jpg` — a bright, highly-saturated, chaotic, subjectless abstract painting — comes out as a
slow-to-mid **ballad**, like every image, because of three confirmed code facts:

1. **Tempo is double-capped.** `brightness_to_tempo_bpm` tops at 120 (`mappings.json:16–20`), then
   `composition.rs:727` re-clamps to the Ballad window `BALLAD_BPM_MIN/MAX = 56/96`
   (`:851–852`). A maximally energetic image cannot exceed 96 BPM.
2. **Character is pinned to Ballad.** `"character": { "default": "ballad", "rules": [] }`
   (`mappings.json:134`) is an empty ladder, so `parse_character` always returns
   `Character::Ballad`. The enum already carries ten variants but only Ballad is reachable, and
   nothing downstream reads the variant anyway.
3. **There is no pooled energy signal.** Saturation feeds only harmonic complexity, edge-activity
   only rhythm/form. Nothing combines the energy-bearing features into one quantity that co-drives
   tempo + loudness + density.

**The unified fix, in one page:**

- **The affect bridge.** Pool the existing perceptual scalars into two continuous axes —
  **arousal** (saturation-led energy) and **valence** (brightness-led mood) — on Russell's
  circumplex. These are computed once per plan by a single new pure fn `affect_composite()` in the
  planner, exposed to the existing `SelectTable` ladder as two new `Knob`s (`Arousal`/`Valence`),
  with the composite weights living in `mappings.json` as data. (§Decision 3.)

- **The reconciled character set — SIX presets, NO enum edit.** **Ballad** (default/identity),
  **Hymn**, **Nocturne**, **March**, **Lament**, and the energetic/joyful corner mapped to the
  **existing `Scherzo` enum variant** (NOT a new `Jubilee` variant). `Scherzo` is selected by the
  affect ladder for high-arousal/high-valence; its craft bundle is the music-craft design's
  energetic preset. Optional later additions (Waltz/Lilt/Gigue/Drone) ride the SAME mechanism and
  already exist in the enum. (§Decisions 1 & 2.)

- **The tempo de-cap.** Replace the single hard `clamp(56,96)` with a **per-character tempo
  window** keyed by the selected character; arousal positions the BPM inside the chosen window.
  `Scherzo`'s window is fast (≈120–168); `Lament`'s is slow (≈44–66). Data-driven, opt-in, and
  byte-stable when the `affect` block is absent. (§Decision 3.)

- **The saliency→role system.** A planner-computed **prominence** vector (from the saliency knobs
  already on `ImageUnderstanding`) reweights the EXISTING five-role system: the salient subject is
  foregrounded into the MELODY (louder/higher/rhythmically freer), recessive regions become a
  fuller-but-quieter background bed. It rides `OrchestrationProfile` as a resolved-only
  `#[serde(skip)]` field and is consumed by the realizer as centered, identity-at-0.5 nudges — so a
  subjectless field (like `example.jpg`) realizes uniformly and byte-stably. (§Decision below;
  Slice B.)

**The whole thing is staged so each slice is independently HEARABLE and the
`engine_equivalence` byte-freeze (goldens 240/114/84/36/79, `realize_step` signature,
`single_section_default` identity path) is provably unmoved.** Recommended S22 first slice:
**Slice A — affect→character + tempo de-cap**, data + planner only, no realizer/engine touch, the
biggest audible win on `example.jpg`.

---

## 1. The reconciled decisions (each PINNED)

### Decision 1 — THE ENERGETIC CHARACTER: use the existing `Scherzo` variant. No enum edit. (PINNED)

**The conflict.** The affect design names the high-arousal/high-valence preset `Scherzo` and
correctly notes it is an existing unused enum variant. The music-craft design names it `Jubilee`
and requests one additive enum variant + one `parse_character` arm. The engine design observes the
enum already declares ten variants and recommends NO enum edit in the freeze-critical path.

**Verified ground truth** (`src/composition.rs:177–188`, `parse_character:919–933`):

```
Character { Ballad, Hymn, Nocturne, Drone, March, Lament, Waltz, Scherzo, Lilt, Gigue }
```

All ten names are already wired in `parse_character`. **`Scherzo` exists. `Jubilee` does not.**

**Decision: the high-arousal/high-valence corner is `Character::Scherzo`.** No new enum variant,
no `parse_character` edit, no enum touch of any kind.

**Rationale (why `Scherzo` wins over `Jubilee`):**
1. **The binding constraint is "prefer NO enum edit if an existing variant fits."** A variant
   that is literally the fast/light/playful/major affect already exists; adding `Jubilee` would be
   a net-new enum variant whose ONLY justification is a name preference, against a hard byte-freeze
   discipline. The cost/benefit is strictly negative.
2. **`Scherzo` is the correct concert-music label** for the high-arousal/high-valence affect — fast,
   light, playful, major-leaning (lit. "joke"). The affect literature's "high arousal + high
   valence" musical signature *is* the scherzo character. The owner (trombone, music-performance
   degree) will recognize `Scherzo` as a real, coherent character; `Jubilee` is a coined label.
3. **The engine design's character ladder already picks `scherzo`** for the high-A/high-V rule —
   adopting it requires zero change to that doc's pinned data.
4. **The music-craft design's `Jubilee` *bundle* is fully preserved** — every craft scalar it
   specified (fast tempo window, articulation toward detached/détaché ≈0.65–0.75, rhythm-band
   shift down, raised dynamic level, figured/animated bed, major/bright modes) becomes the
   **`Scherzo` preset bundle** verbatim. Only the *label* changes from `Jubilee`→`Scherzo`; the
   sound is identical. The music-craft design loses nothing but the enum edit.

**Final character→enum-variant map (every preset maps to an actual enum variant):**

| Preset (this design) | Enum variant (verified exists) | Status |
|---|---|---|
| Ballad (DEFAULT/identity) | `Character::Ballad` | shipped, identity anchor |
| Hymn | `Character::Hymn` | reachable via affect ladder |
| Nocturne | `Character::Nocturne` | reachable via affect ladder |
| March | `Character::March` | reachable via affect ladder |
| Lament | `Character::Lament` | reachable via affect ladder |
| **Energetic/joyful** | **`Character::Scherzo`** | **the `example.jpg` cure; existing variant** |
| (later, optional) Waltz / Lilt / Gigue / Drone | `Waltz`/`Lilt`/`Gigue`/`Drone` | exist; ride same mechanism |

### Decision 2 — THE CHARACTER-PRESET SET: six core, each placed on the V×A plane (PINNED)

**The conflict.** The affect design proposed Ballad/Waltz/March/Lament + Scherzo (5). The
music-craft design proposed Ballad/Hymn/Nocturne/Drone/March/Lament + Jubilee (7). They overlap on
Ballad/March/Lament + an energetic corner but differ on Waltz vs Hymn/Nocturne/Drone.

**Decision: a SIX-preset core set that ships in the affect ladder, plus a documented reserve.**
The core six are the ones with both (a) a distinct, defensible craft bundle from the music-craft
design AND (b) a clean affect-ladder selection from the affect design AND (c) an existing enum
variant. They span all four quadrants of the plane:

```
                    HIGH AROUSAL
                         │
        March ───────────┼─────────── SCHERZO            (energetic corner)
     (high-A, low/neutral-V,  │     (high-A, high-V — example.jpg's home:
      firm/martial/detached)  │      fast, major, dense, light-detached)
                         │
   LOW ──────────────────┼────────────────────── HIGH   VALENCE
   VALENCE               │
                         │
       Lament ───────────┼─────────── Hymn               (Hymn: low/mid-A, high-V,
     (low-A, low-V,       │  Nocturne   consonant, stately block harmony)
      minor, slow, SOFT)  │  Ballad     (Nocturne: low-A, gentle, figured bed)
                         │  (DEFAULT, low-A, mild-V, safe center)
                    LOW AROUSAL
```

| Preset | Enum | V/A corner | Arousal band | Valence band | Tempo window (BPM) | Mode lean | Articulation (ARTIC_BIAS) | Craft signature (from music-craft design) |
|---|---|---|---|---|---|---|---|---|
| **Ballad** *(DEFAULT)* | Ballad | low-A / mild-V | low–mid | mild-+ | **56–96** *(legacy)* | either | legato (1.0 = identity) | 4/4, slow harmonic rhythm, sustained pad bed, broad phrases. **The byte-freeze identity vector.** |
| **Scherzo** *(energetic)* | Scherzo | high-A / high-V | ≥~0.60 | ≥~0.55 | **120–168** | major/bright (Ionian/Lydian/Mixolydian) | détaché ≈0.65–0.75 | fast, bright, dense onsets, rhythm-band shifted down, figured/animated bed, secondary dominants drive. **example.jpg's home.** |
| **March** | March | high-A / low–neutral-V | ≥~0.60 | <~0.45 | **96–132** | either | marcato ≈0.55 | 4/4 or 2/4, chords on strong beats, melody subdivides, terraced firm dynamics. High energy *without* joy. |
| **Lament** | Lament | low-A / low-V | ≤~0.30 | <~0.35 | **44–66** | minor (Aeolian/Phrygian/Dorian) | legato, weighted, exaggerated ritardando | descending bass, suspensions, deceptive cadences, **SOFT** dynamics (see Risk 1). |
| **Hymn** | Hymn | low/mid-A / high-V | <~0.30 | ≥~0.50 | **60–92** | strongly major (Ionian) | legato-weighted ≈0.95–1.0 | chordal/homophonic block harmony, frequent strong/plagal cadences, suppressed chromaticism, little swell. |
| **Nocturne** | Nocturne | low-A / V-neutral-to-+ | <~0.30 (calm-mid V) | mid | **50–80** | minor-lean or warm major | very legato ≈1.10 | slow harmony / fast Alberti accompaniment (the figured-bed virtue), pronounced messa-di-voce, intimate level. |

**Reserve (documented, not in the initial ladder; existing enum variants; add as JSON rows when
the ear wants them):** **Waltz** (mid-A/+V, lilting 3/4), **Lilt** (mid-A/+V softer dance),
**Gigue** (high-A/+V compound-feel dance — a Scherzo neighbor), **Drone** (very low-A/neutral-V,
static pedal). These are real enum variants with craft bundles in the music-craft design; they are
held out of v1 to keep the initial calibration small and the owner's ear unburdened, and added
later purely as `character` SelectTable rows + `character_tempo` windows.

**Affect→character SelectTable (first-match-wins, Ballad as the safe catch-all default):**

| Order | Rule (predicates AND'd, over the `arousal`/`valence` knobs) | Picks | Corner |
|---|---|---|---|
| 1 | `arousal ge 0.60` AND `valence ge 0.55` | **scherzo** | high-A / high-V |
| 2 | `arousal ge 0.60` AND `valence lt 0.45` | **march** | high-A / low-V |
| 3 | `arousal le 0.30` AND `valence lt 0.35` | **lament** | low-A / low-V |
| 4 | `arousal le 0.30` AND `valence ge 0.50` | **hymn** | low-A / high-V |
| 5 | `arousal le 0.35` AND `valence in_range 0.35..0.50` | **nocturne** | low-A / mid-V |
| (default) | — | **ballad** | central calm-mild-V |

Thresholds are a principled STARTING calibration — the owner's ear is the gate (Risk 4). The
ladder is intentionally non-exhaustive so any unclassified image degrades to the safe Ballad
default, never to a wrong strong character. (The affect design's and engine design's ladders
differed slightly on threshold numbers and on whether to use a mid-arousal Waltz/Lilt rung; this
reconciled ladder takes the affect design's authoritative composite and the engine design's
clean rung structure, drops the mid-arousal rung from v1 with Waltz/Lilt, and adds the Hymn/
Nocturne low-arousal rungs that distinguish the calm-positive and calm-intimate corners. All six
picks map to existing enum variants.)

### Decision 3 — THE COMPOSITE: affect-design formula, engine-design threading (PINNED, one mismatch noted)

**The formula (the affect design is authoritative).** Two scalars in [0,1], 0.5 neutral. Inputs
read from existing `ImageUnderstanding` fields; the two HSV scalars (`avg_saturation`,
`avg_brightness`) are 0..100 and divided by 100; the rest are already 0..1.

```
arousal = 0.45*s + 0.25*c + 0.20*e + 0.10*x          // s=sat/100, c=colorfulness, e=edge_activity, x=complexity
        clamp(0,1)                                    // saturation DOMINANT; monotone in every input (no inverted-U)

valence = 0.70*b + 0.20*s + 0.10*fluency             // b=brightness/100, fluency = 0.5 + 0.5*fg_bg_contrast
        clamp(0,1)                                    // brightness DOMINANT; valence OWNS major/minor, NOT hue
```

`texture`, `quadrant_contrast`, and the `*_energy` saliency triplet are deliberately NOT pooled
into macro affect (texture triple-counts busyness; quadrant_contrast is a form signal; the energy
triplet is the saliency/role signal reserved for Slice B). `dominant_hue` is excluded from valence
(warm=happy is culturally contingent and sign-unstable); hue survives only as modal *flavor* within
the valence-selected major/minor family.

**The threading (the engine design realizes the formula — confirmed correct, ONE mismatch to fix).**
The engine design's mechanism is the right one and matches the affect design's recommended option A:

- `affect_composite(u: &ImageUnderstanding, weights: &AffectMappings) -> Affect` — one new pure fn
  in `composition.rs` (the planner module — affect is a composition concern, not a pixel concern,
  so NOT in `pure_analysis.rs`), returning `struct Affect { arousal: f32, valence: f32 }`.
- Two new `Knob` variants `Arousal`/`Valence` whose `Knob::read` arms return planner-filled
  runtime-only fields on `ImageUnderstanding` (`affect_arousal`/`affect_valence`), set to a
  `-1.0` sentinel by `neutral()` and `understand_image_pure` and overwritten by the planner before
  the ladder runs — the **exact S20 `figuration_resolved` `#[serde(skip)]`-filled-by-planner
  precedent.** This keeps `Predicate`/`SelectTable` byte-unchanged.

**The one mismatch to fix — composite weights.** The affect design's authoritative arousal weights
are `{s:0.45, c:0.25, e:0.20, x:0.10}` and valence `{b:0.70, s:0.20, fluency:0.10}`. The engine
design's illustrative `affect.arousal_weights`/`valence_weights` JSON block uses DIFFERENT numbers
(`edge_activity:0.35, complexity:0.20, avg_saturation:0.15, avg_brightness:0.20, subject_energy:0.10`
for arousal; a `value_key:-0.20` term for valence). **The engine block was explicitly illustrative
("the weight NUMBERS… are the affect design's to set"); the affect design's numbers are
authoritative.** The reconciled JSON ships the affect design's weights, with one schema note: the
affect formula's `fluency` term is derived (`0.5 + 0.5*fg_bg_contrast`), so it is computed inside
`affect_composite`, not expressed as a raw-field weight — the valence_weights JSON carries
`{avg_brightness:0.70, avg_saturation:0.20, fg_bg_contrast:0.10}` and `affect_composite` applies
the `0.5 + 0.5*` fluency transform to the `fg_bg_contrast` term. (The engine design's `value_key`
term is dropped — the affect design uses brightness, not value_key, as the valence driver.)

### Decision 4 — MAPPINGS.JSON WRITE DISCIPLINE: single-writer per slice (PINNED)

`assets/mappings.json` is a single-writer file shared by the affect rows, the craft rows, and the
existing harmonic tables. Neither design commits it directly. **Per-slice single-writer plan:**

- **Slice A owns these rows** (affect lane): the de-capped `brightness_to_tempo_bpm` top anchor,
  the `affect` block (`arousal_weights`, `valence_weights`, `character_tempo` windows), and the
  filled `character` SelectTable. All land as `#[serde(default)]` additive blocks — an old
  `mappings.json` with no `affect` key parses to `AffectMappings::default()` (which ships the
  legacy `ballad:{56,96}` window, see Decision 3 / engine §2.4) and stays byte-identical.
  **Writer: the Rust Implementer on Slice A**, after the Rust Architect's buildable spec pins the
  exact rows; the Music Theory Specialist supplies the per-character tempo-window NUMBERS and the
  character craft bundles as a *file-disjoint input doc* (not a direct edit), which the Implementer
  transcribes. One writer, one commit.
- **Slice B owns these rows** (craft lane): the `prominence_catalogue` and the `prominence`
  SelectTable, both `#[serde(default)]`. **Writer: the Rust Implementer on Slice B.** The Music
  Theory Specialist supplies the prominence weight numbers file-disjointly.
- **Non-overlap guarantee:** Slice A touches `brightness_to_tempo_bpm`, `affect`, `character`;
  Slice B touches `prominence`/`prominence_catalogue`. Disjoint key sets — no clobber even if the
  slices run in parallel. Each slice's mappings.json edit goes through the Quality Gate's
  backward-compat + value-range check.
- **The Rust code change** (the two `Knob` variants + the `Affect` struct + the sentinel fields +
  `character_tempo_bpm` + the prominence types) is owned by the slice's Rust Implementer, file
  `composition.rs`/`mapping_loader.rs`, disjoint from the Music Theory Specialist's
  `chord_engine.rs` craft work (Slice B) — see the per-slice cadence in §3.

---

## 2. Consolidated interface surface (signatures; engine-design shapes corrected by reconciliations)

These pull the engine design's signatures forward, corrected for Decisions 1–3 (Scherzo not
Jubilee; affect-design weights). No bodies; binding shapes.

### 2.1 The affect composite (`composition.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affect { pub arousal: f32, pub valence: f32 }   // each 0..1, 0.5 neutral

/// Pure. Weighted blend of EXISTING ImageUnderstanding scalars under JSON weights.
/// Arousal: 0.45*sat/100 + 0.25*colorfulness + 0.20*edge_activity + 0.10*complexity.
/// Valence: 0.70*brightness/100 + 0.20*sat/100 + 0.10*(0.5 + 0.5*fg_bg_contrast).
/// Computed ONCE per plan in composition.rs. No pixels, no RNG, no clock.
fn affect_composite(u: &ImageUnderstanding, weights: &AffectMappings) -> Affect;
```

### 2.2 Affect knobs + sentinel fields (`composition.rs`) — S20 `figuration_resolved` precedent

```rust
pub enum Knob { /* …existing… */ Arousal, Valence }        // serde snake_case; reject unknown

// Knob::read gains exactly:
//   Knob::Arousal => u.affect_arousal,
//   Knob::Valence => u.affect_valence,

pub struct ImageUnderstanding {
    // …existing fields UNCHANGED…
    pub affect_arousal: f32,   // planner-filled; neutral()/understand_image_pure set -1.0 sentinel
    pub affect_valence: f32,   // (NOT serde-deserialized; pixel producer never writes a real value)
}
// ImageUnderstanding::neutral() sets both to -1.0 (a Ge/Gt rule never fires on an unfilled composite).
```

### 2.3 Per-character tempo window + de-cap (`composition.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct CharacterTempo { pub bpm_min: f32, pub bpm_max: f32 }

/// Clamp raw brightness→BPM into the selected character's window; absent window → no clamp.
/// Replaces the hard `bpm.clamp(BALLAD_BPM_MIN, BALLAD_BPM_MAX)` at composition.rs:727.
fn character_tempo_bpm(raw_bpm: f32, character: Character, affect: &AffectMappings) -> f32;

// Planner tempo block (composition.rs:725–728) becomes:
//   let raw_bpm = interp_tempo_bpm(&mappings.global.brightness_to_tempo_bpm, u.avg_brightness);
//   let bpm     = character_tempo_bpm(raw_bpm, character, &self.plan_mappings.affect);
//   let base_ms_per_step = (60_000.0 / bpm.max(1.0)).round() as u64;
// BALLAD_BPM_MIN/MAX consts at :851–852 are DELETED (their value moves to affect.character_tempo.ballad).
// AffectMappings::default() ships the SINGLE ballad:{56,96} window so the no-affect-block path is bit-stable.
```

### 2.4 The prominence representation (`composition.rs`) — resolved-only, S20 `#[serde(skip)]` precedent

```rust
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct LayerProminence { pub role: LayerRole, pub weight: f32 }   // weight 0..1; 0.5 neutral

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ProminenceProfile { pub id: String, pub layers: Vec<LayerProminence> }

pub struct OrchestrationProfile {
    // …id, layers, density, pad_voices, figuration, figuration_resolved UNCHANGED…
    #[serde(skip)] pub prominence: Vec<LayerProminence>,   // planner-filled; empty == uniform/identity
}
// OrchestrationProfile::identity() sets prominence: Vec::new(); is_identity() UNCHANGED
// (keys on pad_voices==0 && layers.is_empty()).

fn lookup_prominence<'a>(catalogue: &'a [ProminenceProfile], id: &str) -> Option<&'a ProminenceProfile>;
```

### 2.5 The realizer consumption (`chord_engine.rs`) — no signature change

```rust
/// Prominence weight (0..1) for `role`, off ctx.section.orchestration.prominence.
/// Returns PROMINENCE_NEUTRAL (0.5) when prominence is EMPTY or the role is unlisted →
/// the legacy realization is byte-identical when prominence is absent. Pure.
fn prominence_weight(ctx: &crate::composition::StepContext, role: OrchestralRole) -> f32;

// Applied as CENTERED nudges, each exactly 0 at w==0.5 (the freeze property):
//   velocity: + (w-0.5)*PROMINENCE_VEL_SPAN   (after the existing contour; clamp 1..=127)
//   register: + (w-0.5)*PROMINENCE_REG_SPAN   (folded into role_pitch bright_octaves; bass exempt)
//   rhythm:   shift the Melody/CounterMelody edge_activity band thresholds by the centered term
// New consts PROMINENCE_NEUTRAL=0.5, PROMINENCE_VEL_SPAN, PROMINENCE_REG_SPAN live in chord_engine.
// realize_step PUBLIC SIGNATURE FROZEN; the weight reaches realize_velocity/realize_rhythm/role_pitch
// via the already-blessed additive-private-param route (the pad_voices/ctx precedent).
```

### 2.6 mappings.json blocks (the reconciled data — affect-design weights, Scherzo not Jubilee)

```jsonc
// (a) de-cap brightness_to_tempo_bpm — raise the top anchor (secondary tempo input once arousal leads)
"brightness_to_tempo_bpm": { "0-30": 72, "31-70": 108, "71-100": 150 },

// (b) the affect block (Slice A) — AUTHORITATIVE affect-design weights
"affect": {
  "arousal_weights": { "avg_saturation": 0.45, "colorfulness": 0.25, "edge_activity": 0.20, "complexity": 0.10 },
  "valence_weights": { "avg_brightness": 0.70, "avg_saturation": 0.20, "fg_bg_contrast": 0.10 },
  "character_tempo": {
    "ballad":   { "bpm_min": 56,  "bpm_max": 96  },
    "scherzo":  { "bpm_min": 120, "bpm_max": 168 },
    "march":    { "bpm_min": 96,  "bpm_max": 132 },
    "lament":   { "bpm_min": 44,  "bpm_max": 66  },
    "hymn":     { "bpm_min": 60,  "bpm_max": 92  },
    "nocturne": { "bpm_min": 50,  "bpm_max": 80  }
  }
}

// (c) the filled character ladder (Slice A) — every pick is an existing enum variant
"character": {
  "default": "ballad",
  "rules": [
    { "when": [ {"knob":"arousal","op":"ge","lo":0.60,"hi":0.0}, {"knob":"valence","op":"ge","lo":0.55,"hi":0.0} ], "pick": "scherzo" },
    { "when": [ {"knob":"arousal","op":"ge","lo":0.60,"hi":0.0}, {"knob":"valence","op":"lt","lo":0.45,"hi":0.0} ], "pick": "march" },
    { "when": [ {"knob":"arousal","op":"le","lo":0.30,"hi":0.0}, {"knob":"valence","op":"lt","lo":0.35,"hi":0.0} ], "pick": "lament" },
    { "when": [ {"knob":"arousal","op":"le","lo":0.30,"hi":0.0}, {"knob":"valence","op":"ge","lo":0.50,"hi":0.0} ], "pick": "hymn" },
    { "when": [ {"knob":"arousal","op":"le","lo":0.35,"hi":0.0}, {"knob":"valence","op":"in_range","lo":0.35,"hi":0.50} ], "pick": "nocturne" }
  ]
}

// (d) the prominence vocabulary (Slice B) — disjoint keys from Slice A
"prominence_catalogue": [
  { "id": "uniform",        "layers": [] },
  { "id": "subject_melody", "layers": [
      { "role": "Melody", "weight": 1.0 }, { "role": "CounterMelody", "weight": 0.6 },
      { "role": "HarmonicFill", "weight": 0.4 }, { "role": "Pad", "weight": 0.3 }, { "role": "Bass", "weight": 0.5 } ] }
],
"prominence": {
  "default": "uniform",
  "rules": [
    { "when": [ {"knob":"subject_size","op":"in_range","lo":0.05,"hi":0.55},
                {"knob":"fg_bg_contrast","op":"ge","lo":0.25,"hi":0.0} ], "pick": "subject_melody" }
  ]
}
```

The `mapping_loader.rs` mirror obligation (engine design Appendix): every new `PlanMappings` field
(`affect`, `prominence`, `prominence_catalogue`) must also be added to `CompositionMappings` with
`#[serde(default)]` AND mapped in `From<CompositionMappings> for PlanMappings`; the `#[serde(skip)]`
resolved fields (`prominence`, the affect sentinels) are NOT in `CompositionMappings`.

---

## 3. THE STAGED BUILD PLAN (the heart — independently-shippable, independently-HEARABLE)

Each slice builds, tests headless, is HEARABLE on `example.jpg`, and keeps `engine_equivalence`
green by an explicit byte-freeze argument. For each slice the specialist cadence is:
**Rust Architect (buildable spec) → Rust Implementer ∥ Music Theory Specialist (file-disjoint) →
Test Engineer → Quality Gate LAST.**

### Slice A — affect→character + tempo de-cap  *(RECOMMENDED S22 FIRST SLICE)*

- **Files touched:** `composition.rs` (the `Affect` struct + `affect_composite` +
  `character_tempo_bpm` + 2 `Knob` variants + 2 `ImageUnderstanding` sentinel fields + replace the
  `:727` clamp + delete `BALLAD_BPM_*`); `mapping_loader.rs` (`AffectMappings` mirror,
  `#[serde(default)]`); `assets/mappings.json` (the §2.6 (a)(b)(c) blocks); `pure_analysis.rs`
  MINIMALLY (set the two affect sentinel fields in `understand_image_pure` + `neutral()`).
  **`chord_engine.rs` and `engine.rs` are NOT touched** — character drives TEMPO only in Slice A
  (the per-character articulation bias is deferred to Slice C so no realizer/engine code moves).
- **Byte-freeze argument (one line):** the entire change is on the compose path
  (`CompositionPlanner::plan`) plus additive `ImageUnderstanding`/`Knob` surface that the
  equivalence net never constructs, and `AffectMappings::default()` ships the legacy `ballad:{56,96}`
  window so a no-`affect`-block mapping is bit-identical — **no realizer change, so goldens
  240/114/84/36/79 cannot move.** (The net hand-builds `Section`/`StepContext` and passes an
  explicit `MS_PER_STEP=200`, so the planner's tempo derivation is off its path entirely.)
- **Property tests to add:**
  1. `affect_absent_block_keeps_ballad_window` — a `mappings.json` with no `affect` key →
     `AffectMappings::default()` → a bright image's compose-path `base_ms_per_step` equals the OLD
     `clamp(56,96)` result.
  2. `affect_composite_monotone` — arousal strictly increases as saturation increases (others
     fixed); valence strictly increases as brightness increases.
  3. `bright_energetic_picks_scherzo` — an `ImageUnderstanding` with high sat/colorfulness/edge →
     character ladder returns `Scherzo`; a dark/calm one returns `Lament`/`Nocturne`; a neutral one
     returns `Ballad` (the default).
  4. `scherzo_tempo_exceeds_ballad_cap` — for a bright image, the selected BPM exceeds the old 96
     ceiling (reaches the Scherzo window); for a dark image, BPM goes below the old 56 floor
     (Lament window).
  5. `git diff HEAD -- src/engine.rs src/chord_engine.rs tests/engine_equivalence.rs` EMPTY.
- **What the owner hears differently:** *`example.jpg` stops being a ballad — it comes out fast
  (~150 BPM), major, energetic; a dark calm image comes out slower and minor; two affect-distinct
  images now sound categorically different in tempo and character, not just in note detail.*
- **Cadence:** Rust Architect pins the exact `mappings.json` rows + the `composition.rs`/
  `mapping_loader.rs` signatures → **Rust Implementer** (composition.rs/mapping_loader.rs/
  mappings.json/pure_analysis sentinels) ∥ **Music Theory Specialist** (file-disjoint input doc:
  the per-character tempo-window numbers + confirms the affect-ladder thresholds against a trained
  ear — no source edit) → **Test Engineer** (the five tests above) → **Quality Gate LAST**
  (engine_equivalence 9/9 green + mappings.json backward-compat + value-range check).

### Slice B — saliency → role prominence  *(the witnessed realizer change)*

- **Files touched:** `composition.rs` (`LayerProminence`/`ProminenceProfile` + `prominence`/
  `prominence_catalogue` on `PlanMappings` + `lookup_prominence` + the resolve block +
  `OrchestrationProfile.prominence` `#[serde(skip)]` field + `identity()` literal);
  `mapping_loader.rs` (mirror fields); `assets/mappings.json` (§2.6 (d) — disjoint keys from
  Slice A); `chord_engine.rs` (the `prominence_weight` helper + the three centered nudges + new
  consts). `engine.rs` NOT touched.
- **Byte-freeze argument (one line):** under identity, `OrchestrationProfile::identity()` carries
  empty `prominence` → `prominence_weight` returns `0.5` → every nudge is `(0.5-0.5)*SPAN == 0.0`
  **exactly** → every emitted `NoteEvent` is bit-for-bit today's, independent of SPAN magnitudes;
  the prominence arms are reached only through a composed `subject_melody` profile, unreachable on
  the net (where `assign_role` delegates to `instrument_role`).
- **Property tests to add:**
  1. `prominence_neutral_is_byte_identical` — realize Melody/Bass/cadence under (a) `identity()`
     and (b) the same profile with every role at `weight:0.5`; the two `Vec<NoteEvent>` are equal.
  2. `high_saliency_melody_louder` — strong-subject image: `vel(Melody) - vel(Pad) > baseline_gap`.
  3. `high_saliency_melody_higher_wider` — `mean_pitch(Melody) > mean_pitch(Pad)` and the
     separation strictly larger at high prominence than at 0.
  4. `no_inversion_invariant` (the hard guard) — across ALL prominence values and all characters,
     `mean_pitch(Bass) < bed/fill < mean_pitch(Melody)` and every note ∈ 24..=108.
  5. `git diff HEAD -- src/engine.rs tests/engine_equivalence.rs` EMPTY.
- **What the owner hears differently:** *an image with a distinct, contrasting subject pushes the
  MELODY forward (louder, higher, rhythmically freer) while the background bed recedes (quieter,
  plainer) — the music tracks the subject, not the whole-image average; a subjectless field (like
  `example.jpg`) still realizes uniformly.*
- **Cadence:** Rust Architect pins the prominence types + the three centered-nudge seams + the
  resolve block → **Rust Implementer** (composition.rs/mapping_loader.rs/mappings.json planner +
  resolution) ∥ **Music Theory Specialist** (the `chord_engine.rs` realizer nudges + the SPAN
  const magnitudes — file-disjoint from the Implementer's planner work) → **Test Engineer** (the
  five tests, esp. the no-inversion sweep) → **Quality Gate LAST**.

### Slice C — per-character realization presets  *(follow-on, optional, deepens Slice A)*

- **Files touched:** `chord_engine.rs` (generalize the existing `BALLAD_ARTIC_BIAS` seam to a
  per-character bias; per-character rhythm-band shift + harmonic-palette lean), threading character
  via the additive-private-param route OR — only if needed — an operator-gated `StepContext.character`
  additive field (the one narrowly-scoped, witnessed `engine.rs`/`StepContext` touch, per the S13
  discipline). `assets/mappings.json` (per-character preset rows: the `CharacterPreset` scalar
  bundle from the music-craft design — artic_bias, rhythm_band_shift, dynamic_level_bias,
  swell_scale, chromaticism_scale, texture_id/figuration_id, per character).
- **Byte-freeze argument (one line):** the per-character bias resolves to the **Ballad value**
  (today's `1.0` const) on the legacy/identity path (the flat path's implicit character is Ballad)
  → byte-identical; the new `StepContext.character` field, if taken, has neutral default
  `Character::Ballad` and is witnessed in-slice.
- **Property tests to add:** `scherzo_denser_than_ballad` (`density(Scherzo) > density(Ballad)`),
  `scherzo_shorter_notes` (`mean_hold_frac(Scherzo) < mean_hold_frac(Ballad)`, `March ≤ Scherzo`),
  `character_changes_2plus_dims` (any two characters differ in ≥2 of {bpm, hold_frac, onset-dist,
  level, mode-lean}), `hymn_more_consonant` (chromaticism_scale<1 reduces borrowed-chord count),
  `ballad_identity_reproduces_goldens` (the freeze), `cadence_ring_byte_stable`.
- **What the owner hears differently:** *each character gains its theory-grounded fingerprint — a
  Scherzo is light and detached and dense, a Nocturne legato over an Alberti bed, a March crisp and
  marcato — beyond tempo alone.*
- **Cadence:** Rust Architect (the `CharacterPreset` bundle shape + the bias-generalization seam +
  the gated `StepContext.character` decision) → **Rust Implementer** (the threading + mappings.json
  rows) ∥ **Music Theory Specialist** (the `chord_engine.rs` per-character realization + the bundle
  numbers) → **Test Engineer** → **Quality Gate LAST**.

**Sequencing rationale.** Slice A is the cheapest decisive win — data + planner only, no
realizer/engine touch, the literal "bright image sounds fast/major" fix — and de-risks everything
by proving the affect ladder before any frozen-kernel code moves. Slice B is the larger music-craft
system and carries the one freeze-critical realizer change (fully witnessed by the centered-nudge
zero property). Slice C is the depth pass once both axes are audible. A and B are file-disjoint
enough to parallelize (A is planner/data; B adds the realizer nudges on disjoint mappings keys),
but **A-first is recommended** so the owner hears the affect win immediately.

---

## 4. Validation against the owner's ear

The owner has a trained ear (trombone, music-performance degree). The mapping is falsifiable by
listening. After **Slice A**, here is the concrete predicted output for three contrasting images so
the owner can confirm or reject the mapping by ear.

### `example.jpg` — bright, highly-saturated, chaotic, subjectless mosaic painting

Expected feature reads (qualitative, confirm by running the analyzer): `s≈0.80`, `c≈0.90`,
`e≈1.0` (dense strokes saturate edge_activity), `x≈0.75`, `b≈0.62`, `fg_bg_contrast≈0.15` (no
subject).

```
arousal ≈ 0.45*0.80 + 0.25*0.90 + 0.20*1.0 + 0.10*0.75 ≈ 0.86   (HIGH)
valence ≈ 0.70*0.62 + 0.20*0.80 + 0.10*(0.5+0.5*0.15)  ≈ 0.65   (HIGH-ish)
```

- **Character:** `arousal 0.86 ≥ 0.60` AND `valence 0.65 ≥ 0.55` → **Scherzo**.
- **BPM:** `target = 52 + 0.86*(168-52) ≈ 152`, clamped to the Scherzo window 120–168 → **~152 BPM**
  (vs today's ≤96). A ~1.6× speedup into genuinely-fast territory.
- **Mode:** valence 0.65 → **major / Ionian**, consonant. Joyful, not somber.
- **Density (after Slice A):** unchanged at the realizer (Slice A is tempo-only), but the *faster
  tempo* alone makes it audibly more energetic; the dense onsets arrive with Slice C's
  Scherzo rhythm-band shift. **Headline:** the lifeless ballad is gone — it is fast and major.

### A calm dark image — e.g. a dim, low-saturation night photograph

Expected: `s≈0.15`, `c≈0.20`, `e≈0.25`, `x≈0.30`, `b≈0.20`, `fg_bg_contrast≈0.40`.

```
arousal ≈ 0.45*0.15 + 0.25*0.20 + 0.20*0.25 + 0.10*0.30 ≈ 0.20   (LOW)
valence ≈ 0.70*0.20 + 0.20*0.15 + 0.10*(0.5+0.5*0.40)  ≈ 0.24   (LOW)
```

- **Character:** `arousal 0.20 ≤ 0.30` AND `valence 0.24 < 0.35` → **Lament**.
- **BPM:** `target = 52 + 0.20*116 ≈ 75`, clamped to Lament 44–66 → **~66 BPM** (slower than even
  today's 56 floor). **Mode:** valence 0.24 → **minor**. **What the owner hears:** slow, dark,
  minor — a dirge, distinct from `example.jpg`'s fast major.

### A mid image — e.g. a softly-lit pastoral landscape, moderate saturation, a clear subject

Expected: `s≈0.45`, `c≈0.40`, `e≈0.45`, `x≈0.40`, `b≈0.55`, `fg_bg_contrast≈0.55`.

```
arousal ≈ 0.45*0.45 + 0.25*0.40 + 0.20*0.45 + 0.10*0.40 ≈ 0.43   (MID)
valence ≈ 0.70*0.55 + 0.20*0.45 + 0.10*(0.5+0.5*0.55)  ≈ 0.55   (MID-HIGH)
```

- **Character:** arousal 0.43 fires no `ge 0.60` or `le 0.30/0.35` rule → falls to **Ballad**
  (the safe default). **BPM:** `target = 52 + 0.43*116 ≈ 102`, clamped to Ballad 56–96 → **~96 BPM**.
  **Mode:** valence 0.55 → **major**. **What the owner hears:** a gentle, pleasant, mid-tempo major
  ballad — the safe center, correctly NOT forced into a strong character. (When Slice B ships, this
  image's clear subject — `fg_bg_contrast 0.55` — additionally foregrounds a melody over a recessive
  bed.)

**One-line acceptance test:** *`example.jpg` → Scherzo, ~150 BPM, major; calm-dark → Lament, ~66
BPM, minor; pastoral-mid → Ballad, ~96 BPM, major.* If the owner's ear agrees the first is fast and
joyful, the second slow and sad, the third gentle, the mapping works. The first knobs to turn if
not: the arousal/valence weights (§2.6b), the ladder thresholds (Decision 2), the per-character
tempo windows (§2.6b).

---

## 5. Risks / trade-offs

1. **REGISTER-SEPARATION INVARIANT (the music-craft design's flagged worry — the one most at
   risk).** Three forces push the melody octave UP and can stack: a Scherzo character's high
   register, a bright image's `role_pitch` `bright_octaves` lift, and (Slice B) the saliency `+`
   foreground lift. If they stack naively the melody clamps at the top of 24..=108 and *flattens*
   (loses range — the very "lifeless" symptom), or a future "lower the bed for contrast" build
   could invert figure-ground. **Mitigation (pinned in this design):** lift the melody, NEVER lower
   the bed; clamp the SUM of all lifts, not each independently; saliency widens by raising the
   foreground only. **Enforced by Slice B's `no_inversion_invariant` sweep test** —
   `mean_pitch(Bass) < bed/fill < mean_pitch(Melody)` across all characters and all prominence
   values, with the melody retaining measurable range at maximum stacked lift. Any build touching
   the melody register MUST run it.
2. **MUSICAL fear/sadness = SOFT, not loud.** The arousal→loudness law is monotone, so a naïve
   high-arousal-minor image would read as anger by default. **Lament must override loudness to soft
   by its character bundle** (Slice C), not by the generic arousal→loudness law. Flagged so the
   music-craft realization does not let arousal→loud win for the sad corner.
3. **Valence owns mode, not hue (load-bearing).** Only major/minor is empirically validated;
   `dominant_hue` is excluded from valence and from the major/minor choice, surviving only as modal
   *flavor* within the valence-selected family. Getting this wrong reintroduces the contested
   warm=happy rule as a control axis. Integration point with the music-craft writer who owns
   `hue_to_mode`.
4. **The numbers are seed values, tuned by ear.** The arousal/valence weights, the ladder
   thresholds, and the per-character BPM windows are a principled STARTING calibration; the
   *directions and ordering* are from the literature, the *exact numbers* are seeds. The owner's
   ear is the gate.
5. **`edge_activity` saturates** at `edge_density ≥ 0.05`, so a very busy painting pins it at 1.0
   and it loses discrimination among already-busy images — saturation/colorfulness carry the
   gradient there. Acceptable; not a bug.
6. **Composite-weight mismatch resolved (Decision 3).** The engine design's illustrative JSON
   weights are superseded by the affect design's authoritative numbers; the build must ship the
   affect-design weights, with `fluency` computed inside `affect_composite` (not a raw-field
   weight).
7. **mapping_loader mirror is easy to forget** (engine design Appendix): every new `PlanMappings`
   field must also land on `CompositionMappings` + the `From` impl, else the data is silently
   dropped at load. The `affect_absent_block_keeps_ballad_window` and a `prominence_round_trips`
   test are the witnesses.

---

## 6. Recommended S22 FIRST BUILD SLICE

**Slice A — affect→character + tempo de-cap.** Data + planner only (`composition.rs`,
`mapping_loader.rs`, `assets/mappings.json`, plus the two affect sentinel fields in
`pure_analysis.rs`); **`chord_engine.rs` and `engine.rs` untouched.** It is the biggest audible
affect win with the least risk — it fixes the literal "always a ballad" complaint by selecting
`Scherzo` and de-capping tempo, and it proves the affect ladder before any frozen-kernel code
moves. The energetic character is the **existing `Scherzo` enum variant — no enum edit.**
Byte-freeze in one line: the whole change is on the compose path with additive `Knob`/
`ImageUnderstanding` surface the equivalence net never constructs, and `AffectMappings::default()`
ships the legacy `ballad:{56,96}` window so a no-`affect`-block mapping is bit-identical — no
realizer change, goldens 240/114/84/36/79 cannot move. Predicted `example.jpg` output after it:
**Scherzo, ~152 BPM, major / Ionian, energetic** — the lifeless ballad gone.

*Design-only. No source, test, or asset modified by this document. Signatures are binding shapes;
bodies are deferred to the slice implementers. The parallel streams are referred to as "the affect
design", "the music-craft design", and "the engine design"; the build role titles
(Rust Architect, Rust Implementer, Music Theory Specialist, Test Engineer, Quality Gate) are the
domain titles already used in the committed S21 docs.*
