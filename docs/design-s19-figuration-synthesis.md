# S19 — Accompaniment Figuration: SYNTHESIS (the single buildable plan)

**Author role:** Rust Architect — SYNTHESIS pass (DESIGN ONLY — no source, test, or asset modified by this document; `docs/` only).
**Date:** 2026-06-15.
**Status:** PROPOSE-FOR-ITERATION → this is the reconciliation of the two parallel S19 design docs into ONE buildable plan an Implementer can pick up next session.
**Reconciles:**
- `docs/design-s19-figuration-musical.md` (the musical contract: figuration vocabulary, the saliency→layer-role budget, the restraint guardrails).
- `docs/design-s19-figuration-engine.md` (the engine architecture: realize-path placement, the serde data thread, the byte-freeze argument, the saliency engine seam).

**Grounded against HEAD (re-verified, not trusted from the two docs):** `src/chord_engine.rs` Pad arm (`:1419`, the `seat_pc_in_register`/de-dup inner-tone seating `:1450`–`:1463`, `PAD_OVERLAP_FRAC` = 1.10, `realize_step` PUBLIC signature FROZEN, `realize_rhythm` receives `ctx` + `pad_voices`); `src/composition.rs` `OrchestrationProfile { id, layers, density (no-op 0.5), pad_voices }` (`:208`), `LayerRole` (`:196`), `identity()`/`is_identity()` keyed on `pad_voices==0 && layers.is_empty()`, the `Knob`/`Predicate`/`SelectRule`/`SelectTable` first-match-wins machinery (`:281`–`:401`), the saliency knobs `SubjectEnergy`/`ForegroundEnergy`/`BackgroundEnergy`/`FgBgContrast`/`SubjectSize` all wired in `Knob::read`, the planner's ONE-per-plan `self.plan_mappings.texture.select(u)` (`:707`) cloned onto every section (`:752`), `PlanMappings.texture` `#[serde(default)]` (`:424`); `assets/mappings.json` `texture_catalogue` (identity/pad_bed/pad_bed_counter) + the one `texture` rule (`foreground_energy ≥ 0.35 ∧ fg_bg_contrast ≥ 0.20 → pad_bed_counter`).

---

## 0. The convergence and the four real divergences

The two docs **agree** on the load-bearing architecture, and the synthesis ratifies all of it:

- Figuration is reachable ONLY on the compose path through a non-identity `OrchestrationProfile`; `single_section_default`'s identity profile never names a Pad, so the figured branch is structurally unreachable on the `engine_equivalence` net. **Byte-freeze holds with NO golden re-derivation.**
- `realize_step`'s public signature is FROZEN; `realize_rhythm` already receives `ctx` + `pad_voices`, so **no new function parameter** (public or private) is needed — the spec is read off the borrowed `ctx`.
- Slice 3a is a **figured-Pad sub-branch** (no new role) selected **once per plan**; a sibling figuration role + per-section selection are deferred.
- Selection is the **existing `texture` `SelectTable`** over the S18 saliency knobs — no new selection axis, no RNG.
- The vocabulary is **data**, not code; adding a pattern is a JSON edit.

They **diverge** on exactly four points, settled below:

| # | Divergence | Musical doc | Engine doc | Synthesis decision |
|---|---|---|---|---|
| 1 | Data schema | string `pad_figure` id → catalogue row | inline `FigurationSpec { onsets[] }` on the profile | **`figuration_catalogue` of `FigurationSpec` rows, referenced BY ID from the profile** (§1) |
| 2 | Counter on 3a | coexist (`pad_bed_broken` keeps `CounterMelody`) | ship WITHOUT the counter (register overlap) | **No counter on the 3a figured profile** — `HarmonicFill` in the 3rd slot (§2) |
| 3 | Saliency gate | `fg_bg_contrast ≥ 0.40 ∧ foreground_energy ≥ 0.55` (Alberti) | `subject_energy ≥ 0.45 ∧ fg_bg_contrast ≥ 0.25` | **One gate: `subject_energy ≥ 0.45 ∧ fg_bg_contrast ≥ 0.25`** (§3) |
| 4 | Per-plan vs per-section | once per section (budget per layer) | once per plan in 3a, per-section in 3b | **Per-plan in 3a; per-section in 3b** (§4) |

