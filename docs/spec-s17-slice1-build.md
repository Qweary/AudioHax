# S17 Slice 1 — The Buildable Per-File Spec (Sustained Harmonic Pad + Bass Bed)

**Author role:** Rust Architect (DESIGN ONLY — no source/test/asset modified by this document; `docs/` only).
**Date:** 2026-06-15.
**Status:** LOCKED for build. This is the single contract the two implementers (Rust Implementer + Music Theory Specialist) build against. It turns the two S16 texture/density design docs (`design-s16-texture-musical.md`, `design-s16-texture-engine.md`) into a per-file build spec against the **as-built S15 code seam** and resolves the one open seam decision (true cross-step sustain vs legato-overlap re-articulation). Modeled on `spec-s15-slice1-build.md` (the house format/altitude).

**Grounded against the actual head working tree** (line numbers verified, NOT trusted from the design docs which reference an S15-era tree):
- `src/chord_engine.rs`: `OrchestralRole` enum `:801`; `instrument_role` `:863`; `realize_step` `:897` (already takes `ctx: &composition::StepContext` `:903`, calls `instrument_role(inst_idx, num_instruments)` directly `:905`); `role_pitch` `:980` (3-arm `match role`); `realize_velocity` `:1051` (3-arm tail `match role` `:1105`); `realize_rhythm` `:1156` (3-arm `match role` `:1261`); the HarmonicFill arm `:1283`–`:1296` with the rest bug at `:1290`; the `edge`/`edge_activity` split at `:1170`–`:1173`; the `sustained` closure `:1236`; `NoteEvent` `:839` (`hold_ms: u64` `:845`); `Chord` `:33` (`notes: Vec<u8>`); `theme_melody_pitch` `:1549`; `resolve_motif` `:1456`; register floors `:974`–`:976`.
- `src/composition.rs`: `ImageUnderstanding` `:39` (note: it already has a SCALAR field `texture: f32` `:44` — the image-texture knob; the NEW per-section orchestration profile must NOT be named `texture` on `ImageUnderstanding`); `Knob` `:213` + `Knob::read` `:233`; `SelectTable` `:304` + `SelectTable::select` `:315`; `PlanMappings` `:330` + `From<CompositionMappings>` `:345`; `Section` `:400`; `CompositionPlan` `:432` + `CompositionPlan::locate` `:454`; `StepContext<'a>` `:477` + `single_section_default` `:492`; `CompositionPlanner::plan` `:538` (section build loop `:609`–`:650`).
- `src/engine.rs`: `decide_instrument_action` `:694` (7 args incl. `ctx` already); `decide_step` `:526` (compose path `:542`, legacy path `:576`); `legacy_default_section` `:745`; `legacy_default_key_tempo` `:763`; `snapshot_phrase` `:599`.
- `src/main.rs`: the per-step note_on/note_off scheduling region `:431`–`:496` (the seam — `:463`–`:477` pair each event with a note_off at `start_instant + hold_ms`, and `:483`–`:495` block the step until its last event fires).
- `tests/engine_equivalence.rs`: `default_section` builder `:81`–`:96` (struct literal — gains the new field), goldens `G_BASS_NOTE=36` `:126`, `G_MELODY_NOTE=79` `:131`, cadence vel `114`/`84` + hold `240` `:258`–`:289`, `MS_PER_STEP=200` `:120`.
- `assets/mappings.json`: the `composition` block `:89`–`:137` (the schema this slice extends, additively).
- `src/mapping_loader.rs`: `CompositionMappings` `:109` + `MappingTable.composition` `:133` (`#[serde(default)]` Option — the back-compat floor).

**Scope is LOCKED by the operator** and is NOT re-litigated or widened here: Slice 1 = **SUSTAINED HARMONIC PAD + BASS BED, Pad-alone, ONE mechanism.** No saliency reader (`analyze_regions_pure`/`pick_subject_region`), no counter-melody realization — those are Slice 2.

---

## 0. The three deliverables + the operator decisions (settled — build to these)

1. **FIX the HarmonicFill rest bug.** Re-point the `edge < 0.15` rest-as-gesture (`chord_engine.rs:1290`) from the RAW per-bar edge to the NORMALIZED `edge_activity`. (§2.)
2. **ADD a sustained `OrchestralRole::Pad` arm** + a Pad realize branch that emits a HELD multi-note chord bed (not a 1-step stab). (§3.)
3. **WIRE role assignment via a plan-aware `assign_role`** that DEFAULT-DELEGATES to the existing `instrument_role`; texture CONTENT (which layers, density, pad_voices) is an `OrchestrationProfile` row in `mappings.json` + the `composition.rs` data machinery; the new ROLE is bounded code. (§4, §5.)

**Operator decision A — CounterMelody now, realize-STUBBED.** Add `OrchestralRole::CounterMelody` as an enum arm in this slice, but its realize branch DELEGATES to the HarmonicFill figure (so Slice 2 is a pure realize-fill). It is byte-neutral because it is unreachable under the default ctx's identity profile. **Confirmed and built into this spec** (§3.4) — it costs one enum arm + the four exhaustive-`match` arms that arm forces, and buys Slice 2 a parameter-free fill.

**Operator decision B — the seam decision.** RESOLVED in §6 below: **Slice 1 ships LEGATO-OVERLAP re-articulation, NO main.rs touch.** Rationale, trade, and the deferred true-sustain path are in §6.

**Naming lock (divergence from the design docs — see §9).** The S16 engine doc names the new per-section profile field `Section.texture: TextureProfile`. `ImageUnderstanding` **already** has an unrelated scalar field `texture: f32` (`composition.rs:44`, the image-texture knob feeding `Knob::Texture`). To avoid a confusing collision across the two structs the realizer reaches, this spec renames the new artifacts: the per-section profile is **`OrchestrationProfile`** and the `Section` field is **`Section.orchestration: OrchestrationProfile`**. The layer enum stays `LayerRole`. Everywhere the S16 docs say "TextureProfile"/"`Section.texture`", read "`OrchestrationProfile`"/"`Section.orchestration`".

