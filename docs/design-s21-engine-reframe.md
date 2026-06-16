# S21 — Engine / Data Reframe: carrying AFFECT (character expansion + tempo de-cap) and CRAFT (saliency→role prominence) on the byte-freeze

**Author role:** Rust Architect — DESIGN ONLY. This document modifies no source, test, or asset; it writes precise Rust signatures (no bodies) and `docs/` prose only. The single artifact this work produces is this file.
**Date:** 2026-06-15.
**Grounded against** the working tree: `src/composition.rs` (1227 lines), `src/engine.rs` (1188 lines), `src/chord_engine.rs` (5156 lines), `src/pure_analysis.rs`, `assets/mappings.json`, re-verified line-by-line — not trusted from prior docs.
**Scope.** This is the ENGINE/DATA REFRAME that carries two parallel design streams without breaking the hard-won byte-freeze:
- the **affect design** — image → valence/arousal → character knobs, removing the tempo cap, adding character presets;
- the **music-craft design** — what each character is in theory terms + a saliency → musical-role system where the salient subject drives MELODY and recessive regions drive ACCOMPANIMENT.

This doc owns the data shapes, signatures, and migration that let both land as DATA (selected by deterministic first-match-wins `SelectTable` ladders over `mappings.json`), never as Rust constants, and proves each change is behaviour-neutral on the identity path that the equivalence net pins.

> Convention (carried from `composition-architecture-engine.md` / `spec-s20-slice3a-build.md`): Rust signatures give the SHAPE of each seam. No implementation bodies. New vocabulary lands as `mappings.json` rows parsed backward-compatibly via `#[serde(default)]`/`#[serde(skip)]`. No internal swarm/framework codenames anywhere; the other two streams are "the affect design" and "the music-craft design."

---

## 0. Executive summary (read first)

**What's wrong, in code.** `assets/mappings.json:134` pins `"character": { "default": "ballad", "rules": [] }` — an empty rule ladder, so `CompositionPlanner::plan` (`composition.rs:701`) always reads `parse_character("ballad") == Character::Ballad`. The `Character` enum (`composition.rs:177–188`) already *declares* ten variants, but only `Ballad` is reachable because no JSON rule ever selects another and nothing downstream reads the variant (the planner stores it on `CompositionPlan.character` at `:834` and the realizer never consults it). Tempo is then double-capped: `compute_key_tempo`-equivalent code at `composition.rs:725–728` computes `bpm = interp_tempo_bpm(...)` then **`bpm.clamp(BALLAD_BPM_MIN=56, BALLAD_BPM_MAX=96)`** (`:727`, consts at `:851–852`). A bright high-energy image whose brightness→BPM lands at 120 is clamped down to 96 and emitted as a ballad. The melody/accompaniment split is index-only (`instrument_role`, `chord_engine.rs:874`), blind to which image region is salient.

**The reframe, in one sentence.** Add the affect composite + character-driven tempo as DATA the existing `SelectTable`/`Predicate` machinery already understands, give each `Section` a per-layer **role-prominence** field the planner fills from saliency tiers and the realizer reads off the already-borrowed `StepContext`, and keep `single_section_default`'s identity profile (`OrchestrationProfile::identity()`) carrying neither a non-ballad character nor a non-uniform prominence — so the new behaviour is structurally unreachable on the equivalence net, exactly as S17's Pad arm and S20's figured arm are.

**The three load-bearing moves:**

1. **Character expansion as DATA (§2).** Fill the empty `character` `SelectTable.rules` over a new pair of composite knobs — `Arousal` and `Valence` — computed by one new pure fn `affect_composite(&ImageUnderstanding) -> Affect` in `composition.rs`. The composite WEIGHTS, the valence mapping, the character rules, and the de-capped tempo curve all live in a new `affect` block in `mappings.json`, parsed via `#[serde(default)]` so an old file still produces today's Ballad bit-for-bit. Per-character tempo windows replace the single hard `BALLAD_BPM_*` clamp.

2. **Saliency → role-prominence plumbing as DATA (§3).** Extend `OrchestrationProfile` with one additive `#[serde(default)]` `prominence: Vec<LayerProminence>` (resolved-only) field, filled by the planner from a `prominence` `SelectTable` over the saliency knobs (`subject_size`, `fg_bg_contrast`, `subject_energy`) that already exist on `ImageUnderstanding`. The realizer reads it off `ctx.section.orchestration.prominence` — no `realize_step` signature change (the frozen public signature already threads `ctx`; `realize_rhythm`/`realize_velocity` take it via the additive-private-param precedent already used for `pad_voices`/`ctx`). Image logic stays out of `chord_engine`: the planner resolves saliency→weights, the realizer consumes resolved scalars.

3. **Byte-freeze guarantee (§4).** `single_section_default` → `OrchestrationProfile::identity()` → `assign_role` delegates to `instrument_role` (returns only `Bass`/`HarmonicFill`/`Melody`); the character on the legacy `legacy_default_section` is `Ballad` and `affect`/`prominence` are absent/empty (identity sentinels). Every new read is behind an `is_identity()`/empty-vocabulary guard that the equivalence net never trips. Goldens **240 / 114 / 84 / 36 / 79** do not move; `realize_step` public signature is frozen; `engine.rs` is touched in exactly ONE narrowly-scoped, operator-gated way (§4.4) — and a zero-`engine.rs`-touch alternative is given.

**Recommended first hearable slice (§5):** **Slice A — affect→character + tempo de-cap.** It is the biggest audible affect win on `example.jpg` with the least risk: a bright energetic image finally sounds fast, major-leaning, and dense, because the character ladder selects a `Driving`/`Scherzo`-class row and the per-character tempo window lets BPM reach 120+. It touches only `composition.rs` (planner) + `mappings.json` (data) + `pure_analysis.rs` is **not** touched (the affect composite reads existing `ImageUnderstanding` fields). The saliency→role system (Slice B) is strictly larger and ships second.

---

# Part 1 — CURRENT STATE (where character/tempo/key are decided, with line refs)

## 1.1 The character decision — pinned to Ballad in three places