---

## 1. THE DATA SCHEMA — `figuration_catalogue` of `FigurationSpec`, referenced by id

### 1.1 The decision

Adopt **neither doc verbatim; take the reconciliation both gestured at.** The figuration vocabulary lives as a **`figuration_catalogue` array of `FigurationSpec` rows in `mappings.json`**, and the `OrchestrationProfile` carries a **`figuration: Option<String>` handle** that names a catalogue row by id (the musical doc's "string id referencing a catalogue row"). The realizer resolves the handle against the catalogue at plan-build time and threads the resolved `FigurationSpec` (the engine doc's struct) onto the section.

This is the only schema consistent with the project's standing **S15 content-as-data discipline**, and it is consistent for a precise, already-shipped reason: it is **byte-for-byte the same shape as the two vocabularies already in the tree** —

- `form` `SelectTable` → picks an id → resolved against `form_catalogue: Vec<FormSpec>`.
- `texture` `SelectTable` → picks an id → resolved against `texture_catalogue: Vec<OrchestrationProfile>`.
- **`figuration` (new) → an id on the profile → resolved against `figuration_catalogue: Vec<FigurationSpec>`.**

The engine doc's inline-struct schema (`figuration: Option<FigurationSpec>` *embedded* in the profile) works and parses, but it **breaks the catalogue-by-handle pattern**: it inlines open content (the onset values, which the operator will tune and multiply) directly into the control object, the exact coupling the `form_catalogue`/`texture_catalogue` split exists to prevent. The musical doc's bare-string-id is right about the handle but under-specifies the catalogue. The synthesis = the handle (musical doc) + the `FigurationSpec` row type (engine doc) + a dedicated catalogue array (the form/texture precedent). Vocabulary stays open and reusable (two profiles can name the same `"alberti"` row); control on the profile stays a bounded deterministic id.

### 1.2 The concrete types (illustrative — Implementer owns final signatures)

```rust
// ── src/composition.rs ────────────────────────────────────────────────────────
/// One named accompaniment-figuration pattern — pure STRUCTURE, no note content. Lives as a
/// row in `figuration_catalogue`; an OrchestrationProfile references it BY ID. Adding a
/// pattern is a JSON row, NOT a Rust edit (the FormSpec / OrchestrationProfile discipline).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct FigurationSpec {
    /// Stable id, e.g. "alberti" / "broken_up" / "block" (block == the no-op sustained bed).
    pub id: String,
    /// Per-step onset template, time-ordered. 2..=4 entries (the bounded burst). Empty == block.
    #[serde(default)]
    pub onsets: Vec<FigurationOnset>,
    /// Distinct inner chord tones the figure draws from (Alberti = 3). Clamped to band tones.
    #[serde(default = "one_u8")]
    pub voices: u8,
}

/// One onset of a figure: when (fraction of step_ms), which seated inner-voice index, hold.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
pub struct FigurationOnset {
    pub at: f32,        // 0.0..1.0 fraction of step_ms (0 == downbeat, 0.25 == S18 off-beat)
    pub tone: u8,       // seated inner-voice index, cycled modulo the seated voice count
    #[serde(default = "one_f32")]
    pub hold_frac: f32, // hold as a fraction of the gap-to-next-onset (in-step articulation)
}

pub struct OrchestrationProfile {
    pub id: String,
    pub layers: Vec<LayerRole>,
    #[serde(default = "half_f32")] pub density: f32,
    #[serde(default)] pub pad_voices: u8,
    /// NEW S19 — id of a `figuration_catalogue` row this profile's Pad animates with, or None
    /// for the S17 block bed. `#[serde(default)]` (== None) so every OLD profile parses unchanged.
    #[serde(default)]
    pub figuration: Option<String>,
}