---

## 1. WHAT IS ALREADY IN PLACE (do not re-build)

The S15 spine the design docs assume is **already shipped** — verify before building, do not duplicate:

- `realize_step` **already takes `ctx: &composition::StepContext`** (`chord_engine.rs:903`). The S16 docs' "thread `ctx` into the realizer" is DONE. The new roles/pad reach `ctx.section.<...>` and `step.chord` zero-copy. **The `realize_step` signature does NOT change in this slice.**
- `decide_instrument_action` **already has the 7-arg form** with `ctx` (`engine.rs:694`–`:701`). Unchanged.
- The compose path (`decide_step` `:542`) **already builds a per-step `StepContext`** from `comp.locate(step_idx)` and passes `&ctx`. Unchanged.
- The legacy path (`decide_step` `:576`) **already builds `single_section_default`** over `legacy_default_section`. The ONLY change here is that `legacy_default_section` (and the test's `default_section`, and `single_section_default`'s consumers) must populate the new `Section.orchestration` field with the IDENTITY profile (§5.3).
- `instrument_role` (`:863`) is unchanged; `assign_role` (§4) is NEW and DELEGATES to it.

So this slice is purely: one threshold re-point + two new enum arms + their forced `match` arms + one new `assign_role` + one new `OrchestrationProfile` data path + the identity-profile defaulting. No signature changes anywhere.

---

## 2. DELIVERABLE 1 — THE HARMONICFILL REST-BUG FIX (Music Theory owns the musical value)

### 2.1 The as-built bug, exactly

`realize_rhythm` (`chord_engine.rs:1156`) computes TWO edge scalars:

```rust
// chord_engine.rs:1170
let edge_activity = (features.edge_density / EDGE_ACTIVITY_RANGE_MAX).clamp(0.0, 1.0); // NORMALIZED 0..1
// chord_engine.rs:1173
let edge = features.edge_density.clamp(0.0, 1.0);   // RAW per-bar, ≈0.005..0.05 on real photos
```

The HarmonicFill arm (`:1283`–`:1296`) gates the rest on the RAW value:

```rust
// chord_engine.rs:1289-1295
let weak_interior = !step.position_in_phrase.is_multiple_of(2);
if edge < 0.15 && weak_interior {
    Vec::new()                               // rest-as-gesture: NO event
} else {
    vec![sustained(0, step_ms, base_frac)]   // one inner tone, one step
}
```

Because real photos carry raw edge ≈ 0.005–0.05 (`EDGE_ACTIVITY_RANGE_MAX = 0.05`, `:1131`), `edge < 0.15` is true for essentially every real image, so on every weak interior beat both fill voices go silent. The inner harmony is dropped ~half the time. The threshold was authored against a normalization the raw value never reaches.

### 2.2 The fix (Music Theory Specialist owns the MUSICAL value of the threshold)

Re-point the guard to read the **normalized `edge_activity`** (already in scope on `:1170`):

```rust
// REPLACE the `edge < 0.15` test (chord_engine.rs:1290) with:
if edge_activity < FILL_REST_ACTIVITY && weak_interior {
    Vec::new()
} else { ... }
```

where `FILL_REST_ACTIVITY` is a new `const f32` the **Music Theory Specialist sets** — the activity floor below which a deliberate inner-voice silence is musically wanted. The intent (S16 musical doc §1.2 / RECOMMENDED SLICE 1 step 1): rest-as-gesture should be RARE and intentional, not constant. A normalized image at activity ≈ 0.08 (the "calm A" fixture, `chord_engine.rs:3288`) should NOT silence the fill; only a genuinely near-static texture should. Music Theory picks the exact value (the S16 doc suggests "≈ 0.15" on the normalized scale; the Specialist confirms or refines against the calm/busy fixtures so the calm bed actually sounds). The `edge` local (`:1173`) becomes **dead** once this is the only reader — Music Theory removes it in the same commit (a free fn, no seam).

**This does NOT move the single-line default operating point.** The equivalence net (`engine_equivalence.rs`) pins Bass (inst 0) and Melody (inst num-1) at the CADENCE step (the `is_cadence` early return `:1257` fires before this arm). The HarmonicFill arm is reached only by inner instruments on NON-cadence steps; the net's golden assertions (P5/P6) never assert on a HarmonicFill event. **No golden moves.** (See §7 for the proof and the one new property test that guards the fill-now-sounds behaviour.)

### 2.3 Why this is byte-safe even though it changes a non-cadence HarmonicFill

The net's `test_step_idx_wraps_via_modulo` (P3) and `test_full_golden_sweep_is_byte_identical` exercise NON-cadence steps, but only assert on the **Melody** instrument's onset COUNT (P3) and on run-to-run determinism (the sweep). Neither asserts the HarmonicFill rest. The sweep test compares `sweep() == sweep()` (determinism), which the fix preserves (still pure). Confirm by reading the asserts: P3 asserts `d.events.len()` for `melody = 1` (`:182`,`:190`,`:199`); the sweep asserts equality of two identical computations. **The fix is invisible to every assert.** This is the §4.1 byte-freeze table row "rest-bug fix → no golden, new property test only."

---

## 3. DELIVERABLE 2 — THE `Pad` ROLE + REALIZE BRANCH (Music Theory owns voicing/values; Implementer owns the enum + the forced match arms)

### 3.1 The enum (additive)

`OrchestralRole` (`chord_engine.rs:801`) gains TWO arms (Pad now, CounterMelody as a stub per decision A):