| Stage | File:line | Fact |
|---|---|---|
| The enum | `composition.rs:175–188` | `Character { Ballad, Hymn, Nocturne, Drone, March, Lament, Waltz, Scherzo, Lilt, Gigue }` — ten variants ALREADY declared, `#[derive(... serde::Deserialize)] #[serde(rename_all="PascalCase")]`. Only `Ballad` is ever produced. |
| The selector | `composition.rs:701` | `let character = parse_character(&self.plan_mappings.character.select(u));` — runs the `character` `SelectTable` over the `ImageUnderstanding`. |
| The parser | `composition.rs:918–933` | `parse_character(s)` maps each lowercase name to its variant; **unknown → `Ballad`**. All ten names are wired, so the parser is NOT the bottleneck. |
| The data | `mappings.json:134` | `"character": { "default": "ballad", "rules": [] }` — **the empty `rules` array is the actual pin.** `SelectTable::select` (`composition.rs:450–457`) returns `default` because no rule exists. |
| The store | `composition.rs:834` | `CompositionPlan { character, .. }` — the selected variant is stored… |
| The (non-)consumer | (none) | …and **never read by the realizer.** `chord_engine` knows `OrchestralRole`, `PerfFeatures`, `StepPlan`, and the orchestration profile, but nothing reads `plan.character`. So even if a non-Ballad were selected today, NO note would change. Character has no downstream wiring yet — §2 adds it via the per-character tempo window + (Slice-A-minimal) the articulation bias seam that already exists (`BALLAD_ARTIC_BIAS`, `chord_engine.rs:1225`). |

**Consequence:** character is a dead axis end-to-end. The fix is two-sided: (a) make the ladder SELECT a real variant from affect (§2.2/§2.3 — pure data), and (b) give the variant at least ONE audible consequence in Slice A (the per-character tempo window, §2.4) so the operator hears it, with the fuller per-character realization (articulation/rhythm/harmonic-palette presets) sequenced behind it.

## 1.2 The tempo decision — and exactly where the cap lives

`CompositionPlanner::plan` derives the tempo spine at `composition.rs:721–728`:

```text
721  let home_mode = lookup_range_map(&mappings.global.hue_to_mode, u.dominant_hue) …
724  let home_root_midi = 60;                              // C4 seed
725  let bpm = interp_tempo_bpm(&mappings.global.brightness_to_tempo_bpm, u.avg_brightness);
727  let bpm = bpm.clamp(BALLAD_BPM_MIN, BALLAD_BPM_MAX);  // ← THE CAP (56..=96)
728  let base_ms_per_step = (60_000.0 / bpm.max(1.0)).round() as u64;
```

- `interp_tempo_bpm` (`composition.rs:949–978`) is a faithful local copy of the engine helper (`engine.rs:787–816`): it linearly interpolates over the `brightness_to_tempo_bpm` anchors (`mappings.json:16–20`: `0-30→60, 31-70→90, 71-100→120`). A bright image legitimately produces **120 BPM**.
- `BALLAD_BPM_MIN = 56.0`, `BALLAD_BPM_MAX = 96.0` (`composition.rs:851–852`). The `.clamp(56,96)` at `:727` **throws away the bright image's 120 BPM**, capping every piece into the ballad window. This is the single line that makes a high-energy image lifeless.
- **The legacy flat path has NO such cap.** `engine.rs:set_features_global` (`:381–440`) computes `self.config.ms_per_step = (60_000.0/bpm).round()` at `:402` with NO clamp. So the cap is a *compose-path-only* artifact — meaning the equivalence net (which never runs the compose path; see §1.5) is already independent of it, and removing it cannot move a golden.

`KeyTempoPlan` (`composition.rs:536–548`) then carries `base_ms_per_step` + the all-equal `tempo_scheme` (`:830`); every `Section.ms_per_step` is `base_ms_per_step` (`:816`). Tempo is section-stable, planned once.

## 1.3 The key decision (for completeness — §2 leaves it untouched in Slice A)

`home_mode` is the hue→mode lookup (`:721–723`); `home_root_midi = 60` is a fixed seed (`:724`); `key_scheme` is all-zeros (`:829`, "home_only", `mappings.json:136`). Modulation is a later stage and OUT of this reframe's Slice A; §2.5 notes how a `Valence`-driven major/minor lean could later ride the SAME `key_scheme` `SelectTable` mechanism without new types.

## 1.4 `single_section_default` keeps the figured/character arms unreachable on `engine_equivalence`

The byte-freeze chain, verified end-to-end:

```text
StepContext::single_section_default(section, key_tempo)          composition.rs:651–662
  └─ theme: None, step_in_section: 0  (behaviour-neutral)
legacy_default_section(plan, ms_per_step, mode)                  engine.rs:745–763
  └─ orchestration: OrchestrationProfile::identity()             engine.rs:760
OrchestrationProfile::identity()                                 composition.rs:246–255
  └─ layers: Vec::new(), pad_voices: 0, figuration: None, figuration_resolved: None
is_identity() == (pad_voices == 0 && layers.is_empty())          composition.rs:259–261
assign_role(inst, num, ctx)                                      chord_engine.rs:918–935
  └─ if prof.is_identity() { return instrument_role(inst, num); }
instrument_role → only Bass | HarmonicFill | Melody              chord_engine.rs:874–887
```

So under the equivalence net's hand-built section (`tests/engine_equivalence.rs:97` literally constructs `orchestration: OrchestrationProfile::identity()`, `MS_PER_STEP = 200` passed explicitly at every `decide_instrument_action` call), the new `Pad`/`CounterMelody` arms are NEVER reached, and the explicit `MS_PER_STEP` means the planner's tempo derivation is never on the net's path at all. The goldens (`engine_equivalence.rs:124–135`): `MS_PER_STEP=200`, `G_BASS_NOTE=36`, `G_MELODY_NOTE=79`, cadence vel `114`/`84`, cadence hold `240` (= `round(200 * min(LEGATO_FRAC(0.95)*1.30, 1.20)) = round(240)`).

## 1.5 The equivalence net never runs the compose path

`engine_equivalence.rs` calls `decide_instrument_action` directly with a fixed `&[StepPlan]` and the default `ctx` (`:147,150` etc.). It never calls `compose_from_image` (`engine.rs:362`) or `CompositionPlanner::plan`. Therefore **everything in §1.1–§1.3 (character selection, tempo derivation, the cap) is OFF the net's path** — the same boundary `set_features_global`'s RNG `pick_progression` and S13's tempo edit already live behind. This is why the cap can be removed and character can be activated with the goldens provably unmoved (§4).

---

# Part 2 — CHARACTER EXPANSION (the affect stream's carrier)

The affect design needs: image → valence/arousal → character; remove the 120-cap; add per-character presets. This part defines the DATA shapes and the ONE new pure fn, all selected by the existing first-match-wins `SelectTable` ladder — no `thread_rng` in the choice (S15 discipline).

## 2.1 The composite — one new pure fn in `composition.rs`