pub struct PlanMappings {
    // … existing axes (form, character, meter, key_scheme, theme_behaviour, texture) …
    /// NEW S19 — the figuration vocabulary, parallel to `form_catalogue`/`texture_catalogue`.
    /// `#[serde(default)]` (empty Vec) so an OLD mappings.json with no `figuration_catalogue`
    /// key parses; an unresolved profile handle then falls back to the block bed.
    #[serde(default)]
    pub figuration_catalogue: Vec<FigurationSpec>,
}
```

### 1.3 The serde back-compat story (the load-bearing proof)

The OLD `mappings.json` on disk today (identity / pad_bed / pad_bed_counter; no `figuration` key on any profile; no `figuration_catalogue` block) **parses unchanged**:

- `OrchestrationProfile.figuration: Option<String>` is `#[serde(default)]` → `None` on every existing profile → the realizer takes the S17 block-bed path → **byte-identical to S18**.
- `PlanMappings.figuration_catalogue: Vec<FigurationSpec>` is `#[serde(default)]` → empty Vec when absent.
- Resolution is total: profile names `Some(id)` → look up in `figuration_catalogue` → `None`/unresolved id → **block bed** (honest degradation, exactly as an unmatched `texture` id falls back to `identity()`). A profile can never panic on a missing figure.
- `identity()` constructs `figuration: None`; `is_identity()` is UNTOUCHED (`pad_voices == 0 && layers.is_empty()`). A figured profile always has `pad_voices > 0`, so it is never mistaken for identity. **The byte-freeze anchor is unchanged.**

### 1.4 Resolution seam (where the handle becomes a spec)

The planner already resolves the `texture` id against `texture_catalogue` at `composition.rs:708` (`lookup_orchestration`). Resolve the figuration handle **at the same point**: after the profile is selected, look up `profile.figuration` against `plan_mappings.figuration_catalogue` and thread the resolved `Option<FigurationSpec>` onto `Section` (a new `Section` field, or — cheaper — store the resolved spec on the cloned `OrchestrationProfile` so nothing on `Section` changes and the realizer reads `ctx.section.orchestration` as the engine doc described). **Recommended: keep the handle on the profile and resolve in the Pad arm** by carrying the catalogue reference no further than the planner — i.e. the planner resolves once and stores the `Option<FigurationSpec>` on the section's orchestration clone. Either way the realizer reads a resolved spec off the already-borrowed `ctx`; **no new function parameter.**

---

## 2. COUNTER COEXISTENCE FOR SLICE 3a — ship WITHOUT the counter

**Decision: take the engine doc's call.** The 3a figured profile (`pad_figured`) does **NOT** carry `CounterMelody`. Its layers are `["Bass","Pad","HarmonicFill","Melody"]` — the figured Pad animates the bed, `HarmonicFill` fills the 3rd inner slot, and the melody carries the tune. The S18 counter ships only on `pad_bed_counter`, selected by its own (unchanged) rule.

**Why:** the figured Pad (harmonic figuration, fill band `[55,67)`) and the S18 counter (melodic figuration, also seated in `[55,67)`) **overlap in register**, and on a held chord the broken Pad re-articulates the same inner tones the counter is stepping through — a real collision risk the musical doc itself flags (§4.1/§4.3) but resolves only with a "de-prioritize the counter's band tone" voicing rule that is **extra coordination not yet specified**. Slice 3a is the cheapest-first cut; shipping the figure and the counter on one profile bundles two un-individually-heard behaviors and an unwritten voicing contract. Hear the figured bed alone first.

**The register-band plan that makes eventual coexistence (3b) safe.** When the counter and the figured bed DO share a profile (3b), enforce a **register split inside the fill band** so the two moving inner lines never sit on the same pitch:

- **Figured Pad:** seats in the **lower fill band `[55, 61)`** (G3–C4) — the broken/Alberti chord tones live in the bottom of the inner register, under the counter.
- **Counter line:** seats in the **upper fill band `[61, 67)`** (C4–F#4) — the moving melodic counter rides above the figured bed, below the melody floor (≥67).
- Both stay strictly under `MELODY_REGISTER_FLOOR=67` and above `BASS_REGISTER_FLOOR=36`. The split is a `register_floor`/`register_ceiling` band passed to each seat call; it is a **3b voicing contract (music-owned)**, named here so 3b doesn't rediscover it. Until then, the two never co-occur on one profile.

---

## 3. SALIENCY → LAYER-ROLE GATING — the one 3a selection rule

**Decision: take the engine doc's predicate (it directly encodes the operator requirement) and keep the S18 counter rule below it.** The two docs proposed different knobs/thresholds; the engine doc's `subject_energy` gate is the correct primary because the operator requirement is literally *"the most prevalent **subject** plays more of a role"* — `subject_energy` is the subject-region knob, `foreground_energy` is the whole-foreground knob. The conjunctive `fg_bg_contrast` guard enforces "there is a real subject/ground stratification," not a busy-but-flat field.

The first-match-wins `texture` ladder for 3a (the figured rule checked FIRST as most-specific):

```jsonc
"figuration_catalogue": [
  { "id": "block",   "onsets": [] },
  { "id": "alberti", "voices": 3,
    "onsets": [ {"at":0.0,"tone":0}, {"at":0.25,"tone":2}, {"at":0.5,"tone":1}, {"at":0.75,"tone":2} ] }
],
"texture_catalogue": [
  { "id": "identity",        "layers": [], "density": 0.5, "pad_voices": 0 },
  { "id": "pad_bed",         "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.55, "pad_voices": 3 },
  { "id": "pad_bed_counter", "layers": ["Bass","Pad","CounterMelody","Melody"], "density": 0.6,  "pad_voices": 3 },
  // NEW S19 — same bed as pad_bed (HarmonicFill, NO counter), but the Pad ANIMATES with Alberti.
  { "id": "pad_figured",     "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.62, "pad_voices": 3,
    "figuration": "alberti" }
],
"texture": {
  "default": "pad_bed",
  "rules": [
    // S19 (checked FIRST): a strong, salient SUBJECT earns the moving figured bed.
    { "when": [ {"knob":"subject_energy","op":"ge","lo":0.45,"hi":0.0},
                {"knob":"fg_bg_contrast","op":"ge","lo":0.25,"hi":0.0} ], "pick": "pad_figured" },
    // S18 (unchanged): a busy foreground over a real subject earns the counter line.
    { "when": [ {"knob":"foreground_energy","op":"ge","lo":0.35,"hi":0.0},
                {"knob":"fg_bg_contrast","op":"ge","lo":0.20,"hi":0.0} ], "pick": "pad_bed_counter" }
  ]
}
```

**The exact 3a gate predicate, stated once:**
> `pad_figured` is selected iff **`subject_energy ≥ 0.45` AND `fg_bg_contrast ≥ 0.25`** (first-match-wins, so this beats the S18 counter rule). Otherwise the S18 counter rule, else `pad_bed` (block bed).

**How this satisfies the operator requirement in 3a (per-plan):**
- **Salient subject, high contrast** (`subject_energy ≥ 0.45 ∧ fg_bg_contrast ≥ 0.25`) ⇒ `pad_figured` ⇒ the bed MOVES (Alberti) under the prominent tune — richer foreground interest, the subject earns the animated accompaniment.
- **Busy foreground, real subject, lower subject energy** ⇒ `pad_bed_counter` ⇒ the moving counter line, static bed.
- **Calm / low-contrast / subject-less** ⇒ matches nothing ⇒ `pad_bed` ⇒ block sustained — **restraint: never figurate a calm image.**

The full musical-doc per-LAYER budget (melody-richness from `subject_energy × fg_bg_contrast`; the "subject-less abstract pushes interest into an animated BACKGROUND bed" inversion using low `fg_bg_contrast`) is **richer than one rule and is deferred to 3b**, where additional catalogue rows + rules quantize it. 3a ships the single highest-value cut of it: subject present → bed animates; subject absent/calm → bed sustains.

---

## 4. PER-PLAN VS PER-SECTION — confirm the staging

**Decision: ratify the engine doc's staging.**

- **Slice 3a is PER-PLAN.** The `texture.select(u)` call stays where it is (`composition.rs:707`), selecting ONE figuration for the whole piece on the whole-image subject knobs, cloned onto every section. This matches exactly how S17 selected the Pad and S18 selected the counter (the `:707` comment already names "section-conditioned selection is a later slice"). It fully realizes *"this image's accompaniment is figured vs plain"* and is immediately hearable. **No planner-loop change in 3a.**
- **Slice 3b adds PER-SECTION.** Move the `texture.select(u)` call INSIDE the per-section loop (`composition.rs:~715`) so a high-salience A section can be figured while a quiet B stays plain. This is a one-line move of an existing call — but it requires a **per-section `ImageUnderstanding`** (region-saliency-per-section), which is its own work (the deeper end of the saliency reader). That dependency is why per-section is 3b, not 3a.

So: 3a = per-plan, no loop change, the whole-image subject decides the piece's figuration. 3b = per-section, the planner-loop move + a per-section understanding. The musical doc's "one figure per layer per section, stable within a section" is satisfied trivially in 3a (per-plan ⇒ identical for every section ⇒ stable within each section) and becomes genuinely per-section in 3b.

---

## 5. THE DEFINITIVE SLICE LADDER

Every slice: builds, tests headless, keeps `engine_equivalence` byte-green (goldens **240/114/84/36/79 unmoved**), `realize_step` signature FROZEN, and touches NONE of the LOCKED-OFF set (`src/modem.rs` + `src/bin/modem_*`, `src/synth_sink.rs`, `src/midi_output.rs`, `src/cli.rs`, `src/tui.rs`, `src/main.rs`/the scheduler).

### Slice 3a — FIGURED PAD (Alberti/broken-chord bed), per-plan, no counter — **BUILD FIRST**

**Goal:** for the first time the held harmony MOVES rhythmically — a salient-subject image gets an Alberti/broken-chord bed under the tune; a calm image keeps the S17 block bed. One figure (`alberti`), one layer (Pad), one selection rule, per-plan, byte-freeze untouched.

**Files touched (all OUTSIDE the locked set):**
- `src/composition.rs` — `FigurationSpec` + `FigurationOnset` serde types; `figuration: Option<String>` `#[serde(default)]` field on `OrchestrationProfile`; `figuration_catalogue: Vec<FigurationSpec>` `#[serde(default)]` on `PlanMappings`; resolve the handle against the catalogue at the existing `texture` resolution point (`:708`) and thread the resolved spec onto the section's orchestration clone. `identity()` literal gains `figuration: None`. (Rust Implementer.)
- `src/chord_engine.rs` — the figured-bed sub-branch INSIDE the existing `Pad` arm (`:1419`): when the resolved spec is `Some` and non-empty, seat the `voices` inner tones (REUSE the existing `:1450`–`:1463` `seat_pc_in_register`/de-dup) then map `onsets` → `NoteEvent`s at `at · step_ms`, each `hold = (gap_to_next · hold_frac)` capped at `step_ms × PAD_OVERLAP_FRAC` (the established ≤1.2× ceiling); `None`/empty → the unchanged block bed. New private helper `figured_bed(spec, &chord.notes, velocity, step_ms) -> Vec<NoteEvent>`. The block path is byte-untouched. (Music Theory owns the figure body/onset semantics; Implementer owns the seam.)
- `assets/mappings.json` — the `figuration_catalogue` (`block` + `alberti`), the `pad_figured` profile row (HarmonicFill, no counter), the `pad_figured` `texture` rule (§3). (Rust Implementer = SOLE writer of `mappings.json`; Music Theory hands the onset/threshold values.)
- `tests/` — the new figuration net below. NO change to `tests/engine_equivalence.rs` (git diff EMPTY).

**Test net:**
- `figuration_emits_bounded_burst` — the figured Pad emits exactly `onsets.len()` events, `2..=4`, never an unbounded count.
- `figuration_onsets_are_in_step` — every event `offset_ms + hold_ms ≤ step_ms × 1.2`; the last onset never overhangs the step.
- `figuration_tones_are_chord_tones_in_band` — each figure note is a chord tone seated in the fill register `[55,67)`.
- `figured_bed_off_beat` — the figure has ≥1 onset at `offset_ms > 0` (distinguishes it from the offset-0 block bed).
- `block_bed_unchanged_when_figuration_none` — a `pad_bed`-style profile with `figuration: None` still emits the S17 simultaneous-at-offset-0 block (the back-compat witness).
- `texture_selects_pad_figured_on_salient_subject` — the `SelectTable` returns `pad_figured` on a hand-built `ImageUnderstanding` with `subject_energy=0.5, fg_bg_contrast=0.3`, and does NOT on a calm one.
- `unresolved_figuration_id_falls_to_block` — a profile naming a missing catalogue id resolves to the block bed (no panic).
- **Freeze witnesses:** `engine_equivalence` green (9/9), goldens confirmed in-file; sha256 of every locked-off file == `git show HEAD:`.

**Byte-freeze argument:** the figured branch is reachable ONLY through `pad_figured` on the compose path. Under `identity()`, `assign_role` delegates to `instrument_role`, which returns only `Bass`/`HarmonicFill`/`Melody` — never `Pad` — so the figured branch is **structurally unreachable** on the equivalence net independent of the green test. `figuration: None` under identity takes the block path regardless. `FigurationSpec`/`FigurationOnset` are serde-only; the net constructs none. `realize_step`/`realize_rhythm` signatures unchanged (spec read off the borrowed `ctx`). **No golden moves.**

### Slice 3b — counter+figure coexistence, per-section selection, the per-layer budget

**Goal:** the foreground/background figuration split becomes audible — a figured bed AND the S18 counter on one profile (register-split per §2), per-section figuration (figured A / plain B), and the musical-doc per-layer saliency budget (subject-less abstract animates the BACKGROUND bed).

**Files:** `src/composition.rs` (move `texture.select(u)` into the per-section loop `:~715`; the per-section `ImageUnderstanding` plumbing), `src/chord_engine.rs` (the figured-Pad register-split band `[55,61)` so it clears the counter's `[61,67)` — a `register_floor`/ceiling on the seat call), `assets/mappings.json` (more `figuration_catalogue` rows: `broken_up`/`broken_down`; the per-budget profiles + rules incl. the low-contrast→animated-background profile). DATA-heavy + the one planner-loop move + the register-split seam.

**Test net (additive):** `counter_and_figure_no_pitch_collision` (the two never share a seated pitch); `per_section_figuration_varies` (A figured, B plain on a two-section plan with differing section understandings); `low_contrast_animates_background` (a subject-less busy image selects the animated-background profile). Freeze witnesses repeat.

**Byte-freeze:** still additive; the planner-loop move does not touch the identity path (single-section/default plans still resolve to one profile); `engine_equivalence` green; goldens unmoved.

### Slice 3c — figuration intensity from `density` + a wider catalogue

**Goal:** the figure's onset count scales with the reserved `OrchestrationProfile.density` (drop off-beat onsets below a density threshold); add `comping_offbeat`/`arp_sweep` rows with finer saliency thresholds.

**Files:** `src/chord_engine.rs` (ONE bounded `ctx.section.orchestration.density` read in the figured-bed arm), `src/composition.rs`/`assets/mappings.json` (more rows/rules). Mostly DATA + one scalar read. No new role.

**Byte-freeze:** `density` defaults to the no-op `0.5`; the figured arm is still unreachable under identity; goldens unmoved.

### Slice 3d (METER-GATED) — waltz / stride / bar-Alberti

**Goal:** the measure-spanning figures (oom-pah, stride, full bar-Alberti).
**Blocked on Stage 3 (meter):** these need `metric_position` (beat-within-measure), which does not exist today. Cannot ship until the Meter stage introduces `beats_per_measure`/`metric_position`. **Flagged, not buildable now.**

### What cannot be done without touching locked files

Nothing in 3a–3c requires a locked-off file. The ONE thing forbidden by the locked `main.rs` scheduler (blocks until a step's last event) is **true cross-step legato** — a figure note whose hold spans into the next step. This is OUT OF SCOPE for all of 3a–3d (same limit the S17 Pad and S18 counter already accept); cross-step continuity is achieved by plan-position choosing each step's starting tone, never by an over-long `hold_ms`. Every figure is complete within its step (last onset `offset+hold ≤ step_ms × 1.2`). The measure-spanning figures (3d) are meter-gated for the same structural reason, NOT a scheduler edit.

---

## 6. CONSOLIDATED OPEN OPERATOR STEERS (deduplicated, each with a recommended default)

These are the decisions that genuinely need you. Each has a recommended default you can one-line confirm; silence = the Implementer proceeds on the default.

1. **Name lock: "figuration."** Confirm the layer/type/JSON-key is called **figuration** (vs "accompaniment pattern" / "comping"). It must be stable before the type lands (the S17 `TextureProfile`→`OrchestrationProfile` rename lesson). There is no `figuration`/`Figuration` identifier in the tree today, so no collision. **Recommended default: lock "figuration"** (`FigurationSpec`, `FigurationOnset`, the `figuration` profile field, `figuration_catalogue`).

2. **Schema = catalogue-by-handle (not inline struct).** The vocabulary lives as `figuration_catalogue` rows referenced by a string id on the profile, mirroring `form_catalogue`/`texture_catalogue`. **Recommended default: confirm the catalogue-by-handle schema** (§1) — it is the only S15-content-as-data-consistent option and gives clean serde back-compat.

3. **3a ships figuration WITHOUT the counter on the same profile.** The 3a `pad_figured` profile uses `HarmonicFill` in the 3rd slot, not `CounterMelody`, so the figured bed and the counter never co-occur (register-overlap risk deferred). The counter+figure stack, with the `[55,61)`/`[61,67)` register split, is Slice 3b. **Recommended default: yes, no counter on the 3a figured profile.**

4. **3a selection is per-plan (whole-image), not per-section.** A salient image gets a figured bed across the whole piece; per-section figuration (figured A / plain B) is 3b and needs a per-section saliency read. **Recommended default: per-plan for the first cut** (matches the S18 posture, immediately hearable).

5. **The 3a saliency gate threshold.** `pad_figured` fires on `subject_energy ≥ 0.45 ∧ fg_bg_contrast ≥ 0.25`. These are tuning numbers, fully in JSON, easy to re-cut after a listen. **Recommended default: ship at `0.45 / 0.25`** and re-tune after hearing real images.

6. **First figure = Alberti only.** 3a ships ONE pattern (`alberti`, the low-high-mid-high cell `{0:t0, ¼:t2, ½:t1, ¾:t2}` over 3 inner tones); `broken_up`/`broken_down`/`comping_offbeat`/`arp_sweep` follow in 3b/3c. **Recommended default: Alberti as the single 3a figure** (canonical "animate the held chord," anti-mud at one note per onset).

7. **Meter-dependent figures stay deferred.** Waltz oom-pah, stride/ragtime, and the full bar-spanning Alberti are Slice 3d, **gated on the Meter stage** (they need beat-within-measure, which doesn't exist). **Recommended default: confirm 3d stays meter-gated** (no attempt to fake measure structure inside one beat).

8. **Tone-index semantics on non-triads (music-owned, flagged for the data rows).** `FigurationOnset.tone` is an index into the seated inner voices, cycled modulo the seated count; how Alberti behaves on a 7th chord (3 non-root band tones) vs a triad (2) is the Music Theory contract the data rows are authored against. **Recommended default: cycle the index modulo the seated voice count** (a 4-onset Alberti over 2 seated tones reads 0,1,0,1; over 3 reads 0,2,1,2) — confirm with Music Theory before the rows are written.

---

*Design-only synthesis. No source, test, or asset modified by this document. The illustrative Rust types and `mappings.json` rows are non-binding: the Music Theory Specialist owns the figure musical contracts (onset/tone values, register banding, the figure↔counter voicing) and the Rust Implementer owns the final signatures and is the sole writer of `mappings.json`.*