```rust
pub enum OrchestralRole {
    Bass,
    HarmonicFill,
    Melody,
    /// NEW S17 — a sustained HARMONY BED: holds multiple chord tones across the step
    /// in the inner register at supporting velocity. Never rests; the widest single-
    /// role simultaneous note count. The "all the harmony / all the background" fix.
    Pad,
    /// NEW S17 (realize-STUBBED) — a second melodic line under/around the Melody.
    /// Slice 1: its realize branch DELEGATES to the HarmonicFill figure (operator
    /// decision A); Slice 2 fills the counter-line craft. Unreachable under the
    /// identity profile, so byte-neutral.
    CounterMelody,
}
```

**This is additive but it FORCES new `match role` arms** in three functions that currently match the 3-arm enum exhaustively (no `_` wildcard — verified): `role_pitch` (`:998`), `realize_velocity` tail (`:1105` — this one HAS a `_ => {}` so it is already total, see §3.3), and `realize_rhythm` (`:1261`). Each MUST gain `Pad` and `CounterMelody` arms or the crate will not compile. Ownership of each arm's body is below.

### 3.2 `role_pitch` — the Pad pitch (Music Theory owns)

`role_pitch` (`:980`) returns ONE `u8`. The Pad needs a multi-note chord bed, so the **pitch selection for Pad is NOT done in `role_pitch`** — it is done in the Pad `realize_rhythm` arm directly off `step.chord.notes` (§3.5), because that arm needs the whole chord, not one tone. But `role_pitch` is still CALLED for the `base_note` (`realize_step:937`) before `realize_rhythm`. Resolution:

- `role_pitch` gains a `Pad` arm that returns a representative inner tone (e.g. the same inner-tone logic as `HarmonicFill`, or the chord root in the fill register) — this is the `base_note` passed into `realize_rhythm`, but the Pad arm there IGNORES the single `note` and re-derives the full bed from `step.chord` (which `realize_rhythm` does NOT currently receive — see §3.5 for the seam-safe way to reach the chord). **Music Theory owns the Pad register/voicing decision.**
- `role_pitch` gains a `CounterMelody` arm that returns the same inner tone as HarmonicFill (the stub delegates to the fill figure anyway).

**Recommended (Music Theory confirms):** both new arms delegate to the existing `HarmonicFill` arm body (an inner chord tone in the fill register `FILL_REGISTER_FLOOR=55`). The Pad's full-chord spread happens in the realize arm.

### 3.3 `realize_velocity` — already total, add the supporting bias (Music Theory owns)

`realize_velocity`'s role-tail `match` (`:1105`) ALREADY ends with `_ => {}` (`:1108`), so it compiles unchanged for the new roles (they fall into `_`, no per-role velocity bump). The S16 musical model wants the Pad QUIETER than the melody (it supports, not competes). **Music Theory owns** whether to add `OrchestralRole::Pad if !is_cadence => vel -= <n>` here (a small negative bias, e.g. the same −1..−3 as Bass) so the bed sits under the line. This is musical content, additive, and CANNOT affect the net (the net never reaches a Pad role under the identity profile). Recommended: add a modest negative bias so the bed supports.

### 3.4 `realize_rhythm` — the new arms (Music Theory owns Pad; CounterMelody is the stub)

`realize_rhythm`'s `match role` (`:1261`) gains two arms. **Both are reached only by the compose-path profile; the cadence early-return (`:1257`) still fires first for any cadence step, so a Pad/Counter step at a cadence rings as the single sustained cadence note — byte-stable structure, no special-casing.**

```rust
// in realize_rhythm match role (chord_engine.rs:1261), AFTER the Melody arm:

OrchestralRole::Pad => {
    // The held HARMONY BED. See §3.5 for HOW the chord reaches this arm
    // (realize_rhythm does not currently receive the Chord — the seam-safe options).
    // Emit `pad_voices` chord tones, all at offset 0, each held the FULL step under
    // legato overlap (base_frac at the connected end — §6 seam decision: hold_ms
    // capped so it never exceeds the step's wall-clock pacing). Music Theory owns
    // the voicing (which tones, which register, the velocity floor).
}
OrchestralRole::CounterMelody => {
    // STUB (operator decision A): delegate to the HarmonicFill figure until Slice 2.
    // Reuse the exact HarmonicFill body (the rest-fixed §2 version) so the role is
    // present but adds no new craft. Slice 2 replaces this body with the counter-
    // line realization — a pure realize-fill, no signature change.
    let weak_interior = !step.position_in_phrase.is_multiple_of(2);
    if edge_activity < FILL_REST_ACTIVITY && weak_interior {
        Vec::new()
    } else {
        vec![sustained(0, step_ms, base_frac)]
    }
}
```

### 3.5 THE ONE REAL SEAM ISSUE — `realize_rhythm` does not receive the `Chord`

`realize_rhythm` (`:1156`) receives a single `note: u8` (`:1157`), NOT the chord. The Pad bed needs `step.chord.notes` (the full voicing). Two seam-safe ways to give the Pad arm the chord, **without changing `realize_step`'s signature** (which is frozen):

- **Option PAD-A (RECOMMENDED — narrowest).** `realize_rhythm` is a private free fn already receiving `step: &StepPlan` (`:1164`). `StepPlan` carries `chord: Chord` (verified: the fixed plan builds `StepPlan { chord: c_major(), ... }`, `engine_equivalence.rs:58`). So **`step.chord.notes` is ALREADY reachable inside `realize_rhythm`** with no signature change at all. The Pad arm reads `step.chord.notes` directly. **This is the clean path — zero seam change.** (The `note: u8` param stays the `base_note` the other arms use; the Pad arm simply also reads `step.chord`.)
- **Option PAD-B (rejected).** Threading a `&Chord` parameter into `realize_rhythm` — unnecessary given PAD-A, and a wider touch.