The valence/arousal composite is computed in `composition.rs` (the planner's module) from the neutral `ImageUnderstanding` — NOT in `pure_analysis.rs` (that module owns pixels and must hold no music/affect semantics) and NOT in `chord_engine` (no image logic there). It is a pure function of the perceptual scalars already on `ImageUnderstanding`; it adds no field to `ImageUnderstanding` and no new `Knob` reader on pixels.

```rust
/// The affect composite — the image's valence/arousal coordinates, each 0..1, derived
/// purely from the perceptual scalars already on `ImageUnderstanding`. NO new image
/// extraction: a weighted blend of EXISTING knobs (energy/brightness/saturation/
/// complexity for arousal; brightness/saturation/value_key/colorfulness for valence).
/// Computed ONCE per plan in `composition.rs` (the planner's module — affect is a
/// composition concern, not a pixel concern). Deterministic, no RNG, no clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affect {
    /// Arousal / energy, 0 (calm/still) .. 1 (energetic/agitated).
    pub arousal: f32,
    /// Valence / mood, 0 (dark/tense) .. 1 (bright/pleasant).
    pub valence: f32,
}

/// Compute the affect composite from the image understanding under the JSON-loaded
/// weights. Pure; the weights are DATA (`mappings.json` `affect.arousal_weights` /
/// `affect.valence_weights`), so re-tuning the blend is a JSON edit, not a recompile.
/// When the `affect` block is absent (old mappings) the loader supplies the
/// `AffectMappings::default()` neutral weights (§2.3), and this still returns a valid
/// composite — but the character ladder will be empty, so the plan stays Ballad.
fn affect_composite(u: &ImageUnderstanding, weights: &AffectMappings) -> Affect;
```

**Why a struct, not two `Knob` variants:** `Affect` is a *derived* composite, not a raw `ImageUnderstanding` field; making it a `Knob` would force a pixel-level reader. Instead it is exposed to the `SelectTable` ladder by adding exactly two `Knob` variants whose `read` arm consults a composite the planner has already computed and stashed — see §2.2.

## 2.2 Threading affect into the `SelectTable` ladder — two new `Knob` variants

The character ladder must test `arousal`/`valence`. The cleanest carrier that keeps the existing `Predicate`/`SelectTable` machinery (one knob, one op, lo/hi) is to add the composite to the value the `Knob::read` arm sees. Two options, PIN option (a):

**(a) PINNED — carry the composite on a thin read-context.** The composite is computed once in `plan` (after `understand`-time fields are available) and the two new `Knob`s read it. Since `Knob::read(self, u: &ImageUnderstanding)` (`composition.rs:361`) takes only `&ImageUnderstanding`, and `Affect` is derived, the minimal change is to **store the composite on the understanding-adjacent read path** by giving the ladder a richer view. Concretely, add the two variants and route them through a derived accessor that the planner pre-seats:

```rust
/// Closed handle naming a selectable knob. NEW S21: `Arousal`/`Valence` name the
/// DERIVED affect composite (not a raw ImageUnderstanding field). serde rejects unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Knob {
    // … all existing variants (EdgeActivity … BackgroundEnergy) UNCHANGED …
    /// NEW S21 — the affect composite's arousal coordinate (0..1). Read from the
    /// composite the planner computed via `affect_composite`, not from a pixel field.
    Arousal,
    /// NEW S21 — the affect composite's valence coordinate (0..1).
    Valence,
}
```

To let `Knob::read` reach the composite without widening every call site, PIN the **affect-augmented understanding** approach: `affect_composite`'s result is written into two NEW `#[serde(skip)]`-style *runtime-only* fields on `ImageUnderstanding` that default to a sentinel and are filled by the planner before the character ladder runs. This mirrors the S20 `figuration_resolved` `#[serde(skip)]` precedent exactly — a planner-filled, not-deserialized field:

```rust
pub struct ImageUnderstanding {
    // … all existing fields UNCHANGED …
    /// NEW S21 — the planner-computed arousal composite (0..1). NOT extracted from
    /// pixels and NOT deserialized; `pure_analysis::understand_image_pure` leaves it at
    /// the `neutral()` sentinel (-1.0 == "not yet computed") and the planner overwrites
    /// it via `affect_composite` before running the character/tempo ladders. The
    /// `Knob::Arousal` read arm returns this. Keeping it off the pixel producer holds
    /// the module boundary: `pure_analysis.rs` writes the sentinel, never a real value.
    pub affect_arousal: f32,
    /// NEW S21 — the planner-computed valence composite (0..1). Same discipline.
    pub affect_valence: f32,
}
```

`Knob::read` gains exactly two arms:

```rust
Knob::Arousal => u.affect_arousal,
Knob::Valence => u.affect_valence,
```

`ImageUnderstanding::neutral()` (`composition.rs:93`) sets `affect_arousal: -1.0, affect_valence: -1.0` (a sentinel below any real 0..1 value, so a `Ge`/`Gt` rule reading an un-filled composite never spuriously fires — the same "default == condition not met" discipline the module already documents at `:35–37`). `understand_image_pure` (`pure_analysis.rs:738`) sets both to the sentinel too (it does NOT compute affect — boundary held).

**Why this over the alternative (b):** option (b) would add `&Affect` to `Predicate::holds`/`SelectTable::select` signatures, rippling through every existing call site and the S18/S20 tests. Option (a) is the **already-blessed** `#[serde(skip)]`-filled-by-planner pattern (S20 `figuration_resolved`); it touches `Knob`/`Knob::read`/`neutral`/`understand_image_pure` additively and leaves the entire `Predicate`/`SelectTable` surface byte-unchanged. The cost — two runtime-only `f32`s on `ImageUnderstanding` — is identical in spirit to `figuration_resolved` on `OrchestrationProfile`.

## 2.3 The `affect` block in `mappings.json` (NEW key, backward-compatible)

A new `affect` object inside the `composition` block carries the composite weights, the valence mapping, and (referenced by §2.4) the per-character tempo windows. It is parsed by a new `AffectMappings` serde struct on `PlanMappings`/`CompositionMappings`, both `#[serde(default)]` so an OLD `mappings.json` with no `affect` key parses to `AffectMappings::default()` (neutral weights, EMPTY character windows) → the character ladder is also empty (`character.rules: []` unchanged) → the plan stays Ballad → byte-identical to today.

```jsonc
"affect": {
  "arousal_weights": {
    "edge_activity": 0.35, "complexity": 0.20, "avg_saturation": 0.15,
    "avg_brightness": 0.20, "subject_energy": 0.10
  },
  "valence_weights": {
    "avg_brightness": 0.40, "avg_saturation": 0.25, "colorfulness": 0.15,
    "value_key": -0.20
  },
  "character_tempo": {
    "ballad":   { "bpm_min": 56,  "bpm_max": 96  },
    "nocturne": { "bpm_min": 50,  "bpm_max": 80  },
    "lament":   { "bpm_min": 44,  "bpm_max": 72  },
    "hymn":     { "bpm_min": 60,  "bpm_max": 92  },
    "drone":    { "bpm_min": 40,  "bpm_max": 66  },
    "waltz":    { "bpm_min": 84,  "bpm_max": 132 },
    "lilt":     { "bpm_min": 88,  "bpm_max": 132 },
    "march":    { "bpm_min": 96,  "bpm_max": 132 },
    "scherzo":  { "bpm_min": 120, "bpm_max": 176 },
    "gigue":    { "bpm_min": 116, "bpm_max": 168 }
  }
}
```

> The weight NUMBERS and the per-character windows are the **affect design's** to set; this doc fixes the SHAPE and pins the de-cap mechanism. `value_key` carries a negative weight in `valence_weights` because `value_key` is "toward dark" (`composition.rs:58`), so darker → lower valence. Weights are normalized at consumption (`affect_composite` clamps the blend to 0..1); a brightness-only fallback is the documented degenerate behaviour when all weights are zero.

The character ladder is then FILLED (this is the data that ends the Ballad pin):

```jsonc
"character": {
  "default": "ballad",
  "rules": [
    { "when": [ {"knob":"arousal","op":"ge","lo":0.70,"hi":0.0},
                {"knob":"valence","op":"ge","lo":0.55,"hi":0.0} ], "pick": "scherzo" },
    { "when": [ {"knob":"arousal","op":"ge","lo":0.70,"hi":0.0},
                {"knob":"valence","op":"lt","lo":0.55,"hi":0.0} ], "pick": "march" },
    { "when": [ {"knob":"arousal","op":"in_range","lo":0.45,"hi":0.70},
                {"knob":"valence","op":"ge","lo":0.55,"hi":0.0} ], "pick": "lilt" },
    { "when": [ {"knob":"arousal","op":"in_range","lo":0.45,"hi":0.70},
                {"knob":"valence","op":"lt","lo":0.40,"hi":0.0} ], "pick": "lament" },
    { "when": [ {"knob":"arousal","op":"lt","lo":0.30,"hi":0.0},
                {"knob":"valence","op":"ge","lo":0.50,"hi":0.0} ], "pick": "hymn" },
    { "when": [ {"knob":"arousal","op":"lt","lo":0.30,"hi":0.0},
                {"knob":"valence","op":"lt","lo":0.35,"hi":0.0} ], "pick": "nocturne" }
  ]
}
```

First-match-wins, falls to `"ballad"` for the central calm-mid-valence region — so the existing Ballad output is preserved for images that genuinely read as ballads, and only the affect-extreme images diverge. (Exact thresholds are the affect design's to tune; the bright-energetic `example.jpg` should land on `scherzo`/`march`.)

## 2.4 The de-capped, character-aware tempo curve

Replace the single hard clamp at `composition.rs:727` with a per-character window lookup. The planner reads the selected `character`'s window from `affect.character_tempo` and clamps to THAT window (de-capping ballad's 96 ceiling for energetic characters), with the absent-window fallback being **no clamp at all** (the legacy flat path's behaviour):

```rust
/// Per-character tempo window (bpm_min, bpm_max), loaded from `affect.character_tempo`.
/// An absent/zero window means "no clamp" — the raw brightness→BPM is used (the legacy
/// `set_features_global` behaviour, which never clamped). NEW S21.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct CharacterTempo {
    pub bpm_min: f32,
    pub bpm_max: f32,
}

/// Resolve the tempo BPM for a character: clamp the raw brightness→BPM into the
/// character's window if one is present, else return the raw BPM unclamped. Pure.
/// Replaces the hard `bpm.clamp(BALLAD_BPM_MIN, BALLAD_BPM_MAX)` at composition.rs:727.
fn character_tempo_bpm(raw_bpm: f32, character: Character, affect: &AffectMappings) -> f32;
```

The planner's tempo block (`composition.rs:725–728`) becomes (signature-level, no body):

```text
let raw_bpm   = interp_tempo_bpm(&mappings.global.brightness_to_tempo_bpm, u.avg_brightness);
let bpm       = character_tempo_bpm(raw_bpm, character, &self.plan_mappings.affect);
let base_ms_per_step = (60_000.0 / bpm.max(1.0)).round() as u64;
```

`BALLAD_BPM_MIN/MAX` (the Rust consts at `:851–852`) are **deleted** — their value moves into `affect.character_tempo.ballad` as DATA. When the `affect` block is absent, `character == Ballad` and the absent window → no clamp → the raw brightness BPM (which, with the default anchors topping at 120, is the SAME 56..120 the old code would have clamped to 56..96 only at the extremes). To make the absent-`affect`-block case BYTE-identical to today's compose path, `AffectMappings::default()` ships the SINGLE `ballad: {56,96}` window (not empty), so a mapping with no `affect` block but the old behaviour is preserved exactly. (A mapping that ships `affect` opts into the de-cap by providing the wider windows above.) This is the one subtlety the implementer must honor: **`AffectMappings::default()` carries the legacy Ballad window so the no-affect-block path is bit-stable; the de-cap arrives only with a populated `affect` block.**

## 2.5 What character drives in Slice A vs later (honest scoping)

- **Slice A (this reframe's first hearable slice):** character drives the **tempo window** (§2.4) — the single biggest audible affect lever — and rides the **already-present** `BALLAD_ARTIC_BIAS` seam (`chord_engine.rs:1225`) by generalizing it to a per-character bias READ FROM the plan. That generalization is the one place character touches `chord_engine`; it is additive and identity-guarded (§4.3). If even that is deemed too much for Slice A, the bias generalization is deferred and Slice A is tempo-window-only — still a decisive audible win.
- **Later slices:** per-character harmonic palette (triad/7th/9th lean), rhythm-pattern bias, meter selection (the `meter` `SelectTable` at `mappings.json:135` is already wired and empty, ready the same way), and a `Valence`-driven major/minor lean riding the existing `key_scheme` ladder. None require new engine seams — all are `mappings.json` rows + existing readers.

---

# Part 3 — SALIENCY → ROLE PROMINENCE PLUMBING (the music-craft stream's carrier)

The music-craft design wants the salient subject to drive MELODY and recessive regions to drive ACCOMPANIMENT, expressed as per-section/per-layer role PROMINENCE driven by saliency tiers. This part defines how prominence is represented and threaded to the realizer, keeping image logic out of `chord_engine`.

## 3.1 Representation — additive `prominence` on `OrchestrationProfile` (NOT a new `Section` field)

The existing carrier is `OrchestrationProfile` on every `Section` (`composition.rs:581`), already cloned per section (`:824`) and already read by the realizer via `ctx.section.orchestration` (`chord_engine.rs:923,968,1477`). Saliency-driven prominence rides it as an additive, resolved-only field — mirroring the S20 `figuration_resolved` precedent EXACTLY:

```rust
/// One layer's prominence weight for this section — the music-craft "who is foreground"
/// signal, resolved by the planner from saliency tiers. Pure data; the realizer reads
/// the weight, never the image. NEW S21.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct LayerProminence {
    /// Which layer this weight applies to (mirrors the profile's `layers` vocabulary).
    pub role: LayerRole,
    /// 0..1 prominence: 1.0 == fully foreground (the salient subject's voice — Melody
    /// gets register lift + dynamic gain + rhythmic freedom); 0.0 == fully recessive
    /// (background accompaniment — quieter, plainer rhythm, lower register bias). The
    /// realizer maps this onto bounded velocity/register/rhythm nudges (§3.3).
    pub weight: f32,
}
```

`OrchestrationProfile` (`composition.rs:208`) gains ONE additive `#[serde(skip)]` field, set by the planner — never deserialized, so `mappings.json` byte-shape is unchanged and `PartialEq`/`Clone` stay total:

```rust
pub struct OrchestrationProfile {
    // … id, layers, density, pad_voices, figuration, figuration_resolved UNCHANGED …
    /// NEW S21 — the RESOLVED per-layer prominence for this section, filled by the planner
    /// from the `prominence` SelectTable over saliency knobs (§3.2). NOT loaded from JSON
    /// (`#[serde(skip)]` → always empty at deserialize); the planner sets it. EMPTY ==
    /// the uniform/identity prominence sentinel: the realizer takes its byte-stable legacy
    /// path (every role at its existing register/dynamics). The realizer reads THIS.
    #[serde(skip)]
    pub prominence: Vec<LayerProminence>,
}
```

`OrchestrationProfile::identity()` (`:246`) sets `prominence: Vec::new()`. `is_identity()` (`:259`) is **UNCHANGED** — it already keys on `pad_voices == 0 && layers.is_empty()`; an empty `prominence` is implied by identity and a non-empty `prominence` only ever rides a non-identity (composed) profile. So identity detection — the byte-freeze anchor — is untouched.

> **Why `OrchestrationProfile`, not `Section`.** Two reasons: (1) it is already the cloned-per-section, already-borrowed-by-the-realizer carrier (`ctx.section.orchestration`), so NO new borrow and NO `StepContext` field; (2) prominence is conceptually part of the orchestration profile (it tells the realizer how to weight the layers the profile already names), so it co-locates with `layers`/`pad_voices`. A `Section.role_prominence` field would duplicate the borrow path and split orchestration data across two structs.

## 3.2 Resolution — a `prominence` `SelectTable` over the existing saliency knobs

The planner resolves saliency → weights as DATA, using the saliency knobs ALREADY on `ImageUnderstanding` and ALREADY in the `Knob` enum: `SubjectSize` (`:351`), `FgBgContrast` (`:352`), `SubjectEnergy` (`:353`), `ForegroundEnergy`, `BackgroundEnergy`. A new `prominence` `SelectTable` on `PlanMappings` picks a *prominence-profile id* from a new `prominence_catalogue`, exactly parallel to how `texture` picks a `texture_catalogue` id (`composition.rs:481,487`). This keeps the saliency→role decision in DATA and the realizer reading resolved scalars.

```rust
pub struct PlanMappings {
    // … form, character, meter, key_scheme, theme_behaviour, texture (#[serde(default)]),
    //     form_catalogue, texture_catalogue (#[serde(default)]),
    //     figuration_catalogue (#[serde(default)]) UNCHANGED …
    /// NEW S21 — the affect weights + per-character tempo windows (§2.3). `#[serde(default)]`
    /// so an old mappings.json parses → `AffectMappings::default()` (legacy Ballad window).
    #[serde(default)]
    pub affect: AffectMappings,
    /// NEW S21 — selects a `prominence_catalogue` id from the saliency knobs (§3.2).
    /// `#[serde(default)]` (empty SelectTable → "") so an old file falls back to uniform
    /// prominence (the byte-stable legacy realization).
    #[serde(default)]
    pub prominence: SelectTable,
    /// NEW S21 — the prominence-profile vocabulary (id → per-layer weights). Parallel to
    /// `texture_catalogue`/`figuration_catalogue`. `#[serde(default)]` empty Vec.
    #[serde(default)]
    pub prominence_catalogue: Vec<ProminenceProfile>,
}

/// One named prominence profile — pure structure, the per-layer weights a salient-subject
/// vs recessive-background reading produces. Selected by the `prominence` SelectTable;
/// the planner copies its `layers` onto the section's `OrchestrationProfile.prominence`.
/// Adding a profile is a JSON row, NOT a Rust edit (the FormSpec/figuration discipline).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct ProminenceProfile {
    pub id: String,
    pub layers: Vec<LayerProminence>,
}
```

The `mappings.json` rows (numbers are the music-craft design's to tune):

```jsonc
"prominence_catalogue": [
  { "id": "uniform",        "layers": [] },
  { "id": "subject_melody", "layers": [
      { "role": "Melody",       "weight": 1.0 },
      { "role": "CounterMelody","weight": 0.6 },
      { "role": "HarmonicFill", "weight": 0.4 },
      { "role": "Pad",          "weight": 0.3 },
      { "role": "Bass",         "weight": 0.5 } ] }
],
"prominence": {
  "default": "uniform",
  "rules": [
    { "when": [ {"knob":"subject_size",  "op":"in_range","lo":0.05,"hi":0.55},
                {"knob":"fg_bg_contrast","op":"ge","lo":0.25,"hi":0.0} ],
      "pick": "subject_melody" }
  ]
}
```

A real, distinct subject (small-to-mid area, high fg/bg contrast) → `subject_melody` (the melody is pushed forward, the bed recedes); a uniform field → the empty `uniform` profile → uniform prominence → byte-stable realization. The planner resolves once per plan at the texture-resolution point (`composition.rs:770–781`), immediately after the figuration resolve, into the section's profile:

```text
// after orchestration + figuration_resolved are set (composition.rs:770–781):
let prom_id = self.plan_mappings.prominence.select(u);
orchestration.prominence = lookup_prominence(&self.plan_mappings.prominence_catalogue, &prom_id)
    .map(|p| p.layers.clone())
    .unwrap_or_default();           // unknown/absent id → empty → uniform realization
```

with a finder mirroring `lookup_orchestration`/`lookup_figuration`:

```rust
fn lookup_prominence<'a>(catalogue: &'a [ProminenceProfile], id: &str) -> Option<&'a ProminenceProfile>;
```

`orchestration.clone()` at `:824` already deep-clones the `prominence` Vec onto each section.

## 3.3 Consumption — the realizer reads resolved weights off the borrowed `ctx` (no signature change)

The realizer takes the prominence weight for a role off `ctx.section.orchestration.prominence` and applies it as a BOUNDED nudge to the dimensions it already shapes. The image is never seen — only the resolved scalar. The seam is exactly one already-borrowed read, so **`realize_step`'s public signature is frozen** and `realize_velocity`/`realize_rhythm`/`role_pitch` receive the weight via the SAME additive-private-param precedent S18/S20 used for `pad_voices`/`ctx` (those are private free fns; widening their private arg list is the blessed precedent, see `chord_engine.rs:1259–1277` doc and `realize_step:968` reading `pad_voices` off `ctx`).

A new pure helper resolves the weight (defaulting to a neutral 0.5 when prominence is empty — the uniform path):

```rust
/// The prominence weight (0..1) for `role` in this section, read off the resolved
/// per-layer prominence. Returns the neutral `PROMINENCE_NEUTRAL` (0.5) when the
/// section's prominence is EMPTY (the identity/uniform path) OR the role is unlisted,
/// so the legacy realization is byte-identical when prominence is absent. Pure.
fn prominence_weight(ctx: &crate::composition::StepContext, role: OrchestralRole) -> f32;
```

The realizer applies `w = prominence_weight(ctx, role)` as bounded nudges, each a no-op at `w == 0.5`:
- **Velocity:** a centered gain `(w - 0.5) * PROMINENCE_VEL_SPAN`, added inside `realize_velocity` AFTER the existing contour and clamped 1..=127. At `w==0.5` the term is 0 → byte-identical.
- **Register:** a centered melody/fill octave bias `(w - 0.5) * PROMINENCE_REG_SPAN` folded into the existing `bright_octaves` lift in `role_pitch` (`chord_engine.rs:1061,1078,1105`). At `w==0.5`, 0 → byte-identical. (Bass stays register-exempt as today.)
- **Rhythm:** a recessive layer (`w < 0.5`) biases toward the plainer/sustained band; a foreground layer (`w > 0.5`) toward the busier band — expressed by nudging the `edge_activity` band thresholds the `Melody`/`CounterMelody` arms already test (`chord_engine.rs:1595,1608,1617`). At `w==0.5`, no threshold shift → byte-identical.

> **All three nudges are centered on 0.5 and zero-valued there.** This is the load-bearing freeze property: when `prominence` is empty (identity/uniform), `prominence_weight` returns 0.5, every nudge is exactly 0, and the realizer's output is bit-for-bit today's. The new constants (`PROMINENCE_NEUTRAL=0.5`, `PROMINENCE_VEL_SPAN`, `PROMINENCE_REG_SPAN`) live in `chord_engine` as the article-bias consts do; the music-craft design sets their magnitudes. They are reachable ONLY through a composed `subject_melody` profile — never under identity.

## 3.4 Boundary check (image logic stays out of `chord_engine`)

- `pure_analysis.rs`: writes the saliency knobs (`subject_size`/`fg_bg_contrast`/`subject_energy`, already produced at `:754–760`) and the affect SENTINEL (§2.2). It computes NO affect, NO prominence — pixels only.
- `composition.rs` (planner): computes the affect composite, runs the character/prominence ladders, resolves saliency→weights. Reads perceptual scalars, emits structure + resolved weights. No pixels.
- `chord_engine.rs` (realizer): reads `ctx.section.orchestration.prominence` (resolved scalars) and the (Slice-A-optional) per-character articulation bias. Sees no image, no saliency, no `ImageUnderstanding`. The split the music-craft design requires holds exactly.

---

# Part 4 — BYTE-FREEZE GUARANTEE

The migration preserves `engine_equivalence`: goldens **240 / 114 / 84 / 36 / 79** unmoved, `realize_step` public signature frozen, `single_section_default`'s identity profile keeps the new behaviour structurally unreachable on the net.

## 4.1 Exactly which functions change, and why each is behaviour-neutral on the identity path

| Function | File | Change | Neutral-on-identity argument |
|---|---|---|---|
| `ImageUnderstanding` struct | composition.rs | +2 fields `affect_arousal`, `affect_valence` | Additive fields; `neutral()`/`understand_image_pure` set the `-1.0` sentinel. The equivalence net never builds an `ImageUnderstanding` (it hand-builds `Section`/`StepContext`), so this struct is OFF the net entirely. |
| `ImageUnderstanding::neutral` | composition.rs:93 | sets the 2 new fields to `-1.0` | Test/no-op constructor; not on the net. |
| `Knob` enum + `Knob::read` | composition.rs:338,361 | +2 variants reading the 2 new fields | New variants are only NAMED by new `character` JSON rules; existing rules/tests reference none. `read` arms are pure field reads. Not on the net. |
| `affect_composite`, `character_tempo_bpm`, `lookup_prominence` | composition.rs | NEW pure fns | Reached only from `CompositionPlanner::plan` (compose path), which the net never calls (§1.5). |
| `CompositionPlanner::plan` | composition.rs:697 | replace `:727` clamp with `character_tempo_bpm`; compute affect; resolve prominence | Compose path only — OFF the net. With `affect` absent → `AffectMappings::default()` ships the legacy `{56,96}` ballad window → the compose-path tempo is bit-identical to the old clamp; with `affect` present → opt-in de-cap. |
| `BALLAD_BPM_MIN/MAX` consts | composition.rs:851 | **deleted** (moved to JSON `affect.character_tempo.ballad`) | Their only reader was `:727`, now replaced. No other reference (grep-verified: the consts appear only at their definition and the `:726–727` comment+clamp). |
| `OrchestrationProfile` struct | composition.rs:208 | +1 `#[serde(skip)]` field `prominence: Vec<LayerProminence>` | `identity()` sets it empty; `is_identity()` UNCHANGED; `#[serde(skip)]` never deserializes → `mappings.json` byte-shape unchanged. The net's `OrchestrationProfile::identity()` (`engine_equivalence.rs:97`) carries empty prominence. |
| `OrchestrationProfile::identity` | composition.rs:246 | +`prominence: Vec::new()` in literal | Empty → uniform → neutral. |
| `PlanMappings`/`CompositionMappings` | composition.rs:464 / mapping_loader.rs | +`affect`, +`prominence`, +`prominence_catalogue`, all `#[serde(default)]`; `From` maps them | Loader-mirror lane; back-compat by `#[serde(default)]`. Not on the net. |
| `prominence_weight` + 3 nudges + new consts | chord_engine.rs | NEW pure helper; centered nudges in `realize_velocity`/`role_pitch`/`realize_rhythm` | **The freeze-critical change.** Each nudge is `(w-0.5)*SPAN`; under identity `prominence` is empty → `prominence_weight` returns 0.5 → every nudge is exactly 0.0 → the emitted `NoteEvent`s are bit-for-bit today's. Argued per-call below. |
| (Slice-A-optional) per-character articulation bias | chord_engine.rs:1225 | generalize `BALLAD_ARTIC_BIAS` to read the plan's character | Under identity, `legacy_default_section`'s character is `Ballad` (it carries no character field today; see §4.2) → the bias resolves to the Ballad value (1.0, the current const) → byte-identical. Deferrable out of Slice A entirely. |