**LOCK: Option PAD-A.** The Pad arm reads `step.chord.notes`, seats `pad_voices` of them into the inner register (Music Theory owns the seating — reuse `seat_pc_in_register` `:1041` with `FILL_REGISTER_FLOOR`), and emits one `NoteEvent` per tone at `offset_ms: 0`, each `hold_ms` per the §6 legato-overlap rule. `pad_voices` comes from `ctx.section.orchestration.pad_voices` — and **`ctx` is NOT currently passed to `realize_rhythm`** (it stops at `realize_step`). So the ONE genuinely needed additive thread in this slice is: **pass `pad_voices: u8` (a plain scalar, read from `ctx.section.orchestration.pad_voices` in `realize_step` before the `realize_rhythm` call) into `realize_rhythm`.** This is an additive parameter on a PRIVATE free fn (`realize_rhythm` is `fn`, not `pub fn` — verified `:1156`), so it is NOT a public-seam change and does NOT touch `realize_step`'s public signature. Under the identity profile `pad_voices == 0` and no instrument is ever assigned `Pad`, so the parameter is inert on the default path.

> **Implementer note:** the additive `pad_voices` param on the private `realize_rhythm` is the single threading change. `realize_step` reads `ctx.section.orchestration.pad_voices` (zero-copy off the borrowed section) and passes it down. The Music Theory Specialist owns the Pad arm BODY; the Rust Implementer owns adding the private param and the read in `realize_step`. Document the split in the commit.

---

## 4. DELIVERABLE 3 — PLAN-AWARE `assign_role` (Rust Implementer owns)

### 4.1 The new assigner

`realize_step` (`:905`) currently calls `instrument_role(inst_idx, num_instruments)`. Replace that ONE call with a plan-aware `assign_role` that DEFAULT-DELEGATES:

```rust
// chord_engine.rs (NEW). The ONE place the new roles enter the realizer.
pub fn assign_role(
    inst_idx: usize,
    num_instruments: usize,
    ctx: &crate::composition::StepContext,
) -> OrchestralRole {
    let prof = &ctx.section.orchestration;
    if prof.is_identity() {
        return instrument_role(inst_idx, num_instruments); // byte-stable delegate
    }
    // Non-identity profile: map inst_idx onto the profile's `layers` (LayerRole list),
    // clamping/wrapping by len; LayerRole → OrchestralRole is a total 1:1 map.
    let layers = &prof.layers;
    let lr = layers[inst_idx.min(layers.len().saturating_sub(1))]; // or a documented map rule
    lr.to_orchestral_role()
}
```

**`assign_role(default ctx) == instrument_role(inst, num)` for all `(inst, num)`** is the freeze witness (a new property test, §7). The identity check (`is_identity`) is the gate: the default profile (carried by `single_section_default`/`legacy_default_section`) returns `true`, so the legacy delegate fires byte-for-byte.

`realize_step:905` changes from:
```rust
let role = instrument_role(inst_idx, num_instruments);
```
to:
```rust
let role = assign_role(inst_idx, num_instruments, ctx);
```
This is the only line `realize_step` changes (plus reading `pad_voices` for §3.5). The signature is unchanged.

### 4.2 The inst-index → layer mapping (Implementer owns; bounded)

The non-identity branch maps `inst_idx` onto `prof.layers`. The simplest total rule (LOCK): `layers[inst_idx]` clamped to the last layer when `inst_idx >= layers.len()`. The Slice-1 "pad bed" profile's `layers` is authored (§5.2) so that, for a default 4-instrument ensemble, the assignment is e.g. `{Bass, Pad, HarmonicFill, Melody}` — one HarmonicFill becomes a Pad, giving the held bed without widening the ensemble. (Wider/independent lines via raising `num_instruments` remain a later, cleaner path; Slice 1 does not require it — the Pad arm emits MULTIPLE NoteEvents from one instrument, so the bed is full even at width 4.)

---

## 5. THE DATA MACHINERY — `OrchestrationProfile` (Rust Implementer owns; Music Theory authors the row values)

### 5.1 The new types (in `composition.rs`)

```rust
/// One named orchestration/texture profile — pure STRUCTURE, no note content. The
/// planner attaches one per Section (selected by the `texture` SelectTable); the
/// realizer's assign_role/realize_rhythm read it. Adding a profile is a JSON row,
/// not a Rust edit (the FormSpec discipline). NEW S17.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct OrchestrationProfile {
    /// Stable id, e.g. "identity" / "pad_bed".
    pub id: String,
    /// Which roles sound, in inst-index order; assign_role maps instruments onto this.
    /// serde rejects an unknown LayerRole.
    pub layers: Vec<LayerRole>,
    /// 0..1 density bias the realizer's edge_activity bands may shift by. Default 0.5
    /// == no-op (slice 1 does NOT wire this into the bands; reserved, see §8 OUT OF SCOPE).
    #[serde(default = "half")]
    pub density: f32,
    /// How many chord tones the Pad holds simultaneously (0 == no pad). Default 0.
    #[serde(default)]
    pub pad_voices: u8,
}

/// The layer vocabulary — closed (mechanism), mirrors OrchestralRole. serde-safe. NEW S17.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LayerRole { Bass, HarmonicFill, Melody, CounterMelody, Pad }
```

`OrchestrationProfile::is_identity()` returns `true` iff `pad_voices == 0` AND `layers` is the legacy split (or empty — see §5.3 for the exact identity definition). `LayerRole::to_orchestral_role()` is the obvious 1:1 map (lives in `chord_engine.rs` or `composition.rs` — Implementer picks; it bridges the two enums).

### 5.2 The `Section` field (additive)

`Section` (`composition.rs:400`) gains ONE field, AFTER `density` (`:422`), BEFORE `steps` (`:426`):

```rust
pub struct Section {
    // ... existing fields unchanged (label .. density) ...
    /// NEW S17 — the selected orchestration profile for this section. The default
    /// paths (legacy_default_section / single_section_default / the planner's
    /// identity-default sections) carry the IDENTITY profile, so the realizer is
    /// byte-stable under it.
    pub orchestration: OrchestrationProfile,
    pub steps: Vec<StepPlan>,
}
```

**Every `Section { ... }` struct-literal must add this field.** There are FOUR (verified): `legacy_default_section` (`engine.rs:746`), the planner's section build (`composition.rs:634`), the test `default_section` (`engine_equivalence.rs:82`), and any planner unit-test fixtures (`composition.rs` tests `:805`+/`:860`+). §5.3/§6.4 list the exact value each gets.

### 5.3 The IDENTITY profile — the byte-freeze anchor

The identity profile is the value that makes `assign_role` delegate to `instrument_role` and `pad_voices == 0` (no pad). Define a constructor:

```rust
impl OrchestrationProfile {
    /// The behaviour-neutral profile: today's role split, no pad. assign_role under it
    /// == instrument_role; realize emits no Pad events. The byte-freeze anchor.
    pub fn identity() -> Self {
        OrchestrationProfile { id: "identity".into(), layers: Vec::new(), density: 0.5, pad_voices: 0 }
    }
    pub fn is_identity(&self) -> bool { self.pad_voices == 0 && self.layers.is_empty() }
}
```

LOCK: identity uses an **empty `layers`** as the sentinel (cleanest `is_identity`); `assign_role`'s identity branch never reads `layers`, it delegates to `instrument_role`. The four default Section literals get `orchestration: OrchestrationProfile::identity()`. The planner attaches a NON-identity profile (`pad_voices > 0`) ONLY on the compose path (§5.4).

### 5.4 Wiring it into the planner (Rust Implementer)

`CompositionPlanner::plan` (`composition.rs:538`):
- `PlanMappings` (`:330`) gains a `texture: SelectTable` axis + a `texture_catalogue: Vec<OrchestrationProfile>`, parallel to `form`/`form_catalogue`. **Both `#[serde(default)]`** so the OLD `mappings.json` still parses (back-compat floor). `From<CompositionMappings>` (`:345`) and `CompositionMappings` (`mapping_loader.rs:109`) gain the two matching fields, also `#[serde(default)]`.
- In the section build loop (`:609`–`:650`): after selecting the profile id once per plan (`let prof_id = self.plan_mappings.texture.select(u);` — or per-section if a section-conditioned selection is wanted; Slice 1 selects ONCE per plan over the whole-image knobs), look it up in `texture_catalogue` (falling back to `OrchestrationProfile::identity()` when absent/unmatched), and set `orchestration: profile.clone()` on each pushed `Section` (`:634`).
- **CRITICAL (the "Pad reachable without saliency" resolution).** The `texture` SelectTable selects over EXISTING `ImageUnderstanding` knobs that `understand_image_pure` already computes for real on the compose path (`edge_activity`, `complexity`, `value_key`, `colorfulness`, `avg_brightness`, etc.) — NOT over any saliency knob. So a real composed image gets a non-identity (`pad_voices > 0`) profile via, e.g., a default rule, while the equivalence net (which builds a FIXED plan by hand and NEVER calls the planner — `engine_equivalence.rs` header lines 7–11) stays on the identity profile. The net is byte-green because it never touches the compose path; the compose path gets the pad because the SelectTable's DEFAULT itself is the pad profile (see §5.5). No saliency reader is needed for Slice 1.

### 5.5 The `mappings.json` additions (Music Theory authors the profile values; Implementer wires the loader)

Add to the `composition` block (`assets/mappings.json:89`), additive:

```jsonc
  "texture_catalogue": [
    { "id": "identity", "layers": [], "density": 0.5, "pad_voices": 0 },
    { "id": "pad_bed",  "layers": ["Bass","Pad","HarmonicFill","Melody"], "density": 0.55, "pad_voices": 3 }
  ],
  "texture": {
    "default": "pad_bed",
    "rules": []
  }
```