## 4.2 The identity path is character-Ballad and prominence-empty by construction

`legacy_default_section` (`engine.rs:745`) builds the flat-path section. It carries NO `character` field today (character lives on `CompositionPlan`, not `Section`). For the Slice-A-optional per-character bias to be neutral, the realizer must resolve "what character" from the plan, and the legacy flat path has no plan. **Resolution:** the per-character bias reads `ctx` — but `StepContext` has no character field. So the bias generalization, IF taken in Slice A, threads the character via the SAME additive-private-param route (`realize_rhythm` already takes `ctx`; add the character as a private param defaulting to `Ballad` on the legacy call). On the legacy path the caller passes `Character::Ballad` (the flat path's implicit character) → bias 1.0 → byte-identical. **Recommendation:** keep the per-character bias OUT of Slice A (tempo-window-only character), so NO `chord_engine` change ships in Slice A and the character axis is proven audible purely through tempo before any realizer touch. Then the bias is a clean, separately-witnessed follow-on.

## 4.3 The prominence nudges are zero at the identity operating point — per golden

The equivalence net's relevant goldens and why each is unmoved:
- **Cadence hold 240 (`engine_equivalence.rs:278`).** The cadence path is the `is_cadence` early return (`chord_engine.rs:1370`), which is BEFORE any role match and reads only `LEGATO_FRAC`+`rit`. The prominence rhythm nudge touches only the `Melody`/`CounterMelody` non-cadence band thresholds; it cannot reach the cadence return. `240 = round(200 * min(0.95*1.30, 1.20))` — untouched.
- **Cadence velocity 114 / 84 (`:274,290`).** `realize_velocity` gains a `+(w-0.5)*SPAN` term; under identity `w==0.5` → +0. `114 = round(96 + sat100_gain 18)`, `84 = round(96 - 12)` — untouched.
- **Register 36 / 79 (`:236,240`).** `role_pitch` folds `(w-0.5)*SPAN` into `bright_octaves`; under identity +0. `G_BASS_NOTE=36` (BASS floor, dark-exempt) and `G_MELODY_NOTE=79` (top tone seated) — untouched.

Because `prominence` is empty under identity (`OrchestrationProfile::identity()`), `prominence_weight` short-circuits to `PROMINENCE_NEUTRAL=0.5` and ALL three nudges evaluate to exactly 0.0 — not "approximately," exactly, since `(0.5-0.5)*SPAN == 0.0` and `round(x + 0.0) == round(x)`. The freeze is preserved by construction, independent of `SPAN` magnitudes.

## 4.4 The one place `engine.rs` is (or is not) touched — narrowly scoped, operator-gated

S20's spec lists `src/engine.rs` in the LOCKED-OFF set with a sha256 freeze witness. This reframe holds that line: **the prominence/affect work needs ZERO `engine.rs` change.** Verification:
- The affect composite + character tempo live entirely in `composition.rs` (planner) + `mappings.json` — `engine.rs` already calls `planner.plan(...)` (`engine.rs:367`) and reads `plan.key_tempo.home_mode`; it needs no new read.
- The prominence field rides `OrchestrationProfile` (composition.rs) and is read in `chord_engine.rs`. `engine.rs` passes `&section.steps`, `section.ms_per_step`, and `&ctx` into `decide_instrument_action` (`:554–562`) already — `ctx.section.orchestration.prominence` is reachable through the SAME borrow with no new parameter.
- `legacy_default_section` (`engine.rs:745`) builds `OrchestrationProfile::identity()`, which gains the empty `prominence` field via composition.rs's `identity()` — that is a composition.rs change the literal in engine.rs does not name (it calls `OrchestrationProfile::identity()`, not a struct literal), so engine.rs's text is unchanged.

**Therefore `engine.rs` is NOT touched and its sha256 stays a freeze witness** (the S20 method). The ONLY way `engine.rs` would need a touch is the Slice-A-optional per-character articulation bias IF it threaded character through `decide_instrument_action` rather than via `ctx` — which is exactly why §4.2 recommends keeping that bias out of Slice A. If a later slice DOES need character on `StepContext`, that is the S13-style narrowly-scoped, operator-approved touch: a single additive field on `StepContext` + its `single_section_default` neutral value (`Character::Ballad`), justified and witnessed in its own slice, never silently.

## 4.5 Freeze witnesses to hand the test engineer

1. `engine_equivalence` 9/9 green; goldens confirmed in-file at `tests/engine_equivalence.rs:124–135`.
2. `git diff HEAD -- tests/engine_equivalence.rs` EMPTY (the net is never edited).
3. `git diff HEAD -- src/engine.rs` EMPTY (the engine driver is untouched in Slice A and Slice B) — sha256 matches HEAD.
4. NEW witness test `prominence_neutral_is_byte_identical`: realize a `Melody`/`Bass`/cadence step under (a) `OrchestrationProfile::identity()` and (b) the SAME profile with an explicit `prominence` listing every role at `weight: 0.5`; assert the two `Vec<NoteEvent>` are equal — proves the centered-nudge zero property directly.
5. NEW witness test `affect_absent_block_keeps_ballad_window`: a `mappings.json` with no `affect` key → `AffectMappings::default()` ships `ballad:{56,96}` → a bright image's compose-path `base_ms_per_step` equals the OLD `clamp(56,96)` result for that brightness.
6. Re-derive `assign_role`/`instrument_role` to confirm `Pad`/`CounterMelody` (and thus any non-uniform prominence on them) are unreachable under identity (the S18/S20 method).

---

# Part 5 — STAGING (independently-shippable, independently-HEARABLE slices)

The owner wants to hear progress, not another isolated micro-slice. Each slice below builds, tests headless, and is HEARABLE on `example.jpg`; each keeps the equivalence net green by the §4 argument.

## Slice A — affect → character + tempo de-cap (RECOMMENDED FIRST — biggest audible affect win, least risk)

**Files touched:** `composition.rs` (the `Affect` struct + `affect_composite` + `character_tempo_bpm` + 2 `Knob` variants + 2 `ImageUnderstanding` sentinel fields + replace the `:727` clamp + delete `BALLAD_BPM_*`); `mapping_loader.rs` (`AffectMappings` mirror, `#[serde(default)]`); `assets/mappings.json` (the `affect` block + the filled `character` ladder). `pure_analysis.rs` touched MINIMALLY (set the 2 affect sentinel fields in `understand_image_pure` + `neutral()`). **`chord_engine.rs` and `engine.rs` are NOT touched** (per §4.2 recommendation: character drives tempo only in Slice A).

**Byte-freeze argument:** the entire change is on the compose path (`CompositionPlanner::plan`) + additive `ImageUnderstanding`/`Knob` surface that the equivalence net never constructs (§1.5, §4.1). `AffectMappings::default()` ships the legacy `ballad:{56,96}` window so a no-`affect`-block mapping is bit-identical; the de-cap is opt-in via the populated `affect` block. No realizer change → no golden can move. Witnesses: §4.5 (2),(3),(5).

**What the owner hears differently:** a bright, high-energy `example.jpg` now lands on `scherzo`/`march` in the character ladder (high arousal), and the per-character tempo window lets brightness→120 BPM through instead of clamping to 96 — so the piece is **decisively faster and more energetic**, the exact "lifeless ballad" complaint, fixed. A dark/calm image lands on `nocturne`/`lament` and is slower than even today's 56 floor allowed (down to 44). Two affect-distinct images now sound categorically different in tempo and pace, not just in note detail. This is the headline win.

## Slice B — saliency → role prominence (the music-craft subject-forward system)

**Files touched:** `composition.rs` (`LayerProminence`/`ProminenceProfile` types + `prominence`/`prominence_catalogue` on `PlanMappings` + `lookup_prominence` + the resolve block + `OrchestrationProfile.prominence` `#[serde(skip)]` field + `identity()` literal); `mapping_loader.rs` (mirror fields); `assets/mappings.json` (the `prominence_catalogue` + `prominence` ladder); `chord_engine.rs` (the `prominence_weight` helper + the three centered nudges + new consts). `engine.rs` NOT touched (§4.4).

**Byte-freeze argument:** the realizer change is the freeze-critical one, and §4.3 proves it: under identity `prominence` is empty → `prominence_weight` returns 0.5 → `(0.5-0.5)*SPAN == 0.0` exactly → every emitted `NoteEvent` is bit-for-bit today's. The new prominence arms are reached only through a composed `subject_melody` profile (non-identity), unreachable on the net (`assign_role` delegates to `instrument_role` under identity). Witness: §4.5 (4),(6) + the `git diff EMPTY` on `engine_equivalence.rs`/`engine.rs`.

**What the owner hears differently:** an image with a distinct, contrasting subject (small-to-mid area, high fg/bg contrast — a face on a calm field, a bright object on a dark ground) now pushes the MELODY forward (louder, higher, rhythmically freer) while the background bed RECEDES (quieter, plainer, lower) — the music tracks the *subject*, not the whole-image average. A uniform field still realizes uniformly (byte-stable). This directly attacks "unrelated to the image" for representational photos and is the music-craft design's headline.

## Slice C (follow-on, optional) — per-character realization presets

**Files touched:** `chord_engine.rs` (generalize `BALLAD_ARTIC_BIAS` to a per-character bias; per-character rhythm/harmonic-palette leans), threading character via the §4.2 additive-private-param OR the operator-gated `StepContext.character` field (§4.4). `mappings.json` (per-character preset rows). 

**Byte-freeze argument:** the per-character bias resolves to the Ballad value (today's 1.0 const) on the legacy/identity path (character `Ballad`) → byte-identical; this is the one slice that may take the narrowly-scoped `engine.rs`/`StepContext` touch, justified and witnessed in-slice per the S13 discipline (a deliberate, reviewed addition with its neutral default `Character::Ballad`).

**What the owner hears differently:** each character gains its theory-grounded fingerprint (the music-craft design's "what each character IS") — a `march` is crisp and detached, a `nocturne` legato and rubato-leaning, a `gigue` lilting in compound feel — beyond tempo alone. This deepens Slice A's audible character distinction but is not required for the headline win.

**Sequencing rationale:** Slice A is the cheapest decisive win (data-only, no realizer/engine touch, the literal "bright image sounds fast/major/dense" fix) and de-risks everything by proving the affect ladder before any frozen-kernel code moves. Slice B is the larger music-craft system and carries the one freeze-critical realizer change, fully witnessed. Slice C is the depth pass once both axes are audible. Slices A and B are file-disjoint enough to parallelize (A is planner/data; B adds the realizer nudges) if desired, but A-first is recommended so the owner hears the affect win immediately.

---

## Appendix — `mapping_loader.rs` mirror obligations (the S20 load-bearing detail)

Per the S20 spec's §A5 ("TWO mirror structs"), every new `PlanMappings` field MUST also be added to `CompositionMappings` (the struct that actually deserializes `mappings.json`) AND mapped in `From<CompositionMappings> for PlanMappings` (`composition.rs:495–512`). For S21 that is: `affect: AffectMappings #[serde(default)]`, `prominence: SelectTable #[serde(default)]`, `prominence_catalogue: Vec<ProminenceProfile> #[serde(default)]` on `CompositionMappings`, and the three lines in the `From` impl. Forgetting any silently drops the new data at load (no panic, falls to neutral) — so the test `affect_absent_block_keeps_ballad_window` and a `prominence_round_trips_from_json` test are the witnesses the mirror is wired. The `#[serde(skip)]` resolved fields (`prominence`, the affect sentinels) are NOT in `CompositionMappings` — they are planner-filled, never deserialized.

*Design-only. No source, test, or asset modified by this document. Signatures are binding shapes; bodies are deferred to the implementer/music-craft lanes. No internal codenames appear; the parallel streams are "the affect design" and "the music-craft design."*