- **The `default` is `pad_bed`** — so EVERY composed image gets the held bed (the operator's "all the harmony / all the background" complaint is answered for all images on the compose path), while the equivalence net (no planner call) stays identity. Music Theory owns `pad_voices` (3 = root+3rd+5th held bed is the recommended starting voicing) and the `layers` order. **Rules stay `[]` in Slice 1** (no saliency knobs to branch on yet); Slice 2/3 add saliency-conditioned departures.
- `pad_voices: 3` means the Pad arm seats the first 3 chord tones (`step.chord.notes` is built by `generate_chords`; a triad has 3, a 7th/9th chord more — `take(pad_voices)` is safe via `.min(notes.len())`).
- The `layers` order `["Bass","Pad","HarmonicFill","Melody"]` for a 4-ensemble assigns inst 0→Bass, 1→Pad, 2→HarmonicFill, 3→Melody: one inner fill becomes the held bed, the other stays a fill, melody and bass unchanged. The held bed sits under the line; the fill still adds an inner voice.

### 5.6 Loader back-compat

`CompositionMappings` (`mapping_loader.rs:109`) gains `texture: SelectTable` and `texture_catalogue: Vec<OrchestrationProfile>`, **both `#[serde(default)]`** (so the field is absent-tolerant; an old `mappings.json` with no `texture`/`texture_catalogue` deserializes with an empty SelectTable default + empty catalogue → planner falls back to `identity()`, i.e. no pad — honest degradation, never a parse error). `From<CompositionMappings>` (`composition.rs:345`) copies the two new fields through. The block itself stays `#[serde(default)] Option<CompositionMappings>` (`:133`) unchanged.

---

## 6. THE RESOLVED SEAM DECISION (operator decision B)

### 6.1 The two options

- **True cross-step sustain.** The Pad's `NoteEvent.hold_ms` spans multiple `ms_per_step` (the chord held under several melody steps, released at the chord change). Requires the main.rs adapter to DEFER the note_off.
- **Legato-overlap re-articulation.** The Pad re-articulates each step with `base_frac` at the connected end (≥ ~1.0 so consecutive Pad notes tie/overlap), `hold_ms` ≤ ~1× step (within the existing `sustained` cap). NO scheduler change.

### 6.2 What the as-built scheduler actually does (the decisive read)

`main.rs:431`–`:496`, read precisely:
- The step loop (`:431` `for step_idx in 0..total_steps`) captures `t0 = Instant::now()` FRESH inside the loop at `:451`.
- For each event it pushes a note_on at `t0 + offset_ms` (`:463`–`:470`) and a note_off at `t0 + offset_ms + hold_ms` (`:471`–`:477`), where `hold_ms` is `ev.hold_ms` after jitter (`:460`–`:461`). **The note_off is already driven by `ev.hold_ms`, NOT by a step boundary** — the design docs' "the adapter pairs every note_on with a note_off at step end" is INACCURATE against this tree; it pairs at `onset + hold_ms`.
- BUT: the loop then `sort`s the step's events and BLOCKS, sleeping until each `sev.at` (`:483`–`:495`), and does NOT advance to the next `step_idx` (nor capture the next `t0`) until the step's LAST event has fired. **There is NO separate per-step pacing sleep** — the step's wall-clock duration IS the time of its last event.

**Consequence (decisive):** if a Pad emits `hold_ms = N * ms_per_step` (a true cross-step sustain), the step's last note_off lands at ≈ N steps' worth of time, and the loop BLOCKS there — stretching that single step's wall-clock duration to N×, wrecking the tempo for the whole piece. A naive realizer-expressed multi-step `hold_ms` is therefore actively HARMFUL under the current single-threaded, block-until-last-event scheduler. True sustain is NOT a "defer the note_off" one-liner; it requires DECOUPLING step pacing from event completion (a scheduler restructure: advance steps on a fixed `ms_per_step` clock, let note_offs fire on a separate timeline) — a non-trivial change to the un-compilable OpenCV binary.

### 6.3 The decision — LEGATO-OVERLAP, no main.rs touch

**Slice 1 ships LEGATO-OVERLAP re-articulation. The main.rs scheduler is UNTOUCHED.** Weighing the four operator-named factors:

- **Byte-freeze safety.** Legato-overlap is pure realizer/data; it touches NO scheduler. Zero risk to the freeze (the net never reaches the Pad anyway). True-sustain would force a main.rs edit on the inspection-only OpenCV binary.
- **Blast radius.** main.rs is the un-compilable OpenCV/ALSA binary (cannot `cargo build` here — verified by the engagement constraint); a scheduler restructure there is unverifiable in this environment and high-risk for a Slice-1 "smallest mechanism" change. Legato-overlap keeps the entire slice inside the headless-testable lib (`cargo test --lib` + the integration nets).
- **Audible sufficiency.** The Pad arm emits MULTIPLE simultaneous chord tones per step (the whole point — `pad_voices` notes at offset 0). Even re-articulated each step, that is a full held-chord bed under the melody at every step, at supporting velocity, with legato overlap (`hold_ms` at the connected end so consecutive beds tie with no audible gap). This is a vast improvement over the current rest-or-one-stab and directly delivers "all the harmony / all the background." The S16 musical doc itself (§5.3) names legato-overlap the acceptable fallback "still a vast improvement over rest-as-gesture."
- **What the operator re-listens through.** MIDI → external engine (Qsynth/FluidSynth per `docs/midi-routing.md`), where a real synth's release/reverb tail SMOOTHS re-articulated overlapping beds into a continuous pad — the re-attack is largely masked by the engine's envelope and reverb, so legato-overlap reads as a held bed on the actual listening path. (On a hard-gated synth the re-attack would be more audible — but the operator's path is the reverberant external engine, which favors this choice.)

### 6.4 The legato-overlap realization (Music Theory owns the exact frac)

The Pad arm (`realize_rhythm`, §3.4) emits, for each of the `pad_voices` chord tones:

```rust
// per pad tone (Music Theory owns PAD_OVERLAP_FRAC and the voicing):
sustained(0, step_ms, PAD_OVERLAP_FRAC)   // offset 0, full step, connected/overlapping
```

where the hold fraction is at the connected end so consecutive beds tie. **Constraint (so §6.2's hazard never bites):** the realized `hold_ms` MUST stay within the existing `sustained` cap behaviour — `sustained` already caps at `(frac*rit).min(1.20)` (`:1237`), so a non-cadence Pad with `rit == 1.0` and frac ≤ 1.20 yields `hold_ms ≤ 1.20 * step_ms`. **That ≤1.2× overlap is small enough that the block-until-last-event loop only over-runs each step by ≤20%** (a tolerable, near-constant tempo wobble identical to what the cadence ring already does), NOT the N× catastrophe of a true multi-step hold. Music Theory picks `PAD_OVERLAP_FRAC` in `[1.0, 1.10]` (the window ceiling `ARTIC_WINDOW_HI` is 1.10) so beds tie within the established non-cadence window. **Do NOT emit a Pad `hold_ms` greater than ~1.2× step_ms** — that is the seam-safety floor this decision rests on.

### 6.5 If the operator later wants TRUE sustain (deferred, specified)

Recorded for the Slice-N that takes it (NOT this slice): true cross-step sustain requires **decoupling step pacing from event completion** in main.rs — advance `step_idx` on a fixed `ms_per_step` cadence (a `t0 + step_idx * ms_per_step` schedule) instead of blocking on the current step's last event, and let long note_offs fire on the shared timeline. That is a scheduler restructure of the OpenCV binary, must be inspection-built + verified on a machine with OpenCV/ALSA, and is its own slice. Until then, legato-overlap is the held bed.

---

## 7. BYTE-FREEZE ARGUMENT (mirrors the S16 engine doc §4.1, updated for Pad-alone)

Every S17 change is behaviour-neutral under `single_section_default`'s IDENTITY profile. The equivalence net builds a FIXED plan by hand and NEVER calls the planner/compose path (`engine_equivalence.rs` header 7–11), so saliency/profile selection cannot reach it.

| S17 change | Default-path behaviour | Net impact |
|---|---|---|
| `OrchestralRole::{Pad, CounterMelody}` new arms | `assign_role` under identity profile delegates to `instrument_role`, which never returns them | GREEN — unreachable at default |
| `assign_role(inst, num, ctx)` replaces the `instrument_role` call at `realize_step:905` | identity profile (`is_identity()==true`) ⇒ returns exactly `instrument_role(inst, num)` | GREEN — byte-identical role |
| `Section.orchestration` field | `legacy_default_section`/`single_section_default`/test `default_section` carry `OrchestrationProfile::identity()` | GREEN — additive field, no realizer effect (4 struct literals updated, §5.2) |
| `realize_rhythm` gains private `pad_voices` param | identity ⇒ no inst is Pad ⇒ param inert; under the default `pad_voices` read is 0 | GREEN — private fn, no public seam change |
| HarmonicFill rest fix (`edge` → `edge_activity`) | net never asserts a HarmonicFill event (P5/P6 are Bass/Melody at the cadence; P3 asserts Melody onset count; sweep asserts determinism) | GREEN — invisible to every assert (§2.3) |
| `realize_velocity` Pad/Counter bias | falls into the existing `_ => {}` (or a new Pad arm reached only off the compose path) | GREEN — default never reaches Pad |
| `texture`/`texture_catalogue` in mappings + loader | serde-only, `#[serde(default)]`; net builds its plan by hand, never loads them | GREEN — same boundary as S13 `set_features_global`/RNG isolation |

**No golden moves in Slice 1.** Unlike S15's §4.4 articulation re-derivation, this slice deliberately moves ZERO golden: the new texture is reachable only through the compose-path profile, which the net never exercises; the cadence branch (`is_cadence` early return `:1257` + the `sustained` `(frac*rit).min(1.20)` 240 ms ring) is untouched; the HarmonicFill fix is invisible to every assert. The ONE test-file touch is **adding `orchestration: OrchestrationProfile::identity()` to the test's `default_section` struct literal** (`engine_equivalence.rs:82`) — a struct-literal field add, NOT an assert relaxation (S15 §3.2 discipline).

---

## 8. OUT OF SCOPE FOR SLICE 1 (explicit)

- **The saliency reader** (`analyze_regions_pure`, `pick_subject_region`, the `subject_energy`/`foreground_energy`/`background_energy` triplet, filling the S15-reserved `subject_size`/`fg_bg_contrast`/etc. with real values). Slice 1's `texture` SelectTable selects over EXISTING image knobs; saliency is Slice 2+.
- **The CounterMelody REALIZATION** (the contrary/oblique voice-leading-against-melody craft). Slice 1 ships only the enum arm + the HarmonicFill-delegating stub (decision A). Slice 2 fills it as a pure realize-fill.
- **The per-phrase / saliency-driven density VARIATION.** Slice 1's `OrchestrationProfile.density` field is present (`#[serde(default)] 0.5`) but is NOT wired into the `realize_rhythm` bands — it is reserved schema. Slice 3 wires it.
- **Promoting the DEFAULT (legacy/identity) texture to fuller.** Slice 1 keeps the identity profile byte-identical; making the legacy 4-ensemble carry a Pad would move the goldens and is a deliberate-golden-move slice of its own (S16 engine doc §4.2 / Slice 4).
- **Any true cross-step sustain / main.rs scheduler change** (§6.5). Deferred to its own inspection-built slice.
- **Raising `num_instruments` for independent lines.** Slice 1 delivers the bed via multi-NoteEvent Pad at the existing width; wider ensembles are a later path.

---

## 9. DIVERGENCES FROM THE TWO S16 DESIGN DOCS (found against the real code)

1. **Naming collision — `texture`.** The S16 engine doc names the per-section field `Section.texture: TextureProfile`. `ImageUnderstanding` already has `texture: f32` (`composition.rs:44`), feeding `Knob::Texture`. Renamed the new artifacts to `OrchestrationProfile` / `Section.orchestration` / `LayerRole` to avoid two `texture` fields on the structs the realizer reaches. (§0 naming lock.)
2. **`ctx`/`realize_step` already threaded.** The S16 docs treat "thread `ctx` into the role-assignment call" as new work; it is DONE in S15 (`realize_step:903`, `decide_instrument_action:701`). The only realize_step change is the one-line `instrument_role` → `assign_role` swap (+ reading `pad_voices`). (§1.)
3. **The Chord reaches the Pad arm with NO seam change.** The S16 engine doc §3.5 sketch reads `chord.notes` as if a chord must be threaded in. `realize_rhythm` already receives `step: &StepPlan`, and `StepPlan.chord` is in scope — Option PAD-A, zero seam change for the chord itself. The ONLY additive thread is the private `pad_voices: u8` scalar. (§3.5.)
4. **The scheduler does NOT pair note_off at step end.** The S16 docs (musical §5.3, engine §3.5) state the adapter pairs every note_on with a note_off at step end (`main.rs:489–492`). The as-built scheduler pairs at `onset + hold_ms` (`main.rs:471–477`) and the real hazard is the block-until-last-event step loop (`:483`–`:495`), which stretches a step's wall-clock to a long `hold_ms`. This SHARPENS the seam decision toward legato-overlap (a long true-sustain `hold_ms` is actively harmful, not merely unscheduled). (§6.2.)
5. **`realize_velocity` is already total** (`_ => {}` at `:1108`), so the new roles compile there with no forced arm; `role_pitch` (`:998`) and `realize_rhythm` (`:1261`) are NOT total and DO force new arms. (§3.1/§3.3.)
6. **The "Pad reachable without saliency" mechanism.** Resolved concretely: the `texture` SelectTable's DEFAULT is the `pad_bed` profile, selecting over existing knobs, so every composed image gets the bed while the hand-built net never does. No saliency knob, no `Knob` enum addition, in Slice 1. (§5.4/§5.5.)

---

## 10. FILE-OWNERSHIP + OFF-LIMITS MAP

| Area | Owner |
|---|---|
| `src/chord_engine.rs` — `OrchestralRole::{Pad, CounterMelody}` arms; the `Pad` realize branch (voicing, `PAD_OVERLAP_FRAC`, the held-bed off `step.chord`); the `CounterMelody` stub (HarmonicFill-delegate); the `FILL_REST_ACTIVITY` const + the rest-bug threshold's MUSICAL value; the Pad/Counter arms in `role_pitch`; the optional Pad velocity bias in `realize_velocity` | **Music Theory Specialist** (musical values) |
| `src/chord_engine.rs` — `assign_role` (the plan-aware delegate); `LayerRole::to_orchestral_role`; the `realize_step:905` `instrument_role`→`assign_role` swap; the private `pad_voices` param plumb on `realize_rhythm` + its read in `realize_step` | **Rust Implementer** (threading) |
| `src/composition.rs` — `OrchestrationProfile` + `LayerRole` structs/enums; `OrchestrationProfile::identity()`/`is_identity()`; `Section.orchestration` field; `PlanMappings.texture`/`texture_catalogue`; `From<CompositionMappings>` copy; attach the profile per section in `plan()` | **Rust Implementer** |
| `src/engine.rs` — `legacy_default_section` gains `orchestration: identity()` | **Rust Implementer** |
| `src/mapping_loader.rs` — `CompositionMappings` gains `texture`/`texture_catalogue` (`#[serde(default)]`); keep OLD mappings.json parsing | **Rust Implementer** (SOLE writer of `mappings.json` schema/loader) |
| `assets/mappings.json` — the `texture_catalogue` profile VALUES (`pad_voices`, `layers` order, density) + the `texture` SelectTable (default = `pad_bed`) | **Music Theory Specialist** authors the profile musical values; **Rust Implementer** is the SOLE writer of the file (Music Theory hands the values to the Implementer) |
| `tests/engine_equivalence.rs` — add `orchestration: OrchestrationProfile::identity()` to `default_section` (`:82`); NO assert relaxed | **Rust Implementer** |
| The Slice-1 property net (§11) | **Test Engineer** |

**OFF-LIMITS to BOTH implementers:** `src/main.rs` (the seam decision ships NO main.rs touch — §6.3), `src/modem.rs`, `src/bin/modem_*`, `src/synth_sink.rs`, `src/midi_output.rs`, `src/cli.rs`, `src/tui.rs`, `src/pure_analysis.rs` (no saliency reader this slice — §8). And: the existing `Bass/HarmonicFill/Melody` arm BODIES of `role_pitch`/`realize_velocity`/`realize_rhythm` are byte-stable — neither implementer reshapes them; they ADD the new arms and the rest-threshold re-point, nothing more.

---

## 11. TEST PLAN HANDOFF (Test Engineer)

`cargo test --lib` + the integration nets are the validation path (the full OpenCV binary cannot build here). The property net for Slice 1:

1. **Freeze witness (the load-bearing one).** `engine_equivalence` stays BYTE-GREEN: all existing asserts pass unchanged after the `default_section` struct-literal gains `orchestration: identity()`. Goldens 36/79/114/84/240 unmoved.
2. **`assign_role(default ctx) == instrument_role(inst, num)`** for all `(inst, num)` over a representative sweep (the freeze witness for the new assigner) — under `single_section_default`'s identity profile.
3. **The Pad bed sounds.** On a NON-cadence step under a `pad_bed`-profile `StepContext` (built by hand in the test, NOT via the planner — keep the test off the RNG path), the instrument assigned `Pad` realizes EXACTLY `min(pad_voices, chord.notes.len())` simultaneous `NoteEvent`s, all at `offset_ms == 0`, all members of `step.chord.notes` (seated into the inner register), each `hold_ms ≤ 1.2 * ms_per_step` (the §6.4 seam-safety cap).
4. **Inner voices non-silent on a low-edge photo.** Under the rest-fixed HarmonicFill, a step with normalized `edge_activity` at the "calm" fixture value (≈0.08) on a weak interior beat NO LONGER rests (emits ≥1 event) — the latent-bug regression guard. (Pair with a genuinely near-static `edge_activity` case that DOES still rest, to pin `FILL_REST_ACTIVITY`.)
5. **A non-cadence step sounds MORE simultaneous notes under `pad_bed` than under identity** (density actually rises): a full-profile step realizes strictly more total `NoteEvent`s across the ensemble than the identity profile on the same inputs.
6. **CounterMelody stub == HarmonicFill figure.** A step whose profile assigns `CounterMelody` realizes the same event shape as the (rest-fixed) HarmonicFill arm would — pinning the stub so Slice 2's replacement is a clean diff.
7. **`mappings.json` back-compat.** An old-shape mapping (no `texture`/`texture_catalogue`) still deserializes and the planner falls back to `identity()` (no pad) — honest degradation, no parse error.

---

*Design-only. No source, test, or asset modified by this document.*
